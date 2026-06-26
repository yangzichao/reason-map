// Context-aware streaming chat (SPEC §7.9). Selected nodes feed the context (shown as
// chips). Persisted per-map history is restored on open. Assistant replies can be dropped
// onto the canvas as a node.

import { useEffect, useRef, useState } from "react";
import * as ipc from "@/lib/ipc";
import { useStore } from "@/state/store";

interface Msg {
  role: "user" | "assistant";
  content: string;
}

export default function ChatPanel() {
  const currentMapId = useStore((s) => s.currentMapId);
  const selectedNodeIds = useStore((s) => s.selectedNodeIds);
  const graph = useStore((s) => s.graph);
  const addNodeFromChat = useStore((s) => s.addNodeFromChat);
  const selectNode = useStore((s) => s.selectNode);
  const aiReady = useStore((s) => s.aiReady);

  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  // Index of the assistant message currently streaming, or null. Per-message so older
  // messages keep their "add to canvas" button during a new stream (nit.5).
  const [streamingIndex, setStreamingIndex] = useState<number | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Restore persisted history when the map changes (medium.4 / low.5).
  useEffect(() => {
    let cancelled = false;
    setMessages([]);
    if (currentMapId) {
      ipc.chatHistory(currentMapId).then((rows) => {
        if (cancelled) return;
        setMessages(
          rows
            .filter((r) => r.role === "user" || r.role === "assistant")
            .map((r) => ({ role: r.role as "user" | "assistant", content: r.content })),
        );
      });
    }
    return () => {
      cancelled = true;
    };
  }, [currentMapId]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const contextNodes = selectedNodeIds
    .map((id) => graph?.nodes.find((n) => n.id === id))
    .filter(Boolean) as { id: string; text: string }[];

  const streaming = streamingIndex !== null;

  const send = async () => {
    if (!currentMapId || !input.trim() || streaming) return;
    const userMsg = input.trim();
    setInput("");
    let assistantIdx = -1;
    setMessages((m) => {
      assistantIdx = m.length + 1;
      return [...m, { role: "user", content: userMsg }, { role: "assistant", content: "" }];
    });
    setStreamingIndex(assistantIdx);
    try {
      await ipc.chat(currentMapId, userMsg, selectedNodeIds, (ev) => {
        if (ev.type === "delta") {
          setMessages((m) => {
            const copy = [...m];
            const last = copy.length - 1;
            copy[last] = { role: "assistant", content: copy[last].content + ev.text };
            return copy;
          });
        } else if (ev.type === "error") {
          setMessages((m) => {
            const copy = [...m];
            copy[copy.length - 1] = { role: "assistant", content: `⚠️ ${ev.message}` };
            return copy;
          });
        }
      });
    } finally {
      setStreamingIndex(null);
    }
  };

  return (
    <div className="chat">
      <div className="chat-context">
        {contextNodes.length > 0 ? (
          <>
            <span className="muted">上下文:</span>
            {contextNodes.map((n) => (
              <span key={n.id} className="ctx-chip" title={n.text}>
                {n.text.length > 14 ? n.text.slice(0, 14) + "…" : n.text}
                <button
                  className="ctx-chip-x"
                  title="从上下文移除"
                  onClick={() => selectNode(n.id, true)}
                >
                  ×
                </button>
              </span>
            ))}
          </>
        ) : (
          <span className="muted">选中节点即可把它们喂给对话</span>
        )}
      </div>

      <div className="chat-log">
        {messages.length === 0 && (
          <div className="muted chat-empty">
            问问当前这张图——"这步推理跳太快了吗""帮我把这段拆成三步"。回答可加到画布。
          </div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`chat-msg ${m.role}`}>
            <div className="chat-bubble">{m.content || (i === streamingIndex ? "…" : "")}</div>
            {m.role === "assistant" && m.content && i !== streamingIndex && (
              <button
                className="chat-add"
                title={
                  contextNodes.length > 0
                    ? "作为 AI 节点加入,并连到当前上下文节点"
                    : "作为 AI 节点加入画布"
                }
                onClick={() => void addNodeFromChat(m.content)}
              >
                + 加到画布
              </button>
            )}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      <div className="chat-input">
        <textarea
          placeholder={aiReady ? "问点什么…(Enter 发送)" : "AI 后端未就绪 — 见右上角 ⚙(需登录 Claude Code)"}
          value={input}
          disabled={!aiReady}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void send();
            }
          }}
        />
        <button className="btn primary" disabled={!aiReady || streaming || !input.trim()} onClick={send}>
          发送
        </button>
      </div>
    </div>
  );
}
