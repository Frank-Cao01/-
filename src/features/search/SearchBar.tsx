import { useEffect, useState } from "react";
import { Heart, Search, X } from "lucide-react";
import { useSearchStore } from "../../stores/searchStore";
import type { Category, SearchQuery, Tag } from "../../types";

export function SearchBar({ categories, tags, view }: { categories: Category[]; tags: Tag[]; view: string }) {
  const { query, setQuery, run, reset, history, loadHistory, clearHistory, loading } = useSearchStore();
  const [focused, setFocused] = useState(false);

  useEffect(() => { void loadHistory(); }, [loadHistory]);
  useEffect(() => { setQuery({ view }); }, [view, setQuery]);
  useEffect(() => {
    if (!query.query.trim()) return;
    const timer = window.setTimeout(() => void run(), 260);
    return () => window.clearTimeout(timer);
  }, [query.query, run]);

  const runWith = (patch: Partial<SearchQuery>) => {
    setQuery(patch);
    window.setTimeout(() => void useSearchStore.getState().run(), 0);
  };

  return (
    <div className="search-wrap">
      <div className="search-input-wrap">
        <Search size={15} />
        <input
          className="search-input"
          value={query.query}
          placeholder="搜索笔记、标签、日期…"
          aria-label="搜索笔记"
          onFocus={() => setFocused(true)}
          onBlur={() => window.setTimeout(() => setFocused(false), 120)}
          onChange={(event) => setQuery({ query: event.target.value })}
          onKeyDown={(event) => { if (event.key === "Enter") void run(); if (event.key === "Escape") reset(); }}
        />
        {(query.query || query.categoryId || query.tagIds?.length || query.dateFrom || query.dateTo || query.isFavorite || query.todoStatus) && <button className="text-button" onClick={reset} title="清除搜索"><X size={14} /></button>}
      </div>
      <div className="search-filters">
        <button className={`chip ${query.isFavorite ? "active" : ""}`} onClick={() => runWith({ isFavorite: query.isFavorite ? null : true })}><Heart size={12} />收藏</button>
        <select className="chip" aria-label="待办状态" value={query.todoStatus ?? ""} onChange={(event) => runWith({ todoStatus: (event.target.value || null) as SearchQuery["todoStatus"] })}>
          <option value="">全部状态</option><option value="not_started">未开始</option><option value="in_progress">进行中</option><option value="completed">已完成</option>
        </select>
        <select className="chip" aria-label="分类筛选" value={query.categoryId ?? ""} onChange={(event) => runWith({ categoryId: event.target.value || null })}>
          <option value="">全部分类</option>{categories.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}
        </select>
        <select className="chip" aria-label="标签筛选" value={query.tagIds?.[0] ?? ""} onChange={(event) => runWith({ tagIds: event.target.value ? [event.target.value] : [] })}>
          <option value="">全部标签</option>{tags.map((tag) => <option key={tag.id} value={tag.id}>{tag.name}</option>)}
        </select>
        <input className="chip date-filter" aria-label="开始日期" title="开始日期" type="date" value={query.dateFrom ?? ""} onChange={(event) => runWith({ dateFrom: event.target.value || null })} />
        <input className="chip date-filter" aria-label="结束日期" title="结束日期" type="date" value={query.dateTo ?? ""} onChange={(event) => runWith({ dateTo: event.target.value || null })} />
        {loading && <span className="chip">检索中…</span>}
      </div>
      {focused && !query.query && history.length > 0 && (
        <div className="history-popover">
          <div className="history-head"><span>最近搜索</span><button className="text-button" onMouseDown={(event) => event.preventDefault()} onClick={() => void clearHistory()}>清除</button></div>
          {history.slice(0, 6).map((item) => <button className="nav-item" key={item.id} onMouseDown={(event) => event.preventDefault()} onClick={() => {
            try { setQuery(JSON.parse(item.filtersJson) as SearchQuery); } catch { setQuery({ query: item.query }); }
            window.setTimeout(() => void useSearchStore.getState().run(), 0);
          }}>{item.query}</button>)}
        </div>
      )}
    </div>
  );
}
