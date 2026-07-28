//! Dispatch module for vkontakte

use std::sync::Arc;

pub mod rules;
pub mod extractors;
pub mod handlers;
pub mod middlewares;
pub mod views;
pub mod dispenser;
pub mod return_manager;
pub mod router;
pub mod event_types;
pub mod state_context;

pub use rules::*;
pub use extractors::*;
pub use handlers::*;
pub use middlewares::*;
pub use views::*;
pub use dispenser::*;
pub use return_manager::*;
pub use router::*;
pub use event_types::*;
pub use state_context::*;

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Dispatch result
pub type DispatchResult<T> = Result<T, crate::exception::VkError>;

/// Rule result
pub enum RuleResult {
    Pass,
    Fail,
    Context(HashMap<String, Value>),
}

impl RuleResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
    
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail)
    }
    
    pub fn has_context(&self) -> bool {
        matches!(self, Self::Context(_))
    }
    
    pub fn context(self) -> Option<HashMap<String, Value>> {
        match self {
            Self::Context(ctx) => Some(ctx),
            _ => None,
        }
    }
    
    pub fn merge_context(self, other: HashMap<String, Value>) -> Self {
        match self {
            Self::Context(mut ctx) => {
                ctx.extend(other);
                Self::Context(ctx)
            },
            Self::Pass if !other.is_empty() => Self::Context(other),
            _ => self,
        }
    }
}

/// Router trait
#[async_trait]
pub trait Router: Send + Sync {
    /// Route an event
    async fn route(
        &self,
        event: &Value,
        api: &crate::api::Api,
        state_dispenser: Option<&dyn crate::dispatch::dispenser::StateDispenser>,
    ) -> DispatchResult<Value>;
    
    /// Register a handler
    fn register_handler(&mut self, handler: Arc<dyn Handler<Value>>) -> &mut Self;
    
    /// Register middleware
    fn register_middleware(&mut self, middleware: Box<dyn Middleware<Value>>) -> &mut Self;
    
    /// Get registered handlers
    fn handlers(&self) -> &[Arc<dyn Handler<Value>>];
    
    /// Get registered middleware
    fn middleware(&self) -> &[Box<dyn Middleware<Value>>];
}

/// Middleware result
pub type MiddlewareResult = Result<(), crate::exception::VkError>;

/// Event context
pub struct EventContext {
    pub event: Value,
    pub can_forward: bool,
    pub error: Option<crate::exception::VkError>,
    pub context_update: HashMap<String, Value>,
    pub handle_responses: Vec<Value>,
    /// API client, attached by the view that owns this dispatch.
    ///
    /// Present for handlers reached through a view; extractor-based handlers
    /// need it to build their arguments.
    pub api: Option<std::sync::Arc<crate::api::Api>>,
    /// Typed shared state registered on the bot.
    pub storage: Option<std::sync::Arc<crate::tools::ctx_storage::CtxStorage>>,
}

impl std::fmt::Debug for EventContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventContext")
            .field("event", &self.event)
            .field("can_forward", &self.can_forward)
            .field("error", &self.error)
            .field("context_update", &self.context_update)
            .field("handle_responses", &self.handle_responses)
            .field("api", &self.api.is_some())
            .field("storage", &self.storage.is_some())
            .finish()
    }
}

impl EventContext {
    pub fn new(event: Value) -> Self {
        Self {
            event,
            can_forward: true,
            error: None,
            context_update: HashMap::new(),
            handle_responses: Vec::new(),
            api: None,
            storage: None,
        }
    }

    /// Attach the API client and shared state used by extractor-based handlers.
    pub fn with_resources(
        mut self,
        api: std::sync::Arc<crate::api::Api>,
        storage: Option<std::sync::Arc<crate::tools::ctx_storage::CtxStorage>>,
    ) -> Self {
        self.api = Some(api);
        self.storage = storage;
        self
    }
    
    pub fn stop(&mut self) {
        self.can_forward = false;
    }
    
    pub fn send(&mut self, data: Value) {
        self.handle_responses.push(data);
    }
    
    pub fn update_context(&mut self, key: String, value: Value) {
        self.context_update.insert(key, value);
    }
    
    pub fn get_context(&self, key: &str) -> Option<&Value> {
        self.context_update.get(key)
    }
    
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
    
    pub fn set_error(&mut self, error: crate::exception::VkError) {
        self.error = Some(error);
    }
}