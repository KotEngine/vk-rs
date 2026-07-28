//! VK Callback / Long Poll event type catalog

/// Known VK bot event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VkEventType {
    Confirmation,
    MessageNew,
    MessageReply,
    MessageEdit,
    MessageAllow,
    MessageDeny,
    MessageTypingState,
    MessageRead,
    MessageEvent,
    PhotoNew,
    PhotoCommentNew,
    PhotoCommentEdit,
    PhotoCommentRestore,
    PhotoCommentDelete,
    AudioNew,
    VideoNew,
    VideoCommentNew,
    VideoCommentEdit,
    VideoCommentRestore,
    VideoCommentDelete,
    WallPostNew,
    WallRepost,
    WallReplyNew,
    WallReplyEdit,
    WallReplyRestore,
    WallReplyDelete,
    BoardPostNew,
    BoardPostEdit,
    BoardPostRestore,
    BoardPostDelete,
    MarketCommentNew,
    MarketCommentEdit,
    MarketCommentRestore,
    MarketCommentDelete,
    GroupLeave,
    GroupJoin,
    UserBlock,
    UserUnblock,
    LeadFormsNew,
    DonutSubscriptionCreate,
    DonutSubscriptionProlonged,
    DonutSubscriptionCancelled,
    DonutSubscriptionExpired,
    DonutSubscriptionPriceChanged,
    DonutMoneyWithdraw,
    DonutMoneyWithdrawTransaction,
    LikeAdd,
    LikeRemove,
    PollVoteNew,
    GroupChangeSettings,
    GroupChangePhoto,
    VkpayTransaction,
    AppPayload,
    Unknown,
}

