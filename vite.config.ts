import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";

const host = process.env.TAURI_DEV_HOST;
const tauriConfig = JSON.parse(
  readFileSync(new URL("./src-tauri/tauri.conf.json", import.meta.url), "utf8"),
) as { productName: string; version: string };

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: "shanji-app-metadata",
      transformIndexHtml: (html) => html.replaceAll("%PRODUCT_NAME%", tauriConfig.productName),
    },
  ],
  define: {
    __PRODUCT_NAME__: JSON.stringify(tauriConfig.productName),
    __APP_VERSION__: JSON.stringify(tauriConfig.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
    css: true,
    coverage: {
      reporter: ["text", "html"],
    },
  },
});
