// Challenge inbox (SPEC §7.3): triage pending adversarial attacks like email, keyboard-first.
// 1 = 认(concede) · 2 = 驳(rebut) · 3 = 待定(defer). Judgment is the fastest action in the app.

import { useEffect, useRef, useState } from "react";
import { useStore } from "@/state/store";
import { pendingChallenges } from "@/state/store";
import { CHALLENGE_KIND_LABEL, type Challenge } from "@/types/domain";

export default function ChallengeInbox() {
  const graph = useStore((s) => s.graph);
  const judge = useStore((s) => s.judge);
  const promote = useStore((s) => s.promote);
  const selectNode = useStore((s) => s.selectNode);

  const pending = pendingChallenges(graph);
  const [active, setActive] = useState(0);
  const [rebutting, setRebutting] = useState(false);
  const [note, setNote] = useState("");
  const [concededId, setConcededId] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const noteRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (active >= pending.length) setActive(Math.max(0, pending.length - 1));
  }, [pending.length, active]);
  useEffect(() => {
    if (rebutting) noteRef.current?.focus();
  }, [rebutting]);
  // Focus the container so 1/2/3/j/k work without a click first (medium.5).
  useEffect(() => {
    if (pending.length && !rebutting) containerRef.current?.focus();
  }, [pending.length, rebutting]);

  const nodeText = (id: string) => graph?.nodes.find((x) => x.id === id)?.text ?? id;
  const targetText = (c: Challenge) => {
    const n = graph?.nodes.find((x) => x.id === c.targetId);
    if (n) return n.text;
    const e = graph?.edges.find((x) => x.id === c.targetId);
    if (e) return `推理: ${nodeText(e.fromNode)} → ${nodeText(e.toNode)}`;
    return c.targetId;
  };

  const current = pending[active];

  const doConcede = async (c: Challenge) => {
    await judge(c.id, "conceded");
    setConcededId(c.id);
  };
  const doRebut = async (c: Challenge) => {
    if (!note.trim()) {
      setRebutting(true);
      return;
    }
    await judge(c.id, "rebutted", note.trim());
    setNote("");
    setRebutting(false);
  };
  const doDefer = async (c: Challenge) => {
    await judge(c.id, "deferred");
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (!current || rebutting) return;
    if (e.key === "1") void doConcede(current);
    else if (e.key === "2") {
      setRebutting(true);
      e.preventDefault();
    } else if (e.key === "3") void doDefer(current);
    else if (e.key === "j") setActive((a) => Math.min(a + 1, pending.length - 1));
    else if (e.key === "k") setActive((a) => Math.max(a - 1, 0));
  };

  if (pending.length === 0) {
    return (
      <div className="inbox empty">
        <div className="muted">没有待判定的攻击。选中节点/边,按对抗键让 Claude 红队。</div>
        {concededId && (
          <PromotePrompt id={concededId} onClose={() => setConcededId(null)} promote={promote} />
        )}
      </div>
    );
  }

  return (
    <div className="inbox" ref={containerRef} tabIndex={0} onKeyDown={onKey}>
      <div className="inbox-head">
        <span>攻击收件箱 · {pending.length}</span>
        <span className="muted small">1 认 · 2 驳 · 3 待定 · j/k 切换</span>
      </div>

      {concededId && (
        <PromotePrompt id={concededId} onClose={() => setConcededId(null)} promote={promote} />
      )}

      <div className="inbox-list">
        {pending.map((c, i) => (
          <div
            key={c.id}
            className={`inbox-item ${i === active ? "active" : ""}`}
            onClick={() => {
              setActive(i);
              if (c.targetKind === "node") selectNode(c.targetId, false);
            }}
          >
            <div className="inbox-kind">{CHALLENGE_KIND_LABEL[c.kind]}</div>
            <div className="inbox-target muted small">打击: {targetText(c)}</div>
            <div className="inbox-content">{c.content}</div>
            {i === active && (
              <div className="inbox-actions">
                {rebutting ? (
                  <div className="rebut-box">
                    <textarea
                      ref={noteRef}
                      placeholder="写下你的反驳理由(它会成为资产)…"
                      value={note}
                      onChange={(e) => setNote(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) void doRebut(c);
                        if (e.key === "Escape") {
                          setRebutting(false);
                          setNote("");
                        }
                        e.stopPropagation();
                      }}
                    />
                    <div className="row">
                      <button className="btn" onClick={() => void doRebut(c)}>
                        提交反驳
                      </button>
                      <button className="btn ghost" onClick={() => setRebutting(false)}>
                        取消
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="row">
                    <button className="btn concede" onClick={() => void doConcede(c)}>
                      1 · 认
                    </button>
                    <button className="btn rebut" onClick={() => setRebutting(true)}>
                      2 · 驳
                    </button>
                    <button className="btn ghost" onClick={() => void doDefer(c)}>
                      3 · 待定
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function PromotePrompt({
  id,
  onClose,
  promote,
}: {
  id: string;
  onClose: () => void;
  promote: (id: string) => Promise<void>;
}) {
  return (
    <div className="promote-prompt">
      <span>已认。要把它晋升为图中的反驳节点吗?</span>
      <button
        className="btn primary small"
        onClick={async () => {
          await promote(id);
          onClose();
        }}
      >
        晋升
      </button>
      <button className="btn ghost small" onClick={onClose}>
        不用
      </button>
    </div>
  );
}
