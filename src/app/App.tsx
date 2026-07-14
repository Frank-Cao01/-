import { useEffect } from "react";
import { MainApp } from "./MainApp";
import { QuickWindow } from "../features/quick/QuickWindow";
import { applyTheme, useSettingsStore } from "../stores/settingsStore";

export function App() {
  const theme = useSettingsStore((state) => state.settings.theme);
  useEffect(() => {
    applyTheme(theme);
    if (theme !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const refresh = () => applyTheme("system");
    media.addEventListener("change", refresh);
    return () => media.removeEventListener("change", refresh);
  }, [theme]);
  const isQuick = window.location.hash.startsWith("#/quick");
  return isQuick ? <QuickWindow /> : <MainApp />;
}
