//! Node + edge mutation commands.

use tauri::State;

use crate::domain::{Edge, EdgeType, Node, NodeStatus, Origin, Strength};
use crate::error::AppResult;
use crate::repo;
use crate::repo::edges::NewEdge;
use crate::repo::nodes::NewNode;
use crate::state::AppState;

#[tauri::command]
pub async fn create_node(state: State<'_, AppState>, input: NewNode) -> AppResult<Node> {
    let node = repo::nodes::create(&state.db, input).await?;
    repo::maps::touch(&state.db, &node.map_id).await?;
    Ok(node)
}

#[tauri::command]
pub async fn update_node_text(state: State<'_, AppState>, id: String, text: String) -> AppResult<Node> {
    repo::nodes::update_text(&state.db, &id, &text).await
}

#[tauri::command]
pub async fn set_node_status(
    state: State<'_, AppState>,
    id: String,
    status: NodeStatus,
) -> AppResult<Node> {
    repo::nodes::set_status(&state.db, &id, status).await
}

#[tauri::command]
pub async fn set_node_origin(
    state: State<'_, AppState>,
    id: String,
    origin: Origin,
) -> AppResult<Node> {
    repo::nodes::set_origin(&state.db, &id, origin).await
}

#[tauri::command]
pub async fn move_node(state: State<'_, AppState>, id: String, x: f64, y: f64) -> AppResult<()> {
    repo::nodes::move_to(&state.db, &id, x, y).await
}

#[tauri::command]
pub async fn delete_node(state: State<'_, AppState>, id: String) -> AppResult<()> {
    repo::nodes::soft_delete(&state.db, &id).await
}

#[tauri::command]
pub async fn create_edge(state: State<'_, AppState>, input: NewEdge) -> AppResult<Edge> {
    let edge = repo::edges::create(&state.db, input).await?;
    repo::maps::touch(&state.db, &edge.map_id).await?;
    Ok(edge)
}

#[tauri::command]
pub async fn set_edge_type(
    state: State<'_, AppState>,
    id: String,
    edge_type: EdgeType,
) -> AppResult<Edge> {
    repo::edges::set_type(&state.db, &id, edge_type).await
}

#[tauri::command]
pub async fn set_edge_strength(
    state: State<'_, AppState>,
    id: String,
    strength: Option<Strength>,
) -> AppResult<Edge> {
    repo::edges::set_strength(&state.db, &id, strength).await
}

#[tauri::command]
pub async fn delete_edge(state: State<'_, AppState>, id: String) -> AppResult<()> {
    repo::edges::soft_delete(&state.db, &id).await
}
