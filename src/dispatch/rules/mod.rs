//! Rules module for vkontakte dispatch system

pub mod abc;
pub mod base;
pub mod cooldown;
pub mod payload_validator;

pub use abc::*;
pub use base::*;
pub use cooldown::*;
pub use payload_validator::*;

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::dispatch::RuleResult;

/// AND rule combination
pub struct AndRule<T> {
    left: Box<dyn Rule<T>>,
    right: Box<dyn Rule<T>>,
}

impl<T> AndRule<T> {
    pub fn new(left: Box<dyn Rule<T>>, right: Box<dyn Rule<T>>) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<T: Send + Sync> Rule<T> for AndRule<T> {
    async fn check(&self, event: &T) -> RuleResult {
        match self.left.check(event).await {
            RuleResult::Pass => self.right.check(event).await,
            RuleResult::Context(mut ctx) => match self.right.check(event).await {
                RuleResult::Pass => RuleResult::Context(ctx),
                RuleResult::Context(ctx2) => {
                    ctx.extend(ctx2);
                    RuleResult::Context(ctx)
                }
                RuleResult::Fail => RuleResult::Fail,
            },
            RuleResult::Fail => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("({}) AND ({})", self.left.description(), self.right.description())
    }
}

/// OR rule combination
pub struct OrRule<T> {
    left: Box<dyn Rule<T>>,
    right: Box<dyn Rule<T>>,
}

impl<T> OrRule<T> {
    pub fn new(left: Box<dyn Rule<T>>, right: Box<dyn Rule<T>>) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<T: Send + Sync> Rule<T> for OrRule<T> {
    async fn check(&self, event: &T) -> RuleResult {
        match self.left.check(event).await {
            RuleResult::Pass => RuleResult::Pass,
            RuleResult::Context(mut ctx) => match self.right.check(event).await {
                RuleResult::Pass => RuleResult::Pass,
                RuleResult::Context(ctx2) => {
                    ctx.extend(ctx2);
                    RuleResult::Context(ctx)
                }
                RuleResult::Fail => RuleResult::Context(ctx),
            },
            RuleResult::Fail => self.right.check(event).await,
        }
    }

    fn description(&self) -> String {
        format!("({}) OR ({})", self.left.description(), self.right.description())
    }
}

/// NOT rule combination
pub struct NotRule<T> {
    rule: Box<dyn Rule<T>>,
}

impl<T> NotRule<T> {
    pub fn new(rule: Box<dyn Rule<T>>) -> Self {
        Self { rule }
    }
}

#[async_trait]
impl<T: Send + Sync> Rule<T> for NotRule<T> {
    async fn check(&self, event: &T) -> RuleResult {
        match self.rule.check(event).await {
            RuleResult::Pass => RuleResult::Fail,
            RuleResult::Context(ctx) => RuleResult::Context(ctx),
            RuleResult::Fail => RuleResult::Pass,
        }
    }

    fn description(&self) -> String {
        format!("NOT ({})", self.rule.description())
    }
}

/// Rule combinators
impl<T: Send + Sync + 'static> std::ops::BitAnd<Box<dyn Rule<T>>> for Box<dyn Rule<T>> {
    type Output = Box<dyn Rule<T>>;

    fn bitand(self, rhs: Box<dyn Rule<T>>) -> Self::Output {
        Box::new(AndRule::new(self, rhs))
    }
}

impl<T: Send + Sync + 'static> std::ops::BitOr<Box<dyn Rule<T>>> for Box<dyn Rule<T>> {
    type Output = Box<dyn Rule<T>>;

    fn bitor(self, rhs: Box<dyn Rule<T>>) -> Self::Output {
        Box::new(OrRule::new(self, rhs))
    }
}

impl<T: Send + Sync + 'static> std::ops::Not for Box<dyn Rule<T>> {
    type Output = Box<dyn Rule<T>>;

    fn not(self) -> Self::Output {
        Box::new(NotRule::new(self))
    }
}

/// Rule factory for easy rule creation
pub struct RuleFactory;

impl RuleFactory {
    pub fn text(text: &str, ignore_case: bool) -> Box<dyn Rule<Value>> {
        Box::new(TextRule::new(text, ignore_case))
    }

    pub fn command(command: &str, prefixes: Vec<&str>, args_count: Option<usize>) -> Box<dyn Rule<Value>> {
        Box::new(CommandRule::new(command, prefixes, args_count))
    }

    pub fn regex(pattern: &str) -> Box<dyn Rule<Value>> {
        Box::new(RegexRule::new(pattern))
    }

    pub fn mention(mentioned: bool) -> Box<dyn Rule<Value>> {
        Box::new(MentionRule::new(mentioned))
    }

    pub fn peer(from_chat: bool) -> Box<dyn Rule<Value>> {
        Box::new(PeerRule::new(from_chat))
    }

    pub fn from_user() -> Box<dyn Rule<Value>> {
        Box::new(FromUserRule::new())
    }

    pub fn payload(payload: &str) -> Box<dyn Rule<Value>> {
        Box::new(PayloadRule::new(payload))
    }

    pub fn state(state: &str) -> Box<dyn Rule<Value>> {
        Box::new(StateRule::new(state))
    }

    pub fn no_state() -> Box<dyn Rule<Value>> {
        Box::new(StateRule::none())
    }

    pub fn cooldown_per_user(duration: std::time::Duration) -> Box<dyn Rule<Value>> {
        Box::new(CooldownRule::per_user(duration))
    }

    pub fn cooldown_per_peer(duration: std::time::Duration) -> Box<dyn Rule<Value>> {
        Box::new(CooldownRule::per_peer(duration))
    }

    pub fn cooldown_global(duration: std::time::Duration) -> Box<dyn Rule<Value>> {
        Box::new(CooldownRule::global(duration))
    }

    pub fn func<F>(func: F) -> Box<dyn Rule<Value>>
    where
        F: Fn(&Value) -> RuleResult + Send + Sync + 'static,
    {
        Box::new(FuncRule::new(func))
    }
}

/// Rule utilities
pub struct RuleUtils;

impl RuleUtils {
    pub fn text_matches(text: &str, pattern: &str, ignore_case: bool) -> bool {
        if ignore_case {
            text.to_lowercase() == pattern.to_lowercase()
        } else {
            text == pattern
        }
    }

    pub fn extract_command_and_args(text: &str, prefixes: &[&str]) -> Option<(String, Vec<String>)> {
        for prefix in prefixes {
            if text.starts_with(prefix) {
                let remaining = text[prefix.len()..].trim();
                let parts: Vec<String> = remaining.split_whitespace().map(|s| s.to_string()).collect();

                if parts.is_empty() {
                    return Some((prefix.to_string(), Vec::new()));
                }

                let command = parts[0].clone();
                let args = parts[1..].to_vec();
                return Some((command, args));
            }
        }
        None
    }

    pub fn is_user_mentioned(text: &str, user_id: i64) -> bool {
        let mention_pattern = format!("[id{}|", user_id);
        text.contains(&mention_pattern)
    }

    pub fn extract_context(rule_result: &RuleResult) -> HashMap<String, Value> {
        match rule_result {
            RuleResult::Context(ctx) => ctx.clone(),
            _ => HashMap::new(),
        }
    }
}
