//! Challenge (adversarial) commands: persistence, the judgment loop, and promotion of a
//! conceded attack into the argument (SPEC §4.1).

use tauri::State;

use crate::domain::{Challenge, ChallengeStatus, ChallengeTargetKind, Node, NodeStatus};
use crate::error::AppResult;
use crate::repo;
use crate::state::AppState;

/// A conceded attack weakens the hit claim one notch toward `open` (SPEC §4.1: conceding
/// "downgrades the node to bet/open"). Returns None when there's nothing left to weaken.
fn downgraded(status: NodeStatus) -> Option<NodeStatus> {
    match status {
        NodeStatus::Fact | NodeStatus::Evidenced | NodeStatus::Assumption => Some(NodeStatus::Bet),
        NodeStatus::Bet => Some(NodeStatus::Open),
        NodeStatus::Open => None,
    }
}

#[tauri::command]
pub async fn list_pending_challenges(
    state: State<'_, AppState>,
    map_id: String,
) -> AppResult<Vec<Challenge>> {
    repo::challenges::pending(&state.db, &map_id).await
}

/// Litigation history for a node/edge (SPEC §7.10).
#[tauri::command]
pub async fn challenges_for_target(
    state: State<'_, AppState>,
    target_id: String,
) -> AppResult<Vec<Challenge>> {
    repo::challenges::for_target(&state.db, &target_id).await
}

/// User judgment (SPEC §4.1): conceded | rebutted | deferred, with a note.
#[tauri::command]
pub async fn judge_challenge(
    state: State<'_, AppState>,
    id: String,
    status: ChallengeStatus,
    user_note: Option<String>,
) -> AppResult<Challenge> {
    let challenge = repo::challenges::judge(&state.db, &id, status, user_note.as_deref()).await?;
    // Conceding a node-targeted attack is a verdict with teeth: weaken the hit claim so the
    // map reflects that it no longer stands as strongly (SPEC §4.1). Edge-targeted attacks
    // don't carry a node status to downgrade.
    if matches!(status, ChallengeStatus::Conceded)
        && matches!(challenge.target_kind, ChallengeTargetKind::Node)
    {
        if let Ok(node) = repo::nodes::get(&state.db, &challenge.target_id).await {
            if let Some(weaker) = downgraded(node.status) {
                repo::nodes::set_status(&state.db, &challenge.target_id, weaker).await?;
            }
        }
    }
    Ok(challenge)
}

/// Promote a settled challenge into the argument as a rebut node + edge (SPEC §4.1).
/// Atomic, gated on a judged verdict, and idempotent — see repo::challenges::promote.
#[tauri::command]
pub async fn promote_challenge(state: State<'_, AppState>, id: String) -> AppResult<Node> {
    repo::challenges::promote(&state.db, &id).await
}
