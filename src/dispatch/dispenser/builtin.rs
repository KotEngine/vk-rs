//! Built-in in-memory state dispenser

use async_trait::async_trait;
use dashmap::DashMap;

use crate::exception::VkResult;
use crate::tools::fsm::StatePeer;
use super::StateDispenser;

/// In-memory concurrent state dispenser
pub struct BuiltinStateDispenser {
    pub(crate) states: DashMap<i64, StatePeer>,
}

impl BuiltinStateDispenser {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    pub fn snapshot(&self) -> Vec<StatePeer> {
        self.states.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for BuiltinStateDispenser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateDispenser for BuiltinStateDispenser {
    async fn get(&self, peer_id: i64) -> VkResult<Option<StatePeer>> {
        Ok(self.states.get(&peer_id).map(|r| r.clone()))
    }

    async fn set(&self, peer: StatePeer) -> VkResult<()> {
        self.states.insert(peer.peer_id, peer);
        Ok(())
    }

    async fn delete(&self, peer_id: i64) -> VkResult<bool> {
        Ok(self.states.remove(&peer_id).is_some())
    }
}
