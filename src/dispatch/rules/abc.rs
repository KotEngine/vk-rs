//! Rule trait definitions

use async_trait::async_trait;

use crate::dispatch::RuleResult;

/// Rule trait for message/event filtering
#[async_trait]
pub trait Rule<T: Send + Sync>: Send + Sync {
    /// Check if rule matches
    async fn check(&self, event: &T) -> RuleResult;

    /// Get rule description
    fn description(&self) -> String;
}
