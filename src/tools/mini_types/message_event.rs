//! Message event mini type (callback keyboard buttons)

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use crate::tools::event_data::{OpenAppEvent, OpenLinkEvent, ShowSnackbarEvent};

/// Parsed `message_event` callback
#[derive(Clone)]
pub struct MessageEventMin {
    pub event_id: String,
    pub user_id: i64,
    pub peer_id: i64,
    pub conversation_message_id: Option<i64>,
    pub payload: Option<Value>,
    pub api: Arc<Api>,
}

impl MessageEventMin {
    pub fn from_raw_event(event: &Value, api: Arc<Api>) -> VkResult<Self> {
        let obj = event
            .get("object")
            .ok_or_else(|| crate::exception::VkError::Validation("missing object".into()))?;

        let event_id = obj
            .get("event_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let user_id = obj
            .get("user_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::exception::VkError::Validation("missing user_id".into()))?;

        let peer_id = obj
            .get("peer_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::exception::VkError::Validation("missing peer_id".into()))?;

        let conversation_message_id = obj.get("conversation_message_id").and_then(|v| v.as_i64());

        let payload = obj.get("payload").cloned();

        Ok(Self {
            event_id,
            user_id,
            peer_id,
            conversation_message_id,
            payload,
            api,
        })
    }

    pub fn get_payload_json(&self) -> Option<Value> {
        self.payload.clone()
    }

    async fn send_event_answer(&self, event_data: Option<String>) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("event_id".to_string(), self.event_id.clone());
        params.insert("user_id".to_string(), self.user_id.to_string());
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        if let Some(data) = event_data {
            params.insert("event_data".to_string(), data);
        }
        self.api
            .request("messages.sendMessageEventAnswer", &params)
            .await
    }

    pub async fn send_empty_answer(&self) -> VkResult<Value> {
        self.send_event_answer(None).await
    }

    pub async fn show_snackbar(&self, text: impl Into<String>) -> VkResult<Value> {
        let data = ShowSnackbarEvent::new(text).to_json();
        self.send_event_answer(Some(data)).await
    }

    pub async fn open_link(&self, link: impl Into<String>) -> VkResult<Value> {
        let data = OpenLinkEvent::new(link).to_json();
        self.send_event_answer(Some(data)).await
    }

    pub async fn open_app(
        &self,
        app_id: i64,
        app_hash: impl Into<String>,
        owner_id: Option<i64>,
    ) -> VkResult<Value> {
        let mut ev = OpenAppEvent::new(app_id, app_hash);
        if let Some(oid) = owner_id {
            ev = ev.with_owner_id(oid);
        }
        self.send_event_answer(Some(ev.to_json())).await
    }

    pub async fn edit_message(&self, text: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        if let Some(cmid) = self.conversation_message_id {
            params.insert("conversation_message_id".to_string(), cmid.to_string());
        }
        self.api.request("messages.edit", &params).await
    }

    pub async fn send_message(&self, text: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        params.insert("random_id".to_string(), "0".to_string());
        self.api.request("messages.send", &params).await
    }
}
