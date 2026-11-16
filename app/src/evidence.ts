import type { TFunction } from "i18next";
import type { Evidence } from "./bindings/Evidence";
import type { Rule } from "./bindings/Rule";

export function categoryName(t: TFunction, id: string, userName?: string | null): string {
  return userName || t(`category.${id}.name`, { defaultValue: id });
}

export function describeEvidence(evidence: Evidence, t: TFunction): string {
  switch (evidence.kind) {
    case "category":
      return t("evidence.category", {
        category: categoryName(t, evidence.id),
        via: describeEvidence(evidence.via, t),
      });
    case "domain":
      return t("evidence.domain", { domain: evidence.domain });
    case "url_glob":
      return t("evidence.url_glob", { glob: evidence.glob });
    case "title_contains":
      return t("evidence.title_contains", { keyword: evidence.keyword });
    case "app":
      return t("evidence.app", { app: evidence.app });
    case "file_glob":
      return t("evidence.file_glob", { glob: evidence.glob });
    case "doc_kind":
      return t("evidence.doc_kind", { kind: t(`doc_kind.${evidence.doc_kind}`) });
    case "private":
      return t("evidence.private");
  }
}

/** Preset rules are named with a message key (`preset.<id>`), not prose; that is no custom name. */
export function customName(name: string | null): string | null {
  return name?.startsWith("preset.") ? null : name;
}

export function ruleName(t: TFunction, rule: Pick<Rule, "id" | "name">): string {
  const custom = customName(rule.name);
  if (custom) return custom;
  return t(rule.name ?? `preset.${rule.id}`, { defaultValue: rule.id });
}

/** Keys from Rust come either bare (`invalid_glob`) or already namespaced. */
export function issueMessage(t: TFunction, code: string, field: string): string {
  const key = code.startsWith("config.error.") ? code : `config.error.${code}`;
  return t(key, { defaultValue: t("config.error.unknown", { field, code }) });
}

export function keyedMessage(t: TFunction, key: string, namespace: "reason" | "note"): string {
  const full = key.startsWith(`${namespace}.`) ? key : `${namespace}.${key}`;
  return t(full, { defaultValue: t(`${namespace}.unknown`, { key }) });
}
