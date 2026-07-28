//! Tools module for vkontakte

pub mod keyboard;
pub mod uploaders;
pub mod formatting;
pub mod template;
pub mod mini_types;
pub mod fsm;
pub mod rate_limiter;
pub mod ctx_storage;
pub mod vbml;
pub mod markdown;
pub mod event_data;
pub mod utils;
pub mod waiter;
pub mod mention;
pub mod scheduling;
pub mod conversation_cache;
pub mod message_builder;
pub mod validators;
pub mod attachment_parser;
pub mod loop_wrapper;
pub mod auth;
pub mod limited_dict;

pub use keyboard::*;
pub use uploaders::*;
pub use formatting::*;
pub use template::*;
pub use mini_types::*;
pub use fsm::*;
pub use rate_limiter::*;
pub use ctx_storage::*;
pub use vbml::*;
pub use markdown::*;
pub use event_data::*;
pub use utils::*;
pub use waiter::*;
pub use mention::*;
pub use scheduling::*;
pub use conversation_cache::*;
pub use message_builder::*;
pub use validators::*;
pub use attachment_parser::*;
pub use loop_wrapper::*;
pub use auth::*;
pub use limited_dict::*;

use serde_json::Value;
use std::collections::HashMap;

/// Attachment types
#[derive(Debug, Clone, PartialEq)]
pub enum AttachmentType {
    Photo,
    Video,
    Audio,
    Doc,
    AudioMessage,
    Graffiti,
    Market,
    MarketAlbum,
    Sticker,
    Gift,
    Link,
    Poll,
    Story,
    Wall,
    WallReply,
    Call,
    Fight,
    MoneyTransfer,
    VKPay,
    Unknown(String),
}

impl AttachmentType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "photo" => Self::Photo,
            "video" => Self::Video,
            "audio" => Self::Audio,
            "doc" => Self::Doc,
            "audio_message" | "audiomessage" => Self::AudioMessage,
            "graffiti" => Self::Graffiti,
            "market" => Self::Market,
            "market_album" | "marketalbum" => Self::MarketAlbum,
            "sticker" => Self::Sticker,
            "gift" => Self::Gift,
            "link" => Self::Link,
            "poll" => Self::Poll,
            "story" => Self::Story,
            "wall" => Self::Wall,
            "wall_reply" | "wallreply" => Self::WallReply,
            "call" => Self::Call,
            "fight" => Self::Fight,
            "money_transfer" | "moneytransfer" => Self::MoneyTransfer,
            "vkpay" => Self::VKPay,
            _ => Self::Unknown(s.to_string()),
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            Self::Photo => "photo",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Doc => "doc",
            Self::AudioMessage => "audio_message",
            Self::Graffiti => "graffiti",
            Self::Market => "market",
            Self::MarketAlbum => "market_album",
            Self::Sticker => "sticker",
            Self::Gift => "gift",
            Self::Link => "link",
            Self::Poll => "poll",
            Self::Story => "story",
            Self::Wall => "wall",
            Self::WallReply => "wall_reply",
            Self::Call => "call",
            Self::Fight => "fight",
            Self::MoneyTransfer => "money_transfer",
            Self::VKPay => "vkpay",
            Self::Unknown(s) => s,
        }
    }
}

/// Attachment structure
#[derive(Debug, Clone)]
pub struct Attachment {
    pub attachment_type: AttachmentType,
    pub owner_id: i64,
    pub id: i64,
    pub access_key: Option<String>,
    pub data: Option<Value>,
}

impl Attachment {
    pub fn new(attachment_type: AttachmentType, owner_id: i64, id: i64) -> Self {
        Self {
            attachment_type,
            owner_id,
            id,
            access_key: None,
            data: None,
        }
    }
    
    pub fn with_access_key(mut self, access_key: String) -> Self {
        self.access_key = Some(access_key);
        self
    }
    
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
    
    /// Convert to attachment string for API
    pub fn to_attachment_string(&self) -> String {
        let mut result = format!("{}_{}", self.owner_id, self.id);
        
        if let Some(ref key) = self.access_key {
            result.push_str(&format!("_{}", key));
        }
        
        format!("{}{}", self.attachment_type.as_str(), result)
    }
    
    /// Parse from API string
    pub fn from_attachment_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.len() < 3 {
            return None;
        }
        
        let attachment_type = AttachmentType::from_str(&parts[0]);
        let owner_id = parts[1].parse::<i64>().ok()?;
        let id = parts[2].parse::<i64>().ok()?;
        
        let mut attachment = Self::new(attachment_type, owner_id, id);
        
