//! VK mention parsing and building

use regex::Regex;
use serde_json::Value;

/// Parsed mention from message text
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mention {
    User(i64),
    Group(i64),
    Link(String),
}

/// Extract all mentions from VK message text
pub fn extract_mentions(text: &str) -> Vec<Mention> {
    let mut out = Vec::new();
    if let Ok(re) = Regex::new(r"\[(id|club|public)(\d+)\|[^\]]+\]") {
        for cap in re.captures_iter(text) {
            let kind = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let id: i64 = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            match kind {
                "id" => out.push(Mention::User(id)),
                "club" | "public" => out.push(Mention::Group(id)),
                _ => {}
            }
        }
    }
    if let Ok(re_link) = Regex::new(r"@([a-zA-Z0-9_]+)") {
        for cap in re_link.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                out.push(Mention::Link(m.as_str().to_string()));
            }
        }
    }
    out
}

/// Build user mention tag
pub fn mention_user(user_id: i64, display: &str) -> String {
    format!("[id{user_id}|{display}]")
}

/// Build community mention tag
pub fn mention_group(group_id: i64, display: &str) -> String {
    format!("[club{group_id}|{display}]")
}

/// Strip mention tags from text, leaving display names
pub fn strip_mentions(text: &str) -> String {
    let mut result = text.to_string();
    if let Ok(re) = Regex::new(r"\[(?:id|club|public)\d+\|([^\]]+)\]") {
        result = re.replace_all(&result, "$1").to_string();
    }
    if let Ok(re_at) = Regex::new(r"@\w+") {
        result = re_at.replace_all(&result, "").to_string();
    }
    result.trim().to_string()
}

/// Whether text contains a mention of given user id
pub fn mentions_user(text: &str, user_id: i64) -> bool {
    extract_mentions(text)
        .iter()
        .any(|m| matches!(m, Mention::User(id) if *id == user_id))
}

/// Serialize mentions as JSON for handler context
pub fn mentions_as_json(text: &str) -> Value {
    let items: Vec<Value> = extract_mentions(text)
        .into_iter()
        .map(|m| match m {
            Mention::User(id) => serde_json::json!({ "type": "user", "id": id }),
            Mention::Group(id) => serde_json::json!({ "type": "group", "id": id }),
            Mention::Link(name) => serde_json::json!({ "type": "link", "name": name }),
        })
        .collect();
    Value::Array(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_mention() {
        let m = extract_mentions("hi [id1|Bot]");
        assert_eq!(m, vec![Mention::User(1)]);
    }

    #[test]
    fn strip_keeps_name() {
        assert_eq!(strip_mentions("hey [id1|Bot]"), "hey Bot");
    }
}
