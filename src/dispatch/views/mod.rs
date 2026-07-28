//! Event views

pub mod message_view;
pub mod message_state;
pub mod message_event_view;
pub mod raw_event_view;
pub mod user_message_view;
pub mod user_update;

pub use message_view::*;
pub use message_state::*;
pub use message_event_view::*;
pub use raw_event_view::*;
pub use user_message_view::*;
pub use user_update::*;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::DispatchResult;

/// View trait — parses raw events and routes to handlers
#[async_trait]
pub trait View: Send + Sync {
    async fn process(
        &self,
        event: &Value,
        api: &Api,
        state_dispenser: Option<&dyn StateDispenser>,
    ) -> DispatchResult<Option<Value>>;
}
