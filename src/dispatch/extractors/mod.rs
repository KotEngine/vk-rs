//! Dependency-injected handler arguments.
//!
//! Instead of taking a fixed `(MessageMin, HashMap<String, Value>)` pair, a
//! handler can declare exactly what it needs and have it pulled from the event,
//! the bot's API client, or the typed [`CtxStorage`]:
//!
//! ```no_run
//! use vkontakte::dispatch::extractors::{Peer, State};
//! use vkontakte::prelude::*;
//! use serde_json::Value;
//!
//! struct Database;
//!
//! async fn ping(
//!     msg: MessageMin,
//!     Peer(peer_id): Peer,
//!     State(_db): State<Database>,
//! ) -> DispatchResult<Option<Value>> {
//!     let _ = peer_id;
//!     msg.answer("pong").await.map(Some)
//! }
//!
//! # fn demo(bot: &mut Bot) {
//! bot.ctx_storage.insert(Database);
//! bot.on()
//!     .message(Box::new(TextRule::new("ping", false)))
//!     .handle_with(ping);
//! # }
//! ```
//!
//! This sits alongside the plain `handle` path — existing handlers keep working
//! unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::DispatchResult;
use crate::exception::VkError;
use crate::tools::ctx_storage::CtxStorage;
use crate::tools::mini_types::{MessageEventMin, MessageMin};

pub mod handler;
pub use handler::{ExtractFuncHandler, ExtractHandler};

/// Everything an extractor may draw from.
#[derive(Clone)]
pub struct ExtractContext {
    /// Raw VK update.
    pub event: Value,
    /// API client bound to this bot.
    pub api: Arc<Api>,
    /// Typed shared state registered on the bot.
    pub storage: Arc<CtxStorage>,
    /// Context accumulated by rules that matched this event.
    pub rule_context: HashMap<String, Value>,
}

impl ExtractContext {
    pub fn new(
        event: Value,
        api: Arc<Api>,
        storage: Arc<CtxStorage>,
        rule_context: HashMap<String, Value>,
    ) -> Self {
        Self {
            event,
            api,
            storage,
            rule_context,
        }
    }
}

/// A type that can be built from an [`ExtractContext`].
#[async_trait]
pub trait FromEventContext: Sized {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self>;
}

// --- Core extractors -------------------------------------------------------

#[async_trait]
impl FromEventContext for ExtractContext {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(ctx.clone())
    }
}

#[async_trait]
impl FromEventContext for MessageMin {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        MessageMin::from_raw_event(&ctx.event, ctx.api.clone()).map_err(Into::into)
    }
}

#[async_trait]
impl FromEventContext for MessageEventMin {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        MessageEventMin::from_raw_event(&ctx.event, ctx.api.clone()).map_err(Into::into)
    }
}

#[async_trait]
impl FromEventContext for Arc<Api> {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(ctx.api.clone())
    }
}

/// The raw update, untouched.
pub struct Event(pub Value);

#[async_trait]
impl FromEventContext for Event {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(Event(ctx.event.clone()))
    }
}

/// Context collected by the rules that matched.
pub struct Ctx(pub HashMap<String, Value>);

#[async_trait]
impl FromEventContext for Ctx {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(Ctx(ctx.rule_context.clone()))
    }
}

/// `peer_id` of the conversation the update came from.
pub struct Peer(pub i64);

#[async_trait]
impl FromEventContext for Peer {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        peer_id_of(&ctx.event)
            .map(Peer)
            .ok_or_else(|| VkError::Validation("event has no peer_id".to_string()).into())
    }
}

/// `from_id` / `user_id` — who triggered the update.
pub struct Sender(pub i64);

#[async_trait]
impl FromEventContext for Sender {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        sender_id_of(&ctx.event)
            .map(Sender)
            .ok_or_else(|| VkError::Validation("event has no sender id".to_string()).into())
    }
}

/// Message body text.
pub struct Text(pub String);

#[async_trait]
impl FromEventContext for Text {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        crate::dispatch::rules::message_text(&ctx.event)
            .map(|t| Text(t.to_string()))
            .ok_or_else(|| VkError::Validation("event has no text".to_string()).into())
    }
}

/// Payload as JSON, from either a keyboard button or a callback event.
pub struct Payload(pub Value);

#[async_trait]
impl FromEventContext for Payload {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        crate::dispatch::rules::extract_payload_value(&ctx.event)
            .map(Payload)
            .ok_or_else(|| VkError::Validation("event has no payload".to_string()).into())
    }
}

