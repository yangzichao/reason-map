// Top toolbar: map switching, view toggle, the AI actions (incl. the adversarial button),
// focus mode, and settings.

import { useStore } from "@/state/store";
import BrandLogo from "./BrandLogo";
import MapBar from "./maps/MapBar";
import SearchBox from "./search/SearchBox";

export default function Toolbar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const focusMode = useStore((s) => s.focusMode);
  const toggleFocus = useStore((s) => s.toggleFocus);
  const aiBusy = useStore((s) => s.aiBusy);
  const aiReady = useStore((s) => s.aiReady);

  const runForward = useStore((s) => s.runForwardInference);
  const runGap = useStore((s) => s.runGapDetection);
  const runChallenge = useStore((s) => s.runChallenge);
  const runWeak = useStore((s) => s.runWeakPointScan);
  const undo = useStore((s) => s.undo);
  const applyAutoLayout = useStore((s) => s.applyAutoLayout);

  const selected = useStore((s) => s.selectedNodeIds);
  const selectedEdge = useStore((s) => s.selectedEdgeId);
  const hasTarget = selected.length > 0 || !!selectedEdge;

  const aiDisabled = !aiReady || !!aiBusy;

  // Tell the user WHY a button is greyed out and what to select (discoverability).
  const aiHint = !aiReady
    ? "AI 后端未就绪 — 见右上角 ⚙"
    : selected.length === 0 && !selectedEdge
      ? "选 1 个节点 → 推演 / 攻击;选 2 个 → 缺口检测;选边 → 攻这步推理"
      : selected.length === 1
        ? "可前向推演 / 对抗;再选一个做缺口检测"
        : selected.length === 2
          ? "可缺口检测(从 → 到)"
          : "";

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <div className="brand" title="reason·map">
          <span className="brand-mark" style={{ color: "var(--accent)" }}>
            <BrandLogo size={22} />
          </span>
          <span className="brand-name">reason·map</span>
        </div>
        <MapBar />
        <SearchBox />
        <div className="view-toggle">
          <button className={view === "graph" ? "on" : ""} onClick={() => setView("graph")}>
            画布
          </button>
          <button className={view === "outline" ? "on" : ""} onClick={() => setView("outline")}>
            文本
          </button>
        </div>
        <button className={`btn ghost ${focusMode ? "on" : ""}`} onClick={toggleFocus} title="只看选中节点的论证邻域">
          聚焦
        </button>
        <button className="btn ghost" onClick={() => void applyAutoLayout()} title="按推理方向自动整理布局">
          整理
        </button>
        <button className="btn ghost" onClick={() => void undo()} title="撤销 (⌘Z)">
          撤销
        </button>
      </div>

      <div className="toolbar-ai">
        <button className="btn" disabled={aiDisabled || selected.length === 0} onClick={() => void runForward()}>
          前向推演
        </button>
        <button className="btn" disabled={aiDisabled || selected.length !== 2} onClick={() => void runGap()}>
          缺口检测
        </button>
        <button
          className="btn attack"
          disabled={aiDisabled || !hasTarget}
          title="让 Claude 红队攻击选中的节点/边"
          onClick={() => void runChallenge(false)}
        >
          ⚔ 对抗
        </button>
        <button
          className="btn attack"
          disabled={aiDisabled || !hasTarget}
          title="多个独立视角同时攻击"
          onClick={() => void runChallenge(true)}
        >
          多视角
        </button>
        <button className="btn" disabled={aiDisabled} onClick={() => void runWeak()} title="让 Claude 扫整张图最该补的弱点">
          扫弱点
        </button>
        {aiBusy ? (
          <span className="ai-spinner">Claude 思考中…</span>
        ) : (
          aiHint && <span className="ai-hint muted small">{aiHint}</span>
        )}
      </div>

      <div className="toolbar-right">
        <button className="btn ghost" onClick={onOpenSettings} title="设置 / 本机 Claude Code 登录态">
          ⚙
        </button>
      </div>
    </div>
  );
}
