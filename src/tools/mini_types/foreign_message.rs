//! Foreign message mini type for vkontakte

use crate::tools::{Attachment, AttachmentType, ChatAction, Geo};
use serde_json::Value;

/// Foreign message structure (for forwarded and replied messages)
#[derive(Debug, Clone)]
pub struct ForeignMessage {
    pub id: i64,
    pub from_id: i64,
    pub date: i64,
    pub text: String,
    pub attachments: Vec<Attachment>,
    pub forward_messages: Vec<ForeignMessage>,
    pub reply_message: Option<Box<ForeignMessage>>,
    pub payload: Option<String>,
    pub geo: Option<Geo>,
    pub action: Option<ChatAction>,
    pub is_mentioned: bool,
    pub out: bool,
    pub important: bool,
    pub deleted: bool,
    pub unread: bool,
    pub from_cache: bool,
    pub pinned: bool,
    pub admin_author_id: Option<i64>,
    pub conversation_message_id: Option<i64>,
    pub is_expired: bool,
    pub is_closed: bool,
    pub update_time: Option<i64>,
}

impl ForeignMessage {
    /// Create a new foreign message
    pub fn new(id: i64, from_id: i64, text: String) -> Self {
        Self {
            id,
            from_id,
            date: 0,
            text,
            attachments: Vec::new(),
            forward_messages: Vec::new(),
            reply_message: None,
            payload: None,
            geo: None,
            action: None,
            is_mentioned: false,
            out: false,
            important: false,
            deleted: false,
            unread: false,
            from_cache: false,
            pinned: false,
            admin_author_id: None,
            conversation_message_id: None,
            is_expired: false,
            is_closed: false,
            update_time: None,
        }
    }
    
    /// Create from raw JSON value
    pub fn from_raw_value(value: &Value) -> Self {
        Self::from_message_value(value)
    }

    /// Parse from a message JSON object
    pub fn from_message_value(value: &Value) -> Self {
        let mut msg = Self::new(
            value.get("id").and_then(|i| i.as_i64()).unwrap_or(0),
            value.get("from_id").and_then(|f| f.as_i64()).unwrap_or(0),
            value
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
        );

        msg.date = value.get("date").and_then(|d| d.as_i64()).unwrap_or(0);
        msg.attachments = parse_message_attachments(value);
        msg.payload = value
            .get("payload")
            .and_then(|p| p.as_str())
            .map(|p| p.to_string());
        msg.reply_message = value
            .get("reply_message")
            .map(|r| Box::new(Self::from_message_value(r)));
        msg.forward_messages = value
            .get("fwd_messages")
            .and_then(|f| f.as_array())
            .map(|arr| arr.iter().map(Self::from_message_value).collect())
            .unwrap_or_default();
        msg.geo = value.get("geo").and_then(parse_geo);
        msg.action = value
            .get("action")
            .and_then(|a| a.get("type"))
            .and_then(|t| t.as_str())
            .map(ChatAction::from_str);
        msg.is_mentioned = value
            .get("is_mentioned")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        msg.out = value.get("out").and_then(|v| v.as_bool()).unwrap_or(false);
        msg.important = value
            .get("important")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        msg.conversation_message_id = value
            .get("conversation_message_id")
            .and_then(|v| v.as_i64());
        msg.update_time = value.get("update_time").and_then(|v| v.as_i64());
        msg
    }
    
    /// Create with date
    pub fn with_date(mut self, date: i64) -> Self {
        self.date = date;
        self
    }
    
