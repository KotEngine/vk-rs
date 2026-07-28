<div align="center">
  <img src="media/logo.png" alt="vkontakte logo" width="200"/>

  # vkontakte

  > 🦀 Rust library for VK API

  <img src="https://img.shields.io/badge/license-GPL--3.0-orange"> <img src="https://img.shields.io/badge/language-Rust-brown"> <img src="https://img.shields.io/badge/status-WIP-yellow">

  *by [Kot Engine](https://github.com/KotEngine)*
</div>

## Quick start

```rust
use vkontakte::prelude::*;

#[tokio::main]
async fn main() -> Result<(), vkontakte::VkError> {
    let mut bot = Bot::new("TOKEN")?.with_group_id(123456789);

    bot.on()
        .message(Box::new(TextRule::new("hello", false)))
        .handle(|msg, _ctx| async move {
            msg.answer("world").await.map(|v| Some(v))
        });

    bot.run_polling().await
}
```

### FSM

```rust
use vkontakte::prelude::*;
use vkontakte::state_group;

state_group! {
    pub enum MenuState {
        Start = "start",
        Info = "info",
    }
}

bot.on()
    .message(Box::new(StateRule::none()))
    .handle(|msg, _| async move {
        msg.set_state(MenuState::Start).await?;
        Ok(None)
    });

bot.on()
    .message(Box::new(StateRule::new(MenuState::Start)))
    .rule(Box::new(TextRule::new("info", false)))
    .handle(|msg, _| async move {
        msg.answer("info").await.map(|v| Some(v))
    });
```

### Extractors

Handlers can declare what they need as arguments — pulled from the event, the
API client, or typed shared state:

```rust
use vkontakte::dispatch::extractors::{Peer, State};

struct Database;

async fn ping(msg: MessageMin, Peer(peer_id): Peer, State(db): State<Database>)
    -> DispatchResult<Option<Value>>
{
    msg.answer("pong").await.map(Some)
}

bot.ctx_storage.insert(Database);
bot.on()
    .message(Box::new(TextRule::new("ping", false)))
    .handle_with(ping);
```

Available extractors: `MessageMin`, `MessageEventMin`, `Arc<Api>`, `Peer`,
`Sender`, `Text`, `Payload`, `Event`, `Ctx`, `State<T>`, `OptionalState<T>`, and
`Option<T>` around any of them.

### Cooldowns

```rust
use std::time::Duration;

bot.on()
    .message(Box::new(CooldownRule::per_user(Duration::from_secs(5))))
    .rule(Box::new(CommandRule::new("buy", vec!["/"], None)))
    .handle(|msg, _| async move { msg.answer("ok").await.map(Some) });
```

`per_user`, `per_peer` and `global` scopes are available.

### Router introspection

```rust
bot.dump_routes();
```

```text
=== vkontakte Router ===

MessageView
 ├─ [TextRule(ping)]
 └─ [CooldownRule(PerUser, 5s), CommandRule(buy)]

Blueprints mounted: admin
Total handlers: 2
```

### Proc macros

Enable the `macros` feature:

```toml
vkontakte = { version = "0.1", features = ["macros"] }
```

```rust
use vkontakte::{on_message, on_message_event, on_raw_event};

#[on_message(text = "hello")]
async fn hello(msg: MessageMin, _) -> DispatchResult<Option<Value>> {
    msg.answer("hi!").await.map(|v| Some(v))
}

// Arguments combine — every one of them must match.
#[on_message(state = "menu:main", text = "профиль", cooldown_secs = 5)]
async fn profile(msg: MessageMin, _) -> DispatchResult<Option<Value>> { .. }

#[on_message_event(payload = r#"{"action": "buy"}"#)]
async fn on_buy(ev: MessageEventMin, _) -> DispatchResult<Option<Value>> { .. }

#[on_raw_event(event_type = "wall_post_new")]
async fn on_post(ev: Value, _) -> DispatchResult<Option<Value>> { .. }
```

`#[on_message]` accepts `text`, `command`, `regex`, `payload`, `state`,
`no_state`, `ignore_case`, `from_chat`, `cooldown_secs` and `cooldown_scope`.

### Persistent FSM state

```toml
vkontakte = { version = "0.1", features = ["redis"] }    # or "postgres"
```

```rust
let dispenser = PostgresStateDispenser::connect("postgres://localhost/bot").await?;
dispenser.migrate().await?;
let bot = Bot::new(&token)?.with_state_dispenser(Arc::new(dispenser));
```

## Examples

```bash
cargo run --example basic_bot
cargo run --example fsm_bot
cargo run --example extractor_bot
cargo run --example macro_bot --features macros
```

## Build

```bash
cargo test
cargo test --all-targets --features macros,redis,postgres
```

The Redis and Postgres dispenser tests need a live server and are `#[ignore]`d
by default:

```bash
cargo test --features postgres --test postgres_dispenser_test -- --ignored
```

<div align="center">
  <img src="media/stariy_bog.webp" alt="stariy bog" width="150" />
  <img src="media/192.webp" alt="192" width="150" />
</div>
