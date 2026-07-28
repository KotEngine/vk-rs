//! Context injection middleware

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::dispatch::{EventContext, Middleware, MiddlewareResult};

/// Injects static context keys before handler runs
pub struct ContextMiddleware {
    values: Vec<(String, Value)>,
}

impl ContextMiddleware {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn insert(mut self, key: impl Into<String>, value: Value) -> Self {
        self.values.push((key.into(), value));
        self
    }

    pub fn insert_str(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.push((key.into(), json!(value.into())));
        self
    }
}

impl Default for ContextMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware<Value> for ContextMiddleware {
    async fn pre(&self, ctx: &mut EventContext) -> MiddlewareResult {
        for (k, v) in &self.values {
            ctx.context_update.insert(k.clone(), v.clone());
        }
        Ok(())
    }

    async fn post(&self, _ctx: &mut EventContext) {}
}

/// Stops event propagation after this middleware if condition matches
pub struct StopMiddleware {
    stop: bool,
}

impl StopMiddleware {
    pub fn always() -> Self {
        Self { stop: true }
    }

    pub fn never() -> Self {
        Self { stop: false }
    }
}

#[async_trait]
impl Middleware<Value> for StopMiddleware {
    async fn pre(&self, ctx: &mut EventContext) -> MiddlewareResult {
        if self.stop {
            ctx.can_forward = false;
        }
        Ok(())
    }

    async fn post(&self, _ctx: &mut EventContext) {}
}
