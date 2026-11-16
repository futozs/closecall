// Fails when Rust can emit a message key that locales/en/ui.json or locales/hu/ui.json lacks.
// Rust sends keys, never prose, so a missing key shows up in the window as a raw code.
// Scans string literals shaped like reason.*, note.*, config.error.*, config.problem.*, preset.*
// and the old macos.(reason|note|health).* spelling, plus the bare validation codes and preset
// rule ids that crates/core/src/config.rs turns into config.error.<code> and preset.<id>.
// Usage: bun scripts/ci/check-message-keys.ts [repo root]
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? join(import.meta.dir, "../.."));
const SOURCES = ["crates", "app/src-tauri/src"];
const CONFIG = "crates/core/src/config.rs";
const LANGUAGES = ["en", "hu"];

const KEY = /"((?:reason|note|preset|config\.error|config\.problem|macos\.(?:reason|note|health))\.[A-Za-z0-9_.-]+)"/g;
const PUSH_CODE = /\.push\([^;]*?,\s*"([a-z][a-z0-9_]*)"\s*,?\s*\)/gs;
const ARM_CODE = /(?:=>\s*|\{\s*)"([a-z][a-z0-9_]*)"\s*[,}]/g;
const PRESET_ID = /preset_rule\(\s*"([^"]+)"/g;

function fail(message: string): never {
  console.error(`check-message-keys: ${message}`);
  process.exit(1);
}

function rustFiles(dir: string, out: string[]): string[] {
  for (const entry of readdirSync(dir)) {
    if (entry === "target" || entry === "node_modules" || entry.startsWith(".")) continue;
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) rustFiles(path, out);
    else if (entry.endsWith(".rs")) out.push(path);
  }
  return out;
}

function leafKeys(value: unknown, prefix: string, out: Set<string>): Set<string> {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const [key, child] of Object.entries(value)) leafKeys(child, prefix ? `${prefix}.${key}` : key, out);
  } else {
    out.add(prefix);
  }
  return out;
}

/** i18next resolves plurals from `<key>_one`/`_other`, so those count as the key. */
function has(keys: Set<string>, key: string): boolean {
  return keys.has(key) || keys.has(`${key}_one`) || keys.has(`${key}_other`);
}

const used = new Map<string, Set<string>>();
function use(key: string, file: string) {
  const files = used.get(key) ?? new Set<string>();
  files.add(relative(root, file));
  used.set(key, files);
}

for (const dir of SOURCES) {
  const path = join(root, dir);
  if (!existsSync(path)) fail(`${dir} does not exist under ${root}`);
  for (const file of rustFiles(path, [])) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(KEY)) use(match[1], file);
  }
}

const configPath = join(root, CONFIG);
const config = readFileSync(configPath, "utf8");
for (const pattern of [PUSH_CODE, ARM_CODE]) {
  for (const match of config.matchAll(pattern)) use(`config.error.${match[1]}`, configPath);
}
for (const match of config.matchAll(PRESET_ID)) use(`preset.${match[1]}`, configPath);
if (![...used.keys()].some((key) => key.startsWith("config.error."))) {
  fail(`found no validation codes in ${CONFIG}; the patterns in this script need updating`);
}

const problems: string[] = [];
for (const language of LANGUAGES) {
  const file = join(root, "locales", language, "ui.json");
  let data: unknown;
  try {
    data = JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    fail(`${relative(root, file)} is unreadable: ${error instanceof Error ? error.message : error}`);
  }
  const keys = leafKeys(data, "", new Set());
  for (const [key, files] of [...used].sort(([a], [b]) => a.localeCompare(b))) {
    if (!has(keys, key)) problems.push(`${language}: missing "${key}" (used in ${[...files].join(", ")})`);
  }
}

if (problems.length > 0) {
  fail(`${problems.length} message key(s) Rust can emit have no text:\n  ${problems.join("\n  ")}`);
}
console.log(`check-message-keys: all ${used.size} keys Rust can emit exist in ${LANGUAGES.join(" and ")}.`);
