//! Event handlers

pub mod func_handler;
pub mod message_handler;
pub mod message_event_handler;
pub mod message_reply;

pub use func_handler::*;
pub use message_handler::*;
pub use message_event_handler::*;
pub use message_reply::*;

use async_trait::async_trait;
use serde_json::Value;

use crate::dispatch::{DispatchResult, EventContext, RuleResult};
use crate::dispatch::rules::Rule;

/// Evaluate rules in order, stopping at the first failure.
///
/// Context returned by individual rules is merged, so a later rule can see — and
/// overwrite — keys set by an earlier one.
pub async fn evaluate_rules<E: Send + Sync>(rules: &[Box<dyn Rule<E>>], event: &E) -> RuleResult {
    let mut combined = RuleResult::Pass;

    for rule in rules {
        let result = rule.check(event).await;

        if result.is_fail() {
            tracing::trace!(rule = %rule.description(), "rule rejected event");
            return RuleResult::Fail;
        }

        combined = match (combined, result) {
            (RuleResult::Fail, _) => return RuleResult::Fail,
            (RuleResult::Pass, r) => r,
            (RuleResult::Context(ctx), RuleResult::Pass) => RuleResult::Context(ctx),
            (RuleResult::Context(mut ctx), RuleResult::Context(ctx2)) => {
                ctx.extend(ctx2);
                RuleResult::Context(ctx)
            }
            (ctx, RuleResult::Fail) => {
                let _ = ctx;
                return RuleResult::Fail;
            }
        };
    }

    combined
}

/// Handler trait
#[async_trait]
pub trait Handler<E: Send + Sync>: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &E, ctx: &mut EventContext) -> DispatchResult<Option<Value>>;

    /// Rules that must pass before handling
    fn rules(&self) -> &[Box<dyn Rule<E>>];

    /// Human-readable label for router introspection.
    fn describe(&self) -> String {
        let rules = self
            .rules()
            .iter()
            .map(|r| r.description())
            .collect::<Vec<_>>()
            .join(", ");
        if rules.is_empty() {
            "[no rules]".to_string()
        } else {
            format!("[{rules}]")
        }
    }

    /// Check all rules
    async fn check_rules(&self, event: &E) -> RuleResult {
        evaluate_rules(self.rules(), event).await
    }
}
