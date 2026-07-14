import { create } from "zustand";

type MainPanel = "notes" | "settings";

interface UiState {
  panel: MainPanel;
  mobilePane: "sidebar" | "list" | "editor";
  sidebarOpen: boolean;
  setPanel: (panel: MainPanel) => void;
  setMobilePane: (pane: UiState["mobilePane"]) => void;
  setSidebarOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  panel: "notes",
  mobilePane: "list",
  sidebarOpen: false,
  setPanel: (panel) => set({ panel }),
  setMobilePane: (mobilePane) => set({ mobilePane }),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
}));