    /// Create with attachments
    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }
    
    /// Create with reply message
    pub fn with_reply_message(mut self, reply_message: Box<ForeignMessage>) -> Self {
        self.reply_message = Some(reply_message);
        self
    }
    
    /// Create with forwarded messages
    pub fn with_forward_messages(mut self, forward_messages: Vec<ForeignMessage>) -> Self {
        self.forward_messages = forward_messages;
        self
    }
    
    /// Create with payload
    pub fn with_payload(mut self, payload: String) -> Self {
        self.payload = Some(payload);
        self
    }
    
    /// Create with geo
    pub fn with_geo(mut self, geo: Geo) -> Self {
        self.geo = Some(geo);
        self
    }
    
    /// Create with action
    pub fn with_action(mut self, action: ChatAction) -> Self {
        self.action = Some(action);
        self
    }
    
    /// Create with metadata
    pub fn with_metadata(mut self, is_mentioned: bool, out: bool, important: bool, deleted: bool, unread: bool) -> Self {
        self.is_mentioned = is_mentioned;
        self.out = out;
        self.important = important;
        self.deleted = deleted;
        self.unread = unread;
        self
    }
    
    /// Create with admin info
    pub fn with_admin_info(mut self, admin_author_id: Option<i64>, conversation_message_id: Option<i64>) -> Self {
        self.admin_author_id = admin_author_id;
        self.conversation_message_id = conversation_message_id;
        self
    }
    
    /// Create with status info
    pub fn with_status_info(mut self, from_cache: bool, pinned: bool, is_expired: bool, is_closed: bool, update_time: Option<i64>) -> Self {
        self.from_cache = from_cache;
        self.pinned = pinned;
        self.is_expired = is_expired;
        self.is_closed = is_closed;
        self.update_time = update_time;
        self
    }
    
    /// Check if message is from user
    pub fn is_from_user(&self) -> bool {
        self.from_id > 0
    }
    
    /// Check if message is from chat
    pub fn is_from_chat(&self) -> bool {
        self.from_id < 0
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
    
    /// Check if message has attachments
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }
    
    /// Check if message has forwarded messages
    pub fn has_forward_messages(&self) -> bool {
        !self.forward_messages.is_empty()
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
    
    /// Get payload as JSON
    pub fn get_payload_json(&self) -> Option<Value> {
        self.payload.as_ref().and_then(|p| {
            serde_json::from_str(p).ok()
        })
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

    /// Get formatted date string (unix timestamp)
    pub fn get_formatted_date(&self) -> String {
        self.date.to_string()
    }

    fn unix_now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Check if message is recent (within last hour)
    pub fn is_recent(&self) -> bool {
        Self::unix_now() - self.date < 3600
    }

    /// Check if message is old (more than 1 day)
    pub fn is_old(&self) -> bool {
        Self::unix_now() - self.date > 86400
    }

    /// Get message age in seconds
    pub fn get_age_seconds(&self) -> i64 {
        Self::unix_now() - self.date
    }
    
    /// Get message age in minutes
    pub fn get_age_minutes(&self) -> i64 {
        self.get_age_seconds() / 60
    }
    
    /// Get message age in hours
    pub fn get_age_hours(&self) -> i64 {
        self.get_age_minutes() / 60
    }
    
    /// Get message age in days
    pub fn get_age_days(&self) -> i64 {
        self.get_age_hours() / 24
    }
    
    /// Convert to attachment string for forwarding
    pub fn to_attachment_string(&self) -> String {
        format!("message{}_{}", self.from_id, self.id)
    }
    
    /// Get message type description
    pub fn get_message_type(&self) -> &'static str {
        if self.has_attachments() {
            "with_attachment"
        } else if self.has_forward_messages() {
            "forwarded"
        } else if self.has_reply_message() {
            "reply"
        } else if self.has_geo() {
            "with_geo"
        } else if self.has_payload() {
            "with_payload"
        } else {
            "text"
        }
    }
    
    /// Get message summary
    pub fn get_summary(&self) -> String {
        let mut summary = format!("Message #{} from user {}", self.id, self.from_id);
        
        if self.is_from_user() {
            summary.push_str(" (user)");
        } else if self.is_from_chat() {
            summary.push_str(" (chat)");
        }
        
        if self.out {
            summary.push_str(" (outgoing)");
        }
        
        if self.is_mentioned {
            summary.push_str(" (mentioned)");
        }
        
        if self.important {
            summary.push_str(" (important)");
        }
        
        if self.has_attachments() {
            summary.push_str(" with attachments");
        }
        
        if self.has_forward_messages() {
            summary.push_str(" with forwarded messages");
        }
        
        if self.has_reply_message() {
            summary.push_str(" with reply");
        }
        
        if self.has_geo() {
            summary.push_str(" with location");
        }
        
        summary
    }
}

impl From<Value> for ForeignMessage {
    fn from(value: Value) -> Self {
        Self::from_message_value(&value)
    }
}

fn parse_message_attachments(value: &Value) -> Vec<Attachment> {
    value
        .get("attachments")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|att| {
                    let att_type = att.get("type")?.as_str()?;
                    let inner = att.get(att_type)?;
                    let owner_id = inner.get("owner_id").and_then(|o| o.as_i64()).unwrap_or(0);
                    let id = inner.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
                    Some(Attachment::new(
                        AttachmentType::from_str(att_type),
                        owner_id,
                        id,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_geo(g: &Value) -> Option<Geo> {
    let obj = g.as_object()?;
    Some(Geo {
        geo_type: obj
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("point")
            .to_string(),
        coordinates: obj
            .get("coordinates")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string(),
        place: None,
        access_key: None,
    })
}