//! Return manager for message_event handlers

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::exception::VkResult;
use crate::tools::mini_types::MessageEventMin;
use super::ReturnManager;

pub struct MessageEventReturnManager;

impl MessageEventReturnManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MessageEventReturnManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReturnManager<MessageEventMin> for MessageEventReturnManager {
    async fn process(&self, event: &MessageEventMin, _api: &Api, value: Value) -> VkResult<Value> {
        if let Some(text) = value.as_str() {
            return event.show_snackbar(text).await;
        }
        if let Some(obj) = value.as_object() {
            if let Some(text) = obj.get("snackbar").and_then(|t| t.as_str()) {
                return event.show_snackbar(text).await;
            }
            if let Some(link) = obj.get("link").and_then(|l| l.as_str()) {
                return event.open_link(link).await;
            }
        }
        Ok(value)
    }
}
