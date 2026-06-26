// Central app store. Keeps the loaded graph, derived analysis, selection, and AI staging.
// Structural mutations reload the graph + analysis (graphs are small, so this stays correct
// and simple rather than maintaining local invariants by hand).

import { create } from "zustand";
import * as ipc from "@/lib/ipc";
import { autoLayout } from "@/components/canvas/layout";
import type {
  Challenge,
  ChallengeStatus,
  ClaimNode,
  EdgeType,
  GapNode,
  MapDoc,
  MapGraph,
  NodeCriticality,
  NodeStatus,
  Strength,
  Suggestion,
  WeakPoint,
} from "@/types/domain";

const LAST_MAP_KEY = "lastMapId";

const STATUS_CYCLE: NodeStatus[] = ["open", "assumption", "bet", "evidenced", "fact"];

interface GapStaging {
  fromId: string;
  toId: string;
  items: GapNode[];
}

interface SuggestionStaging {
  focus: string[]; // the selection the suggestions were generated from (low.2)
  items: Suggestion[];
}

interface AppStore {
  maps: MapDoc[];
  currentMapId: string | null;
  graph: MapGraph | null;
  criticality: Record<string, NodeCriticality>;
  circular: string[];
  selectedNodeIds: string[];
  selectedEdgeId: string | null;
  view: "graph" | "outline";
  focusMode: boolean;
  suggestions: SuggestionStaging | null;
  gaps: GapStaging | null;
  weakPoints: WeakPoint[] | null;
  searchQuery: string;
  searchResults: ClaimNode[];
  aiBusy: string | null; // label of the in-flight AI op, or null
  aiReady: boolean; // the local `claude` CLI backend is available
  aiChecked: boolean; // backend status has been probed at least once
  aiDetail: string; // human-readable backend status / how-to-fix
  aiVersion: string | null;
  error: string | null;

  init: () => Promise<void>;
  refreshAiStatus: () => Promise<void>;
  refreshMaps: () => Promise<void>;
  openMap: (id: string) => Promise<void>;
  newMap: (title: string) => Promise<void>;
  renameMap: (id: string, title: string) => Promise<void>;
  deleteMap: (id: string) => Promise<void>;
  refreshGraph: () => Promise<void>;

  selectNode: (id: string, additive: boolean) => void;
  selectEdge: (id: string | null) => void;
  clearSelection: () => void;
  setView: (v: "graph" | "outline") => void;
  toggleFocus: () => void;
  setError: (e: string | null) => void;

  addNode: (text: string, status?: NodeStatus, x?: number, y?: number) => Promise<string | null>;
  editNode: (id: string, text: string) => Promise<void>;
  cycleStatus: (id: string) => Promise<void>;
  setStatus: (id: string, status: NodeStatus) => Promise<void>;
  removeNode: (id: string) => Promise<void>;
  persistNodePosition: (id: string, x: number, y: number) => Promise<void>;
  undo: () => Promise<void>;
  applyAutoLayout: () => Promise<void>;

  addEdge: (fromNode: string, toNode: string) => Promise<void>;
  removeEdge: (id: string) => Promise<void>;

  runForwardInference: () => Promise<void>;
  acceptSuggestion: (s: Suggestion) => Promise<void>;
  dismissSuggestions: () => void;
  runGapDetection: () => Promise<void>;
  acceptGap: (g: GapNode) => Promise<void>;
  dismissGaps: () => void;
  runChallenge: (diverse: boolean) => Promise<void>;
  runWeakPointScan: () => Promise<void>;
  judge: (id: string, status: ChallengeStatus, note?: string) => Promise<void>;
  promote: (id: string) => Promise<void>;
}

function indexCriticality(list: NodeCriticality[]): Record<string, NodeCriticality> {
  const out: Record<string, NodeCriticality> = {};
  for (const c of list) out[c.nodeId] = c;
  return out;
}

async function withAi<T>(
  set: (p: Partial<AppStore>) => void,
  label: string,
  fn: () => Promise<T>,
): Promise<T | null> {
  set({ aiBusy: label, error: null });
  try {
    return await fn();
  } catch (e) {
    set({ error: String(e) });
    return null;
  } finally {
    set({ aiBusy: null });
  }
}

