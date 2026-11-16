import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { RunReport } from "../bindings/RunReport";
import { RestoreIcon } from "../components/icons";
import { countsOf, RunSteps, useCountsText } from "../components/RunSteps";
import { Button, EmptyState, ErrorNote, PageHeader } from "../components/ui";
import { formatDur, formatWhen } from "../format";
import * as ipc from "../ipc";
import { useAppData } from "../store";
import { triggerLabel } from "../triggers";

function Run({ report, onRestored }: { report: RunReport; onRestored: (report: RunReport) => void }) {
  const { t } = useTranslation();
  const { config, locale } = useAppData();
  const countsText = useCountsText();
  const [state, setState] = useState<"idle" | "busy" | "done" | "error">("idle");
  const restore = async () => {
    setState("busy");
    try {
      onRestored(await ipc.restore(report.run_id));
      setState("done");
    } catch {
      setState("error");
    }
  };
  return (
    <article className="border-t border-rule pt-5 pb-7">
      <header className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
        <div className="min-w-0">
          <h2 className="flex flex-wrap items-baseline gap-x-2 font-semibold">
            {triggerLabel(t, report.trigger, locale)}
            <span className="data text-sm font-normal text-ink-2">{formatWhen(report.at_ms, locale)}</span>
            {report.dry_run && (
              <span className="rounded-ctl border border-rule-strong px-1.5 text-xs font-medium text-ink-2">
                {t("activity.dry_run")}
              </span>
            )}
          </h2>
          <p className="mt-0.5 text-sm text-ink-2">{countsText(countsOf(report))}</p>
        </div>
        {report.restorable ? (
          <Button variant="secondary" size="sm" onClick={() => void restore()} disabled={state === "busy"}>
            <RestoreIcon className="size-3.5" />
            {t("activity.restore")}
          </Button>
        ) : (
          <span className="text-sm text-ink-3">
            {state === "done" ? t("activity.restored") : t("activity.not_restorable")}
          </span>
        )}
      </header>
      {state === "error" && (
        <div className="mt-3">
          <ErrorNote>{t("common.error")}</ErrorNote>
        </div>
      )}
      <RunSteps report={report} rules={config.rule} />
    </article>
  );
}

export function Activity() {
  const { t } = useTranslation();
  const { config, locale } = useAppData();
  const [runs, setRuns] = useState<RunReport[] | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    ipc.getActivity().then(setRuns, () => setFailed(true));
    const unlisten = ipc.onRun((report) => {
      if (report.dry_run) return;
      setRuns((current) =>
        current && !current.some((known) => known.run_id === report.run_id) ? [report, ...current] : current,
      );
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const replace = (next: RunReport) =>
    setRuns((current) => current?.map((report) => (report.run_id === next.run_id ? next : report)) ?? null);

  return (
    <div>
      <PageHeader
        title={t("activity.title")}
        lead={t("activity.lead", { duration: formatDur(config.restore_window, locale) })}
      />
      {failed && <ErrorNote>{t("common.error")}</ErrorNote>}
      {runs?.length === 0 && <EmptyState title={t("activity.empty_title")} body={t("activity.empty_body")} />}
      {runs?.map((report) => (
        <Run key={report.run_id} report={report} onRestored={replace} />
      ))}
    </div>
  );
}
