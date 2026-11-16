import { type ReactNode, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Capability } from "../bindings/Capability";
import type { Config } from "../bindings/Config";
import { CloseIcon, ExternalIcon } from "../components/icons";
import { PermissionRow, permissionKey, SourceRow } from "../components/Preflight";
import { Button, CheckRow, Choice, ErrorNote, Keys, PageHeader, Section, Switch } from "../components/ui";
import { categoryName, issueMessage, keyedMessage } from "../evidence";
import { acceleratorFromEvent, formatDur, hotkeyKeys, isMac } from "../format";
import * as ipc from "../ipc";
import { appSelectorFromLine, appSelectorParts } from "../model";
import { useAppData, useLoad } from "../store";

const SOURCE_URL = "https://github.com/futozs/closecall";
const RESTORE_WINDOWS = ["30m", "1h", "2h", "4h"] as const;

function useSave() {
  const { t } = useTranslation();
  const { config, saveConfig } = useAppData();
  const [error, setError] = useState<string | null>(null);
  const save = async (patch: Partial<Config>) => {
    try {
      const issues = await saveConfig({ ...config, ...patch });
      setError(
        issues.length > 0 ? issues.map((issue) => issueMessage(t, issue.code, issue.field)).join(" ") : null,
      );
    } catch {
      setError(t("common.error"));
    }
  };
  return { config, save, error };
}

function Row({ label, hint, children }: { label: ReactNode; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="grid gap-x-8 gap-y-1.5 py-3 sm:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] sm:items-start">
      <div>
        <div className="font-medium">{label}</div>
        {hint && <p className="mt-0.5 text-sm text-ink-2">{hint}</p>}
      </div>
      <div className="min-w-0">{children}</div>
    </div>
  );
}

function HotkeyField() {
  const { t } = useTranslation();
  const { status } = useAppData();
  const { config, save } = useSave();
  const [recording, setRecording] = useState(false);
  const keys = config.panic_hotkey ? hotkeyKeys(config.panic_hotkey, isMac(status.platform)) : null;
  return (
    <div className="flex flex-wrap items-center gap-2">
      {recording ? (
        <button
          type="button"
          // biome-ignore lint/a11y/noAutofocus: focus moves to the recorder the user just asked for
          autoFocus
          className="btn btn-secondary min-w-48 border-ink"
          onBlur={() => setRecording(false)}
          onKeyDown={(event) => {
            event.preventDefault();
            if (event.key === "Escape") return setRecording(false);
            const accelerator = acceleratorFromEvent(event);
            if (!accelerator) return;
            setRecording(false);
            void save({ panic_hotkey: accelerator });
          }}
        >
          {t("settings.hotkey_press")}
        </button>
      ) : (
        <>
          {keys ? (
            <Keys keys={keys} label={config.panic_hotkey} />
          ) : (
            <span className="text-ink-2">{t("settings.hotkey_none")}</span>
          )}
          <Button size="sm" onClick={() => setRecording(true)}>
            {t("settings.hotkey_record")}
          </Button>
          {keys && (
            <Button size="sm" variant="ghost" onClick={() => void save({ panic_hotkey: "" })}>
              {t("settings.hotkey_clear")}
            </Button>
          )}
        </>
      )}
    </div>
  );
}

/** Saves on blur. Key it by `value` so a saved change resets the draft. */
function TextSetting({
  value,
  onSave,
  label,
}: {
  value: string;
  onSave: (value: string) => void;
  label: string;
}) {
  const [draft, setDraft] = useState(value);
  return (
    <input
      className="input data max-w-md"
      aria-label={label}
      value={draft}
      spellCheck={false}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => draft.trim() && draft !== value && onSave(draft.trim())}
      onKeyDown={(event) => event.key === "Enter" && event.currentTarget.blur()}
    />
  );
}

