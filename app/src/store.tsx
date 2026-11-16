import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CategoryInfo } from "./bindings/CategoryInfo";
import type { Config } from "./bindings/Config";
import type { Status } from "./bindings/Status";
import type { ValidationIssue } from "./bindings/ValidationIssue";
import { resolveLocale, setLocale } from "./i18n";
import * as ipc from "./ipc";

export interface Initial {
  status: Status;
  config: Config;
  categories: CategoryInfo[];
}

interface AppData extends Initial {
  locale: string;
  setStatus: (status: Status) => void;
  setConfig: (config: Config) => void;
  saveConfig: (config: Config) => Promise<ValidationIssue[]>;
}

const Context = createContext<AppData | null>(null);

export function loadInitial(): Promise<Initial> {
  return Promise.all([ipc.getStatus(), ipc.getConfig(), ipc.listCategories()]).then(
    ([status, config, categories]) => ({ status, config, categories }),
  );
}

export function AppDataProvider({ initial, children }: { initial: Initial; children: ReactNode }) {
  const { i18n } = useTranslation();
  const [status, setStatus] = useState(initial.status);
  const [config, setConfig] = useState(initial.config);

  useEffect(() => {
    const unlisten = ipc.onStatus(setStatus);
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    void setLocale(resolveLocale(config.language));
  }, [config.language]);

  const saveConfig = useCallback(async (next: Config) => {
    const issues = await ipc.saveConfig(next);
    if (issues.length === 0) setConfig(next);
    return issues;
  }, []);

  const value = useMemo(
    () => ({
      status,
      config,
      categories: initial.categories,
      locale: i18n.language,
      setStatus,
      setConfig,
      saveConfig,
    }),
    [status, config, initial.categories, i18n.language, saveConfig],
  );
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useAppData(): AppData {
  const value = useContext(Context);
  if (!value) throw new Error("useAppData outside AppDataProvider");
  return value;
}

export function useLoad<T>(load: () => Promise<T>, key: unknown = null) {
  const [state, setState] = useState<{ data: T | null; error: unknown; loading: boolean }>({
    data: null,
    error: null,
    loading: true,
  });
  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` is the reload signal; `load` is a fresh closure every render.
  useEffect(() => {
    let live = true;
    setState((prev) => ({ ...prev, loading: true }));
    load().then(
      (data) => live && setState({ data, error: null, loading: false }),
      (error: unknown) => live && setState({ data: null, error, loading: false }),
    );
    return () => {
      live = false;
    };
  }, [key]);
  return { ...state, setData: (data: T) => setState({ data, error: null, loading: false }) };
}
