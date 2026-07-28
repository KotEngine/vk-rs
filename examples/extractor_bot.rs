//! Handlers that declare their dependencies as arguments (`handle_with`).
//!
//! ```bash
//! cargo run --example extractor_bot
//! ```

use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::Value;
use vkontakte::dispatch::extractors::{Payload, Peer, Sender, State, Text};
use vkontakte::prelude::*;

/// Anything can go in the bot's shared state — a pool, a config, a counter.
#[derive(Default)]
struct Stats {
    seen: AtomicI64,
}

async fn ping(msg: MessageMin, State(stats): State<Stats>) -> DispatchResult<Option<Value>> {
    let seen = stats.seen.fetch_add(1, Ordering::Relaxed) + 1;
    msg.answer(&format!("pong #{seen}")).await.map(Some)
}

/// Only the pieces this handler actually needs — no `MessageMin`, no context map.
async fn whoami(
    msg: MessageMin,
    Peer(peer_id): Peer,
    Sender(user_id): Sender,
    Text(text): Text,
) -> DispatchResult<Option<Value>> {
    msg.answer(&format!("peer={peer_id} user={user_id} text={text:?}"))
        .await
        .map(Some)
}

/// Callback buttons get their payload the same way.
async fn on_button(
    Payload(payload): Payload,
    Peer(peer_id): Peer,
) -> DispatchResult<Option<Value>> {
    tracing::info!(%peer_id, %payload, "button pressed");
    Ok(None)
}

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("VK_TOKEN").expect("set VK_TOKEN env var");
    let group_id: i64 = std::env::var("VK_GROUP_ID")
        .expect("set VK_GROUP_ID env var")
        .parse()
        .expect("VK_GROUP_ID must be integer");

    let mut bot = Bot::new(&token)?.with_group_id(group_id);

    // Register once at startup; extractors resolve it per update.
    bot.ctx_storage.insert(Stats::default());

    bot.on()
        .message(Box::new(TextRule::new("ping", true)))
        .handle_with(ping);

    bot.on()
        .message(Box::new(CommandRule::new("whoami", vec!["/"], None)))
        .handle_with(whoami);

    bot.on()
        .message_event(Box::new(PayloadHasKeyRule::new("action")))
        .handle_with(on_button);

    // Print what actually got registered before going live.
    bot.dump_routes();

    bot.run_polling().await
}
