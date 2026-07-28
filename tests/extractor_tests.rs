//! Extractor-based handlers (`handle_with`) driven through a real dispatch.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use vkontakte::dispatch::extractors::{Ctx, Event, Payload, Peer, Sender, State, Text};
use vkontakte::dispatch::rules::{PayloadContainsRule, TextRule};
use vkontakte::dispatch::Router;
use vkontakte::framework::{Bot, BotBlueprint};
use vkontakte::prelude::*;

/// Shared state injected via `State<T>`.
#[derive(Default)]
struct Counter(AtomicI64);

impl Counter {
    fn bump(&self, by: i64) -> i64 {
        self.0.fetch_add(by, Ordering::SeqCst) + by
    }

    fn get(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn message(text: &str, peer_id: i64, from_id: i64) -> Value {
    json!({
        "type": "message_new",
        "object": { "message": {
            "id": 1, "peer_id": peer_id, "from_id": from_id, "date": 0, "text": text
        }}
    })
}

async fn dispatch(bot: &mut Bot, event: Value) -> Value {
    bot.sync_router();
    let router = bot.router();
    let api = bot.api.clone();
    router.route(&event, &api, None).await.expect("dispatch ok")
}

#[tokio::test]
async fn extracts_peer_sender_and_text() {
    async fn handler(
        Peer(peer): Peer,
        Sender(sender): Sender,
        Text(text): Text,
    ) -> DispatchResult<Option<Value>> {
        Ok(Some(json!({ "peer": peer, "sender": sender, "text": text })))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle_with(handler);

    let out = dispatch(&mut bot, message("ping", 100, 42)).await;

    assert_eq!(out, json!({"peer": 100, "sender": 42, "text": "ping"}));
}

#[tokio::test]
async fn state_is_injected_from_ctx_storage() {
    async fn handler(State(counter): State<Counter>) -> DispatchResult<Option<Value>> {
        Ok(Some(json!(counter.bump(5))))
    }

    let mut bot = Bot::new("dummy").unwrap();
    let counter = Arc::new(Counter::default());
    bot.ctx_storage.insert_arc(counter.clone());

    bot.on()
        .message(Box::new(TextRule::new("count", false)))
        .handle_with(handler);

    assert_eq!(dispatch(&mut bot, message("count", 1, 1)).await, json!(5));
    assert_eq!(dispatch(&mut bot, message("count", 1, 1)).await, json!(10));
    assert_eq!(counter.get(), 10);
}

#[tokio::test]
async fn unregistered_state_fails_with_a_named_error() {
    async fn handler(State(_c): State<Counter>) -> DispatchResult<Option<Value>> {
        Ok(Some(json!("unreachable")))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("count", false)))
        .handle_with(handler);
    bot.sync_router();

    let router = bot.router();
    let api = bot.api.clone();
    let err = router
        .route(&message("count", 1, 1), &api, None)
        .await
        .expect_err("missing state must fail");

    assert!(err.to_string().contains("Counter"), "{err}");
}

#[tokio::test]
async fn message_min_and_raw_event_extract_together() {
    async fn handler(msg: MessageMin, Event(raw): Event) -> DispatchResult<Option<Value>> {
        Ok(Some(json!({
            "peer": msg.peer_id,
            "type": raw.get("type").and_then(|t| t.as_str()).unwrap_or(""),
        })))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("hi", false)))
        .handle_with(handler);

    let out = dispatch(&mut bot, message("hi", 77, 5)).await;

    assert_eq!(out, json!({"peer": 77, "type": "message_new"}));
}

#[tokio::test]
async fn rules_still_gate_extractor_handlers() {
    async fn handler(Text(text): Text) -> DispatchResult<Option<Value>> {
        Ok(Some(json!(text)))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle_with(handler);

    // Text does not match the rule — nothing handles it.
    let out = dispatch(&mut bot, message("nope", 1, 1)).await;

    assert_eq!(out, Value::Null);
}

#[tokio::test]
async fn optional_extractor_absorbs_missing_field() {
    async fn handler(peer: Option<Peer>) -> DispatchResult<Option<Value>> {
        Ok(Some(json!(peer.map(|Peer(p)| p))))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("x", false)))
        .handle_with(handler);

    assert_eq!(dispatch(&mut bot, message("x", 9, 9)).await, json!(9));
}

#[tokio::test]
async fn message_event_handlers_extract_payload() {
    async fn handler(Payload(payload): Payload, Peer(peer): Peer) -> DispatchResult<Option<Value>> {
        Ok(Some(json!({ "payload": payload, "peer": peer })))
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message_event(Box::new(PayloadContainsRule::new("action", json!("buy"))))
        .handle_with(handler);

    let event = json!({
        "type": "message_event",
        "object": { "user_id": 3, "peer_id": 500, "event_id": "e", "payload": {"action": "buy"} }
    });
    let out = dispatch(&mut bot, event).await;

    assert_eq!(
        out,
        json!({"payload": {"action": "buy"}, "peer": 500})
    );
}

#[tokio::test]
async fn extractor_handlers_work_inside_blueprints() {
    async fn handler(Peer(peer): Peer) -> DispatchResult<Option<Value>> {
        Ok(Some(json!(peer)))
    }

    let mut bp = BotBlueprint::new().with_name("mod");
    bp.on()
        .message(Box::new(TextRule::new("bp", false)))
        .handle_with(handler);

    let mut bot = Bot::new("dummy").unwrap();
    bot.mount(bp);

    assert_eq!(dispatch(&mut bot, message("bp", 321, 1)).await, json!(321));
}

#[tokio::test]
async fn rule_context_reaches_the_handler() {
    async fn handler(Ctx(ctx): Ctx) -> DispatchResult<Option<Value>> {
        Ok(Some(json!(ctx.contains_key("action"))))
    }

    let mut bot = Bot::new("dummy").unwrap();
    // PayloadMapRule returns the payload map as rule context.
    bot.on()
        .message_event(Box::new(
            vkontakte::dispatch::rules::PayloadMapRule::required_keys(vec!["action".to_string()]),
        ))
        .handle_with(handler);

    let event = json!({
        "type": "message_event",
        "object": { "user_id": 1, "peer_id": 1, "event_id": "e", "payload": {"action": "go"} }
    });

    assert_eq!(dispatch(&mut bot, event).await, json!(true));
}

#[tokio::test]
async fn extractor_routes_show_up_in_dump_routes() {
    async fn handler(Peer(_p): Peer) -> DispatchResult<Option<Value>> {
        Ok(None)
    }

    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle_with(handler);

    let report = bot.format_routes();

    assert!(report.contains("MessageView"), "{report}");
    assert!(report.contains("TextRule"), "{report}");
    assert!(report.contains("Total handlers: 1"), "{report}");
}
