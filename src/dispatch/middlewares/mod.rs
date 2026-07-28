//! Middleware module

pub mod base;
pub mod context;
pub mod waiter;

pub use base::*;
pub use context::*;
pub use waiter::*;

use async_trait::async_trait;

use crate::dispatch::{EventContext, MiddlewareResult};

/// Middleware trait
#[async_trait]
pub trait Middleware<E>: Send + Sync {
    async fn pre(&self, ctx: &mut EventContext) -> MiddlewareResult;
    async fn post(&self, ctx: &mut EventContext);
}
