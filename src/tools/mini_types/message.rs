//! Message mini type for vkontakte

use super::*;
use crate::api::VkApi;
use crate::exception::*;
use crate::tools::{Attachment, AttachmentType, ChatAction, Geo};
use crate::tools::fsm::StatePeer;
use serde_json::Value;
use std::sync::Arc;

/// Check whether `user_id` is admin or owner in a chat conversation
pub async fn user_is_admin_in_chat(
    api: &crate::api::Api,
    peer_id: i64,
    user_id: i64,
) -> VkResult<bool> {
    let mut params = std::collections::HashMap::new();
    params.insert("peer_id".to_string(), peer_id.to_string());

    let response = api.request("messages.getConversationMembers", &params).await?;

    if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
        for member in items {
            let member_id = member.get("member_id").and_then(|m| m.as_i64());
            let is_admin = member
                .get("is_admin")
                .and_then(|a| a.as_bool())
                .unwrap_or(false)
                || member
                    .get("is_owner")
                    .and_then(|o| o.as_bool())
                    .unwrap_or(false);

            if member_id == Some(user_id) && is_admin {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
#[derive(Clone)]
pub struct MessageMin {
    pub peer_id: i64,
    pub from_id: i64,
    pub id: i64,
    pub text: String,
    pub date: i64,
    pub attachments: Vec<Attachment>,
    pub payload: Option<String>,
    pub reply_message: Option<ForeignMessage>,
    pub fwd_messages: Vec<ForeignMessage>,
    pub geo: Option<Geo>,
    pub action: Option<ChatAction>,
    pub is_mentioned: bool,
    pub important: bool,
    pub out: bool,
    pub conversation_message_id: Option<i64>,
    pub is_expired: bool,
    pub is_closed: bool,
    pub update_time: Option<i64>,
    pub state_peer: Option<StatePeer>,

    state_dispenser: Option<Arc<dyn crate::dispatch::dispenser::StateDispenser>>,

    pub api: Arc<crate::api::Api>,
}

impl MessageMin {
    /// Create a new message
    pub fn new(peer_id: i64, from_id: i64, text: String, api: Arc<crate::api::Api>) -> Self {
        Self {
            peer_id,
            from_id,
            id: 0,
            text,
            date: 0,
            attachments: Vec::new(),
            payload: None,
            reply_message: None,
            fwd_messages: Vec::new(),
            geo: None,
            action: None,
            is_mentioned: false,
            important: false,
            out: false,
            conversation_message_id: None,
            is_expired: false,
            is_closed: false,
            update_time: None,
            state_peer: None,
            state_dispenser: None,
            api,
        }
    }
    
    /// Create from raw event
    pub fn from_raw_event(event: &Value, api: Arc<crate::api::Api>) -> Result<Self, VkError> {
        let message_obj = event
            .get("object")
            .and_then(|o| o.get("message"))
            .or_else(|| event.get("message"))
            .ok_or_else(|| {
                VkError::Validation("Missing message field in event".to_string())
            })?;
        
        let peer_id = message_obj.get("peer_id")
            .and_then(|p| p.as_i64())
            .ok_or_else(|| VkError::Validation("Missing peer_id".to_string()))?;
        
        let from_id = message_obj.get("from_id")
            .and_then(|f| f.as_i64())
            .ok_or_else(|| VkError::Validation("Missing from_id".to_string()))?;
        
        let id = message_obj.get("id")
            .and_then(|i| i.as_i64())
            .ok_or_else(|| VkError::Validation("Missing message id".to_string()))?;
        
        let text = message_obj.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        
        let date = message_obj.get("date")
            .and_then(|d| d.as_i64())
            .unwrap_or(0);
        
        // Parse attachments
        let attachments = message_obj.get("attachments")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|att| att.as_object())
                    .filter_map(|att| {
                        let attachment_type = att.get("type")
                            .and_then(|t| t.as_str())
                            .map(AttachmentType::from_str)
                            .unwrap_or(AttachmentType::Unknown("unknown".to_string()));
                        
                        let owner_id = att.get("owner_id")
                            .and_then(|o| o.as_i64())
                            .unwrap_or(0);
                        
                        let id = att.get("id")
                            .and_then(|i| i.as_i64())
                            .unwrap_or(0);
                        
                        Some(Attachment::new(attachment_type, owner_id, id))
                    })
                    .collect()
            })
            .unwrap_or(Vec::new());
        
        // Parse payload
        let payload = message_obj.get("payload")
            .and_then(|p| p.as_str())
            .map(|p| p.to_string());
        
        // Parse reply message
        let reply_message = message_obj.get("reply_message")
            .and_then(|r| r.as_object())
            .map(|r| {
                let reply_value = serde_json::to_value(r).unwrap_or(Value::Null);
                ForeignMessage::from(reply_value)
            });
        
        // Parse forwarded messages
        let fwd_messages = message_obj.get("fwd_messages")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|msg| msg.as_object())
                    .map(|msg| {
                        let msg_value = serde_json::to_value(msg).unwrap_or(Value::Null);
                        ForeignMessage::from(msg_value)
                    })
                    .collect()
            })
            .unwrap_or(Vec::new());
        
        // Parse geo
        let geo = message_obj.get("geo")
            .and_then(|g| g.as_object())
            .map(|g| Geo {
                geo_type: g
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("point")
                    .to_string(),
                coordinates: g.get("coordinates")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string(),
                place: None,
                access_key: None,
            });
        
        // Parse action
        let action = message_obj.get("action")
            .and_then(|a| a.as_object())
            .and_then(|a| a.get("type"))
            .and_then(|t| t.as_str())
            .map(ChatAction::from_str);
        
        let is_mentioned = message_obj.get("is_mentioned")
            .and_then(|i| i.as_bool())
            .unwrap_or(false);
        
        let important = message_obj.get("important")
            .and_then(|i| i.as_bool())
            .unwrap_or(false);
        
        let out = message_obj.get("out")
            .and_then(|o| o.as_bool())
            .unwrap_or(false);
        
        let conversation_message_id = message_obj.get("conversation_message_id")
            .and_then(|c| c.as_i64());
        
        let is_expired = message_obj.get("is_expired")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        
        let is_closed = message_obj.get("is_closed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        
        let update_time = message_obj.get("update_time")
            .and_then(|u| u.as_i64());
        
        Ok(Self {
            peer_id,
            from_id,
            id,
            text,
            date,
            attachments,
            payload,
            reply_message,
            fwd_messages,
            geo,
            action,
            is_mentioned,
            important,
            out,
            conversation_message_id,
            is_expired,
            is_closed,
            update_time,
            state_peer: None,
            state_dispenser: None,
            api,
        })
    }

    pub fn with_state_dispenser(
        mut self,
        dispenser: Arc<dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> Self {
        self.state_dispenser = Some(dispenser);
        self
    }

    fn dispenser(&self) -> VkResult<&dyn crate::dispatch::dispenser::StateDispenser> {
        self.state_dispenser
            .as_deref()
            .ok_or_else(|| VkError::Internal("State dispenser is not configured".to_string()))
    }

    /// Set FSM state for this message peer
    pub async fn set_state(&self, state: impl Into<String>) -> VkResult<()> {
        let dispenser = self.dispenser()?;
        crate::tools::fsm::set_peer_state(dispenser, self.peer_id, state).await
    }

    /// Set FSM state with payload
    pub async fn set_state_with_payload(
        &self,
        state: impl Into<String>,
        payload: std::collections::HashMap<String, Value>,
    ) -> VkResult<()> {
        let dispenser = self.dispenser()?;
        crate::tools::fsm::set_peer_state_with_payload(dispenser, self.peer_id, state, payload)
            .await
    }

    /// Clear FSM state for this message peer
    pub async fn delete_state(&self) -> VkResult<bool> {
        let dispenser = self.dispenser()?;
        crate::tools::fsm::delete_peer_state(dispenser, self.peer_id).await
    }

    /// Current FSM state string for this peer, if any
    pub async fn get_state(&self) -> VkResult<Option<String>> {
        let Some(peer) = self.state_peer.as_ref() else {
            let dispenser = self.dispenser()?;
            return Ok(dispenser
                .get(self.peer_id)
                .await?
                .map(|p| p.state));
        };
        Ok(Some(peer.state.clone()))
    }

    /// Load FSM state from dispenser into `state_peer`
    pub async fn refresh_state(&mut self) -> VkResult<()> {
        let dispenser = self.dispenser()?;
        self.state_peer = dispenser.get(self.peer_id).await?;
        Ok(())
    }
    
    /// Answer the message
    pub async fn answer(&self, text: &str) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        params.insert("random_id".to_string(), "0".to_string());
        
        if self.has_attachments() {
            let attachment_strings: Vec<String> = self.attachments.iter()
                .map(|a| a.to_attachment_string())
                .collect();
            params.insert("attachment".to_string(), attachment_strings.join(","));
        }
        
        self.api.request("messages.send", &params).await
    }
    
    /// Reply to the message
    pub async fn reply(&self, text: &str) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        params.insert("random_id".to_string(), "0".to_string());
        params.insert("reply_to".to_string(), self.id.to_string());
        
        self.api.request("messages.send", &params).await
    }
    
    /// Forward the message
    pub async fn forward(&self, text: &str) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        params.insert("random_id".to_string(), "0".to_string());
        params.insert("forward_messages".to_string(), self.id.to_string());
        
        self.api.request("messages.send", &params).await
    }
    
    /// Edit the message
    pub async fn edit(&self, text: &str) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), text.to_string());
        params.insert("message_id".to_string(), self.id.to_string());
        
        self.api.request("messages.edit", &params).await
    }
    
    /// Delete the message
    pub async fn delete(&self) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("id".to_string(), self.id.to_string());
        params.insert("delete_for_all".to_string(), "1".to_string());
        
        self.api.request("messages.delete", &params).await
    }
    
    /// Pin the message
    pub async fn pin(&self) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message_id".to_string(), self.id.to_string());
        
        self.api.request("messages.pin", &params).await
    }
    
    /// Unpin the message
    pub async fn unpin(&self) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        
        self.api.request("messages.unpin", &params).await
    }
    
    /// Get user information
    pub async fn get_user_info(&self) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("user_ids".to_string(), self.from_id.to_string());
        params.insert("fields".to_string(), "photo_50,photo_100,photo_200,is_online,online,last_seen,sex,status".to_string());
        
        self.api.request("users.get", &params).await
    }
    
    /// Get chat information
    pub async fn get_chat_info(&self) -> VkResult<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("chat_ids".to_string(), self.peer_id.to_string());
        params.insert("fields".to_string(), "photo_50,photo_100,photo_200,name,admin_id,members_count".to_string());
        
        self.api.request("messages.getChat", &params).await
    }
    
    /// Check if user is admin in chat
    pub async fn user_is_admin(&self, user_id: i64) -> VkResult<bool> {
        user_is_admin_in_chat(&self.api, self.peer_id, user_id).await
    }

    /// Conversation members with optional TTL cache
    pub async fn get_conversation_members(
        &self,
        cache: Option<&crate::tools::ConversationMembersCache>,
    ) -> VkResult<Value> {
        match cache {
            Some(c) => c.get_members(&self.api, self.peer_id).await,
            None => {
                let mut params = std::collections::HashMap::new();
                params.insert("peer_id".to_string(), self.peer_id.to_string());
                self.api
                    .request("messages.getConversationMembers", &params)
                    .await
            }
        }
    }

    /// Send via fluent `MessageBuilder`
    pub async fn send_builder(
        &self,
        builder: crate::tools::MessageBuilder,
    ) -> VkResult<Value> {
        builder.send(&self.api).await
    }

    /// Shorthand: answer with keyboard JSON
    pub async fn answer_with_keyboard(
        &self,
        text: &str,
        keyboard_json: &str,
    ) -> VkResult<Value> {
        crate::tools::MessageBuilder::new(self.peer_id)
            .text(text)
            .keyboard_json(keyboard_json)
            .send(&self.api)
            .await
    }
    
    /// Get payload as JSON
    pub fn get_payload_json(&self) -> Option<Value> {
        self.payload.as_ref().and_then(|p| {
            serde_json::from_str(p).ok()
        })
    }
    
    /// Check if message has attachments
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }
    
    /// Check if message has forwarded messages
    pub fn has_forward_messages(&self) -> bool {
        !self.fwd_messages.is_empty()
    }
    
    /// Check if message has reply message
    pub fn has_reply_message(&self) -> bool {
        self.reply_message.is_some()
    }
    
    /// Check if message has geo
    pub fn has_geo(&self) -> bool {
        self.geo.is_some()
    }
    
    /// Check if message has payload
    pub fn has_payload(&self) -> bool {
        self.payload.is_some()
    }
    
    /// Check if message is from user
    pub fn is_from_user(&self) -> bool {
        self.from_id > 0
    }
    
    /// Check if message is from chat
    pub fn is_from_chat(&self) -> bool {
        self.from_id < 0
    }
    
    /// Check if message is from current user
    pub fn is_from_me(&self) -> bool {
        self.out
    }
    
    /// Check if message is mentioned
    pub fn is_mentioned(&self) -> bool {
        self.is_mentioned
    }
    
    /// Check if message is important
    pub fn is_important(&self) -> bool {
        self.important
    }
    
    /// Check if message is expired
    pub fn is_expired(&self) -> bool {
        self.is_expired
    }
    
    /// Check if message is closed
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }
    
    /// Get user ID if from user
    pub fn get_user_id(&self) -> Option<i64> {
        if self.from_id > 0 {
            Some(self.from_id)
        } else {
            None
        }
    }
    
    /// Get chat ID if from chat
    pub fn get_chat_id(&self) -> Option<i64> {
        if self.from_id < 0 {
            Some(-self.from_id)
        } else {
            None
        }
    }
    
    /// Get attachment by type
    pub fn get_attachments_by_type(&self, attachment_type: &AttachmentType) -> Vec<&Attachment> {
        self.attachments.iter()
            .filter(|a| a.attachment_type == *attachment_type)
            .collect()
    }
    
    /// Get text without mentions
    pub fn get_text_without_mentions(&self) -> String {
        let mut text = self.text.clone();

        if let Ok(re) = regex::Regex::new(r"@?\[id\d+\|[^\]]+\]") {
            text = re.replace_all(&text, "").to_string();
        }
        if let Ok(re_mention) = regex::Regex::new(r"@\w+") {
            text = re_mention.replace_all(&text, "").to_string();
        }

        text.trim().to_string()
    }
}

impl From<Value> for MessageMin {
    fn from(value: Value) -> Self {
        Self::from_raw_event(&value, Arc::new(crate::api::api("dummy").unwrap())).unwrap_or_else(|_| Self::new(0, 0, "".to_string(), Arc::new(crate::api::api("dummy").unwrap())))
    }
}