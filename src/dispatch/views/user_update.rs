//! User long poll update normalization

use serde_json::{json, Value};

use crate::constants::{message_flags, user_lp_events, CHAT_PEER_ID_OFFSET};

/// VK user long poll code for a new message.
///
/// Kept as an alias of [`user_lp_events::NEW_MESSAGE`] for backwards compatibility.
pub const USER_LP_NEW_MESSAGE: i64 = user_lp_events::NEW_MESSAGE;

/// Convert a user long poll update array into a bot-style `message_new` event.
pub fn normalize_user_update(update: &Value) -> Option<Value> {
    let arr = update.as_array()?;
    if arr.len() < 6 {
        return None;
    }

    let code = arr[0].as_i64()?;
    if code != USER_LP_NEW_MESSAGE {
        return None;
    }

    let message_id = arr.get(1).and_then(|v| v.as_i64())?;
    let flags = arr.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
    let peer_id = arr.get(3).and_then(|v| v.as_i64())?;
    let date = arr.get(4).and_then(|v| v.as_i64()).unwrap_or(0);
    let text = arr
        .get(5)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let attachments = parse_attachments(arr.get(6));
    let payload = arr
        .get(11)
        .and_then(|v| v.as_str())
        .map(String::from);
    let from_id = extract_from_id(peer_id, flags, arr);
    let out = flags & message_flags::OUTBOX != 0;

    let mut message = json!({
        "id": message_id,
        "peer_id": peer_id,
        "from_id": from_id,
        "date": date,
        "text": text,
        "out": out,
        "important": flags & message_flags::IMPORTANT != 0,
    });

    if !attachments.is_empty() {
        message["attachments"] = Value::Array(attachments);
    }
    if let Some(p) = payload {
        message["payload"] = Value::String(p);
    }

    Some(json!({
        "type": "message_new",
        "object": {
            "message": message
        }
    }))
}

fn extract_from_id(peer_id: i64, flags: i64, arr: &[Value]) -> i64 {
    if let Some(extra) = arr.get(10).and_then(|v| v.as_str()) {
        if let Ok(extra_json) = serde_json::from_str::<Value>(extra) {
            if let Some(from) = extra_json.get("from").and_then(|f| f.as_i64()) {
                return from;
            }
        }
    }

    if peer_id > CHAT_PEER_ID_OFFSET {
        return 0;
    }

    if flags & message_flags::OUTBOX != 0 {
        0
    } else {
        peer_id
    }
}

fn parse_attachments(raw: Option<&Value>) -> Vec<Value> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let attach_str = match raw {
        Value::String(s) if !s.is_empty() => s.as_str(),
        _ => return Vec::new(),
    };

    attach_str
        .split(',')
        .filter_map(parse_attachment_token)
        .collect()
}

fn parse_attachment_token(token: &str) -> Option<Value> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let (kind, rest) = token.split_once('_')?;
    let mut parts = rest.split('_');
    let owner_id: i64 = parts.next()?.parse().ok()?;
    let id: i64 = parts.next()?.parse().ok()?;
    let access_key = parts.next().map(String::from);

    let mut attachment = json!({
        "type": kind,
        kind: {
            "id": id,
            "owner_id": owner_id,
        }
    });

    if let Some(key) = access_key {
        attachment[kind]["access_key"] = Value::String(key);
    }

    Some(attachment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_private_message() {
        let update = json!([4, 100, 0, 12345, 1700000000, "hello", ""]);
        let event = normalize_user_update(&update).expect("normalized");
        assert_eq!(event["type"], "message_new");
        assert_eq!(event["object"]["message"]["text"], "hello");
        assert_eq!(event["object"]["message"]["from_id"], 12345);
    }

    #[test]
    fn ignores_non_message_updates() {
        assert!(normalize_user_update(&json!([5, 1, 2])).is_none());
    }
}
