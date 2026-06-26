//! Full-text search over node text (FTS5). Semantic/vector search is a separate path
//! (see `crate::embeddings`) that gracefully degrades to this when sqlite-vec is absent.

use crate::db::Db;
use crate::domain::Node;
use crate::error::AppResult;

/// Sanitize a user query into a safe FTS5 MATCH expression. With the trigram tokenizer
/// (migration 0002) a quoted phrase does substring matching, which works for CJK. Trigram
/// needs at least 3 characters to match anything.
fn to_match_expr(query: &str) -> String {
    let cleaned = query.trim().replace('"', "");
    if cleaned.chars().count() < 3 {
        return String::new();
    }
    format!("\"{cleaned}\"")
}

/// Full-text search within a map. Empty/whitespace queries return nothing.
pub async fn search_nodes(db: &Db, map_id: &str, query: &str) -> AppResult<Vec<Node>> {
    let expr = to_match_expr(query);
    if expr.is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, Node>(
        "SELECT n.id, n.map_id, n.text, n.status, n.origin, n.x, n.y, \
                n.created_at, n.updated_at, n.deleted_at \
         FROM nodes_fts f \
         JOIN nodes n ON n.id = f.node_id \
         WHERE f.text MATCH ? AND n.map_id = ? AND n.deleted_at IS NULL \
         ORDER BY rank LIMIT 50",
    )
    .bind(expr)
    .bind(map_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}
