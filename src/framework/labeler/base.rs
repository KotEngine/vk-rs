//! Base labeler — custom rule shortcuts

use std::collections::HashMap;

use serde_json::Value;

use crate::dispatch::rules::{
    AttachmentTypeRule, ChatActionRule, CommandRule, FromPeerRule, FromUserRule,
    FuncRule, LevenshteinRule, MacroRule, MentionRule, MessageLengthRule, PayloadContainsRule,
    PayloadMapRule, PayloadRule, PeerRule, RegexRule, ReplyMessageRule, Rule, StateGroupRule,
    StateRule, StickerRule, TextRule, VBMLRule,
};
use crate::dispatch::RuleResult;

fn box_rule<R: Rule<Value> + 'static>(rule: R) -> Box<dyn Rule<Value>> {
    Box::new(rule)
}

/// Build a rule from shortcut name and JSON value
pub fn custom_rule(name: &str, value: &Value) -> Option<Box<dyn Rule<Value>>> {
    match name {
        "from_chat" => Some(box_rule(PeerRule::new(true))),
        "mention" => value.as_bool().map(|m| box_rule(MentionRule::new(m))),
        "command" => {
            let cmd = value.as_str()?.to_string();
            Some(box_rule(CommandRule::new(cmd, vec!["/", "!"], None)))
        }
        "from_user" => Some(box_rule(FromUserRule::new())),
        "peer_ids" => {
            let ids: Vec<i64> = value
                .as_array()?
                .iter()
                .filter_map(|v| v.as_i64())
                .collect();
            Some(box_rule(FromPeerRule::new(ids)))
        }
        "sticker" => {
            if let Some(id) = value.as_i64() {
                Some(box_rule(StickerRule::new(Some(vec![id]))))
            } else {
                let ids: Vec<i64> = value
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_i64())
                    .collect();
                Some(box_rule(StickerRule::new(Some(ids))))
            }
        }
        "attachment" => value
            .as_str()
            .map(|t| box_rule(AttachmentTypeRule::new(t))),
        "levenshtein" | "lev" => {
            let text = value.get("text")?.as_str()?.to_string();
            let max_distance = value.get("max_distance")?.as_u64()? as usize;
            Some(box_rule(LevenshteinRule::new(text, max_distance)))
        }
        "length" => {
            let min = value
                .get("min")
                .or(Some(value))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            Some(box_rule(MessageLengthRule::new(min)))
        }
        "action" => value
            .as_str()
            .map(|a| box_rule(ChatActionRule::new(Some(a.to_string())))),
        "payload" => value.as_str().map(|p| box_rule(PayloadRule::new(p))),
        "payload_contains" => {
            if let (Some(key), Some(val)) = (value.get("key"), value.get("value")) {
                Some(box_rule(PayloadContainsRule::new(
                    key.as_str()?.to_string(),
                    val.clone(),
                )))
            } else {
                value.as_str().map(|p| {
                    box_rule(PayloadContainsRule::new(
                        "text",
                        Value::String(p.to_string()),
                    ))
                })
            }
        }
        "payload_map" => {
            let map = value.as_object()?.clone();
            Some(box_rule(PayloadMapRule::from_json(map)))
        }
        "macro" => {
            if let Some(s) = value.as_str() {
                Some(box_rule(MacroRule::new(s)))
            } else {
                let patterns: Vec<&str> = value
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect();
                Some(box_rule(MacroRule::many(patterns)))
            }
        }
        "vbml" => value.as_str().map(|p| box_rule(VBMLRule::new(p))),
        "regexp" | "regex" => value.as_str().map(|p| box_rule(RegexRule::new(p))),
        "reply_message" => Some(box_rule(ReplyMessageRule::new())),
        "state" => value.as_str().map(|s| box_rule(StateRule::new(s))),
        "state_group" => {
            let states: Vec<String> = value
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            Some(box_rule(StateGroupRule::new(states)))
        }
        "text" => {
            let text = value.as_str()?.to_string();
            let ignore_case = value
                .get("ignore_case")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(box_rule(TextRule::new(text, ignore_case)))
        }
        "func" => None,
        _ => None,
    }
}

pub fn rules_from_shortcuts(shortcuts: HashMap<String, Value>) -> Vec<Box<dyn Rule<Value>>> {
    let mut rules = Vec::new();
    for (name, value) in shortcuts {
        if let Some(rule) = custom_rule(&name, &value) {
            rules.push(rule);
        }
    }
    rules
}

pub struct LabelerEntry {
    pub rules: Vec<Box<dyn Rule<Value>>>,
}

impl LabelerEntry {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn rule(mut self, rule: Box<dyn Rule<Value>>) -> Self {
        self.rules.push(rule);
        self
    }
}

impl Default for LabelerEntry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn bool_func_rule(expected: bool) -> Box<dyn Rule<Value>> {
    box_rule(FuncRule::new(move |event: &Value| {
        let ok = event.get("type").and_then(|t| t.as_str()) == Some("message_new");
        if ok == expected {
            RuleResult::Pass
        } else {
            RuleResult::Fail
        }
    }))
}
