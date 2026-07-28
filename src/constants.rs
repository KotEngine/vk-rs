//! VK-wide constants shared across the crate.
//!
//! Kept in one place so a VK-side change is a one-line edit rather than a
//! grep-and-replace across the crate.

/// VK API version every request is sent with.
pub const VK_API_VERSION: &str = "5.199";

/// Base URL for VK API methods (trailing slash included).
pub const VK_API_URL: &str = "https://api.vk.com/method/";

/// OAuth authorize endpoint.
pub const VK_OAUTH_URL: &str = "https://oauth.vk.com/authorize";

/// OAuth token endpoint.
pub const VK_OAUTH_TOKEN_URL: &str = "https://oauth.vk.com/access_token";

/// Long poll protocol version requested from VK.
pub const DEFAULT_LONGPOLL_VERSION: u8 = 3;

/// Official VK Android app credentials, used for user auth when the caller does
/// not supply their own application.
pub const MOBILE_APP_ID: i64 = 2274003;
/// Companion secret for [`MOBILE_APP_ID`].
pub const MOBILE_APP_SECRET: &str = "hHbZxrka2uZ6jB1inYsH";

/// Lowest `peer_id` that belongs to a group chat — anything above is a chat,
/// anything below is a private conversation.
pub const CHAT_PEER_ID_OFFSET: i64 = 2_000_000_000;

/// Why VK told a long poll request to back off.
///
/// Sent by VK as the `failed` field of a long poll response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    /// `ts` is stale — VK returned a fresh one to resume from.
    HistoryOutdated = 1,
    /// The long poll key expired; request a new server.
    KeyExpired = 2,
    /// User information was lost; request a new server and key.
    InformationLost = 3,
    /// Requested long poll version is out of the supported range.
    InvalidVersion = 4,
}

impl FailureCode {
    /// Map VK's numeric `failed` value onto a known code.
    pub fn from_code(code: i64) -> Option<Self> {
        match code {
            1 => Some(Self::HistoryOutdated),
            2 => Some(Self::KeyExpired),
            3 => Some(Self::InformationLost),
            4 => Some(Self::InvalidVersion),
            _ => None,
        }
    }

    pub fn as_code(self) -> i64 {
        self as i64
    }

    /// Whether recovering requires fetching a new long poll server.
    pub fn needs_new_server(self) -> bool {
        !matches!(self, Self::HistoryOutdated)
    }

    /// How a poller should recover from this failure.
    pub fn recovery(self) -> FailureRecovery {
        match self {
            // VK sent a fresh `ts` in the same response; the key is still good.
            Self::HistoryOutdated => FailureRecovery::KeepServer,
            // New key needed, but history survived — resume from the same `ts`.
            Self::KeyExpired => FailureRecovery::NewServerKeepTs,
            // History is gone, and a stale version must restart from scratch.
            Self::InformationLost | Self::InvalidVersion => FailureRecovery::NewServerResetTs,
        }
    }
}

/// What to do after VK reports a long poll failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureRecovery {
    /// Carry on with the current server and `ts`.
    KeepServer,
    /// Request a new server but resume from the current `ts`.
    NewServerKeepTs,
    /// Request a new server and adopt the `ts` it returns.
    NewServerResetTs,
}

/// Bit flags on a message in the user long poll stream.
///
/// See <https://dev.vk.com/api/user-long-poll/getting-started#Флаги сообщений>.
pub mod message_flags {
    /// Message has not been read.
    pub const UNREAD: i64 = 1;
    /// Message was sent by the account itself.
    pub const OUTBOX: i64 = 2;
    /// Replied to.
    pub const REPLIED: i64 = 4;
    /// Marked important.
    pub const IMPORTANT: i64 = 8;
    /// Sent through the chat interface.
    pub const CHAT: i64 = 16;
    /// Sent by a friend.
    pub const FRIENDS: i64 = 32;
    /// Marked as spam.
    pub const SPAM: i64 = 64;
    /// Deleted locally.
    pub const DELETED: i64 = 128;
    /// Checked for spam.
    pub const FIXED: i64 = 256;
    /// Contains attachments.
    pub const MEDIA: i64 = 512;
    /// Greeting sticker from a community.
    pub const HIDDEN: i64 = 65536;
    /// Deleted for everyone.
    pub const DELETED_FOR_ALL: i64 = 131_072;
    /// Not delivered.
    pub const NOT_DELIVERED: i64 = 262_144;
}

/// Event codes in the user long poll stream.
///
/// See <https://dev.vk.com/api/user-long-poll/getting-started>.
pub mod user_lp_events {
    /// Message flags replaced.
    pub const MESSAGE_FLAGS_REPLACE: i64 = 2;
    /// Message flags set.
    pub const MESSAGE_FLAGS_SET: i64 = 3;
    /// New message.
    pub const NEW_MESSAGE: i64 = 4;
    /// Message edited.
    pub const MESSAGE_EDIT: i64 = 5;
    /// Incoming messages read up to a given id.
    pub const READ_INBOUND: i64 = 6;
    /// Outgoing messages read up to a given id.
    pub const READ_OUTBOUND: i64 = 7;
    /// A friend came online.
    pub const FRIEND_ONLINE: i64 = 8;
    /// A friend went offline.
    pub const FRIEND_OFFLINE: i64 = 9;
    /// Conversation flags reset.
    pub const CHAT_FLAGS_RESET: i64 = 10;
    /// Conversation flags replaced.
    pub const CHAT_FLAGS_REPLACE: i64 = 11;
    /// Conversation flags set.
    pub const CHAT_FLAGS_SET: i64 = 12;
    /// Conversation removed.
    pub const CHAT_DELETED: i64 = 13;
    /// Conversation parameters changed.
    pub const CHAT_CHANGED: i64 = 51;
    /// Conversation info changed (title, avatar, pin, ...).
    pub const CHAT_INFO_CHANGED: i64 = 52;
    /// User is typing in a private conversation.
    pub const USER_TYPING: i64 = 61;
    /// User is typing in a chat.
    pub const USER_TYPING_IN_CHAT: i64 = 62;
    /// Users are typing in a conversation.
    pub const USERS_TYPING_IN_CHAT: i64 = 63;
    /// Users are recording a voice message.
    pub const USERS_RECORDING_VOICE: i64 = 64;
    /// User made a call.
    pub const USER_CALL: i64 = 70;
    /// Unread dialog counter changed.
    pub const COUNTER_CHANGED: i64 = 80;
    /// Notification settings changed.
    pub const NOTIFICATION_SETTINGS_CHANGED: i64 = 114;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_codes_round_trip() {
        for code in 1..=4 {
            let parsed = FailureCode::from_code(code).expect("known code");
            assert_eq!(parsed.as_code(), code);
        }
        assert!(FailureCode::from_code(0).is_none());
        assert!(FailureCode::from_code(5).is_none());
    }

    #[test]
    fn only_history_outdated_reuses_server() {
        assert!(!FailureCode::HistoryOutdated.needs_new_server());
        assert!(FailureCode::KeyExpired.needs_new_server());
        assert!(FailureCode::InformationLost.needs_new_server());
        assert!(FailureCode::InvalidVersion.needs_new_server());
    }
}
