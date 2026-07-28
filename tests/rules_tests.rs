use serde_json::json;
use vkontakte::dispatch::rules::{CommandRule, FromUserRule, Rule, TextRule};
use vkontakte::dispatch::RuleResult;

#[tokio::test]
async fn text_rule_matches_exact() {
    let rule = TextRule::new("hello", false);
    let event = json!({
        "type": "message_new",
        "object": { "message": { "text": "hello", "peer_id": 1, "from_id": 1 } }
    });
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}

#[tokio::test]
async fn text_rule_ignore_case() {
    let rule = TextRule::new("Hello", true);
    let event = json!({
        "type": "message_new",
        "object": { "message": { "text": "hello", "peer_id": 1, "from_id": 1 } }
    });
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}

#[tokio::test]
async fn command_rule_parses_args() {
    let rule = CommandRule::new("ban", vec!["!"], Some(1));
    let event = json!({
        "type": "message_new",
        "object": { "message": { "text": "!ban 123", "peer_id": 1, "from_id": 1 } }
    });
    match rule.check(&event).await {
        RuleResult::Context(ctx) => {
            assert_eq!(ctx.get("args").and_then(|v| v.as_array()).map(|a| a.len()), Some(1));
        }
        _ => panic!("expected context"),
    }
}

#[tokio::test]
async fn is_admin_rule_rejects_private_chat() {
    use std::sync::Arc;
    use vkontakte::api::api;
    use vkontakte::dispatch::rules::IsAdminRule;

    let rule = IsAdminRule::new(Arc::new(api("dummy").unwrap()));
    let event = json!({
        "type": "message_new",
        "object": { "message": { "text": "ban", "peer_id": 100, "from_id": 100 } }
    });
    assert!(matches!(rule.check(&event).await, RuleResult::Fail));
}

#[tokio::test]
async fn and_rule_combinator() {
    fn box_rule(r: impl Rule<serde_json::Value> + 'static) -> Box<dyn Rule<serde_json::Value>> {
        Box::new(r)
    }

    let rule = box_rule(TextRule::new("hi", false)) & box_rule(FromUserRule::new());
    let event = json!({
        "type": "message_new",
        "object": { "message": { "text": "hi", "peer_id": 100, "from_id": 100 } }
    });
    assert!(matches!(rule.check(&event).await, RuleResult::Pass));
}
