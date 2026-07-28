//! Base polling functionality

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;
use std::sync::atomic::{AtomicI16, Ordering};
use tokio::sync::RwLock;

use crate::api::*;
use crate::constants::{FailureCode, DEFAULT_LONGPOLL_VERSION};
use super::*;

/// Base polling implementation
pub struct BasePolling {
    api: Box<dyn VkApi>,
    group_id: Option<i64>,
    user_id: Option<i64>,
    config: PollingConfig,
    server_info: RwLock<Option<PollingServer>>,
    last_ts: RwLock<i64>,
    /// Live long poll version. Starts at `config.version` and is lowered to
    /// [`DEFAULT_LONGPOLL_VERSION`] if VK rejects it with `failed=4`.
    lp_version: AtomicI16,
}

impl BasePolling {
    pub fn new(api: Box<dyn VkApi>, group_id: Option<i64>, user_id: Option<i64>) -> Self {
        Self {
            api,
            group_id,
            user_id,
            lp_version: AtomicI16::new(PollingConfig::default().version),
            config: PollingConfig::default(),
            server_info: RwLock::new(None),
            last_ts: RwLock::new(0),
        }
    }

    pub fn with_config(
        api: Box<dyn VkApi>,
        group_id: Option<i64>,
        user_id: Option<i64>,
        config: PollingConfig,
    ) -> Self {
        Self {
            api,
            group_id,
            user_id,
            lp_version: AtomicI16::new(config.version),
            config,
            server_info: RwLock::new(None),
            last_ts: RwLock::new(0),
        }
    }

    pub fn for_bot(api: Box<dyn VkApi>, group_id: i64) -> Self {
        Self::new(api, Some(group_id), None)
    }

    pub fn for_user(api: Box<dyn VkApi>, user_id: i64) -> Self {
        Self::new(api, None, Some(user_id))
    }

    pub fn with_config_mut(&mut self, config: PollingConfig) {
        self.lp_version.store(config.version, Ordering::Relaxed);
        self.config = config;
    }

    /// Long poll version currently in use.
    pub fn lp_version(&self) -> i16 {
        self.lp_version.load(Ordering::Relaxed)
    }

    /// Override the long poll version for subsequent requests.
    pub fn set_lp_version(&self, version: i16) {
        self.lp_version.store(version, Ordering::Relaxed);
    }

    pub fn group_id(&self) -> Option<i64> {
        self.group_id
    }

    pub fn user_id(&self) -> Option<i64> {
        self.user_id
    }

    pub fn config(&self) -> &PollingConfig {
        &self.config
    }

    pub fn api(&self) -> &dyn VkApi {
        self.api.as_ref()
    }

    pub fn last_ts_lock(&self) -> &RwLock<i64> {
        &self.last_ts
    }

    fn build_polling_url(&self, server: &PollingServer, ts: i64) -> String {
        format!(
            "{}?act=a_check&key={}&ts={}&wait={}&mode={}&version={}&failed={}",
            server.server,
            server.key,
            ts,
            self.config.wait,
            self.config.mode,
            self.lp_version(),
            self.config.failed
        )
    }

    pub async fn get_current_ts(&self) -> i64 {
        *self.last_ts.read().await
    }

    pub async fn set_current_ts(&self, ts: i64) {
        *self.last_ts.write().await = ts;
    }
}

#[async_trait]
impl Polling for BasePolling {
    async fn get_server(&self) -> PollingResult<PollingServer> {
        let mut params = std::collections::HashMap::new();

        if let Some(group_id) = self.group_id {
            params.insert("group_id".to_string(), group_id.to_string());
            let response = self
                .api
                .request("groups.getLongPollServer", &params)
                .await
                .map_err(|e| PollingError::server_info_error(e.to_string()))?;
            return parse_server_response(response, &self.server_info).await;
        }

        if self.user_id.is_some() {
            let response = self
                .api
                .request("messages.getLongPollServer", &params)
                .await
                .map_err(|e| PollingError::server_info_error(e.to_string()))?;
            return parse_server_response(response, &self.server_info).await;
        }

        Err(PollingError::config_error(
            "Either group_id or user_id must be specified".to_string(),
        ))
    }

