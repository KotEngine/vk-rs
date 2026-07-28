use serde_json::json;
use vkontakte::dispatch::rules::{PayloadMapRule, Rule};
use vkontakte::dispatch::RuleResult;

fn payload_event(payload: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "message_event",
        "object": {
            "user_id": 1,
            "peer_id": 2,
            "event_id": "abc",
            "payload": payload
        }
    })
}

#[tokio::test]
async fn payload_map_rule_validates_keys() {
    let validators = vkontakte::dispatch::rules::payload_validator::validators_from_json(
        &serde_json::Map::from_iter([
            ("action".into(), json!("open")),
            ("id".into(), json!(42)),
        ]),
    );
    let rule = PayloadMapRule::new(validators);

    let ok = payload_event(json!({"action": "open", "id": 42}));
    assert!(matches!(rule.check(&ok).await, RuleResult::Context(_)));

    let bad = payload_event(json!({"action": "close", "id": 42}));
    assert!(matches!(rule.check(&bad).await, RuleResult::Fail));
}
