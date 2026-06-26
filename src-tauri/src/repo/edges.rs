//! Edge (typed reasoning relation) persistence.

use serde::Deserialize;

use crate::db::{new_id, now, Db};
use crate::domain::{Edge, EdgeType, Strength};
use crate::error::{AppError, AppResult};
use crate::repo::events;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEdge {
    pub map_id: String,
    pub from_node: String,
    pub to_node: String,
    #[serde(default = "default_type")]
    pub edge_type: EdgeType,
    #[serde(default)]
    pub strength: Option<Strength>,
}

fn default_type() -> EdgeType {
    EdgeType::Support
}

const COLS: &str =
    "id, map_id, from_node, to_node, type, strength, created_at, updated_at, deleted_at";

pub async fn get(db: &Db, id: &str) -> AppResult<Edge> {
    sqlx::query_as::<_, Edge>(&format!("SELECT {COLS} FROM edges WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("edge {id}")))
}

async fn get_tx(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: &str) -> AppResult<Edge> {
    sqlx::query_as::<_, Edge>(&format!("SELECT {COLS} FROM edges WHERE id = ?"))
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(AppError::from)
}

pub async fn create(db: &Db, input: NewEdge) -> AppResult<Edge> {
    if input.from_node == input.to_node {
        return Err(AppError::Invalid("an edge cannot connect a node to itself".into()));
    }
    let id = new_id();
    let ts = now();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO edges (id, map_id, from_node, to_node, type, strength, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.map_id)
    .bind(&input.from_node)
    .bind(&input.to_node)
    .bind(input.edge_type)
    .bind(input.strength)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;
    let edge = get_tx(&mut tx, &id).await?;
    events::append_tx(
        &mut tx,
        &input.map_id,
        "edge.create",
        &serde_json::json!({ "after": edge }),
    )
    .await?;
    tx.commit().await?;
    Ok(edge)
}

pub async fn set_type(db: &Db, id: &str, edge_type: EdgeType) -> AppResult<Edge> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE edges SET type = ?, updated_at = ? WHERE id = ?")
        .bind(edge_type)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "edge.set_type",
        &serde_json::json!({ "id": id, "before": before.edge_type, "after": edge_type }),
    )
    .await?;
    tx.commit().await?;
    get(db, id).await
}

pub async fn set_strength(db: &Db, id: &str, strength: Option<Strength>) -> AppResult<Edge> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE edges SET strength = ?, updated_at = ? WHERE id = ?")
        .bind(strength)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "edge.set_strength",
        &serde_json::json!({ "id": id, "before": before.strength, "after": strength }),
    )
    .await?;
    tx.commit().await?;
    get(db, id).await
}

pub async fn soft_delete(db: &Db, id: &str) -> AppResult<()> {
    let before = get(db, id).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE edges SET deleted_at = ? WHERE id = ?")
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    events::append_tx(
        &mut tx,
        &before.map_id,
        "edge.delete",
        &serde_json::json!({ "before": before }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