impl VkEventType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "confirmation" => Self::Confirmation,
            "message_new" => Self::MessageNew,
            "message_reply" => Self::MessageReply,
            "message_edit" => Self::MessageEdit,
            "message_allow" => Self::MessageAllow,
            "message_deny" => Self::MessageDeny,
            "message_typing_state" => Self::MessageTypingState,
            "message_read" => Self::MessageRead,
            "message_event" => Self::MessageEvent,
            "photo_new" => Self::PhotoNew,
            "photo_comment_new" => Self::PhotoCommentNew,
            "photo_comment_edit" => Self::PhotoCommentEdit,
            "photo_comment_restore" => Self::PhotoCommentRestore,
            "photo_comment_delete" => Self::PhotoCommentDelete,
            "audio_new" => Self::AudioNew,
            "video_new" => Self::VideoNew,
            "video_comment_new" => Self::VideoCommentNew,
            "video_comment_edit" => Self::VideoCommentEdit,
            "video_comment_restore" => Self::VideoCommentRestore,
            "video_comment_delete" => Self::VideoCommentDelete,
            "wall_post_new" => Self::WallPostNew,
            "wall_repost" => Self::WallRepost,
            "wall_reply_new" => Self::WallReplyNew,
            "wall_reply_edit" => Self::WallReplyEdit,
            "wall_reply_restore" => Self::WallReplyRestore,
            "wall_reply_delete" => Self::WallReplyDelete,
            "board_post_new" => Self::BoardPostNew,
            "board_post_edit" => Self::BoardPostEdit,
            "board_post_restore" => Self::BoardPostRestore,
            "board_post_delete" => Self::BoardPostDelete,
            "market_comment_new" => Self::MarketCommentNew,
            "market_comment_edit" => Self::MarketCommentEdit,
            "market_comment_restore" => Self::MarketCommentRestore,
            "market_comment_delete" => Self::MarketCommentDelete,
            "group_leave" => Self::GroupLeave,
            "group_join" => Self::GroupJoin,
            "user_block" => Self::UserBlock,
            "user_unblock" => Self::UserUnblock,
            "lead_forms_new" => Self::LeadFormsNew,
            "donut_subscription_create" => Self::DonutSubscriptionCreate,
            "donut_subscription_prolonged" => Self::DonutSubscriptionProlonged,
            "donut_subscription_cancelled" => Self::DonutSubscriptionCancelled,
            "donut_subscription_expired" => Self::DonutSubscriptionExpired,
            "donut_subscription_price_changed" => Self::DonutSubscriptionPriceChanged,
            "donut_money_withdraw" => Self::DonutMoneyWithdraw,
            "donut_money_withdraw_transaction" => Self::DonutMoneyWithdrawTransaction,
            "like_add" => Self::LikeAdd,
            "like_remove" => Self::LikeRemove,
            "poll_vote_new" => Self::PollVoteNew,
            "group_change_settings" => Self::GroupChangeSettings,
            "group_change_photo" => Self::GroupChangePhoto,
            "vkpay_transaction" => Self::VkpayTransaction,
            "app_payload" => Self::AppPayload,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmation => "confirmation",
            Self::MessageNew => "message_new",
            Self::MessageReply => "message_reply",
            Self::MessageEdit => "message_edit",
            Self::MessageAllow => "message_allow",
            Self::MessageDeny => "message_deny",
            Self::MessageTypingState => "message_typing_state",
            Self::MessageRead => "message_read",
            Self::MessageEvent => "message_event",
            Self::PhotoNew => "photo_new",
            Self::PhotoCommentNew => "photo_comment_new",
            Self::PhotoCommentEdit => "photo_comment_edit",
            Self::PhotoCommentRestore => "photo_comment_restore",
            Self::PhotoCommentDelete => "photo_comment_delete",
            Self::AudioNew => "audio_new",
            Self::VideoNew => "video_new",
            Self::VideoCommentNew => "video_comment_new",
            Self::VideoCommentEdit => "video_comment_edit",
            Self::VideoCommentRestore => "video_comment_restore",
            Self::VideoCommentDelete => "video_comment_delete",
            Self::WallPostNew => "wall_post_new",
            Self::WallRepost => "wall_repost",
            Self::WallReplyNew => "wall_reply_new",
            Self::WallReplyEdit => "wall_reply_edit",
            Self::WallReplyRestore => "wall_reply_restore",
            Self::WallReplyDelete => "wall_reply_delete",
            Self::BoardPostNew => "board_post_new",
            Self::BoardPostEdit => "board_post_edit",
            Self::BoardPostRestore => "board_post_restore",
            Self::BoardPostDelete => "board_post_delete",
            Self::MarketCommentNew => "market_comment_new",
            Self::MarketCommentEdit => "market_comment_edit",
            Self::MarketCommentRestore => "market_comment_restore",
            Self::MarketCommentDelete => "market_comment_delete",
            Self::GroupLeave => "group_leave",
            Self::GroupJoin => "group_join",
            Self::UserBlock => "user_block",
            Self::UserUnblock => "user_unblock",
            Self::LeadFormsNew => "lead_forms_new",
            Self::DonutSubscriptionCreate => "donut_subscription_create",
            Self::DonutSubscriptionProlonged => "donut_subscription_prolonged",
            Self::DonutSubscriptionCancelled => "donut_subscription_cancelled",
            Self::DonutSubscriptionExpired => "donut_subscription_expired",
            Self::DonutSubscriptionPriceChanged => "donut_subscription_price_changed",
            Self::DonutMoneyWithdraw => "donut_money_withdraw",
            Self::DonutMoneyWithdrawTransaction => "donut_money_withdraw_transaction",
            Self::LikeAdd => "like_add",
            Self::LikeRemove => "like_remove",
            Self::PollVoteNew => "poll_vote_new",
            Self::GroupChangeSettings => "group_change_settings",
            Self::GroupChangePhoto => "group_change_photo",
            Self::VkpayTransaction => "vkpay_transaction",
            Self::AppPayload => "app_payload",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_message_related(self) -> bool {
        matches!(
            self,
            Self::MessageNew
                | Self::MessageReply
                | Self::MessageEdit
                | Self::MessageAllow
                | Self::MessageDeny
                | Self::MessageTypingState
                | Self::MessageRead
                | Self::MessageEvent
        )
    }

    pub fn is_wall_related(self) -> bool {
        matches!(
            self,
            Self::WallPostNew
                | Self::WallRepost
                | Self::WallReplyNew
                | Self::WallReplyEdit
                | Self::WallReplyRestore
                | Self::WallReplyDelete
        )
    }

    pub fn is_comment_related(self) -> bool {
        matches!(
            self,
            Self::PhotoCommentNew
                | Self::PhotoCommentEdit
                | Self::PhotoCommentRestore
                | Self::PhotoCommentDelete
                | Self::VideoCommentNew
                | Self::VideoCommentEdit
                | Self::VideoCommentRestore
                | Self::VideoCommentDelete
                | Self::MarketCommentNew
                | Self::MarketCommentEdit
                | Self::MarketCommentRestore
                | Self::MarketCommentDelete
        )
    }

    pub fn category(self) -> EventCategory {
        if self == Self::Confirmation {
            return EventCategory::System;
        }
        if self.is_message_related() {
            return EventCategory::Messages;
        }
        if self.is_wall_related() {
            return EventCategory::Wall;
        }
        if self.is_comment_related() {
            return EventCategory::Comments;
        }
        match self {
            Self::PhotoNew | Self::AudioNew | Self::VideoNew => EventCategory::Media,
            Self::GroupLeave | Self::GroupJoin | Self::UserBlock | Self::UserUnblock => {
                EventCategory::Community
            }
            Self::LikeAdd | Self::LikeRemove | Self::PollVoteNew => EventCategory::Engagement,
            Self::DonutSubscriptionCreate
            | Self::DonutSubscriptionProlonged
            | Self::DonutSubscriptionCancelled
            | Self::DonutSubscriptionExpired
            | Self::DonutSubscriptionPriceChanged
            | Self::DonutMoneyWithdraw
            | Self::DonutMoneyWithdrawTransaction => EventCategory::Donut,
            Self::VkpayTransaction => EventCategory::Payments,
            Self::AppPayload => EventCategory::Apps,
            Self::LeadFormsNew => EventCategory::Leads,
            Self::GroupChangeSettings | Self::GroupChangePhoto => EventCategory::GroupAdmin,
            Self::BoardPostNew
            | Self::BoardPostEdit
            | Self::BoardPostRestore
            | Self::BoardPostDelete => EventCategory::Board,
            Self::Unknown => EventCategory::Other,
            _ => EventCategory::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    System,
    Messages,
    Wall,
    Comments,
    Media,
    Community,
    Engagement,
    Donut,
    Payments,
    Apps,
    Leads,
    GroupAdmin,
    Board,
    Other,
}

/// Parse event type from raw VK JSON
pub fn event_type_from_value(event: &serde_json::Value) -> VkEventType {
    event
        .get("type")
        .and_then(|t| t.as_str())
        .map(VkEventType::from_str)
        .unwrap_or(VkEventType::Unknown)
}

/// All standard bot event type strings (for callback settings)
pub fn all_bot_event_types() -> Vec<&'static str> {
    use VkEventType::*;
    [
        MessageNew,
        MessageReply,
        MessageEdit,
        MessageAllow,
        MessageDeny,
        MessageTypingState,
        MessageRead,
        MessageEvent,
        PhotoNew,
        PhotoCommentNew,
        PhotoCommentEdit,
        PhotoCommentRestore,
        PhotoCommentDelete,
        AudioNew,
        VideoNew,
        VideoCommentNew,
        VideoCommentEdit,
        VideoCommentRestore,
        VideoCommentDelete,
        WallPostNew,
        WallRepost,
        WallReplyNew,
        WallReplyEdit,
        WallReplyRestore,
        WallReplyDelete,
        BoardPostNew,
        BoardPostEdit,
        BoardPostRestore,
        BoardPostDelete,
        MarketCommentNew,
        MarketCommentEdit,
        MarketCommentRestore,
        MarketCommentDelete,
        GroupLeave,
        GroupJoin,
        UserBlock,
        UserUnblock,
        LeadFormsNew,
        LikeAdd,
        LikeRemove,
        PollVoteNew,
        GroupChangeSettings,
        GroupChangePhoto,
        VkpayTransaction,
        AppPayload,
    ]
    .iter()
    .map(|e| e.as_str())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_message_new() {
        let ev = json!({"type": "message_new"});
        assert_eq!(event_type_from_value(&ev), VkEventType::MessageNew);
    }

    #[test]
    fn message_category() {
        assert_eq!(
            VkEventType::MessageEvent.category(),
            EventCategory::Messages
        );
    }
}
