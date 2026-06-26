//! Search, settings, AI-backend status, and history commands.

use tauri::State;

use crate::domain::Node;
use crate::error::AppResult;
use crate::repo;
use crate::repo::events::Event;
use crate::state::AppState;

#[tauri::command]
pub async fn search_nodes(
    state: State<'_, AppState>,
    map_id: String,
    query: String,
) -> AppResult<Vec<Node>> {
    repo::search::search_nodes(&state.db, &map_id, &query).await
}

/// Semantic search. Falls back to FTS when no local embedder is available (SPEC §6).
#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    map_id: String,
    query: String,
) -> AppResult<Vec<Node>> {
    let hits =
        crate::embeddings::semantic_search(&state.db, state.embedder.as_ref(), &map_id, &query, 20)
            .await?;
    if hits.is_empty() {
        return repo::search::search_nodes(&state.db, &map_id, &query).await;
    }
    Ok(hits)
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> AppResult<Option<String>> {
    repo::settings::get(&state.db, &key).await
}

#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    repo::settings::set(&state.db, &key, &value).await
}

#[tauri::command]
pub async fn recent_events(
    state: State<'_, AppState>,
    map_id: String,
    limit: i64,
) -> AppResult<Vec<Event>> {
    repo::events::recent(&state.db, &map_id, limit).await
}

// --- AI backend (local Claude Code CLI / OAuth, SPEC §6). ---

/// Probe whether the local `claude` CLI is available. The app drives it as the LLM backend
/// using the user's Claude Code login, so no API key is required.
#[tauri::command]
pub async fn ai_backend_status() -> crate::llm::cli::BackendStatus {
    crate::llm::cli::backend_status().await
}
