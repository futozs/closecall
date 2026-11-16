import { useTranslation } from "react-i18next";
import type { DocKind } from "../bindings/DocKind";
import type { Matcher } from "../bindings/Matcher";
import { CloseIcon, PlusIcon } from "../components/icons";
import { Button } from "../components/ui";
import { categoryName } from "../evidence";
import {
  type AppSelectorKind,
  appSelector,
  appSelectorParts,
  defaultMatcher,
  MATCHER_KINDS,
  type MatcherKind,
  matcherKind,
} from "../model";
import { useAppData } from "../store";

const DOC_KINDS: DocKind[] = [
  "pdf",
  "image",
  "word",
  "excel",
  "power_point",
  "pages",
  "keynote",
  "numbers",
  "text",
  "other",
];
const APP_KINDS: AppSelectorKind[] = ["bundle_id", "name", "exe", "desktop_id"];

export type IssuesAt = (path: string) => string[];

function isGroup(kind: MatcherKind) {
  return kind === "any" || kind === "all" || kind === "not";
}

function children(matcher: Matcher): Matcher[] {
  if (matcher === "private") return [];
  if ("any" in matcher) return matcher.any;
  if ("all" in matcher) return matcher.all;
  if ("not" in matcher) return [matcher.not];
  return [];
}

function withKind(current: Matcher, kind: MatcherKind, firstCategory: string): Matcher {
  const currentKind = matcherKind(current);
  if (kind === currentKind) return current;
  if (isGroup(kind)) {
    const kept = isGroup(currentKind) ? children(current) : [current];
    if (kind === "not") return { not: kept[0] ?? defaultMatcher("category", firstCategory) };
    return kind === "any" ? { any: kept } : { all: kept };
  }
  return defaultMatcher(kind, firstCategory);
}

function LeafValue({
  matcher,
  onChange,
  invalid,
  label,
}: {
  matcher: Matcher;
  onChange: (next: Matcher) => void;
  invalid: boolean;
  label: string;
}) {
  const { t } = useTranslation();
  const { categories, config } = useAppData();
  if (matcher === "private")
    return <span className="self-center text-sm text-ink-2">{t("matcher.private_hint")}</span>;
  const text = (value: string, make: (value: string) => Matcher, placeholder: string) => (
    <input
      className="input data min-w-40 flex-1"
      value={value}
      placeholder={placeholder}
      aria-label={label}
      aria-invalid={invalid}
      spellCheck={false}
      autoCapitalize="off"
      autoCorrect="off"
      onChange={(event) => onChange(make(event.target.value))}
    />
  );
  if ("category" in matcher) {
    const ids = [...categories.map((info) => info.id), ...config.categories.map((def) => def.id)];
    const names = new Map(config.categories.map((def) => [def.id, def.name]));
    return (
      <select
        className="select min-w-40 flex-1"
        value={matcher.category}
        aria-label={label}
        onChange={(event) => onChange({ category: event.target.value })}
      >
        {[...new Set(ids)].map((id) => (
          <option key={id} value={id}>
            {categoryName(t, id, names.get(id))}
          </option>
        ))}
      </select>
    );
  }
  if ("domain" in matcher) return text(matcher.domain, (domain) => ({ domain }), "otpbank.hu");
  if ("url_glob" in matcher)
    return text(matcher.url_glob, (url_glob) => ({ url_glob }), "https://mail.google.com/mail/u/1/*");
  if ("title_contains" in matcher)
    return text(
      matcher.title_contains,
      (title_contains) => ({ title_contains }),
      t("matcher.title_placeholder"),
    );
  if ("file_glob" in matcher)
    return text(matcher.file_glob, (file_glob) => ({ file_glob }), "~/Documents/Bank/**");
  if ("doc_kind" in matcher) {
    return (
      <select
        className="select min-w-40 flex-1"
        value={matcher.doc_kind}
        aria-label={label}
        onChange={(event) => {
          const kind = DOC_KINDS.find((candidate) => candidate === event.target.value);
          if (kind) onChange({ doc_kind: kind });
        }}
      >
        {DOC_KINDS.map((kind) => (
          <option key={kind} value={kind}>
            {t(`doc_kind.${kind}`)}
          </option>
        ))}
      </select>
    );
  }
  if ("app" in matcher) {
    const { kind, value } = appSelectorParts(matcher.app);
    return (
      <div className="flex min-w-0 flex-1 flex-wrap gap-2">
        <select
          className="select w-auto"
          value={kind}
          aria-label={t("matcher.app_by")}
          onChange={(event) => {
            const next = APP_KINDS.find((candidate) => candidate === event.target.value);
            if (next) onChange({ app: appSelector(next, value) });
          }}
        >
          {APP_KINDS.map((option) => (
            <option key={option} value={option}>
              {t(`matcher.app.${option}`)}
            </option>
          ))}
        </select>
        <input
          className="input data min-w-40 flex-1"
          value={value}
          placeholder={kind === "bundle_id" ? "com.agilebits.onepassword7" : "1Password"}
          aria-label={label}
          aria-invalid={invalid}
          spellCheck={false}
          onChange={(event) => onChange({ app: appSelector(kind, event.target.value) })}
        />
      </div>
    );
  }
  return null;
}

