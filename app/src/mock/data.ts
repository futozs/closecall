import type { Action } from "../bindings/Action";
import type { CapabilityReport } from "../bindings/CapabilityReport";
import type { CategoryInfo } from "../bindings/CategoryInfo";
import type { Config } from "../bindings/Config";
import type { Evidence } from "../bindings/Evidence";
import type { PermissionState } from "../bindings/PermissionState";
import type { Rule } from "../bindings/Rule";
import type { RunReport } from "../bindings/RunReport";
import type { SignalSourceStatus } from "../bindings/SignalSourceStatus";
import type { Support } from "../bindings/Support";
import type { TargetKindTag } from "../bindings/TargetKindTag";
import type { TargetSummary } from "../bindings/TargetSummary";
import type { Wire } from "../ipc";

const shareTriggers: Rule["when"] = ["panic", "screen_share", "display_mirrored"];

function presetRule(rule: Pick<Rule, "id" | "when" | "targets" | "match" | "action"> & Partial<Rule>): Rule {
  return {
    name: null,
    enabled: true,
    except: null,
    on_unsaved: "save_then_close",
    confirm: "never",
    ...rule,
  };
}

/** Mirrors the recommended preset in docs/research/architecture-recommendation.md §4.4. */
export function recommendedConfig(): Config {
  return {
    version: 1,
    language: "system",
    panic_hotkey: "Ctrl+Alt+Shift+B",
    share_min_confidence: "medium",
    untitled_save_dir: "~/Documents/Close Call",
    restore_window: "2h",
    protect: { categories: ["meeting"], apps: [], domains: [], audible_tabs: false, pinned_tabs: false },
    categories: [],
    rule: [
      presetRule({
        id: "share-close-sensitive-tabs",
        when: shareTriggers,
        targets: ["tab"],
        match: {
          any: [
            { category: "adult" },
            { category: "banking" },
            { category: "webmail" },
            { category: "medical" },
          ],
        },
        action: "close",
      }),
      presetRule({
        id: "share-hide-private-apps",
        when: shareTriggers,
        targets: ["app_window", "process"],
        match: { any: [{ category: "password_managers" }, { category: "messengers" }] },
        action: "hide",
      }),
      presetRule({
        id: "share-close-invoices",
        when: shareTriggers,
        targets: ["document"],
        match: { category: "invoices" },
        action: "close",
        on_unsaved: "hide",
      }),
      presetRule({
        id: "lid-close-adult",
        when: ["lid_closed", "will_sleep"],
        targets: ["tab"],
        match: { category: "adult" },
        action: "close",
      }),
      presetRule({
        id: "away-close-banking",
        when: [{ away_longer_than: "15m" }],
        targets: ["tab"],
        match: { category: "banking" },
        action: "close",
      }),
      presetRule({
        id: "bank-tab-open-too-long",
        when: [{ item_open_longer_than: "2h" }],
        targets: ["tab"],
        match: { category: "banking" },
        action: "close",
        confirm: { countdown: "60s" },
      }),
    ],
  };
}

export function emptyConfig(): Config {
  return { ...recommendedConfig(), rule: [] };
}

export interface MockItem {
  id: number;
  kind: TargetKindTag;
  summary: TargetSummary;
  category: string;
  via: Evidence;
  /** Bundle id whose Automation grant the provider needs to act on this item. */
  automation?: string;
  dirty?: boolean;
}

function tab(id: number, app: string, title: string, url: string, category: string, via: Evidence): MockItem {
  return {
    id,
    kind: "tab",
    summary: { kind: "tab", app_name: app, title, url, path: null },
    category,
    via,
    automation: app === "Google Chrome" ? "com.google.Chrome" : "com.apple.Safari",
  };
}

