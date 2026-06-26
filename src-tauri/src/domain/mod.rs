//! Domain model. Enums are stored as TEXT (matching the schema CHECK constraints) and
//! serialize to snake_case JSON for the frontend. Timestamps are ISO-8601 strings.

use serde::{Deserialize, Serialize};

macro_rules! text_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
        #[serde(rename_all = "snake_case")]
        #[sqlx(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }
        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }
        }
    };
}

text_enum!(NodeStatus {
    Fact => "fact",
    Assumption => "assumption",
    Bet => "bet",
    Evidenced => "evidenced",
    Open => "open",
});

text_enum!(Origin {
    User => "user",
    AiSuggested => "ai_suggested",
    AiAccepted => "ai_accepted",
});

text_enum!(EdgeType {
    Support => "support",
    Rebut => "rebut",
    PremiseOf => "premise_of",
    DependsOn => "depends_on",
});

text_enum!(Strength {
    Strong => "strong",
    Weak => "weak",
    Tentative => "tentative",
});

text_enum!(EvidenceKind {
    Url => "url",
    Quote => "quote",
    Data => "data",
    File => "file",
});

text_enum!(ChallengeTargetKind {
    Node => "node",
    Edge => "edge",
});

text_enum!(ChallengeKind {
    Rebuttal => "rebuttal",
    Counterexample => "counterexample",
    HiddenAssumption => "hidden_assumption",
    Alternative => "alternative",
    NonSequitur => "non_sequitur",
});

text_enum!(ChallengeStatus {
    Pending => "pending",
    Conceded => "conceded",
    Rebutted => "rebutted",
    Deferred => "deferred",
});

text_enum!(ChatRole {
    User => "user",
    Assistant => "assistant",
    System => "system",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub map_id: String,
    pub text: String,
    pub status: NodeStatus,
    pub origin: Origin,
    pub x: f64,
    pub y: f64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub map_id: String,
    pub from_node: String,
    pub to_node: String,
    #[sqlx(rename = "type")]
    pub edge_type: EdgeType,
    pub strength: Option<Strength>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub node_id: String,
    pub kind: EvidenceKind,
    pub payload: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Challenge {
    pub id: String,
    pub map_id: String,
    pub target_kind: ChallengeTargetKind,
    pub target_id: String,
    pub kind: ChallengeKind,
    pub content: String,
    pub status: ChallengeStatus,
    pub verdict: Option<String>,
    pub user_note: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub map_id: String,
    pub role: ChatRole,
    pub content: String,
    pub context_node_ids: String,
    pub created_at: String,
}

/// The full graph for one map, sent to the frontend in a single round trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapGraph {
    pub map: Map,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub challenges: Vec<Challenge>,
}

/// Structural criticality of a node, computed in memory (SPEC §3) — derived, never stored.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCriticality {
    pub node_id: String,
    /// How many nodes ultimately depend on this one (downstream reachability).
    pub downstream_count: usize,
    /// True if removing this node disconnects part of the argument (articulation point).
    pub is_load_bearing: bool,
    /// True if this node is a bet/assumption AND load-bearing — the weakest links.
    pub is_weak_link: bool,
    /// Count of unresolved (pending/conceded) challenges against this node.
    pub open_challenges: usize,
}