/// Shared state registered on the bot via `bot.ctx_storage.insert(..)`.
///
/// Extraction fails at dispatch time if `T` was never registered — the error
/// names the type, so the cause is obvious in the log.
pub struct State<T: Send + Sync + 'static>(pub Arc<T>);

impl<T: Send + Sync + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        State(self.0.clone())
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> FromEventContext for State<T> {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        ctx.storage.get::<T>().map(State).ok_or_else(|| {
            VkError::Validation(format!(
                "state `{}` is not registered — call bot.ctx_storage.insert(..) at startup",
                std::any::type_name::<T>()
            ))
            .into()
        })
    }
}

/// Like [`State`], but `None` instead of an error when unregistered.
pub struct OptionalState<T: Send + Sync + 'static>(pub Option<Arc<T>>);

#[async_trait]
impl<T: Send + Sync + 'static> FromEventContext for OptionalState<T> {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(OptionalState(ctx.storage.get::<T>()))
    }
}

/// Any extractor becomes optional when wrapped in `Option`.
#[async_trait]
impl<T: FromEventContext> FromEventContext for Option<T> {
    async fn from_ctx(ctx: &ExtractContext) -> DispatchResult<Self> {
        Ok(T::from_ctx(ctx).await.ok())
    }
}

fn peer_id_of(event: &Value) -> Option<i64> {
    crate::dispatch::rules::message_peer_id(event).or_else(|| {
        event
            .get("object")
            .and_then(|o| o.get("peer_id"))
            .and_then(|p| p.as_i64())
    })
}

fn sender_id_of(event: &Value) -> Option<i64> {
    crate::dispatch::rules::message_from_id(event).or_else(|| {
        event
            .get("object")
            .and_then(|o| o.get("user_id"))
            .and_then(|p| p.as_i64())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Database(&'static str);

    fn ctx(event: Value) -> ExtractContext {
        ExtractContext::new(
            event,
            Arc::new(Api::new("dummy").unwrap()),
            Arc::new(CtxStorage::new()),
            HashMap::new(),
        )
    }

    fn message_event() -> Value {
        json!({
            "type": "message_new",
            "object": { "message": {
                "id": 1, "date": 0, "peer_id": 100, "from_id": 42, "text": "hi",
                "payload": "{\"action\":\"buy\"}"
            }}
        })
    }

    #[tokio::test]
    async fn extracts_peer_sender_and_text() {
        let ctx = ctx(message_event());

        assert_eq!(Peer::from_ctx(&ctx).await.unwrap().0, 100);
        assert_eq!(Sender::from_ctx(&ctx).await.unwrap().0, 42);
        assert_eq!(Text::from_ctx(&ctx).await.unwrap().0, "hi");
    }

    #[tokio::test]
    async fn extracts_callback_event_ids() {
        let ctx = ctx(json!({
            "type": "message_event",
            "object": { "user_id": 7, "peer_id": 200, "payload": {"action": "buy"} }
        }));

        assert_eq!(Peer::from_ctx(&ctx).await.unwrap().0, 200);
        assert_eq!(Sender::from_ctx(&ctx).await.unwrap().0, 7);
        assert_eq!(
            Payload::from_ctx(&ctx).await.unwrap().0,
            json!({"action": "buy"})
        );
    }

    #[tokio::test]
    async fn missing_field_is_an_error() {
        let ctx = ctx(json!({"type": "wall_post_new"}));

        assert!(Peer::from_ctx(&ctx).await.is_err());
        assert!(Text::from_ctx(&ctx).await.is_err());
        // ...but optional extractors absorb it.
        assert!(Option::<Peer>::from_ctx(&ctx).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn state_round_trips_through_storage() {
        let ctx = ctx(message_event());
        ctx.storage.insert(Database("postgres://"));

        let State(db) = State::<Database>::from_ctx(&ctx).await.unwrap();
        assert_eq!(db.0, "postgres://");
    }

    #[tokio::test]
    async fn unregistered_state_names_the_type() {
        let ctx = ctx(message_event());

        let err = match State::<Database>::from_ctx(&ctx).await {
            Ok(_) => panic!("unregistered state must not resolve"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("Database"), "{err}");

        // The optional flavour stays quiet.
        assert!(OptionalState::<Database>::from_ctx(&ctx)
            .await
            .unwrap()
            .0
            .is_none());
    }

    #[tokio::test]
    async fn message_min_comes_from_the_event() {
        let ctx = ctx(message_event());

        let msg = MessageMin::from_ctx(&ctx).await.unwrap();
        assert_eq!(msg.peer_id, 100);
        assert_eq!(msg.text, "hi");
    }
}