export const inventory: MockItem[] = [
  tab(
    101,
    "Google Chrome",
    "OTP Bank – Internetbank – Számlatörténet",
    "https://netbank.otpbank.hu/portal/hu/Szamlatortenet",
    "banking",
    { kind: "domain", domain: "otpbank.hu" },
  ),
  tab(
    102,
    "Google Chrome",
    "Beérkezett üzenetek (12) – Gmail",
    "https://mail.google.com/mail/u/0/#inbox",
    "webmail",
    {
      kind: "domain",
      domain: "mail.google.com",
    },
  ),
  tab(103, "Safari", "EESZT Lakossági Portál – Leleteim", "https://www.eeszt.gov.hu/hu/leleteim", "medical", {
    kind: "domain",
    domain: "eeszt.gov.hu",
  }),
  {
    id: 201,
    kind: "document",
    summary: {
      kind: "document",
      app_name: "Preview",
      title: "szamla_2026_10_digi.pdf",
      url: null,
      path: "~/Downloads/szamla_2026_10_digi.pdf",
    },
    category: "invoices",
    via: { kind: "title_contains", keyword: "számla" },
  },
  {
    id: 202,
    kind: "document",
    summary: {
      kind: "document",
      app_name: "Microsoft Word",
      title: "Bérjegyzék_2026_október.docx",
      url: null,
      path: "~/Documents/HR/Bérjegyzék_2026_október.docx",
    },
    category: "invoices",
    via: { kind: "file_glob", glob: "**/*bérjegyzék*" },
    dirty: true,
  },
  {
    id: 301,
    kind: "app_window",
    summary: {
      kind: "app_window",
      app_name: "1Password",
      title: "Personal – 1Password",
      url: null,
      path: null,
    },
    category: "password_managers",
    via: { kind: "app", app: "1Password" },
  },
  {
    id: 302,
    kind: "app_window",
    summary: { kind: "app_window", app_name: "Signal", title: "Signal", url: null, path: null },
    category: "messengers",
    via: { kind: "app", app: "Signal" },
  },
];

export const permissions: PermissionState[] = [
  { permission: { kind: "accessibility" }, status: "granted" },
  {
    permission: { kind: "automation", bundle_id: "com.google.Chrome", app_name: "Google Chrome" },
    status: "not_determined",
  },
  {
    permission: { kind: "automation", bundle_id: "com.apple.Safari", app_name: "Safari" },
    status: "granted",
  },
  {
    permission: { kind: "automation", bundle_id: "company.thebrowser.Browser", app_name: "Arc" },
    status: "app_not_running",
  },
  {
    permission: { kind: "automation", bundle_id: "com.apple.Preview", app_name: "Preview" },
    status: "granted",
  },
  {
    permission: { kind: "automation", bundle_id: "com.microsoft.Word", app_name: "Microsoft Word" },
    status: "app_not_running",
  },
  { permission: { kind: "notifications" }, status: "not_determined" },
];

export const signalSources: SignalSourceStatus[] = [
  { id: "share_watcher", health: { state: "ok" } },
  { id: "notify_proc", health: { state: "unavailable", reason_key: "reason.experimental_off" } },
  { id: "display", health: { state: "ok" } },
  { id: "call_audio", health: { state: "noisy", reason_key: "reason.watcher_noisy" } },
  { id: "power", health: { state: "ok" } },
  { id: "lock", health: { state: "ok" } },
];

const full: Support = { level: "full" };
const partial = (note_key: string): Support => ({ level: "partial", note_key });
const unsupported: Support = { level: "unsupported" };

function cap(
  provider: string,
  app: string | null,
  target_kind: TargetKindTag,
  action: Action,
  support: Support,
) {
  return { provider, app, target_kind, action, support };
}

export const capabilities: CapabilityReport = {
  platform: "macos",
  signal_sources: signalSources,
  capabilities: [
    cap("chromium", "Google Chrome", "tab", "close", full),
    cap("chromium", "Google Chrome", "tab", "hide", partial("note.tab_hide_minimizes")),
    cap("chromium", "Arc", "tab", "close", full),
    cap("safari", "Safari", "tab", "close", full),
    cap("safari", "Safari", "tab", "hide", partial("note.safari_private_unknown")),
    cap("firefox", "Firefox", "tab", "close", unsupported),
    cap("firefox", "Firefox", "tab", "hide", partial("note.firefox_session_file")),
    cap("documents", "Preview", "document", "close", full),
    cap("documents", "Microsoft Word", "document", "close", partial("note.sheet_by_role")),
    cap("documents", "Pages", "document", "close", partial("note.autosave_discard_hides")),
    cap("apps", null, "app_window", "hide", full),
    cap("apps", null, "app_window", "minimize", partial("note.minimize_animates")),
    cap("apps", null, "process", "quit", full),
  ],
};

function source(id: string, name: string, url: string, license: string, entries: number) {
  return { id, name, url, license, snapshot_date: "2026-11-09", entries };
}

const seed = (entries: number) =>
  source(
    "closecall-seed",
    "Close Call seed list",
    "https://github.com/futozs/closecall",
    "GPL-3.0-or-later",
    entries,
  );
const ut1 = (list: string, entries: number) =>
  source(`ut1-${list}`, `UT1 ${list}`, "https://dsi.ut-capitole.fr/blacklists/", "CC BY-SA 4.0", entries);

