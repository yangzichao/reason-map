//! Evidence commands: attach / list / detach citations on a claim (SPEC §2).

use tauri::State;

use crate::domain::{Evidence, EvidenceKind};
use crate::error::AppResult;
use crate::repo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_evidence(state: State<'_, AppState>, node_id: String) -> AppResult<Vec<Evidence>> {
    repo::evidence::for_node(&state.db, &node_id).await
}

#[tauri::command]
pub async fn add_evidence(
    state: State<'_, AppState>,
    node_id: String,
    kind: EvidenceKind,
    payload: String,
) -> AppResult<Evidence> {
    repo::evidence::add(&state.db, &node_id, kind, &payload).await
}

#[tauri::command]
pub async fn delete_evidence(state: State<'_, AppState>, id: String) -> AppResult<()> {
    repo::evidence::delete(&state.db, &id).await
}
