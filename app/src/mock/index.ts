import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Action } from "../bindings/Action";
import type { Config } from "../bindings/Config";
import type { Evidence } from "../bindings/Evidence";
import type { FiredTrigger } from "../bindings/FiredTrigger";
import type { Matcher } from "../bindings/Matcher";
import type { Permission } from "../bindings/Permission";
import type { PermissionState } from "../bindings/PermissionState";
import type { Phase } from "../bindings/Phase";
import type { RequestOutcome } from "../bindings/RequestOutcome";
import type { Rule } from "../bindings/Rule";
import type { RunReport } from "../bindings/RunReport";
import type { Status } from "../bindings/Status";
import type { StepOutcome } from "../bindings/StepOutcome";
import type { StepReport } from "../bindings/StepReport";
import { hostOf, parseDur } from "../format";
import { EVENTS } from "../ipc";
import { appSelectorMatches, categoriesIn, triggerDur, triggerKind } from "../model";
import {
  capabilities,
  categories,
  displays,
  emptyConfig,
  inventory,
  type MockItem,
  permissions,
  recommendedConfig,
  runningApps,
  seedActivity,
  signalSources,
} from "./data";
import { validate } from "./validate";

const STRENGTH: Action[] = ["notify_only", "minimize", "hide", "close", "quit"];
const PHASES: Phase[] = ["conceal", "close_tabs", "close_documents", "notify"];
const UNIT_MS = { s: 1_000, m: 60_000, h: 3_600_000, d: 86_400_000 } as const;

function durMs(dur: string | null): number {
  const parsed = dur ? parseDur(dur) : null;
  return parsed ? parsed.value * UNIT_MS[parsed.unit] : 0;
}

function armed(rule: Rule, fired: FiredTrigger): boolean {
  if (!rule.enabled) return false;
  return rule.when.some((trigger) => {
    const kind = triggerKind(trigger);
    switch (fired.kind) {
      case "away":
        return kind === "away_longer_than" && durMs(fired.away) >= durMs(triggerDur(trigger));
      case "idle":
        return kind === "idle_longer_than" && durMs(fired.idle) >= durMs(triggerDur(trigger));
      case "item_age":
        return kind === "item_open_longer_than" && durMs(fired.age) >= durMs(triggerDur(trigger));
      default:
        return kind === fired.kind;
    }
  });
}

function isProtected(item: MockItem, config: Config): boolean {
  const { protect } = config;
  if (item.category !== null && protect.categories.includes(item.category)) return true;
  if (protect.apps.some((app) => "name" in app && app.name === item.summary.app_name)) return true;
  const host = item.summary.url ? hostOf(item.summary.url) : "";
  return protect.domains.some((domain) => host === domain || host.endsWith(`.${domain}`));
}

function phaseFor(item: MockItem, action: Action): Phase {
  if (action === "notify_only") return "notify";
  if (action === "hide" || action === "minimize") return "conceal";
  return item.kind === "tab" ? "close_tabs" : item.kind === "document" ? "close_documents" : "conceal";
}

interface Planned {
  item: MockItem;
  rule: Rule;
  step: StepReport;
}

function mentionsPrivate(matcher: Matcher): boolean {
  if (matcher === "private") return true;
  if ("any" in matcher) return matcher.any.some(mentionsPrivate);
  if ("all" in matcher) return matcher.all.some(mentionsPrivate);
  return false;
}

function evidenceFor(rule: Rule, item: MockItem): Evidence | null {
  if (item.private && mentionsPrivate(rule.match)) return { kind: "private" };
  if (item.category === null) return null;
  if (!categoriesIn(rule.match).includes(item.category)) return null;
  if (categoriesIn(rule.except).includes(item.category)) return null;
  return { kind: "category", id: item.category, via: item.via };
}

function plan(fired: FiredTrigger, config: Config): Planned[] {
  const planned = new Map<number, Planned>();
  for (const rule of config.rule.filter((candidate) => armed(candidate, fired))) {
    for (const item of inventory) {
      if (!rule.targets.includes(item.kind) || isProtected(item, config)) continue;
      const evidence = evidenceFor(rule, item);
      if (!evidence) continue;
      const reason = { rule: rule.id, evidence };
      const existing = planned.get(item.id);
      if (existing) {
        existing.step.because.push(reason);
        if (STRENGTH.indexOf(rule.action) > STRENGTH.indexOf(existing.step.action)) {
          existing.step.action = rule.action;
          existing.step.phase = phaseFor(item, rule.action);
          existing.rule = rule;
        }
        continue;
      }
      planned.set(item.id, {
        item,
        rule,
        step: {
          target: item.id,
          summary: item.summary,
          action: rule.action,
          phase: phaseFor(item, rule.action),
          because: [reason],
          outcome: null,
        },
      });
    }
  }
  return [...planned.values()].sort((a, b) => PHASES.indexOf(a.step.phase) - PHASES.indexOf(b.step.phase));
}

