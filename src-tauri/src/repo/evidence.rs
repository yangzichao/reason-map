//! Evidence / citations attached to a node (SPEC §2). `payload` is a free-form JSON string
//! whose shape depends on `kind` (url/quote/data/file); the app stores `{"value": "..."}`.

use crate::db::{new_id, now, Db};
use crate::domain::{Evidence, EvidenceKind};
use crate::error::{AppError, AppResult};

const COLS: &str = "id, node_id, kind, payload, created_at";

pub async fn get(db: &Db, id: &str) -> AppResult<Evidence> {
    sqlx::query_as::<_, Evidence>(&format!("SELECT {COLS} FROM evidence WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("evidence {id}")))
}

pub async fn for_node(db: &Db, node_id: &str) -> AppResult<Vec<Evidence>> {
    let rows = sqlx::query_as::<_, Evidence>(&format!(
        "SELECT {COLS} FROM evidence WHERE node_id = ? ORDER BY created_at"
    ))
    .bind(node_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn add(db: &Db, node_id: &str, kind: EvidenceKind, payload: &str) -> AppResult<Evidence> {
    let id = new_id();
    sqlx::query("INSERT INTO evidence (id, node_id, kind, payload, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(node_id)
        .bind(kind)
        .bind(payload)
        .bind(now())
        .execute(db)
        .await?;
    get(db, &id).await
}

pub async fn delete(db: &Db, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM evidence WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
