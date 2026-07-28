//! Waiter machine — wait for the next matching event from a peer

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::oneshot;

use crate::dispatch::rules::Rule;
use crate::dispatch::RuleResult;

struct WaiterEntry {
    rules: Vec<Arc<dyn Rule<Value>>>,
    notify: oneshot::Sender<Value>,
}

/// In-memory waiter storage keyed by view name → peer_id → waiter
pub struct WaiterMachine {
    storage: DashMap<String, DashMap<i64, WaiterEntry>>,
    max_per_view: usize,
}

impl WaiterMachine {
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
            max_per_view: 1000,
        }
    }

    pub fn with_capacity(max_per_view: usize) -> Self {
        Self {
            storage: DashMap::new(),
            max_per_view,
        }
    }

    fn view_bucket(&self, view: &str) -> dashmap::mapref::one::RefMut<'_, String, DashMap<i64, WaiterEntry>> {
        if !self.storage.contains_key(view) {
            self.storage.insert(view.to_string(), DashMap::new());
        }
        self.storage.get_mut(view).unwrap()
    }

    /// Wait until an event matching `rules` arrives for `peer_id`
    pub async fn wait(
        self: &Arc<Self>,
        view: &str,
        peer_id: i64,
        rules: Vec<Arc<dyn Rule<Value>>>,
        timeout: Option<Duration>,
    ) -> Result<Value, WaiterError> {
        let (tx, rx) = oneshot::channel();

        {
            let bucket = self.view_bucket(view);
            if bucket.len() >= self.max_per_view {
                return Err(WaiterError::StorageFull);
            }
            bucket.insert(peer_id, WaiterEntry { rules, notify: tx });
        }

        if let Some(dur) = timeout {
            tokio::select! {
                res = rx => res.map_err(|_| WaiterError::Cancelled),
                _ = tokio::time::sleep(dur) => {
                    self.cancel(view, peer_id);
                    Err(WaiterError::Timeout)
                }
            }
        } else {
            rx.await.map_err(|_| WaiterError::Cancelled)
        }
    }

    /// Try to resolve a pending waiter; returns true if consumed
    pub async fn feed(&self, view: &str, peer_id: i64, event: &Value) -> bool {
        let Some(bucket) = self.storage.get(view) else {
            return false;
        };
        let Some((_, entry)) = bucket.remove(&peer_id) else {
            return false;
        };

        for rule in &entry.rules {
            if matches!(rule.check(event).await, RuleResult::Fail) {
                return false;
            }
        }

        let _ = entry.notify.send(event.clone());
        true
    }

    pub fn cancel(&self, view: &str, peer_id: i64) -> bool {
        self.storage
            .get(view)
            .and_then(|b| b.remove(&peer_id))
            .is_some()
    }
}

impl Default for WaiterMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WaiterError {
    #[error("waiter cancelled")]
    Cancelled,
    #[error("waiter storage full")]
    StorageFull,
    #[error("waiter timed out")]
    Timeout,
}

/// Shared waiter machine for bots
pub type SharedWaiter = Arc<WaiterMachine>;

pub fn shared_waiter() -> SharedWaiter {
    Arc::new(WaiterMachine::new())
}

/// Middleware hook: call from custom middleware to resolve waiters on message events
pub async fn try_feed_message_waiters(
    machine: &WaiterMachine,
    view: &str,
    event: &Value,
) -> bool {
    let peer_id = event
        .get("object")
        .and_then(|o| o.get("message"))
        .and_then(|m| m.get("peer_id"))
        .and_then(|p| p.as_i64())
        .or_else(|| {
            event
                .get("object")
                .and_then(|o| o.get("peer_id"))
                .and_then(|p| p.as_i64())
        });
    if let Some(peer_id) = peer_id {
        return machine.feed(view, peer_id, event).await;
    }
    false
}
