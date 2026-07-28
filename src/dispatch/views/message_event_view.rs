//! Bot message_event view

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::handlers::Handler;
use crate::dispatch::{DispatchResult, EventContext, View};
use crate::tools::mini_types::MessageEventMin;

/// View for `message_event` callback events
pub struct MessageEventView {
    api: Arc<Api>,
    storage: Option<Arc<crate::tools::ctx_storage::CtxStorage>>,
    handlers: Vec<Arc<dyn Handler<MessageEventMin>>>,
}

impl MessageEventView {
    pub fn new(api: Arc<Api>) -> Self {
        Self {
            api,
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

    pub fn register_handler(&mut self, handler: Arc<dyn Handler<MessageEventMin>>) -> &mut Self {
        self.handlers.push(handler);
        self
    }

    fn is_message_event(event: &Value) -> bool {
        event.get("type").and_then(|t| t.as_str()) == Some("message_event")
    }
}

#[async_trait]
impl View for MessageEventView {
    async fn process(
        &self,
        event: &Value,
        _api: &Api,
        _state_dispenser: Option<&dyn StateDispenser>,
    ) -> DispatchResult<Option<Value>> {
        if !Self::is_message_event(event) {
            return Ok(None);
        }

        let message_event = MessageEventMin::from_raw_event(event, self.api.clone())?;

        for handler in &self.handlers {
            let mut ctx = EventContext::new(event.clone())
                .with_resources(self.api.clone(), self.storage.clone());
            if let Some(result) = handler.handle(&message_event, &mut ctx).await? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}
