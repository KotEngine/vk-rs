//! Integration tests for the Redis-backed FSM dispenser.
//!
//! These require a live Redis instance. Run with:
//!
//! ```sh
//! cargo test --features redis --test redis_dispenser_test -- --ignored
//! ```

#![cfg(feature = "redis")]

use std::sync::atomic::{AtomicU64, Ordering};

use vkontakte::dispatch::dispenser::RedisStateDispenser;
use vkontakte::dispatch::StateDispenser;
use vkontakte::tools::fsm::StatePeer;
use serde_json::json;

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_peer_id() -> i64 {
    // Deterministic, conflict-free peer ids within a test run.
    -(9_900_000_000 + SEQ.fetch_add(1, Ordering::Relaxed) as i64)
}

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
}

#[tokio::test]
#[ignore = "requires a running Redis (set REDIS_URL)"]
async fn redis_set_get_delete_roundtrip() {
    let dispenser = RedisStateDispenser::new(&redis_url()).await.unwrap();
    let peer_id = unique_peer_id();

    // Clean slate.
    let _ = dispenser.delete(peer_id).await;

    assert!(dispenser.get(peer_id).await.unwrap().is_none());

    let mut peer = StatePeer::new(peer_id, "Menu:Main");
    peer.set_payload("step", json!(1));
    dispenser.set(peer.clone()).await.unwrap();

    let got = dispenser.get(peer_id).await.unwrap().expect("peer stored");
    assert_eq!(got.peer_id, peer_id);
    assert_eq!(got.state, "Menu:Main");
    assert_eq!(got.get_payload("step"), Some(&json!(1)));

    assert!(dispenser.delete(peer_id).await.unwrap());
    assert!(dispenser.get(peer_id).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires a running Redis (set REDIS_URL)"]
async fn redis_overwrite_replaces_state() {
    let dispenser = RedisStateDispenser::new(&redis_url()).await.unwrap();
    let peer_id = unique_peer_id();
    let _ = dispenser.delete(peer_id).await;

    dispenser.set(StatePeer::new(peer_id, "Menu:Main")).await.unwrap();
    dispenser.set(StatePeer::new(peer_id, "Menu:Info")).await.unwrap();

    let got = dispenser.get(peer_id).await.unwrap().unwrap();
    assert_eq!(got.state, "Menu:Info");

    let _ = dispenser.delete(peer_id).await;
}

#[tokio::test]
#[ignore = "requires a running Redis (set REDIS_URL)"]
async fn redis_custom_prefix_isolated() {
    let prefix = "vkrs_tests:state:";
    let dispenser = RedisStateDispenser::with_prefix(prefix, &redis_url())
        .await
        .unwrap();
    let peer_id = unique_peer_id();
    let _ = dispenser.delete(peer_id).await;

    dispenser.set(StatePeer::new(peer_id, "Menu:Main")).await.unwrap();
    assert_eq!(
        dispenser.get(peer_id).await.unwrap().unwrap().state,
        "Menu:Main"
    );

    let _ = dispenser.delete(peer_id).await;
}
