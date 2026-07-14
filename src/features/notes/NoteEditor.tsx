import { useEffect, useMemo, useState } from "react";
import {
  Archive,
  ArchiveRestore,
  Check,
  CircleAlert,
  Eye,
  FilePenLine,
  Heart,
  Lightbulb,
  LoaderCircle,
  RotateCcw,
  Save,
  SlidersHorizontal,
  Sparkles,
  Trash2,
} from "lucide-react";
import { api } from "../../services/api";
import type { Category, Note, OrganizerResult, Priority, SaveStatus, TodoStatus } from "../../types";
import { MarkdownView } from "../../components/MarkdownView";
import { useNoteDraft } from "./useNoteDraft";

interface NoteEditorProps {
  note: Note | null;
  categories: Category[];
  onSaved: (note: Note) => void;
  onRemoved: (id: string) => void;
  onFlushReady?: (flush: (() => Promise<Note>) | null) => void;
  onBack?: () => void;
}

function statusLabel(status: SaveStatus) {
  switch (status) {
    case "dirty": return "待保存";
    case "saving": return "正在保存…";
    case "saved": return "已保存";
    case "error": return "保存失败";
    case "conflict": return "发现编辑冲突";
    default: return "所有更改均保存在本机";
  }
}

function StatusIcon({ status }: { status: SaveStatus }) {
  if (status === "saving") return <LoaderCircle size={13} className="animate-spin" />;
  if (status === "error" || status === "conflict") return <CircleAlert size={13} />;
  if (status === "saved") return <Check size={13} />;
  return <Save size={13} />;
}

export function NoteEditor(props: NoteEditorProps) {
  if (!props.note) {
    return (
      <section className="editor-pane">
        <div className="empty-state"><div><FilePenLine size={34} /><p>选择一条笔记，或新建内容开始记录</p></div></div>
      </section>
    );
  }
  return <NoteEditorContent {...props} note={props.note} />;
}