function outcomeFor({ item, rule }: Planned, perms: PermissionState[]): StepOutcome {
  if (item.automation) {
    const grant = perms.find(
      (state) => state.permission.kind === "automation" && state.permission.bundle_id === item.automation,
    );
    if (grant && grant.status !== "granted") {
      return { kind: "fell_back", action: "hide", reason_key: "reason.automation_missing" };
    }
  }
  if (item.dirty && (rule.action === "close" || rule.action === "quit")) {
    switch (rule.on_unsaved) {
      case "hide":
        return { kind: "fell_back", action: "hide", reason_key: "reason.unsaved_hidden" };
      case "skip_and_notify":
        return { kind: "skipped", reason_key: "reason.unsaved_skipped" };
      case "save_then_close":
        if (item.summary.path === null)
          return { kind: "fell_back", action: "hide", reason_key: "reason.untitled_hidden" };
        break;
    }
  }
  return { kind: "done" };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isConfig(value: unknown): value is Config {
  return isRecord(value) && Array.isArray(value.rule) && isRecord(value.protect);
}

const FIRED_KINDS = new Set([
  "panic",
  "screen_share",
  "display_mirrored",
  "display_connected",
  "call_likely",
  "will_sleep",
  "lid_closed",
  "locked",
  "away",
  "idle",
  "item_age",
]);

function isFiredTrigger(value: unknown): value is FiredTrigger {
  return isRecord(value) && typeof value.kind === "string" && FIRED_KINDS.has(value.kind);
}

function isPermission(value: unknown): value is Permission {
  return isRecord(value) && typeof value.kind === "string";
}

function expect<T>(value: unknown, guard: (value: unknown) => value is T, what: string): T {
  if (!guard(value)) throw new Error(`mock: bad ${what} argument`);
  return value;
}

export function installMock() {
  const now = Date.now();
  const query = new URLSearchParams(window.location.search);
  const state = query.get("state");
  let config = recommendedConfig();
  let perms = structuredClone(permissions);
  let activity = seedActivity(now);
  let nextRunId = 3;
  let autostart = true;
  // `?state=docked`: the DisplayLink agent holds the share watcher, like on a USB dock.
  const docked = state === "docked";
  const heldBy = (current: Config) =>
    docked
      ? (runningApps.find(
          (app) =>
            app.id === "com.displaylink.DisplayLinkUserAgent" &&
            current.ignore.capture_apps.some((selector) => appSelectorMatches(selector, app)),
        )?.name ?? null)
      : null;
  const sourcesFor = (ignored: string | null) =>
    signalSources.map((source) =>
      ignored && source.id === "share_watcher"
        ? { ...source, health: { state: "noisy" as const, reason_key: "reason.capture_ignored" } }
        : source,
    );
  let status: Status = {
    platform: "macos",
    app_version: "0.1.0",
    share: {
      active: state === "sharing",
      confidence: state === "sharing" ? "high" : null,
      mirrored_display: state === "mirrored",
      external_display: false,
      call_likely: false,
      ignored_capture: heldBy(config),
    },
    guard_active: state === "sharing" || state === "mirrored",
    paused_until_ms: state === "paused" ? now + 40 * 60_000 : null,
    last_run: null,
    permissions: perms,
    signal_sources: sourcesFor(heldBy(config)),
    needs_onboarding: query.has("onboarding"),
    // `?state=config_problem`: config.toml was rejected and the recommended rules run instead.
    config_problem: state === "config_problem" ? "config.problem.unparsable" : null,
  };
  const summarize = (report: RunReport) => {
    const count = (kind: StepOutcome["kind"]) =>
      report.steps.filter((step) => step.outcome?.kind === kind).length;
    status = {
      ...status,
      last_run: {
        run_id: report.run_id,
        at_ms: report.at_ms,
        trigger: report.trigger,
        done: count("done"),
        fell_back: count("fell_back"),
        skipped: count("skipped"),
        failed: count("failed"),
      },
    };
  };
  const firstRun = activity[0];
  if (firstRun) summarize(firstRun);

  // Rust emits after the command has answered; a microtask keeps that order.
  const announce = (report?: RunReport) => {
    queueMicrotask(() => {
      if (report) void emit(EVENTS.run, report);
      void emit(EVENTS.status, status);
    });
  };

  const run = (trigger: FiredTrigger, dryRun: boolean): RunReport => {
    const steps = plan(trigger, config).map((planned) => ({
      ...planned.step,
      outcome: dryRun ? null : outcomeFor(planned, perms),
    }));
    const report: RunReport = {
      run_id: dryRun ? 0 : nextRunId++,
      at_ms: Date.now(),
      trigger,
      dry_run: dryRun,
      steps,
      restorable: !dryRun && steps.length > 0,
      restore_of: null,
    };
    if (!dryRun) {
      activity = [report, ...activity];
      summarize(report);
      announce(report);
    }
    return report;
  };

  const setPermission = (permission: Permission, next: PermissionState["status"]) => {
    perms = perms.map((entry) =>
      JSON.stringify(entry.permission) === JSON.stringify(permission) ? { ...entry, status: next } : entry,
    );
    status = { ...status, permissions: perms };
    announce();
  };

  const saved = (next: Config) => {
    config = next;
    const ignored = heldBy(next);
    status = {
      ...status,
      share: { ...status.share, ignored_capture: ignored },
      signal_sources: sourcesFor(ignored),
      needs_onboarding: false,
    };
    announce();
  };

  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      const args = isRecord(payload) ? payload : {};
      switch (cmd) {
        case "get_config":
          return structuredClone(config);
        case "save_config": {
          const next = expect(args.config, isConfig, "config");
          const issues = validate(next);
          if (issues.length === 0) saved(structuredClone(next));
          return issues;
        }
        case "reset_config":
          saved(args.preset === "empty" ? emptyConfig() : recommendedConfig());
          return structuredClone(config);
        case "get_status":
          return structuredClone(status);
        case "preview":
          return run(expect(args.trigger, isFiredTrigger, "trigger"), true);
        case "run_panic":
          return run({ kind: "panic" }, false);
        case "simulate":
          return run(expect(args.trigger, isFiredTrigger, "trigger"), false);
        case "get_activity":
          return structuredClone(activity);
        case "restore": {
          const report = activity.find((entry) => entry.run_id === args.run_id);
          if (!report?.restorable) throw new Error(`run ${String(args.run_id)} is not restorable`);
          report.restorable = false;
          const restored: RunReport = {
            ...structuredClone(report),
            run_id: nextRunId++,
            at_ms: Date.now(),
            restore_of: report.run_id,
            steps: report.steps
              .filter((step) => step.outcome?.kind === "done" || step.outcome?.kind === "fell_back")
              .reverse()
              .map((step) => ({
                ...step,
                action: step.outcome?.kind === "fell_back" ? step.outcome.action : step.action,
                outcome:
                  step.summary.kind === "tab" &&
                  step.action === "close" &&
                  (step.summary.app_name === "Safari" ||
                    step.because.some((reason) => reason.evidence.kind === "private"))
                    ? { kind: "skipped", reason_key: "reason.private_not_restorable" }
                    : { kind: "done" },
              })),
          };
          activity = [restored, ...activity];
          announce(restored);
          return structuredClone(restored);
        }
        case "get_permissions":
          return structuredClone(perms);
        case "request_permission": {
          const permission = expect(args.permission, isPermission, "permission");
          const current = perms.find(
            (entry) => JSON.stringify(entry.permission) === JSON.stringify(permission),
          );
          const { share } = status;
          if (share.active || share.mirrored_display || share.external_display)
            return { kind: "not_while_sharing" } satisfies RequestOutcome;
          if (current?.status === "app_not_running")
            return { kind: "app_not_running" } satisfies RequestOutcome;
          if (permission.kind === "accessibility") {
            setTimeout(() => setPermission(permission, "granted"), 1_500);
            return { kind: "opened_settings" } satisfies RequestOutcome;
          }
          setPermission(permission, "granted");
          return { kind: "granted" } satisfies RequestOutcome;
        }
        case "get_capabilities":
          return structuredClone(capabilities);
        case "list_categories":
          return structuredClone(categories);
        case "list_displays":
          return structuredClone(displays);
        case "list_running_apps":
          return structuredClone(runningApps);
        case "pause":
          status = { ...status, paused_until_ms: Date.now() + Number(args.minutes) * 60_000 };
          announce();
          return structuredClone(status);
        case "resume":
          status = { ...status, paused_until_ms: null };
          announce();
          return structuredClone(status);
        case "plugin:autostart|is_enabled":
          return autostart;
        case "plugin:autostart|enable":
          autostart = true;
          return null;
        case "plugin:autostart|disable":
          autostart = false;
          return null;
        case "plugin:opener|open_url":
          window.open(String(args.url), "_blank", "noopener,noreferrer");
          return null;
        default:
          throw new Error(`mock: unknown command ${cmd}`);
      }
    },
    { shouldMockEvents: true },
  );
}
