//! Proc macro registration tests (requires the `macros` feature).
#![cfg(feature = "macros")]

use std::collections::HashMap;

use serde_json::{json, Value};
use vkontakte::framework::Bot;
use vkontakte::prelude::*;
use vkontakte::{on_message, on_message_event, on_raw_event};

#[on_message(text = "hello")]
async fn hello(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message(state = "menu:main", text = "профиль")]
async fn profile(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message(command = "buy", cooldown_secs = 5)]
async fn buy(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message(command = "raid", cooldown_secs = 10, cooldown_scope = "peer")]
async fn raid(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message(regex = r"^\d+$", from_chat = true)]
async fn numbers(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message(no_state = true)]
async fn stateless(_msg: MessageMin, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message_event(payload = r#"{"action":"buy"}"#)]
async fn on_buy(
    _ev: MessageEventMin,
    _ctx: HashMap<String, Value>,
) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_message_event(payload_contains = "action", payload_value = "sell")]
async fn on_sell(
    _ev: MessageEventMin,
    _ctx: HashMap<String, Value>,
) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_raw_event(event_type = "wall_post_new")]
async fn on_wall_post(_ev: Value, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[on_raw_event("group_join")]
async fn on_join(_ev: Value, _ctx: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    Ok(None)
}

#[test]
fn message_macros_register_handlers() {
    let mut bot = Bot::new("dummy").unwrap();

    register_hello(&mut bot);
    register_profile(&mut bot);
    register_buy(&mut bot);
    register_raid(&mut bot);
    register_numbers(&mut bot);
    register_stateless(&mut bot);

    assert_eq!(bot.labeler.message_handler_count(), 6);
}

#[test]
fn event_macros_register_handlers() {
    let mut bot = Bot::new("dummy").unwrap();

    register_on_buy(&mut bot);
    register_on_sell(&mut bot);

    assert_eq!(bot.labeler.message_event_handler_count(), 2);
    assert_eq!(bot.labeler.message_handler_count(), 0);
}

#[test]
fn raw_event_macros_register_handlers() {
    let mut bot = Bot::new("dummy").unwrap();

    register_on_wall_post(&mut bot);
    register_on_join(&mut bot);
    bot.sync_router();

    // Raw handlers live in their own map, not the message list.
    assert_eq!(bot.labeler.message_handler_count(), 0);
}

/// `state` + `text` must produce *both* rules, not just the last one.
#[tokio::test]
async fn combined_state_and_text_rules_both_apply() {
    use vkontakte::dispatch::rules::{Rule, StateRule, TextRule};

    let state_rule = StateRule::new("menu:main");
    let text_rule = TextRule::new("профиль", false);

    let wrong_text = json!({
        "object": { "message": { "text": "другое", "peer_id": 1, "from_id": 1 } }
    });

    // The text rule alone rejects it, which is what the second macro arg buys us.
    assert!(text_rule.check(&wrong_text).await.is_fail());
    assert!(state_rule.description().contains("menu:main"));
}

#[tokio::test]
async fn payload_rule_matches_message_event_object() {
    use vkontakte::dispatch::rules::{PayloadRule, Rule};

    let rule = PayloadRule::new(r#"{"action":"buy"}"#);
    let event = json!({
        "type": "message_event",
        "object": { "user_id": 1, "peer_id": 1, "payload": {"action": "buy"} }
    });

    assert!(rule.check(&event).await.is_pass());
}

#[tokio::test]
async fn payload_rule_matches_message_keyboard_payload() {
    use vkontakte::dispatch::rules::{PayloadRule, Rule};

    let rule = PayloadRule::new(r#"{"action":"buy"}"#);
    // Key order differs from the pattern — structural compare must still match.
    let event = json!({
        "object": { "message": { "peer_id": 1, "payload": "{\"action\": \"buy\"}" } }
    });

    assert!(rule.check(&event).await.is_pass());
}

#[tokio::test]
async fn payload_rule_rejects_other_payloads() {
    use vkontakte::dispatch::rules::{PayloadRule, Rule};

    let rule = PayloadRule::new(r#"{"action":"buy"}"#);
    let event = json!({
        "type": "message_event",
        "object": { "user_id": 1, "peer_id": 1, "payload": {"action": "sell"} }
    });

    assert!(rule.check(&event).await.is_fail());
}

#[tokio::test]
async fn payload_has_key_ignores_value() {
    use vkontakte::dispatch::rules::{PayloadHasKeyRule, Rule};

    let rule = PayloadHasKeyRule::new("action");
    let event = json!({
        "type": "message_event",
        "object": { "user_id": 1, "peer_id": 1, "payload": {"action": "anything"} }
    });
    let without = json!({
        "type": "message_event",
        "object": { "user_id": 1, "peer_id": 1, "payload": {"other": 1} }
    });

    assert!(rule.check(&event).await.is_pass());
    assert!(rule.check(&without).await.is_fail());
}
