//! Redis-backed FSM state dispenser.
//!
//! Stores peer FSM states in Redis so they survive bot restarts and can be
//! shared across multiple bot instances behind a load balancer.
//!
//! Each peer is stored under `vkontakte:state:{peer_id}` as a JSON-serialized
//! [`StatePeer`]. Enable the `redis` cargo feature to use it:
//!
//! ```toml
//! vkontakte = { version = "0.1", features = ["redis"] }
//! ```
//!
//! ```no_run
//! # use vkontakte::dispatch::dispenser::RedisStateDispenser;
//! # async fn run() -> vkontakte::VkResult<()> {
//! let dispenser = RedisStateDispenser::new("redis://127.0.0.1/").await?;
//! // ... pass `Arc::new(dispenser)` to `Bot::with_state_dispenser`.
//! # Ok(()) }
//! ```

use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config, Pool, Runtime};

use crate::exception::{VkError, VkResult};
use crate::tools::fsm::StatePeer;

use super::StateDispenser;

const DEFAULT_KEY_PREFIX: &str = "vkontakte:state:";

/// FSM dispenser backed by a Redis connection pool (via `deadpool-redis`).
pub struct RedisStateDispenser {
    pool: Pool,
    key_prefix: String,
}

impl RedisStateDispenser {
    /// Connect to `redis://...` and build a dispenser with the default key prefix.
    pub async fn new(url: &str) -> VkResult<Self> {
        Self::with_prefix(DEFAULT_KEY_PREFIX, url).await
    }

    /// Connect with a custom key prefix (e.g. `"mybot:state:"`).
    pub async fn with_prefix(prefix: &str, url: &str) -> VkResult<Self> {
        let cfg = Config::from_url(url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| VkError::Configuration(format!("redis pool: {e}")))?;
        Ok(Self {
            pool,
            key_prefix: prefix.to_string(),
        })
    }

    /// Wrap an existing deadpool-redis pool (e.g. shared with other parts of the app).
    pub fn from_pool(pool: Pool) -> Self {
        Self {
            pool,
            key_prefix: DEFAULT_KEY_PREFIX.to_string(),
        }
    }

    /// Borrow the underlying pool for ad-hoc commands.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    fn key(&self, peer_id: i64) -> String {
        format!("{}{peer_id}", self.key_prefix)
    }
}

#[async_trait]
impl StateDispenser for RedisStateDispenser {
    async fn get(&self, peer_id: i64) -> VkResult<Option<StatePeer>> {
        let mut conn = self.pool.get().await.map_err(pool_err)?;
        let payload: Option<String> = conn
            .get(self.key(peer_id))
            .await
            .map_err(cmd_err)?;
        match payload {
            Some(json) => {
                let peer: StatePeer =
                    serde_json::from_str(&json).map_err(|e| {
                        VkError::Deserialization(format!("state for peer {peer_id}: {e}"))
                    })?;
                Ok(Some(peer))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, peer: StatePeer) -> VkResult<()> {
        let mut conn = self.pool.get().await.map_err(pool_err)?;
        let json = serde_json::to_string(&peer)
            .map_err(|e| VkError::Serialization(e.to_string()))?;
        let _: () = conn.set(self.key(peer.peer_id), json).await.map_err(cmd_err)?;
        Ok(())
    }

    async fn delete(&self, peer_id: i64) -> VkResult<bool> {
        let mut conn = self.pool.get().await.map_err(pool_err)?;
        let removed: i64 = conn
            .del(self.key(peer_id))
            .await
            .map_err(cmd_err)?;
        Ok(removed > 0)
    }
}

fn pool_err(e: deadpool_redis::PoolError) -> VkError {
    match e {
        deadpool_redis::PoolError::Backend(inner) => {
            VkError::Internal(format!("redis: {inner}"))
        }
        other => VkError::Internal(format!("redis pool: {other}")),
    }
}

fn cmd_err(e: deadpool_redis::redis::RedisError) -> VkError {
    VkError::Internal(format!("redis: {e}"))
}

impl std::fmt::Debug for RedisStateDispenser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisStateDispenser")
            .field("key_prefix", &self.key_prefix)
            .finish_non_exhaustive()
    }
}
