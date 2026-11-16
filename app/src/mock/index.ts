import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Action } from "../bindings/Action";
import type { Config } from "../bindings/Config";
import type { FiredTrigger } from "../bindings/FiredTrigger";
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
import { EVENTS, type Wire } from "../ipc";
import { categoriesIn, triggerDur, triggerKind } from "../model";
import {
  capabilities,
  categories,
  emptyConfig,
  inventory,
  type MockItem,
  permissions,
  recommendedConfig,
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
  if (protect.categories.includes(item.category)) return true;
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
  step: Wire<StepReport>;
}

function plan(fired: FiredTrigger, config: Config): Planned[] {
  const planned = new Map<number, Planned>();
  for (const rule of config.rule.filter((candidate) => armed(candidate, fired))) {
    for (const item of inventory) {
      const hit =
        rule.targets.includes(item.kind) &&
        categoriesIn(rule.match).includes(item.category) &&
        !categoriesIn(rule.except).includes(item.category) &&
        !isProtected(item, config);
      if (!hit) continue;
      const reason = {
        rule: rule.id,
        evidence: { kind: "category" as const, id: item.category, via: item.via },
      };
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
    if (rule.on_unsaved === "hide")
      return { kind: "fell_back", action: "hide", reason_key: "reason.unsaved_hidden" };
    if (rule.on_unsaved === "skip_and_notify")
      return { kind: "skipped", reason_key: "reason.unsaved_skipped" };
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
  "call_likely",
  "will_sleep",
  "lid_closed",
  "locked",
  "away",
  "idle",
  "item_age",
]);

function isFiredTrigger(value: unknown): value is Wire<FiredTrigger> {
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
  let status: Wire<Status> = {
    platform: "macos",
    app_version: "0.1.0",
    share: {
      active: state === "sharing",
      confidence: state === "sharing" ? "high" : null,
      mirrored_display: state === "mirrored",
      call_likely: false,
    },
    guard_active: state === "sharing" || state === "mirrored",
    paused_until_ms: state === "paused" ? now + 40 * 60_000 : null,
    last_run: null,
    permissions: perms,
    signal_sources: signalSources,
    needs_onboarding: query.has("onboarding"),
  };
  const summarize = (report: Wire<RunReport>) => {
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
  const announce = (report?: Wire<RunReport>) => {
    queueMicrotask(() => {
      if (report) void emit(EVENTS.run, report);
      void emit(EVENTS.status, status);
    });
  };

  const run = (trigger: Wire<FiredTrigger>, dryRun: boolean): Wire<RunReport> => {
    const fired: FiredTrigger =
      trigger.kind === "item_age" ? { ...trigger, target: BigInt(trigger.target) } : trigger;
    const steps = plan(fired, config).map((planned) => ({
      ...planned.step,
      outcome: dryRun ? null : outcomeFor(planned, perms),
    }));
    const report: Wire<RunReport> = {
      run_id: dryRun ? 0 : nextRunId++,
      at_ms: Date.now(),
      trigger,
      dry_run: dryRun,
      steps,
      restorable: !dryRun && steps.length > 0,
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
    status = { ...status, needs_onboarding: false };
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
          if (!report) throw new Error(`no run ${String(args.run_id)}`);
          report.restorable = false;
          return structuredClone(report);
        }
        case "get_permissions":
          return structuredClone(perms);
        case "request_permission": {
          const permission = expect(args.permission, isPermission, "permission");
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
