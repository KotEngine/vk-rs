//! Middleware that resolves pending `WaiterMachine` waiters on incoming events

use async_trait::async_trait;

use crate::dispatch::{EventContext, Middleware, MiddlewareResult};
use crate::tools::waiter::WaiterMachine;

/// Feed message events into a shared waiter machine before handlers run
pub struct WaiterMiddleware {
    machine: std::sync::Arc<WaiterMachine>,
    view: String,
}

impl WaiterMiddleware {
    pub fn new(machine: std::sync::Arc<WaiterMachine>, view: impl Into<String>) -> Self {
        Self {
            machine,
            view: view.into(),
        }
    }

    pub fn for_messages(machine: std::sync::Arc<WaiterMachine>) -> Self {
        Self::new(machine, "message")
    }
}

#[async_trait]
impl Middleware<serde_json::Value> for WaiterMiddleware {
    async fn pre(&self, ctx: &mut EventContext) -> MiddlewareResult {
        let _ = crate::tools::waiter::try_feed_message_waiters(
            &self.machine,
            &self.view,
            &ctx.event,
        )
        .await;
        Ok(())
    }

    async fn post(&self, _ctx: &mut EventContext) {}
}
