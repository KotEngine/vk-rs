//! Built-in dispatch rules

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::dispatch::RuleResult;
use super::abc::Rule;
use super::RuleUtils;

/// Extract message object from a VK event
pub fn extract_message(event: &Value) -> Option<&Value> {
    event
        .get("object")
        .and_then(|o| o.get("message"))
        .or_else(|| event.get("message"))
}

pub fn message_text(event: &Value) -> Option<&str> {
    extract_message(event).and_then(|m| m.get("text")).and_then(|t| t.as_str())
}

pub fn message_peer_id(event: &Value) -> Option<i64> {
    extract_message(event)
        .and_then(|m| m.get("peer_id"))
        .and_then(|p| p.as_i64())
}

pub fn message_from_id(event: &Value) -> Option<i64> {
    extract_message(event)
        .and_then(|m| m.get("from_id"))
        .and_then(|f| f.as_i64())
}

/// Exact text match
pub struct TextRule {
    text: String,
    ignore_case: bool,
}

impl TextRule {
    pub fn new(text: impl Into<String>, ignore_case: bool) -> Self {
        Self {
            text: text.into(),
            ignore_case,
        }
    }
}

#[async_trait]
impl Rule<Value> for TextRule {
    async fn check(&self, event: &Value) -> RuleResult {
        match message_text(event) {
            Some(text) if RuleUtils::text_matches(text, &self.text, self.ignore_case) => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("TextRule({})", self.text)
    }
}

/// Command rule with prefix and optional args count
pub struct CommandRule {
    command: String,
    prefixes: Vec<String>,
    args_count: Option<usize>,
}

impl CommandRule {
    pub fn new(command: impl Into<String>, prefixes: Vec<&str>, args_count: Option<usize>) -> Self {
        Self {
            command: command.into(),
            prefixes: prefixes.into_iter().map(String::from).collect(),
            args_count,
        }
    }
}

#[async_trait]
impl Rule<Value> for CommandRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let text = match message_text(event) {
            Some(t) => t,
            None => return RuleResult::Fail,
        };

        let prefix_refs: Vec<&str> = self.prefixes.iter().map(String::as_str).collect();
        let Some((cmd, args)) = RuleUtils::extract_command_and_args(text, &prefix_refs) else {
            return RuleResult::Fail;
        };

        if cmd != self.command {
            return RuleResult::Fail;
        }

        if let Some(expected) = self.args_count {
            if args.len() != expected {
                return RuleResult::Fail;
            }
        }

        let mut ctx = HashMap::new();
        ctx.insert("args".to_string(), Value::Array(args.into_iter().map(Value::String).collect()));
        RuleResult::Context(ctx)
    }

    fn description(&self) -> String {
        format!("CommandRule({})", self.command)
    }
}

/// Regex rule on message text
pub struct RegexRule {
    pattern: Regex,
}

impl RegexRule {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap_or_else(|_| Regex::new("$^").unwrap()),
        }
    }
}

#[async_trait]
impl Rule<Value> for RegexRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let text = match message_text(event) {
            Some(t) => t,
            None => return RuleResult::Fail,
        };

        let Some(captures) = self.pattern.captures(text) else {
            return RuleResult::Fail;
        };

        let mut ctx = HashMap::new();
        let groups: Vec<Value> = captures
            .iter()
            .flatten()
            .map(|m| Value::String(m.as_str().to_string()))
            .collect();
        ctx.insert("match".to_string(), Value::Array(groups));
        RuleResult::Context(ctx)
    }

    fn description(&self) -> String {
        format!("RegexRule({})", self.pattern.as_str())
    }
}

/// Peer type rule (chat vs PM)
pub struct PeerRule {
    from_chat: bool,
}

impl PeerRule {
    pub fn new(from_chat: bool) -> Self {
        Self { from_chat }
    }
}

#[async_trait]
impl Rule<Value> for PeerRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let peer_id = match message_peer_id(event) {
            Some(id) => id,
            None => return RuleResult::Fail,
        };

        let is_chat = peer_id > 2_000_000_000;
        if is_chat == self.from_chat {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        format!("PeerRule(from_chat={})", self.from_chat)
    }
}

/// Mention rule
pub struct MentionRule {
    mentioned: bool,
}

impl MentionRule {
    pub fn new(mentioned: bool) -> Self {
        Self { mentioned }
    }
}

