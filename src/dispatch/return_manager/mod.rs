//! Return value managers

pub mod message;
pub mod message_event;
pub mod user_message;

pub use message::*;
pub use message_event::*;
pub use user_message::*;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::exception::VkResult;

/// Handles handler return values
#[async_trait]
pub trait ReturnManager<E>: Send + Sync {
    async fn process(&self, event: &E, api: &Api, value: Value) -> VkResult<Value>;
}
