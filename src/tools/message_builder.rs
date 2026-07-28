//! Fluent builder for `messages.send` parameters

use std::collections::HashMap;

use serde_json::Value;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use crate::tools::utils::random_id;
use crate::tools::Attachment;

/// Build and send VK messages with a fluent API
pub struct MessageBuilder {
    peer_id: i64,
    message: String,
    random_id: i64,
    keyboard: Option<String>,
    attachments: Vec<String>,
    reply_to: Option<i64>,
    forward_messages: Vec<i64>,
    sticker_id: Option<i64>,
    dont_parse_links: bool,
    disable_mentions: bool,
    template: Option<String>,
    payload: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
}

impl MessageBuilder {
    pub fn new(peer_id: i64) -> Self {
        Self {
            peer_id,
            message: String::new(),
            random_id: random_id(),
            keyboard: None,
            attachments: Vec::new(),
            reply_to: None,
            forward_messages: Vec::new(),
            sticker_id: None,
            dont_parse_links: false,
            disable_mentions: false,
            template: None,
            payload: None,
            lat: None,
            lon: None,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.message = text.into();
        self
    }

    pub fn keyboard_json(mut self, keyboard: impl Into<String>) -> Self {
        self.keyboard = Some(keyboard.into());
        self
    }

    pub fn attachment(mut self, att: &Attachment) -> Self {
        self.attachments.push(att.to_attachment_string());
        self
    }

    pub fn attachment_str(mut self, s: impl Into<String>) -> Self {
        self.attachments.push(s.into());
        self
    }

    pub fn reply_to(mut self, message_id: i64) -> Self {
        self.reply_to = Some(message_id);
        self
    }

    pub fn forward(mut self, message_id: i64) -> Self {
        self.forward_messages.push(message_id);
        self
    }

    pub fn sticker(mut self, sticker_id: i64) -> Self {
        self.sticker_id = Some(sticker_id);
        self
    }

    pub fn dont_parse_links(mut self, v: bool) -> Self {
        self.dont_parse_links = v;
        self
    }

    pub fn disable_mentions(mut self, v: bool) -> Self {
        self.disable_mentions = v;
        self
    }

    pub fn carousel_template(mut self, template_json: impl Into<String>) -> Self {
        self.template = Some(template_json.into());
        self
    }

    pub fn payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    pub fn geo(mut self, lat: f64, lon: f64) -> Self {
        self.lat = Some(lat);
        self.lon = Some(lon);
        self
    }

    pub fn build_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), self.peer_id.to_string());
        params.insert("message".to_string(), self.message.clone());
        params.insert("random_id".to_string(), self.random_id.to_string());

        if !self.attachments.is_empty() {
            params.insert("attachment".to_string(), self.attachments.join(","));
        }
        if let Some(kb) = &self.keyboard {
            params.insert("keyboard".to_string(), kb.clone());
        }
        if let Some(id) = self.reply_to {
            params.insert("reply_to".to_string(), id.to_string());
        }
        if !self.forward_messages.is_empty() {
            params.insert(
                "forward_messages".to_string(),
                self.forward_messages
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        if let Some(sid) = self.sticker_id {
            params.insert("sticker_id".to_string(), sid.to_string());
        }
        if self.dont_parse_links {
            params.insert("dont_parse_links".to_string(), "1".to_string());
        }
        if self.disable_mentions {
            params.insert("disable_mentions".to_string(), "1".to_string());
        }
        if let Some(t) = &self.template {
            params.insert("template".to_string(), t.clone());
        }
        if let Some(p) = &self.payload {
            params.insert("payload".to_string(), p.clone());
        }
        if let (Some(lat), Some(lon)) = (self.lat, self.lon) {
            params.insert("lat".to_string(), lat.to_string());
            params.insert("long".to_string(), lon.to_string());
        }
        params
    }

    pub async fn send(self, api: &Api) -> VkResult<Value> {
        api.request("messages.send", &self.build_params()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_collects_attachments() {
        let params = MessageBuilder::new(1)
            .text("hi")
            .attachment_str("photo1_2")
            .attachment_str("doc3_4")
            .build_params();
        assert_eq!(params.get("attachment").map(String::as_str), Some("photo1_2,doc3_4"));
    }
}
