//! State dispenser trait

use async_trait::async_trait;

use crate::exception::VkResult;
use crate::tools::fsm::StatePeer;

/// FSM state dispenser
#[async_trait]
pub trait StateDispenser: Send + Sync {
    async fn get(&self, peer_id: i64) -> VkResult<Option<StatePeer>>;
    async fn set(&self, peer: StatePeer) -> VkResult<()>;
    async fn delete(&self, peer_id: i64) -> VkResult<bool>;
}
