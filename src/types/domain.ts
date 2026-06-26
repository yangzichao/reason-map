// Frontend mirror of the Rust domain (serde rename_all = camelCase). Keep in sync with
// src-tauri/src/domain/mod.rs.

export type NodeStatus = "fact" | "assumption" | "bet" | "evidenced" | "open";
export type Origin = "user" | "ai_suggested" | "ai_accepted";
export type EdgeType = "support" | "rebut" | "premise_of" | "depends_on";
export type Strength = "strong" | "weak" | "tentative";
export type ChallengeTargetKind = "node" | "edge";
export type ChallengeKind =
  | "rebuttal"
  | "counterexample"
  | "hidden_assumption"
  | "alternative"
  | "non_sequitur";
export type ChallengeStatus = "pending" | "conceded" | "rebutted" | "deferred";
export type ChatRole = "user" | "assistant" | "system";

export interface MapDoc {
  id: string;
  title: string;
  meta: string;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface ClaimNode {
  id: string;
  mapId: string;
  text: string;
  status: NodeStatus;
  origin: Origin;
  x: number;
  y: number;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface RelationEdge {
  id: string;
  mapId: string;
  fromNode: string;
  toNode: string;
  edgeType: EdgeType;
  strength: Strength | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface Challenge {
  id: string;
  mapId: string;
  targetKind: ChallengeTargetKind;
  targetId: string;
  kind: ChallengeKind;
  content: string;
  status: ChallengeStatus;
  verdict: string | null;
  userNote: string | null;
  createdAt: string;
  resolvedAt: string | null;
}

export interface ChatMessageRow {
  id: string;
  mapId: string;
  role: ChatRole;
  content: string;
  contextNodeIds: string; // JSON-encoded string[] on the wire
  createdAt: string;
}

export interface MapGraph {
  map: MapDoc;
  nodes: ClaimNode[];
  edges: RelationEdge[];
  challenges: Challenge[];
}

export interface NodeCriticality {
  nodeId: string;
  downstreamCount: number;
  isLoadBearing: boolean;
  isWeakLink: boolean;
  openChallenges: number;
}

export interface Suggestion {
  text: string;
  rationale: string;
  suggestedStatus: NodeStatus;
}

export interface GapNode {
  text: string;
  rationale: string;
}

export interface WeakPoint {
  nodeId: string;
  reason: string;
}

export type StreamEvent =
  | { type: "delta"; text: string }
  | { type: "done"; full: string }
  | { type: "error"; message: string };

// Readiness of the local AI backend (the `claude` CLI, driven via the user's Claude Code login).
export interface AiBackendStatus {
  ready: boolean;
  version: string | null;
  detail: string;
}

// Display metadata for statuses and edge/challenge kinds (SPEC §7.4: weak points glow).
export const STATUS_META: Record<NodeStatus, { label: string; color: string; glow: boolean }> = {
  fact: { label: "事实", color: "#34d399", glow: false },
  evidenced: { label: "有证据", color: "#22d3ee", glow: false },
  assumption: { label: "假设", color: "#fbbf24", glow: true },
  bet: { label: "赌", color: "#fb7185", glow: true },
  open: { label: "开放", color: "#94a3b8", glow: false },
};

export const EDGE_META: Record<EdgeType, { label: string; color: string }> = {
  support: { label: "支持", color: "#34d399" },
  rebut: { label: "反驳", color: "#fb7185" },
  premise_of: { label: "前提", color: "#818cf8" },
  depends_on: { label: "依赖", color: "#a78bfa" },
};

export const CHALLENGE_KIND_LABEL: Record<ChallengeKind, string> = {
  rebuttal: "反驳",
  counterexample: "反例",
  hidden_assumption: "隐藏假设",
  alternative: "替代解释",
  non_sequitur: "跳步",
};
