// Node inspector (SPEC §7.10): structural criticality + the node's litigation history —
// what was thrown at it and how the user answered. Accessible, but doesn't clutter the canvas.

import { useEffect, useState } from "react";
import * as ipc from "@/lib/ipc";
import { useStore } from "@/state/store";
import { CHALLENGE_KIND_LABEL, STATUS_META, type Challenge } from "@/types/domain";

const VERDICT_LABEL: Record<string, string> = {
  conceded: "认了",
  rebutted: "已驳",
  deferred: "待定",
  pending: "未判",
};

export default function NodeInspector() {
  const graph = useStore((s) => s.graph);
  const selectedNodeIds = useStore((s) => s.selectedNodeIds);
  const criticality = useStore((s) => s.criticality);
  const id = selectedNodeIds.length === 1 ? selectedNodeIds[0] : null;
  const node = id ? graph?.nodes.find((n) => n.id === id) : null;
  const crit = id ? criticality[id] : undefined;

  const [history, setHistory] = useState<Challenge[]>([]);
  useEffect(() => {
    let cancelled = false;
    if (id) {
      ipc.challengesForTarget(id).then((h) => {
        if (!cancelled) setHistory(h);
      });
    } else {
      setHistory([]);
    }
    return () => {
      cancelled = true;
    };
  }, [id, graph]);

  if (!node) {
    return <div className="inspector empty muted">选中一个节点查看它的承重情况与战绩。</div>;
  }

  const meta = STATUS_META[node.status];
  return (
    <div className="inspector">
      <div className="inspector-claim">
        <span className="status-chip sm" style={{ background: meta.color }}>
          {meta.label}
        </span>
        <span>{node.text}</span>
      </div>

      <div className="inspector-stats">
        <Stat label="下游依赖" value={crit?.downstreamCount ?? 0} />
        <Stat label="承重" value={crit?.isLoadBearing ? "是" : "否"} hot={crit?.isLoadBearing} />
        <Stat label="最弱环节" value={crit?.isWeakLink ? "是" : "否"} hot={crit?.isWeakLink} />
        <Stat label="未结攻击" value={crit?.openChallenges ?? 0} hot={(crit?.openChallenges ?? 0) > 0} />
      </div>

      <div className="inspector-history">
        <div className="inspector-history-title">战绩 · {history.length}</div>
        {history.length === 0 && <div className="muted small">还没有人攻击过它。</div>}
        {history.map((c) => (
          <div key={c.id} className={`hist-item ${c.status}`}>
            <div className="hist-head">
              <span className="hist-kind">{CHALLENGE_KIND_LABEL[c.kind]}</span>
              <span className={`hist-verdict ${c.status}`}>{VERDICT_LABEL[c.status]}</span>
            </div>
            <div className="hist-content">{c.content}</div>
            {c.userNote && <div className="hist-note">↳ {c.userNote}</div>}
          </div>
        ))}
      </div>
    </div>
  );
}

function Stat({ label, value, hot }: { label: string; value: string | number; hot?: boolean }) {
  return (
    <div className={`stat ${hot ? "hot" : ""}`}>
      <div className="stat-value">{value}</div>
      <div className="stat-label">{label}</div>
    </div>
  );
}
