//! Node (claim) persistence. Mutations log an inverse-capable event (full before/after
//! snapshots) in the same transaction so history + undo stay consistent (SPEC §5).

use serde::Deserialize;

use crate::db::{new_id, now, Db};
use crate::domain::{Edge, Node, NodeStatus, Origin};
use crate::error::{AppError, AppResult};
use crate::repo::events;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewNode {
    pub map_id: String,
    pub text: String,
    #[serde(default = "default_status")]
    pub status: NodeStatus,
    #[serde(default = "default_origin")]
    pub origin: Origin,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

fn default_status() -> NodeStatus {
    NodeStatus::Open
}
fn default_origin() -> Origin {
    Origin::User
}

const COLS: &str =
    "id, map_id, text, status, origin, x, y, created_at, updated_at, deleted_at";

pub async fn get(db: &Db, id: &str) -> AppResult<Node> {
    sqlx::query_as::<_, Node>(&format!("SELECT {COLS} FROM nodes WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("node {id}")))
}

pub async fn create(db: &Db, input: NewNode) -> AppResult<Node> {
    let id = new_id();
    let ts = now();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO nodes (id, map_id, text, status, origin, x, y, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.map_id)
    .bind(&input.text)
    .bind(input.status)
    .bind(input.origin)
    .bind(input.x)
    .bind(input.y)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;
    let node = sqlx::query_as::<_, Node>(&format!("SELECT {COLS} FROM nodes WHERE id = ?"))
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &input.map_id,
        "node.create",
        &serde_json::json!({ "after": node }),
    )
    .await?;
    tx.commit().await?;
    Ok(node)
}

pub async fn update_text(db: &Db, id: &str, text: &str) -> AppResult<Node> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE nodes SET text = ?, updated_at = ? WHERE id = ?")
        .bind(text)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // Text changed → its embedding is stale (SPEC §5).
    sqlx::query("UPDATE node_embeddings SET dirty = 1 WHERE node_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "node.update_text",
        &serde_json::json!({ "id": id, "before": before.text, "after": text }),
    )
    .await?;
    tx.commit().await?;
    get(db, id).await
}

pub async fn set_status(db: &Db, id: &str, status: NodeStatus) -> AppResult<Node> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE nodes SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "node.set_status",
        &serde_json::json!({ "id": id, "before": before.status, "after": status }),
    )
    .await?;
    tx.commit().await?;
    get(db, id).await
}

pub async fn set_origin(db: &Db, id: &str, origin: Origin) -> AppResult<Node> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE nodes SET origin = ?, updated_at = ? WHERE id = ?")
        .bind(origin)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "node.set_origin",
        &serde_json::json!({ "id": id, "before": before.origin, "after": origin }),
    )
    .await?;
    tx.commit().await?;
    get(db, id).await
}

/// Position updates are high-frequency (dragging); no event is logged per drag tick.
pub async fn move_to(db: &Db, id: &str, x: f64, y: f64) -> AppResult<()> {
    sqlx::query("UPDATE nodes SET x = ?, y = ?, updated_at = ? WHERE id = ?")
        .bind(x)
        .bind(y)
        .bind(now())
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn soft_delete(db: &Db, id: &str) -> AppResult<()> {
    let before = get(db, id).await?;
    let ts = now();

    // Capture the edges that will be detached so the delete is reversible and auditable.
    let detached = sqlx::query_as::<_, Edge>(&format!(
        "SELECT id, map_id, from_node, to_node, type, strength, created_at, updated_at, deleted_at \
         FROM edges WHERE (from_node = ? OR to_node = ?) AND deleted_at IS NULL"
    ))
    .bind(id)
    .bind(id)
    .fetch_all(db)
    .await?;

    let mut tx = db.begin().await?;
    sqlx::query("UPDATE nodes SET deleted_at = ? WHERE id = ?")
        .bind(&ts)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE edges SET deleted_at = ? WHERE (from_node = ? OR to_node = ?) AND deleted_at IS NULL",
    )
    .bind(&ts)
    .bind(id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    // Resolve pending challenges targeting this node OR its now-detached edges so they
    // don't linger in the inbox as dangling attacks (high.5 / medium.2).
    sqlx::query(
        "UPDATE challenges SET status = 'deferred', verdict = 'deferred', resolved_at = ? \
         WHERE status = 'pending' AND (target_id = ? \
           OR target_id IN (SELECT id FROM edges WHERE from_node = ? OR to_node = ?))",
    )
    .bind(&ts)
    .bind(id)
    .bind(id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "node.delete",
        &serde_json::json!({ "before": before, "detachedEdges": detached }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
