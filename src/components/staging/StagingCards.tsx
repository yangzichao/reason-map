// AI staging (SPEC §7.2/§7.6): forward-inference suggestions and gap-detection results
// appear as ghost cards (visually distinct, dashed). Accept = enters the source of truth;
// dismiss = gone. Nothing the AI proposes mutates truth without this gesture.

import { useStore } from "@/state/store";
import { STATUS_META } from "@/types/domain";

export default function StagingCards() {
  const suggestions = useStore((s) => s.suggestions);
  const gaps = useStore((s) => s.gaps);
  const weakPoints = useStore((s) => s.weakPoints);
  const graph = useStore((s) => s.graph);
  const selectNode = useStore((s) => s.selectNode);
  const acceptSuggestion = useStore((s) => s.acceptSuggestion);
  const dismissSuggestions = useStore((s) => s.dismissSuggestions);
  const acceptGap = useStore((s) => s.acceptGap);
  const dismissGaps = useStore((s) => s.dismissGaps);
  const dismissWeakPoints = useStore((s) => s.dismissWeakPoints);

  const hasSuggestions = suggestions && suggestions.items.length > 0;
  const hasGaps = gaps && gaps.items.length > 0;
  const hasWeak = weakPoints && weakPoints.length > 0;
  if (!hasSuggestions && !hasGaps && !hasWeak) return null;

  const nodeText = (id: string) => graph?.nodes.find((n) => n.id === id)?.text ?? id;

  return (
    <div className="staging">
      {hasWeak && (
        <div className="staging-group">
          <div className="staging-title">
            扫弱点 · Claude 觉得最该补的地方
            <button className="btn ghost small" onClick={dismissWeakPoints}>
              全部忽略
            </button>
          </div>
          {weakPoints!.map((w, i) => (
            <div
              key={i}
              className="ghost-card weakpoint"
              onClick={() => selectNode(w.nodeId, false)}
              title="点击在画布上选中这个节点"
            >
              <div className="ghost-text">{nodeText(w.nodeId)}</div>
              <div className="ghost-rationale muted small">{w.reason}</div>
            </div>
          ))}
        </div>
      )}

      {hasSuggestions && (
        <div className="staging-group">
          <div className="staging-title">
            前向推演 · 候选下游
            <button className="btn ghost small" onClick={dismissSuggestions}>
              全部忽略
            </button>
          </div>
          {suggestions!.items.map((s, i) => (
            <div key={i} className="ghost-card">
              <span className="status-chip sm" style={{ background: STATUS_META[s.suggestedStatus].color }}>
                {STATUS_META[s.suggestedStatus].label}
              </span>
              <div className="ghost-text">{s.text}</div>
              {s.rationale && <div className="ghost-rationale muted small">{s.rationale}</div>}
              <button className="btn primary small" onClick={() => void acceptSuggestion(s)}>
                + 接受
              </button>
            </div>
          ))}
        </div>
      )}

      {hasGaps && (
        <div className="staging-group">
          <div className="staging-title">
            缺口检测 · 缺失的中间命题
            <button className="btn ghost small" onClick={dismissGaps}>
              全部忽略
            </button>
          </div>
          {gaps!.items.map((g, i) => (
            <div key={i} className="ghost-card">
              <div className="ghost-text">{g.text}</div>
              {g.rationale && <div className="ghost-rationale muted small">{g.rationale}</div>}
              <button className="btn primary small" onClick={() => void acceptGap(g)}>
                + 插入到中间
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
