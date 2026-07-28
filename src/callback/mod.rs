//! Callback API (webhook) module

pub mod settings;
pub mod abc;

pub use settings::*;
pub use abc::*;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router as AxumRouter,
};
use serde_json::Value;
use tokio::net::TcpListener;

use crate::api::{Api, VkApi};
use crate::dispatch::router::DispatchRouter;
use crate::dispatch::Router as DispatchRouterTrait;
use crate::exception::{VkError, VkResult};

/// Callback API configuration
#[derive(Debug, Clone)]
pub struct CallbackConfig {
    pub group_id: i64,
    pub secret: String,
    pub confirmation_code: String,
    pub server_url: String,
    pub host: String,
    pub port: u16,
}

impl CallbackConfig {
    pub fn new(
        group_id: i64,
        secret: impl Into<String>,
        confirmation_code: impl Into<String>,
        server_url: impl Into<String>,
    ) -> Self {
        Self {
            group_id,
            secret: secret.into(),
            confirmation_code: confirmation_code.into(),
            server_url: server_url.into(),
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }

    /// Generate a random 32-char secret key for callback servers
    pub fn generate_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub fn with_listen(mut self, host: impl Into<String>, port: u16) -> Self {
        self.host = host.into();
        self.port = port;
        self
    }
}

#[derive(Clone)]
struct CallbackState {
    config: CallbackConfig,
    api: Arc<Api>,
    router: Arc<DispatchRouter>,
    state_dispenser: Arc<dyn crate::dispatch::dispenser::StateDispenser>,
    waiter_machine: Arc<crate::tools::waiter::WaiterMachine>,
}

/// Bot callback API server
pub struct BotCallback {
    config: CallbackConfig,
    api: Arc<Api>,
}

impl BotCallback {
    pub fn new(config: CallbackConfig, api: Arc<Api>) -> Self {
        Self { config, api }
    }

    pub fn config(&self) -> &CallbackConfig {
        &self.config
    }

    pub async fn register_server(&self) -> VkResult<i64> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), self.config.group_id.to_string());
        params.insert("url".to_string(), self.config.server_url.clone());
        params.insert("title".to_string(), "vkontakte callback".to_string());
        params.insert("secret_key".to_string(), self.config.secret.clone());

        let resp = self.api.request("groups.addCallbackServer", &params).await?;
        Ok(resp
            .get("server_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0))
    }

    pub async fn get_confirmation_code(&self) -> VkResult<String> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), self.config.group_id.to_string());
        let resp = self
            .api
            .request("groups.getCallbackConfirmationCode", &params)
            .await?;
        Ok(resp
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string())
    }

    pub async fn get_servers(&self) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), self.config.group_id.to_string());
        self.api.request("groups.getCallbackServers", &params).await
    }

    pub async fn set_callback_settings(
        &self,
        server_id: i64,
        settings: HashMap<String, String>,
    ) -> VkResult<()> {
        let mut params = settings;
        params.insert("group_id".to_string(), self.config.group_id.to_string());
        params.insert("server_id".to_string(), server_id.to_string());
        self.api.request("groups.setCallbackSettings", &params).await?;
        Ok(())
    }

    pub async fn delete_server(&self, server_id: i64) -> VkResult<()> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), self.config.group_id.to_string());
        params.insert("server_id".to_string(), server_id.to_string());
        self.api.request("groups.deleteCallbackServer", &params).await?;
        Ok(())
    }

    pub async fn run(
        &self,
        router: Arc<DispatchRouter>,
        state_dispenser: Arc<dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> VkResult<()> {
        self.run_with_waiter(router, state_dispenser, Arc::new(crate::tools::waiter::WaiterMachine::new()))
            .await
    }

    pub async fn run_with_waiter(
        &self,
        router: Arc<DispatchRouter>,
        state_dispenser: Arc<dyn crate::dispatch::dispenser::StateDispenser>,
        waiter_machine: Arc<crate::tools::waiter::WaiterMachine>,
    ) -> VkResult<()> {
        let state = CallbackState {
            config: self.config.clone(),
            api: self.api.clone(),
            router,
            state_dispenser,
            waiter_machine,
        };

        let app = AxumRouter::new()
            .route("/", post(handle_callback))
            .with_state(state);

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| VkError::Internal(format!("bind {addr}: {e}")))?;

        tracing::info!("Callback server listening on {addr}");
        axum::serve(listener, app)
            .await
            .map_err(|e| VkError::Internal(e.to_string()))?;

        Ok(())
    }
}

async fn handle_callback(State(state): State<CallbackState>, body: Bytes) -> Response {
    let event: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Invalid callback JSON: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    if let Some(secret) = event.get("secret").and_then(|s| s.as_str()) {
        if secret != state.config.secret {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    if event_type == "confirmation" {
        return (StatusCode::OK, state.config.confirmation_code.clone()).into_response();
    }

    let _ = crate::tools::waiter::try_feed_message_waiters(
        &state.waiter_machine,
        "message",
        &event,
    )
    .await;

    let route_result = DispatchRouterTrait::route(
        &*state.router,
        &event,
        &state.api,
        Some(state.state_dispenser.as_ref()),
    )
    .await;

    match route_result {
        Ok(_) => callback_ok_response(),
        Err(e) => {
            tracing::error!("Callback dispatch error: {e}");
            callback_ok_response()
        }
    }
}

fn callback_ok_response() -> Response {
    (StatusCode::OK, "ok").into_response()
}

/// Parse callback body as JSON value
pub fn parse_event(bytes: &[u8]) -> Result<Value, VkError> {
    serde_json::from_slice(bytes).map_err(VkError::Json)
}

#[async_trait::async_trait]
impl Callback for BotCallback {
    fn config(&self) -> &CallbackConfig {
        &self.config
    }

    async fn register_server(&self) -> VkResult<i64> {
        BotCallback::register_server(self).await
    }

    async fn run(
        &self,
        router: Arc<DispatchRouter>,
        state_dispenser: Arc<dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> VkResult<()> {
        BotCallback::run(self, router, state_dispenser).await
    }
}
