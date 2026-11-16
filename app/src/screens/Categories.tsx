import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CategoryDef } from "../bindings/CategoryDef";
import type { CategoryInfo } from "../bindings/CategoryInfo";
import type { ValidationIssue } from "../bindings/ValidationIssue";
import { ChevronLeft, ChevronRight, ExternalIcon, PlusIcon } from "../components/icons";
import { Button, EmptyState, ErrorNote, Field, PageHeader, Section } from "../components/ui";
import { categoryName, issueMessage } from "../evidence";
import { formatNumber } from "../format";
import { go, useSubpath } from "../hash";
import * as ipc from "../ipc";
import { appSelectorFromLine, appSelectorParts, rulesUsingCategory, slug, splitLines } from "../model";
import { useAppData } from "../store";

function Counts({
  domains,
  apps,
  keywords,
  files = 0,
}: {
  domains: number;
  apps: number;
  keywords: number;
  files?: number;
}) {
  const { t } = useTranslation();
  const { locale } = useAppData();
  const parts = Object.entries({ domains, apps, keywords, files })
    .filter(([, count]) => count > 0)
    .map(([key, count]) => t(`categories.${key}`, { count, n: formatNumber(count, locale) }));
  return <span className="data">{parts.length > 0 ? parts.join(" · ") : t("categories.nothing")}</span>;
}

function UsedBy({ id }: { id: string }) {
  const { t } = useTranslation();
  const { config } = useAppData();
  if (config.protect.categories.includes(id)) {
    return <span className="text-sm font-medium text-ok">{t("categories.protected")}</span>;
  }
  const count = rulesUsingCategory(config.rule, id).length;
  return (
    <span className={`text-sm ${count > 0 ? "text-ink" : "text-ink-3"}`}>
      {count > 0 ? t("categories.used_by", { count }) : t("categories.unused")}
    </span>
  );
}

function BuiltinRow({ info }: { info: CategoryInfo }) {
  const { t } = useTranslation();
  const { locale } = useAppData();
  return (
    <li className="py-4">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h3 className="font-semibold">{categoryName(t, info.id)}</h3>
        <UsedBy id={info.id} />
      </div>
      <p className="mt-0.5 max-w-prose text-sm text-ink-2">
        {t(`category.${info.id}.description`, { defaultValue: "" })}
      </p>
      <details className="group mt-2">
        <summary className="inline-flex cursor-default list-none items-center gap-1.5 rounded-ctl text-sm text-ink-2 hover:text-ink [&::-webkit-details-marker]:hidden">
          <ChevronRight className="size-3.5 transition-transform group-open:rotate-90" />
          <Counts domains={info.domain_count} apps={info.app_count} keywords={info.keyword_count} />
          <span className="text-ink-3">· {t("categories.sources", { count: info.sources.length })}</span>
        </summary>
        <ul className="mt-2 ml-5 grid gap-2 border-l border-rule pl-4 text-sm">
          {info.sources.map((source) => (
            <li key={source.id}>
              <button
                type="button"
                className="inline-flex items-center gap-1 font-medium underline decoration-rule-strong underline-offset-3 hover:decoration-current"
                onClick={() => void ipc.openUrl(source.url)}
              >
                {source.name}
                <ExternalIcon className="size-3" />
              </button>
              <span className="data block text-ink-2">
                {t("categories.source_line", {
                  license: source.license,
                  entries: formatNumber(source.entries, locale),
                  date: source.snapshot_date,
                })}
              </span>
            </li>
          ))}
        </ul>
      </details>
    </li>
  );
}

const ID_PATTERN = /^[a-z0-9_]+$/;

