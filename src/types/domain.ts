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
export type EvidenceKind = "url" | "quote" | "data" | "file";

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

export interface Evidence {
  id: string;
  nodeId: string;
  kind: EvidenceKind;
  payload: string; // JSON string, app shape: { value: string }
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
// Sage Desk 配色：低饱和、土质，与主题统一（绿=确证/支持，陶土=赌/反驳，赭黄=假设）。
export const STATUS_META: Record<NodeStatus, { label: string; color: string; glow: boolean }> = {
  fact: { label: "事实", color: "#4e7a52", glow: false }, // 叶绿 — 已确证
  evidenced: { label: "有证据", color: "#4c8c8a", glow: false }, // 静蓝绿
  assumption: { label: "假设", color: "#c2913c", glow: true }, // 赭黄 — 待验
  bet: { label: "赌", color: "#b0613a", glow: true }, // 陶土 — 高风险
  open: { label: "开放", color: "#8c9486", glow: false }, // 鼠尾草灰
};

export const EDGE_META: Record<EdgeType, { label: string; color: string }> = {
  support: { label: "支持", color: "#6e8a5c" }, // 沙绿
  rebut: { label: "反驳", color: "#b0613a" }, // 陶土
  premise_of: { label: "前提", color: "#9a8467" }, // 暖棕
  depends_on: { label: "依赖", color: "#7e97a4" }, // 灰蓝
};

export const CHALLENGE_KIND_LABEL: Record<ChallengeKind, string> = {
  rebuttal: "反驳",
  counterexample: "反例",
  hidden_assumption: "隐藏假设",
  alternative: "替代解释",
  non_sequitur: "跳步",
};

export const STRENGTH_LABEL: Record<Strength, string> = {
  strong: "强",
  weak: "弱",
  tentative: "存疑",
};

export const EVIDENCE_KIND_LABEL: Record<EvidenceKind, string> = {
  url: "链接",
  quote: "引文",
  data: "数据",
  file: "文件",
};

export const ORIGIN_LABEL: Record<Origin, string> = {
  user: "我写的",
  ai_suggested: "AI 建议",
  ai_accepted: "AI(已采纳)",
};
