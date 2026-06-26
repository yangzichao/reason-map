//! High-level LLM operations: forward inference, gap detection, adversarial challenge,
//! weak-point scan, and streaming chat. All produce STAGING data the user judges.

pub mod cli;
pub mod prompts;
pub mod types;

use serde::Deserialize;

use crate::domain::MapGraph;
use crate::error::{AppError, AppResult};
use cli::ClaudeCli;
use types::{GapNode, GeneratedChallenge, StreamEvent, Suggestion, WeakPoint};

/// Pull the first balanced JSON object out of a model response (it may add prose around it).
fn extract_json(s: &str) -> AppResult<&str> {
    let start = s.find('{').ok_or_else(|| AppError::Llm("no JSON in response".into()))?;
    let end = s.rfind('}').ok_or_else(|| AppError::Llm("no JSON in response".into()))?;
    if end < start {
        return Err(AppError::Llm("malformed JSON in response".into()));
    }
    Ok(&s[start..=end])
}

fn parse<T: for<'de> Deserialize<'de>>(s: &str) -> AppResult<T> {
    let json = extract_json(s)?;
    serde_json::from_str(json).map_err(|e| AppError::Llm(format!("parse: {e} — body: {json}")))
}

#[derive(Deserialize)]
struct SuggestionsWrap {
    suggestions: Vec<Suggestion>,
}
#[derive(Deserialize)]
struct GapsWrap {
    gaps: Vec<GapNode>,
}
#[derive(Deserialize)]
struct ChallengesWrap {
    challenges: Vec<GeneratedChallenge>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeakPointsWrap {
    weak_points: Vec<WeakPoint>,
}

pub async fn forward_inference(
    client: &ClaudeCli,
    graph: &MapGraph,
    focus: &[String],
) -> AppResult<Vec<Suggestion>> {
    let (system, user) = prompts::forward_inference_prompt(graph, focus);
    let text = client.complete_text(&system, &user, 1500, true).await?;
    Ok(parse::<SuggestionsWrap>(&text)?.suggestions)
}

pub async fn detect_gap(
    client: &ClaudeCli,
    graph: &MapGraph,
    from_id: &str,
    to_id: &str,
) -> AppResult<Vec<GapNode>> {
    let (system, user) = prompts::gap_detection_prompt(graph, from_id, to_id);
    let text = client.complete_text(&system, &user, 1500, true).await?;
    Ok(parse::<GapsWrap>(&text)?.gaps)
}

pub async fn generate_challenges(
    client: &ClaudeCli,
    graph: &MapGraph,
    target_kind: &str,
    target_id: &str,
    diverse: bool,
) -> AppResult<Vec<GeneratedChallenge>> {
    let (system, user) = prompts::challenge_prompt(graph, target_kind, target_id, diverse);
    let text = client.complete_text(&system, &user, 2000, true).await?;
    Ok(parse::<ChallengesWrap>(&text)?.challenges)
}

pub async fn scan_weak_points(
    client: &ClaudeCli,
    graph: &MapGraph,
) -> AppResult<Vec<WeakPoint>> {
    let (system, user) = prompts::weak_points_prompt(graph);
    let text = client.complete_text(&system, &user, 1500, true).await?;
    Ok(parse::<WeakPointsWrap>(&text)?.weak_points)
}

/// Stream a chat turn, invoking `on_event` for each delta and on completion.
pub async fn chat_stream<F: FnMut(StreamEvent)>(
    client: &ClaudeCli,
    graph: &MapGraph,
    focus: &[String],
    user_message: &str,
    mut on_event: F,
) -> AppResult<String> {
    let system = prompts::chat_system(graph, focus);
    let full = client
        .stream_text(&system, user_message, 4000, true, |delta| {
            on_event(StreamEvent::Delta {
                text: delta.to_string(),
            })
        })
        .await;
    match full {
        Ok(text) => {
            on_event(StreamEvent::Done { full: text.clone() });
            Ok(text)
        }
        Err(e) => {
            on_event(StreamEvent::Error {
                message: e.to_string(),
            });
            Err(e)
        }
    }
}
