//! Message return manager — auto-sends handler return values

use async_trait::async_trait;
use serde_json::Value;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use crate::tools::mini_types::MessageMin;
use super::ReturnManager;

/// Processes handler return values (strings, arrays, send dicts)
pub struct MessageReturnManager;

impl MessageReturnManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MessageReturnManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReturnManager<MessageMin> for MessageReturnManager {
    async fn process(&self, message: &MessageMin, _api: &Api, value: Value) -> VkResult<Value> {
        if let Some(text) = value.as_str() {
            return message.answer(text).await;
        }

        if let Some(arr) = value.as_array() {
            let mut last = Value::Null;
            for item in arr {
                if let Some(text) = item.as_str() {
                    last = message.answer(text).await?;
                }
            }
            return Ok(last);
        }

        if let Some(obj) = value.as_object() {
            let text = obj
                .get("message")
                .or_else(|| obj.get("text"))
                .and_then(|t| t.as_str());

            if let Some(text) = text {
                let mut params = std::collections::HashMap::new();
                params.insert("peer_id".to_string(), message.peer_id.to_string());
                params.insert("message".to_string(), text.to_string());
                params.insert("random_id".to_string(), "0".to_string());

                if let Some(kb) = obj.get("keyboard").and_then(|k| k.as_str()) {
                    params.insert("keyboard".to_string(), kb.to_string());
                }
                if let Some(att) = obj.get("attachment").and_then(|a| a.as_str()) {
                    params.insert("attachment".to_string(), att.to_string());
                }

                return message.api.request("messages.send", &params).await;
            }
        }

        Ok(value)
    }
}
