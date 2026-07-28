//! Common validators for handlers and payload rules

use serde_json::Value;

/// Validate peer id is a user DM (positive peer, from_id > 0)
pub fn is_private_message_peer(peer_id: i64) -> bool {
    peer_id > 0 && peer_id < 2_000_000_000
}

/// Validate peer id is a chat (2000000000 + chat_id)
pub fn is_chat_peer(peer_id: i64) -> bool {
    peer_id >= 2_000_000_000
}

pub fn chat_id_from_peer(peer_id: i64) -> Option<i64> {
    if is_chat_peer(peer_id) {
        Some(peer_id - 2_000_000_000)
    } else {
        None
    }
}

pub fn peer_id_from_chat(chat_id: i64) -> i64 {
    chat_id + 2_000_000_000
}

/// Check JSON payload has string field equal to expected
pub fn payload_field_eq(payload: &Value, key: &str, expected: &str) -> bool {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s == expected)
        .unwrap_or(false)
}

/// Check JSON payload has numeric field
pub fn payload_field_is_i64(payload: &Value, key: &str) -> bool {
    payload.get(key).and_then(|v| v.as_i64()).is_some()
}

/// Validate VK user id range
pub fn is_valid_user_id(user_id: i64) -> bool {
    user_id > 0
}

/// Validate group id (negative owner id convention)
pub fn is_valid_group_id(group_id: i64) -> bool {
    group_id > 0
}

pub fn owner_id_for_group(group_id: i64) -> i64 {
    -group_id
}

/// Trim and check non-empty command args
pub fn non_empty_args(args: &[String]) -> bool {
    args.iter().any(|a| !a.trim().is_empty())
}

/// Parse command args as integers
pub fn parse_i64_args(args: &[String]) -> Option<Vec<i64>> {
    let parsed: Option<Vec<i64>> = args.iter().map(|a| a.parse().ok()).collect();
    parsed
}

/// Max message length for VK (4096)
pub fn message_length_ok(text: &str, max: usize) -> bool {
    text.chars().count() <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_peer_roundtrip() {
        let chat_id = 42;
        let peer = peer_id_from_chat(chat_id);
        assert_eq!(chat_id_from_peer(peer), Some(chat_id));
    }

    #[test]
    fn payload_field_eq_works() {
        let p = serde_json::json!({"action": "open"});
        assert!(payload_field_eq(&p, "action", "open"));
    }
}
