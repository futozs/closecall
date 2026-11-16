import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { RunReport } from "../bindings/RunReport";
import { ChevronRight, PlayIcon } from "../components/icons";
import { PreflightList } from "../components/Preflight";
import { countsOf, RunSteps, useCountsText } from "../components/RunSteps";
import { Button, CheckMark, ErrorNote, Keys, Section } from "../components/ui";
import { formatWhen, hotkeyKeys, isMac } from "../format";
import * as ipc from "../ipc";
import { useAppData, useLoad } from "../store";
import { triggerLabel } from "../triggers";

type Flow =
  | { step: "idle" }
  | { step: "busy" }
  | { step: "preview"; report: RunReport }
  | { step: "result"; report: RunReport; title: string }
  | { step: "error" };

function ShareHeadline() {
  const { t } = useTranslation();
  const { status, locale } = useAppData();
  const { share } = status;
  const paused = status.paused_until_ms !== null && !share.active;
  const state = share.active ? "sharing" : share.mirrored_display ? "mirrored" : paused ? "paused" : "idle";
  const until = status.paused_until_ms === null ? "" : formatWhen(status.paused_until_ms, locale);
  const live = state === "sharing" || state === "mirrored";
  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <p
          className={`flex items-center gap-2 text-sm font-medium ${
            live ? "-ml-2.5 rounded-full bg-signal-wash px-2.5 py-1 text-signal-text" : "text-ink-2"
          }`}
        >
          <span
            aria-hidden="true"
            className={`size-2 rounded-full ${live ? "bg-signal" : state === "paused" ? "bg-caution" : "bg-ok"}`}
          />
          {t(`home.state.${state}`)}
          {share.active && share.confidence && (
            <span className="font-normal">
              · {t("home.confidence", { level: t(`confidence.${share.confidence}`) })}
            </span>
          )}
        </p>
        <PauseControls />
      </div>
      <h1 className="mt-3 text-2xl font-semibold tracking-tight">
        {t(`home.title.${state}`, { time: until })}
      </h1>
      <p className="mt-2 max-w-prose text-ink-2">{t(`home.body.${state}`)}</p>
      {share.call_likely && !share.active && <p className="mt-2 text-sm text-ink">{t("home.call_likely")}</p>}
    </div>
  );
}

function PauseControls() {
  const { t } = useTranslation();
  const { status, setStatus } = useAppData();
  const act = (call: () => Promise<typeof status>) => () => void call().then(setStatus);
  if (status.paused_until_ms !== null) {
    return (
      <Button variant="secondary" size="sm" onClick={act(ipc.resume)}>
        {t("home.resume")}
      </Button>
    );
  }
  return (
    <fieldset className="flex items-center gap-1">
      <legend className="sr-only">{t("home.pause_label")}</legend>
      <Button variant="ghost" size="sm" onClick={act(() => ipc.pause(15))}>
        {t("home.pause_15")}
      </Button>
      <Button variant="ghost" size="sm" onClick={act(() => ipc.pause(60))}>
        {t("home.pause_60")}
      </Button>
    </fieldset>
  );
}