        if parts.len() > 3 {
            attachment.access_key = Some(parts[3].to_string());
        }
        
        Some(attachment)
    }
}

/// Chat action types
#[derive(Debug, Clone, PartialEq)]
pub enum ChatAction {
    ChatCreate,
    ChatTitleUpdate,
    ChatPhotoUpdate,
    ChatPhotoRemove,
    ChatInviteUser,
    ChatKickUser,
    ChatPinMessage,
    ChatUnpinMessage,
    ChatInviteUserByLink,
    ChatScheduledMessage,
    Unknown(String),
}

impl ChatAction {
    pub fn from_str(s: &str) -> Self {
        match s {
            "chat_create" => Self::ChatCreate,
            "chat_title_update" => Self::ChatTitleUpdate,
            "chat_photo_update" => Self::ChatPhotoUpdate,
            "chat_photo_remove" => Self::ChatPhotoRemove,
            "chat_invite_user" => Self::ChatInviteUser,
            "chat_kick_user" => Self::ChatKickUser,
            "chat_pin_message" => Self::ChatPinMessage,
            "chat_unpin_message" => Self::ChatUnpinMessage,
            "chat_invite_user_by_link" => Self::ChatInviteUserByLink,
            "chat_scheduled_message" => Self::ChatScheduledMessage,
            _ => Self::Unknown(s.to_string()),
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            Self::ChatCreate => "chat_create",
            Self::ChatTitleUpdate => "chat_title_update",
            Self::ChatPhotoUpdate => "chat_photo_update",
            Self::ChatPhotoRemove => "chat_photo_remove",
            Self::ChatInviteUser => "chat_invite_user",
            Self::ChatKickUser => "chat_kick_user",
            Self::ChatPinMessage => "chat_pin_message",
            Self::ChatUnpinMessage => "chat_unpin_message",
            Self::ChatInviteUserByLink => "chat_invite_user_by_link",
            Self::ChatScheduledMessage => "chat_scheduled_message",
            Self::Unknown(s) => s,
        }
    }
}

/// Geo location structure
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Geo {
    #[serde(rename = "type")]
    pub geo_type: String,
    pub coordinates: String,
    pub place: Option<Value>,
    pub access_key: Option<String>,
}

impl Geo {
    pub fn new(coordinates: String, geo_type: String) -> Self {
        Self {
            geo_type,
            coordinates,
            place: None,
            access_key: None,
        }
    }
    
    pub fn with_place(mut self, place: Value) -> Self {
        self.place = Some(place);
        self
    }
    
    pub fn with_access_key(mut self, access_key: String) -> Self {
        self.access_key = Some(access_key);
        self
    }
}

/// Message payload structure
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Payload {
    pub command: String,
    pub args: HashMap<String, Value>,
}

impl Payload {
    pub fn new(command: String) -> Self {
        Self {
            command,
            args: HashMap::new(),
        }
    }
    
    pub fn with_args(mut self, args: HashMap<String, Value>) -> Self {
        self.args = args;
        self
    }
    
    pub fn add_arg(&mut self, key: String, value: Value) {
        self.args.insert(key, value);
    }
    
    pub fn get_arg(&self, key: &str) -> Option<&Value> {
        self.args.get(key)
    }
    
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    pub fn from_json_string(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// Common message types
pub type MessageId = i64;
pub type PeerId = i64;
pub type UserId = i64;
pub type ChatId = i64;

/// Message flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageFlag {
    Unread,
    Outbox,
    Replied,
    Important,
    Chat,
    Friends,
    Spam,
    Deleted,
    Fixed,
    Media,
    Hidden,
}

impl MessageFlag {
    pub fn from_bit(bit: i32) -> Option<Self> {
        match bit {
            1 => Some(Self::Unread),
            2 => Some(Self::Outbox),
            4 => Some(Self::Replied),
            8 => Some(Self::Important),
            16 => Some(Self::Chat),
            32 => Some(Self::Friends),
            64 => Some(Self::Spam),
            128 => Some(Self::Deleted),
            256 => Some(Self::Fixed),
            512 => Some(Self::Media),
            65536 => Some(Self::Hidden),
            _ => None,
        }
    }
    
    pub fn to_bit(&self) -> i32 {
        match self {
            Self::Unread => 1,
            Self::Outbox => 2,
            Self::Replied => 4,
            Self::Important => 8,
            Self::Chat => 16,
            Self::Friends => 32,
            Self::Spam => 64,
            Self::Deleted => 128,
            Self::Fixed => 256,
            Self::Media => 512,
            Self::Hidden => 65536,
        }
    }
}