//! DTOs the LLM is asked to produce (parsed from its JSON), plus streaming events.
//!
//! The model is generative: it sometimes returns enum labels outside our closed sets (e.g. a
//! `kind` of `counter_evidence`). We deserialize those fields LENIENTLY — normalize the string
//! and map anything unrecognized to the closest valid variant — so one hallucinated label never
//! discards an entire batch of suggestions/challenges.

use serde::{Deserialize, Deserializer, Serialize};

use crate::domain::{ChallengeKind, ChallengeTargetKind, NodeStatus};

/// A candidate downstream node from forward inference (SPEC §4.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub text: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default = "default_open", deserialize_with = "lenient_status")]
    pub suggested_status: NodeStatus,
}

/// A candidate missing intermediate node from gap detection (SPEC §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GapNode {
    pub text: String,
    #[serde(default)]
    pub rationale: String,
}

/// A generated adversarial attack, before it is persisted as a `challenges` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedChallenge {
    #[serde(deserialize_with = "lenient_challenge_kind")]
    pub kind: ChallengeKind,
    pub content: String,
    #[serde(default)]
    pub target_kind: Option<ChallengeTargetKind>,
    #[serde(default)]
    pub target_id: Option<String>,
}

/// A weak point the LLM flags when scanning the whole map (SPEC §8: whole-map scan mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakPoint {
    pub node_id: String,
    pub reason: String,
}

fn default_open() -> NodeStatus {
    NodeStatus::Open
}

/// Normalize a raw label: lowercase, trim, and collapse spaces/hyphens to underscores.
fn normalize(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

/// Map any model-produced status string to a valid `NodeStatus`; unknown → `open`.
fn lenient_status<'de, D: Deserializer<'de>>(d: D) -> Result<NodeStatus, D::Error> {
    let raw = String::deserialize(d)?;
    Ok(match normalize(&raw).as_str() {
        "fact" => NodeStatus::Fact,
        "evidenced" | "evidence" | "supported" => NodeStatus::Evidenced,
        "assumption" | "hidden_assumption" => NodeStatus::Assumption,
        "bet" | "speculation" => NodeStatus::Bet,
        _ => NodeStatus::Open,
    })
}

/// Map any model-produced challenge-kind string to a valid `ChallengeKind`. Unrecognized
/// labels (e.g. `counter_evidence`) fall back to the closest fit, `rebuttal`.
fn lenient_challenge_kind<'de, D: Deserializer<'de>>(d: D) -> Result<ChallengeKind, D::Error> {
    let raw = String::deserialize(d)?;
    Ok(match normalize(&raw).as_str() {
        "counterexample" | "counter_example" => ChallengeKind::Counterexample,
        "hidden_assumption" | "assumption" | "implicit_assumption" | "unstated_premise" => {
            ChallengeKind::HiddenAssumption
        }
        "alternative" | "alternative_explanation" | "alternative_hypothesis" => {
            ChallengeKind::Alternative
        }
        "non_sequitur" | "nonsequitur" | "logical_fallacy" | "invalid_inference" => {
            ChallengeKind::NonSequitur
        }
        // rebuttal, rebut, refutation, counter_evidence, and any other label → rebuttal.
        _ => ChallengeKind::Rebuttal,
    })
}

/// Events streamed to the frontend during a chat turn (SPEC §7.2: ambient, streaming).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "delta")]
    Delta { text: String },
    #[serde(rename = "done")]
    Done { full: String },
    #[serde(rename = "error")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_challenge_kind_falls_back_to_rebuttal() {
        // The exact label from the reported parse error.
        let c: GeneratedChallenge =
            serde_json::from_str(r#"{"kind":"counter_evidence","content":"x"}"#).unwrap();
        assert_eq!(c.kind, ChallengeKind::Rebuttal);
    }

    #[test]
    fn known_challenge_kinds_and_synonyms_map_correctly() {
        let cases = [
            (r#"{"kind":"counterexample","content":"x"}"#, ChallengeKind::Counterexample),
            (r#"{"kind":"Hidden Assumption","content":"x"}"#, ChallengeKind::HiddenAssumption),
            (r#"{"kind":"non-sequitur","content":"x"}"#, ChallengeKind::NonSequitur),
        ];
        for (json, want) in cases {
            let c: GeneratedChallenge = serde_json::from_str(json).unwrap();
            assert_eq!(c.kind, want, "json: {json}");
        }
    }

    #[test]
    fn unknown_or_missing_status_falls_back_to_open() {
        let bad: Suggestion =
            serde_json::from_str(r#"{"text":"t","suggestedStatus":"speculative"}"#).unwrap();
        assert_eq!(bad.suggested_status, NodeStatus::Open);
        let missing: Suggestion = serde_json::from_str(r#"{"text":"t"}"#).unwrap();
        assert_eq!(missing.suggested_status, NodeStatus::Open);
    }
}
