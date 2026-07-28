//! Fluent builder for configuring `Bot` instances

use std::path::PathBuf;
use std::sync::Arc;

use crate::callback::CallbackConfig;
use crate::dispatch::dispenser::{FileStateDispenser, StateDispenser};
use crate::exception::{ErrorHandler, VkResult};
use crate::framework::{Bot, BotBlueprint};
use crate::tools::waiter::{SharedWaiter, WaiterMachine};

/// Configure a bot before first `run_polling` / `run_callback`
pub struct BotBuilder {
    token: String,
    group_id: Option<i64>,
    state_file: Option<PathBuf>,
    custom_dispenser: Option<Arc<dyn StateDispenser>>,
    waiter: Option<SharedWaiter>,
    blueprints: Vec<BotBlueprint>,
    error_handler: Option<Box<dyn ErrorHandler>>,
}

impl BotBuilder {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            group_id: None,
            state_file: None,
            custom_dispenser: None,
            waiter: None,
            blueprints: Vec::new(),
            error_handler: None,
        }
    }

    pub fn group_id(mut self, id: i64) -> Self {
        self.group_id = Some(id);
        self
    }

    pub fn persistent_state_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.state_file = Some(path.into());
        self
    }

    pub fn state_dispenser(mut self, dispenser: Arc<dyn StateDispenser>) -> Self {
        self.custom_dispenser = Some(dispenser);
        self
    }

    pub fn waiter_machine(mut self, machine: SharedWaiter) -> Self {
        self.waiter = Some(machine);
        self
    }

    pub fn include_blueprint(mut self, bp: BotBlueprint) -> Self {
        self.blueprints.push(bp);
        self
    }

    pub fn error_handler(mut self, handler: Box<dyn ErrorHandler>) -> Self {
        self.error_handler = Some(handler);
        self
    }

    pub async fn build(mut self) -> VkResult<Bot> {
        let mut bot = Bot::new(&self.token)?;

        if let Some(gid) = self.group_id {
            bot = bot.with_group_id(gid);
        }

        if let Some(path) = self.state_file.take() {
            let dispenser = Arc::new(FileStateDispenser::open(path).await?);
            bot = bot.with_state_dispenser(dispenser);
        } else if let Some(dispenser) = self.custom_dispenser.take() {
            bot = bot.with_state_dispenser(dispenser);
        }

        if let Some(waiter) = self.waiter.take() {
            bot = bot.with_waiter_machine(waiter);
        }

        if let Some(handler) = self.error_handler.take() {
            bot.error_handler = handler;
        }

        for bp in self.blueprints {
            bot.include(bp);
        }

        Ok(bot)
    }
}

/// Callback server preset from env vars
pub struct CallbackBuilder {
    group_id: i64,
    secret: String,
    confirmation: String,
    url: String,
    host: String,
    port: u16,
}

impl CallbackBuilder {
    pub fn new(
        group_id: i64,
        secret: impl Into<String>,
        confirmation: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            group_id,
            secret: secret.into(),
            confirmation: confirmation.into(),
            url: url.into(),
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }

    pub fn from_env(group_id: i64) -> Option<Self> {
        let secret = std::env::var("VK_CALLBACK_SECRET").ok()?;
        let confirmation = std::env::var("VK_CALLBACK_CONFIRMATION").ok()?;
        let url = std::env::var("VK_CALLBACK_URL").ok()?;
        Some(Self::new(group_id, secret, confirmation, url))
    }

    pub fn listen(mut self, host: impl Into<String>, port: u16) -> Self {
        self.host = host.into();
        self.port = port;
        self
    }

    pub fn build(self) -> CallbackConfig {
        CallbackConfig::new(
            self.group_id,
            self.secret,
            self.confirmation,
            self.url,
        )
        .with_listen(self.host, self.port)
    }
}

/// Shared waiter with custom capacity
pub fn default_waiter(max_per_view: usize) -> SharedWaiter {
    Arc::new(WaiterMachine::with_capacity(max_per_view))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bot_builder_without_persistence() {
        let bot = BotBuilder::new("dummy")
            .group_id(1)
            .build()
            .await
            .unwrap();
        assert_eq!(bot.group_id(), 1);
    }

    #[test]
    fn callback_builder_listen() {
        let cfg = CallbackBuilder::new(1, "s", "c", "https://example.com")
            .listen("127.0.0.1", 3000)
            .build();
        assert_eq!(cfg.port, 3000);
    }
}
