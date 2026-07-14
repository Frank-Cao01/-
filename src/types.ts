export type ThemeMode = "light" | "dark" | "system";
export type TodoStatus = "not_started" | "in_progress" | "completed";
export type Priority = "low" | "normal" | "high" | "urgent";
export type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error" | "conflict";

export interface Category {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface Tag {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface Note {
  id: string;
  title: string;
  content: string;
  categoryId: string | null;
  category: Category | null;
  tags: Tag[];
  isFavorite: boolean;
  isTodo: boolean;
  todoStatus: TodoStatus;
  priority: Priority;
  dueDate: string | null;
  completedAt: string | null;
  source: "main" | "quick" | "import";
  revision: number;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  archivedAt: string | null;
}

export interface SaveNoteInput {
  id: string | null;
  expectedRevision: number | null;
  title: string;
  content: string;
  categoryId: string | null;
  tagNames: string[];
  isFavorite: boolean;
  isTodo: boolean;
  todoStatus: TodoStatus;
  priority: Priority;
  dueDate: string | null;
  source: "main" | "quick" | "import";
}

export interface SaveNoteResponse {
  status: "created" | "saved" | "conflict";
  note: Note;
}

export interface SearchQuery {
  query: string;
  view?: string | null;
  categoryId?: string | null;
  tagIds?: string[];
  dateFrom?: string | null;
  dateTo?: string | null;
  isFavorite?: boolean | null;
  todoStatus?: TodoStatus | null;
  limit?: number;
}

export interface SearchResult {
  note: Note;
  matchedTerms: string[];
}

export interface SearchHistoryItem {
  id: string;
  query: string;
  filtersJson: string;
  searchedAt: string;
}

export interface AppSettings {
  theme: ThemeMode;
  closeBehavior: "tray" | "quit";
  hideOnBlur: boolean;
  quickPinned: boolean;
  autostart: boolean;
  shortcut: string;
  quickActiveNoteId: string | null;
  organizerEnabled: boolean;
  archiveDays: number;
}

export interface Suggestion {
  id: string;
  kind: string;
  label: string;
  value: Record<string, unknown>;
  confidence: number;
  ruleId: string;
}

export interface EntityMatch {
  entityType: "url" | "email" | "phone";
  value: string;
  startOffset: number;
  endOffset: number;
}

export interface OrganizerResult {
  suggestions: Suggestion[];
  entities: EntityMatch[];
}

export interface ImportPreview {
  notesToCreate: number;
  duplicatesToSkip: number;
  conflictsAsCopies: number;
  categoriesToMerge: number;
  tagsToMerge: number;
}

export interface ImportResult {
  created: number;
  skipped: number;
  copiedConflicts: number;
}

export interface BackupManifest {
  schemaVersion: number;
  appVersion: string;
  createdAt: string;
  databaseSha256: string;
}

export function emptyNote(source: "main" | "quick" = "main"): Note {
  const now = new Date().toISOString();
  return {
    id: `draft:${crypto.randomUUID()}`,
    title: "",
    content: "",
    categoryId: null,
    category: null,
    tags: [],
    isFavorite: false,
    isTodo: false,
    todoStatus: "not_started",
    priority: "normal",
    dueDate: null,
    completedAt: null,
    source,
    revision: 0,
    createdAt: now,
    updatedAt: now,
    deletedAt: null,
    archivedAt: null,
  };
}

export function noteToSaveInput(note: Note): SaveNoteInput {
  return {
    id: note.id.startsWith("draft:") ? null : note.id,
    expectedRevision: note.id.startsWith("draft:") ? null : note.revision,
    title: note.title,
    content: note.content,
    categoryId: note.categoryId,
    tagNames: note.tags.map((tag) => tag.name),
    isFavorite: note.isFavorite,
    isTodo: note.isTodo,
    todoStatus: note.todoStatus,
    priority: note.priority,
    dueDate: note.dueDate,
    source: note.source,
  };
}
