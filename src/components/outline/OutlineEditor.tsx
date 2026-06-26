// Text-first capture (SPEC §7.1: the canvas is not the primary input). Brain-dump an
// indented outline; indentation = a support edge from child up to its parent claim.
// This is the fast "倒进来" path; the canvas is for refining and seeing.

import { useMemo, useState } from "react";
import * as ipc from "@/lib/ipc";
import { useStore } from "@/state/store";
import { STATUS_META, type NodeStatus } from "@/types/domain";

const PREFIX: Record<string, NodeStatus> = {
  "!": "bet",
  "~": "assumption",
  "=": "fact",
  "?": "open",
  "+": "evidenced",
};

function indentOf(line: string): number {
  const m = line.match(/^[\t ]*/);
  if (!m) return 0;
  return m[0].replace(/\t/g, "  ").length >> 1; // 2 spaces per level
}

export default function OutlineEditor() {
  const graph = useStore((s) => s.graph);
  const currentMapId = useStore((s) => s.currentMapId);
  const refreshGraph = useStore((s) => s.refreshGraph);
  const setView = useStore((s) => s.setView);
  const setError = useStore((s) => s.setError);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  // Read-only outline of what's already on the canvas, for orientation.
  const existing = useMemo(() => {
    if (!graph) return [];
    return graph.nodes.map((n) => `${STATUS_META[n.status].label} · ${n.text}`);
  }, [graph]);

  const apply = async () => {
    if (!currentMapId || !text.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const lines = text.split("\n");
      const stack: { level: number; id: string }[] = [];
      let row = 0;
      for (const raw of lines) {
        if (!raw.trim()) continue;
        const level = indentOf(raw);
        let body = raw.trim();
        let status: NodeStatus = "open";
        const p = body[0];
        if (p && PREFIX[p]) {
          status = PREFIX[p];
          body = body.slice(1).trim();
        }
        if (!body) continue;
        const node = await ipc.createNode({
          mapId: currentMapId,
          text: body,
          status,
          x: 120 + level * 60,
          y: 80 + row * 110,
        });
        row += 1;
        while (stack.length && stack[stack.length - 1].level >= level) stack.pop();
        const parent = stack[stack.length - 1];
        if (parent) {
          await ipc.createEdge({
            mapId: currentMapId,
            fromNode: node.id,
            toNode: parent.id,
            edgeType: "support",
          });
        }
        stack.push({ level, id: node.id });
      }
      setText("");
      await refreshGraph();
      setView("graph");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="outline">
      <div className="outline-capture">
        <div className="outline-hint">
          一行一个命题 · 缩进(两空格)= 该行支持上一层 · 行首 <code>!</code>赌 <code>~</code>假设{" "}
          <code>=</code>事实 <code>?</code>开放 <code>+</code>证据
        </div>
        <textarea
          className="outline-text"
          placeholder={"我应该投入做 reason-map\n  !本地+Claude 能做出差异化\n  ~现有工具没把对抗做进去"}
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <div className="outline-actions">
          <button className="btn primary" disabled={busy || !text.trim()} onClick={apply}>
            {busy ? "添加中…" : "添加到图 →"}
          </button>
        </div>
      </div>
      <div className="outline-existing">
        <div className="outline-existing-title">当前图中的命题</div>
        {existing.length === 0 && <div className="muted">还没有节点</div>}
        <ul>
          {existing.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
