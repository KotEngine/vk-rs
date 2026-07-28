//! Smoke tests for public API surface

use std::sync::Arc;

use vkontakte::api::{Api, MethodPrefixRateLimiter};
use vkontakte::callback::{BotCallback, Callback, CallbackConfig, CallbackSettingsBuilder};
use vkontakte::dispatch::event_types::VkEventType;
use vkontakte::dispatch::middlewares::WaiterMiddleware;
use vkontakte::framework::BotBuilder;
use vkontakte::tools::loop_wrapper::LoopRunner;
use vkontakte::tools::auth::build_implicit_auth_url;
use vkontakte::tools::formatting::strikethrough;
use vkontakte::tools::waiter::WaiterMachine;
use vkontakte::tools::CtxStorage;

#[test]
fn event_type_catalog() {
    assert_eq!(VkEventType::MessageNew.as_str(), "message_new");
}

#[test]
fn callback_settings_builder() {
    let _ = CallbackSettingsBuilder::new().enable_all_messages().build();
}

#[test]
fn ctx_storage_roundtrip() {
    let ctx = CtxStorage::new();
    ctx.insert(42i32);
    assert_eq!(*ctx.get::<i32>().unwrap(), 42);
}

#[test]
fn auth_url_builds() {
    let url = build_implicit_auth_url(1, "https://example.com", &["messages"], None);
    assert!(url.starts_with("https://oauth.vk.com/authorize"));
}

#[test]
fn strikethrough_format() {
    let fmt = strikethrough("deleted");
    let (text, _) = fmt.render();
    assert_eq!(text, "deleted");
}

#[tokio::test]
async fn bot_builder_creates() {
    let bot = BotBuilder::new("token")
        .group_id(1)
        .build()
        .await
        .unwrap();
    assert_eq!(bot.group_id(), 1);
}

#[test]
fn method_rate_limiter_prefix() {
    let _ = MethodPrefixRateLimiter::for_vk_defaults();
}

#[test]
fn waiter_middleware_type_checks() {
    let m = WaiterMiddleware::for_messages(Arc::new(WaiterMachine::new()));
    let _ = m;
}

#[test]
fn loop_runner_default() {
    let _ = LoopRunner::new();
}

#[tokio::test]
async fn api_with_vk_rate_limits() {
    let api = Api::new("dummy").unwrap().with_vk_rate_limits();
    let _ = api;
}

#[test]
fn callback_trait_object() {
    let api = Arc::new(Api::new("dummy").unwrap());
    let cb = BotCallback::new(
        CallbackConfig::new(1, "s", "c", "https://example.com"),
        api,
    );
    let _: &dyn Callback = &cb;
}
