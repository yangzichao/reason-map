//! Key/value application settings, kept separate from document data (SPEC §5).
//! The app needs no API-key secret: the LLM backend uses the local Claude Code login
//! (see `crate::llm::cli`).

use crate::db::Db;
use crate::error::AppResult;

pub async fn get(db: &Db, key: &str) -> AppResult<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set(db: &Db, key: &str, value: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}