function NoteEditorContent({ note, categories, onSaved, onRemoved, onFlushReady, onBack }: NoteEditorProps & { note: Note }) {
  const { draft, update, flush, setComposing, status, error, conflict, loadLatest, saveAsCopy } = useNoteDraft(note, onSaved);
  const [mode, setMode] = useState<"edit" | "preview">("edit");
  const [showDetails, setShowDetails] = useState(false);
  const [analysis, setAnalysis] = useState<OrganizerResult | null>(null);
  const [analyzing, setAnalyzing] = useState(false);

  useEffect(() => {
    onFlushReady?.(flush);
    return () => onFlushReady?.(null);
  }, [flush, onFlushReady]);

  useEffect(() => {
    setAnalysis(null);
    if (status !== "saved" || draft.id.startsWith("draft:")) return;
    const timer = window.setTimeout(async () => {
      try { setAnalysis(await api.analyzeNote(draft.id)); } catch { /* 分析失败不影响保存 */ }
    }, 350);
    return () => window.clearTimeout(timer);
  }, [status, draft.id, draft.revision]);

  const tagText = useMemo(() => draft.tags.map((tag) => tag.name).join(", "), [draft.tags]);

  const runAnalysis = async () => {
    const saved = await flush();
    if (saved.id.startsWith("draft:")) return;
    setAnalyzing(true);
    try { setAnalysis(await api.analyzeNote(saved.id)); } finally { setAnalyzing(false); }
  };

  const applySuggestion = async (suggestion: OrganizerResult["suggestions"][number]) => {
    const value = suggestion.value;
    if (suggestion.kind === "due_date") update({ dueDate: String(value.dueDate) });
    if (suggestion.kind === "todo") update({ isTodo: true, todoStatus: "not_started" });
    if (suggestion.kind === "todo_status") update({ isTodo: true, todoStatus: "completed" });
    if (suggestion.kind === "tag") {
      const name = String(value.tagName);
      if (!draft.tags.some((tag) => tag.name === name)) {
        const now = new Date().toISOString();
        update({ tags: [...draft.tags, { id: `draft-tag:${name}`, name, createdAt: now, updatedAt: now }] });
      }
    }
    if (suggestion.kind === "category") {
      const categoryId = String(value.categoryId);
      update({ categoryId, category: categories.find((item) => item.id === categoryId) ?? null });
    }
    if (suggestion.kind === "archive" && !draft.id.startsWith("draft:")) {
      onSaved(await api.setNoteArchived(draft.id, true));
    }
    await api.setOrganizerSuggestionStatus(suggestion.id, "accepted");
    setAnalysis((current) => current ? { ...current, suggestions: current.suggestions.filter((item) => item.id !== suggestion.id) } : null);
  };

  const dismissSuggestion = async (id: string) => {
    await api.setOrganizerSuggestionStatus(id, "dismissed");
    setAnalysis((current) => current ? { ...current, suggestions: current.suggestions.filter((item) => item.id !== id) } : null);
  };

  const remove = async () => {
    if (draft.id.startsWith("draft:")) return onRemoved(draft.id);
    if (draft.deletedAt) {
      if (!window.confirm("永久删除后无法恢复，确定继续吗？")) return;
      await api.permanentlyDeleteNote(draft.id);
    } else {
      await flush();
      await api.softDeleteNote(draft.id);
    }
    onRemoved(draft.id);
  };

  const restore = async () => {
    await api.restoreNote(draft.id);
    onRemoved(draft.id);
  };

  const toggleArchive = async () => {
    if (draft.id.startsWith("draft:")) return;
    const saved = await flush();
    const updated = await api.setNoteArchived(saved.id, !saved.archivedAt);
    onSaved(updated);
  };

  return (
    <section className="editor-pane">
      <div className="editor-toolbar">
        {onBack && <button className="icon-button md:hidden" onClick={onBack} title="返回列表"><RotateCcw size={17} /></button>}
        <div className={`save-state ${status}`}><StatusIcon status={status} />{statusLabel(status)}</div>
        <button className={`icon-button ${draft.isFavorite ? "active" : ""}`} onClick={() => update({ isFavorite: !draft.isFavorite })} title="收藏">
          <Heart size={17} fill={draft.isFavorite ? "currentColor" : "none"} />
        </button>
        {!draft.deletedAt && !draft.id.startsWith("draft:") && (
          <button className="icon-button" onClick={toggleArchive} title={draft.archivedAt ? "取消归档" : "归档"}>
            {draft.archivedAt ? <ArchiveRestore size={17} /> : <Archive size={17} />}
          </button>
        )}
        {draft.deletedAt && <button className="button small" onClick={restore}><RotateCcw size={14} />恢复</button>}
        <button className="icon-button" onClick={remove} title={draft.deletedAt ? "永久删除" : "移至最近删除"}><Trash2 size={17} /></button>
      </div>
      <div className="editor-scroll">
        <div className="editor-form">
          {error && <div className="error-banner">{error}</div>}
          {conflict && (
            <div className="conflict-banner">
              另一窗口已修改这条笔记。为避免覆盖，请选择
              <button className="text-button" onClick={loadLatest}>加载最新内容</button>
              或 <button className="text-button" onClick={saveAsCopy}>另存为副本</button>。
            </div>
          )}
          <input
            className="title-input"
            value={draft.title}
            maxLength={500}
            placeholder="无标题"
            onChange={(event) => update({ title: event.target.value })}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
          />
          <div className="editor-tabs">
            <button className={`chip ${mode === "edit" ? "active" : ""}`} onClick={() => setMode("edit")}><FilePenLine size={13} />编辑</button>
            <button className={`chip ${mode === "preview" ? "active" : ""}`} onClick={() => setMode("preview")}><Eye size={13} />预览</button>
            <button className="chip" onClick={runAnalysis} disabled={analyzing || draft.id.startsWith("draft:")}><Sparkles size={13} />{analyzing ? "整理中" : "自动整理"}</button>
            <button className={`chip details-toggle ${showDetails ? "active" : ""}`} onClick={() => setShowDetails((visible) => !visible)} aria-expanded={showDetails}><SlidersHorizontal size={13} />{showDetails ? "收起信息" : "更多信息"}</button>
          </div>
          {showDetails && (
            <div className="editor-meta">
              <div className="field"><label>分类</label><select className="select" value={draft.categoryId ?? ""} onChange={(event) => {
                const categoryId = event.target.value || null;
                update({ categoryId, category: categories.find((item) => item.id === categoryId) ?? null });
              }}><option value="">未分类</option>{categories.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}</select></div>
              <div className="field"><label>标签（逗号分隔）</label><input className="input" value={tagText} placeholder="工作, 灵感" onChange={(event) => {
                const now = new Date().toISOString();
                update({ tags: event.target.value.split(/[,，]/).map((name) => name.trim()).filter(Boolean).map((name) => ({ id: `draft-tag:${name}`, name, createdAt: now, updatedAt: now })) });
              }} /></div>
              <div className="field"><label>重要程度</label><select className="select" value={draft.priority} onChange={(event) => update({ priority: event.target.value as Priority })}><option value="low">低</option><option value="normal">普通</option><option value="high">重要</option><option value="urgent">紧急</option></select></div>
              <div className="field"><label>截止日期</label><input className="input" type="date" value={draft.dueDate ?? ""} onChange={(event) => update({ dueDate: event.target.value || null })} /></div>
              <div className="field"><label>待办事项</label><select className="select" value={draft.isTodo ? draft.todoStatus : "none"} onChange={(event) => {
                const value = event.target.value;
                update({ isTodo: value !== "none", todoStatus: (value === "none" ? "not_started" : value) as TodoStatus });
              }}><option value="none">不是待办</option><option value="not_started">未开始</option><option value="in_progress">进行中</option><option value="completed">已完成</option></select></div>
            </div>
          )}
          {mode === "edit" ? (
            <textarea
              className="editor-body"
              value={draft.content}
              placeholder="开始记录…支持 Markdown 语法"
              onChange={(event) => update({ content: event.target.value })}
              onCompositionStart={() => setComposing(true)}
              onCompositionEnd={() => setComposing(false)}
              onKeyDown={(event) => {
                if ((event.ctrlKey || event.metaKey) && event.key === "Enter") { event.preventDefault(); void flush(); }
              }}
            />
          ) : <MarkdownView>{draft.content || "*暂无正文*"}</MarkdownView>}
          {analysis && (analysis.suggestions.length > 0 || analysis.entities.length > 0) && (
            <div className="suggestion-panel">
              <div className="suggestion-title"><Lightbulb size={16} />整理建议</div>
              {analysis.suggestions.map((suggestion) => (
                <div className="suggestion-row" key={suggestion.id}><span>{suggestion.label}</span><button className="button small primary" onClick={() => void applySuggestion(suggestion)}>采用</button><button className="button small" onClick={() => void dismissSuggestion(suggestion.id)}>忽略</button></div>
              ))}
              {analysis.entities.length > 0 && <div className="entity-list">{analysis.entities.map((entity) => <span className="chip" key={`${entity.entityType}-${entity.startOffset}`}>{entity.entityType}: {entity.value}</span>)}</div>}
            </div>
          )}
          {showDetails && !draft.id.startsWith("draft:") && <div className="editor-times"><span>创建 {new Date(draft.createdAt).toLocaleString()}</span><span>更新 {new Date(draft.updatedAt).toLocaleString()}</span></div>}
        </div>
      </div>
    </section>
  );
}
