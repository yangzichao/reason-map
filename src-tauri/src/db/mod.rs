//! SQLite connection + migrations. WAL, foreign keys, and a busy timeout are set per
//! connection here (cannot live inside the migration transaction).

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use crate::error::AppResult;

pub type Db = SqlitePool;

/// Open (creating if needed) the SQLite database at `path` and run migrations.
pub async fn connect(path: &Path) -> AppResult<Db> {
    let url = format!("sqlite://{}", path.to_string_lossy());
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal) // durability + concurrent reads (SPEC §5)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// An in-memory database, used by tests.
#[cfg(test)]
pub async fn connect_memory() -> AppResult<Db> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// New monotonic, sortable id (ULID) generated app-side (SPEC §5: never autoincrement).
pub fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

/// Current timestamp as an ISO-8601 string (the storage format for all timestamps).
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
