//! Framework module — Bot, User, blueprints

pub mod bot;
pub mod blueprint;
pub mod labeler;
pub mod multibot;
pub mod user;
pub mod bot_builder;
pub mod routes;
pub mod user_builder;

pub use bot::*;
pub use bot_builder::*;
pub use routes::*;
pub use user_builder::*;
pub use blueprint::*;
pub use labeler::*;
pub use multibot::*;
pub use user::*;

use async_trait::async_trait;

/// Framework trait
#[async_trait]
pub trait Framework: Send + Sync {
    async fn run_polling(&self) -> crate::exception::VkResult<()>;
    async fn on_startup(&self);
    async fn on_shutdown(&self);
}