export const useStore = create<AppStore>()((set, get) => ({
  maps: [],
  currentMapId: null,
  graph: null,
  criticality: {},
  circular: [],
  selectedNodeIds: [],
  selectedEdgeId: null,
  view: "graph",
  focusMode: false,
  suggestions: null,
  gaps: null,
  weakPoints: null,
  aiBusy: null,
  aiReady: false,
  aiChecked: false,
  aiDetail: "",
  aiVersion: null,
  error: null,

  init: async () => {
    await get().refreshAiStatus();
    await get().refreshMaps();
    const first = get().maps[0];
    if (first) await get().openMap(first.id);
  },

  refreshAiStatus: async () => {
    try {
      const s = await ipc.aiBackendStatus();
      set({ aiReady: s.ready, aiChecked: true, aiDetail: s.detail, aiVersion: s.version });
    } catch (e) {
      set({ aiChecked: true, error: String(e) });
    }
  },

  refreshMaps: async () => {
    set({ maps: await ipc.listMaps() });
  },

  openMap: async (id) => {
    set({
      currentMapId: id,
      selectedNodeIds: [],
      selectedEdgeId: null,
      suggestions: null,
      gaps: null,
      weakPoints: null,
    });
    await get().refreshGraph();
  },

  newMap: async (title) => {
    const m = await ipc.createMap(title || "Untitled");
    await get().refreshMaps();
    await get().openMap(m.id);
  },

  renameMap: async (id, title) => {
    const trimmed = title.trim();
    if (!trimmed) return;
    await ipc.renameMap(id, trimmed);
    await get().refreshMaps();
    // Keep the open graph's embedded title in sync so views reading graph.map don't go stale.
    set((s) =>
      s.graph && s.graph.map.id === id
        ? { graph: { ...s.graph, map: { ...s.graph.map, title: trimmed } } }
        : {},
    );
  },

  deleteMap: async (id) => {
    await ipc.deleteMap(id);
    await get().refreshMaps();
    if (get().currentMapId !== id) return;
    // Deleted the open map: fall back to another, or start fresh so the canvas is never orphaned.
    const next = get().maps[0];
    if (next) await get().openMap(next.id);
    else await get().newMap("Untitled");
  },

  refreshGraph: async () => {
    const id = get().currentMapId;
    if (!id) return;
    const [graph, crit, circular] = await Promise.all([
      ipc.loadGraph(id),
      ipc.analyzeMap(id),
      ipc.detectCircular(id),
    ]);
    set({ graph, criticality: indexCriticality(crit), circular });
  },

  selectNode: (id, additive) =>
    set((s) => {
      const has = s.selectedNodeIds.includes(id);
      if (additive) {
        return {
          selectedNodeIds: has
            ? s.selectedNodeIds.filter((x) => x !== id)
            : [...s.selectedNodeIds, id],
          selectedEdgeId: null,
        };
      }
      return { selectedNodeIds: [id], selectedEdgeId: null };
    }),

  selectEdge: (id) => set({ selectedEdgeId: id, selectedNodeIds: [] }),
  clearSelection: () => set({ selectedNodeIds: [], selectedEdgeId: null }),
  setView: (v) => set({ view: v }),
  toggleFocus: () => set((s) => ({ focusMode: !s.focusMode })),
  setError: (e) => set({ error: e }),

  addNode: async (text, status = "open", x = 240, y = 160) => {
    const id = get().currentMapId;
    if (!id) return null;
    const node = await ipc.createNode({ mapId: id, text, status, x, y });
    await get().refreshGraph();
    return node.id;
  },

  editNode: async (id, text) => {
    await ipc.updateNodeText(id, text);
    await get().refreshGraph();
  },

  cycleStatus: async (id) => {
    const node = get().graph?.nodes.find((n) => n.id === id);
    if (!node) return;
    const next = STATUS_CYCLE[(STATUS_CYCLE.indexOf(node.status) + 1) % STATUS_CYCLE.length];
    await get().setStatus(id, next);
  },

  setStatus: async (id, status) => {
    await ipc.setNodeStatus(id, status);
    await get().refreshGraph();
  },

  removeNode: async (id) => {
    await ipc.deleteNode(id);
    set((s) => ({ selectedNodeIds: s.selectedNodeIds.filter((x) => x !== id) }));
    await get().refreshGraph();
  },

  persistNodePosition: async (id, x, y) => {
    await ipc.moveNode(id, x, y);
    // Reflect the new position in the source-of-truth graph so a later node rebuild
    // (e.g. on selection change) doesn't snap the node back to its old spot (high.3).
    set((s) => ({
      graph: s.graph
        ? { ...s.graph, nodes: s.graph.nodes.map((n) => (n.id === id ? { ...n, x, y } : n)) }
        : s.graph,
    }));
  },

  undo: async () => {
    const id = get().currentMapId;
    if (!id) return;
    const did = await ipc.undo(id);
    if (did) await get().refreshGraph();
  },

  applyAutoLayout: async () => {
    const g = get().graph;
    if (!g) return;
    const pos = autoLayout(g.nodes, g.edges);
    await Promise.all(Object.entries(pos).map(([id, p]) => ipc.moveNode(id, p.x, p.y)));
    await get().refreshGraph();
  },

  addEdge: async (fromNode, toNode) => {
    const id = get().currentMapId;
    if (!id || fromNode === toNode) return;
    await ipc.createEdge({ mapId: id, fromNode, toNode });
    await get().refreshGraph();
  },

  removeEdge: async (id) => {
    await ipc.deleteEdge(id);
    set({ selectedEdgeId: null });
    await get().refreshGraph();
  },

  runForwardInference: async () => {
    const mapId = get().currentMapId;
    const focus = get().selectedNodeIds;
    if (!mapId || focus.length === 0) {
      set({ error: "先选中至少一个节点,再做前向推演" });
      return;
    }
    const res = await withAi(set, "forward", () => ipc.forwardInference(mapId, focus));
    if (res) set({ suggestions: { focus: [...focus], items: res } });
  },

  acceptSuggestion: async (s) => {
    const focus = get().suggestions?.focus ?? get().selectedNodeIds;
    const base = get().graph?.nodes.find((n) => n.id === focus[0]);
    const x = (base?.x ?? 240) + 60;
    const y = (base?.y ?? 160) + 180;
    const newId = await get().addNode(s.text, s.suggestedStatus, x, y);
    if (newId) {
      for (const f of focus) await ipc.createEdge({ mapId: get().currentMapId!, fromNode: f, toNode: newId });
      await ipc.setNodeOrigin(newId, "ai_accepted");
      await get().refreshGraph();
    }
    set((st) => ({
      suggestions: st.suggestions
        ? { ...st.suggestions, items: st.suggestions.items.filter((x) => x !== s) }
        : null,
    }));
  },

  dismissSuggestions: () => set({ suggestions: null }),

  runGapDetection: async () => {
    const mapId = get().currentMapId;
    const sel = get().selectedNodeIds;
    if (!mapId || sel.length !== 2) {
      set({ error: "缺口检测需要正好选中两个节点(从 → 到)" });
      return;
    }
    const [fromId, toId] = sel;
    const items = await withAi(set, "gap", () => ipc.detectGap(mapId, fromId, toId));
    if (items) set({ gaps: { fromId, toId, items } });
  },

  acceptGap: async (g) => {
    const gaps = get().gaps;
    if (!gaps) return;
    const from = get().graph?.nodes.find((n) => n.id === gaps.fromId);
    const to = get().graph?.nodes.find((n) => n.id === gaps.toId);
    const x = ((from?.x ?? 0) + (to?.x ?? 0)) / 2;
    const y = ((from?.y ?? 0) + (to?.y ?? 0)) / 2;
    const newId = await get().addNode(g.text, "open", x, y);
    if (newId) {
      const mapId = get().currentMapId!;
      await ipc.createEdge({ mapId, fromNode: gaps.fromId, toNode: newId });
      await ipc.createEdge({ mapId, fromNode: newId, toNode: gaps.toId });
      await ipc.setNodeOrigin(newId, "ai_accepted");
      await get().refreshGraph();
    }
    set((st) => ({
      gaps: st.gaps ? { ...st.gaps, items: st.gaps.items.filter((x) => x !== g) } : null,
    }));
  },

  dismissGaps: () => set({ gaps: null }),

  runChallenge: async (diverse) => {
    const mapId = get().currentMapId;
    const sel = get().selectedNodeIds;
    const edge = get().selectedEdgeId;
    if (!mapId) return;
    if (sel.length === 0 && !edge) {
      set({ error: "选中一个节点或一条边,再发起对抗" });
      return;
    }
    const targetKind = edge ? "edge" : "node";
    const targetId = edge ?? sel[0];
    const res = await withAi(set, "challenge", () =>
      ipc.generateChallenge(mapId, targetKind, targetId, diverse),
    );
    if (res) await get().refreshGraph();
  },

  runWeakPointScan: async () => {
    const mapId = get().currentMapId;
    if (!mapId) return;
    const res = await withAi(set, "weakpoints", () => ipc.scanWeakPoints(mapId));
    if (res) set({ weakPoints: res });
  },

  judge: async (id, status, note) => {
    await ipc.judgeChallenge(id, status, note);
    await get().refreshGraph();
  },

  promote: async (id) => {
    await ipc.promoteChallenge(id);
    await get().refreshGraph();
  },
}));

export function pendingChallenges(graph: MapGraph | null): Challenge[] {
  return graph?.challenges.filter((c) => c.status === "pending") ?? [];
}
