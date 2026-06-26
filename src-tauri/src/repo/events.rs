//! Append-only change log (history + undo). State tables stay the source of truth;
//! this records enough (before/after JSON) to invert each mutation.

use serde::Serialize;
use sqlx::{Sqlite, Transaction};

use crate::db::{new_id, now, Db};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub map_id: String,
    pub ts: String,
    pub op: String,
    pub payload: String,
}

/// Append an event inside an existing transaction (preferred — keeps the mutation and its
/// history entry atomic).
pub async fn append_tx(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
    op: &str,
    payload: &serde_json::Value,
) -> AppResult<()> {
    sqlx::query("INSERT INTO events (id, map_id, ts, op, payload) VALUES (?, ?, ?, ?, ?)")
        .bind(new_id())
        .bind(map_id)
        .bind(now())
        .bind(op)
        .bind(payload.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Append an event on the pool directly (for ops not already in a transaction).
pub async fn append(db: &Db, map_id: &str, op: &str, payload: &serde_json::Value) -> AppResult<()> {
    sqlx::query("INSERT INTO events (id, map_id, ts, op, payload) VALUES (?, ?, ?, ?, ?)")
        .bind(new_id())
        .bind(map_id)
        .bind(now())
        .bind(op)
        .bind(payload.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// Most recent events for a map (newest first).
pub async fn recent(db: &Db, map_id: &str, limit: i64) -> AppResult<Vec<Event>> {
    let rows = sqlx::query_as::<_, Event>(
        "SELECT id, map_id, ts, op, payload FROM events WHERE map_id = ? ORDER BY ts DESC, id DESC LIMIT ?",
    )
    .bind(map_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows)
}