#[async_trait]
impl Rule<Value> for MentionRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let is_mentioned = extract_message(event)
            .and_then(|m| m.get("is_mentioned"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_mentioned == self.mentioned {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        format!("MentionRule({})", self.mentioned)
    }
}

/// From real user rule
pub struct FromUserRule {
    from_user: bool,
}

impl FromUserRule {
    pub fn new() -> Self {
        Self { from_user: true }
    }

    pub fn with_expected(from_user: bool) -> Self {
        Self { from_user }
    }
}

#[async_trait]
impl Rule<Value> for FromUserRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let is_user = message_from_id(event).map(|id| id > 0).unwrap_or(false);
        if is_user == self.from_user {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        format!("FromUserRule(from_user={})", self.from_user)
    }
}

/// Payload exact match rule.
///
/// Works for both keyboard buttons on a message (where VK sends the payload as a
/// JSON *string*) and callback buttons on a `message_event` (where it arrives as
/// a JSON *value*). When the expected payload parses as JSON the comparison is
/// structural, so key order and whitespace do not matter.
pub struct PayloadRule {
    payload: String,
    parsed: Option<Value>,
}

impl PayloadRule {
    pub fn new(payload: impl Into<String>) -> Self {
        let payload = payload.into();
        let parsed = serde_json::from_str::<Value>(&payload).ok();
        Self { payload, parsed }
    }

    /// Build from a JSON value instead of its string form.
    pub fn from_json(payload: Value) -> Self {
        Self {
            payload: payload.to_string(),
            parsed: Some(payload),
        }
    }
}

#[async_trait]
impl Rule<Value> for PayloadRule {
    async fn check(&self, event: &Value) -> RuleResult {
        if let Some(expected) = &self.parsed {
            if extract_payload_value(event).as_ref() == Some(expected) {
                return RuleResult::Pass;
            }
            return RuleResult::Fail;
        }

        // Non-JSON payload — fall back to a raw string comparison.
        let raw = extract_message(event)
            .and_then(|m| m.get("payload"))
            .and_then(|p| p.as_str());

        match raw {
            Some(p) if p == self.payload => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("PayloadRule({})", self.payload)
    }
}

/// FSM state rule — pass `StateRule::none()` for handlers without state
pub struct StateRule {
    states: Vec<String>,
}

impl StateRule {
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            states: vec![state.into()],
        }
    }

    pub fn any_of(states: Vec<String>) -> Self {
        Self { states }
    }

    /// Match only when peer has no FSM state
    pub fn none() -> Self {
        Self { states: Vec::new() }
    }
}

#[async_trait]
impl Rule<Value> for StateRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let peer = super::super::state_context::extract_state_peer(event);

        if self.states.is_empty() {
            return if peer.is_none() {
                RuleResult::Pass
            } else {
                RuleResult::Fail
            };
        }

        match peer {
            Some(p) if self.states.iter().any(|s| s == &p.state) => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        if self.states.is_empty() {
            "StateRule(none)".to_string()
        } else {
            format!("StateRule({:?})", self.states)
        }
    }
}

/// Custom function rule (sync)
pub struct FuncRule {
    func: Arc<dyn Fn(&Value) -> RuleResult + Send + Sync>,
    desc: String,
}

impl FuncRule {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&Value) -> RuleResult + Send + Sync + 'static,
    {
        Self {
            func: Arc::new(func),
            desc: "FuncRule".to_string(),
        }
    }
}

#[async_trait]
impl Rule<Value> for FuncRule {
    async fn check(&self, event: &Value) -> RuleResult {
        (self.func)(event)
    }

    fn description(&self) -> String {
        self.desc.clone()
    }
}

/// Async custom function rule
pub struct CoroutineRule {
    func: Arc<
        dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = RuleResult> + Send>>
            + Send
            + Sync,
    >,
}

impl CoroutineRule {
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RuleResult> + Send + 'static,
    {
        Self {
            func: Arc::new(move |event| Box::pin(func(event))),
        }
    }
}

#[async_trait]
impl Rule<Value> for CoroutineRule {
    async fn check(&self, event: &Value) -> RuleResult {
        (self.func)(event.clone()).await
    }

    fn description(&self) -> String {
        "CoroutineRule".to_string()
    }
}

/// Reply message rule
pub struct ReplyMessageRule;

