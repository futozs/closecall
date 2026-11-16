import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppRef } from "../bindings/AppRef";
import type { Preset } from "../bindings/Preset";
import { ExternalIcon, Mark } from "../components/icons";
import { PermissionRow, permissionKey } from "../components/Preflight";
import { RunSteps } from "../components/RunSteps";
import { Button, CheckMark, CheckRow, Choice, ErrorNote, Switch } from "../components/ui";
import { go } from "../hash";
import { type Locale, setLocale, systemLocale } from "../i18n";
import * as ipc from "../ipc";
import { appSelectorMatches, KNOWN_CAPTURE_APPS } from "../model";
import { useAppData, useLoad } from "../store";

const SOURCE_URL = "https://github.com/futozs/closecall";
const MAC_LINKS = {
  notifications: "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
  lock: "x-apple.systempreferences:com.apple.Lock-Screen-Settings.extension",
};

type StepId =
  | "welcome"
  | "accessibility"
  | "automation"
  | "notifications"
  | "capture"
  | "os"
  | "dry_run"
  | "login";

function StepFrame({ title, body, children }: { title: ReactNode; body?: ReactNode; children?: ReactNode }) {
  return (
    <div className="reveal">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      {body && <p className="mt-3 max-w-prose text-md text-ink-2">{body}</p>}
      {children && <div className="mt-8">{children}</div>}
    </div>
  );
}

function Welcome({
  locale,
  onLocale,
  preset,
  onPreset,
}: {
  locale: Locale;
  onLocale: (locale: Locale) => void;
  preset: Preset;
  onPreset: (preset: Preset) => void;
}) {
  const { t } = useTranslation();
  return (
    <StepFrame title={t("onboarding.welcome.title")} body={t("onboarding.welcome.body")}>
      <div className="rounded-panel border border-rule-strong bg-surface p-5">
        <h2 className="font-semibold">{t("onboarding.welcome.promise_title")}</h2>
        <p className="mt-1 max-w-prose text-ink-2">{t("onboarding.welcome.promise_body")}</p>
        <Button size="sm" className="mt-3" onClick={() => void ipc.openUrl(SOURCE_URL)}>
          {t("onboarding.welcome.source")}
          <ExternalIcon className="size-3.5" />
        </Button>
      </div>
      <div className="mt-8 grid gap-8 sm:grid-cols-[minmax(0,12rem)_minmax(0,1fr)]">
        <fieldset>
          <legend className="mb-2 font-semibold">{t("onboarding.welcome.language")}</legend>
          <Choice
            type="radio"
            name="lang"
            label="English"
            checked={locale === "en"}
            onChange={() => onLocale("en")}
          />
          <Choice
            type="radio"
            name="lang"
            label="Magyar"
            checked={locale === "hu"}
            onChange={() => onLocale("hu")}
          />
        </fieldset>
        <fieldset>
          <legend className="mb-2 font-semibold">{t("onboarding.welcome.preset")}</legend>
          <Choice
            type="radio"
            name="preset"
            label={t("onboarding.welcome.preset_recommended")}
            description={t("onboarding.welcome.preset_recommended_hint")}
            checked={preset === "recommended"}
            onChange={() => onPreset("recommended")}
          />
          <Choice
            type="radio"
            name="preset"
            label={t("onboarding.welcome.preset_empty")}
            description={t("onboarding.welcome.preset_empty_hint")}
            checked={preset === "empty"}
            onChange={() => onPreset("empty")}
          />
        </fieldset>
      </div>
    </StepFrame>
  );
}

function Accessibility() {
  const { t } = useTranslation();
  const { status, setStatus } = useAppData();
  const [waiting, setWaiting] = useState(false);
  const state = status.permissions.find((entry) => entry.permission.kind === "accessibility");
  const granted = state?.status === "granted";

  useEffect(() => {
    if (!waiting || granted) return;
    // macOS has no callback for this grant, so poll the way System Settings expects.
    const timer = window.setInterval(() => void ipc.getStatus().then(setStatus), 1_000);
    return () => window.clearInterval(timer);
  }, [waiting, granted, setStatus]);

  return (
    <StepFrame title={t("onboarding.accessibility.title")} body={t("onboarding.accessibility.body")}>
      {granted ? (
        <p className="flex items-center gap-2.5 font-medium">
          <CheckMark state="ok" />
          {t("onboarding.accessibility.granted")}
        </p>
      ) : (
        <div className="grid justify-items-start gap-3">
          <Button
            variant="primary"
            size="lg"
            onClick={() => {
              setWaiting(true);
              void ipc.requestPermission({ kind: "accessibility" });
            }}
          >
            {t("onboarding.accessibility.button")}
          </Button>
          {waiting && (
            <p className="text-sm text-ink-2" role="status">
              {t("onboarding.accessibility.waiting")}
            </p>
          )}
        </div>
      )}
    </StepFrame>
  );
}

