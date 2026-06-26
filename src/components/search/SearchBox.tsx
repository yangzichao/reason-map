// In-map search (SPEC §5: FTS5 + semantic, the "比传统论证软件强一档" capability). ⌘K /
// Ctrl+K focuses it. Picking a result selects the node on the canvas. Debounced so we don't
// hit the backend on every keystroke.

import { useEffect, useRef, useState } from "react";
import { useStore } from "@/state/store";

export default function SearchBox() {
  const query = useStore((s) => s.searchQuery);
  const results = useStore((s) => s.searchResults);
  const runSearch = useStore((s) => s.runSearch);
  const clearSearch = useStore((s) => s.clearSearch);
  const selectNode = useStore((s) => s.selectNode);
  const setView = useStore((s) => s.setView);

  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ⌘K / Ctrl+K focuses search from anywhere.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
        setOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const onChange = (v: string) => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void runSearch(v), 180);
    setOpen(true);
  };

  const pick = (id: string) => {
    setView("graph");
    selectNode(id, false);
    setOpen(false);
    clearSearch();
    inputRef.current?.blur();
  };

  return (
    <div className="search-box">
      <input
        ref={inputRef}
        className="search-input"
        placeholder="搜命题 (⌘K)"
        defaultValue={query}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setOpen(true)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setOpen(false);
            (e.target as HTMLInputElement).blur();
          }
        }}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && query.trim() && (
        <div className="search-results">
          {results.length === 0 ? (
            <div className="muted small search-empty">没有匹配的命题</div>
          ) : (
            results.map((n) => (
              <button key={n.id} className="search-result" onMouseDown={() => pick(n.id)}>
                {n.text}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
