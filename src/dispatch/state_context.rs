//! FSM state embedding in dispatch events

use serde_json::{json, Value};

use crate::tools::fsm::StatePeer;

/// Build canonical state string: `GroupName:value`
pub fn make_state_repr(group: &str, value: &str) -> String {
    format!("{group}:{value}")
}

/// Group prefix from a state repr (`MenuState:start` → `MenuState`)
pub fn state_group_name(state: &str) -> &str {
    state.split(':').next().unwrap_or(state)
}

/// Attach peer state to a raw event so rules can read it
pub fn embed_state_peer(event: &mut Value, peer: &StatePeer) {
    event["state_peer"] = json!({
        "peer_id": peer.peer_id,
        "state": peer.state,
        "payload": peer.payload,
    });
}

/// Read peer state previously embedded into an event
pub fn extract_state_peer(event: &Value) -> Option<StatePeer> {
    let obj = event.get("state_peer")?;
    let peer_id = obj.get("peer_id")?.as_i64()?;
    let state = obj.get("state")?.as_str()?.to_string();
    let payload = obj
        .get("payload")
        .and_then(|p| p.as_object())
        .map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();

    Some(StatePeer {
        peer_id,
        state,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_and_extract_roundtrip() {
        let peer = StatePeer::new(100, make_state_repr("MenuState", "start"));
        let mut event = json!({"type": "message_new"});
        embed_state_peer(&mut event, &peer);
        let parsed = extract_state_peer(&event).expect("state");
        assert_eq!(parsed.state, "MenuState:start");
        assert_eq!(parsed.peer_id, 100);
    }
}
