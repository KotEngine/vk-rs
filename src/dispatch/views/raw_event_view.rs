//! Raw event view — routes by event type string

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::api::Api;
use crate::dispatch::handlers::Handler;
use crate::dispatch::{DispatchResult, EventContext, View};

/// View for raw VK group events
pub struct RawEventView {
    handlers: HashMap<String, Vec<Arc<dyn Handler<Value>>>>,
}

impl RawEventView {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, event_type: &str, handler: Arc<dyn Handler<Value>>) -> &mut Self {
        self.handlers
            .entry(event_type.to_string())
            .or_default()
            .push(handler);
        self
    }
}

impl Default for RawEventView {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl View for RawEventView {
    async fn process(
        &self,
        event: &Value,
        _api: &Api,
        _state_dispenser: Option<&dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> DispatchResult<Option<Value>> {
        let event_type = match event.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => return Ok(None),
        };

        let Some(handlers) = self.handlers.get(event_type) else {
            return Ok(None);
        };

        for handler in handlers {
            let mut ctx = EventContext::new(event.clone());
            if let Some(result) = handler.handle(event, &mut ctx).await? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}
