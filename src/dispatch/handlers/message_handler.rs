//! Message handler with return-value processing

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::dispatch::rules::Rule;
use crate::dispatch::return_manager::{MessageReturnManager, ReturnManager};
use crate::dispatch::{DispatchResult, EventContext, Handler, RuleResult};
use crate::tools::mini_types::MessageMin;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Handler for `message_new` that checks rules on raw event and applies return manager
pub struct MessageFuncHandler {
    rules: Vec<Box<dyn Rule<Value>>>,
    return_manager: MessageReturnManager,
    handler: Arc<
        dyn Fn(MessageMin, HashMap<String, Value>) -> BoxFuture<DispatchResult<Option<Value>>>
            + Send
            + Sync,
    >,
}

impl MessageFuncHandler {
    pub fn new<F, Fut>(rules: Vec<Box<dyn Rule<Value>>>, handler: F) -> Self
    where
        F: Fn(MessageMin, HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DispatchResult<Option<Value>>> + Send + 'static,
    {
        Self {
            rules,
            return_manager: MessageReturnManager::new(),
            handler: Arc::new(move |msg, ctx| Box::pin(handler(msg, ctx))),
        }
    }

    async fn check_rules(&self, event: &Value) -> RuleResult {
        super::evaluate_rules(&self.rules, event).await
    }
}

#[async_trait]
impl Handler<MessageMin> for MessageFuncHandler {
    async fn handle(&self, message: &MessageMin, ctx: &mut EventContext) -> DispatchResult<Option<Value>> {
        match self.check_rules(&ctx.event).await {
            RuleResult::Fail => return Ok(None),
            RuleResult::Pass => {}
            RuleResult::Context(rule_ctx) => {
                ctx.context_update.extend(rule_ctx);
            }
        }

        if !ctx.can_forward {
            return Ok(None);
        }

        let result = (self.handler)(message.clone(), ctx.context_update.clone()).await?;

        if let Some(value) = result {
            let processed = self
                .return_manager
                .process(message, &message.api, value)
                .await?;
            return Ok(Some(processed));
        }

        Ok(None)
    }

    fn rules(&self) -> &[Box<dyn Rule<MessageMin>>] {
        &[]
    }

    /// The rules live as `Rule<Value>`, so report them directly rather than
    /// through the empty typed `rules()` slice.
    fn describe(&self) -> String {
        if self.rules.is_empty() {
            return "[no rules]".to_string();
        }
        format!(
            "[{}]",
            self.rules
                .iter()
                .map(|r| r.description())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
