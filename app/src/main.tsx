import "./styles.css";
import { isTauri } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { i18next, initI18n, resolveLocale, systemLocale } from "./i18n";
import { AppDataProvider, loadInitial } from "./store";

async function start() {
  const root = document.getElementById("root");
  if (!root) throw new Error("#root missing from index.html");

  // The mock only exists in dev and in the `mock` build used by the UI tests; production
  // builds drop this branch and the chunk with it.
  if (!isTauri() && (import.meta.env.DEV || import.meta.env.MODE === "mock")) {
    const { installMock } = await import("./mock");
    installMock();
  }

  try {
    const initial = await loadInitial();
    await initI18n(resolveLocale(initial.config.language));
    createRoot(root).render(
      <StrictMode>
        <AppDataProvider initial={initial}>
          <App />
        </AppDataProvider>
      </StrictMode>,
    );
  } catch (error) {
    await initI18n(systemLocale());
    root.textContent = i18next.t("common.startup_error", { error: String(error) });
    console.error(error);
  }
}

void start();
