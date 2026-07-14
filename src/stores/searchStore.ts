import { create } from "zustand";
import { api } from "../services/api";
import type { SearchHistoryItem, SearchQuery, SearchResult } from "../types";

interface SearchState {
  query: SearchQuery;
  results: SearchResult[];
  history: SearchHistoryItem[];
  active: boolean;
  loading: boolean;
  setQuery: (patch: Partial<SearchQuery>) => void;
  run: () => Promise<void>;
  reset: () => void;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
}

const initialQuery: SearchQuery = { query: "", view: "all", tagIds: [], limit: 200 };

export const useSearchStore = create<SearchState>((set, get) => ({
  query: initialQuery,
  results: [],
  history: [],
  active: false,
  loading: false,
  setQuery: (patch) => set({ query: { ...get().query, ...patch } }),
  run: async () => {
    const query = get().query;
    if (!query.query.trim() && !query.categoryId && !query.tagIds?.length && !query.dateFrom && !query.dateTo && !query.isFavorite && !query.todoStatus) {
      set({ active: false, results: [] });
      return;
    }
    set({ loading: true, active: true });
    try {
      const results = await api.searchNotes(query);
      set({ results, loading: false });
      await get().loadHistory();
    } catch {
      set({ loading: false });
    }
  },
  reset: () => set({ query: initialQuery, results: [], active: false }),
  loadHistory: async () => set({ history: await api.getSearchHistory() }),
  clearHistory: async () => {
    await api.clearSearchHistory();
    set({ history: [] });
  },
}));