impl ReplyMessageRule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Rule<Value> for ReplyMessageRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let has_reply = extract_message(event)
            .and_then(|m| m.get("reply_message"))
            .is_some();
        if has_reply {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        "ReplyMessageRule".to_string()
    }
}

/// Forward messages rule
pub struct ForwardMessagesRule;

impl ForwardMessagesRule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Rule<Value> for ForwardMessagesRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let has_fwd = extract_message(event)
            .and_then(|m| m.get("fwd_messages"))
            .and_then(|f| f.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if has_fwd {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        "ForwardMessagesRule".to_string()
    }
}

/// Attachment type rule
pub struct AttachmentTypeRule {
    attachment_type: String,
}

impl AttachmentTypeRule {
    pub fn new(attachment_type: impl Into<String>) -> Self {
        Self {
            attachment_type: attachment_type.into(),
        }
    }
}

#[async_trait]
impl Rule<Value> for AttachmentTypeRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let attachments = extract_message(event)
            .and_then(|m| m.get("attachments"))
            .and_then(|a| a.as_array());

        let Some(attachments) = attachments else {
            return RuleResult::Fail;
        };

        for att in attachments {
            if att.get("type").and_then(|t| t.as_str()) == Some(self.attachment_type.as_str()) {
                return RuleResult::Pass;
            }
        }
        RuleResult::Fail
    }

    fn description(&self) -> String {
        format!("AttachmentTypeRule({})", self.attachment_type)
    }
}

/// Message length rule
pub struct MessageLengthRule {
    min_length: usize,
}

impl MessageLengthRule {
    pub fn new(min_length: usize) -> Self {
        Self { min_length }
    }
}

#[async_trait]
impl Rule<Value> for MessageLengthRule {
    async fn check(&self, event: &Value) -> RuleResult {
        match message_text(event) {
            Some(text) if text.len() >= self.min_length => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("MessageLengthRule(min={})", self.min_length)
    }
}

/// Whitelist peer IDs rule
pub struct FromPeerRule {
    peer_ids: Vec<i64>,
}

impl FromPeerRule {
    pub fn new(peer_ids: Vec<i64>) -> Self {
        Self { peer_ids }
    }
}

#[async_trait]
impl Rule<Value> for FromPeerRule {
    async fn check(&self, event: &Value) -> RuleResult {
        match message_peer_id(event) {
            Some(id) if self.peer_ids.contains(&id) => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("FromPeerRule({:?})", self.peer_ids)
    }
}

/// Sticker rule
pub struct StickerRule {
    sticker_ids: Option<Vec<i64>>,
}

impl StickerRule {
    pub fn new(sticker_ids: Option<Vec<i64>>) -> Self {
        Self { sticker_ids }
    }
}

#[async_trait]
impl Rule<Value> for StickerRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let attachments = extract_message(event)
            .and_then(|m| m.get("attachments"))
            .and_then(|a| a.as_array());

        let Some(attachments) = attachments else {
            return RuleResult::Fail;
        };

        for att in attachments {
            if att.get("type").and_then(|t| t.as_str()) != Some("sticker") {
                continue;
            }
            if let Some(ids) = &self.sticker_ids {
                let sticker_id = att
                    .get("sticker")
                    .and_then(|s| s.get("sticker_id"))
                    .and_then(|id| id.as_i64());
                if sticker_id.map(|id| ids.contains(&id)).unwrap_or(false) {
                    return RuleResult::Pass;
                }
            } else {
                return RuleResult::Pass;
            }
        }
        RuleResult::Fail
    }

    fn description(&self) -> String {
        "StickerRule".to_string()
    }
}

/// Geo rule
pub struct GeoRule;

impl GeoRule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Rule<Value> for GeoRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let has_geo = extract_message(event).and_then(|m| m.get("geo")).is_some();
        if has_geo {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        "GeoRule".to_string()
    }
}

/// Chat action rule
pub struct ChatActionRule {
    action_type: Option<String>,
}

impl ChatActionRule {
    pub fn new(action_type: Option<String>) -> Self {
        Self { action_type }
    }
}

#[async_trait]
impl Rule<Value> for ChatActionRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let action = extract_message(event)
            .and_then(|m| m.get("action"))
            .and_then(|a| a.get("type"))
            .and_then(|t| t.as_str());

        match (&self.action_type, action) {
            (None, Some(_)) => RuleResult::Pass,
            (Some(expected), Some(actual)) if expected == actual => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        "ChatActionRule".to_string()
    }
}

