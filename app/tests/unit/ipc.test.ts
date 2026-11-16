import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { RunReport } from "../../src/bindings/RunReport";
import * as ipc from "../../src/ipc";
import { installMock } from "../../src/mock";

const share = { kind: "screen_share", confidence: "high" } as const;

beforeEach(() => installMock());
afterEach(() => clearMocks());

describe("browser mock", () => {
  test("status reports permissions the way onboarding expects", async () => {
    const status = await ipc.getStatus();
    expect(status.platform).toBe("macos");
    expect(typeof status.last_run?.run_id).toBe("number");
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

  test("private tabs close on a share through their own preset rule", async () => {
    const report = await ipc.preview(share);
    const incognito = report.steps.find((step) =>
      step.because.some((reason) => reason.evidence.kind === "private"),
    );
    expect(incognito?.because).toEqual([{ rule: "private-windows-on-share", evidence: { kind: "private" } }]);
    expect(incognito?.action).toBe("close");
  });

  test("conceal steps come before tab and document closes", async () => {
    const phases = (await ipc.preview(share)).steps.map((step) => step.phase);
    expect(phases).toEqual([...phases].sort((a, b) => order(a) - order(b)));
  });

  test("turning a rule off changes the plan", async () => {
    const config = await ipc.getConfig();
    const tabsRule = "share-close-sensitive-tabs";
    const planned = async () =>
      (await ipc.preview(share)).steps.flatMap((step) => step.because.map((reason) => reason.rule));
    expect(await planned()).toContain(tabsRule);
    const issues = await ipc.saveConfig({
      ...config,
      rule: config.rule.map((rule) => (rule.id === tabsRule ? { ...rule, enabled: false } : rule)),
    });
    expect(issues).toEqual([]);
    expect(await planned()).not.toContain(tabsRule);
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
    // The default unsaved policy saves a file-backed document, then closes it.
    const word = report.steps.find((step) => step.summary.app_name === "Microsoft Word");
    expect(word?.outcome).toEqual({ kind: "done" });
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

  test("asking an app that is not running defers the prompt", async () => {
    const outcome = await ipc.requestPermission({
      kind: "automation",
      bundle_id: "company.thebrowser.Browser",
      app_name: "Arc",
    });
    expect(outcome).toEqual({ kind: "app_not_running" });
  });

  test("only the broad UT1 categories are opt-in", async () => {
    const optIn = (await ipc.listCategories()).filter((info) => !info.default_enabled).map((info) => info.id);
    expect(optIn).toEqual(["financial", "dating", "gambling"]);
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
