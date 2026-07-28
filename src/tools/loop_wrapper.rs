//! Compatibility runner for bots (startup tasks + polling)

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::exception::VkResult;
use crate::framework::Bot;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Run a bot with optional background tasks (replaces legacy loop wrapper pattern)
pub struct LoopRunner {
    startup_tasks: Vec<BoxFuture>,
    shutdown_tasks: Vec<BoxFuture>,
}

impl LoopRunner {
    pub fn new() -> Self {
        Self {
            startup_tasks: Vec::new(),
            shutdown_tasks: Vec::new(),
        }
    }

    pub fn on_startup<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.startup_tasks.push(Box::pin(f()));
        self
    }

    pub fn on_shutdown<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.shutdown_tasks.push(Box::pin(f()));
        self
    }

    pub async fn run(mut self, bot: &mut Bot) -> VkResult<()> {
        for task in self.startup_tasks.drain(..) {
            task.await;
        }
        let result = bot.run_polling().await;
        for task in self.shutdown_tasks.drain(..) {
            task.await;
        }
        result
    }
}

impl Default for LoopRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a detached background task tied to bot API
pub fn spawn_background<F>(f: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(f);
}

/// Shared API clone helper for background workers
pub fn shared_api_from_bot(bot: &Bot) -> Arc<crate::api::Api> {
    bot.api.clone()
}