function PermissionStep({ step, kind }: { step: StepId; kind: "automation" | "notifications" }) {
  const { t } = useTranslation();
  const { status } = useAppData();
  const states = status.permissions.filter(
    (state) => state.permission.kind === kind && state.status !== "not_applicable",
  );
  return (
    <StepFrame title={t(`onboarding.${step}.title`)} body={t(`onboarding.${step}.body`)}>
      <ul className="max-w-xl">
        {states.map((state) => (
          <PermissionRow key={permissionKey(state)} state={state} />
        ))}
      </ul>
    </StepFrame>
  );
}

/** Offered only while a known always-on capturer runs, such as the agent of a DisplayLink dock. */
function Capture({ app }: { app: AppRef }) {
  const { t } = useTranslation();
  const { config, saveConfig } = useAppData();
  const [failed, setFailed] = useState(false);
  const { ignore } = config;
  const ignored = ignore.capture_apps.some((selector) => appSelectorMatches(selector, app));
  const set = async (on: boolean) => {
    const capture_apps = on
      ? [...ignore.capture_apps, { bundle_id: app.id }]
      : ignore.capture_apps.filter((selector) => !appSelectorMatches(selector, app));
    setFailed(false);
    try {
      await saveConfig({ ...config, ignore: { ...ignore, capture_apps } });
    } catch {
      setFailed(true);
    }
  };
  return (
    <StepFrame title={t("onboarding.capture.title", { app: app.name })} body={t("onboarding.capture.body")}>
      <div className="max-w-md">
        <Switch
          label={t("onboarding.capture.toggle")}
          description={t("onboarding.capture.toggle_hint")}
          checked={ignored}
          onChange={(on) => void set(on)}
        />
      </div>
      {failed && <ErrorNote>{t("common.error")}</ErrorNote>}
    </StepFrame>
  );
}

function OsSettings() {
  const { t } = useTranslation();
  const { status } = useAppData();
  const [opened, setOpened] = useState<string[]>([]);
  const mac = status.platform === "macos";
  const open = (url: string) => (
    <Button
      size="sm"
      onClick={() => {
        setOpened((current) => [...current, url]);
        void ipc.openUrl(url);
      }}
    >
      {t("onboarding.os.open")}
      <ExternalIcon className="size-3.5" />
    </Button>
  );
  const item = (label: string, hint: string, url?: string) => (
    <CheckRow
      state={url && opened.includes(url) ? "ok" : "open"}
      label={label}
      detail={hint}
      action={mac && url ? open(url) : undefined}
    />
  );
  return (
    <StepFrame title={t("onboarding.os.title")} body={t("onboarding.os.body")}>
      <ul className="max-w-xl">
        {item(t("onboarding.os.dnd"), t("onboarding.os.dnd_hint"), MAC_LINKS.notifications)}
        {item(t("onboarding.os.password"), t("onboarding.os.password_hint"), MAC_LINKS.lock)}
        {item(t("onboarding.os.window_share"), t("onboarding.os.window_share_hint"))}
      </ul>
    </StepFrame>
  );
}

function DryRun() {
  const { t } = useTranslation();
  const { config } = useAppData();
  const [round, setRound] = useState(0);
  const report = useLoad(() => ipc.preview({ kind: "screen_share", confidence: "high" }), round);
  return (
    <StepFrame title={t("onboarding.dry_run.title")} body={t("onboarding.dry_run.body")}>
      {report.error !== null && <ErrorNote>{t("common.error")}</ErrorNote>}
      {report.data &&
        (report.data.steps.length > 0 ? (
          <RunSteps report={report.data} rules={config.rule} />
        ) : (
          <p className="flex items-center gap-2.5">
            <CheckMark state="ok" />
            {t("onboarding.dry_run.nothing")}
          </p>
        ))}
      <Button className="mt-4" size="sm" onClick={() => setRound((n) => n + 1)} disabled={report.loading}>
        {t("onboarding.dry_run.again")}
      </Button>
    </StepFrame>
  );
}

function Login({ enabled, onChange }: { enabled: boolean; onChange: (enabled: boolean) => void }) {
  const { t } = useTranslation();
  return (
    <StepFrame title={t("onboarding.login.title")} body={t("onboarding.login.body")}>
      <div className="max-w-md">
        <Switch label={t("onboarding.login.toggle")} checked={enabled} onChange={onChange} />
      </div>
    </StepFrame>
  );
}

