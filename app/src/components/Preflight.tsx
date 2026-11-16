import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { PermissionState } from "../bindings/PermissionState";
import type { SignalSourceStatus } from "../bindings/SignalSourceStatus";
import { keyedMessage } from "../evidence";
import * as ipc from "../ipc";
import { useAppData } from "../store";
import { Button, CheckRow, type MarkState } from "./ui";

const PERMISSION_MARK: Record<PermissionState["status"], MarkState> = {
  granted: "ok",
  denied: "attention",
  not_determined: "open",
  app_not_running: "open",
  not_applicable: "off",
};

export function permissionKey(state: PermissionState): string {
  return JSON.stringify(state.permission);
}

export function PermissionRow({ state }: { state: PermissionState }) {
  const { t } = useTranslation();
  const { status, setStatus } = useAppData();
  const [asking, setAsking] = useState(false);
  // macOS asks for Automation only while the app runs; the engine asks again when it starts.
  const [waitingForApp, setWaitingForApp] = useState(false);
  const [refused, setRefused] = useState(false);
  const shared = status.share.active || status.share.mirrored_display || status.share.external_display;
  const { permission } = state;
  const app = permission.kind === "automation" ? permission.app_name : "";
  const label = permission.kind === "automation" ? app : t(`permission.${permission.kind}`);
  const canAsk = (state.status === "not_determined" || state.status === "denied") && !waitingForApp;
  const ask = async () => {
    setAsking(true);
    try {
      const outcome = await ipc.requestPermission(permission);
      setWaitingForApp(outcome.kind === "app_not_running");
      setRefused(outcome.kind === "not_while_sharing");
      setStatus(await ipc.getStatus());
    } finally {
      setAsking(false);
    }
  };
  return (
    <CheckRow
      state={PERMISSION_MARK[state.status]}
      label={label}
      value={t(`permission.status.${state.status}`, { app })}
      detail={
        waitingForApp
          ? t("permission.asks_when_open")
          : refused && shared
            ? t("permission.not_while_sharing")
            : undefined
      }
      action={
        canAsk && (
          <Button
            size="sm"
            variant={state.status === "denied" ? "secondary" : "primary"}
            onClick={ask}
            disabled={asking || shared}
            title={shared ? t("permission.not_while_sharing") : undefined}
          >
            {state.status === "denied" ? t("permission.open_settings") : t("permission.allow")}
          </Button>
        )
      }
    />
  );
}

export function SourceRow({ source }: { source: SignalSourceStatus }) {
  const { t } = useTranslation();
  const { health } = source;
  return (
    <CheckRow
      state={health.state === "ok" ? "ok" : health.state === "noisy" ? "attention" : "off"}
      label={t(`source.${source.id}`, { defaultValue: source.id })}
      value={t(`health.${health.state}`)}
      detail={health.state === "ok" ? undefined : keyedMessage(t, health.reason_key, "reason")}
    />
  );
}

export function PreflightList() {
  const { t } = useTranslation();
  const { status, config } = useAppData();
  const shown = status.permissions.filter((state) => state.status !== "not_applicable");
  const system = shown.filter((state) => state.permission.kind !== "automation");
  const apps = shown.filter((state) => state.permission.kind === "automation");
  const active = config.rule.filter((rule) => rule.enabled).length;
  return (
    <div className="grid gap-6">
      <div>
        <h3 className="text-sm font-semibold text-ink-2">{t("home.check.access")}</h3>
        <ul className="mt-1">
          {system.map((state) => (
            <PermissionRow key={permissionKey(state)} state={state} />
          ))}
          <CheckRow
            state={active > 0 ? "ok" : "attention"}
            label={t("home.check.rules")}
            value={active > 0 ? t("home.check.rules_active", { count: active }) : t("home.check.rules_none")}
            action={
              active === 0 && (
                <a href="#rules" className="btn btn-secondary btn-sm">
                  {t("home.check.rules_open")}
                </a>
              )
            }
          />
        </ul>
      </div>
      {apps.length > 0 && (
        <div>
          <h3 className="text-sm font-semibold text-ink-2">{t("home.check.apps")}</h3>
          <ul className="mt-1">
            {apps.map((state) => (
              <PermissionRow key={permissionKey(state)} state={state} />
            ))}
          </ul>
        </div>
      )}
      <div>
        <h3 className="text-sm font-semibold text-ink-2">{t("home.check.watching")}</h3>
        <ul className="mt-1">
          {status.signal_sources.map((source) => (
            <SourceRow key={source.id} source={source} />
          ))}
        </ul>
      </div>
    </div>
  );
}