export function Home() {
  const { t } = useTranslation();
  const { status, config, locale } = useAppData();
  const countsText = useCountsText();
  const [flow, setFlow] = useState<Flow>({ step: "idle" });
  const rightNow = useLoad(
    () => ipc.preview({ kind: "panic" }),
    `${status.last_run?.run_id}-${JSON.stringify(config)}`,
  );
  const mac = isMac(status.platform);
  const hotkey = config.panic_hotkey ? hotkeyKeys(config.panic_hotkey, mac) : null;
  const shareTrigger = { kind: "screen_share", confidence: "high" } as const;

  const cleanNow = async () => {
    setFlow({ step: "busy" });
    try {
      setFlow({ step: "result", report: await ipc.runPanic(), title: t("home.panic_done") });
    } catch {
      setFlow({ step: "error" });
    }
  };
  const previewShare = async () => {
    setFlow({ step: "busy" });
    try {
      setFlow({ step: "preview", report: await ipc.preview(shareTrigger) });
    } catch {
      setFlow({ step: "error" });
    }
  };
  const simulateShare = async () => {
    setFlow({ step: "busy" });
    try {
      setFlow({ step: "result", report: await ipc.simulate(shareTrigger), title: t("home.sim.done") });
    } catch {
      setFlow({ step: "error" });
    }
  };

  const sensitive = rightNow.data?.steps.length ?? 0;
  const last = status.last_run;

  return (
    <div>
      <ShareHeadline />

      <div className="mt-6 flex flex-wrap items-center gap-x-3 gap-y-3">
        <Button
          variant="signal"
          size="lg"
          className="on-signal"
          onClick={cleanNow}
          disabled={flow.step === "busy"}
        >
          {t("home.clean_now")}
          {hotkey && <Keys keys={hotkey} label={config.panic_hotkey ?? undefined} />}
        </Button>
        <Button variant="secondary" size="lg" onClick={previewShare} disabled={flow.step === "busy"}>
          <PlayIcon className="size-4" />
          {t("home.simulate")}
        </Button>
      </div>

      <div aria-live="polite">
        {flow.step === "preview" && (
          <div className="reveal mt-6 rounded-panel border border-rule-strong bg-surface p-5">
            <h2 className="text-md font-semibold">{t("home.sim.title")}</h2>
            <p className="mt-1 text-sm text-ink-2">{t("home.sim.lead")}</p>
            {flow.report.steps.length > 0 ? (
              <RunSteps report={flow.report} rules={config.rule} />
            ) : (
              <p className="mt-3">{t("home.sim.nothing")}</p>
            )}
            <div className="mt-4 flex flex-wrap gap-2">
              {flow.report.steps.length > 0 && (
                <Button variant="primary" onClick={simulateShare}>
                  {t("home.sim.run")}
                </Button>
              )}
              <Button variant="ghost" onClick={() => setFlow({ step: "idle" })}>
                {t("common.cancel")}
              </Button>
            </div>
          </div>
        )}
        {flow.step === "result" && (
          <div className="reveal mt-6 rounded-panel border border-rule-strong bg-surface p-5">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <h2 className="text-md font-semibold">{flow.title}</h2>
              <p className="data text-sm text-ink-2">{countsText(countsOf(flow.report))}</p>
            </div>
            {flow.report.steps.length > 0 && <RunSteps report={flow.report} rules={config.rule} />}
            <div className="mt-4 flex flex-wrap gap-2">
              <a href="#activity" className="btn btn-secondary">
                {t("home.see_activity")}
              </a>
              <Button variant="ghost" onClick={() => setFlow({ step: "idle" })}>
                {t("common.dismiss")}
              </Button>
            </div>
          </div>
        )}
        {flow.step === "error" && (
          <div className="mt-6">
            <ErrorNote>{t("common.error")}</ErrorNote>
          </div>
        )}
      </div>

      <Section title={t("home.open_now")} lead={t("home.open_now_lead")}>
        {rightNow.data && (
          <>
            <p className="flex items-center gap-2.5 font-medium">
              <CheckMark state={sensitive > 0 ? "alert" : "ok"} />
              <span className={sensitive > 0 ? "text-signal-text" : ""}>
                {sensitive > 0 ? t("home.sensitive", { count: sensitive }) : t("home.sensitive_none")}
              </span>
            </p>
            {sensitive > 0 && <RunSteps report={rightNow.data} rules={config.rule} />}
          </>
        )}
        {rightNow.error !== null && <ErrorNote>{t("common.error")}</ErrorNote>}
      </Section>

      <Section title={t("home.preflight")} lead={t("home.preflight_lead")}>
        <PreflightList />
      </Section>

      <Section
        title={t("home.last_run")}
        actions={
          <a href="#activity" className="inline-flex items-center gap-1 text-sm text-ink-2 hover:text-ink">
            {t("home.see_activity")}
            <ChevronRight className="size-3.5" />
          </a>
        }
      >
        {last ? (
          <p>
            <span className="font-medium">{triggerLabel(t, last.trigger, locale)}</span>
            <span className="data text-sm text-ink-2"> · {formatWhen(last.at_ms, locale)}</span>
            <span className="mt-0.5 block text-sm text-ink-2">{countsText(last)}</span>
          </p>
        ) : (
          <p className="text-ink-2">{t("home.last_run_none")}</p>
        )}
      </Section>
    </div>
  );
}
