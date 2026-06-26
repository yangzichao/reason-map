// Map controls in the toolbar: switch / new / rename / delete.
// Rename and delete go through small confirm modals rather than window.prompt/confirm,
// which are unreliable inside the Tauri webview.

import { useEffect, useRef, useState } from "react";
import { useStore } from "@/state/store";

type Dialog = { mode: "rename" | "delete"; id: string; title: string } | null;

export default function MapBar() {
  const maps = useStore((s) => s.maps);
  const currentMapId = useStore((s) => s.currentMapId);
  const openMap = useStore((s) => s.openMap);
  const newMap = useStore((s) => s.newMap);
  const renameMap = useStore((s) => s.renameMap);
  const deleteMap = useStore((s) => s.deleteMap);

  const [dialog, setDialog] = useState<Dialog>(null);
  const current = maps.find((m) => m.id === currentMapId) ?? null;

  return (
    <>
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
      <button
        className="btn ghost"
        title="重命名当前图"
        disabled={!current}
        onClick={() => current && setDialog({ mode: "rename", id: current.id, title: current.title })}
      >
        ✎
      </button>
      <button
        className="btn ghost"
        title="删除当前图"
        disabled={!current}
        onClick={() => current && setDialog({ mode: "delete", id: current.id, title: current.title })}
      >
        🗑
      </button>

      {dialog?.mode === "rename" && (
        <RenameDialog
          initial={dialog.title}
          onCancel={() => setDialog(null)}
          onSubmit={async (title) => {
            await renameMap(dialog.id, title);
            setDialog(null);
          }}
        />
      )}
      {dialog?.mode === "delete" && (
        <DeleteDialog
          title={dialog.title}
          onCancel={() => setDialog(null)}
          onConfirm={async () => {
            await deleteMap(dialog.id);
            setDialog(null);
          }}
        />
      )}
    </>
  );
}

function RenameDialog({
  initial,
  onSubmit,
  onCancel,
}: {
  initial: string;
  onSubmit: (title: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initial);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onEsc);
    return () => window.removeEventListener("keydown", onEsc);
  }, [onCancel]);

  const canSave = value.trim().length > 0;

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>重命名图</h3>
        <input
          ref={inputRef}
          value={value}
          placeholder="图的名字"
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && canSave && onSubmit(value)}
        />
        <div className="modal-actions">
          <div className="spacer" />
          <button className="btn ghost" onClick={onCancel}>
            取消
          </button>
          <button className="btn primary" disabled={!canSave} onClick={() => onSubmit(value)}>
            保存
          </button>
        </div>
      </div>
    </div>
  );
}

function DeleteDialog({
  title,
  onConfirm,
  onCancel,
}: {
  title: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onEsc);
    return () => window.removeEventListener("keydown", onEsc);
  }, [onCancel]);

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h3>删除图</h3>
        <p className="muted small">
          确定删除「{title || "Untitled"}」吗?其中的节点、关系与对抗记录都会一并移除。
        </p>
        <div className="modal-actions">
          <div className="spacer" />
          <button className="btn ghost" onClick={onCancel}>
            取消
          </button>
          <button className="btn attack" onClick={onConfirm}>
            删除
          </button>
        </div>
      </div>
    </div>
  );
}