/// Payload contains rule
pub struct PayloadContainsRule {
    key: String,
    value: Value,
}

impl PayloadContainsRule {
    pub fn new(key: impl Into<String>, value: Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

#[async_trait]
impl Rule<Value> for PayloadContainsRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let Some(payload) = extract_payload_value(event) else {
            return RuleResult::Fail;
        };

        if payload.get(&self.key) == Some(&self.value) {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        format!("PayloadContainsRule({})", self.key)
    }
}

/// Passes when the payload has the given key, whatever its value.
pub struct PayloadHasKeyRule {
    key: String,
}

impl PayloadHasKeyRule {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

#[async_trait]
impl Rule<Value> for PayloadHasKeyRule {
    async fn check(&self, event: &Value) -> RuleResult {
        match extract_payload_value(event) {
            Some(payload) if payload.get(&self.key).is_some() => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("PayloadHasKeyRule({})", self.key)
    }
}

/// Fuzzy text match by Levenshtein distance
pub struct LevenshteinRule {
    text: String,
    max_distance: usize,
}

impl LevenshteinRule {
    pub fn new(text: impl Into<String>, max_distance: usize) -> Self {
        Self {
            text: text.into(),
            max_distance,
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().skip(1) {
        *val = j;
    }
    for (i, ca) in a.iter().enumerate() {
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            dp[i + 1][j + 1] = (dp[i][j + 1] + 1)
                .min(dp[i + 1][j] + 1)
                .min(dp[i][j] + cost);
        }
    }
    dp[a.len()][b.len()]
}

#[async_trait]
impl Rule<Value> for LevenshteinRule {
    async fn check(&self, event: &Value) -> RuleResult {
        match message_text(event) {
            Some(text) if levenshtein(text, &self.text) <= self.max_distance => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("LevenshteinRule({})", self.text)
    }
}

/// Fuzzy text ratio rule
pub struct FuzzyTextRule {
    text: String,
    min_ratio: f64,
}

impl FuzzyTextRule {
    pub fn new(text: impl Into<String>, min_ratio: f64) -> Self {
        Self {
            text: text.into(),
            min_ratio,
        }
    }
}

#[async_trait]
impl Rule<Value> for FuzzyTextRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let Some(text) = message_text(event) else {
            return RuleResult::Fail;
        };
        let max_len = text.len().max(self.text.len()).max(1);
        let distance = levenshtein(text, &self.text);
        let ratio = 1.0 - (distance as f64 / max_len as f64);
        if ratio >= self.min_ratio {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        format!("FuzzyTextRule({})", self.text)
    }
}

/// State group rule — matches any state belonging to named groups
pub struct StateGroupRule {
    groups: Vec<String>,
}

impl StateGroupRule {
    pub fn new(groups: Vec<String>) -> Self {
        Self { groups }
    }

    pub fn group(name: impl Into<String>) -> Self {
        Self {
            groups: vec![name.into()],
        }
    }
}

#[async_trait]
impl Rule<Value> for StateGroupRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let peer = super::super::state_context::extract_state_peer(event);

        match peer {
            Some(p) => {
                let group = super::super::state_context::state_group_name(&p.state);
                if self.groups.iter().any(|g| g == group) {
                    RuleResult::Pass
                } else {
                    RuleResult::Fail
                }
            }
            None if self.groups.is_empty() => RuleResult::Pass,
            None => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("StateGroupRule({:?})", self.groups)
    }
}

/// Admin check rule — passes when sender is chat admin (not in private chats)
pub struct IsAdminRule {
    api: Arc<crate::api::Api>,
}

impl IsAdminRule {
    pub fn new(api: Arc<crate::api::Api>) -> Self {
        Self { api }
    }
}

#[async_trait]
impl Rule<Value> for IsAdminRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let peer_id = match message_peer_id(event) {
            Some(p) => p,
            None => return RuleResult::Fail,
        };
        let from_id = match message_from_id(event) {
            Some(f) => f,
            None => return RuleResult::Fail,
        };

        if peer_id == from_id {
            return RuleResult::Fail;
        }

