// Node inspector (SPEC §7.10 + §2 + §4.1): structural criticality, evidence, and the node's
// litigation history — what was thrown at it and how the user answered. Also the calm place
// to edit the claim, change its status, attach evidence, attack it, or delete it — so the
// canvas stays uncluttered.

import { useEffect, useState } from "react";
import * as ipc from "@/lib/ipc";
import { useStore } from "@/state/store";
import {
  CHALLENGE_KIND_LABEL,
  EVIDENCE_KIND_LABEL,
  ORIGIN_LABEL,
  STATUS_META,
  type Challenge,
  type Evidence,
  type EvidenceKind,
} from "@/types/domain";

const VERDICT_LABEL: Record<string, string> = {
  conceded: "认了",
  rebutted: "已驳",
  deferred: "待定",
  pending: "未判",
};

const EVIDENCE_KINDS: EvidenceKind[] = ["url", "quote", "data", "file"];

function evidenceValue(payload: string): string {
  try {
    const v = JSON.parse(payload);
    return typeof v?.value === "string" ? v.value : payload;
  } catch {
    return payload;
  }
}

export default function NodeInspector() {
  const graph = useStore((s) => s.graph);
  const selectedNodeIds = useStore((s) => s.selectedNodeIds);
  const criticality = useStore((s) => s.criticality);
  const aiReady = useStore((s) => s.aiReady);
  const aiBusy = useStore((s) => s.aiBusy);
  const editNode = useStore((s) => s.editNode);
  const cycleStatus = useStore((s) => s.cycleStatus);
  const removeNode = useStore((s) => s.removeNode);
  const runChallenge = useStore((s) => s.runChallenge);

  const id = selectedNodeIds.length === 1 ? selectedNodeIds[0] : null;
  const node = id ? graph?.nodes.find((n) => n.id === id) : null;
  const crit = id ? criticality[id] : undefined;

  const [history, setHistory] = useState<Challenge[]>([]);
  const [evidence, setEvidence] = useState<Evidence[]>([]);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [evKind, setEvKind] = useState<EvidenceKind>("url");
  const [evValue, setEvValue] = useState("");

  const reloadEvidence = (nodeId: string) => ipc.listEvidence(nodeId).then(setEvidence);

  useEffect(() => {
    let cancelled = false;
    if (id) {
      ipc.challengesForTarget(id).then((h) => !cancelled && setHistory(h));
      ipc.listEvidence(id).then((e) => !cancelled && setEvidence(e));
    } else {
      setHistory([]);
      setEvidence([]);
    }
    setEditing(false);
    return () => {
      cancelled = true;
    };
  }, [id, graph]);

  if (!node) {
    if (selectedNodeIds.length > 1) {
      return <div className="inspector empty muted">选中了 {selectedNodeIds.length} 个节点。单选一个看它的承重情况与战绩。</div>;
    }
    return <div className="inspector empty muted">选中一个节点查看它的承重情况与战绩。</div>;
  }

  const meta = STATUS_META[node.status];
  const aiDisabled = !aiReady || !!aiBusy;

  const addEv = async () => {
    if (!evValue.trim()) return;
    await ipc.addEvidence(node.id, evKind, JSON.stringify({ value: evValue.trim() }));
    setEvValue("");
    await reloadEvidence(node.id);
  };

  return (
    <div className="inspector">
      <div className="inspector-claim-head">
        <button
          className="status-chip sm"
          style={{ background: meta.color }}
          title="点击循环切换 status"
          onClick={() => void cycleStatus(node.id)}
        >
          {meta.label}
        </button>
        <span className="muted small">{ORIGIN_LABEL[node.origin]}</span>
      </div>

      {editing ? (
        <textarea
          className="claim-edit inspector-edit"
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => {
            if (draft.trim() && draft.trim() !== node.text) void editNode(node.id, draft.trim());
            setEditing(false);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) (e.target as HTMLTextAreaElement).blur();
            if (e.key === "Escape") setEditing(false);
          }}
        />
      ) : (
        <div
          className="inspector-claim-text"
          title="双击编辑"
          onDoubleClick={() => {
            setDraft(node.text);
            setEditing(true);
          }}
        >
          {node.text}
        </div>
      )}

      <div className="inspector-stats">
        <Stat label="下游依赖" value={crit?.downstreamCount ?? 0} />
        <Stat label="承重" value={crit?.isLoadBearing ? "是" : "否"} hot={crit?.isLoadBearing} />
        <Stat label="最弱环节" value={crit?.isWeakLink ? "是" : "否"} hot={crit?.isWeakLink} />
        <Stat label="未结攻击" value={crit?.openChallenges ?? 0} hot={(crit?.openChallenges ?? 0) > 0} />
      </div>

      <div className="inspector-actions">
        <button
          className="btn attack small"
          disabled={aiDisabled}
          title="让 Claude 红队攻击这个命题"
          onClick={() => void runChallenge(false)}
        >
          ⚔ 攻它
        </button>
        <button className="btn ghost small" onClick={() => void removeNode(node.id)}>
          删除节点
        </button>
      </div>

      <div className="inspector-evidence">
        <div className="inspector-history-title">证据 · {evidence.length}</div>
        {evidence.map((e) => (
          <div key={e.id} className="ev-item">
            <span className="ev-kind">{EVIDENCE_KIND_LABEL[e.kind]}</span>
            <span className="ev-value">{evidenceValue(e.payload)}</span>
            <button
              className="ev-del"
              title="删除"
              onClick={async () => {
                await ipc.deleteEvidence(e.id);
                await reloadEvidence(node.id);
              }}
            >
              ×
            </button>
          </div>
        ))}
        <div className="ev-add">
          <select value={evKind} onChange={(e) => setEvKind(e.target.value as EvidenceKind)}>
            {EVIDENCE_KINDS.map((k) => (
              <option key={k} value={k}>
                {EVIDENCE_KIND_LABEL[k]}
              </option>
            ))}
          </select>
          <input
            value={evValue}
            placeholder="链接 / 引文 / 数据…"
            onChange={(e) => setEvValue(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void addEv()}
          />
          <button className="btn ghost small" disabled={!evValue.trim()} onClick={() => void addEv()}>
            挂上
          </button>
        </div>
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
            <div className="hist-time muted small">
              {c.createdAt.slice(0, 10)}
              {c.resolvedAt ? ` · 判定 ${c.resolvedAt.slice(0, 10)}` : ""}
            </div>
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
