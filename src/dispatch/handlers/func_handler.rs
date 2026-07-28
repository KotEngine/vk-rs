//! Function-based handler

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use serde_json::Value;

use crate::dispatch::rules::Rule;
use crate::dispatch::{DispatchResult, EventContext, Handler, RuleResult};

/// Handler backed by an async function
pub struct FuncHandler<E> {
    rules: Vec<Box<dyn Rule<E>>>,
    handler: Box<
        dyn Fn(E, HashMap<String, Value>) -> Pin<Box<dyn Future<Output = DispatchResult<Option<Value>>> + Send>>
            + Send
            + Sync,
    >,
}

impl<E> FuncHandler<E>
where
    E: Send + Sync + Clone + 'static,
{
    pub fn new<F, Fut>(rules: Vec<Box<dyn Rule<E>>>, handler: F) -> Self
    where
        F: Fn(E, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DispatchResult<Option<Value>>> + Send + 'static,
    {
        Self {
            rules,
            handler: Box::new(move |event, ctx| Box::pin(handler(event, ctx))),
        }
    }
}

#[async_trait]
impl<E> Handler<E> for FuncHandler<E>
where
    E: Send + Sync + Clone + 'static,
{
    async fn handle(&self, event: &E, ctx: &mut EventContext) -> DispatchResult<Option<Value>> {
        match self.check_rules(event).await {
            RuleResult::Fail => return Ok(None),
            RuleResult::Pass => {}
            RuleResult::Context(rule_ctx) => {
                ctx.context_update.extend(rule_ctx);
            }
        }

        if !ctx.can_forward {
            return Ok(None);
        }

        (self.handler)(event.clone(), ctx.context_update.clone()).await
    }

    fn rules(&self) -> &[Box<dyn Rule<E>>] {
        &self.rules
    }
}

/// Builder for FuncHandler with rules
pub struct HandlerBuilder<E> {
    rules: Vec<Box<dyn Rule<E>>>,
}

impl<E> HandlerBuilder<E>
where
    E: Send + Sync + Clone + 'static,
{
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn rule(mut self, rule: Box<dyn Rule<E>>) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn handle<F, Fut>(self, handler: F) -> FuncHandler<E>
    where
        F: Fn(E, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DispatchResult<Option<Value>>> + Send + 'static,
    {
        FuncHandler::new(self.rules, handler)
    }
}

impl<E: Send + Sync + Clone + 'static> Default for HandlerBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}
