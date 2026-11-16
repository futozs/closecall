import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Rule } from "../bindings/Rule";
import { ChevronRight, PlusIcon } from "../components/icons";
import { Button, EmptyState, ErrorNote, PageHeader } from "../components/ui";
import { ruleName } from "../evidence";
import { useSubpath } from "../hash";
import * as ipc from "../ipc";
import { newRuleId } from "../model";
import { useAppData } from "../store";
import { matcherSummary, ruleSummary } from "../summary";
import { blankRule, RuleEditor } from "./RuleEditor";

function RuleRow({ rule }: { rule: Rule }) {
  const { t } = useTranslation();
  const { config, saveConfig, locale } = useAppData();
  const [failed, setFailed] = useState(false);
  const name = ruleName(t, rule);
  const toggle = async () => {
    const next = { ...rule, enabled: !rule.enabled };
    const issues = await saveConfig({
      ...config,
      rule: config.rule.map((r) => (r.id === rule.id ? next : r)),
    });
    setFailed(issues.length > 0);
  };
  return (
    <li className="row-hover flex items-start gap-4 rounded-ctl px-3 py-3.5">
      <button
        type="button"
        role="switch"
        aria-checked={rule.enabled}
        aria-label={t("rules.enabled_label", { name })}
        className="switch mt-0.5"
        onClick={() => void toggle()}
      />
      <a href={`#rules/edit/${encodeURIComponent(rule.id)}`} className="group min-w-0 flex-1 no-underline">
        <span className={`block font-medium ${rule.enabled ? "" : "text-ink-2"}`}>{name}</span>
        <span className="mt-0.5 block text-sm text-ink-2">{ruleSummary(t, rule, locale)}</span>
        <span className="mt-0.5 block truncate text-sm text-ink-3">
          {t("rules.matches", { what: matcherSummary(t, rule.match, locale, config.categories) })}
          {rule.except &&
            ` ${t("rules.except", { what: matcherSummary(t, rule.except, locale, config.categories) })}`}
        </span>
        {failed && (
          <span className="mt-1 block text-sm text-signal-text" role="alert">
            {t("rules.toggle_failed")}
          </span>
        )}
      </a>
      <ChevronRight className="mt-1 size-4 flex-none text-ink-3" />
    </li>
  );
}

function ResetRules() {
  const { t } = useTranslation();
  const { setConfig } = useAppData();
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState(false);
  const reset = async () => {
    try {
      setConfig(await ipc.resetConfig("recommended"));
      setConfirming(false);
    } catch {
      setError(true);
    }
  };
  if (!confirming) {
    return (
      <Button variant="ghost" onClick={() => setConfirming(true)}>
        {t("rules.reset")}
      </Button>
    );
  }
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-sm">{t("rules.reset_confirm")}</span>
      <Button variant="secondary" size="sm" onClick={() => void reset()}>
        {t("rules.reset_yes")}
      </Button>
      <Button variant="ghost" size="sm" onClick={() => setConfirming(false)}>
        {t("common.cancel")}
      </Button>
      {error && <ErrorNote>{t("common.error")}</ErrorNote>}
    </div>
  );
}

export function Rules() {
  const { t } = useTranslation();
  const { config, categories, setConfig } = useAppData();
  const [mode, id] = useSubpath();
  const firstCategory = categories[0]?.id ?? "banking";

  if (mode === "new") {
    return <RuleEditor key="new" initial={blankRule(newRuleId(config.rule), firstCategory)} isNew />;
  }
  const editing = mode === "edit" ? config.rule.find((rule) => rule.id === id) : undefined;
  if (editing) return <RuleEditor key={editing.id} initial={editing} isNew={false} />;

  const active = config.rule.filter((rule) => rule.enabled).length;
  return (
    <div>
      <PageHeader
        title={t("rules.title")}
        lead={t("rules.lead")}
        actions={
          <a href="#rules/new" className="btn btn-primary">
            <PlusIcon className="size-4" />
            {t("rules.new")}
          </a>
        }
      />
      {config.rule.length === 0 ? (
        <EmptyState
          title={t("rules.empty_title")}
          body={t("rules.empty_body")}
          action={
            <div className="flex flex-wrap gap-2">
              <Button variant="primary" onClick={() => void ipc.resetConfig("recommended").then(setConfig)}>
                {t("rules.use_recommended")}
              </Button>
              <a href="#rules/new" className="btn btn-secondary">
                {t("rules.new")}
              </a>
            </div>
          }
        />
      ) : (
        <>
          <p className="data mb-2 text-sm text-ink-2">
            {t("rules.count", { count: config.rule.length, active })}
          </p>
          <ul className="-mx-3 divide-y divide-rule border-y border-rule">
            {config.rule.map((rule) => (
              <RuleRow key={rule.id} rule={rule} />
            ))}
          </ul>
          <div className="mt-6">
            <ResetRules />
          </div>
        </>
      )}
    </div>
  );
}
