//! Bot labeler

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use crate::dispatch::handlers::{
    FuncHandler, Handler, MessageEventFuncHandler, MessageFuncHandler, MessageReplyHandler,
};
use crate::dispatch::rules::{PeerRule, Rule};
use crate::exception::VkResult;
use crate::tools::mini_types::{MessageEventMin, MessageMin};

/// Bot-specific labeler — holds message and raw event handlers
pub struct BotLabeler {
    pub(crate) message_handlers: Vec<Arc<dyn Handler<MessageMin>>>,
    pub(crate) message_event_handlers: Vec<Arc<dyn Handler<MessageEventMin>>>,
    raw_handlers: HashMap<String, Vec<Arc<dyn Handler<Value>>>>,
    /// Raw value handlers not scoped to a specific event type.
    pub(crate) value_handlers: Vec<Arc<dyn Handler<Value>>>,
}

impl BotLabeler {
    pub fn new() -> Self {
        Self {
            message_handlers: Vec::new(),
            message_event_handlers: Vec::new(),
            raw_handlers: HashMap::new(),
            value_handlers: Vec::new(),
        }
    }

    pub fn message_handler_count(&self) -> usize {
        self.message_handlers.len()
    }

    pub fn message_event_handler_count(&self) -> usize {
        self.message_event_handlers.len()
    }

    pub fn value_handler_count(&self) -> usize {
        self.value_handlers.len()
    }

    /// Handlers registered so far.
    ///
    /// Cloning the `Arc`s rather than draining keeps the labeler intact, so
    /// syncing the router twice does not silently lose every handler.
    pub fn cloned_message_handlers(&mut self) -> Vec<Arc<dyn Handler<MessageMin>>> {
        self.message_handlers.clone()
    }

    pub fn cloned_raw_handlers(&mut self) -> HashMap<String, Vec<Arc<dyn Handler<Value>>>> {
        self.raw_handlers.clone()
    }

    pub fn cloned_value_handlers(&mut self) -> Vec<Arc<dyn Handler<Value>>> {
        self.value_handlers.clone()
    }

    pub fn cloned_message_event_handlers(&mut self) -> Vec<Arc<dyn Handler<MessageEventMin>>> {
        self.message_event_handlers.clone()
    }

    pub(crate) fn push_message_handler(&mut self, handler: Arc<dyn Handler<MessageMin>>) {
        self.message_handlers.push(handler);
    }

    pub(crate) fn push_message_event_handler(
        &mut self,
        handler: Arc<dyn Handler<MessageEventMin>>,
    ) {
        self.message_event_handlers.push(handler);
    }

    pub(crate) fn push_raw_handler(&mut self, event_type: String, handler: Arc<dyn Handler<Value>>) {
        self.raw_handlers.entry(event_type).or_default().push(handler);
    }
}

impl Default for BotLabeler {
    fn default() -> Self {
        Self::new()
    }
}

/// On-event namespace (`bot.on()`)
pub struct BotOn<'a> {
    pub labeler: &'a mut BotLabeler,
}

impl<'a> BotOn<'a> {
    /// Register handler for `message_new` with `MessageMin` and auto return processing
    pub fn message(self, rule: Box<dyn Rule<Value>>) -> MessageHandlerBuilder<'a> {
        MessageHandlerBuilder {
            bot_on: self,
            rules: vec![rule],
        }
    }

    /// Chat-only messages (peer_id > 2e9)
    pub fn chat_message(self, rule: Box<dyn Rule<Value>>) -> MessageHandlerBuilder<'a> {
        MessageHandlerBuilder {
            bot_on: self,
            rules: vec![Box::new(PeerRule::new(true)), rule],
        }
    }

    pub fn private_message(self, rule: Box<dyn Rule<Value>>) -> MessageHandlerBuilder<'a> {
        MessageHandlerBuilder {
            bot_on: self,
            rules: vec![Box::new(PeerRule::new(false)), rule],
        }
    }

    /// Callback keyboard `message_event`
    pub fn message_event(self, rule: Box<dyn Rule<Value>>) -> MessageEventHandlerBuilder<'a> {
        MessageEventHandlerBuilder {
            bot_on: self,
            rules: vec![rule],
        }
    }

    /// Static text reply when rules match (no custom handler body)
    pub fn auto_reply(self, rule: Box<dyn Rule<Value>>, text: impl Into<String>) -> Self {
        self.labeler.push_message_handler(Arc::new(MessageReplyHandler::new(
            text,
            vec![rule],
        )));
        self
    }

    /// Raw VK event by type string (e.g. `"wall_post_new"`)
    pub fn raw_event(self, event_type: &str) -> RawEventHandlerBuilder<'a> {
        RawEventHandlerBuilder {
            bot_on: self,
            event_type: event_type.to_string(),
            rules: Vec::new(),
        }
    }

    /// Handler on raw `Value` event (legacy / low-level)
    pub fn raw_value(self, rule: Box<dyn Rule<Value>>) -> ValueHandlerBuilder<'a> {
        ValueHandlerBuilder {
            bot_on: self,
            rules: vec![rule],
            event_type: None,
        }
    }
}