function CategoryEditor({ initial, isNew }: { initial: CategoryDef; isNew: boolean }) {
  const { t } = useTranslation();
  const { config, categories, saveConfig } = useAppData();
  const [typedId, setTypedId] = useState<string | null>(isNew ? null : initial.id);
  const [name, setName] = useState(initial.name ?? "");
  const id = typedId ?? slug(name);
  const [domains, setDomains] = useState(initial.domains.join("\n"));
  const [apps, setApps] = useState(initial.apps.map((app) => appSelectorParts(app).value).join("\n"));
  const [keywords, setKeywords] = useState(initial.title_keywords.join("\n"));
  const [globs, setGlobs] = useState(initial.file_globs.join("\n"));
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [touched, setTouched] = useState(false);

  const taken =
    isNew && (categories.some((info) => info.id === id) || config.categories.some((def) => def.id === id));
  const idError = !ID_PATTERN.test(id)
    ? t("categories.editor.id_invalid")
    : taken
      ? t("categories.editor.id_taken")
      : null;
  const usedBy = rulesUsingCategory(config.rule, initial.id).length;

  const save = async () => {
    setTouched(true);
    if (idError) return;
    const def: CategoryDef = {
      ...initial,
      id,
      name: name.trim() || null,
      domains: splitLines(domains),
      apps: splitLines(apps).map(appSelectorFromLine),
      title_keywords: splitLines(keywords),
      file_globs: splitLines(globs),
    };
    const next = isNew
      ? [...config.categories, def]
      : config.categories.map((existing) => (existing.id === initial.id ? def : existing));
    const result = await saveConfig({ ...config, categories: next });
    setIssues(result);
    if (result.length === 0) go("categories");
  };
  const remove = async () => {
    const result = await saveConfig({
      ...config,
      categories: config.categories.filter((def) => def.id !== initial.id),
    });
    if (result.length === 0) go("categories");
    else setIssues(result);
  };

  const area = (label: string, hint: string, value: string, set: (value: string) => void) => (
    <Field label={label} hint={hint}>
      {({ id: fieldId, describedBy }) => (
        <textarea
          id={fieldId}
          aria-describedby={describedBy}
          className="input"
          value={value}
          spellCheck={false}
          onChange={(event) => set(event.target.value)}
        />
      )}
    </Field>
  );

  return (
    <form
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <a
        href="#categories"
        className="inline-flex items-center gap-1 text-sm text-ink-2 no-underline hover:text-ink"
      >
        <ChevronLeft className="size-3.5" />
        {t("categories.back")}
      </a>
      <h1 className="mt-3 mb-6 text-xl font-semibold tracking-tight">
        {isNew ? t("categories.editor.title_new") : name || initial.id}
      </h1>
      {issues.length > 0 && (
        <div className="mb-4">
          <ErrorNote>{issues.map((issue) => issueMessage(t, issue.code, issue.field)).join(" ")}</ErrorNote>
        </div>
      )}
      <div className="grid gap-5">
        <div className="grid gap-5 sm:grid-cols-2">
          <Field label={t("categories.editor.name")}>
            {({ id: fieldId }) => (
              <input
                id={fieldId}
                className="input"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            )}
          </Field>
          <Field
            label={t("categories.editor.id")}
            hint={t("categories.editor.id_hint")}
            error={touched || id !== initial.id ? idError : null}
          >
            {({ id: fieldId, describedBy, invalid }) => (
              <input
                id={fieldId}
                className="input data"
                value={id}
                readOnly={!isNew}
                aria-describedby={describedBy}
                aria-invalid={invalid}
                spellCheck={false}
                onChange={(event) => setTypedId(event.target.value.toLowerCase())}
              />
            )}
          </Field>
        </div>
        {area(t("categories.editor.domains"), t("categories.editor.domains_hint"), domains, setDomains)}
        {area(t("categories.editor.apps"), t("categories.editor.apps_hint"), apps, setApps)}
        {area(t("categories.editor.keywords"), t("categories.editor.keywords_hint"), keywords, setKeywords)}
        {area(t("categories.editor.files"), t("categories.editor.files_hint"), globs, setGlobs)}
      </div>
      <div className="mt-6 flex flex-wrap items-center gap-2 border-t border-rule pt-5">
        <Button type="submit" variant="primary">
          {t("categories.editor.save")}
        </Button>
        <a href="#categories" className="btn btn-ghost">
          {t("common.cancel")}
        </a>
        {!isNew &&
          (confirmDelete ? (
            <span className="ml-auto flex flex-wrap items-center gap-2">
              <span className="text-sm">
                {usedBy > 0
                  ? t("categories.editor.delete_used", { count: usedBy })
                  : t("categories.editor.delete_confirm")}
              </span>
              <Button variant="secondary" size="sm" onClick={() => void remove()}>
                {t("categories.editor.delete_yes")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                {t("common.cancel")}
              </Button>
            </span>
          ) : (
            <Button variant="ghost" className="ml-auto" onClick={() => setConfirmDelete(true)}>
              {t("categories.editor.delete")}
            </Button>
          ))}
      </div>
    </form>
  );
}

function emptyCategory(): CategoryDef {
  return { id: "", name: null, domains: [], lists: [], apps: [], title_keywords: [], file_globs: [] };
}

export function Categories() {
  const { t } = useTranslation();
  const { config, categories } = useAppData();
  const [mode, id] = useSubpath();

  if (mode === "new") return <CategoryEditor key="new" initial={emptyCategory()} isNew />;
  const editing = mode === "edit" ? config.categories.find((def) => def.id === id) : undefined;
  if (editing) return <CategoryEditor key={editing.id} initial={editing} isNew={false} />;

  return (
    <div>
      <PageHeader title={t("categories.title")} lead={t("categories.lead")} />
      <Section
        title={t("categories.yours")}
        actions={
          <a href="#categories/new" className="btn btn-secondary btn-sm">
            <PlusIcon className="size-3.5" />
            {t("categories.new")}
          </a>
        }
      >
        {config.categories.length === 0 ? (
          <EmptyState title={t("categories.empty_title")} body={t("categories.empty_body")} />
        ) : (
          <ul className="-mx-3 divide-y divide-rule">
            {config.categories.map((def) => (
              <li key={def.id}>
                <a
                  href={`#categories/edit/${encodeURIComponent(def.id)}`}
                  className="row-hover flex items-center gap-4 rounded-ctl px-3 py-3 no-underline"
                >
                  <span className="min-w-0 flex-1">
                    <span className="block font-medium">{def.name || def.id}</span>
                    <span className="block text-sm text-ink-2">
                      <Counts
                        domains={def.domains.length}
                        apps={def.apps.length}
                        keywords={def.title_keywords.length}
                        files={def.file_globs.length}
                      />
                    </span>
                  </span>
                  <UsedBy id={def.id} />
                  <ChevronRight className="size-4 flex-none text-ink-3" />
                </a>
              </li>
            ))}
          </ul>
        )}
      </Section>
      <Section title={t("categories.builtin")} lead={t("categories.builtin_lead")}>
        <ul className="divide-y divide-rule">
          {categories.map((info) => (
            <BuiltinRow key={info.id} info={info} />
          ))}
        </ul>
      </Section>
    </div>
  );
}