export function MatcherNode({
  matcher,
  onChange,
  onRemove,
  path,
  issuesAt,
  depth = 0,
}: {
  matcher: Matcher;
  onChange: (next: Matcher) => void;
  onRemove?: () => void;
  path: string;
  issuesAt: IssuesAt;
  depth?: number;
}) {
  const { t } = useTranslation();
  const { categories } = useAppData();
  const kind = matcherKind(matcher);
  const firstCategory = categories[0]?.id ?? "banking";
  const issues = issuesAt(path);
  const kindLabel = t("matcher.kind_label");
  const kindSelect = (
    <select
      className="select w-auto flex-none"
      value={kind}
      aria-label={kindLabel}
      onChange={(event) => {
        const next = MATCHER_KINDS.find((candidate) => candidate === event.target.value);
        if (next) onChange(withKind(matcher, next, firstCategory));
      }}
    >
      {MATCHER_KINDS.map((option) => (
        <option key={option} value={option}>
          {t(`matcher.kind.${option}`)}
        </option>
      ))}
    </select>
  );
  const remove = onRemove && (
    <Button
      variant="ghost"
      size="sm"
      onClick={onRemove}
      aria-label={t("matcher.remove")}
      title={t("matcher.remove")}
    >
      <CloseIcon className="size-3.5" />
    </Button>
  );
  const errors = issues.length > 0 && (
    <ul className="mt-1.5 text-sm font-medium text-signal-text">
      {issues.map((issue) => (
        <li key={issue}>{issue}</li>
      ))}
    </ul>
  );

  if (!isGroup(kind)) {
    return (
      <div>
        <div className="flex items-start gap-2">
          <div className="flex min-w-0 flex-1 flex-wrap items-start gap-2">
            {kindSelect}
            <LeafValue
              matcher={matcher}
              onChange={onChange}
              invalid={issues.length > 0}
              label={t(`matcher.kind.${kind}`)}
            />
          </div>
          {remove}
        </div>
        {errors}
      </div>
    );
  }

  const items = children(matcher);
  const setItems = (next: Matcher[]) => {
    if (matcher === "private") return;
    if ("any" in matcher) onChange({ any: next });
    else if ("all" in matcher) onChange({ all: next });
    else if (next[0]) onChange({ not: next[0] });
  };
  const childKey = kind === "not" ? "not" : kind;
  return (
    <div>
      <div className="flex items-center gap-2">
        {kindSelect}
        <span className="text-sm text-ink-2">{t(`matcher.group_hint.${kind}`)}</span>
        <span className="ml-auto">{remove}</span>
      </div>
      {errors}
      <div className={`mt-2 grid gap-2 border-l border-rule-strong pl-4 ${depth > 2 ? "" : "ml-2"}`}>
        {items.map((child, i) => (
          <MatcherNode
            // biome-ignore lint/suspicious/noArrayIndexKey: conditions have no identity; the config stores them by position
            key={i}
            matcher={child}
            path={kind === "not" ? `${path}.not` : `${path}.${childKey}[${i}]`}
            issuesAt={issuesAt}
            depth={depth + 1}
            onChange={(next) => setItems(items.map((item, j) => (j === i ? next : item)))}
            onRemove={kind === "not" ? undefined : () => setItems(items.filter((_, j) => j !== i))}
          />
        ))}
        {kind !== "not" && (
          <div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setItems([...items, defaultMatcher("category", firstCategory)])}
            >
              <PlusIcon className="size-3.5" />
              {t("matcher.add")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
