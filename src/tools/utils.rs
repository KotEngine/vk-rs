//! VK utility helpers

use rand::Rng;
use serde_json::Value;

/// Offset added to chat peer ids in VK
pub const PEER_ID_OFFSET: i64 = 2_000_000_000;

/// Convert chat id to peer id
pub fn chat_peer_id(chat_id: i64) -> i64 {
    PEER_ID_OFFSET + chat_id
}

/// Extract chat id from peer id (returns None for user/dialog peers)
pub fn peer_to_chat_id(peer_id: i64) -> Option<i64> {
    if peer_id > PEER_ID_OFFSET {
        Some(peer_id - PEER_ID_OFFSET)
    } else {
        None
    }
}

/// Whether peer is a chat (group chat / community chat)
pub fn is_chat_peer(peer_id: i64) -> bool {
    peer_id > PEER_ID_OFFSET
}

/// Whether peer is a user dialog
pub fn is_user_peer(peer_id: i64) -> bool {
    peer_id > 0 && peer_id < PEER_ID_OFFSET
}

/// Random id for `messages.send`
pub fn random_id() -> i64 {
    rand::thread_rng().gen()
}

/// Parse VK user mention `[id123|Name]` from text
pub fn parse_mentions(text: &str) -> Vec<i64> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[id") {
        rest = &rest[start + 3..];
        if let Some(pipe) = rest.find('|') {
            if let Ok(id) = rest[..pipe].parse::<i64>() {
                ids.push(id);
            }
        } else if let Some(end) = rest.find(']') {
            if let Ok(id) = rest[..end].parse::<i64>() {
                ids.push(id);
            }
        }
        rest = rest.split_once(']').map(|(_, r)| r).unwrap_or("");
    }
    ids
}

/// Build mention string for VK messages
pub fn mention_user(user_id: i64, display_name: &str) -> String {
    format!("[id{user_id}|{display_name}]")
}

/// Extract `group_id` from negative owner id
pub fn owner_to_group_id(owner_id: i64) -> Option<i64> {
    if owner_id < 0 {
        Some(-owner_id)
    } else {
        None
    }
}

/// Split long message into VK-safe chunks (4096 chars)
pub fn chunk_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + max_len).min(text.len());
        chunks.push(text[start..end].to_string());
        start = end;
    }
    chunks
}

/// Read string field from VK event JSON (`object.message` or top-level)
pub fn event_message_field<'a>(event: &'a Value, field: &str) -> Option<&'a str> {
    event
        .get("object")
        .and_then(|o| o.get("message"))
        .or_else(|| event.get("message"))
        .and_then(|m| m.get(field))
        .and_then(|v| v.as_str())
}

/// Read i64 field from message object inside event
pub fn event_message_i64(event: &Value, field: &str) -> Option<i64> {
    event
        .get("object")
        .and_then(|o| o.get("message"))
        .or_else(|| event.get("message"))
        .and_then(|m| m.get(field))
        .and_then(|v| v.as_i64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_peer_roundtrip() {
        assert_eq!(peer_to_chat_id(chat_peer_id(42)), Some(42));
    }

    #[test]
    fn chunk_long_text() {
        let text = "a".repeat(5000);
        let chunks = chunk_message(&text, 4096);
        assert_eq!(chunks.len(), 2);
    }
}