export function Onboarding() {
  const { t } = useTranslation();
  const { status, config, saveConfig } = useAppData();
  const system = systemLocale();
  const [index, setIndex] = useState(0);
  const [locale, setLocaleChoice] = useState<Locale>(config.language === "system" ? system : config.language);
  const [preset, setPreset] = useState<Preset>("recommended");
  const [login, setLogin] = useState(true);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const running = useLoad(ipc.listRunningApps);
  const capturer = running.data?.find((app) => KNOWN_CAPTURE_APPS.includes(app.id));

  const has = (kind: string) =>
    status.permissions.some((state) => state.permission.kind === kind && state.status !== "not_applicable");
  const steps: StepId[] = [
    "welcome",
    ...(has("accessibility") ? (["accessibility"] as const) : []),
    ...(has("automation") ? (["automation"] as const) : []),
    ...(has("notifications") ? (["notifications"] as const) : []),
    ...(capturer ? (["capture"] as const) : []),
    "os",
    "dry_run",
    "login",
  ];
  const current = steps[index] ?? "welcome";
  const last = index === steps.length - 1;

  const chooseLocale = (next: Locale) => {
    setLocaleChoice(next);
    void setLocale(next);
  };

  const next = async () => {
    setFailed(false);
    setBusy(true);
    try {
      if (current === "welcome") {
        const fresh = await ipc.resetConfig(preset);
        const withLanguage = { ...fresh, language: locale === system ? ("system" as const) : locale };
        await saveConfig(withLanguage);
      }
      if (last) {
        await ipc.setAutostart(login);
        go("home");
        return;
      }
      setIndex(index + 1);
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-dvh sm:grid sm:grid-cols-[16rem_minmax(0,1fr)]">
      <aside className="border-b border-rule bg-side px-5 py-5 sm:sticky sm:top-0 sm:h-dvh sm:border-r sm:border-b-0 sm:px-6 sm:py-8">
        <div className="flex items-center gap-2.5">
          <Mark className="size-8" />
          <div className="leading-tight">
            <p className="font-semibold">{t("app.name")}</p>
            <p className="text-xs text-ink-2">{t("app.tagline")}</p>
          </div>
        </div>
        <p className="data mt-4 text-sm text-ink-2 sm:hidden">
          {t("onboarding.progress", { current: index + 1, total: steps.length })} ·{" "}
          {t(`onboarding.steps.${current}`)}
        </p>
        <ol className="mt-8 hidden gap-1 sm:grid" aria-label={t("onboarding.title")}>
          {steps.map((step, i) => (
            <li
              key={step}
              aria-current={i === index ? "step" : undefined}
              className={`flex items-center gap-3 rounded-ctl px-2 py-1.5 ${
                i === index ? "bg-hover font-semibold text-ink" : i < index ? "text-ink" : "text-ink-3"
              }`}
            >
              <CheckMark state={i < index ? "ok" : "open"} />
              {t(`onboarding.steps.${step}`)}
            </li>
          ))}
        </ol>
      </aside>
      <main className="flex min-h-[calc(100dvh-8rem)] flex-col px-5 pt-8 pb-6 sm:min-h-dvh sm:px-12 sm:pt-14">
        <div className="w-full max-w-[44rem] flex-1">
          {current === "welcome" && (
            <Welcome locale={locale} onLocale={chooseLocale} preset={preset} onPreset={setPreset} />
          )}
          {current === "accessibility" && <Accessibility />}
          {current === "automation" && <PermissionStep step="automation" kind="automation" />}
          {current === "notifications" && <PermissionStep step="notifications" kind="notifications" />}
          {current === "capture" && capturer && <Capture app={capturer} />}
          {current === "os" && <OsSettings />}
          {current === "dry_run" && <DryRun />}
          {current === "login" && <Login enabled={login} onChange={setLogin} />}
          {failed && (
            <div className="mt-6">
              <ErrorNote>{t("common.error")}</ErrorNote>
            </div>
          )}
        </div>
        <div className="mt-10 flex w-full max-w-[44rem] flex-wrap items-center gap-2 border-t border-rule pt-5">
          {index > 0 && (
            <Button variant="ghost" onClick={() => setIndex(index - 1)} disabled={busy}>
              {t("onboarding.back")}
            </Button>
          )}
          <Button variant="primary" size="lg" className="ml-auto" onClick={() => void next()} disabled={busy}>
            {last ? t("onboarding.finish") : t("onboarding.next")}
          </Button>
        </div>
      </main>
    </div>
  );
}
