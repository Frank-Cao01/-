import { create } from "zustand";
import { api } from "../services/api";
import type { AppSettings, ThemeMode } from "../types";

const defaults: AppSettings = {
  theme: "light",
  closeBehavior: "tray",
  hideOnBlur: true,
  quickPinned: false,
  autostart: false,
  shortcut: navigator.platform.toLowerCase().includes("mac")
    ? "Command+Shift+Space"
    : "Ctrl+Shift+Space",
  quickActiveNoteId: null,
  organizerEnabled: true,
  archiveDays: 90,
};

function resolveTheme(theme: ThemeMode) {
  if (theme !== "system") return theme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(theme: ThemeMode) {
  document.documentElement.dataset.theme = resolveTheme(theme);
  document.documentElement.style.colorScheme = resolveTheme(theme);
}

interface SettingsState {
  settings: AppSettings;
  loading: boolean;
  error: string | null;
  load: () => Promise<AppSettings>;
  save: (settings: AppSettings) => Promise<void>;
  patch: (patch: Partial<AppSettings>) => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: defaults,
  loading: false,
  error: null,
  load: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await api.getSettings();
      applyTheme(settings.theme);
      set({ settings, loading: false });
      return settings;
    } catch (error) {
      set({ error: String(error), loading: false });
      return get().settings;
    }
  },
  save: async (settings) => {
    set({ loading: true, error: null });
    try {
      await api.updateSettings(settings);
      applyTheme(settings.theme);
      set({ settings, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
      throw error;
    }
  },
  patch: (patch) => set({ settings: { ...get().settings, ...patch } }),
}));
