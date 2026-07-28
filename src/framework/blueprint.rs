//! Blueprints for modular handler organization.
//!
//! A [`BotBlueprint`] / [`UserBlueprint`] lets you split a bot into independent
//! modules (admin, profile, music, ...) and compose them at startup:
//!
//! ```no_run
//! use vkontakte::prelude::*;
//! use vkontakte::framework::BotBlueprint;
//! use vkontakte::dispatch::rules::CommandRule;
//!
//! fn admin_blueprint() -> BotBlueprint {
//!     let mut bp = BotBlueprint::new().with_name("admin");
//!     bp.on().message(Box::new(CommandRule::new("ban", vec!["/"], None)));
//!     bp
//! }
//!
//! # async fn run() -> vkontakte::VkResult<()> {
//! let mut bot = Bot::new("token")?;
//! bot.mount(admin_blueprint());
//! bot.run_polling().await
//! # }
//! ```

use serde_json::Value;

use crate::dispatch::middlewares::Middleware;
use crate::framework::{BotLabeler, UserLabeler};
use crate::framework::labeler::{BotOn, UserOn};

/// Blueprint for splitting bot handlers into modules.
pub struct BotBlueprint {
    pub labeler: BotLabeler,
    pub name: String,
    /// Middleware scoped to this blueprint, applied when the bot mounts it.
    pub(crate) middleware: Vec<Box<dyn Middleware<Value>>>,
    /// Nested blueprints flattened into this one on mount.
    pub(crate) nested: Vec<BotBlueprint>,
}

impl BotBlueprint {
    /// Create an empty blueprint.
    pub fn new() -> Self {
        Self {
            labeler: BotLabeler::new(),
            name: String::new(),
            middleware: Vec::new(),
            nested: Vec::new(),
        }
    }

    /// Name the blueprint — surfaced by router introspection (`dump_routes()`).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Begin a handler-registration chain backed by this blueprint's labeler.
    ///
    /// Mirrors [`crate::framework::Bot::on`] so the same fluent API works inside
    /// a module and on the top-level bot.
    pub fn on(&mut self) -> BotOn<'_> {
        BotOn {
            labeler: &mut self.labeler,
        }
    }

    /// Register a middleware scoped to this blueprint.
    ///
    /// When the bot mounts the blueprint, these are forwarded to the bot's
    /// router alongside the blueprint's handlers.
    pub fn middleware<M>(&mut self, middleware: M)
    where
        M: Middleware<Value> + 'static,
    {
        self.middleware.push(Box::new(middleware));
    }

    /// Merge another blueprint into this one.
    ///
    /// Handlers, middleware and nested blueprints are absorbed from `other`,
    /// which is consumed.
    pub fn include(&mut self, mut other: BotBlueprint) {
        for handler in other.labeler.cloned_message_handlers() {
            self.labeler.push_message_handler(handler);
        }
        for handler in other.labeler.cloned_message_event_handlers() {
            self.labeler.push_message_event_handler(handler);
        }
        for (event_type, handlers) in other.labeler.cloned_raw_handlers() {
            for handler in handlers {
                self.labeler.push_raw_handler(event_type.clone(), handler);
            }
        }
        for handler in other.labeler.cloned_value_handlers() {
            self.labeler.value_handlers.push(handler);
        }
        self.middleware.append(&mut other.middleware);
        self.nested.append(&mut other.nested);
    }

    /// Number of handlers registered directly on this blueprint
    /// (excluding nested blueprints).
    pub fn handler_count(&self) -> usize {
        self.labeler.message_handler_count()
            + self.labeler.value_handlers.len()
    }

    /// Drain blueprint-local middleware (used by `Bot::mount`).
    pub(crate) fn take_middleware(&mut self) -> Vec<Box<dyn Middleware<Value>>> {
        std::mem::take(&mut self.middleware)
    }

    /// Drain nested blueprints (used by `Bot::mount`).
    pub(crate) fn take_nested(&mut self) -> Vec<BotBlueprint> {
        std::mem::take(&mut self.nested)
    }
}

impl Default for BotBlueprint {
    fn default() -> Self {
        Self::new()
    }
}

/// Blueprint for splitting user account handlers into modules.
pub struct UserBlueprint {
    pub labeler: UserLabeler,
    pub name: String,
    pub(crate) middleware: Vec<Box<dyn Middleware<Value>>>,
    pub(crate) nested: Vec<UserBlueprint>,
}

impl UserBlueprint {
    pub fn new() -> Self {
        Self {
            labeler: UserLabeler::new(),
            name: String::new(),
            middleware: Vec::new(),
            nested: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Begin a handler-registration chain, mirroring [`crate::framework::User::on`].
    pub fn on(&mut self) -> UserOn<'_> {
        UserOn {
            labeler: &mut self.labeler,
        }
    }

    pub fn middleware<M>(&mut self, middleware: M)
    where
        M: Middleware<Value> + 'static,
    {
        self.middleware.push(Box::new(middleware));
    }

    pub fn include(&mut self, mut other: UserBlueprint) {
        for handler in other.labeler.cloned_message_handlers() {
            self.labeler.push_message_handler(handler);
        }
        for handler in other.labeler.cloned_raw_handlers() {
            self.labeler.push_raw_handler(handler);
        }
        self.middleware.append(&mut other.middleware);
        self.nested.append(&mut other.nested);
    }

    pub fn handler_count(&self) -> usize {
        self.labeler.message_handler_count()
    }

    pub(crate) fn take_middleware(&mut self) -> Vec<Box<dyn Middleware<Value>>> {
        std::mem::take(&mut self.middleware)
    }

    pub(crate) fn take_nested(&mut self) -> Vec<UserBlueprint> {
        std::mem::take(&mut self.nested)
    }
}

impl Default for UserBlueprint {
    fn default() -> Self {
        Self::new()
    }
}
