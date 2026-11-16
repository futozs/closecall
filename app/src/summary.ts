import type { TFunction } from "i18next";
import type { CategoryDef } from "./bindings/CategoryDef";
import type { Matcher } from "./bindings/Matcher";
import type { Rule } from "./bindings/Rule";
import { categoryName } from "./evidence";
import { appSelectorParts } from "./model";
import { whenLabel } from "./triggers";

function list(items: string[], locale: string, type: "conjunction" | "disjunction"): string {
  return new Intl.ListFormat(locale, { style: "long", type }).format(items);
}

export function matcherSummary(
  t: TFunction,
  matcher: Matcher,
  locale: string,
  userCategories: CategoryDef[],
): string {
  if (matcher === "private") return t("matcher.summary.private");
  if ("category" in matcher) {
    const def = userCategories.find((candidate) => candidate.id === matcher.category);
    return categoryName(t, matcher.category, def?.name);
  }
  if ("domain" in matcher) return matcher.domain;
  if ("url_glob" in matcher) return matcher.url_glob;
  if ("title_contains" in matcher)
    return t("matcher.summary.title_contains", { keyword: matcher.title_contains });
  if ("file_glob" in matcher) return matcher.file_glob;
  if ("doc_kind" in matcher)
    return t("matcher.summary.doc_kind", { kind: t(`doc_kind.${matcher.doc_kind}`) });
  if ("app" in matcher) return appSelectorParts(matcher.app).value;
  const inner = (items: Matcher[]) => items.map((item) => matcherSummary(t, item, locale, userCategories));
  if ("any" in matcher) return list(inner(matcher.any), locale, "disjunction");
  if ("all" in matcher) return list(inner(matcher.all), locale, "conjunction");
  return t("matcher.summary.not", { inner: matcherSummary(t, matcher.not, locale, userCategories) });
}

export function ruleSummary(t: TFunction, rule: Rule, locale: string): string {
  const text = t(`rules.summary.${rule.action}`, {
    when: list(
      rule.when.map((trigger) => whenLabel(t, trigger, locale)),
      locale,
      "disjunction",
    ),
    targets: list(
      rule.targets.map((kind) => t(`target_plural.${kind}`)),
      locale,
      "conjunction",
    ),
  });
  return text.charAt(0).toLocaleUpperCase(locale) + text.slice(1);
}
