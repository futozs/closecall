import { useTranslation } from "react-i18next";
import type { Rule } from "../bindings/Rule";
import type { RunReport } from "../bindings/RunReport";
import type { StepReport } from "../bindings/StepReport";
import type { TargetKindTag } from "../bindings/TargetKindTag";
import { describeEvidence, keyedMessage, ruleName } from "../evidence";
import { hostOf } from "../format";
import { AppIcon, DocumentIcon, TabIcon, WindowIcon } from "./icons";

export function KindIcon({ kind, className }: { kind: TargetKindTag; className?: string }) {
  switch (kind) {
    case "tab":
      return <TabIcon className={className} />;
    case "document":
      return <DocumentIcon className={className} />;
    case "app_window":
      return <WindowIcon className={className} />;
    case "process":
      return <AppIcon className={className} />;
  }
}

export interface Counts {
  done: number;
  fell_back: number;
  skipped: number;
  failed: number;
}

export function countsOf(report: RunReport): Counts {
  const count = (kind: string) => report.steps.filter((step) => step.outcome?.kind === kind).length;
  return {
    done: count("done"),
    fell_back: count("fell_back"),
    skipped: count("skipped"),
    failed: count("failed"),
  };
}

export function useCountsText() {
  const { t } = useTranslation();
  return (counts: Counts) => {
    const parts = (["done", "fell_back", "skipped", "failed"] as const)
      .filter((key) => counts[key] > 0)
      .map((key) => t(`run.${key}`, { count: counts[key] }));
    return parts.length > 0 ? parts.join(" · ") : t("run.nothing");
  };
}

function Outcome({ step }: { step: StepReport }) {
  const { t } = useTranslation();
  const outcome = step.outcome;
  if (!outcome) {
    return <span className="text-sm font-medium">{t(`plan.${step.action}`)}</span>;
  }
  switch (outcome.kind) {
    case "done":
      return <span className="text-sm font-medium text-ok">{t(`action_past.${step.action}`)}</span>;
    case "fell_back":
      return (
        <span className="text-sm font-medium text-caution">
          {t("outcome.fell_back", { action: t(`action_past.${outcome.action}`) })}
        </span>
      );
    case "skipped":
      return <span className="text-sm font-medium text-caution">{t("outcome.skipped")}</span>;
    case "failed":
      return <span className="text-sm font-medium text-signal-text">{t("outcome.failed")}</span>;
  }
}

function outcomeNote(step: StepReport, t: ReturnType<typeof useTranslation>["t"]): string | null {
  const outcome = step.outcome;
  if (!outcome) return null;
  if (outcome.kind === "fell_back" || outcome.kind === "skipped")
    return keyedMessage(t, outcome.reason_key, "reason");
  if (outcome.kind === "failed") return outcome.error;
  return null;
}

function where(step: StepReport): string | null {
  if (step.summary.url) return hostOf(step.summary.url);
  return step.summary.path;
}

export function RunSteps({ report, rules }: { report: RunReport; rules: Rule[] }) {
  const { t } = useTranslation();
  const ruleById = new Map(rules.map((rule) => [rule.id, rule]));
  return (
    <ul className="divide-y divide-rule">
      {report.steps.map((step) => {
        const note = outcomeNote(step, t);
        const place = where(step);
        return (
          <li key={step.target} className="grid grid-cols-[auto_minmax(0,1fr)_auto] gap-x-3 py-3">
            <KindIcon kind={step.summary.kind} className="mt-0.5 size-4 text-ink-2" />
            <div className="min-w-0">
              <p className="truncate font-medium" title={step.summary.title}>
                {step.summary.title}
              </p>
              <p className="truncate text-sm text-ink-2">
                {step.summary.app_name}
                {place && (
                  <>
                    {" · "}
                    <span className="data">{place}</span>
                  </>
                )}
              </p>
              <p className="mt-1 text-sm text-ink-2">
                <span className="text-ink-3">{t("evidence.why")} </span>
                {step.because.map((reason, i) => {
                  const rule = ruleById.get(reason.rule);
                  return (
                    <span key={`${reason.rule}-${JSON.stringify(reason.evidence)}`}>
                      {i > 0 && "; "}
                      <span className="text-ink">{describeEvidence(reason.evidence, t)}</span>
                      {" · "}
                      {rule ? ruleName(t, rule) : reason.rule}
                    </span>
                  );
                })}
              </p>
              {note && <p className="mt-1 text-sm text-caution">{note}</p>}
            </div>
            <div className="pt-0.5 text-right">
              <Outcome step={step} />
            </div>
          </li>
        );
      })}
    </ul>
  );
}
