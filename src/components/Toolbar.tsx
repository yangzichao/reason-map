// Top toolbar: map switching, view toggle, the AI actions (incl. the adversarial button),
// focus mode, and settings.

import { useStore } from "@/state/store";

export default function Toolbar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const maps = useStore((s) => s.maps);
  const currentMapId = useStore((s) => s.currentMapId);
  const openMap = useStore((s) => s.openMap);
  const newMap = useStore((s) => s.newMap);
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

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <select
          className="map-select"
          value={currentMapId ?? ""}
          onChange={(e) => void openMap(e.target.value)}
        >
          {maps.map((m) => (
            <option key={m.id} value={m.id}>
              {m.title}
            </option>
          ))}
        </select>
        <button className="btn ghost" title="新建图" onClick={() => void newMap("Untitled")}>
          ＋
        </button>
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
        <button className="btn" disabled={aiDisabled} onClick={() => void runWeak()}>
          扫弱点
        </button>
        {aiBusy && <span className="ai-spinner">Claude 思考中…</span>}
      </div>

      <div className="toolbar-right">
        <button className="btn ghost" onClick={onOpenSettings} title="设置 / API key">
          ⚙
        </button>
      </div>
    </div>
  );
}
