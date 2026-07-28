//! Rate-limiting rule — stops a handler from firing too often.
//!
//! ```no_run
//! use std::time::Duration;
//! use vkontakte::dispatch::rules::{CommandRule, CooldownRule};
//! # use vkontakte::framework::Bot;
//! # fn demo(bot: &mut Bot) {
//! bot.on()
//!     .message(Box::new(CooldownRule::per_user(Duration::from_secs(5))))
//!     .rule(Box::new(CommandRule::new("buy", vec!["/"], None)))
//!     .handle(|_msg, _ctx| async move { Ok(None) });
//! # }
//! ```

use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;

use crate::dispatch::RuleResult;
use super::abc::Rule;
use super::base::{extract_message, message_from_id, message_peer_id};

/// Number of tracked keys past which a rule prunes expired entries.
///
/// Without this the map grows once per unique user for the lifetime of the bot.
const PRUNE_THRESHOLD: usize = 1024;

/// What a cooldown is scoped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooldownMode {
    /// One cooldown per user (`from_id` / `user_id`).
    PerUser,
    /// One cooldown per conversation (`peer_id`).
    PerPeer,
    /// One cooldown shared by everyone.
    Global,
}

/// Passes at most once per `duration`, scoped by [`CooldownMode`].
///
/// The timer is only reset when the rule *passes*, so a user hammering a command
/// cannot keep pushing their own cooldown further out.
pub struct CooldownRule {
    mode: CooldownMode,
    duration: Duration,
    last_call: DashMap<i64, Instant>,
}

impl CooldownRule {
    /// Per-user cooldown — each user gets their own timer.
    pub fn per_user(duration: Duration) -> Self {
        Self::with_mode(CooldownMode::PerUser, duration)
    }

    /// Per-peer cooldown — a whole chat shares one timer.
    pub fn per_peer(duration: Duration) -> Self {
        Self::with_mode(CooldownMode::PerPeer, duration)
    }

    /// Global cooldown — the handler fires at most once per `duration` overall.
    pub fn global(duration: Duration) -> Self {
        Self::with_mode(CooldownMode::Global, duration)
    }

    pub fn with_mode(mode: CooldownMode, duration: Duration) -> Self {
        Self {
            mode,
            duration,
            last_call: DashMap::new(),
        }
    }

    pub fn mode(&self) -> CooldownMode {
        self.mode
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Time left before `key` may pass again, or `None` if it can pass now.
    pub fn remaining(&self, key: i64) -> Option<Duration> {
        let last = self.last_call.get(&key)?;
        self.duration.checked_sub(last.elapsed())
    }

    /// Forget every recorded timer.
    pub fn reset(&self) {
        self.last_call.clear();
    }

    /// Forget the timer for a single key.
    pub fn reset_key(&self, key: i64) {
        self.last_call.remove(&key);
    }

    /// Number of keys currently tracked.
    pub fn tracked_keys(&self) -> usize {
        self.last_call.len()
    }

    /// Key an event maps to under this rule's mode.
    ///
    /// Falls back to the raw event object for `message_event` updates, whose
    /// payload carries `user_id` / `peer_id` instead of a nested message.
    fn key_for(&self, event: &Value) -> Option<i64> {
        match self.mode {
            CooldownMode::Global => Some(0),
            CooldownMode::PerUser => message_from_id(event).or_else(|| object_field(event, "user_id")),
            CooldownMode::PerPeer => message_peer_id(event).or_else(|| object_field(event, "peer_id")),
        }
    }

    /// Drop entries whose cooldown has already expired.
    fn prune(&self) {
        let duration = self.duration;
        self.last_call.retain(|_, last| last.elapsed() < duration);
    }
}

#[async_trait]
impl Rule<Value> for CooldownRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let Some(key) = self.key_for(event) else {
            return RuleResult::Fail;
        };

        if let Some(last) = self.last_call.get(&key) {
            if last.elapsed() < self.duration {
                return RuleResult::Fail;
            }
        }

        if self.last_call.len() >= PRUNE_THRESHOLD {
            self.prune();
        }

