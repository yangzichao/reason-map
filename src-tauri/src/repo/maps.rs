//! Map (document) persistence.

use crate::db::{new_id, now, Db};
use crate::domain::{Challenge, Edge, Map, MapGraph, Node};
use crate::error::{AppError, AppResult};

pub async fn create(db: &Db, title: &str) -> AppResult<Map> {
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO maps (id, title, meta, created_at, updated_at) VALUES (?, ?, '{}', ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(&ts)
    .bind(&ts)
    .execute(db)
    .await?;
    get(db, &id).await
}

pub async fn get(db: &Db, id: &str) -> AppResult<Map> {
    sqlx::query_as::<_, Map>(
        "SELECT id, title, meta, created_at, updated_at, deleted_at FROM maps WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("map {id}")))
}

pub async fn list(db: &Db) -> AppResult<Vec<Map>> {
    let rows = sqlx::query_as::<_, Map>(
        "SELECT id, title, meta, created_at, updated_at, deleted_at FROM maps \
         WHERE deleted_at IS NULL ORDER BY updated_at DESC",
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn rename(db: &Db, id: &str, title: &str) -> AppResult<Map> {
    sqlx::query("UPDATE maps SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(now())
        .bind(id)
        .execute(db)
        .await?;
    get(db, id).await
}

pub async fn soft_delete(db: &Db, id: &str) -> AppResult<()> {
    sqlx::query("UPDATE maps SET deleted_at = ? WHERE id = ?")
        .bind(now())
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn touch(db: &Db, id: &str) -> AppResult<()> {
    sqlx::query("UPDATE maps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Load the full graph for a map in one round trip.
pub async fn graph(db: &Db, map_id: &str) -> AppResult<MapGraph> {
    let map = get(db, map_id).await?;
    let nodes = sqlx::query_as::<_, Node>(
        "SELECT id, map_id, text, status, origin, x, y, created_at, updated_at, deleted_at \
         FROM nodes WHERE map_id = ? AND deleted_at IS NULL ORDER BY created_at",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;
    let edges = sqlx::query_as::<_, Edge>(
        "SELECT id, map_id, from_node, to_node, type, strength, created_at, updated_at, deleted_at \
         FROM edges WHERE map_id = ? AND deleted_at IS NULL ORDER BY created_at",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;
    let challenges = sqlx::query_as::<_, Challenge>(
        "SELECT id, map_id, target_kind, target_id, kind, content, status, verdict, user_note, \
         created_at, resolved_at FROM challenges WHERE map_id = ? ORDER BY created_at",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;
    Ok(MapGraph {
        map,
        nodes,
        edges,
        challenges,
    })
}
