import type { Dur } from "./bindings/Dur";

export type DurUnit = "s" | "m" | "h" | "d";

export function parseDur(dur: Dur): { value: number; unit: DurUnit } | null {
  const match = /^(\d+)([smhd])$/.exec(dur.trim());
  if (!match?.[1] || !match[2]) return null;
  const unit = match[2];
  if (unit !== "s" && unit !== "m" && unit !== "h" && unit !== "d") return null;
  return { value: Number(match[1]), unit };
}

const UNIT_NAMES = { s: "second", m: "minute", h: "hour", d: "day" } as const;

export function formatDur(dur: Dur, locale: string): string {
  const parsed = parseDur(dur);
  if (!parsed) return dur;
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: UNIT_NAMES[parsed.unit],
    unitDisplay: "long",
  }).format(parsed.value);
}

export function formatNumber(value: number, locale: string): string {
  return new Intl.NumberFormat(locale).format(value);
}

function sameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function formatWhen(ms: number, locale: string, now: Date = new Date()): string {
  const date = new Date(ms);
  const options: Intl.DateTimeFormatOptions = sameDay(date, now)
    ? { hour: "numeric", minute: "2-digit" }
    : { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" };
  return new Intl.DateTimeFormat(locale, options).format(date);
}

export function isMac(platform: string): boolean {
  return platform === "macos";
}

const MAC_SYMBOLS: Record<string, string> = {
  ctrl: "⌃",
  control: "⌃",
  alt: "⌥",
  option: "⌥",
  shift: "⇧",
  cmd: "⌘",
  command: "⌘",
  super: "⌘",
  meta: "⌘",
  commandorcontrol: "⌘",
  cmdorctrl: "⌘",
};

const MODIFIER_ORDER = ["⌃", "⌥", "⇧", "⌘"];

export function hotkeyKeys(accelerator: string, mac: boolean): string[] {
  const parts = accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (!mac) return parts.map((part) => (part.toLowerCase() === "commandorcontrol" ? "Ctrl" : part));
  const modifiers: string[] = [];
  const keys: string[] = [];
  for (const part of parts) {
    const symbol = MAC_SYMBOLS[part.toLowerCase()];
    if (symbol) modifiers.push(symbol);
    else keys.push(part.length === 1 ? part.toUpperCase() : part);
  }
  modifiers.sort((a, b) => MODIFIER_ORDER.indexOf(a) - MODIFIER_ORDER.indexOf(b));
  return [...modifiers, ...keys];
}

const NAMED_CODES: Record<string, string> = {
  Space: "Space",
  Escape: "Escape",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
};

/** Uses `code`, not `key`: on macOS ⌥ changes `key`, and the layout must not matter. */
export function acceleratorFromEvent(event: {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}): string | null {
  let key: string | undefined;
  if (/^Key[A-Z]$/.test(event.code)) key = event.code.slice(3);
  else if (/^Digit\d$/.test(event.code)) key = event.code.slice(5);
  else if (/^F\d{1,2}$/.test(event.code)) key = event.code;
  else key = NAMED_CODES[event.code];
  if (!key) return null;
  const modifiers = [
    event.ctrlKey && "Ctrl",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Super",
  ].filter((part): part is string => typeof part === "string");
  if (modifiers.length === 0) return null;
  return [...modifiers, key].join("+");
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}
