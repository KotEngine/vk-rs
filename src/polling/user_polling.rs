//! User polling implementation

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;
use tokio::sync::RwLock;

use crate::api::*;
use crate::constants::{FailureCode, FailureRecovery};
use super::*;

/// User polling implementation
pub struct UserPolling {
    base: BasePolling,
    server_info: RwLock<Option<PollingServer>>,
    last_ts: RwLock<i64>,
}

impl UserPolling {
    pub fn new(api: Box<dyn VkApi>, user_id: i64) -> Self {
        Self {
            base: BasePolling::for_user(api, user_id),
            server_info: RwLock::new(None),
            last_ts: RwLock::new(0),
        }
    }

    pub fn with_config(api: Box<dyn VkApi>, user_id: i64, config: PollingConfig) -> Self {
        Self {
            base: BasePolling::with_config(api, None, Some(user_id), config),
            server_info: RwLock::new(None),
            last_ts: RwLock::new(0),
        }
    }

    pub fn user_id(&self) -> i64 {
        self.base.user_id().unwrap_or(0)
    }

    pub fn config(&self) -> &PollingConfig {
        self.base.config()
    }

    pub fn set_config(&mut self, config: PollingConfig) {
        self.base.with_config_mut(config);
    }

    async fn set_current_ts(&self, ts: i64) {
        *self.last_ts.write().await = ts;
        self.base.set_current_ts(ts).await;
    }

    fn build_user_polling_url(&self, server: &PollingServer, ts: i64) -> String {
        format!(
            "{}?act=a_check&key={}&ts={}&wait={}&mode={}&version={}&rps_delay=0",
            server.server,
            server.key,
            ts,
            self.config().wait,
            self.config().mode,
            self.config().version
        )
    }

    /// Recover from a `failed` response, following VK's documented semantics.
    ///
    /// Returns a replacement server when one is needed, plus the `ts` to resume
    /// from — `None` means "keep the current one".
    async fn handle_user_failed_response(
        &self,
        failed: i32,
    ) -> PollingResult<(Option<PollingServer>, Option<i64>)> {
        let Some(code) = FailureCode::from_code(failed as i64) else {
            return Err(PollingError::events_error(format!(
                "Unknown user polling failure code: {failed}"
            )));
        };

        tracing::warn!(?code, "user polling failure");

        match code.recovery() {
            FailureRecovery::KeepServer => Ok((None, None)),
            FailureRecovery::NewServerKeepTs => Ok((Some(self.get_server().await?), None)),
            FailureRecovery::NewServerResetTs => {
                let server = self.get_server().await?;
                let ts = server.ts;
                Ok((Some(server), Some(ts)))
            }
        }
    }
}

#[async_trait]
impl Polling for UserPolling {
    async fn get_server(&self) -> PollingResult<PollingServer> {
        let user_id = self.user_id();
        let mut params = std::collections::HashMap::new();
        params.insert("user_id".to_string(), user_id.to_string());

        let response = self
            .base
            .api()
            .request("messages.getLongPollServer", &params)
            .await
            .map_err(|e| PollingError::server_info_error(format!("Failed to get user polling server: {e}")))?;

        let response_obj = response.as_object().ok_or_else(|| {
            PollingError::server_info_error("User polling server response is not an object".to_string())
        })?;

        let key = response_obj
            .get("key")
            .and_then(|k| k.as_str())
            .ok_or_else(|| PollingError::server_info_error("Missing user polling server key".to_string()))?
            .to_string();

        let server = response_obj
            .get("server")
            .and_then(|s| s.as_str())
            .ok_or_else(|| PollingError::server_info_error("Missing user polling server URL".to_string()))?
            .to_string();

        let ts = response_obj
            .get("ts")
            .and_then(|ts| ts.as_i64())
            .ok_or_else(|| PollingError::server_info_error("Missing user polling server TS".to_string()))?;

        let server_info = PollingServer::new(key, server, ts);
        *self.server_info.write().await = Some(server_info.clone());
        self.set_current_ts(ts).await;

        Ok(server_info)
    }

    async fn get_events(&self, server: &PollingServer, ts: i64) -> PollingResult<PollingEvents> {
        let url = self.build_user_polling_url(server, ts);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(self.config().wait as u64 + 5))
            .send()
            .await
            .map_err(|e| PollingError::events_error(format!("Failed to request user polling events: {e}")))?;

        if !response.status().is_success() {
            return Err(PollingError::events_error(format!(
                "User polling server returned status: {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| PollingError::events_error(format!("Failed to read user polling response: {e}")))?;

        parse_events_response(&text)
    }

    fn listen(&self) -> Pin<Box<dyn Stream<Item = Value> + Send + '_>> {
        Box::pin(stream! {
            let mut current_ts = if let Some(path) = &self.config().ts_file {
                load_ts_file(path).await.unwrap_or(0)
            } else {
                0
            };
            let mut server: Option<PollingServer> = None;

            loop {
                if server.is_none() {
                    match self.get_server().await {
                        Ok(srv) => {
                            if current_ts == 0 {
                                current_ts = srv.ts;
                            }
                            server = Some(srv);
                        }
                        Err(e) => {
                            tracing::error!("Failed to get user polling server: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }

                if let Some(srv) = &server {
                    match self.get_events(srv, current_ts).await {
                        Ok(mut events) => {
                            for update in std::mem::take(&mut events.updates) {
                                yield update;
                            }
                            if events.has_ts() {
                                current_ts = events.ts;
                                let _ = self.save_server_ts(current_ts).await;
                            }

                            if let Some(failed) = events.failed {
                                match self.handle_user_failed_response(failed).await {
                                    Ok((new_server, new_ts)) => {
                                        if let Some(ts) = new_ts {
                                            current_ts = ts;
                                        }
                                        if let Some(srv) = new_server {
                                            server = Some(srv);
                                        }
                                    }
                                    Err(e) => tracing::error!("Polling failed recovery: {e}"),
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get user polling events: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        })
    }

    async fn restore_server_ts(&self) -> PollingResult<i64> {
        self.base.restore_server_ts().await
    }

    async fn save_server_ts(&self, ts: i64) -> PollingResult<()> {
        *self.last_ts.write().await = ts;
        self.base.save_server_ts(ts).await
    }
}

pub fn create_user_polling(api: Box<dyn VkApi>, user_id: i64) -> UserPolling {
    UserPolling::new(api, user_id)
}

pub fn create_user_polling_with_config(
    api: Box<dyn VkApi>,
    user_id: i64,
    config: PollingConfig,
) -> UserPolling {
    UserPolling::with_config(api, user_id, config)
}
