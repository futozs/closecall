import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";
import enNative from "../../../locales/en/native.json";
import enUi from "../../../locales/en/ui.json";
import huNative from "../../../locales/hu/native.json";
import huUi from "../../../locales/hu/ui.json";

type Tree = { [key: string]: string | Tree };

function flatten(tree: Tree, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") out.set(path, value);
    else for (const [k, v] of flatten(value, path)) out.set(k, v);
  }
  return out;
}

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{\{(\w+)\}\}/g)].map((match) => match[1] ?? "").sort();
}

const files = [
  { name: "ui.json", en: flatten(enUi), hu: flatten(huUi) },
  { name: "native.json", en: flatten(enNative), hu: flatten(huNative) },
];

describe.each(files)("$name", ({ en, hu }) => {
  test("en and hu have identical key sets", () => {
    expect([...hu.keys()].sort()).toEqual([...en.keys()].sort());
  });

  test("every translation uses the same placeholders", () => {
    for (const [key, text] of en) {
      expect(placeholders(hu.get(key) ?? ""), key).toEqual(placeholders(text));
    }
  });

  test("Hungarian uses Hungarian quotes, never straight or English ones", () => {
    for (const [key, text] of hu) {
      expect(text, key).not.toMatch(/["“]/);
      expect(text, key).not.toMatch(/[õû]/);
    }
  });
});

test("native strings never carry titles, URLs or paths", () => {
  for (const [key, text] of flatten(enNative)) {
    for (const name of placeholders(text)) expect(["count"], key).toContain(name);
  }
});

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === "bindings" ? [] : sourceFiles(path);
    return /\.tsx?$/.test(entry.name) ? [path] : [];
  });
}

test("every literal key used in the UI exists in en/ui.json", () => {
  const keys = flatten(enUi);
  const exists = (key: string) => keys.has(key) || keys.has(`${key}_one`) || keys.has(`${key}_other`);
  const missing: string[] = [];
  for (const file of sourceFiles(join(import.meta.dirname, "../../src"))) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(/\bt\(\s*"([a-z0-9_.-]+)"/g)) {
      const key = match[1] ?? "";
      if (!exists(key)) missing.push(`${file}: ${key}`);
    }
  }
  expect(missing).toEqual([]);
});
