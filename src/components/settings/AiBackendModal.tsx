// AI backend status (SPEC §6, revised): the app drives the local `claude` CLI using the user's
// Claude Code login (subscription / OAuth). No API key is entered or stored — this modal only
// reports whether the backend is reachable and, if not, how to fix it.

import { useEffect, useState } from "react";
import { useStore } from "@/state/store";

export default function AiBackendModal({ onClose }: { onClose: () => void }) {
  const ready = useStore((s) => s.aiReady);
  const detail = useStore((s) => s.aiDetail);
  const version = useStore((s) => s.aiVersion);
  const refresh = useStore((s) => s.refreshAiStatus);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onEsc);
    return () => window.removeEventListener("keydown", onEsc);
  }, [onClose]);

  const recheck = async () => {
    setChecking(true);
    try {
      await refresh();
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>AI 后端 · 本机 Claude Code 登录态</h3>
        <p className="muted small">
          本 app 通过本机的 <code>claude</code> CLI 调用 Claude(模型 <code>claude-opus-4-8</code>),
          用的是你 Claude Code 的订阅登录,<b>不需要 API key</b>,也不读取你的 OAuth token。
        </p>

        <div className={ready ? "ok-text" : "error-text"}>
          {ready ? `✓ 已就绪${version ? ` · ${version}` : ""}` : "✗ 后端不可用"}
        </div>
        <p className="muted small">{detail}</p>

        {!ready && (
          <ol className="muted small" style={{ paddingLeft: "1.2em", lineHeight: 1.7 }}>
            <li>
              安装 Claude Code:<code>npm i -g @anthropic-ai/claude-code</code>
            </li>
            <li>
              登录订阅:在终端运行 <code>claude login</code>
            </li>
            <li>回来点「重新检测」</li>
          </ol>
        )}

        <p className="muted small">
          注意:走订阅额度(5 小时 / 周窗口),密集分析可能触发限流,届时调用会返回限流提示。
        </p>

        <div className="modal-actions">
          <button className="btn ghost" disabled={checking} onClick={() => void recheck()}>
            {checking ? "检测中…" : "重新检测"}
          </button>
          <div className="spacer" />
          <button className="btn primary" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
