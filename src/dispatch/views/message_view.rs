//! Bot message view — handles message_new events

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::handlers::Handler;
use crate::dispatch::{DispatchResult, EventContext, View};
use crate::tools::mini_types::MessageMin;

use super::message_state::prepare_message;

/// View for bot message events
pub struct BotMessageView {
    api: Arc<Api>,
    state_dispenser: Option<Arc<dyn StateDispenser>>,
    storage: Option<Arc<crate::tools::ctx_storage::CtxStorage>>,
    handlers: Vec<Arc<dyn Handler<MessageMin>>>,
}

impl BotMessageView {
    pub fn new(api: Arc<Api>) -> Self {
        Self {
            api,
            state_dispenser: None,
            storage: None,
            handlers: Vec::new(),
        }
    }

    /// Shared state handed to extractor-based handlers.
    pub fn with_storage(
        mut self,
        storage: Arc<crate::tools::ctx_storage::CtxStorage>,
    ) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn with_state_dispenser(mut self, dispenser: Arc<dyn StateDispenser>) -> Self {
        self.state_dispenser = Some(dispenser);
        self
    }

    pub fn register_handler(&mut self, handler: Arc<dyn Handler<MessageMin>>) -> &mut Self {
        self.handlers.push(handler);
        self
    }

    pub fn handlers(&self) -> &[Arc<dyn Handler<MessageMin>>] {
        &self.handlers
    }

    fn is_message_event(event: &Value) -> bool {
        event.get("type").and_then(|t| t.as_str()) == Some("message_new")
    }
}

#[async_trait]
impl View for BotMessageView {
    async fn process(
        &self,
        event: &Value,
        _api: &Api,
        _state_dispenser: Option<&dyn StateDispenser>,
    ) -> DispatchResult<Option<Value>> {
        if !Self::is_message_event(event) {
            return Ok(None);
        }

        let (message, enriched) =
            prepare_message(event, self.api.clone(), self.state_dispenser.clone()).await?;

        for handler in &self.handlers {
            let mut ctx = EventContext::new(enriched.clone())
                .with_resources(self.api.clone(), self.storage.clone());
            if let Some(result) = handler.handle(&message, &mut ctx).await? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}
