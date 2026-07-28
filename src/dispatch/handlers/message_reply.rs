//! Auto-reply handler — sends a fixed text when rules match

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::VkApi;
use crate::dispatch::rules::Rule;
use crate::dispatch::{DispatchResult, EventContext, Handler, RuleResult};
use crate::tools::mini_types::MessageMin;

/// Sends `text` via `answer` or `reply` when all rules pass
pub struct MessageReplyHandler {
    rules: Vec<Box<dyn Rule<Value>>>,
    text: String,
    as_reply: bool,
    extra_params: HashMap<String, String>,
}

impl MessageReplyHandler {
    pub fn new(text: impl Into<String>, rules: Vec<Box<dyn Rule<Value>>>) -> Self {
        Self {
            rules,
            text: text.into(),
            as_reply: false,
            extra_params: HashMap::new(),
        }
    }

    pub fn as_reply(mut self, value: bool) -> Self {
        self.as_reply = value;
        self
    }

    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_params.insert(key.into(), value.into());
        self
    }

    async fn check_rules(&self, event: &Value) -> RuleResult {
        super::evaluate_rules(&self.rules, event).await
    }
}

#[async_trait]
impl Handler<MessageMin> for MessageReplyHandler {
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

        if self.as_reply {
            let mut params = self.extra_params.clone();
            params.insert("peer_id".to_string(), message.peer_id.to_string());
            params.insert("message".to_string(), self.text.clone());
            params.insert("random_id".to_string(), "0".to_string());
            params.insert("reply_to".to_string(), message.id.to_string());
            let result = message.api.request("messages.send", &params).await?;
            Ok(Some(result))
        } else {
            let result = message.answer(&self.text).await?;
            Ok(Some(result))
        }
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

/// Builder for quick static replies in blueprints
pub struct MessageReplyBuilder {
    text: String,
    rules: Vec<Box<dyn Rule<Value>>>,
    as_reply: bool,
}

impl MessageReplyBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            rules: Vec::new(),
            as_reply: false,
        }
    }

    pub fn rule(mut self, rule: Box<dyn Rule<Value>>) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn as_reply(mut self) -> Self {
        self.as_reply = true;
        self
    }

    pub fn build(self) -> MessageReplyHandler {
        MessageReplyHandler::new(self.text, self.rules).as_reply(self.as_reply)
    }
}
