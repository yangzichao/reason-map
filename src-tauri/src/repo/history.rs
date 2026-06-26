//! Undo built on the event log. Reads the most recent event, applies its inverse (the
//! events now carry full before/after snapshots), then consumes that event so repeated
//! calls walk back through history. Multi-level undo; redo is not yet implemented.

use serde_json::Value;

use crate::db::{now, Db};
use crate::error::AppResult;
use crate::repo::events::Event;

/// Undo the most recent change on a map. Returns false if there is nothing to undo.
pub async fn undo_last(db: &Db, map_id: &str) -> AppResult<bool> {
    let ev = sqlx::query_as::<_, Event>(
        "SELECT id, map_id, ts, op, payload FROM events \
         WHERE map_id = ? ORDER BY ts DESC, id DESC LIMIT 1",
    )
    .bind(map_id)
    .fetch_optional(db)
    .await?;
    let Some(ev) = ev else {
        return Ok(false);
    };
    let p: Value = serde_json::from_str(&ev.payload).unwrap_or(Value::Null);

    let mut tx = db.begin().await?;
    match ev.op.as_str() {
        "node.create" => {
            if let Some(id) = p["after"]["id"].as_str() {
                sqlx::query("UPDATE nodes SET deleted_at = ? WHERE id = ?")
                    .bind(now())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        "node.delete" => {
            if let Some(id) = p["before"]["id"].as_str() {
                sqlx::query("UPDATE nodes SET deleted_at = NULL WHERE id = ?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
            if let Some(edges) = p["detachedEdges"].as_array() {
                for e in edges {
                    if let Some(eid) = e["id"].as_str() {
                        sqlx::query("UPDATE edges SET deleted_at = NULL WHERE id = ?")
                            .bind(eid)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
            }
        }
        "node.update_text" => {
            revert_text(&mut tx, "nodes", "text", &p).await?;
        }
        "node.set_status" => {
            revert_text(&mut tx, "nodes", "status", &p).await?;
        }
        "node.set_origin" => {
            revert_text(&mut tx, "nodes", "origin", &p).await?;
        }
        "edge.create" => {
            if let Some(id) = p["after"]["id"].as_str() {
                sqlx::query("UPDATE edges SET deleted_at = ? WHERE id = ?")
                    .bind(now())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        "edge.delete" => {
            if let Some(id) = p["before"]["id"].as_str() {
                sqlx::query("UPDATE edges SET deleted_at = NULL WHERE id = ?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        "edge.set_type" => {
            revert_text(&mut tx, "edges", "type", &p).await?;
        }
        "edge.set_strength" => {
            // strength is nullable.
            if let Some(id) = p["id"].as_str() {
                let before = p["before"].as_str();
                sqlx::query("UPDATE edges SET strength = ? WHERE id = ?")
                    .bind(before)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        _ => {}
    }

    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(&ev.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

/// Revert a single TEXT column to the `before` value carried in the event payload.
async fn revert_text(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    table: &str,
    column: &str,
    p: &Value,
) -> AppResult<()> {
    if let (Some(id), Some(before)) = (p["id"].as_str(), p["before"].as_str()) {
        let sql = format!("UPDATE {table} SET {column} = ? WHERE id = ?");
        sqlx::query(&sql)
            .bind(before)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
