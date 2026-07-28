//! Callback API trait

use std::sync::Arc;

use async_trait::async_trait;

use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::router::DispatchRouter;
use crate::exception::VkResult;

use super::CallbackConfig;

/// Callback webhook server abstraction
#[async_trait]
pub trait Callback: Send + Sync {
    fn config(&self) -> &CallbackConfig;

    async fn register_server(&self) -> VkResult<i64>;

    async fn run(
        &self,
        router: Arc<DispatchRouter>,
        state_dispenser: Arc<dyn StateDispenser>,
    ) -> VkResult<()>;
}
