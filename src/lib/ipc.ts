// Typed wrappers over the Tauri command surface. One function per backend command,
// keeping all `invoke` string literals in a single place.

import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  AiBackendStatus,
  Challenge,
  ChallengeStatus,
  ChallengeTargetKind,
  ChatMessageRow,
  ClaimNode,
  EdgeType,
  GapNode,
  MapDoc,
  MapGraph,
  NodeCriticality,
  NodeStatus,
  Origin,
  RelationEdge,
  Strength,
  StreamEvent,
  Suggestion,
  WeakPoint,
} from "@/types/domain";

// --- maps + analysis ---
export const listMaps = () => invoke<MapDoc[]>("list_maps");
export const createMap = (title: string) => invoke<MapDoc>("create_map", { title });
export const renameMap = (id: string, title: string) => invoke<MapDoc>("rename_map", { id, title });
export const deleteMap = (id: string) => invoke<void>("delete_map", { id });
export const loadGraph = (mapId: string) => invoke<MapGraph>("load_graph", { mapId });
export const analyzeMap = (mapId: string) => invoke<NodeCriticality[]>("analyze_map", { mapId });
export const detectCircular = (mapId: string) => invoke<string[]>("detect_circular", { mapId });
export const undo = (mapId: string) => invoke<boolean>("undo", { mapId });

// --- graph edit ---
export interface NewNodeInput {
  mapId: string;
  text: string;
  status?: NodeStatus;
  origin?: Origin;
  x?: number;
  y?: number;
}
export const createNode = (input: NewNodeInput) => invoke<ClaimNode>("create_node", { input });
export const updateNodeText = (id: string, text: string) =>
  invoke<ClaimNode>("update_node_text", { id, text });
export const setNodeStatus = (id: string, status: NodeStatus) =>
  invoke<ClaimNode>("set_node_status", { id, status });
export const setNodeOrigin = (id: string, origin: Origin) =>
  invoke<ClaimNode>("set_node_origin", { id, origin });
export const moveNode = (id: string, x: number, y: number) =>
  invoke<void>("move_node", { id, x, y });
export const deleteNode = (id: string) => invoke<void>("delete_node", { id });

export interface NewEdgeInput {
  mapId: string;
  fromNode: string;
  toNode: string;
  edgeType?: EdgeType;
  strength?: Strength | null;
}
export const createEdge = (input: NewEdgeInput) => invoke<RelationEdge>("create_edge", { input });
export const setEdgeType = (id: string, edgeType: EdgeType) =>
  invoke<RelationEdge>("set_edge_type", { id, edgeType });
export const setEdgeStrength = (id: string, strength: Strength | null) =>
  invoke<RelationEdge>("set_edge_strength", { id, strength });
export const deleteEdge = (id: string) => invoke<void>("delete_edge", { id });

// --- challenges ---
export const listPendingChallenges = (mapId: string) =>
  invoke<Challenge[]>("list_pending_challenges", { mapId });
export const challengesForTarget = (targetId: string) =>
  invoke<Challenge[]>("challenges_for_target", { targetId });
export const judgeChallenge = (id: string, status: ChallengeStatus, userNote?: string) =>
  invoke<Challenge>("judge_challenge", { id, status, userNote: userNote ?? null });
export const promoteChallenge = (id: string) => invoke<ClaimNode>("promote_challenge", { id });

// --- ai (staging) ---
export const forwardInference = (mapId: string, focusNodeIds: string[]) =>
  invoke<Suggestion[]>("forward_inference", { mapId, focusNodeIds });
export const detectGap = (mapId: string, fromId: string, toId: string) =>
  invoke<GapNode[]>("detect_gap", { mapId, fromId, toId });
export const generateChallenge = (
  mapId: string,
  targetKind: ChallengeTargetKind,
  targetId: string,
  diverse: boolean,
) => invoke<Challenge[]>("generate_challenge", { mapId, targetKind, targetId, diverse });
export const scanWeakPoints = (mapId: string) => invoke<WeakPoint[]>("scan_weak_points", { mapId });
export const chatHistory = (mapId: string) => invoke<ChatMessageRow[]>("chat_history", { mapId });

/// Streaming chat. `onEvent` fires for each delta and on done/error.
export function chat(
  mapId: string,
  message: string,
  contextNodeIds: string[],
  onEvent: (e: StreamEvent) => void,
): Promise<void> {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("chat", { mapId, message, contextNodeIds, onEvent: channel });
}

// --- misc ---
export const searchNodes = (mapId: string, query: string) =>
  invoke<ClaimNode[]>("search_nodes", { mapId, query });
export const semanticSearch = (mapId: string, query: string) =>
  invoke<ClaimNode[]>("semantic_search", { mapId, query });
export const getSetting = (key: string) => invoke<string | null>("get_setting", { key });
export const setSetting = (key: string, value: string) =>
  invoke<void>("set_setting", { key, value });
export const aiBackendStatus = () => invoke<AiBackendStatus>("ai_backend_status");
