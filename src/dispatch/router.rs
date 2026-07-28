//! Event router

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::Api;
use crate::dispatch::handlers::Handler;
use crate::dispatch::middlewares::Middleware;
use crate::dispatch::views::View;
use crate::dispatch::{DispatchResult, EventContext, Router};

/// Default event router
pub struct DispatchRouter {
    handlers: Vec<Arc<dyn Handler<Value>>>,
    middleware: Vec<Box<dyn Middleware<Value>>>,
    views: Vec<Box<dyn View>>,
}

impl DispatchRouter {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            middleware: Vec::new(),
            views: Vec::new(),
        }
    }

    pub fn add_view(&mut self, view: Box<dyn View>) -> &mut Self {
        self.views.push(view);
        self
    }

    pub fn take_handlers(&mut self) -> Vec<Arc<dyn Handler<Value>>> {
        std::mem::take(&mut self.handlers)
    }

    pub fn take_middleware(&mut self) -> Vec<Box<dyn Middleware<Value>>> {
        std::mem::take(&mut self.middleware)
    }
}

impl Default for DispatchRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Router for DispatchRouter {
    #[tracing::instrument(
        name = "dispatch",
        skip_all,
        fields(
            event_type = event_type(event),
            peer_id = peer_id(event),
        )
    )]
    async fn route(
        &self,
        event: &Value,
        api: &Api,
        state_dispenser: Option<&dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> DispatchResult<Value> {
        tracing::debug!(
            views = self.views.len(),
            handlers = self.handlers.len(),
            middleware = self.middleware.len(),
            "incoming update"
        );

        let mut ctx = EventContext::new(event.clone());

        for mw in &self.middleware {
            mw.pre(&mut ctx).await?;
            if !ctx.can_forward {
                tracing::debug!("update stopped by middleware");
                return Ok(Value::Null);
            }
        }

        for (idx, view) in self.views.iter().enumerate() {
            if let Some(result) = view.process(event, api, state_dispenser).await? {
                tracing::debug!(view = idx, "handled by view");
                for mw in &self.middleware {
                    mw.post(&mut ctx).await;
                }
                return Ok(result);
            }
        }

        for handler in &self.handlers {
            if let Some(result) = handler.handle(event, &mut ctx).await? {
                tracing::debug!(handler = %handler.describe(), "handled by handler");
                for mw in &self.middleware {
                    mw.post(&mut ctx).await;
                }
                return Ok(result);
            }
        }

        tracing::debug!("no handler matched");

        for mw in &self.middleware {
            mw.post(&mut ctx).await;
        }

        Ok(Value::Null)
    }

    fn register_handler(&mut self, handler: Arc<dyn Handler<Value>>) -> &mut Self {
        self.handlers.push(handler);
        self
    }

    fn register_middleware(&mut self, middleware: Box<dyn Middleware<Value>>) -> &mut Self {
        self.middleware.push(middleware);
        self
    }

    fn handlers(&self) -> &[Arc<dyn Handler<Value>>] {
        &self.handlers
    }

    fn middleware(&self) -> &[Box<dyn Middleware<Value>>] {
        &self.middleware
    }
}

/// Default router implementation alias
pub type RouterImpl = DispatchRouter;

/// `type` of an update, for span fields.
fn event_type(event: &Value) -> &str {
    event.get("type").and_then(|t| t.as_str()).unwrap_or("unknown")
}

/// `peer_id` of an update, for span fields. `-1` when the update has none.
fn peer_id(event: &Value) -> i64 {
    crate::dispatch::rules::message_peer_id(event)
        .or_else(|| {
            event
                .get("object")
                .and_then(|o| o.get("peer_id"))
                .and_then(|p| p.as_i64())
        })
        .unwrap_or(-1)
}
