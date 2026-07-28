//! Extended rule coverage

use serde_json::json;
use vkontakte::dispatch::rules::*;
use vkontakte::dispatch::rules::base::message_text;
use vkontakte::dispatch::RuleResult;

fn msg(text: &str, peer_id: i64, from_id: i64) -> serde_json::Value {
    json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": peer_id,
            "from_id": from_id,
            "text": text,
            "id": 1
        }}
    })
}

#[tokio::test]
async fn regex_rule_matches() {
    let rule = RegexRule::new(r"^\d+$");
    assert!(matches!(
        rule.check(&msg("123", 1, 1)).await,
        RuleResult::Context(_)
    ));
    assert!(matches!(rule.check(&msg("abc", 1, 1)).await, RuleResult::Fail));
}

#[tokio::test]
async fn peer_rule_chat_only() {
    let rule = PeerRule::new(true);
    let chat_peer = 2_000_000_042_i64;
    assert!(matches!(
        rule.check(&msg("hi", chat_peer, 1)).await,
        RuleResult::Pass
    ));
    assert!(matches!(rule.check(&msg("hi", 1, 1)).await, RuleResult::Fail));
}

#[tokio::test]
async fn mention_rule() {
    let rule = MentionRule::new(true);
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "hi",
            "is_mentioned": true
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn from_user_rule() {
    let rule = FromUserRule::new();
    assert!(matches!(rule.check(&msg("x", 1, 5)).await, RuleResult::Pass));
    let group = json!({
        "type": "message_new",
        "object": { "message": { "peer_id": 1, "from_id": -1, "text": "x" }}
    });
    assert!(matches!(rule.check(&group).await, RuleResult::Fail));
}

#[tokio::test]
async fn payload_rule_string() {
    let rule = PayloadRule::new(r#"{"cmd":"start"}"#);
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "payload": r#"{"cmd":"start"}"#
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn reply_message_rule() {
    let rule = ReplyMessageRule::new();
    let with_reply = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "ok",
            "reply_message": { "id": 9, "from_id": 2, "text": "prev" }
        }}
    });
    assert!(matches!(rule.check(&with_reply).await, RuleResult::Pass));
    assert!(matches!(rule.check(&msg("x", 1, 2)).await, RuleResult::Fail));
}

#[tokio::test]
async fn forward_messages_rule() {
    let rule = ForwardMessagesRule::new();
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "fwd_messages": [{ "id": 1, "from_id": 3, "text": "f" }]
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn attachment_type_rule() {
    let rule = AttachmentTypeRule::new("photo");
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "attachments": [{ "type": "photo", "photo": { "id": 1, "owner_id": 1 }}]
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn message_length_rule() {
    let rule = MessageLengthRule::new(3);
    assert!(matches!(rule.check(&msg("1234", 1, 1)).await, RuleResult::Pass));
    assert!(matches!(rule.check(&msg("ab", 1, 1)).await, RuleResult::Fail));
}

#[tokio::test]
async fn levenshtein_rule_close_enough() {
    let rule = LevenshteinRule::new("hello", 1);
    assert!(matches!(rule.check(&msg("hallo", 1, 1)).await, RuleResult::Pass));
}

#[tokio::test]
async fn fuzzy_text_rule() {
    let rule = FuzzyTextRule::new("test", 0.8);
    assert!(matches!(rule.check(&msg("test!", 1, 1)).await, RuleResult::Pass));
}

#[tokio::test]
async fn geo_rule() {
    let rule = GeoRule::new();
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "geo": { "type": "point", "coordinates": "1,2" }
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn chat_action_rule() {
    let rule = ChatActionRule::new(Some("chat_invite_user".to_string()));
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "action": { "type": "chat_invite_user", "member_id": 3 }
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn from_peer_rule() {
    let rule = FromPeerRule::new(vec![100, 200]);
    assert!(matches!(rule.check(&msg("x", 100, 1)).await, RuleResult::Pass));
    assert!(matches!(rule.check(&msg("x", 50, 1)).await, RuleResult::Fail));
}

#[tokio::test]
async fn payload_contains_rule() {
    let rule = PayloadContainsRule::new("cmd", json!("go"));
    let ev = json!({
        "type": "message_new",
        "object": { "message": {
            "peer_id": 1, "from_id": 2, "text": "",
            "payload": r#"{"cmd":"go","n":1}"#
        }}
    });
    assert!(matches!(rule.check(&ev).await, RuleResult::Pass));
}

#[tokio::test]
async fn func_rule_custom() {
    let rule = FuncRule::new(|ev| {
        if message_text(ev).map(|t| t.contains("magic")).unwrap_or(false) {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    });
    assert!(matches!(rule.check(&msg("magic!", 1, 1)).await, RuleResult::Pass));
}
