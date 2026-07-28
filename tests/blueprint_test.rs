//! Tests for the modular blueprint system (`BotBlueprint` / `mount` / `include`).

use serde_json::{json, Value};
use vkontakte::dispatch::middlewares::base::LoggingMiddleware;
use vkontakte::dispatch::rules::{CommandRule, PayloadContainsRule, TextRule};
use vkontakte::framework::{Bot, BotBlueprint};

#[tokio::test]
async fn blueprint_message_handler_is_mounted() {
    let mut bot = Bot::new("dummy").unwrap();

    let mut bp = BotBlueprint::new().with_name("greeter");
    bp.on()
        .message(Box::new(TextRule::new("hello", false)))
        .handle(|_msg, _ctx| async move { Ok(Some(Value::Null)) });

    assert_eq!(bp.handler_count(), 1);
    assert_eq!(bp.name, "greeter");

    bot.mount(bp);
    bot.sync_router();

    // The labeler keeps its handlers after a sync, so syncing again (which
    // `run_polling` does whenever middleware or a blueprint arrives late) still
    // produces a router with every handler in it.
    assert_eq!(bot.labeler.message_handler_count(), 1);
    assert_eq!(bot.routes().len(), 1);

    bot.sync_router();
    assert_eq!(bot.routes().len(), 1);
}

#[test]
fn syncing_twice_keeps_every_handler() {
    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });

    bot.sync_router();
    assert_eq!(bot.routes().len(), 1);

    // Registering after a sync used to wipe the previously synced handlers.
    bot.on()
        .message(Box::new(TextRule::new("pong", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.sync_router();

    assert_eq!(bot.routes().len(), 2);
}

#[test]
fn mounting_after_sync_keeps_earlier_handlers() {
    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("first", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.sync_router();

    let mut late = BotBlueprint::new().with_name("late");
    late.on()
        .message(Box::new(TextRule::new("second", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.mount(late);
    bot.sync_router();

    let report = bot.format_routes();
    assert!(report.contains("first"), "{report}");
    assert!(report.contains("second"), "{report}");
    assert_eq!(bot.routes().len(), 2);
}

#[test]
fn nested_blueprints_flatten_into_parent() {
    let mut parent = BotBlueprint::new().with_name("parent");

    let mut child_a = BotBlueprint::new().with_name("a");
    child_a
        .on()
        .message(Box::new(TextRule::new("a", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    let mut child_b = BotBlueprint::new().with_name("b");
    child_b
        .on()
        .message(Box::new(TextRule::new("b", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });

    parent.include(child_a);
    parent.include(child_b);

    assert_eq!(parent.handler_count(), 2);
}

#[test]
fn deep_nesting_recurses() {
    let mut root = BotBlueprint::new();

    let mut middle = BotBlueprint::new();
    let mut leaf = BotBlueprint::new();
    leaf.on()
        .message(Box::new(CommandRule::new("deep", vec!["/"], None)))
        .handle(|_msg, _ctx| async move { Ok(None) });

    middle.include(leaf);
    root.include(middle);

    assert_eq!(root.handler_count(), 1);
}

#[tokio::test]
async fn blueprint_middleware_forwarded_to_bot() {
    let mut bot = Bot::new("dummy").unwrap();

    let mut bp = BotBlueprint::new().with_name("mw");
    bp.middleware(LoggingMiddleware);
    bp.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });

    bot.mount(bp);
    // Pending middleware is non-empty until sync_router drains it.
    assert_eq!(bot.pending_middleware_len(), 1);
    bot.sync_router();
    assert_eq!(bot.pending_middleware_len(), 0);
}

#[test]
fn raw_value_handler_in_blueprint() {
    let mut bp = BotBlueprint::new();
    bp.on()
        .raw_value(Box::new(TextRule::new("x", false)))
        .handle(|_ev, _ctx| async move { Ok(None) });
    // value_handlers live outside message_handler_count, so check directly.
    assert_eq!(bp.labeler.value_handler_count(), 1);
}

#[test]
fn message_event_handler_in_blueprint() {
    let mut bp = BotBlueprint::new();
    bp.on()
        .message_event(Box::new(PayloadContainsRule::new("action", json!("buy"))))
        .handle(|_ev, _ctx| async move { Ok(None) });
    // Message event handlers are counted separately from message/value handlers.
    assert_eq!(bp.handler_count(), 0);
    assert_eq!(bp.labeler.message_event_handler_count(), 1);
}

#[test]
fn dump_routes_lists_registered_handlers() {
    let mut bot = Bot::new("dummy").unwrap();

    let mut bp = BotBlueprint::new().with_name("admin");
    bp.on()
        .message(Box::new(CommandRule::new("ban", vec!["/"], None)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.mount(bp);

    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.on()
        .message_event(Box::new(PayloadContainsRule::new("action", json!("buy"))))
        .handle(|_ev, _ctx| async move { Ok(None) });
    bot.on()
        .raw_event("wall_post_new")
        .handle(|_ev, _ctx| async move { Ok(None) });

    let report = bot.format_routes();

    assert!(report.contains("MessageView"), "{report}");
    assert!(report.contains("CommandRule"), "{report}");
    assert!(report.contains("TextRule"), "{report}");
    assert!(report.contains("MessageEventView"), "{report}");
    assert!(report.contains("RawEventView"), "{report}");
    assert!(report.contains("wall_post_new"), "{report}");
    assert!(report.contains("Blueprints mounted: admin"), "{report}");
    assert!(report.contains("Total handlers: 4"), "{report}");
}

#[test]
fn routes_are_available_programmatically() {
    let mut bot = Bot::new("dummy").unwrap();
    bot.on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|_msg, _ctx| async move { Ok(None) });
    bot.sync_router();

    let routes = bot.routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].kind, vkontakte::framework::RouteKind::Message);
    assert!(routes[0].rules.contains("TextRule"));
    assert!(routes[0].event_type.is_none());
}
