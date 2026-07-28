//! PostgreSQL-backed FSM state dispenser.
//!
//! Same role as the Redis dispenser, for deployments that already run Postgres
//! and want FSM state in the same database as everything else. Enable the
//! `postgres` cargo feature:
//!
//! ```toml
//! vkontakte = { version = "0.1", features = ["postgres"] }
//! ```
//!
//! ```no_run
//! # use vkontakte::dispatch::dispenser::PostgresStateDispenser;
//! # async fn run() -> vkontakte::VkResult<()> {
//! let dispenser = PostgresStateDispenser::connect("postgres://localhost/bot").await?;
//! dispenser.migrate().await?; // creates the table if it is missing
//! // ... pass `Arc::new(dispenser)` to `Bot::with_state_dispenser`.
//! # Ok(()) }
//! ```

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::exception::{VkError, VkResult};
use crate::tools::fsm::StatePeer;

use super::StateDispenser;

/// Table holding one row per peer.
pub const DEFAULT_TABLE: &str = "vkontakte_states";

/// FSM dispenser backed by a Postgres connection pool.
pub struct PostgresStateDispenser {
    pool: PgPool,
    table: String,
}

impl PostgresStateDispenser {
    /// Connect to `postgres://...` using a default-sized pool.
    pub async fn connect(url: &str) -> VkResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(db_err)?;
        Ok(Self::from_pool(pool))
    }

    /// Wrap an existing pool, e.g. one shared with the rest of the application.
    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            pool,
            table: DEFAULT_TABLE.to_string(),
        }
    }

    /// Use a different table name.
    ///
    /// The name is embedded in SQL directly — Postgres does not accept a bind
    /// parameter there — so it is validated as a plain identifier first.
    pub fn with_table(mut self, table: impl Into<String>) -> VkResult<Self> {
        self.table = validate_table_name(table.into())?;
        Ok(self)
    }

    /// Borrow the underlying pool for ad-hoc queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn table(&self) -> &str {
        &self.table
    }

    /// Create the state table if it does not exist yet.
    pub async fn migrate(&self) -> VkResult<()> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                 peer_id BIGINT PRIMARY KEY,
                 state   TEXT   NOT NULL,
                 payload JSONB  NOT NULL DEFAULT '{{}}'::jsonb
             )",
            self.table
        );
        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

#[async_trait]
impl StateDispenser for PostgresStateDispenser {
    async fn get(&self, peer_id: i64) -> VkResult<Option<StatePeer>> {
        let sql = format!(
            "SELECT state, payload FROM {} WHERE peer_id = $1",
            self.table
        );
        let row = sqlx::query(&sql)
            .bind(peer_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let state: String = row.try_get("state").map_err(db_err)?;
        let payload: Value = row.try_get("payload").map_err(db_err)?;
        let payload: HashMap<String, Value> = serde_json::from_value(payload)
            .map_err(|e| VkError::Deserialization(format!("state payload for {peer_id}: {e}")))?;

        Ok(Some(StatePeer {
            peer_id,
            state,
            payload,
        }))
    }

    async fn set(&self, peer: StatePeer) -> VkResult<()> {
        let payload = serde_json::to_value(&peer.payload)
            .map_err(|e| VkError::Serialization(e.to_string()))?;

        let sql = format!(
            "INSERT INTO {} (peer_id, state, payload) VALUES ($1, $2, $3)
             ON CONFLICT (peer_id) DO UPDATE SET state = EXCLUDED.state, payload = EXCLUDED.payload",
            self.table
        );
        sqlx::query(&sql)
            .bind(peer.peer_id)
            .bind(&peer.state)
            .bind(payload)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, peer_id: i64) -> VkResult<bool> {
        let sql = format!("DELETE FROM {} WHERE peer_id = $1", self.table);
        let result = sqlx::query(&sql)
            .bind(peer_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

fn db_err(e: sqlx::Error) -> VkError {
    VkError::Internal(format!("postgres: {e}"))
}

/// Accept only bare identifiers, since the table name is interpolated into SQL.
fn validate_table_name(table: String) -> VkResult<String> {
    let valid = !table.is_empty()
        && !table.starts_with(|c: char| c.is_ascii_digit())
        && table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');

    if valid {
        Ok(table)
    } else {
        Err(VkError::Configuration(format!(
            "invalid table name {table:?}: expected letters, digits and underscores, \
             not starting with a digit"
        )))
    }
}

impl std::fmt::Debug for PostgresStateDispenser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStateDispenser")
            .field("table", &self.table)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_identifiers() {
        for name in ["vkontakte_states", "states2", "My_Table", "_private"] {
            assert_eq!(
                validate_table_name(name.to_string()).unwrap(),
                name,
                "{name} should be accepted"
            );
        }
    }

    /// The table name is interpolated into SQL, so this validation is the only
    /// thing between a caller and an injection.
    #[test]
    fn rejects_anything_that_is_not_an_identifier() {
        for name in [
            "",
            "states; DROP TABLE users",
            "public.states",
            "sta tes",
            "states--",
            "\"states\"",
            "2states",
        ] {
            assert!(
                validate_table_name(name.to_string()).is_err(),
                "{name:?} should be rejected"
            );
        }
    }
}