/// Builder for `MessageMin` handlers
pub struct MessageEventHandlerBuilder<'a> {
    bot_on: BotOn<'a>,
    rules: Vec<Box<dyn Rule<Value>>>,
}

impl<'a> MessageEventHandlerBuilder<'a> {
    pub fn rule(mut self, rule: Box<dyn Rule<Value>>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Register a handler whose arguments are extracted from the event and the
    /// bot's shared state — see [`crate::dispatch::extractors`].
    pub fn handle_with<H, Args>(self, handler: H)
    where
        H: crate::dispatch::extractors::ExtractHandler<Args>,
        Args: 'static,
    {
        let func = crate::dispatch::extractors::ExtractFuncHandler::<MessageEventMin>::new(
            self.rules, handler,
        );
        self.bot_on
            .labeler
            .push_message_event_handler(Arc::new(func));
    }

    pub fn handle<F, Fut>(self, handler: F)
    where
        F: Fn(MessageEventMin, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = VkResult<Option<Value>>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let func = MessageEventFuncHandler::new(self.rules, {
            let handler = handler.clone();
            move |ev, ctx| {
                let handler = handler.clone();
                async move { handler(ev, ctx).await.map_err(Into::into) }
            }
        });
        self.bot_on
            .labeler
            .push_message_event_handler(Arc::new(func));
    }
}

pub struct MessageHandlerBuilder<'a> {
    bot_on: BotOn<'a>,
    rules: Vec<Box<dyn Rule<Value>>>,
}

impl<'a> MessageHandlerBuilder<'a> {
    /// Register a handler whose arguments are extracted from the event and the
    /// bot's shared state — see [`crate::dispatch::extractors`].
    pub fn handle_with<H, Args>(self, handler: H)
    where
        H: crate::dispatch::extractors::ExtractHandler<Args>,
        Args: 'static,
    {
        let func = crate::dispatch::extractors::ExtractFuncHandler::<MessageMin>::new(
            self.rules, handler,
        );
        self.bot_on.labeler.push_message_handler(Arc::new(func));
    }
}

impl<'a> MessageHandlerBuilder<'a> {
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
        self.bot_on.labeler.push_message_handler(Arc::new(handler));
    }
}

/// Builder for typed raw events
pub struct RawEventHandlerBuilder<'a> {
    bot_on: BotOn<'a>,
    event_type: String,
    rules: Vec<Box<dyn Rule<Value>>>,
}

impl<'a> RawEventHandlerBuilder<'a> {
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
        self.bot_on
            .labeler
            .push_raw_handler(self.event_type, Arc::new(func));
    }
}

/// Builder for raw `Value` handlers (optionally scoped to event type)
pub struct ValueHandlerBuilder<'a> {
    bot_on: BotOn<'a>,
    rules: Vec<Box<dyn Rule<Value>>>,
    event_type: Option<String>,
}

impl<'a> ValueHandlerBuilder<'a> {
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
        if let Some(event_type) = self.event_type {
            self.bot_on.labeler.push_raw_handler(event_type, Arc::new(func));
        } else {
            self.bot_on.labeler.value_handlers.push(Arc::new(func));
        }
    }
}

pub use crate::dispatch::rules::TextRule;
