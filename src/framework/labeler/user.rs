//! User labeler

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use crate::dispatch::handlers::{FuncHandler, Handler, MessageFuncHandler};
use crate::dispatch::rules::{PeerRule, Rule};
use crate::exception::VkResult;
use crate::tools::mini_types::MessageMin;

/// User-specific labeler — holds message and raw update handlers
pub struct UserLabeler {
    pub(crate) message_handlers: Vec<Arc<dyn Handler<MessageMin>>>,
    raw_handlers: Vec<Arc<dyn Handler<Value>>>,
}

impl UserLabeler {
    pub fn new() -> Self {
        Self {
            message_handlers: Vec::new(),
            raw_handlers: Vec::new(),
        }
    }

    pub fn message_handler_count(&self) -> usize {
        self.message_handlers.len()
    }

    /// Handlers registered so far — cloned, not drained, so repeated router
    /// syncs stay idempotent.
    pub fn cloned_message_handlers(&mut self) -> Vec<Arc<dyn Handler<MessageMin>>> {
        self.message_handlers.clone()
    }

    pub fn cloned_raw_handlers(&mut self) -> Vec<Arc<dyn Handler<Value>>> {
        self.raw_handlers.clone()
    }

    pub(crate) fn push_message_handler(&mut self, handler: Arc<dyn Handler<MessageMin>>) {
        self.message_handlers.push(handler);
    }

    pub(crate) fn push_raw_handler(&mut self, handler: Arc<dyn Handler<Value>>) {
        self.raw_handlers.push(handler);
    }
}

impl Default for UserLabeler {
    fn default() -> Self {
        Self::new()
    }
}

/// On-event namespace (`user.on()`)
pub struct UserOn<'a> {
    pub labeler: &'a mut UserLabeler,
}

impl<'a> UserOn<'a> {
    pub fn message(self, rule: Box<dyn Rule<Value>>) -> UserMessageHandlerBuilder<'a> {
        UserMessageHandlerBuilder {
            user_on: self,
            rules: vec![rule],
        }
    }

    pub fn chat_message(self, rule: Box<dyn Rule<Value>>) -> UserMessageHandlerBuilder<'a> {
        UserMessageHandlerBuilder {
            user_on: self,
            rules: vec![Box::new(PeerRule::new(true)), rule],
        }
    }

    pub fn private_message(self, rule: Box<dyn Rule<Value>>) -> UserMessageHandlerBuilder<'a> {
        UserMessageHandlerBuilder {
            user_on: self,
            rules: vec![Box::new(PeerRule::new(false)), rule],
        }
    }

    /// Handler on raw user long poll update (`Value` array or object)
    pub fn raw_update(self, rule: Box<dyn Rule<Value>>) -> UserRawHandlerBuilder<'a> {
        UserRawHandlerBuilder {
            user_on: self,
            rules: vec![rule],
        }
    }
}

pub struct UserMessageHandlerBuilder<'a> {
    user_on: UserOn<'a>,
    rules: Vec<Box<dyn Rule<Value>>>,
}

impl<'a> UserMessageHandlerBuilder<'a> {
    pub fn rule(mut self, rule: Box<dyn Rule<Value>>) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn handle<F, Fut>(self, handler: F)
    where
        F: Fn(MessageMin, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = VkResult<Option<Value>>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let handler = MessageFuncHandler::new(self.rules, {
            let handler = handler.clone();
            move |msg, ctx| {
                let handler = handler.clone();
                async move { handler(msg, ctx).await.map_err(Into::into) }
            }
        });
        self.user_on.labeler.push_message_handler(Arc::new(handler));
    }
}

pub struct UserRawHandlerBuilder<'a> {
    user_on: UserOn<'a>,
    rules: Vec<Box<dyn Rule<Value>>>,
}

impl<'a> UserRawHandlerBuilder<'a> {
    pub fn rule(mut self, rule: Box<dyn Rule<Value>>) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn handle<F, Fut>(self, handler: F)
    where
        F: Fn(Value, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = VkResult<Option<Value>>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let func = FuncHandler::new(self.rules, {
            let handler = handler.clone();
            move |event, ctx| {
                let handler = handler.clone();
                async move { handler(event, ctx).await.map_err(Into::into) }
            }
        });
        self.user_on.labeler.push_raw_handler(Arc::new(func));
    }
}

pub use crate::dispatch::rules::TextRule;