    async fn get_events(&self, server: &PollingServer, ts: i64) -> PollingResult<PollingEvents> {
        let url = self.build_polling_url(server, ts);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(self.config.wait as u64 + 5))
            .send()
            .await
            .map_err(|e| PollingError::events_error(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PollingError::events_error(format!(
                "Polling server returned status: {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| PollingError::events_error(e.to_string()))?;

        parse_events_response(&text)
    }

    fn listen(&self) -> Pin<Box<dyn Stream<Item = Value> + Send + '_>> {
        Box::pin(stream! {
            let mut current_ts = if let Some(path) = &self.config.ts_file {
                super::load_ts_file(path).await.unwrap_or(0)
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
                            tracing::error!("Failed to get polling server: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }

                if let Some(ref srv) = server {
                    match self.get_events(srv, current_ts).await {
                        Ok(mut events) => {
                            for update in std::mem::take(&mut events.updates) {
                                yield update;
                            }
                            if events.has_ts() {
                                current_ts = events.ts;
                                self.set_current_ts(current_ts).await;
                                if let Some(path) = &self.config.ts_file {
                                    let _ = super::save_ts_file(path, current_ts).await;
                                }
                            }

                            if let Some(failed) = events.failed {
                                match FailureCode::from_code(failed as i64) {
                                    // `ts` was stale; VK already handed us a fresh
                                    // one above and the key stays valid.
                                    Some(FailureCode::HistoryOutdated) => {
                                        tracing::warn!(ts = current_ts, "long poll history outdated, resuming from new ts");
                                    }
                                    // Key died but history is intact — new server,
                                    // same ts, so nothing is skipped.
                                    Some(FailureCode::KeyExpired) => {
                                        tracing::warn!("long poll key expired, requesting a new server");
                                        server = None;
                                    }
                                    // History is gone — take the new server's ts.
                                    Some(FailureCode::InformationLost) => {
                                        tracing::warn!("long poll information lost, resetting ts");
                                        server = None;
                                        current_ts = 0;
                                    }
                                    Some(FailureCode::InvalidVersion) => {
                                        tracing::error!(
                                            min_version = ?events.min_version,
                                            max_version = ?events.max_version,
                                            fallback = DEFAULT_LONGPOLL_VERSION,
                                            "unsupported long poll version, falling back"
                                        );
                                        self.set_lp_version(DEFAULT_LONGPOLL_VERSION as i16);
                                        server = None;
                                    }
                                    None => {
                                        tracing::error!(code = failed, "unknown long poll failure code");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get polling events: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        })
    }

    async fn restore_server_ts(&self) -> PollingResult<i64> {
        if let Some(path) = &self.config.ts_file {
            if let Some(ts) = super::load_ts_file(path).await {
                self.set_current_ts(ts).await;
                return Ok(ts);
            }
        }
        Ok(self.get_current_ts().await)
    }

    async fn save_server_ts(&self, ts: i64) -> PollingResult<()> {
        self.set_current_ts(ts).await;
        if let Some(path) = &self.config.ts_file {
            super::save_ts_file(path, ts).await?;
        }
        Ok(())
    }
}

async fn parse_server_response(
    response: Value,
    cache: &RwLock<Option<PollingServer>>,
) -> PollingResult<PollingServer> {
    let response_obj = response
        .as_object()
        .ok_or_else(|| PollingError::server_info_error("Server info is not an object".to_string()))?;

    let key = response_obj
        .get("key")
        .and_then(|k| k.as_str())
        .ok_or_else(|| PollingError::server_info_error("Missing server key".to_string()))?
        .to_string();

    let server = response_obj
        .get("server")
        .and_then(|s| s.as_str())
        .ok_or_else(|| PollingError::server_info_error("Missing server URL".to_string()))?
        .to_string();

    let ts = response_obj
        .get("ts")
        .and_then(|ts| ts.as_i64())
        .ok_or_else(|| PollingError::server_info_error("Missing server TS".to_string()))?;

    let server_info = PollingServer::new(key, server, ts);
    *cache.write().await = Some(server_info.clone());
    Ok(server_info)
}

pub(crate) fn parse_events_response(text: &str) -> PollingResult<PollingEvents> {
    let response_obj: Value = serde_json::from_str(text)
        .map_err(|e| PollingError::events_error(e.to_string()))?;

    let updates = response_obj
        .get("updates")
        .and_then(|u| u.as_array())
        .map(|arr| arr.clone())
        .unwrap_or_default();

    // VK omits `ts` on most failures (only `failed=1` carries a fresh one), so a
    // missing `ts` is only an error for an otherwise-successful response.
    let failed = response_obj.get("failed").and_then(|f| f.as_i64());
    let new_ts = match response_obj.get("ts").and_then(|ts| ts.as_i64()) {
        Some(ts) => ts,
        None if failed.is_some() => 0,
        None => {
            return Err(PollingError::events_error(
                "Missing TS in polling response".to_string(),
            ))
        }
    };

    let mut events = PollingEvents::new(new_ts, updates);

    if let Some(failed) = failed {
        let min = response_obj
            .get("min_version")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let max = response_obj
            .get("max_version")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        events = events.with_failed(failed as i32).with_version_range(min, max);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::FailureRecovery;

    #[test]
    fn normal_response_parses_ts_and_updates() {
        let events = parse_events_response(r#"{"ts": 42, "updates": [{"type": "message_new"}]}"#)
            .expect("valid response");

        assert_eq!(events.ts, 42);
        assert!(events.has_ts());
        assert_eq!(events.updates.len(), 1);
        assert!(events.failed.is_none());
    }

    #[test]
    fn missing_ts_without_failure_is_an_error() {
        assert!(parse_events_response(r#"{"updates": []}"#).is_err());
    }

    #[test]
    fn history_outdated_carries_a_fresh_ts() {
        let events = parse_events_response(r#"{"failed": 1, "ts": 99}"#).expect("valid response");

        assert_eq!(events.failed, Some(1));
        assert_eq!(events.ts, 99);
        assert!(events.has_ts());
    }

    /// VK sends `{"failed": 2}` with no `ts`; parsing must not fail, otherwise the
    /// poller retries forever against an expired key.
    #[test]
    fn key_expired_without_ts_still_parses() {
        let events = parse_events_response(r#"{"failed": 2}"#).expect("valid response");

        assert_eq!(events.failed, Some(2));
        assert!(!events.has_ts());
    }

    #[test]
    fn invalid_version_exposes_supported_range() {
        let events = parse_events_response(r#"{"failed": 4, "min_version": 1, "max_version": 3}"#)
            .expect("valid response");

        assert_eq!(events.failed, Some(4));
        assert_eq!(events.min_version, Some(1));
        assert_eq!(events.max_version, Some(3));
        assert!(!events.has_ts());
    }

    #[test]
    fn recovery_matches_vk_protocol() {
        use FailureCode::*;

        assert_eq!(HistoryOutdated.recovery(), FailureRecovery::KeepServer);
        assert_eq!(KeyExpired.recovery(), FailureRecovery::NewServerKeepTs);
        assert_eq!(InformationLost.recovery(), FailureRecovery::NewServerResetTs);
        assert_eq!(InvalidVersion.recovery(), FailureRecovery::NewServerResetTs);
    }

    #[test]
    fn lp_version_falls_back_on_invalid_version() {
        let polling = BasePolling::new(Box::new(crate::api::Api::new("t").unwrap()), Some(1), None);
        polling.set_lp_version(99);
        assert_eq!(polling.lp_version(), 99);

        polling.set_lp_version(DEFAULT_LONGPOLL_VERSION as i16);
        assert_eq!(polling.lp_version(), 3);
    }
}
