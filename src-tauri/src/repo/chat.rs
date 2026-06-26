//! Chat history, bound to a map (SPEC §7.9).

use crate::db::{new_id, now, Db};
use crate::domain::{ChatMessage, ChatRole};
use crate::error::AppResult;

pub async fn append(
    db: &Db,
    map_id: &str,
    role: ChatRole,
    content: &str,
    context_node_ids: &[String],
) -> AppResult<ChatMessage> {
    let id = new_id();
    let ctx = serde_json::to_string(context_node_ids)?;
    sqlx::query(
        "INSERT INTO chat_messages (id, map_id, role, content, context_node_ids, created_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(map_id)
    .bind(role)
    .bind(content)
    .bind(ctx)
    .bind(now())
    .execute(db)
    .await?;
    get(db, &id).await
}

pub async fn get(db: &Db, id: &str) -> AppResult<ChatMessage> {
    let row = sqlx::query_as::<_, ChatMessage>(
        "SELECT id, map_id, role, content, context_node_ids, created_at FROM chat_messages WHERE id = ?",
    )
    .bind(id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

pub async fn history(db: &Db, map_id: &str) -> AppResult<Vec<ChatMessage>> {
    let rows = sqlx::query_as::<_, ChatMessage>(
        "SELECT id, map_id, role, content, context_node_ids, created_at FROM chat_messages \
         WHERE map_id = ? ORDER BY created_at",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}
