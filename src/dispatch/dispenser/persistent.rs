//! File-backed persistent state dispenser

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::exception::{VkError, VkResult};
use crate::tools::fsm::StatePeer;

use super::builtin::BuiltinStateDispenser;
use super::StateDispenser;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedStates {
    peers: Vec<StatePeer>,
}

/// FSM dispenser that persists peer states to a JSON file
pub struct FileStateDispenser {
    path: PathBuf,
    inner: BuiltinStateDispenser,
    save_lock: Mutex<()>,
}

impl FileStateDispenser {
    pub async fn open(path: impl AsRef<Path>) -> VkResult<Self> {
        let path = path.as_ref().to_path_buf();
        let inner = BuiltinStateDispenser::new();

        if path.exists() {
            let data = tokio::fs::read_to_string(&path).await.map_err(VkError::Io)?;
            if !data.trim().is_empty() {
                let stored: PersistedStates = serde_json::from_str(&data)
                    .map_err(|e| VkError::Validation(format!("Invalid state file: {e}")))?;
                for peer in stored.peers {
                    inner.set(peer).await?;
                }
            }
        } else if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(VkError::Io)?;
        }

        Ok(Self {
            path,
            inner,
            save_lock: Mutex::new(()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn flush(&self) -> VkResult<()> {
        let _guard = self.save_lock.lock().await;
        let data = PersistedStates {
            peers: self.inner.snapshot(),
        };
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| VkError::Internal(e.to_string()))?;
        tokio::fs::write(&self.path, json).await.map_err(VkError::Io)?;
        Ok(())
    }
}

#[async_trait]
impl StateDispenser for FileStateDispenser {
    async fn get(&self, peer_id: i64) -> VkResult<Option<StatePeer>> {
        self.inner.get(peer_id).await
    }

    async fn set(&self, peer: StatePeer) -> VkResult<()> {
        self.inner.set(peer).await?;
        self.flush().await
    }

    async fn delete(&self, peer_id: i64) -> VkResult<bool> {
        let removed = self.inner.delete(peer_id).await?;
        if removed {
            self.flush().await?;
        }
        Ok(removed)
    }
}
