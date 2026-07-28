use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use vkontakte::dispatch::rules::{Rule, TextRule};
use vkontakte::tools::waiter::{WaiterError, WaiterMachine};

#[tokio::test]
async fn waiter_resolves_on_matching_event() {
    let machine = Arc::new(WaiterMachine::new());
    let peer_id = 100;
    let rules: Vec<Arc<dyn Rule<serde_json::Value>>> =
        vec![Arc::new(TextRule::new("yes", false))];

    let m = machine.clone();
    let handle = tokio::spawn(async move {
        m.wait("message", peer_id, rules, Some(Duration::from_secs(2)))
            .await
    });

    tokio::time::sleep(Duration::from_millis(20)).await;

    let event = json!({
        "type": "message_new",
        "object": { "message": { "peer_id": peer_id, "text": "yes" } }
    });
    assert!(machine.feed("message", peer_id, &event).await);

    let got = handle.await.unwrap().unwrap();
    assert_eq!(got["type"], "message_new");
}

#[tokio::test]
async fn waiter_times_out() {
    let machine = Arc::new(WaiterMachine::new());
    let rules: Vec<Arc<dyn Rule<serde_json::Value>>> =
        vec![Arc::new(TextRule::new("nope", false))];
    let err = machine
        .wait("message", 1, rules, Some(Duration::from_millis(50)))
        .await
        .unwrap_err();
    assert!(matches!(err, WaiterError::Timeout));
}
