//! vkontakte - Rust library for VK API
//!
//! Async VK Bot API framework for Rust.

// Layered glob re-exports (e.g. `rules::abc` / `middlewares::base`) intentionally
// expose submodules with the same name across sibling modules. The public types and
// functions they forward are unambiguous; only the module aliases collide.
#![allow(ambiguous_glob_reexports)]

pub mod api;
pub mod constants;
pub mod http;
pub mod polling;
pub mod callback;
pub mod dispatch;
pub mod framework;
pub mod tools;
pub mod exception;

pub use constants::{VK_API_URL, VK_API_VERSION};
pub use exception::{VkError, VkResult};

#[cfg(feature = "macros")]
pub use vkontakte_macros::{on_message, on_message_event, on_raw_event, StateGroup};

pub mod prelude {
    pub use crate::api::*;
    pub use crate::constants::*;
    pub use crate::callback::*;
    pub use crate::dispatch::*;
    pub use crate::framework::*;
    pub use crate::tools::*;
    pub use crate::exception::*;
}
