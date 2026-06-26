// Stable hierarchical layout along the reasoning direction (SPEC §7.4/§7.5). Premises sit
// below, conclusions above (bottom-to-top), so the map reads with a consistent spatial grammar.

import dagre from "dagre";
import type { ClaimNode, RelationEdge } from "@/types/domain";

const NODE_W = 230;
const NODE_H = 96;

export function autoLayout(
  nodes: ClaimNode[],
  edges: RelationEdge[],
): Record<string, { x: number; y: number }> {
  const g = new dagre.graphlib.Graph();
  // BT = bottom-to-top: supports/premises flow upward into conclusions.
  g.setGraph({ rankdir: "BT", nodesep: 60, ranksep: 110, marginx: 40, marginy: 40 });
  g.setDefaultEdgeLabel(() => ({}));

  for (const n of nodes) g.setNode(n.id, { width: NODE_W, height: NODE_H });
  for (const e of edges) {
    // Rebuttals are not part of the dependency hierarchy; skip them for ranking.
    if (e.edgeType === "rebut") continue;
    g.setEdge(e.fromNode, e.toNode);
  }

  dagre.layout(g);

  const out: Record<string, { x: number; y: number }> = {};
  for (const n of nodes) {
    const pos = g.node(n.id);
    if (pos) out[n.id] = { x: pos.x - NODE_W / 2, y: pos.y - NODE_H / 2 };
  }
  return out;
}

/// Nodes within the argument neighborhood (ancestors + descendants) of the focus set.
/// Drives focus mode dimming (SPEC §7.4).
export function neighborhood(
  focus: string[],
  edges: RelationEdge[],
): Set<string> {
  const up: Record<string, string[]> = {};
  const down: Record<string, string[]> = {};
  for (const e of edges) {
    (down[e.fromNode] ||= []).push(e.toNode);
    (up[e.toNode] ||= []).push(e.fromNode);
  }
  const seen = new Set<string>(focus);
  const walk = (start: string, adj: Record<string, string[]>) => {
    const stack = [start];
    while (stack.length) {
      const cur = stack.pop()!;
      for (const nxt of adj[cur] ?? []) {
        if (!seen.has(nxt)) {
          seen.add(nxt);
          stack.push(nxt);
        }
      }
    }
  };
  for (const f of focus) {
    walk(f, up);
    walk(f, down);
  }
  return seen;
}