        self.last_call.insert(key, Instant::now());
        RuleResult::Pass
    }

    fn description(&self) -> String {
        format!("CooldownRule({:?}, {:?})", self.mode, self.duration)
    }
}

/// Read an i64 field straight off `object` — used for non-message events.
fn object_field(event: &Value, field: &str) -> Option<i64> {
    if extract_message(event).is_some() {
        return None;
    }
    event
        .get("object")
        .and_then(|o| o.get(field))
        .or_else(|| event.get(field))
        .and_then(|v| v.as_i64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message(from_id: i64, peer_id: i64) -> Value {
        json!({
            "type": "message_new",
            "object": { "message": { "from_id": from_id, "peer_id": peer_id, "text": "hi" } }
        })
    }

    #[tokio::test]
    async fn second_call_within_window_fails() {
        let rule = CooldownRule::per_user(Duration::from_secs(60));
        let event = message(1, 1);

        assert!(rule.check(&event).await.is_pass());
        assert!(rule.check(&event).await.is_fail());
    }

    #[tokio::test]
    async fn per_user_keys_are_independent() {
        let rule = CooldownRule::per_user(Duration::from_secs(60));

        assert!(rule.check(&message(1, 100)).await.is_pass());
        assert!(rule.check(&message(2, 100)).await.is_pass());
        assert!(rule.check(&message(1, 100)).await.is_fail());
    }

    #[tokio::test]
    async fn per_peer_shares_one_timer() {
        let rule = CooldownRule::per_peer(Duration::from_secs(60));

        assert!(rule.check(&message(1, 100)).await.is_pass());
        // Different user, same chat — still cooling down.
        assert!(rule.check(&message(2, 100)).await.is_fail());
        assert!(rule.check(&message(3, 200)).await.is_pass());
    }

    #[tokio::test]
    async fn global_ignores_sender() {
        let rule = CooldownRule::global(Duration::from_secs(60));

        assert!(rule.check(&message(1, 100)).await.is_pass());
        assert!(rule.check(&message(2, 200)).await.is_fail());
        assert_eq!(rule.tracked_keys(), 1);
    }

    #[tokio::test]
    async fn expired_cooldown_passes_again() {
        let rule = CooldownRule::per_user(Duration::from_millis(30));
        let event = message(1, 1);

        assert!(rule.check(&event).await.is_pass());
        assert!(rule.check(&event).await.is_fail());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(rule.check(&event).await.is_pass());
    }

    #[tokio::test]
    async fn blocked_call_does_not_extend_window() {
        let rule = CooldownRule::per_user(Duration::from_millis(60));
        let event = message(1, 1);

        assert!(rule.check(&event).await.is_pass());
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(rule.check(&event).await.is_fail()); // must not restart the timer
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(rule.check(&event).await.is_pass());
    }

    #[tokio::test]
    async fn message_event_uses_object_fields() {
        let rule = CooldownRule::per_user(Duration::from_secs(60));
        let event = json!({
            "type": "message_event",
            "object": { "user_id": 42, "peer_id": 2000000001, "payload": {"a": 1} }
        });

        assert!(rule.check(&event).await.is_pass());
        assert!(rule.check(&event).await.is_fail());
    }

    #[tokio::test]
    async fn remaining_reports_time_left() {
        let rule = CooldownRule::per_user(Duration::from_secs(60));
        assert!(rule.remaining(1).is_none());

        rule.check(&message(1, 1)).await;
        assert!(rule.remaining(1).is_some());

        rule.reset_key(1);
        assert!(rule.remaining(1).is_none());
    }

    #[tokio::test]
    async fn unkeyable_event_fails() {
        let rule = CooldownRule::per_user(Duration::from_secs(60));
        assert!(rule.check(&json!({"type": "wall_post_new"})).await.is_fail());
    }

    #[tokio::test]
    async fn prune_drops_expired_entries() {
        let rule = CooldownRule::per_user(Duration::from_millis(1));
        for id in 0..10 {
            rule.check(&message(id, id)).await;
        }
        assert_eq!(rule.tracked_keys(), 10);

        tokio::time::sleep(Duration::from_millis(20)).await;
        rule.prune();
        assert_eq!(rule.tracked_keys(), 0);
    }
}
