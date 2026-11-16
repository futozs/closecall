import type { Config } from "../bindings/Config";
import type { Matcher } from "../bindings/Matcher";
import type { ValidationIssue } from "../bindings/ValidationIssue";
import { matcherKind, triggerAllowsConfirm, triggerKind } from "../model";

const DOMAIN = /^(?!-)[\p{L}\p{N}-]{1,63}(\.[\p{L}\p{N}-]{1,63})+$/u;

function balanced(glob: string): boolean {
  let square = 0;
  let curly = 0;
  for (const char of glob) {
    if (char === "[") square += 1;
    if (char === "]") square -= 1;
    if (char === "{") curly += 1;
    if (char === "}") curly -= 1;
    if (square < 0 || curly < 0) return false;
  }
  return square === 0 && curly === 0 && glob.trim() !== "";
}

function matcherIssues(matcher: Matcher, field: string, rule: string): ValidationIssue[] {
  const issue = (code: string, at = field): ValidationIssue => ({ rule, field: at, code });
  const kind = matcherKind(matcher);
  if (matcher === "private") return [];
  if ("any" in matcher || "all" in matcher) {
    const children = "any" in matcher ? matcher.any : matcher.all;
    if (children.length === 0) return [issue("empty_composite")];
    return children.flatMap((child, i) => matcherIssues(child, `${field}.${kind}[${i}]`, rule));
  }
  if ("not" in matcher) return matcherIssues(matcher.not, `${field}.not`, rule);
  if ("domain" in matcher) return DOMAIN.test(matcher.domain) ? [] : [issue("invalid_domain")];
  if ("url_glob" in matcher) return balanced(matcher.url_glob) ? [] : [issue("invalid_glob")];
  if ("file_glob" in matcher) return balanced(matcher.file_glob) ? [] : [issue("invalid_glob")];
  return [];
}

/** A stand-in for `Config::validate` in crates/core, close enough for the browser mock. */
export function validate(config: Config): ValidationIssue[] {
  if (config.version > 1) return [{ rule: null, field: "version", code: "unsupported_version" }];
  const issues: ValidationIssue[] = [];
  const seen = new Set<string>();
  for (const rule of config.rule) {
    if (seen.has(rule.id)) issues.push({ rule: rule.id, field: "id", code: "duplicate_rule_id" });
    seen.add(rule.id);
    if (rule.when.length === 0) issues.push({ rule: rule.id, field: "when", code: "empty_when" });
    if (rule.targets.length === 0) issues.push({ rule: rule.id, field: "targets", code: "empty_targets" });
    if (
      rule.confirm !== "never" &&
      !rule.when.every((trigger) => triggerAllowsConfirm(triggerKind(trigger)))
    ) {
      issues.push({ rule: rule.id, field: "confirm", code: "countdown_not_allowed" });
    }
    if (rule.match !== "private" && "not" in rule.match) {
      issues.push({ rule: rule.id, field: "match", code: "lone_not" });
    }
    issues.push(...matcherIssues(rule.match, "match", rule.id));
    if (rule.except) issues.push(...matcherIssues(rule.except, "except", rule.id));
  }
  return issues;
}
