import { type ButtonHTMLAttributes, type ReactNode, useId } from "react";

type Variant = "signal" | "primary" | "secondary" | "ghost";
type Size = "sm" | "md" | "lg";

export function Button({
  variant = "secondary",
  size = "md",
  className = "",
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: Size }) {
  const sizeClass = size === "md" ? "" : `btn-${size}`;
  return <button type={type} className={`btn btn-${variant} ${sizeClass} ${className}`} {...props} />;
}

export function Switch({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
}) {
  const id = useId();
  return (
    <div className="flex items-start justify-between gap-6 py-3">
      <div className="min-w-0">
        <label htmlFor={id} className="block font-medium">
          {label}
        </label>
        {description && (
          <p id={`${id}-desc`} className="mt-0.5 text-sm text-ink-2">
            {description}
          </p>
        )}
      </div>
      <button
        id={id}
        type="button"
        role="switch"
        aria-checked={checked}
        aria-describedby={description ? `${id}-desc` : undefined}
        disabled={disabled}
        className="switch mt-0.5"
        onClick={() => onChange(!checked)}
      />
    </div>
  );
}

export function Field({
  label,
  hint,
  error,
  children,
  className = "",
}: {
  label: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  children: (ids: { id: string; describedBy: string | undefined; invalid: boolean }) => ReactNode;
  className?: string;
}) {
  const id = useId();
  const describedBy = [hint && `${id}-hint`, error && `${id}-error`].filter(Boolean).join(" ") || undefined;
  return (
    <div className={className}>
      <label htmlFor={id} className="mb-1.5 block font-medium">
        {label}
      </label>
      {children({ id, describedBy, invalid: Boolean(error) })}
      {hint && (
        <p id={`${id}-hint`} className="mt-1.5 text-sm text-ink-2">
          {hint}
        </p>
      )}
      {error && (
        <p id={`${id}-error`} className="mt-1.5 text-sm font-medium text-signal-text">
          {error}
        </p>
      )}
    </div>
  );
}

export function Choice({
  type,
  name,
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  type: "checkbox" | "radio";
  name?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
}) {
  return (
    <label className={`flex gap-2.5 py-1.5 ${disabled ? "text-ink-3" : ""}`}>
      <input
        type={type}
        name={name}
        className="check mt-0.5"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="min-w-0">
        <span className="block">{label}</span>
        {description && <span className="mt-0.5 block text-sm text-ink-2">{description}</span>}
      </span>
    </label>
  );
}

export function Keys({ keys, label }: { keys: string[]; label?: string }) {
  return (
    <span className="inline-flex">
      <span className="sr-only">{label ?? keys.join(" ")}</span>
      <span className="inline-flex items-center gap-1" aria-hidden="true">
        {keys.map((key) => (
          <kbd key={key} className="key">
            {key}
          </kbd>
        ))}
      </span>
    </span>
  );
}

export function PageHeader({
  title,
  lead,
  actions,
}: {
  title: ReactNode;
  lead?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="mb-8 flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div className="min-w-0 max-w-prose">
        <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
        {lead && <p className="mt-1.5 text-ink-2">{lead}</p>}
      </div>
      {actions && <div className="flex flex-wrap gap-2">{actions}</div>}
    </header>
  );
}

export function Section({
  title,
  lead,
  actions,
  children,
  id,
}: {
  title: ReactNode;
  lead?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  id?: string;
}) {
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} id={id} className="mt-10 first:mt-0">
      <div className="mb-3 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-b border-rule pb-2">
        <h2 id={headingId} className="text-md font-semibold">
          {title}
        </h2>
        {actions}
      </div>
      {lead && <p className="mb-3 max-w-prose text-sm text-ink-2">{lead}</p>}
      {children}
    </section>
  );
}

export type MarkState = "ok" | "open" | "attention" | "alert" | "off";

/** Each state has its own shape, so it reads without colour too. */
export function CheckMark({ state, className = "" }: { state: MarkState; className?: string }) {
  const box =
    state === "ok"
      ? "fill-ink stroke-ink"
      : state === "alert"
        ? "fill-signal stroke-signal"
        : state === "attention"
          ? "fill-none stroke-caution"
          : "fill-none stroke-rule-strong";
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false" className={`size-4 flex-none ${className}`}>
      <rect x="1.25" y="1.25" width="13.5" height="13.5" rx="2.5" strokeWidth="1.5" className={box} />
      {state === "ok" && (
        <path d="m4.5 8.2 2.3 2.3 4.7-5" fill="none" strokeWidth="1.8" className="stroke-on-ink" />
      )}
      {state === "attention" && <path d="M5 8h6" strokeWidth="1.8" className="stroke-caution" />}
      {state === "alert" && <path d="M8 4.5v4.2M8 11v.3" strokeWidth="1.8" className="stroke-ink" />}
      {state === "off" && <path d="M5 8h6" strokeWidth="1.5" className="stroke-rule-strong" />}
    </svg>
  );
}

export function CheckRow({
  state,
  label,
  value,
  detail,
  action,
}: {
  state: MarkState;
  label: ReactNode;
  value?: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <li className="flex items-start gap-3 py-2">
      <CheckMark state={state} className="mt-[3px]" />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
          <div className="flex min-w-0 flex-1 basis-60 items-baseline">
            <span className="min-w-0">{label}</span>
            {value && (
              <>
                <span className="leader" aria-hidden="true" />
                <span
                  className={`shrink-0 text-right text-sm font-medium tabular-nums ${
                    state === "attention"
                      ? "text-caution"
                      : state === "alert"
                        ? "text-signal-text"
                        : "text-ink-2"
                  }`}
                >
                  {value}
                </span>
              </>
            )}
          </div>
          {action && <div className="shrink-0">{action}</div>}
        </div>
        {detail && <p className="mt-1 text-sm text-ink-2">{detail}</p>}
      </div>
    </li>
  );
}

export function EmptyState({
  title,
  body,
  action,
}: {
  title: ReactNode;
  body: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="rounded-panel border border-dashed border-rule-strong px-6 py-8">
      <h3 className="font-semibold">{title}</h3>
      <p className="mt-1 max-w-prose text-ink-2">{body}</p>
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

export function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <p role="alert" className="rounded-ctl bg-signal-wash px-3 py-2 text-sm text-ink">
      {children}
    </p>
  );
}
