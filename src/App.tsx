// App shell: toolbar on top, canvas/outline in the center, a right rail with the adversarial
// inbox + AI staging, the node inspector, and chat. Error banner is non-blocking (SPEC §7:
// errors must not interrupt flow).

import { useEffect, useState } from "react";
import Toolbar from "@/components/Toolbar";
import GraphCanvas from "@/components/canvas/GraphCanvas";
import OutlineEditor from "@/components/outline/OutlineEditor";
import ChatPanel from "@/components/chat/ChatPanel";
import ChallengeInbox from "@/components/challenges/ChallengeInbox";
import NodeInspector from "@/components/inspector/NodeInspector";
import StagingCards from "@/components/staging/StagingCards";
import AiBackendModal from "@/components/settings/AiBackendModal";
import { useStore } from "@/state/store";

type Tab = "inbox" | "inspector" | "chat";

export default function App() {
  const init = useStore((s) => s.init);
  const view = useStore((s) => s.view);
  const aiReady = useStore((s) => s.aiReady);
  const aiChecked = useStore((s) => s.aiChecked);
  const error = useStore((s) => s.error);
  const setError = useStore((s) => s.setError);
  const [tab, setTab] = useState<Tab>("inbox");
  const [settingsOpen, setSettingsOpen] = useState(false);

  const undo = useStore((s) => s.undo);

  useEffect(() => {
    void init();
  }, [init]);

  // Global undo (⌘/Ctrl+Z), unless typing in a field.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      const typing = t && (t.tagName === "TEXTAREA" || t.tagName === "INPUT");
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z" && !typing) {
        e.preventDefault();
        void undo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo]);

  // First-run: only after we've actually probed the backend and found it not ready, open the
  // modal (so a logged-in user isn't nagged on every launch). SPEC §7.7.
  useEffect(() => {
    if (aiChecked && !aiReady) setSettingsOpen(true);
  }, [aiChecked, aiReady]);

  return (
    <div className="app">
      <Toolbar onOpenSettings={() => setSettingsOpen(true)} />

      <div className="body">
        <main className="center">
          {view === "graph" ? <GraphCanvas /> : <OutlineEditor />}
          {error && (
            <div className="error-banner" onClick={() => setError(null)}>
              {error} <span className="muted small">(点击关闭)</span>
            </div>
          )}
        </main>

        <aside className="right">
          <div className="tabs">
            <button className={tab === "inbox" ? "on" : ""} onClick={() => setTab("inbox")}>
              对抗
            </button>
            <button className={tab === "inspector" ? "on" : ""} onClick={() => setTab("inspector")}>
              详情
            </button>
            <button className={tab === "chat" ? "on" : ""} onClick={() => setTab("chat")}>
              对话
            </button>
          </div>
          <div className="tab-body">
            {tab === "inbox" && (
              <>
                <StagingCards />
                <ChallengeInbox />
              </>
            )}
            {tab === "inspector" && <NodeInspector />}
            {tab === "chat" && <ChatPanel />}
          </div>
        </aside>
      </div>

      {settingsOpen && <AiBackendModal onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
