//! LLM-backed commands. All outputs are STAGING — the user accepts/judges before anything
//! enters the source of truth (SPEC §5 provenance, §7.2 ambient).

use tauri::ipc::Channel;
use tauri::State;

use crate::domain::{Challenge, ChallengeTargetKind, ChatRole};
use crate::error::AppResult;
use crate::llm;
use crate::llm::types::{GapNode, StreamEvent, Suggestion, WeakPoint};
use crate::repo;
use crate::repo::challenges::NewChallenge;
use crate::state::AppState;

/// Forward inference: candidate downstream nodes from the selection (SPEC §4.1).
#[tauri::command]
pub async fn forward_inference(
    state: State<'_, AppState>,
    map_id: String,
    focus_node_ids: Vec<String>,
) -> AppResult<Vec<Suggestion>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    llm::forward_inference(&state.llm, &graph, &focus_node_ids).await
}

/// Gap detection: missing intermediate claim(s) between two nodes (SPEC §4).
#[tauri::command]
pub async fn detect_gap(
    state: State<'_, AppState>,
    map_id: String,
    from_id: String,
    to_id: String,
) -> AppResult<Vec<GapNode>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    llm::detect_gap(&state.llm, &graph, &from_id, &to_id).await
}

/// The adversarial button: generate attacks and persist them as PENDING challenges
/// (SPEC §4.1). They appear as ghost cards / in the inbox until judged.
#[tauri::command]
pub async fn generate_challenge(
    state: State<'_, AppState>,
    map_id: String,
    target_kind: ChallengeTargetKind,
    target_id: String,
    diverse: bool,
) -> AppResult<Vec<Challenge>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    // Validate the target exists and is live in this map (low.9).
    let target_ok = match target_kind {
        ChallengeTargetKind::Node => graph.nodes.iter().any(|n| n.id == target_id),
        ChallengeTargetKind::Edge => graph.edges.iter().any(|e| e.id == target_id),
    };
    if !target_ok {
        return Err(crate::error::AppError::NotFound(format!(
            "challenge target {target_id}"
        )));
    }
    let generated = llm::generate_challenges(
        &state.llm,
        &graph,
        target_kind.as_str(),
        &target_id,
        diverse,
    )
    .await?;

    let mut out = Vec::new();
    for g in generated {
        let ch = repo::challenges::create(
            &state.db,
            NewChallenge {
                map_id: map_id.clone(),
                target_kind,
                target_id: target_id.clone(),
                kind: g.kind,
                content: g.content,
            },
        )
        .await?;
        out.push(ch);
    }
    Ok(out)
}

/// Whole-map scan for the most attackable nodes (SPEC §8 whole-map mode).
#[tauri::command]
pub async fn scan_weak_points(
    state: State<'_, AppState>,
    map_id: String,
) -> AppResult<Vec<WeakPoint>> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    llm::scan_weak_points(&state.llm, &graph).await
}

/// Context-aware streaming chat (SPEC §7.9). Deltas stream to the frontend via `on_event`.
#[tauri::command]
pub async fn chat(
    state: State<'_, AppState>,
    map_id: String,
    message: String,
    context_node_ids: Vec<String>,
    on_event: Channel<StreamEvent>,
) -> AppResult<()> {
    let graph = repo::maps::graph(&state.db, &map_id).await?;
    repo::chat::append(&state.db, &map_id, ChatRole::User, &message, &context_node_ids).await?;

    // chat_stream emits Done/Error on the channel itself, so a failure is already surfaced
    // to the frontend; don't also reject the invoke promise (nit.3). Persist the assistant
    // turn only on success.
    if let Ok(full) = llm::chat_stream(&state.llm, &graph, &context_node_ids, &message, |ev| {
        let _ = on_event.send(ev);
    })
    .await
    {
        repo::chat::append(
            &state.db,
            &map_id,
            ChatRole::Assistant,
            &full,
            &context_node_ids,
        )
        .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn chat_history(
    state: State<'_, AppState>,
    map_id: String,
) -> AppResult<Vec<crate::domain::ChatMessage>> {
    repo::chat::history(&state.db, &map_id).await
}
