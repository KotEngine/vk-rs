//! Return manager for user-account message handlers

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::exception::VkResult;
use crate::tools::mini_types::MessageMin;
use super::ReturnManager;

/// Same return processing as bot messages (strings → send)
pub struct UserMessageReturnManager;

impl UserMessageReturnManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UserMessageReturnManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReturnManager<MessageMin> for UserMessageReturnManager {
    async fn process(&self, message: &MessageMin, _api: &Api, value: Value) -> VkResult<Value> {
        super::message::MessageReturnManager::new()
            .process(message, &message.api, value)
            .await
    }
}
