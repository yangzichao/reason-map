//! Map + graph + analysis commands.

use tauri::State;

use crate::analysis;
use crate::domain::{Map, MapGraph, NodeCriticality};
use crate::error::AppResult;
use crate::repo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_maps(state: State<'_, AppState>) -> AppResult<Vec<Map>> {
    repo::maps::list(&state.db).await
}

#[tauri::command]
pub async fn create_map(state: State<'_, AppState>, title: String) -> AppResult<Map> {
    repo::maps::create(&state.db, &title).await
}

#[tauri::command]
pub async fn rename_map(state: State<'_, AppState>, id: String, title: String) -> AppResult<Map> {
    repo::maps::rename(&state.db, &id, &title).await
}

#[tauri::command]
pub async fn delete_map(state: State<'_, AppState>, id: String) -> AppResult<()> {
    repo::maps::soft_delete(&state.db, &id).await
}

/// Export one map as a self-contained `.argmap.json` document (SPEC §5: JSON is the share /
/// git / diff format; the DB stays source of truth). Bundles graph + per-node evidence.
#[tauri::command]
pub async fn export_map(state: State<'_, AppState>, id: String) -> AppResult<String> {
    let graph = repo::maps::graph(&state.db, &id).await?;
    let mut evidence = Vec::new();
    for node in &graph.nodes {
        evidence.extend(repo::evidence::for_node(&state.db, &node.id).await?);
    }
    let doc = serde_json::json!({
        "format": "reason-map.argmap",
        "version": 1,
        "map": graph.map,
        "nodes": graph.nodes,
        "edges": graph.edges,
        "challenges": graph.challenges,
        "evidence": evidence,
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

#[tauri::command]
pub async fn load_graph(state: State<'_, AppState>, map_id: String) -> AppResult<MapGraph> {
    repo::maps::graph(&state.db, &map_id).await
}

/// Structural criticality for every node — derived in memory, never stored (SPEC §3).
#[tauri::command]
pub async fn analyze_map(state: State<'_, AppState>, map_id: String) -> AppResult<Vec<NodeCriticality>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    Ok(analysis::criticality(&graph))
}

/// Node ids that participate in circular reasoning (SPEC §5).
#[tauri::command]
pub async fn detect_circular(state: State<'_, AppState>, map_id: String) -> AppResult<Vec<String>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    Ok(analysis::circular_nodes(&graph).into_iter().collect())
}

/// Undo the most recent change on a map (multi-level; SPEC §5). Returns false if nothing
/// to undo.
#[tauri::command]
pub async fn undo(state: State<'_, AppState>, map_id: String) -> AppResult<bool> {
    repo::history::undo_last(&state.db, &map_id).await
}
