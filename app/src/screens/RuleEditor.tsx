import { type ReactNode, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Rule } from "../bindings/Rule";
import type { UnsavedPolicy } from "../bindings/UnsavedPolicy";
import type { ValidationIssue } from "../bindings/ValidationIssue";
import { DurationInput } from "../components/DurationInput";
import { ChevronLeft } from "../components/icons";
import { Button, Choice, ErrorNote } from "../components/ui";
import { issueMessage, ruleName } from "../evidence";
import { go } from "../hash";
import {
  ACTIONS,
  defaultMatcher,
  isTimed,
  SIMPLE_TRIGGERS,
  TARGET_KINDS,
  TIMED_TRIGGERS,
  type TriggerKind,
  timedTrigger,
  triggerAllowsConfirm,
  triggerDur,
  triggerKind,
} from "../model";
import { useAppData } from "../store";
import { MatcherNode } from "./MatcherEditor";

const POLICIES: UnsavedPolicy[] = ["save_then_close", "hide", "skip_and_notify", "discard"];
const DEFAULT_DURATIONS: Record<(typeof TIMED_TRIGGERS)[number], string> = {
  away_longer_than: "15m",
  idle_longer_than: "10m",
  item_open_longer_than: "2h",
};

function fieldRoot(field: string): string {
  return field.split(/[.[]/)[0] ?? field;
}

function Group({
  legend,
  hint,
  issues,
  children,
}: {
  legend: ReactNode;
  hint?: ReactNode;
  issues: string[];
  children: ReactNode;
}) {
  const hintId = useId();
  return (
    <fieldset className="border-t border-rule pt-5 pb-6" aria-describedby={hint ? hintId : undefined}>
      <legend className="float-left mb-1 w-full font-semibold">{legend}</legend>
      {hint && (
        <p id={hintId} className="clear-left mb-3 max-w-prose text-sm text-ink-2">
          {hint}
        </p>
      )}
      <div className="clear-left">{children}</div>
      {issues.map((issue) => (
        <p key={issue} className="mt-2 text-sm font-medium text-signal-text">
          {issue}
        </p>
      ))}
    </fieldset>
  );
}

export function blankRule(id: string, firstCategory: string): Rule {
  return {
    id,
    name: null,
    enabled: true,
    when: ["panic", "screen_share"],
    targets: ["tab"],
    match: { any: [defaultMatcher("category", firstCategory)] },
    except: null,
    action: "close",
    on_unsaved: "save_then_close",
    confirm: "never",
  };
}

export function RuleEditor({ initial, isNew }: { initial: Rule; isNew: boolean }) {
  const { t } = useTranslation();
  const { config, saveConfig } = useAppData();
  const [draft, setDraft] = useState(initial);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const update = (patch: Partial<Rule>) => setDraft((current) => ({ ...current, ...patch }));

  const mine = issues.filter((issue) => issue.rule === null || issue.rule === draft.id);
  const others = issues.length - mine.length;
  const messagesFor = (root: string) =>
    mine
      .filter((issue) => fieldRoot(issue.field) === root)
      .map((issue) => issueMessage(t, issue.code, issue.field));
  const issuesAt = (path: string) =>
    mine.filter((issue) => issue.field === path).map((issue) => issueMessage(t, issue.code, issue.field));

  const suggested = ruleName(t, { id: draft.id, name: null });
  const kinds = draft.when.map(triggerKind);
  const countdownAllowed = kinds.length > 0 && kinds.every(triggerAllowsConfirm);
  const closes = draft.action === "close" || draft.action === "quit";

  const setTrigger = (kind: TriggerKind, on: boolean) => {
    const rest = draft.when.filter((trigger) => triggerKind(trigger) !== kind);
    if (!on) return update({ when: rest });
    update({ when: [...rest, isTimed(kind) ? timedTrigger(kind, DEFAULT_DURATIONS[kind]) : kind] });
  };

  const save = async () => {
    setSaving(true);
    const rules = isNew
      ? [...config.rule, draft]
      : config.rule.map((rule) => (rule.id === initial.id ? draft : rule));
    try {
      const result = await saveConfig({ ...config, rule: rules });
      setIssues(result);
      if (result.length === 0) go("rules");
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    const result = await saveConfig({
      ...config,
      rule: config.rule.filter((rule) => rule.id !== initial.id),
    });
    if (result.length === 0) go("rules");
    else setIssues(result);
  };

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
      noValidate
    >
      <a
        href="#rules"
        className="inline-flex items-center gap-1 text-sm text-ink-2 no-underline hover:text-ink"
      >
        <ChevronLeft className="size-3.5" />
        {t("editor.back")}
      </a>
      <header className="mt-3 mb-6">
        <h1 className="text-xl font-semibold tracking-tight">
          {isNew ? t("editor.title_new") : ruleName(t, initial)}
        </h1>
        <p className="data mt-1 text-xs text-ink-3">{t("editor.id", { id: draft.id })}</p>
        {messagesFor("id").map((message) => (
          <p key={message} className="mt-1 text-sm font-medium text-signal-text">
            {message}
          </p>
        ))}
      </header>

      {mine.length > 0 && (
        <div className="mb-4">
          <ErrorNote>{t("editor.issues", { count: mine.length })}</ErrorNote>
        </div>
      )}
      {others > 0 && (
        <div className="mb-4">
          <ErrorNote>{t("editor.other_issues", { count: others })}</ErrorNote>
        </div>
      )}

      <Group legend={t("editor.name")} hint={t("editor.name_hint")} issues={messagesFor("name")}>
        <input
          className="input max-w-md"
          aria-label={t("editor.name")}
          value={draft.name ?? ""}
          placeholder={suggested === draft.id ? t("editor.name_placeholder") : suggested}
          onChange={(event) => update({ name: event.target.value || null })}
        />
      </Group>

      <Group legend={t("editor.when")} hint={t("editor.when_hint")} issues={messagesFor("when")}>
        <div className="grid gap-x-6 sm:grid-cols-2">
          {SIMPLE_TRIGGERS.map((kind) => (
            <Choice
              key={kind}
              type="checkbox"
              label={t(`when.${kind}`)}
              description={kind === "screen_share" ? t("editor.share_hint") : undefined}
              checked={kinds.includes(kind)}
              onChange={(on) => setTrigger(kind, on)}
            />
          ))}
        </div>
        <div className="mt-2 grid gap-1">
          {TIMED_TRIGGERS.map((kind) => {
            const trigger = draft.when.find((candidate) => triggerKind(candidate) === kind);
            return (
              <div key={kind} className="flex flex-wrap items-center gap-x-3 gap-y-1">
                <Choice
                  type="checkbox"
                  label={t(`when.${kind}`)}
                  checked={Boolean(trigger)}
                  onChange={(on) => setTrigger(kind, on)}
                />
                {trigger && (
                  <DurationInput
                    label={t(`when.${kind}`)}
                    value={triggerDur(trigger) ?? DEFAULT_DURATIONS[kind]}
                    onChange={(dur) =>
                      update({
                        when: draft.when.map((candidate) =>
                          triggerKind(candidate) === kind ? timedTrigger(kind, dur) : candidate,
                        ),
                      })
                    }
                  />
                )}
              </div>
            );
          })}
        </div>
      </Group>

      <Group legend={t("editor.targets")} issues={messagesFor("targets")}>
        <div className="grid gap-x-6 sm:grid-cols-2">
          {TARGET_KINDS.map((kind) => (
            <Choice
              key={kind}
              type="checkbox"
              label={t(`target.${kind}`)}
              description={t(`target_hint.${kind}`)}
              checked={draft.targets.includes(kind)}
              onChange={(on) =>
                update({ targets: on ? [...draft.targets, kind] : draft.targets.filter((k) => k !== kind) })
              }
            />
          ))}
        </div>
      </Group>

      <Group legend={t("editor.match")} hint={t("editor.match_hint")} issues={[]}>
        <MatcherNode
          matcher={draft.match}
          path="match"
          issuesAt={issuesAt}
          onChange={(match) => update({ match })}
        />
      </Group>

      <Group legend={t("editor.except")} hint={t("editor.except_hint")} issues={[]}>
        {draft.except ? (
          <MatcherNode
            matcher={draft.except}
            path="except"
            issuesAt={issuesAt}
            onChange={(except) => update({ except })}
            onRemove={() => update({ except: null })}
          />
        ) : (
          <Button variant="secondary" size="sm" onClick={() => update({ except: { domain: "" } })}>
            {t("editor.except_add")}
          </Button>
        )}
      </Group>

      <Group legend={t("editor.action")} issues={messagesFor("action")}>
        <div className="grid gap-x-6 sm:grid-cols-2">
          {ACTIONS.map((action) => (
            <Choice
              key={action}
              type="radio"
              name="action"
              label={t(`action.${action}`)}
              description={t(`action_hint.${action}`)}
              checked={draft.action === action}
              onChange={() => update({ action })}
            />
          ))}
        </div>
        {draft.action === "quit" && (
          <p className="mt-3 max-w-prose rounded-ctl bg-hover px-3 py-2 text-sm">
            {t("editor.quit_warning")}
          </p>
        )}
      </Group>

      {closes && (
        <Group
          legend={t("editor.on_unsaved")}
          hint={t("editor.on_unsaved_hint")}
          issues={messagesFor("on_unsaved")}
        >
          {POLICIES.map((policy) => (
            <Choice
              key={policy}
              type="radio"
              name="on_unsaved"
              label={t(`unsaved.${policy}`)}
              description={t(`unsaved_hint.${policy}`, { folder: config.untitled_save_dir })}
              checked={draft.on_unsaved === policy}
              onChange={() => update({ on_unsaved: policy })}
            />
          ))}
        </Group>
      )}

      <Group
        legend={t("editor.confirm")}
        hint={countdownAllowed ? t("editor.confirm_hint") : t("editor.confirm_not_allowed")}
        issues={messagesFor("confirm")}
      >
        <Choice
          type="radio"
          name="confirm"
          label={t("confirm.never")}
          checked={draft.confirm === "never"}
          onChange={() => update({ confirm: "never" })}
        />
        <div className="flex flex-wrap items-center gap-x-3">
          <Choice
            type="radio"
            name="confirm"
            label={t("confirm.countdown")}
            disabled={!countdownAllowed && draft.confirm === "never"}
            checked={draft.confirm !== "never"}
            onChange={() => update({ confirm: { countdown: "60s" } })}
          />
          {draft.confirm !== "never" && (
            <DurationInput
              label={t("confirm.countdown")}
              units={["s", "m"]}
              value={draft.confirm.countdown}
              onChange={(countdown) => update({ confirm: { countdown } })}
            />
          )}
        </div>
      </Group>

      <div className="flex flex-wrap items-center gap-2 border-t border-rule pt-5">
        <Button type="submit" variant="primary" disabled={saving}>
          {t("editor.save")}
        </Button>
        <a href="#rules" className="btn btn-ghost">
          {t("common.cancel")}
        </a>
        {!isNew &&
          (confirmDelete ? (
            <span className="ml-auto flex flex-wrap items-center gap-2">
              <span className="text-sm">{t("editor.delete_confirm")}</span>
              <Button variant="secondary" size="sm" onClick={() => void remove()}>
                {t("editor.delete_yes")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                {t("common.cancel")}
              </Button>
            </span>
          ) : (
            <Button variant="ghost" className="ml-auto" onClick={() => setConfirmDelete(true)}>
              {t("editor.delete")}
            </Button>
          ))}
      </div>
    </form>
  );
}
