//! Base middleware implementation

use async_trait::async_trait;

use crate::dispatch::{EventContext, Middleware, MiddlewareResult};

/// Middleware context wrapper (alias for EventContext)
pub type MiddlewareContext = EventContext;

/// Logging middleware
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware<serde_json::Value> for LoggingMiddleware {
    async fn pre(&self, ctx: &mut EventContext) -> MiddlewareResult {
        tracing::debug!("Processing event: {:?}", ctx.event.get("type"));
        Ok(())
    }

    async fn post(&self, ctx: &mut EventContext) {
        if ctx.has_error() {
            tracing::error!("Event processing failed");
        }
    }
}

/// Error-catching middleware
pub struct ErrorMiddleware;

impl ErrorMiddleware {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Middleware<serde_json::Value> for ErrorMiddleware {
    async fn pre(&self, _ctx: &mut EventContext) -> MiddlewareResult {
        Ok(())
    }

    async fn post(&self, ctx: &mut EventContext) {
        if let Some(error) = ctx.error.take() {
            tracing::error!("Handler error: {}", error);
        }
    }
}
