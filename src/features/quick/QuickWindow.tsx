import { useCallback, useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { Expand, LoaderCircle, Pin, PinOff, Plus, Save, X, Zap } from "lucide-react";
import { api } from "../../services/api";
import { useSettingsStore } from "../../stores/settingsStore";
import { emptyNote, type Note, type Priority, type TodoStatus } from "../../types";
import { useNoteDraft } from "../notes/useNoteDraft";

export function QuickWindow() {
  const [note, setNote] = useState<Note>(() => emptyNote("quick"));
  const [ready, setReady] = useState(false);
  useEffect(() => {
    void (async () => {
      const settings = await useSettingsStore.getState().load();
      if (settings.quickActiveNoteId) {
        try { setNote(await api.getNote(settings.quickActiveNoteId)); } catch { setNote(emptyNote("quick")); }
      }
      setReady(true);
    })();
  }, []);

  if (!ready) return <div className="quick-shell"><div className="quick-card"><div className="empty-state"><LoaderCircle className="animate-spin" />正在准备本地草稿…</div></div></div>;
  return <QuickEditor key={note.id} note={note} setNote={setNote} />;
}

function QuickEditor({ note, setNote }: { note: Note; setNote: (note: Note) => void }) {
  const settingsStore = useSettingsStore();
  const contentRef = useRef<HTMLTextAreaElement>(null);
  const hidingRef = useRef(false);
  const { draft, update, flush, setComposing, status, error } = useNoteDraft(note, async (saved) => {
    setNote(saved);
    if (settingsStore.settings.quickActiveNoteId !== saved.id) {
      try { await settingsStore.save({ ...settingsStore.settings, quickActiveNoteId: saved.id }); } catch { /* 保存设置失败不影响笔记 */ }
    }
  });

  const hideAfterSave = useCallback(async () => {
    if (hidingRef.current) return;
    hidingRef.current = true;
    try {
      const saved = await flush();
      if (!saved.id.startsWith("draft:") && settingsStore.settings.quickActiveNoteId !== saved.id) {
        await settingsStore.save({ ...settingsStore.settings, quickActiveNoteId: saved.id });
      }
      await api.hideQuickWindow();
    } catch {
      contentRef.current?.focus();
    } finally {
      hidingRef.current = false;
    }
  }, [flush, settingsStore]);

  const createNew = useCallback(async () => {
    try { await flush(); } catch { return; }
    const next = emptyNote("quick");
    setNote(next);
    try { await settingsStore.save({ ...settingsStore.settings, quickActiveNoteId: null }); } catch { /* 已保留当前笔记 */ }
    window.setTimeout(() => contentRef.current?.focus(), 30);
  }, [flush, setNote, settingsStore]);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    void listen("quick:toggle-request", () => void hideAfterSave()).then((unlisten) => unlisteners.push(unlisten));
    void listen("quick:new", () => void createNew()).then((unlisten) => unlisteners.push(unlisten));
    void listen("quick:focus-editor", () => window.setTimeout(() => contentRef.current?.focus(), 30)).then((unlisten) => unlisteners.push(unlisten));
    void listen("app:quit-request", () => void (async () => {
      try {
        await flush();
        await emit("quick:quit-ready");
      } catch {
        await api.showQuickWindow(false);
        contentRef.current?.focus();
        if (window.confirm("快速记录保存失败。仍然退出可能丢失未保存内容，确定继续吗？")) {
          await emit("quick:quit-ready");
        }
      }
    })()).then((unlisten) => unlisteners.push(unlisten));
    return () => unlisteners.forEach((unlisten) => unlisten());
  }, [hideAfterSave, createNew, flush]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.isComposing) { event.preventDefault(); void hideAfterSave(); }
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") { event.preventDefault(); void flush(); }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [flush, hideAfterSave]);

  const tagText = draft.tags.map((tag) => tag.name).join(", ");
  const openMain = async () => {
    const saved = await flush();
    await api.showMainWindow(saved.id.startsWith("draft:") ? null : saved.id);
    await api.hideQuickWindow();
  };

  const togglePinned = async () => {
    const quickPinned = !settingsStore.settings.quickPinned;
    try {
      await settingsStore.save({ ...settingsStore.settings, quickPinned });
    } catch {
      contentRef.current?.focus();
    }
  };

  return (
    <div className="quick-shell">
      <div className="quick-card">
        <header className="quick-header" data-tauri-drag-region title="按住此处可移动窗口">
          <Zap size={15} color="var(--color-accent)" fill="currentColor" />
          <span className="quick-title" data-tauri-drag-region>快速记录</span>
          <button
            className={`icon-button ${settingsStore.settings.quickPinned ? "active" : ""}`}
            onClick={togglePinned}
            title={settingsStore.settings.quickPinned ? "取消固定窗口" : "固定窗口位置"}
            aria-pressed={settingsStore.settings.quickPinned}
          >
            {settingsStore.settings.quickPinned ? <PinOff size={16} /> : <Pin size={16} />}
          </button>
          <button className="icon-button" onClick={createNew} title="快速新建"><Plus size={17} /></button>
          <button className="icon-button" onClick={openMain} title="展开主界面"><Expand size={16} /></button>
          <button className="icon-button" onClick={hideAfterSave} title="保存并隐藏"><X size={17} /></button>
        </header>
        <div className="quick-form">
          {error && <div className="error-banner">保存失败，窗口已保持打开：{error}</div>}
          <input
            className="quick-title-input"
            value={draft.title}
            placeholder="标题（可选）"
            onChange={(event) => update({ title: event.target.value })}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
          />
          <textarea
            ref={contentRef}
            className="quick-content"
            value={draft.content}
            placeholder="记下此刻的想法…"
            onChange={(event) => update({ content: event.target.value })}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
          />
          <div className="quick-meta">
            <div className="field"><label>标签</label><input className="input" value={tagText} placeholder="工作, 临时" onChange={(event) => {
              const now = new Date().toISOString();
              update({ tags: event.target.value.split(/[,，]/).map((name) => name.trim()).filter(Boolean).map((name) => ({ id: `draft-tag:${name}`, name, createdAt: now, updatedAt: now })) });
            }} /></div>
            <div className="field"><label>重要程度</label><select className="select" value={draft.priority} onChange={(event) => update({ priority: event.target.value as Priority })}><option value="low">低</option><option value="normal">普通</option><option value="high">重要</option><option value="urgent">紧急</option></select></div>
            <div className="field"><label>待办</label><select className="select" value={draft.isTodo ? draft.todoStatus : "none"} onChange={(event) => {
              const value = event.target.value;
              update({ isTodo: value !== "none", todoStatus: (value === "none" ? "not_started" : value) as TodoStatus });
            }}><option value="none">普通笔记</option><option value="not_started">未开始</option><option value="in_progress">进行中</option><option value="completed">已完成</option></select></div>
          </div>
          <footer className="quick-footer">
            <span>{status === "saving" ? "正在保存…" : status === "saved" ? "已保存到本机" : status === "dirty" ? "等待自动保存" : "Ctrl + Enter 保存"}</span>
            <button className="button small" onClick={openMain}><Expand size={14} />主界面</button>
            <button className="button small primary" onClick={() => void flush()}><Save size={14} />保存</button>
          </footer>
        </div>
      </div>
    </div>
  );
}
