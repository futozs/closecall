// Fails when locales/en and locales/hu do not have the same namespace files and the same keys.
// Usage: bun scripts/ci/check-i18n.ts [locales dir]
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? join(import.meta.dir, "../../locales"));

function fail(message: string): never {
  console.error(`check-i18n: ${message}`);
  process.exit(1);
}

function show(path: string): string {
  const rel = relative(process.cwd(), path);
  return rel && !rel.startsWith("..") ? rel : path;
}

function keysOf(value: unknown, prefix: string, out: Set<string>): Set<string> {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const [key, child] of Object.entries(value)) {
      keysOf(child, prefix ? `${prefix}.${key}` : key, out);
    }
  } else {
    out.add(prefix);
  }
  return out;
}

function loadLanguage(language: string): Map<string, Set<string>> {
  const dir = join(root, language);
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    fail(`${show(dir)} does not exist; expected i18next JSON files in locales/<language>/`);
  }
  const namespaces = new Map<string, Set<string>>();
  for (const file of readdirSync(dir).filter((name) => name.endsWith(".json")).sort()) {
    const path = join(dir, file);
    let data: unknown;
    try {
      data = JSON.parse(readFileSync(path, "utf8"));
    } catch (error) {
      fail(`${show(path)} is not valid JSON: ${error instanceof Error ? error.message : error}`);
    }
    namespaces.set(file, keysOf(data, "", new Set()));
  }
  if (namespaces.size === 0) fail(`${show(dir)} contains no .json files`);
  return namespaces;
}

const en = loadLanguage("en");
const hu = loadLanguage("hu");
const problems: string[] = [];

for (const file of new Set([...en.keys(), ...hu.keys()])) {
  const enKeys = en.get(file);
  const huKeys = hu.get(file);
  if (!enKeys || !huKeys) {
    problems.push(`${file} exists only in locales/${enKeys ? "en" : "hu"}`);
    continue;
  }
  for (const key of enKeys) if (!huKeys.has(key)) problems.push(`hu/${file}: missing "${key}"`);
  for (const key of huKeys) if (!enKeys.has(key)) problems.push(`en/${file}: missing "${key}"`);
}

if (problems.length > 0) {
  fail(`locale key sets differ (${problems.length}):\n  ${problems.join("\n  ")}`);
}
console.log(`check-i18n: en and hu match (${en.size} namespaces).`);
