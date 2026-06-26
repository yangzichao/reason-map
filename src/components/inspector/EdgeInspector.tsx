// Edge inspector: the one place to change what a reasoning step IS (type) and how strong
// it is (SPEC §2/§3). Without this every edge stays 'support' and the map degrades to a mind
// map. Also surfaces attacks aimed at this step (e.g. non_sequitur) and lets you delete it.

import { useEffect, useState } from "react";
import * as ipc from "@/lib/ipc";
import { useStore } from "@/state/store";
import {
  CHALLENGE_KIND_LABEL,
  EDGE_META,
  STRENGTH_LABEL,
  type Challenge,
  type EdgeType,
  type Strength,
} from "@/types/domain";

const EDGE_TYPES: EdgeType[] = ["support", "rebut", "premise_of", "depends_on"];
const STRENGTHS: Strength[] = ["strong", "weak", "tentative"];

export default function EdgeInspector() {
  const graph = useStore((s) => s.graph);
  const selectedEdgeId = useStore((s) => s.selectedEdgeId);
  const setEdgeType = useStore((s) => s.setEdgeType);
  const setEdgeStrength = useStore((s) => s.setEdgeStrength);
  const removeEdge = useStore((s) => s.removeEdge);
  const selectNode = useStore((s) => s.selectNode);

  const edge = graph?.edges.find((e) => e.id === selectedEdgeId) ?? null;

  const [history, setHistory] = useState<Challenge[]>([]);
  useEffect(() => {
    let cancelled = false;
    if (selectedEdgeId) {
      ipc.challengesForTarget(selectedEdgeId).then((h) => !cancelled && setHistory(h));
    } else {
      setHistory([]);
    }
    return () => {
      cancelled = true;
    };
  }, [selectedEdgeId, graph]);

  if (!edge) {
    return <div className="inspector empty muted">选中一条边,改它的推理性质与强度。</div>;
  }

  const nodeText = (id: string) => graph?.nodes.find((n) => n.id === id)?.text ?? id;

  return (
    <div className="inspector">
      <div className="inspector-edge-ends">
        <button className="link-node" onClick={() => selectNode(edge.fromNode, false)}>
          {nodeText(edge.fromNode)}
        </button>
        <span className="muted">↓ 这一步推理</span>
        <button className="link-node" onClick={() => selectNode(edge.toNode, false)}>
          {nodeText(edge.toNode)}
        </button>
      </div>

      <div className="field">
        <div className="field-label">推理性质</div>
        <div className="seg">
          {EDGE_TYPES.map((t) => (
            <button
              key={t}
              className={edge.edgeType === t ? "on" : ""}
              style={edge.edgeType === t ? { borderColor: EDGE_META[t].color, color: EDGE_META[t].color } : undefined}
              onClick={() => void setEdgeType(edge.id, t)}
            >
              {EDGE_META[t].label}
            </button>
          ))}
        </div>
      </div>

      <div className="field">
        <div className="field-label">强度(可选)</div>
        <div className="seg">
          <button className={!edge.strength ? "on" : ""} onClick={() => void setEdgeStrength(edge.id, null)}>
            未定
          </button>
          {STRENGTHS.map((s) => (
            <button
              key={s}
              className={edge.strength === s ? "on" : ""}
              onClick={() => void setEdgeStrength(edge.id, s)}
            >
              {STRENGTH_LABEL[s]}
            </button>
          ))}
        </div>
      </div>

      {history.length > 0 && (
        <div className="inspector-history">
          <div className="inspector-history-title">这步推理的战绩 · {history.length}</div>
          {history.map((c) => (
            <div key={c.id} className={`hist-item ${c.status}`}>
              <div className="hist-head">
                <span className="hist-kind">{CHALLENGE_KIND_LABEL[c.kind]}</span>
              </div>
              <div className="hist-content">{c.content}</div>
              {c.userNote && <div className="hist-note">↳ {c.userNote}</div>}
            </div>
          ))}
        </div>
      )}

      <div className="inspector-actions">
        <button className="btn attack small" onClick={() => void removeEdge(edge.id)}>
          删除这条边
        </button>
      </div>
    </div>
  );
}
