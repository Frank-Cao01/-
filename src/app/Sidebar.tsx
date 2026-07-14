import {
  Archive,
  CalendarDays,
  CheckSquare,
  FileText,
  Folder,
  Heart,
  Plus,
  Settings,
  Tag as TagIcon,
  Trash2,
  Zap,
} from "lucide-react";
import { api } from "../services/api";
import { PRODUCT_NAME } from "../config/app";
import { useNotesStore } from "../stores/notesStore";
import { useSearchStore } from "../stores/searchStore";
import { useUiStore } from "../stores/uiStore";

const systemViews = [
  ["all", "全部笔记", FileText],
  ["today", "今日记录", CalendarDays],
  ["todo", "待办事项", CheckSquare],
  ["favorite", "收藏", Heart],
  ["archive", "归档", Archive],
  ["trash", "最近删除", Trash2],
] as const;

export function Sidebar({ beforeNavigate }: { beforeNavigate: () => Promise<void> }) {
  const notes = useNotesStore();
  const ui = useUiStore();
  const search = useSearchStore();

  const changeView = async (view: string) => {
    await beforeNavigate();
    search.reset();
    ui.setPanel("notes");
    ui.setSidebarOpen(false);
    await notes.loadView(view);
  };

  const addCategory = async () => {
    const name = window.prompt("分类名称");
    if (!name?.trim()) return;
    await api.createCategory(name.trim());
    await notes.loadMetadata();
  };

  const filterCategory = async (id: string) => {
    await beforeNavigate();
    ui.setPanel("notes");
    ui.setSidebarOpen(false);
    search.setQuery({ query: "", categoryId: id, view: "all" });
    await search.run();
  };

  const removeCategory = async (id: string, name: string) => {
    if (!window.confirm(`删除分类“${name}”后，其中的笔记会回到未分类。确定继续吗？`)) return;
    await beforeNavigate();
    await api.deleteCategory(id);
    search.reset();
    await Promise.all([notes.loadMetadata(), notes.loadView("all")]);
  };

  const removeTag = async (id: string, name: string) => {
    if (!window.confirm(`删除标签“${name}”会从所有笔记中移除该标签。确定继续吗？`)) return;
    await beforeNavigate();
    await api.deleteTag(id);
    search.reset();
    await Promise.all([notes.loadMetadata(), notes.loadView("all")]);
  };

  return (
    <aside className={`sidebar ${ui.sidebarOpen ? "open" : ""}`}>
      <div className="brand"><span className="brand-mark"><Zap size={17} fill="currentColor" /></span><span>{PRODUCT_NAME}</span></div>
      <nav className="sidebar-section">
        {systemViews.map(([id, label, Icon]) => <button className={`nav-item ${ui.panel === "notes" && notes.view === id && !search.active ? "active" : ""}`} key={id} onClick={() => void changeView(id)}><Icon size={16} /><span>{label}</span></button>)}
      </nav>
      <div className="sidebar-section">
        <div className="sidebar-label">分类 <button className="text-button" style={{ float: "right" }} onClick={addCategory} title="新建分类"><Plus size={13} /></button></div>
        {notes.categories.length === 0 ? <span className="nav-item">暂无分类</span> : notes.categories.map((category) => <div className="sidebar-entry" key={category.id}><button className={`nav-item ${search.query.categoryId === category.id ? "active" : ""}`} onClick={() => void filterCategory(category.id)}><Folder size={15} /><span>{category.name}</span></button><button className="icon-button nav-delete" onClick={() => void removeCategory(category.id, category.name)} title={`删除分类 ${category.name}`}><Trash2 size={13} /></button></div>)}
      </div>
      <div className="sidebar-section">
        <div className="sidebar-label">标签</div>
        {notes.tags.slice(0, 8).map((tag) => <div className="sidebar-entry" key={tag.id}><button className="nav-item" onClick={() => void (async () => { await beforeNavigate(); ui.setPanel("notes"); ui.setSidebarOpen(false); search.setQuery({ query: "", tagIds: [tag.id], view: "all" }); await useSearchStore.getState().run(); })()}><TagIcon size={14} /><span>{tag.name}</span></button><button className="icon-button nav-delete" onClick={() => void removeTag(tag.id, tag.name)} title={`删除标签 ${tag.name}`}><Trash2 size={13} /></button></div>)}
      </div>
      <div className="sidebar-footer"><button className={`nav-item ${ui.panel === "settings" ? "active" : ""}`} onClick={() => void (async () => { await beforeNavigate(); ui.setPanel("settings"); ui.setSidebarOpen(false); })()}><Settings size={16} />设置</button></div>
    </aside>
  );
}
