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
import type { RunSummary } from "./bindings/RunSummary";
import type { Status } from "./bindings/Status";
import type { ValidationIssue } from "./bindings/ValidationIssue";

export const EVENTS = {
  status: "closecall://status",
  run: "closecall://run",
} as const;

/**
 * The shape that actually crosses the bridge. ts-rs types u64 as `bigint`, but serde_json
 * sends plain JSON numbers, so every u64 is converted here and nowhere else.
 */
export type Wire<T> = T extends bigint
  ? number
  : T extends readonly (infer U)[]
    ? Wire<U>[]
    : T extends object
      ? { [K in keyof T]: Wire<T[K]> }
      : T;

export function decodeTrigger(trigger: Wire<FiredTrigger>): FiredTrigger {
  return trigger.kind === "item_age" ? { ...trigger, target: BigInt(trigger.target) } : trigger;
}

function encodeTrigger(trigger: FiredTrigger): Wire<FiredTrigger> {
  return trigger.kind === "item_age" ? { ...trigger, target: Number(trigger.target) } : trigger;
}

export function decodeRunReport(report: Wire<RunReport>): RunReport {
  return {
    ...report,
    run_id: BigInt(report.run_id),
    at_ms: BigInt(report.at_ms),
    trigger: decodeTrigger(report.trigger),
    steps: report.steps.map((step) => ({ ...step, target: BigInt(step.target) })),
  };
}

function decodeRunSummary(summary: Wire<RunSummary>): RunSummary {
  return {
    ...summary,
    run_id: BigInt(summary.run_id),
    at_ms: BigInt(summary.at_ms),
    trigger: decodeTrigger(summary.trigger),
  };
}

export function decodeStatus(status: Wire<Status>): Status {
  return {
    ...status,
    paused_until_ms: status.paused_until_ms === null ? null : BigInt(status.paused_until_ms),
    last_run: status.last_run === null ? null : decodeRunSummary(status.last_run),
  };
}

export const getConfig = () => invoke<Config>("get_config");

/** Resolves to the validation issues; an empty list means the config was saved. */
export const saveConfig = (config: Config) => invoke<ValidationIssue[]>("save_config", { config });

export const resetConfig = (preset: Preset) => invoke<Config>("reset_config", { preset });

export const getStatus = async () => decodeStatus(await invoke<Wire<Status>>("get_status"));

export const preview = async (trigger: FiredTrigger) =>
  decodeRunReport(await invoke<Wire<RunReport>>("preview", { trigger: encodeTrigger(trigger) }));

export const runPanic = async () => decodeRunReport(await invoke<Wire<RunReport>>("run_panic"));

/** Unlike `preview`, this really acts, as if the trigger had fired. */
export const simulate = async (trigger: FiredTrigger) =>
  decodeRunReport(await invoke<Wire<RunReport>>("simulate", { trigger: encodeTrigger(trigger) }));

export const getActivity = async () => (await invoke<Wire<RunReport>[]>("get_activity")).map(decodeRunReport);

export const restore = async (runId: bigint) =>
  decodeRunReport(await invoke<Wire<RunReport>>("restore", { run_id: Number(runId) }));

export const getPermissions = () => invoke<PermissionState[]>("get_permissions");

export const requestPermission = (permission: Permission) =>
  invoke<RequestOutcome>("request_permission", { permission });

export const getCapabilities = () => invoke<CapabilityReport>("get_capabilities");

export const listCategories = () => invoke<CategoryInfo[]>("list_categories");

export const pause = async (minutes: number) =>
  decodeStatus(await invoke<Wire<Status>>("pause", { minutes }));

export const resume = async () => decodeStatus(await invoke<Wire<Status>>("resume"));

export const onStatus = (handler: (status: Status) => void): Promise<UnlistenFn> =>
  listen<Wire<Status>>(EVENTS.status, (event) => handler(decodeStatus(event.payload)));

export const onRun = (handler: (report: RunReport) => void): Promise<UnlistenFn> =>
  listen<Wire<RunReport>>(EVENTS.run, (event) => handler(decodeRunReport(event.payload)));

// Plugin commands. The JS wrappers of these plugins are thin `invoke` calls; using them
// directly keeps every bridge call in this file without two extra dependencies.

export const autostartEnabled = () => invoke<boolean>("plugin:autostart|is_enabled");

export const setAutostart = (enabled: boolean) =>
  invoke<void>(enabled ? "plugin:autostart|enable" : "plugin:autostart|disable");

export const openUrl = (url: string) => invoke<void>("plugin:opener|open_url", { url });