function ListSetting({
  items,
  onChange,
  label,
  placeholder,
}: {
  items: string[];
  onChange: (items: string[]) => void;
  label: string;
  placeholder: string;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const inputId = useId();
  const add = () => {
    const value = draft.trim();
    if (!value || items.includes(value)) return;
    onChange([...items, value]);
    setDraft("");
  };
  return (
    <div>
      {items.length > 0 ? (
        <ul className="mb-2 divide-y divide-rule border-y border-rule">
          {items.map((item) => (
            <li key={item} className="flex items-center justify-between gap-2 py-1.5">
              <span className="data truncate text-sm">{item}</span>
              <Button
                size="sm"
                variant="ghost"
                aria-label={t("settings.list_remove", { item })}
                onClick={() => onChange(items.filter((other) => other !== item))}
              >
                <CloseIcon className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mb-2 text-sm text-ink-3">{t("settings.list_empty")}</p>
      )}
      <div className="flex gap-2">
        <label htmlFor={inputId} className="sr-only">
          {label}
        </label>
        <input
          id={inputId}
          className="input data"
          value={draft}
          placeholder={placeholder}
          spellCheck={false}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              add();
            }
          }}
        />
        <Button onClick={add} disabled={!draft.trim()}>
          {t("settings.list_add")}
        </Button>
      </div>
    </div>
  );
}

function General() {
  const { t } = useTranslation();
  const { locale } = useAppData();
  const { config, save, error } = useSave();
  const autostart = useLoad(ipc.autostartEnabled);
  return (
    <Section title={t("settings.general")}>
      {error && <ErrorNote>{error}</ErrorNote>}
      <div className="divide-y divide-rule">
        <Row label={t("settings.language")}>
          <select
            aria-label={t("settings.language")}
            className="select max-w-xs"
            value={config.language}
            onChange={(event) => {
              const language = (["system", "en", "hu"] as const).find(
                (value) => value === event.target.value,
              );
              if (language) void save({ language });
            }}
          >
            <option value="system">{t("settings.language_system")}</option>
            <option value="en">English</option>
            <option value="hu">Magyar</option>
          </select>
        </Row>
        <Switch
          label={t("settings.login")}
          description={t("settings.login_hint")}
          checked={autostart.data ?? false}
          disabled={autostart.data === null}
          onChange={(enabled) => void ipc.setAutostart(enabled).then(() => autostart.setData(enabled))}
        />
        <Row label={t("settings.hotkey")} hint={t("settings.hotkey_hint")}>
          <HotkeyField />
        </Row>
        <Row label={t("settings.confidence")} hint={t("settings.confidence_hint")}>
          <fieldset>
            <legend className="sr-only">{t("settings.confidence")}</legend>
            {(["low", "medium", "high"] as const).map((level) => (
              <Choice
                key={level}
                type="radio"
                name="confidence"
                label={t(`settings.confidence_${level}`)}
                checked={config.share_min_confidence === level}
                onChange={() => void save({ share_min_confidence: level })}
              />
            ))}
          </fieldset>
        </Row>
        <Row label={t("settings.untitled_dir")} hint={t("settings.untitled_dir_hint")}>
          <TextSetting
            key={config.untitled_save_dir}
            label={t("settings.untitled_dir")}
            value={config.untitled_save_dir}
            onSave={(untitled_save_dir) => void save({ untitled_save_dir })}
          />
        </Row>
        <Row label={t("settings.restore_window")} hint={t("settings.restore_window_hint")}>
          <select
            aria-label={t("settings.restore_window")}
            className="select max-w-xs"
            value={config.restore_window}
            onChange={(event) => void save({ restore_window: event.target.value })}
          >
            {[...new Set([...RESTORE_WINDOWS, config.restore_window])].map((value) => (
              <option key={value} value={value}>
                {formatDur(value, locale)}
              </option>
            ))}
          </select>
        </Row>
      </div>
    </Section>
  );
}

function Protection() {
  const { t } = useTranslation();
  const { categories } = useAppData();
  const { config, save, error } = useSave();
  const { protect } = config;
  const ids = [...new Set([...categories.map((info) => info.id), ...config.categories.map((def) => def.id)])];
  const userNames = new Map(config.categories.map((def) => [def.id, def.name]));
  return (
    <Section title={t("settings.protection")} lead={t("settings.protection_lead")}>
      {error && <ErrorNote>{error}</ErrorNote>}
      <div className="divide-y divide-rule">
        <Row label={t("settings.protected_categories")}>
          <fieldset className="grid gap-x-4 sm:grid-cols-2">
            <legend className="sr-only">{t("settings.protected_categories")}</legend>
            {ids.map((id) => (
              <Choice
                key={id}
                type="checkbox"
                label={categoryName(t, id, userNames.get(id))}
                checked={protect.categories.includes(id)}
                onChange={(on) =>
                  void save({
                    protect: {
                      ...protect,
                      categories: on
                        ? [...protect.categories, id]
                        : protect.categories.filter((c) => c !== id),
                    },
                  })
                }
              />
            ))}
          </fieldset>
        </Row>
        <Row label={t("settings.protected_apps")} hint={t("settings.protected_apps_hint")}>
          <ListSetting
            label={t("settings.protected_apps")}
            placeholder="us.zoom.xos"
            items={protect.apps.map((app) => appSelectorParts(app).value)}
            onChange={(items) => void save({ protect: { ...protect, apps: items.map(appSelectorFromLine) } })}
          />
        </Row>
        <Row label={t("settings.protected_domains")} hint={t("settings.protected_domains_hint")}>
          <ListSetting
            label={t("settings.protected_domains")}
            placeholder="meet.google.com"
            items={protect.domains}
            onChange={(domains) => void save({ protect: { ...protect, domains } })}
          />
        </Row>
        <Switch
          label={t("settings.protect_audible")}
          description={t("settings.protect_audible_hint")}
          checked={protect.audible_tabs}
          onChange={(audible_tabs) => void save({ protect: { ...protect, audible_tabs } })}
        />
        <Switch
          label={t("settings.protect_pinned")}
          description={t("settings.protect_pinned_hint")}
          checked={protect.pinned_tabs}
          onChange={(pinned_tabs) => void save({ protect: { ...protect, pinned_tabs } })}
        />
      </div>
    </Section>
  );
}

function capabilityLine(t: ReturnType<typeof useTranslation>["t"], capability: Capability, locale: string) {
  const line = t(`capability.${capability.action}`, { kinds: t(`target_plural.${capability.target_kind}`) });
  return capability.app ? line : line.charAt(0).toLocaleUpperCase(locale) + line.slice(1);
}

function WhatICanSee() {
  const { t } = useTranslation();
  const { status, locale } = useAppData();
  const report = useLoad(ipc.getCapabilities);
  const groups = new Map<string, Capability[]>();
  for (const capability of report.data?.capabilities ?? []) {
    groups.set(capability.provider, [...(groups.get(capability.provider) ?? []), capability]);
  }
  return (
    <Section title={t("settings.capabilities")} lead={t("settings.capabilities_lead")}>
      {report.error !== null && <ErrorNote>{t("common.error")}</ErrorNote>}
      <div className="grid gap-6">
        {[...groups].map(([provider, capabilities]) => (
          <div key={provider}>
            <h3 className="text-sm font-semibold text-ink-2">
              {t(`provider.${provider}`, { defaultValue: provider })}
            </h3>
            <ul className="mt-1">
              {capabilities.map((capability) => (
                <CheckRow
                  key={`${capability.app}-${capability.target_kind}-${capability.action}`}
                  state={
                    capability.support.level === "full"
                      ? "ok"
                      : capability.support.level === "partial"
                        ? "attention"
                        : "off"
                  }
                  label={
                    <>
                      {capability.app && <span className="font-medium">{capability.app}: </span>}
                      {capabilityLine(t, capability, locale)}
                    </>
                  }
                  value={t(`support.${capability.support.level}`)}
                  detail={
                    capability.support.level === "partial"
                      ? keyedMessage(t, capability.support.note_key, "note")
                      : undefined
                  }
                />
              ))}
            </ul>
          </div>
        ))}
        <div>
          <h3 className="text-sm font-semibold text-ink-2">{t("home.check.watching")}</h3>
          <ul className="mt-1">
            {(report.data?.signal_sources ?? status.signal_sources).map((source) => (
              <SourceRow key={source.id} source={source} />
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}

function About() {
  const { t } = useTranslation();
  const { status, categories } = useAppData();
  const sources = new Map(categories.flatMap((info) => info.sources).map((source) => [source.id, source]));
  return (
    <Section title={t("settings.about")}>
      <div className="grid gap-4">
        <p>
          <span className="font-semibold">{t("app.name")}</span>{" "}
          <span className="data text-sm text-ink-2">
            {t("settings.version", {
              version: status.app_version,
              platform: t(`platform.${status.platform}`, { defaultValue: status.platform }),
            })}
          </span>
        </p>
        <p className="max-w-prose">{t("settings.no_network")}</p>
        <p className="max-w-prose text-ink-2">{t("settings.licence")}</p>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void ipc.openUrl(SOURCE_URL)}>
            {t("settings.source")}
            <ExternalIcon className="size-3.5" />
          </Button>
          <a href="#setup" className="btn btn-ghost btn-sm">
            {t("settings.rerun_setup")}
          </a>
        </div>
        <div>
          <h3 className="text-sm font-semibold text-ink-2">{t("settings.list_licences")}</h3>
          <ul className="mt-1 divide-y divide-rule">
            {[...sources.values()].map((source) => (
              <li
                key={source.id}
                className="flex flex-wrap items-baseline justify-between gap-x-4 py-2 text-sm"
              >
                <span>{source.name}</span>
                <span className="data text-ink-2">{source.license}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}

export function Settings() {
  const { t } = useTranslation();
  const { status } = useAppData();
  const permissions = status.permissions.filter((state) => state.status !== "not_applicable");
  return (
    <div>
      <PageHeader title={t("settings.title")} />
      <General />
      <Protection />
      <Section title={t("settings.permissions")} lead={t("settings.permissions_lead")}>
        <ul>
          {permissions.map((state) => (
            <PermissionRow key={permissionKey(state)} state={state} />
          ))}
        </ul>
      </Section>
      <WhatICanSee />
      <About />
    </div>
  );
}
