import { useCallback, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Note } from "../types";
import { api } from "../services/api";
import { useNotesStore } from "../stores/notesStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useUiStore } from "../stores/uiStore";
import { Sidebar } from "./Sidebar";
import { NoteList } from "../features/notes/NoteList";
import { NoteEditor } from "../features/notes/NoteEditor";
import { SettingsPage } from "../features/settings/SettingsPage";

export function MainApp() {
  const notes = useNotesStore();
  const ui = useUiStore();
  const flushRef = useRef<(() => Promise<Note>) | null>(null);
  const flushCurrent = useCallback(async () => {
    if (flushRef.current) await flushRef.current();
  }, []);

  useEffect(() => {
    void Promise.all([
      useNotesStore.getState().loadView("all"),
      useNotesStore.getState().loadMetadata(),
      useSettingsStore.getState().load(),
    ]);
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    void listen<string>("note:changed", () => void useNotesStore.getState().loadView()).then((unlisten) => unlisteners.push(unlisten));
    void listen<string>("app:open-note", (event) => void (async () => {
      await flushCurrent();
      useUiStore.getState().setPanel("notes");
      useUiStore.getState().setMobilePane("editor");
      await useNotesStore.getState().selectById(event.payload);
    })()).then((unlisten) => unlisteners.push(unlisten));
    void listen<string>("app:navigate", (event) => void (async () => {
      if (event.payload === "settings") {
        await flushCurrent();
        useUiStore.getState().setPanel("settings");
      }
    })()).then((unlisten) => unlisteners.push(unlisten));
    void listen("main:hide-request", () => void (async () => {
      try {
        await flushCurrent();
        await api.hideMainWindow();
      } catch {
        await api.showMainWindow();
      }
    })()).then((unlisten) => unlisteners.push(unlisten));
    void listen("quick:quit-ready", () => void (async () => {
      try {
        await flushCurrent();
        await api.quitApp();
      } catch {
        await api.showMainWindow();
        if (window.confirm("当前笔记保存失败。仍然退出可能丢失未保存内容，确定继续吗？")) {
          await api.quitApp();
        }
      }
    })()).then((unlisten) => unlisteners.push(unlisten));
    void listen("settings:changed", () => void useSettingsStore.getState().load()).then((unlisten) => unlisteners.push(unlisten));
    return () => unlisteners.forEach((unlisten) => unlisten());
  }, [flushCurrent]);

  const select = (note: Note) => {
    notes.select(note);
    ui.setMobilePane("editor");
  };

  const createNew = async () => {
    await flushCurrent();
    notes.newDraft("main");
    ui.setPanel("notes");
    ui.setMobilePane("editor");
  };

  if (ui.panel === "settings") return <SettingsPage />;

  return (
    <main className="app-shell" data-mobile-pane={ui.mobilePane}>
      <Sidebar beforeNavigate={flushCurrent} />
      <NoteList
        beforeSelect={flushCurrent}
        onSelect={select}
        onNew={() => void createNew()}
        onMenu={() => ui.setSidebarOpen(!ui.sidebarOpen)}
      />
      <NoteEditor
        note={notes.selected}
        categories={notes.categories}
        onSaved={(note) => { notes.upsertLocal(note); void notes.loadMetadata(); }}
        onRemoved={(id) => { notes.removeLocal(id); void notes.loadView(); }}
        onFlushReady={(flush) => { flushRef.current = flush; }}
        onBack={() => ui.setMobilePane("list")}
      />
    </main>
  );
}
