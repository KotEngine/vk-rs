//! VK Longpoll module

pub mod base;
pub mod bot_polling;
pub mod user_polling;

pub use base::*;
pub use bot_polling::*;
pub use user_polling::*;

use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Polling trait for VK longpoll
#[async_trait]
pub trait Polling: Send + Sync {
    async fn get_server(&self) -> PollingResult<PollingServer>;
    async fn get_events(&self, server: &PollingServer, ts: i64) -> PollingResult<PollingEvents>;
    fn listen(&self) -> Pin<Box<dyn Stream<Item = Value> + Send + '_>>;
    async fn restore_server_ts(&self) -> PollingResult<i64>;
    async fn save_server_ts(&self, ts: i64) -> PollingResult<()>;
}

/// Polling server information
#[derive(Debug, Clone)]
pub struct PollingServer {
    pub key: String,
    pub server: String,
    pub ts: i64,
}

impl PollingServer {
    pub fn new(key: String, server: String, ts: i64) -> Self {
        Self { key, server, ts }
    }
    
    pub fn with_ts(mut self, ts: i64) -> Self {
        self.ts = ts;
        self
    }
}

/// Polling events
#[derive(Debug, Clone)]
pub struct PollingEvents {
    /// New `ts` to resume from. `0` when VK omitted it — which it does for every
    /// failure except `failed=1`.
    pub ts: i64,
    pub updates: Vec<Value>,
    pub failed: Option<i32>,
    /// Lowest long poll version VK accepts, sent alongside `failed=4`.
    pub min_version: Option<i32>,
    /// Highest long poll version VK accepts, sent alongside `failed=4`.
    pub max_version: Option<i32>,
}

impl PollingEvents {
    pub fn new(ts: i64, updates: Vec<Value>) -> Self {
        Self {
            ts,
            updates,
            failed: None,
            min_version: None,
            max_version: None,
        }
    }

    pub fn with_failed(mut self, failed: i32) -> Self {
        self.failed = Some(failed);
        self
    }

    pub fn with_version_range(mut self, min: Option<i32>, max: Option<i32>) -> Self {
        self.min_version = min;
        self.max_version = max;
        self
    }

    /// Whether VK supplied a usable `ts`.
    pub fn has_ts(&self) -> bool {
        self.ts != 0
    }
    
    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.updates.len()
    }
}

/// Polling configuration
#[derive(Debug, Clone)]
pub struct PollingConfig {
    pub wait: u16,
    pub mode: i16,
    pub version: i16,
    pub failed: i16,
    /// Optional path to persist long-poll `ts` between restarts
    pub ts_file: Option<String>,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            wait: 25,
            mode: 2,
            version: 3,
            failed: 3,
            ts_file: None,
        }
    }
}

impl PollingConfig {
    pub fn new(wait: u16, mode: i16, version: i16, failed: i16) -> Self {
        Self {
            wait,
            mode,
            version,
            failed,
            ts_file: None,
        }
    }
    
    pub fn with_wait(mut self, wait: u16) -> Self {
        self.wait = wait;
        self
    }
    
    pub fn with_mode(mut self, mode: i16) -> Self {
        self.mode = mode;
        self
    }
    
    pub fn with_version(mut self, version: i16) -> Self {
        self.version = version;
        self
    }
    
    pub fn with_failed(mut self, failed: i16) -> Self {
        self.failed = failed;
        self
    }

    pub fn with_ts_file(mut self, path: impl Into<String>) -> Self {
        self.ts_file = Some(path.into());
        self
    }
}

/// Load persisted long-poll TS from disk
pub async fn load_ts_file(path: &str) -> Option<i64> {
    let content = tokio::fs::read_to_string(path).await.ok()?;
    content.trim().parse().ok()
}

/// Save long-poll TS to disk
pub async fn save_ts_file(path: &str, ts: i64) -> PollingResult<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| PollingError::config_error(e.to_string()))?;
        }
    }
    tokio::fs::write(path, ts.to_string())
        .await
        .map_err(|e| PollingError::config_error(e.to_string()))
}

/// Polling error types
#[derive(Debug, thiserror::Error)]
pub enum PollingError {
    #[error("Polling server error: {0}")]
    ServerError(String),
    
    #[error("Failed to get server info: {0}")]
    ServerInfoError(String),
    
    #[error("Failed to get events: {0}")]
    EventsError(String),
    
    #[error("Invalid TS value: {0}")]
    InvalidTs(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl PollingError {
    pub fn server_error(msg: String) -> Self {
        Self::ServerError(msg)
    }
    
    pub fn server_info_error(msg: String) -> Self {
        Self::ServerInfoError(msg)
    }
    
    pub fn events_error(msg: String) -> Self {
        Self::EventsError(msg)
    }
    
    pub fn invalid_ts(msg: String) -> Self {
        Self::InvalidTs(msg)
    }
    
    pub fn config_error(msg: String) -> Self {
        Self::ConfigError(msg)
    }
}

/// Polling result type
pub type PollingResult<T> = Result<T, PollingError>;