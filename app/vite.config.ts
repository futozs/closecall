import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  // The modulepreload polyfill calls fetch(), which the CSP (connect-src ipc: only) forbids;
  // WKWebView and WebView2 support modulepreload natively.
  build: { target: "safari16", outDir: "dist", sourcemap: false, modulePreload: { polyfill: false } },
});