        match crate::tools::mini_types::user_is_admin_in_chat(&self.api, peer_id, from_id).await {
            Ok(true) => RuleResult::Pass,
            _ => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        "IsAdminRule".to_string()
    }
}

/// VBML pattern rule
pub struct VBMLRule {
    pattern: crate::tools::vbml::Pattern,
    description: String,
}

impl VBMLRule {
    pub fn new(pattern: &str) -> Self {
        let compiled = crate::tools::vbml::Pattern::compile(pattern)
            .unwrap_or_else(|_| crate::tools::vbml::Pattern::compile("$^").unwrap());
        Self {
            pattern: compiled,
            description: pattern.to_string(),
        }
    }
}

#[async_trait]
impl Rule<Value> for VBMLRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let text = match message_text(event) {
            Some(t) => t,
            None => return RuleResult::Fail,
        };
        match self.pattern.check(text) {
            Some(captures) => {
                let mut ctx = HashMap::new();
                for (k, v) in captures {
                    ctx.insert(k, v);
                }
                RuleResult::Context(ctx)
            }
            None => RuleResult::Fail,
        }
    }

    fn description(&self) -> String {
        format!("VBMLRule({})", self.description)
    }
}

/// Pattern macro rule (multiple VBML patterns, first match wins)
pub struct MacroRule {
    patterns: Vec<crate::tools::vbml::Pattern>,
    description: String,
}

impl MacroRule {
    pub fn new(pattern: &str) -> Self {
        Self::many(vec![pattern])
    }

    pub fn many(patterns: Vec<&str>) -> Self {
        let compiled: Vec<_> = patterns
            .iter()
            .map(|p| {
                crate::tools::vbml::Pattern::compile(p)
                    .unwrap_or_else(|_| crate::tools::vbml::Pattern::compile("$^").unwrap())
            })
            .collect();
        let description = patterns.join(" | ");
        Self {
            patterns: compiled,
            description,
        }
    }
}

#[async_trait]
impl Rule<Value> for MacroRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let text = match message_text(event) {
            Some(t) => t,
            None => return RuleResult::Fail,
        };
        for pattern in &self.patterns {
            if let Some(captures) = pattern.check(text) {
                let mut ctx = HashMap::new();
                for (k, v) in captures {
                    ctx.insert(k, v);
                }
                return RuleResult::Context(ctx);
            }
        }
        RuleResult::Fail
    }

    fn description(&self) -> String {
        format!("MacroRule({})", self.description)
    }
}

/// Read a payload as JSON from either shape VK uses.
///
/// `message_event` carries `object.payload` as a value; a keyboard button on a
/// message carries `object.message.payload` as a JSON-encoded string.
pub fn extract_payload_value(event: &Value) -> Option<Value> {
    if let Some(p) = event.get("object").and_then(|o| o.get("payload")) {
        if p.is_string() {
            return p.as_str().and_then(|s| serde_json::from_str(s).ok());
        }
        return Some(p.clone());
    }
    extract_message(event)
        .and_then(|m| m.get("payload"))
        .and_then(|p| p.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
}

/// Payload map validation rule with typed validators
pub struct PayloadMapRule {
    validators: Vec<(String, super::payload_validator::PayloadValidator)>,
}

impl PayloadMapRule {
    pub fn new(validators: Vec<(String, super::payload_validator::PayloadValidator)>) -> Self {
        Self { validators }
    }

    pub fn required_keys(keys: Vec<String>) -> Self {
        Self {
            validators: keys
                .into_iter()
                .map(|k| {
                    (
                        k,
                        super::payload_validator::PayloadValidator::func(|_| true),
                    )
                })
                .collect(),
        }
    }

    pub fn from_json(map: serde_json::Map<String, Value>) -> Self {
        Self {
            validators: super::payload_validator::validators_from_json(&map),
        }
    }
}

#[async_trait]
impl Rule<Value> for PayloadMapRule {
    async fn check(&self, event: &Value) -> RuleResult {
        let Some(payload) = extract_payload_value(event) else {
            return RuleResult::Fail;
        };

        if super::payload_validator::match_payload_map(&payload, &self.validators) {
            if let Some(obj) = payload.as_object() {
                RuleResult::Context(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            } else {
                RuleResult::Pass
            }
        } else {
            RuleResult::Fail
        }
    }

    fn description(&self) -> String {
        "PayloadMapRule".to_string()
    }
}
