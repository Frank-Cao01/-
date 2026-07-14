import { useMemo, useState } from "react";
import { CheckCircle2, FileText, Menu, Plus } from "lucide-react";
import { Highlight } from "../../components/Highlight";
import { useSearchStore } from "../../stores/searchStore";
import type { Note } from "../../types";
import { SearchBar } from "../search/SearchBar";
import { useNotesStore } from "../../stores/notesStore";

const viewTitles: Record<string, string> = {
  all: "全部笔记",
  today: "今日记录",
  todo: "待办事项",
  favorite: "收藏",
  archive: "归档",
  trash: "最近删除",
};

interface NoteListProps {
  beforeSelect: () => Promise<void>;
  onSelect: (note: Note) => void;
  onNew: () => void;
  onMenu: () => void;
}

export function NoteList({ beforeSelect, onSelect, onNew, onMenu }: NoteListProps) {
  const { notes, selected, view, categories, tags, loading } = useNotesStore();
  const search = useSearchStore();
  const [sort, setSort] = useState<"default" | "updated" | "created" | "priority" | "due">("default");
  const entries = search.active ? search.results.map((result) => ({ note: result.note, terms: result.matchedTerms })) : notes.map((note) => ({ note, terms: [] as string[] }));
  const sortedEntries = useMemo(() => {
    if (sort === "default") return entries;
    const priority = { low: 0, normal: 1, high: 2, urgent: 3 };
    return [...entries].sort((left, right) => {
      if (sort === "priority") return priority[right.note.priority] - priority[left.note.priority];
      if (sort === "due") return (left.note.dueDate ?? "9999-12-31").localeCompare(right.note.dueDate ?? "9999-12-31");
      const field = sort === "created" ? "createdAt" : "updatedAt";
      return right.note[field].localeCompare(left.note[field]);
    });
  }, [entries, sort]);

  return (
    <section className="note-list-pane">
      <header className="pane-header">
        <button className="icon-button" onClick={onMenu} title="导航"><Menu size={18} /></button>
        <h1 className="pane-title">{search.active ? `搜索结果 · ${sortedEntries.length}` : viewTitles[view] ?? "笔记"}</h1>
        <select className="sort-select" aria-label="笔记排序" value={sort} onChange={(event) => setSort(event.target.value as typeof sort)}>
          <option value="default">默认顺序</option><option value="updated">最近更新</option><option value="created">最近创建</option><option value="priority">重要程度</option><option value="due">截止日期</option>
        </select>
        <button className="icon-button" onClick={onNew} title="新建笔记"><Plus size={19} /></button>
      </header>
      <SearchBar categories={categories} tags={tags} view={view} />
      <div className="notes-scroll">
        {loading ? <div className="empty-state">正在读取本地笔记…</div> : sortedEntries.length === 0 ? (
          <div className="empty-state"><div><FileText size={32} /><p>{search.active ? "没有找到匹配内容" : "这里还没有笔记"}</p><button className="button small" onClick={onNew}><Plus size={14} />新建第一条</button></div></div>
        ) : sortedEntries.map(({ note, terms }) => (
          <button
            className={`note-card ${selected?.id === note.id ? "active" : ""}`}
            key={note.id}
            onClick={async () => { await beforeSelect(); onSelect(note); }}
          >
            <div className="note-card-title">
              {note.isTodo && note.todoStatus === "completed" && <CheckCircle2 size={15} color="var(--color-success)" />}
              <span><Highlight text={note.title || "无标题"} terms={terms} /></span>
              <span className={`priority-dot ${note.priority}`} />
            </div>
            <div className="note-card-snippet"><Highlight text={note.content.replace(/[#>*_`]/g, " ").trim() || "暂无正文"} terms={terms} /></div>
            <div className="note-card-meta">
              <span>{new Date(note.updatedAt).toLocaleDateString()}</span>
              {note.tags.slice(0, 2).map((tag) => <span className="tag-pill" key={tag.id}>#{tag.name}</span>)}
              {note.category && <span className="tag-pill">{note.category.name}</span>}
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}
