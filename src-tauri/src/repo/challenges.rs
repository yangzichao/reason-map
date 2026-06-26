//! Challenge (adversarial attack) persistence — the staging layer (SPEC §4.1).
//! Challenges are NOT argument nodes; the user promotes them explicitly.

use serde::Deserialize;

use crate::db::{new_id, now, Db};
use crate::domain::{
    Challenge, ChallengeKind, ChallengeStatus, ChallengeTargetKind, EdgeType, Node, Origin,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewChallenge {
    pub map_id: String,
    pub target_kind: ChallengeTargetKind,
    pub target_id: String,
    pub kind: ChallengeKind,
    pub content: String,
}

pub async fn get(db: &Db, id: &str) -> AppResult<Challenge> {
    sqlx::query_as::<_, Challenge>(
        "SELECT id, map_id, target_kind, target_id, kind, content, status, verdict, user_note, \
         created_at, resolved_at FROM challenges WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("challenge {id}")))
}

pub async fn create(db: &Db, input: NewChallenge) -> AppResult<Challenge> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO challenges (id, map_id, target_kind, target_id, kind, content, status, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id)
    .bind(&input.map_id)
    .bind(input.target_kind)
    .bind(&input.target_id)
    .bind(input.kind)
    .bind(&input.content)
    .bind(now())
    .execute(db)
    .await?;
    get(db, &id).await
}

/// User judgment. `status` must be a terminal verdict (conceded/rebutted/deferred).
pub async fn judge(
    db: &Db,
    id: &str,
    status: ChallengeStatus,
    user_note: Option<&str>,
) -> AppResult<Challenge> {
    if matches!(status, ChallengeStatus::Pending) {
        return Err(AppError::Invalid("judgment cannot be 'pending'".into()));
    }
    sqlx::query(
        "UPDATE challenges SET status = ?, verdict = ?, user_note = ?, resolved_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(status.as_str())
    .bind(user_note)
    .bind(now())
    .bind(id)
    .execute(db)
    .await?;
    get(db, id).await
}

/// Promote a settled (conceded/rebutted) node-targeted challenge into the argument as a
/// rebut node + edge — atomically and at most once (SPEC §4.1; fixes the duplicate/orphan
/// bug). Returns the created node.
pub async fn promote(db: &Db, id: &str) -> AppResult<Node> {
    let ch = get(db, id).await?;
    if !matches!(ch.target_kind, ChallengeTargetKind::Node) {
        return Err(AppError::Invalid(
            "only node-targeted challenges can be promoted to a node".into(),
        ));
    }
    // SPEC §4.1: both 认(conceded) and 驳(rebutted) can become a node; pending/deferred cannot.
    if !matches!(ch.status, ChallengeStatus::Conceded | ChallengeStatus::Rebutted) {
        return Err(AppError::Invalid(
            "only a judged (conceded/rebutted) challenge can be promoted".into(),
        ));
    }
    // Idempotency: never spawn a second node for the same challenge.
    let existing: Option<(Option<String>,)> =
        sqlx::query_as("SELECT promoted_node_id FROM challenges WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?;
    if let Some((Some(node_id),)) = existing {
        return crate::repo::nodes::get(db, &node_id).await;
    }

    let node_id = new_id();
    let edge_id = new_id();
    let ts = now();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO nodes (id, map_id, text, status, origin, x, y, created_at, updated_at) \
         VALUES (?, ?, ?, 'open', ?, 0, 0, ?, ?)",
    )
    .bind(&node_id)
    .bind(&ch.map_id)
    .bind(&ch.content)
    .bind(Origin::AiAccepted)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO edges (id, map_id, from_node, to_node, type, strength, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(&edge_id)
    .bind(&ch.map_id)
    .bind(&node_id)
    .bind(&ch.target_id)
    .bind(EdgeType::Rebut)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE challenges SET promoted_node_id = ? WHERE id = ?")
        .bind(&node_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE maps SET updated_at = ? WHERE id = ?")
        .bind(&ts)
        .bind(&ch.map_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    crate::repo::nodes::get(db, &node_id).await
}

/// All pending challenges for a map — drives the challenge inbox (SPEC §7.3).
pub async fn pending(db: &Db, map_id: &str) -> AppResult<Vec<Challenge>> {
    let rows = sqlx::query_as::<_, Challenge>(
        "SELECT id, map_id, target_kind, target_id, kind, content, status, verdict, user_note, \
         created_at, resolved_at FROM challenges \
         WHERE map_id = ? AND status = 'pending' ORDER BY created_at",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Full litigation history for a node/edge (SPEC §7.10).
pub async fn for_target(db: &Db, target_id: &str) -> AppResult<Vec<Challenge>> {
    let rows = sqlx::query_as::<_, Challenge>(
        "SELECT id, map_id, target_kind, target_id, kind, content, status, verdict, user_note, \
         created_at, resolved_at FROM challenges \
         WHERE target_id = ? ORDER BY created_at DESC",
    )
    .bind(target_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}