export const categories: CategoryInfo[] = [
  {
    id: "adult",
    builtin: true,
    domain_count: 141_206,
    app_count: 0,
    keyword_count: 0,
    sources: [
      source(
        "stevenblack-porn",
        "StevenBlack hosts, porn-only",
        "https://github.com/StevenBlack/hosts",
        "MIT",
        76_811,
      ),
      source("hagezi-nsfw", "HaGeZi NSFW", "https://github.com/hagezi/dns-blocklists", "GPL-3.0", 83_101),
    ],
  },
  {
    id: "banking",
    builtin: true,
    domain_count: 6_684,
    app_count: 0,
    keyword_count: 3,
    sources: [ut1("bank", 6_646), seed(38)],
  },
  {
    id: "webmail",
    builtin: true,
    domain_count: 431,
    app_count: 2,
    keyword_count: 0,
    sources: [ut1("webmail", 413), seed(18)],
  },
  { id: "medical", builtin: true, domain_count: 27, app_count: 0, keyword_count: 9, sources: [seed(27)] },
  {
    id: "financial",
    builtin: true,
    domain_count: 473,
    app_count: 0,
    keyword_count: 0,
    sources: [ut1("financial", 473)],
  },
  {
    id: "dating",
    builtin: true,
    domain_count: 10_564,
    app_count: 0,
    keyword_count: 0,
    sources: [ut1("dating", 10_564)],
  },
  {
    id: "gambling",
    builtin: true,
    domain_count: 47_823,
    app_count: 0,
    keyword_count: 0,
    sources: [ut1("gambling", 47_823)],
  },
  {
    id: "password_managers",
    builtin: true,
    domain_count: 0,
    app_count: 14,
    keyword_count: 0,
    sources: [seed(14)],
  },
  { id: "messengers", builtin: true, domain_count: 0, app_count: 16, keyword_count: 0, sources: [seed(16)] },
  { id: "invoices", builtin: true, domain_count: 0, app_count: 0, keyword_count: 11, sources: [seed(11)] },
  { id: "meeting", builtin: true, domain_count: 12, app_count: 9, keyword_count: 0, sources: [seed(21)] },
];

const minutes = (n: number) => n * 60_000;

export function seedActivity(now: number): Wire<RunReport>[] {
  return [
    {
      run_id: 2,
      at_ms: now - minutes(42),
      trigger: { kind: "screen_share", confidence: "high" },
      dry_run: false,
      restorable: true,
      steps: [
        {
          target: 302,
          summary: { kind: "app_window", app_name: "Signal", title: "Signal", url: null, path: null },
          action: "hide",
          phase: "conceal",
          because: [
            {
              rule: "share-hide-private-apps",
              evidence: { kind: "category", id: "messengers", via: { kind: "app", app: "Signal" } },
            },
          ],
          outcome: { kind: "done" },
        },
        {
          target: 104,
          summary: {
            kind: "tab",
            app_name: "Google Chrome",
            title: "Új inkognitólap",
            url: null,
            path: null,
          },
          action: "close",
          phase: "close_tabs",
          because: [{ rule: "share-close-sensitive-tabs", evidence: { kind: "private" } }],
          outcome: { kind: "done" },
        },
        {
          target: 105,
          summary: {
            kind: "tab",
            app_name: "Firefox",
            title: "Erste NetBank",
            url: "https://netbank.erstebank.hu/",
            path: null,
          },
          action: "hide",
          phase: "conceal",
          because: [
            {
              rule: "share-close-sensitive-tabs",
              evidence: { kind: "category", id: "banking", via: { kind: "domain", domain: "erstebank.hu" } },
            },
          ],
          outcome: { kind: "fell_back", action: "hide", reason_key: "reason.firefox_no_tab_close" },
        },
      ],
    },
    {
      run_id: 1,
      at_ms: now - minutes(60 * 20),
      trigger: { kind: "away", away: "15m" },
      dry_run: false,
      restorable: false,
      steps: [
        {
          target: 106,
          summary: {
            kind: "tab",
            app_name: "Safari",
            title: "Revolut",
            url: "https://app.revolut.com/home",
            path: null,
          },
          action: "close",
          phase: "close_tabs",
          because: [
            {
              rule: "away-close-banking",
              evidence: { kind: "category", id: "banking", via: { kind: "domain", domain: "revolut.com" } },
            },
          ],
          outcome: { kind: "done" },
        },
        {
          target: 107,
          summary: {
            kind: "tab",
            app_name: "Safari",
            title: "K&H e-bank",
            url: "https://ebank.kh.hu/",
            path: null,
          },
          action: "close",
          phase: "close_tabs",
          because: [
            {
              rule: "away-close-banking",
              evidence: { kind: "category", id: "banking", via: { kind: "domain", domain: "kh.hu" } },
            },
          ],
          outcome: { kind: "skipped", reason_key: "reason.changed_before_action" },
        },
      ],
    },
  ];
}
