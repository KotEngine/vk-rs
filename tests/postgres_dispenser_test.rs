//! Integration tests for the Postgres-backed FSM dispenser.
//!
//! These require a live PostgreSQL instance. Run with:
//!
//! ```sh
//! cargo test --features postgres --test postgres_dispenser_test -- --ignored
//! ```

#![cfg(feature = "postgres")]

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use vkontakte::dispatch::dispenser::PostgresStateDispenser;
use vkontakte::dispatch::StateDispenser;
use vkontakte::tools::fsm::StatePeer;

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_peer_id() -> i64 {
    // Deterministic, conflict-free peer ids within a test run.
    -(9_800_000_000 + SEQ.fetch_add(1, Ordering::Relaxed) as i64)
}

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/vkontakte_test".to_string())
}

async fn dispenser() -> PostgresStateDispenser {
    let d = PostgresStateDispenser::connect(&database_url())
        .await
        .expect("connect to postgres");
    d.migrate().await.expect("migrate");
    d
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL (set DATABASE_URL)"]
async fn postgres_set_get_delete_roundtrip() {
    let dispenser = dispenser().await;
    let peer_id = unique_peer_id();

    let _ = dispenser.delete(peer_id).await;
    assert!(dispenser.get(peer_id).await.unwrap().is_none());

    let mut peer = StatePeer::new(peer_id, "Menu:Main");
    peer.set_payload("step", json!(1));
    dispenser.set(peer).await.unwrap();

    let got = dispenser.get(peer_id).await.unwrap().expect("peer stored");
    assert_eq!(got.peer_id, peer_id);
    assert_eq!(got.state, "Menu:Main");
    assert_eq!(got.get_payload("step"), Some(&json!(1)));

    assert!(dispenser.delete(peer_id).await.unwrap());
    assert!(dispenser.get(peer_id).await.unwrap().is_none());
    // Deleting a missing row is not an error, just `false`.
    assert!(!dispenser.delete(peer_id).await.unwrap());
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL (set DATABASE_URL)"]
async fn postgres_upsert_replaces_state() {
    let dispenser = dispenser().await;
    let peer_id = unique_peer_id();
    let _ = dispenser.delete(peer_id).await;

    dispenser
        .set(StatePeer::new(peer_id, "Menu:Main"))
        .await
        .unwrap();
    dispenser
        .set(StatePeer::new(peer_id, "Menu:Info"))
        .await
        .unwrap();

    let got = dispenser.get(peer_id).await.unwrap().unwrap();
    assert_eq!(got.state, "Menu:Info");
    assert!(got.payload.is_empty());

    let _ = dispenser.delete(peer_id).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL (set DATABASE_URL)"]
async fn postgres_migrate_is_idempotent() {
    let dispenser = dispenser().await;
    // Running it twice must not error.
    dispenser.migrate().await.unwrap();
    dispenser.migrate().await.unwrap();
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL (set DATABASE_URL)"]
async fn postgres_custom_table_is_isolated() {
    let alt = PostgresStateDispenser::connect(&database_url())
        .await
        .unwrap()
        .with_table("vkontakte_states_alt")
        .unwrap();
    alt.migrate().await.unwrap();

    let peer_id = unique_peer_id();
    let _ = alt.delete(peer_id).await;
    alt.set(StatePeer::new(peer_id, "Alt:State")).await.unwrap();

    assert_eq!(alt.get(peer_id).await.unwrap().unwrap().state, "Alt:State");
    // The default table must not see it.
    let default = dispenser().await;
    assert!(default.get(peer_id).await.unwrap().is_none());

    let _ = alt.delete(peer_id).await;
}
