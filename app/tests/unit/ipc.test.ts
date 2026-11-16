import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { RunReport } from "../../src/bindings/RunReport";
import * as ipc from "../../src/ipc";
import { installMock } from "../../src/mock";

const share = { kind: "screen_share", confidence: "high" } as const;

beforeEach(() => installMock());
afterEach(() => clearMocks());

describe("wire decoding", () => {
  test("u64 fields arrive as numbers and leave ipc.ts as bigint", () => {
    const report = ipc.decodeRunReport({
      run_id: 7,
      at_ms: 1_700_000_000_000,
      trigger: { kind: "item_age", target: 42, age: "2h" },
      dry_run: false,
      restorable: true,
      steps: [],
    });
    expect(report.run_id).toBe(7n);
    expect(report.at_ms).toBe(1_700_000_000_000n);
    expect(report.trigger).toEqual({ kind: "item_age", target: 42n, age: "2h" });
  });
});

describe("browser mock", () => {
  test("status reports permissions the way onboarding expects", async () => {
    const status = await ipc.getStatus();
    expect(status.platform).toBe("macos");
    expect(typeof status.last_run?.run_id).toBe("bigint");
    const byKind = status.permissions.map((state) => [state.permission, state.status]);
    expect(byKind).toContainEqual([{ kind: "accessibility" }, "granted"]);
    expect(byKind).toContainEqual([
      { kind: "automation", bundle_id: "com.google.Chrome", app_name: "Google Chrome" },
      "not_determined",
    ]);
  });

  test("a dry run explains every step and touches nothing", async () => {
    const report = await ipc.preview(share);
    expect(report.dry_run).toBe(true);
    expect(report.steps.length).toBeGreaterThan(0);
    expect(report.steps.every((step) => step.outcome === null && step.because.length > 0)).toBe(true);
    const otp = report.steps.find((step) => step.summary.url?.includes("otpbank.hu"));
    expect(otp?.because[0]?.evidence).toEqual({
      kind: "category",
      id: "banking",
      via: { kind: "domain", domain: "otpbank.hu" },
    });
    expect(await ipc.getActivity()).toHaveLength(2);
  });

  test("conceal steps come before tab and document closes", async () => {
    const phases = (await ipc.preview(share)).steps.map((step) => step.phase);
    expect(phases).toEqual([...phases].sort((a, b) => order(a) - order(b)));
  });

  test("turning a rule off changes the plan", async () => {
    const config = await ipc.getConfig();
    const tabsRule = "share-close-sensitive-tabs";
    expect((await ipc.preview(share)).steps.some((step) => step.summary.kind === "tab")).toBe(true);
    const issues = await ipc.saveConfig({
      ...config,
      rule: config.rule.map((rule) => (rule.id === tabsRule ? { ...rule, enabled: false } : rule)),
    });
    expect(issues).toEqual([]);
    expect((await ipc.preview(share)).steps.some((step) => step.summary.kind === "tab")).toBe(false);
  });

  test("an invalid config is rejected with issue codes and not saved", async () => {
    const config = await ipc.getConfig();
    const broken = config.rule.map((rule, i) =>
      i === 0 ? { ...rule, when: [], match: { any: [{ domain: "https://otpbank.hu/" }] } } : rule,
    );
    const issues = await ipc.saveConfig({ ...config, rule: broken });
    expect(issues.map((issue) => [issue.field, issue.code])).toEqual([
      ["when", "empty_when"],
      ["match.any[0]", "invalid_domain"],
    ]);
    expect(await ipc.getConfig()).toEqual(config);
  });

  test("a countdown on a share trigger is refused", async () => {
    const config = await ipc.getConfig();
    const rule = config.rule[0];
    if (!rule) throw new Error("recommended preset is empty");
    const issues = await ipc.saveConfig({ ...config, rule: [{ ...rule, confirm: { countdown: "60s" } }] });
    expect(issues).toEqual([{ rule: rule.id, field: "confirm", code: "countdown_not_allowed" }]);
  });

  test("a simulated share acts, falls back where permission is missing, and reaches Activity", async () => {
    const events: RunReport[] = [];
    await ipc.onRun((report) => events.push(report));
    const report = await ipc.simulate(share);
    expect(report.dry_run).toBe(false);
    const chrome = report.steps.find((step) => step.summary.app_name === "Google Chrome");
    expect(chrome?.outcome).toEqual({
      kind: "fell_back",
      action: "hide",
      reason_key: "reason.automation_missing",
    });
    const word = report.steps.find((step) => step.summary.app_name === "Microsoft Word");
    expect(word?.outcome).toEqual({ kind: "fell_back", action: "hide", reason_key: "reason.unsaved_hidden" });
    expect((await ipc.getActivity())[0]?.run_id).toBe(report.run_id);
    await vi.waitFor(() => expect(events.map((event) => event.run_id)).toEqual([report.run_id]));
  });

  test("granting automation lets the next run close tabs one by one", async () => {
    await ipc.requestPermission({
      kind: "automation",
      bundle_id: "com.google.Chrome",
      app_name: "Google Chrome",
    });
    const report = await ipc.simulate(share);
    const chrome = report.steps.filter((step) => step.summary.app_name === "Google Chrome");
    expect(chrome.every((step) => step.outcome?.kind === "done")).toBe(true);
  });

  test("restore is one-shot", async () => {
    const [latest] = await ipc.getActivity();
    if (!latest) throw new Error("mock has no activity");
    expect(latest.restorable).toBe(true);
    expect((await ipc.restore(latest.run_id)).restorable).toBe(false);
  });

  test("pause and resume round-trip through Status", async () => {
    const paused = await ipc.pause(15);
    expect(paused.paused_until_ms).not.toBeNull();
    expect((await ipc.resume()).paused_until_ms).toBeNull();
  });

  test("login item goes through the autostart plugin commands", async () => {
    expect(await ipc.autostartEnabled()).toBe(true);
    await ipc.setAutostart(false);
    expect(await ipc.autostartEnabled()).toBe(false);
  });
});

function order(phase: string) {
  return ["conceal", "close_tabs", "close_documents", "notify"].indexOf(phase);
}
