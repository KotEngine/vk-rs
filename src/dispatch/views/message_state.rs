//! Shared message + FSM preparation for views

use std::sync::Arc;

use serde_json::Value;

use crate::api::Api;
use crate::dispatch::dispenser::StateDispenser;
use crate::dispatch::state_context::embed_state_peer;
use crate::exception::VkResult;
use crate::tools::mini_types::MessageMin;

/// Load FSM state, enrich the event for rules, and attach dispenser to the message
pub async fn prepare_message(
    event: &Value,
    api: Arc<Api>,
    state_dispenser: Option<Arc<dyn StateDispenser>>,
) -> VkResult<(MessageMin, Value)> {
    let mut enriched = event.clone();
    let peer_id = event
        .get("object")
        .and_then(|o| o.get("message"))
        .or_else(|| event.get("message"))
        .and_then(|m| m.get("peer_id"))
        .and_then(|p| p.as_i64());

    let mut message = MessageMin::from_raw_event(event, api)?;

    if let (Some(dispenser), Some(peer_id)) = (state_dispenser.as_ref(), peer_id) {
        message = message.with_state_dispenser(dispenser.clone());
        if let Some(peer) = dispenser.get(peer_id).await? {
            message.state_peer = Some(peer.clone());
            embed_state_peer(&mut enriched, &peer);
        }
    }

    Ok((message, enriched))
}
