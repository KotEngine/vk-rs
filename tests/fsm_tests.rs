use serde_json::json;
use vkontakte::dispatch::rules::{Rule, StateGroupRule, StateRule, TextRule};
use vkontakte::dispatch::state_context::{embed_state_peer, extract_state_peer};
use vkontakte::dispatch::RuleResult;
use vkontakte::state_group;
use vkontakte::tools::fsm::StatePeer;

state_group! {
    enum TestState {
        Alpha = "alpha",
        Beta = "beta",
    }
}

#[test]
fn state_group_macro_repr() {
    assert_eq!(TestState::Alpha.as_str(), "TestState:alpha");
    assert_eq!(String::from(TestState::Beta), "TestState:beta");
}

#[tokio::test]
async fn state_rule_none_matches_absent_state() {
    let rule = StateRule::none();
    let event = json!({"type": "message_new"});
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}

#[tokio::test]
async fn state_rule_matches_embedded_state() {
    let rule = StateRule::new(TestState::Alpha);
    let mut event = json!({"type": "message_new"});
    embed_state_peer(
        &mut event,
        &StatePeer::new(1, TestState::Alpha.as_str()),
    );
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}

#[tokio::test]
async fn state_rule_rejects_wrong_state() {
    let rule = StateRule::new(TestState::Beta);
    let mut event = json!({"type": "message_new"});
    embed_state_peer(
        &mut event,
        &StatePeer::new(1, TestState::Alpha.as_str()),
    );
    assert!(matches!(rule.check(&event).await, RuleResult::Fail));
}

#[tokio::test]
async fn state_group_rule_matches_group_prefix() {
    let rule = StateGroupRule::group("TestState");
    let mut event = json!({"type": "message_new"});
    embed_state_peer(
        &mut event,
        &StatePeer::new(1, TestState::Beta.as_str()),
    );
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}

#[test]
fn extract_state_peer_roundtrip() {
    let peer = StatePeer::new(42, "MenuState:start");
    let mut event = json!({});
    embed_state_peer(&mut event, &peer);
    let parsed = extract_state_peer(&event).expect("peer");
    assert_eq!(parsed.peer_id, 42);
    assert_eq!(parsed.state, "MenuState:start");
}

#[tokio::test]
async fn text_rule_still_works_with_state_context() {
    let rule = TextRule::new("ping", false);
    let mut event = json!({
        "type": "message_new",
        "object": { "message": { "text": "ping", "peer_id": 1, "from_id": 1 } }
    });
    embed_state_peer(
        &mut event,
        &StatePeer::new(1, TestState::Alpha.as_str()),
    );
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}
