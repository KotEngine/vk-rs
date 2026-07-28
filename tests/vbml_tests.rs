use serde_json::json;
use vkontakte::dispatch::rules::{MacroRule, Rule, VBMLRule};
use vkontakte::dispatch::RuleResult;

fn message_event(text: &str) -> serde_json::Value {
    json!({
        "type": "message_new",
        "object": {
            "message": {
                "peer_id": 1,
                "from_id": 2,
                "text": text
            }
        }
    })
}

#[tokio::test]
async fn vbml_rule_captures_placeholder() {
    let rule = VBMLRule::new("hello {name}");
    let event = message_event("hello world");
    match rule.check(&event).await {
        RuleResult::Context(ctx) => {
            assert_eq!(ctx.get("name").and_then(|v| v.as_str()), Some("world"));
        }
        _ => panic!("expected context"),
    }
}

#[tokio::test]
async fn macro_rule_tries_patterns_in_order() {
    let rule = MacroRule::many(vec!["ping", "hello {name}"]);
    assert!(matches!(
        rule.check(&message_event("ping")).await,
        RuleResult::Context(_)
    ));
    match rule.check(&message_event("hello bob")).await {
        RuleResult::Context(ctx) => {
            assert_eq!(ctx.get("name").and_then(|v| v.as_str()), Some("bob"));
        }
        _ => panic!("expected context"),
    }
}
