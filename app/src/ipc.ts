import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CapabilityReport } from "./bindings/CapabilityReport";
import type { CategoryInfo } from "./bindings/CategoryInfo";
import type { Config } from "./bindings/Config";
import type { FiredTrigger } from "./bindings/FiredTrigger";
import type { Permission } from "./bindings/Permission";
import type { PermissionState } from "./bindings/PermissionState";
import type { Preset } from "./bindings/Preset";
import type { RequestOutcome } from "./bindings/RequestOutcome";
import type { RunReport } from "./bindings/RunReport";
import type { Status } from "./bindings/Status";
import type { ValidationIssue } from "./bindings/ValidationIssue";

export const EVENTS = {
  status: "closecall://status",
  run: "closecall://run",
} as const;

export const getConfig = () => invoke<Config>("get_config");

/** Resolves to the validation issues; an empty list means the config was saved. */
export const saveConfig = (config: Config) => invoke<ValidationIssue[]>("save_config", { config });

export const resetConfig = (preset: Preset) => invoke<Config>("reset_config", { preset });

export const getStatus = () => invoke<Status>("get_status");

export const preview = (trigger: FiredTrigger) => invoke<RunReport>("preview", { trigger });

export const runPanic = () => invoke<RunReport>("run_panic");

/** Unlike `preview`, this really acts, as if the trigger had fired. */
export const simulate = (trigger: FiredTrigger) => invoke<RunReport>("simulate", { trigger });

export const getActivity = () => invoke<RunReport[]>("get_activity");

export const restore = (runId: number) => invoke<RunReport>("restore", { run_id: runId });

export const getPermissions = () => invoke<PermissionState[]>("get_permissions");

export const requestPermission = (permission: Permission) =>
  invoke<RequestOutcome>("request_permission", { permission });

export const getCapabilities = () => invoke<CapabilityReport>("get_capabilities");

export const listCategories = () => invoke<CategoryInfo[]>("list_categories");

export const pause = (minutes: number) => invoke<Status>("pause", { minutes });

export const resume = () => invoke<Status>("resume");

export const onStatus = (handler: (status: Status) => void): Promise<UnlistenFn> =>
  listen<Status>(EVENTS.status, (event) => handler(event.payload));

export const onRun = (handler: (report: RunReport) => void): Promise<UnlistenFn> =>
  listen<RunReport>(EVENTS.run, (event) => handler(event.payload));

// Plugin commands. The JS wrappers of these plugins are thin `invoke` calls; using them
// directly keeps every bridge call in this file without two extra dependencies.

export const autostartEnabled = () => invoke<boolean>("plugin:autostart|is_enabled");

export const setAutostart = (enabled: boolean) =>
  invoke<void>(enabled ? "plugin:autostart|enable" : "plugin:autostart|disable");

export const openUrl = (url: string) => invoke<void>("plugin:opener|open_url", { url });
