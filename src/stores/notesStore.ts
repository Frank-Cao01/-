import { create } from "zustand";
import { api } from "../services/api";
import { emptyNote, type Category, type Note, type Tag } from "../types";

interface NotesState {
  notes: Note[];
  selected: Note | null;
  view: string;
  categories: Category[];
  tags: Tag[];
  loading: boolean;
  error: string | null;
  loadView: (view?: string) => Promise<void>;
  loadMetadata: () => Promise<void>;
  select: (note: Note | null) => void;
  selectById: (id: string) => Promise<void>;
  newDraft: (source?: "main" | "quick") => Note;
  upsertLocal: (note: Note) => void;
  removeLocal: (id: string) => void;
}

export const useNotesStore = create<NotesState>((set, get) => ({
  notes: [],
  selected: null,
  view: "all",
  categories: [],
  tags: [],
  loading: false,
  error: null,
  loadView: async (nextView) => {
    const view = nextView ?? get().view;
    set({ loading: true, error: null, view });
    try {
      const notes = await api.listNotes(view);
      const selected = get().selected;
      const keepSelected = selected && (selected.id.startsWith("draft:") || notes.some((note) => note.id === selected.id));
      set({
        notes,
        selected: keepSelected ? selected : (notes[0] ?? null),
        loading: false,
      });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },
  loadMetadata: async () => {
    try {
      const [categories, tags] = await Promise.all([api.listCategories(), api.listTags()]);
      set({ categories, tags });
    } catch (error) {
      set({ error: String(error) });
    }
  },
  select: (selected) => set({ selected }),
  selectById: async (id) => {
    try {
      const note = await api.getNote(id);
      set({ selected: note });
      get().upsertLocal(note);
    } catch (error) {
      set({ error: String(error) });
    }
  },
  newDraft: (source = "main") => {
    const draft = emptyNote(source);
    set({ selected: draft });
    return draft;
  },
  upsertLocal: (note) => {
    const notes = get().notes;
    const exists = notes.some((item) => item.id === note.id);
    set({
      notes: exists ? notes.map((item) => (item.id === note.id ? note : item)) : [note, ...notes],
      selected: get().selected?.id === note.id || get().selected?.id.startsWith("draft:") ? note : get().selected,
    });
  },
  removeLocal: (id) => {
    const notes = get().notes.filter((item) => item.id !== id);
    set({ notes, selected: get().selected?.id === id ? (notes[0] ?? null) : get().selected });
  },
}));
