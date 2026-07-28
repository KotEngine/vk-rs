//! User message view — handles user long poll message updates

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::handlers::Handler;
use crate::dispatch::{DispatchResult, EventContext, View};
use crate::tools::mini_types::MessageMin;

use super::message_state::prepare_message;
use super::user_update::normalize_user_update;

/// View for user account message events (long poll arrays or normalized events)
pub struct UserMessageView {
    api: Arc<Api>,
    state_dispenser: Option<Arc<dyn StateDispenser>>,
    handlers: Vec<Arc<dyn Handler<MessageMin>>>,
}

impl UserMessageView {
    pub fn new(api: Arc<Api>) -> Self {
        Self {
            api,
            state_dispenser: None,
            handlers: Vec::new(),
        }
    }

    pub fn with_state_dispenser(mut self, dispenser: Arc<dyn StateDispenser>) -> Self {
        self.state_dispenser = Some(dispenser);
        self
    }

    pub fn register_handler(&mut self, handler: Arc<dyn Handler<MessageMin>>) -> &mut Self {
        self.handlers.push(handler);
        self
    }

    fn resolve_event(update: &Value) -> Option<Value> {
        if update.get("type").and_then(|t| t.as_str()) == Some("message_new") {
            return Some(update.clone());
        }
        normalize_user_update(update)
    }
}

#[async_trait]
impl View for UserMessageView {
    async fn process(
        &self,
        event: &Value,
        _api: &Api,
        _state_dispenser: Option<&dyn StateDispenser>,
    ) -> DispatchResult<Option<Value>> {
        let Some(normalized) = Self::resolve_event(event) else {
            return Ok(None);
        };

        let (message, enriched) =
            prepare_message(&normalized, self.api.clone(), self.state_dispenser.clone()).await?;

        for handler in &self.handlers {
            let mut ctx = EventContext::new(enriched.clone());
            if let Some(result) = handler.handle(&message, &mut ctx).await? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}
