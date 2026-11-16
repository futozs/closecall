import type { Action } from "./bindings/Action";
import type { AppSelector } from "./bindings/AppSelector";
import type { Dur } from "./bindings/Dur";
import type { Matcher } from "./bindings/Matcher";
import type { Rule } from "./bindings/Rule";
import type { TargetKindTag } from "./bindings/TargetKindTag";
import type { Trigger } from "./bindings/Trigger";

export const SIMPLE_TRIGGERS = [
  "panic",
  "screen_share",
  "display_mirrored",
  "call_likely",
  "will_sleep",
  "lid_closed",
  "locked",
] as const;
export const TIMED_TRIGGERS = ["away_longer_than", "idle_longer_than", "item_open_longer_than"] as const;

export type SimpleTrigger = (typeof SIMPLE_TRIGGERS)[number];
export type TimedTrigger = (typeof TIMED_TRIGGERS)[number];
export type TriggerKind = SimpleTrigger | TimedTrigger;

export const TARGET_KINDS: TargetKindTag[] = ["tab", "document", "app_window", "process"];
export const ACTIONS: Action[] = ["close", "hide", "minimize", "quit", "notify_only"];

export function triggerKind(trigger: Trigger): TriggerKind {
  if (typeof trigger === "string") return trigger;
  if ("away_longer_than" in trigger) return "away_longer_than";
  if ("idle_longer_than" in trigger) return "idle_longer_than";
  return "item_open_longer_than";
}

export function triggerDur(trigger: Trigger): Dur | null {
  if (typeof trigger === "string") return null;
  if ("away_longer_than" in trigger) return trigger.away_longer_than;
  if ("idle_longer_than" in trigger) return trigger.idle_longer_than;
  return trigger.item_open_longer_than;
}

export function timedTrigger(kind: TimedTrigger, dur: Dur): Trigger {
  switch (kind) {
    case "away_longer_than":
      return { away_longer_than: dur };
    case "idle_longer_than":
      return { idle_longer_than: dur };
    case "item_open_longer_than":
      return { item_open_longer_than: dur };
  }
}

export function isTimed(kind: TriggerKind): kind is TimedTrigger {
  return (TIMED_TRIGGERS as readonly string[]).includes(kind);
}

/** Mirrors `trigger_allows_confirm` in crates/core/src/plan.rs. */
export function triggerAllowsConfirm(kind: TriggerKind): boolean {
  return kind === "item_open_longer_than" || kind === "idle_longer_than";
}

export const MATCHER_KINDS = [
  "category",
  "domain",
  "url_glob",
  "title_contains",
  "app",
  "file_glob",
  "doc_kind",
  "private",
  "any",
  "all",
  "not",
] as const;
export type MatcherKind = (typeof MATCHER_KINDS)[number];

export function matcherKind(matcher: Matcher): MatcherKind {
  if (matcher === "private") return "private";
  if ("category" in matcher) return "category";
  if ("domain" in matcher) return "domain";
  if ("url_glob" in matcher) return "url_glob";
  if ("title_contains" in matcher) return "title_contains";
  if ("app" in matcher) return "app";
  if ("file_glob" in matcher) return "file_glob";
  if ("doc_kind" in matcher) return "doc_kind";
  if ("any" in matcher) return "any";
  if ("all" in matcher) return "all";
  return "not";
}

export function defaultMatcher(kind: MatcherKind, firstCategory = "banking"): Matcher {
  switch (kind) {
    case "category":
      return { category: firstCategory };
    case "domain":
      return { domain: "" };
    case "url_glob":
      return { url_glob: "" };
    case "title_contains":
      return { title_contains: "" };
    case "app":
      return { app: { name: "" } };
    case "file_glob":
      return { file_glob: "" };
    case "doc_kind":
      return { doc_kind: "pdf" };
    case "private":
      return "private";
    case "any":
      return { any: [{ category: firstCategory }] };
    case "all":
      return { all: [{ category: firstCategory }] };
    case "not":
      return { not: { category: firstCategory } };
  }
}

export type AppSelectorKind = "bundle_id" | "exe" | "desktop_id" | "name";

export function appSelectorParts(selector: AppSelector): { kind: AppSelectorKind; value: string } {
  if ("bundle_id" in selector) return { kind: "bundle_id", value: selector.bundle_id };
  if ("exe" in selector) return { kind: "exe", value: selector.exe };
  if ("desktop_id" in selector) return { kind: "desktop_id", value: selector.desktop_id };
  return { kind: "name", value: selector.name };
}

export function appSelector(kind: AppSelectorKind, value: string): AppSelector {
  switch (kind) {
    case "bundle_id":
      return { bundle_id: value };
    case "exe":
      return { exe: value };
    case "desktop_id":
      return { desktop_id: value };
    case "name":
      return { name: value };
  }
}

/** A reverse-DNS line becomes a bundle id, anything else an app name. */
export function appSelectorFromLine(line: string): AppSelector {
  return /^[a-z0-9-]+(\.[a-z0-9-]+)+$/i.test(line) ? { bundle_id: line } : { name: line };
}

export function categoriesIn(matcher: Matcher | null): string[] {
  if (matcher === null || matcher === "private") return [];
  if ("category" in matcher) return [matcher.category];
  if ("any" in matcher) return matcher.any.flatMap(categoriesIn);
  if ("all" in matcher) return matcher.all.flatMap(categoriesIn);
  if ("not" in matcher) return categoriesIn(matcher.not);
  return [];
}

export function rulesUsingCategory(rules: Rule[], id: string): Rule[] {
  return rules.filter(
    (rule) => categoriesIn(rule.match).includes(id) || categoriesIn(rule.except).includes(id),
  );
}

export function newRuleId(existing: Rule[]): string {
  let n = existing.length + 1;
  while (existing.some((rule) => rule.id === `rule-${n}`)) n += 1;
  return `rule-${n}`;
}

export function splitLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export function slug(name: string): string {
  return name
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}
