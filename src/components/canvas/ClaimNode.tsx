// Custom React Flow node = a claim. Rich, inline-editable (SPEC §7.7), with status chip,
// weak-link glow, AI-provenance marker, and an open-challenge badge (SPEC §7.4/§7.6).

import { memo, useEffect, useRef, useState } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { STATUS_META, type ClaimNode as ClaimNodeT, type NodeCriticality } from "@/types/domain";
import { useStore } from "@/state/store";

export interface ClaimNodeData {
  node: ClaimNodeT;
  criticality?: NodeCriticality;
  circular: boolean;
  dimmed: boolean;
  [key: string]: unknown;
}

function ClaimNodeView({ data, selected }: NodeProps) {
  const { node, criticality, circular, dimmed } = data as ClaimNodeData;
  const meta = STATUS_META[node.status];
  const editNode = useStore((s) => s.editNode);
  const cycleStatus = useStore((s) => s.cycleStatus);

  const [editing, setEditing] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [draft, setDraft] = useState(node.text);
  const ref = useRef<HTMLTextAreaElement>(null);

  // Long pasted text (e.g. a whole chat reply) would otherwise stretch the node into a
  // giant blob. Clamp to a few lines by default and let the user expand on demand.
  const isLong = node.text.length > 140 || node.text.split("\n").length > 4;

  useEffect(() => {
    if (editing) ref.current?.focus();
  }, [editing]);
  useEffect(() => setDraft(node.text), [node.text]);

  const commit = () => {
    setEditing(false);
    if (draft.trim() && draft !== node.text) void editNode(node.id, draft.trim());
  };

  const isWeak = criticality?.isWeakLink;
  const openCh = criticality?.openChallenges ?? 0;
  const isAi = node.origin !== "user";

  const classes = [
    "claim-node",
    selected ? "selected" : "",
    dimmed ? "dimmed" : "",
    isWeak && meta.glow ? "weak-glow" : "",
    circular ? "circular" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} style={{ borderColor: selected ? meta.color : undefined }}>
      <Handle type="target" position={Position.Top} />
      <div className="claim-head">
        <button
          className="status-chip"
          style={{ background: meta.color }}
          title="点击循环切换 status"
          onClick={(e) => {
            e.stopPropagation();
            void cycleStatus(node.id);
          }}
        >
          {meta.label}
        </button>
        {isAi && (
          <span className="ai-mark" title="AI 来源(已接受)">
            ✦
          </span>
        )}
        {openCh > 0 && (
          <span className="challenge-badge" title={`${openCh} 个未结攻击`}>
            {openCh}
          </span>
        )}
      </div>

      {editing ? (
        <textarea
          ref={ref}
          className="claim-edit"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) commit();
            if (e.key === "Escape") {
              setDraft(node.text);
              setEditing(false);
            }
            e.stopPropagation();
          }}
        />
      ) : (
        <>
          <div
            className={`claim-text${isLong && !expanded ? " clamped" : ""}`}
            onDoubleClick={() => setEditing(true)}
          >
            {node.text}
          </div>
          {isLong && (
            <button
              className="claim-toggle"
              onClick={(e) => {
                e.stopPropagation();
                setExpanded((v) => !v);
              }}
            >
              {expanded ? "收起" : "展开"}
            </button>
          )}
        </>
      )}

      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

export default memo(ClaimNodeView);
