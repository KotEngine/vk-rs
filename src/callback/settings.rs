//! Helpers for `groups.setCallbackSettings`

use std::collections::HashMap;

/// VK callback event toggles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackEvent {
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
}

impl CallbackEvent {
    pub fn api_key(self) -> &'static str {
        match self {
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
        }
    }
}

/// Fluent builder for callback settings params
#[derive(Debug, Clone, Default)]
pub struct CallbackSettingsBuilder {
    params: HashMap<String, String>,
}

impl CallbackSettingsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enable(mut self, event: CallbackEvent) -> Self {
        self.params
            .insert(event.api_key().to_string(), "1".to_string());
        self
    }

    pub fn disable(mut self, event: CallbackEvent) -> Self {
        self.params
            .insert(event.api_key().to_string(), "0".to_string());
        self
    }

    pub fn enable_all_messages(self) -> Self {
        self.enable(CallbackEvent::MessageNew)
            .enable(CallbackEvent::MessageReply)
            .enable(CallbackEvent::MessageEdit)
            .enable(CallbackEvent::MessageEvent)
    }

    pub fn api_version(mut self, version: &str) -> Self {
        self.params
            .insert("api_version".to_string(), version.to_string());
        self
    }

    pub fn build(self) -> HashMap<String, String> {
        self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_builder_enables_events() {
        let map = CallbackSettingsBuilder::new()
            .enable(CallbackEvent::MessageNew)
            .enable(CallbackEvent::MessageEvent)
            .build();
        assert_eq!(map.get("message_new").map(String::as_str), Some("1"));
        assert_eq!(map.get("message_event").map(String::as_str), Some("1"));
    }
}
