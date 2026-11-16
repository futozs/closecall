# AI-ready open-source repository blueprint for "Bezártam?" / "Did I close it?"

Research date: **2026-11-16**. All versions below were checked against crates.io, the npm registry and GitHub releases on that date. Claims come from primary docs (Claude Code, OpenAI Codex, Cursor, GitHub, Tauri, Agent Skills, MADR, OpenSSF) unless marked *secondary* or *unverified*. My web-search budget ran out near the end. The last few checks (CodeQL Rust, SLSA level) were done by fetching the docs pages directly, and I've flagged the points that are still open.

**Location (per your request):** create the repo at `/Users/futozs/Developer/personal/<repo-name>`. That folder already holds your other personal projects. This research step created no files. The tree below uses the slug `bezartam`. The name is your call; an ASCII slug is easier for non-Hungarian contributors to type.

---

## 0. Decisions at a glance

| Area | Recommendation |
|---|---|
| Agent instructions | **One `AGENTS.md` at the root plus a few nested `AGENTS.md` files. No `CLAUDE.md`, no `.cursor/rules`, no `.github/copilot-instructions.md` for now.** Put Claude-only notes in `.claude/rules/`. |
| Skills | Keep them canonically in **`.agents/skills/<name>/SKILL.md`** (read by Codex, Cursor and Copilot). Expose them to Claude Code through a local directory plugin, the pattern astral-sh/ruff uses. |
| Claude Code | Commit `.claude/settings.json` (permissions, two hooks, the local skills plugin), 2-3 project subagents, and one small `.claude/rules/claude-code.md`. |
| Project memory | `docs/adr/` (MADR 4.0), `GLOSSARY.md`, a short `docs/ARCHITECTURE.md` codemap, `docs/platform/*` and `docs/research/*`. Skip `llms.txt` and growing "memory log" files. |
| License | **GPL-3.0-or-later** with DCO sign-off and no CLA. MPL-2.0 is the runner-up. |
| Release | Conventional commits, release-please v5, tauri-action v1, `actions/attest` provenance plus SBOM, immutable releases, no auto-updater. |
| Supply chain | cargo-deny (advisories, licenses, **bans on network crates**), Dependabot with cooldown, zizmor, CodeQL (JS/TS and Rust), OpenSSF Scorecard, SHA-pinned actions. |
| Stack baseline | Rust 1.100 / edition 2024, Tauri **2.12.x** (not the 3.0 alphas), React 19.3, TypeScript 7.0, Vite 8, Vitest 5, Bun 1.4, i18next 27 with react-i18next 18. |

---

## 1. Agent instruction files

### 1.1 What each tool actually reads (verified, Nov 2026)

| Tool | Root instructions | Nested files | Skills directories | Notes |
|---|---|---|---|---|
| **Claude Code** (v2.1.277+; current is v2.1.325) | Reads `AGENTS.md` natively **only if there is no `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` in the working directory or above it**. If any of those exist, it reads the CLAUDE.md files only, and they can `@AGENTS.md`-import ([docs](https://code.claude.com/docs/en/memory#agents-md)). | In AGENTS.md mode, a subdirectory's `AGENTS.md` loads when Claude reads a file in that directory. | `.claude/skills/` and plugin skills. Nothing under `.agents/` is read. | `.claude/rules/*.md` (with optional `paths:` globs) load alongside AGENTS.md and don't switch it off. A size target of under 200 lines per file. `/doctor prompt-audit` (v2.1.283+) checks instruction files for staleness and conflicts. |
| **OpenAI Codex** | `AGENTS.override.md`, then `AGENTS.md`, walking from the git root **down to the cwd**, at most one file per directory, concatenated root-first, capped at **32 KiB** by default (`project_doc_max_bytes`) ([docs](https://learn.chatgpt.com/docs/agent-configuration/agents-md)). | Only directories on the root-to-cwd path. Files below the cwd are not auto-loaded. | `.agents/skills` from the cwd up to the repo root; follows symlinks ([docs](https://learn.chatgpt.com/docs/build-skills)). | An optional `agents/openai.yaml` per skill sets `policy.allow_implicit_invocation`. |
| **Cursor** | `AGENTS.md` in the root and subdirectories; nested files combine, and the more specific one wins. `.cursor/rules/*.mdc` is optional ([docs](https://cursor.com/docs/context/rules)). | Yes | `.agents/skills`, `.cursor/skills`, and for compatibility `.claude/skills` and `.codex/skills` ([docs](https://cursor.com/docs/skills)). | Plain `.md` files in `.cursor/rules` are ignored; rules need `.mdc`. |
| **GitHub Copilot** | `.github/copilot-instructions.md` on every surface. `AGENTS.md` in the cloud agent, GitHub.com code review, VS Code chat and the CLI. `CLAUDE.md`/`GEMINI.md` only in the cloud agent and CLI ([support matrix](https://docs.github.com/en/copilot/reference/custom-instructions-support)). | The nearest `AGENTS.md` wins. | `.github/skills`, `.claude/skills`, `.agents/skills` | GitHub.com chat and JetBrains, Visual Studio and Xcode chat read only `copilot-instructions.md`. |
| Gemini CLI, Zed, Junie, Amp, goose and others | AGENTS.md, either natively or via config ([agents.md](https://agents.md/)) | varies | varies | Gemini needs `context.fileName` set to `AGENTS.md` in `.gemini/settings.json`. |

The AGENTS.md format is now stewarded by the Agentic AI Foundation under the Linux Foundation ([agents.md](https://agents.md/)).

### 1.2 Recommendation and how to avoid duplication

1. **`AGENTS.md` is the only hand-written instruction source.** Large Rust/TS repos converge on this: zed, uv, biome, microsoft/vscode, cline and next.js all have a root `AGENTS.md`. ruff and goose keep a `CLAUDE.md` that contains only `@AGENTS.md`.
2. **Ship no `CLAUDE.md` at all.** Since v2.1.277, Claude Code reads root and nested `AGENTS.md` directly. A root `CLAUDE.md` has a hidden cost: with the default `claude-md-or-agents-md` setting, its presence makes Claude **stop reading every nested `AGENTS.md`**. Put Claude-only guidance in `.claude/rules/claude-code.md`, which loads alongside AGENTS.md.
   - Fallback, if some contributors run an older Claude Code: add a root `CLAUDE.md` containing `@AGENTS.md`. Every directory with a nested `AGENTS.md` then also needs a one-line `CLAUDE.md` with `@AGENTS.md`. Use an import rather than a symlink: Claude's Edit tool won't write through symlinks, and Windows clones check symlinks out as text files ([docs](https://code.claude.com/docs/en/memory#share-one-file-with-other-coding-tools)).
   - Tell contributors **not to create `CLAUDE.local.md`**, because it switches AGENTS.md reading off. Personal notes belong in `~/.claude/rules/` or `.claude/settings.local.json`.
3. **Skip `.github/copilot-instructions.md`** until someone actually uses Copilot IDE chat. The cloud agent, PR code review and the CLI already read AGENTS.md. If you add it later, generate it from AGENTS.md with `just sync-agent-docs` and have CI check for drift, rather than hand-maintaining a second copy.
4. **Skip `.cursor/rules/`.** Cursor reads AGENTS.md, nested files included, and the skill directories.
5. **Codex only sees nested files on the root-to-cwd path.** So the root AGENTS.md needs a directory table that says "before editing under X, read X/AGENTS.md".

### 1.3 What to put in AGENTS.md, and what to leave out

The only controlled study so far is [Gloaguen et al., *Evaluating AGENTS.md*, arXiv:2602.11988](https://arxiv.org/abs/2602.11988) (ETH Zurich / LogicStar, ICLR 2026). It found that context files "do not generally improve task success rates", raise inference cost by more than 20%, and that "repository overviews ... are not helpful". They do help when they specify **non-standard practices**. Secondary summaries report LLM-generated files slightly hurting results and human-written ones giving a small gain (about +4%); this is *secondary*. Claude's docs say to target under 200 lines and to move procedures into skills. Codex truncates at 32 KiB.

So AGENTS.md should hold invariants, commands and gotchas. Leave out tutorials, architecture prose and generic style rules. Generic Rust/TS style already lives in your personal `~/.claude/rules/{rust,typescript-react}.md` and your global reviewers. In the repo, state only what is specific to this project.

**Root `AGENTS.md` skeleton (aim for 80-120 lines):**

```markdown
# AGENTS.md

Local-only desktop utility (Rust + Tauri 2 + React/TS) that closes or hides sensitive
tabs, documents and apps on triggers (screen share start, wake, lock timeout, age, hotkey).

## Invariants (never break; ask first if a task seems to need it)
- No network I/O anywhere: no HTTP clients, sockets, telemetry, crash reporters,
  auto-updater, remote fonts or CDNs. deny.toml bans, ESLint/Biome rules and the Tauri CSP
  enforce this; never weaken them. See docs/adr/0002-local-only.md.
- Never log, persist or display window titles, URLs, file paths or document names outside
  the user's own config. Log `Redacted<Item>` only.
- Unsaved documents: default action is skip + notify. Only the per-rule `on_unsaved`
  setting may save, discard or hide. See ADR-0008.
- OS-specific code lives only in crates/platform-*. crates/core, rules, engine and config
  compile on every target without cfg(target_os).
- Every user-visible string lives in locales/ (en + hu). No hard-coded UI text.

## Commands (run `just check` before claiming done)
- `just setup`: toolchains and deps (mise, bun install)
- `just check`: fmt check, clippy -D warnings, biome/oxlint, tsc, typos, tests
- `just test-rust [crate]` / `just test-ui` / `just test-contract` (platform-fake)
- `just dev`: run the desktop app; `just cli -- --dry-run`: what would be closed
- `just schema` (regen schemas/config.schema.json), `just bindings` (regen TS IPC types)
- `just i18n-check`, `just audit-network`

## Map (read the nested AGENTS.md before editing there)
| Path | Purpose | Guide |
| crates/core | domain types + Platform traits | - |
| crates/rules | pure rules engine | crates/rules/AGENTS.md |
| crates/platform-{macos,windows,linux} | OS adapters | each has AGENTS.md |
| apps/desktop | Tauri app + React UI | apps/desktop/AGENTS.md |
| apps/browser-extension | WebExtension + native messaging | its AGENTS.md |
| locales | translations | locales/AGENTS.md |

## Process
- Conventional commits (feat|fix|refactor|docs|test|chore|perf|ci), `git commit -s` (DCO).
- New dependency, new OS permission, new trigger type, or any invariant change needs an
  ADR in docs/adr (skill: write-adr).
- You can only verify the OS you run on. State in the PR which OSes were not verified;
  manual test plans are in docs/platform/test-plans/.

## Gotchas
- (add only things an agent got wrong twice)
```

**Nested `AGENTS.md` files.** Add one only where the rules genuinely differ, and keep each to 15-60 lines:

- `crates/rules/AGENTS.md`: pure functions, no I/O and no clock reads (inject a `Clock`), property tests with proptest, snapshot tests with insta for action plans.
- `crates/platform-macos/AGENTS.md`: objc2 and objc2-* crates only. Every `unsafe` block gets a `// SAFETY:` comment. Every API that needs a TCC permission is listed in `docs/platform/macos.md` and checked via `permissions::status()`; never prompt from adapter code. If you couldn't run the macOS tests, say so.
- `crates/platform-windows/AGENTS.md`: `windows` crate / UI Automation, COM apartment rules, session notifications.
- `crates/platform-linux/AGENTS.md`: X11 vs Wayland capability gaps (return `Unsupported`, never fake success), AT-SPI/D-Bus via zbus/atspi.
- `apps/desktop/AGENTS.md`: React conventions, the IPC types are generated (don't edit `bindings.ts`), capabilities and CSP need an owner review, UI strings come from i18n keys.
- `apps/browser-extension/AGENTS.md`: MV3, no remote code, minimal permissions, a versioned native-messaging protocol in `crates/protocol`.
- `locales/AGENTS.md`: key naming, plural suffixes, Hungarian style (correct ő/ű, Hungarian quote marks „…”, natural phrasing rather than translated English).

---

## 2. Claude Code project setup

### 2.1 `.claude/settings.json` (committed)

Verified syntax ([settings](https://code.claude.com/docs/en/settings), [example team file](https://code.claude.com/docs/en/settings-example), [permissions](https://code.claude.com/docs/en/permissions), [hooks](https://code.claude.com/docs/en/hooks)):

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "permissions": {
    "allow": [
      "Bash(just *)",
      "Bash(cargo check *)", "Bash(cargo clippy *)", "Bash(cargo fmt *)",
      "Bash(cargo nextest *)", "Bash(cargo test *)", "Bash(cargo tree *)", "Bash(cargo deny *)",
      "Bash(bun run *)", "Bash(bun test *)", "Bash(bunx tsc *)"
    ],
    "ask": ["Bash(cargo add *)", "Bash(bun add *)", "Bash(git push *)", "Bash(gh release *)"],
    "deny": [
      "Read(./.env)", "Read(./.env.*)",
      "Read(./**/*.p12)", "Read(./**/*.p8)", "Read(./**/*.key)",
      "Read(./rules/presets/third_party/**/*)"
    ]
  },
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [
        { "type": "command", "if": "Bash(cargo add *)", "command": "\"${CLAUDE_PROJECT_DIR}\"/.claude/hooks/guard-deps.sh" },
        { "type": "command", "if": "Bash(bun add *)",   "command": "\"${CLAUDE_PROJECT_DIR}\"/.claude/hooks/guard-deps.sh" }
      ]
    }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "\"${CLAUDE_PROJECT_DIR}\"/.claude/hooks/fmt.sh", "timeout": 60 }] }]
  },
  "extraKnownMarketplaces": {
    "bezartam-local": { "source": { "source": "directory", "path": ".agents" } }
  },
  "enabledPlugins": { "bezartam@bezartam-local": true }
}
```

Notes:
- Rules are evaluated deny, then ask, then allow. `deny` and `ask` apply immediately. `allow` rules and `extraKnownMarketplaces` wait until each person trusts the folder.
- `guard-deps.sh` exits with code 2 (a blocking error) when someone tries to add a network-capable crate or package (`reqwest`, `hyper`, `ureq`, `tauri-plugin-http`, `tauri-plugin-updater`, `axios`, `sentry*` and similar). It gives early feedback; CI's cargo-deny is the real gate.
- `fmt.sh` runs `cargo fmt --all` plus Biome format once, at Stop time. Formatting after every edit can trigger "file modified since read" errors, which your global `format-queue.sh` already avoids for TS. The global queue doesn't format `.rs`, and contributors don't have it.
- **Leave `attribution` out of the project file.** It's a personal choice and your user settings already handle it. Outside contributors are covered by AI_POLICY.md (section 3.4).
- Don't commit `"sandbox": {"enabled": true}`: `bun install` and `cargo fetch` need the network. It's fine in your local file.

### 2.2 Skills: format, location, and the set for this project

**Format.** These are [Agent Skills](https://agentskills.io/specification) folders containing a `SKILL.md`:
- Portable frontmatter: `name` (at most 64 characters, lowercase and hyphens, must match the folder name) and `description` (at most 1024 characters, saying what the skill does and when to use it). Optional: `license`, `compatibility`, `metadata`, and the experimental `allowed-tools`.
- Claude Code adds `disable-model-invocation`, `user-invocable`, `paths`, `context: fork`, `agent`, `arguments`/`$ARGUMENTS`, `` !`cmd` `` injection, `model` and `effort`. It ignores unknown keys, and lists `description` plus `when_to_use` truncated at 1,536 characters ([docs](https://code.claude.com/docs/en/skills)).
- Keep `SKILL.md` under 500 lines. Put detail in `references/`, scripts in `scripts/`, templates in `assets/`.
- Custom slash commands **have been merged into skills**. `.claude/commands/*.md` still works but can't carry supporting files, so write skills.

**Location.** Use `.agents/skills/` as canonical; Codex, Cursor and Copilot all read it.
- **Option A (recommended), the [astral-sh/ruff](https://github.com/astral-sh/ruff) pattern.** Add `.agents/.claude-plugin/marketplace.json` and `plugin.json`, register `.agents` as a `directory` marketplace in `.claude/settings.json`, and enable the plugin. This needs no symlinks, so it works on Windows. Skills then appear as `/bezartam:add-detection-rule`. Check after the first trust prompt with `claude plugin list` that the plugin auto-enables.
- **Option B, the [vercel/next.js](https://github.com/vercel/next.js) pattern.** `.claude/skills` is a git symlink to `../.agents/skills`. Simpler on macOS/Linux, but breaks on Windows clones without `core.symlinks`, and Cursor/Copilot may list each skill twice because they read both directories.
- For Codex-only invocation control, add `agents/openai.yaml` with `policy.allow_implicit_invocation: false` to side-effecting skills, as ruff and next.js do.
- Add `.agents/skills/README.md` with a catalog table and "adding a skill" rules, modeled on [biome's](https://github.com/biomejs/biome/tree/main/.agents/skills). It keeps universal rules in AGENTS.md and out of skills.

**Skills for this project:**

| Skill | Trigger (description gist) | Invocation | Key contents |
|---|---|---|---|
| `add-platform-adapter` | Add or extend an OS capability (enumerate, close/hide, unsaved-state, screen-share, sleep/lid events) | model + user | Trait first in `crates/core`, then the fake implementation plus contract test, then the OS implementations. Update `docs/platform/capability-matrix.md`. A new OS permission needs an ADR, a PRIVACY.md entry and onboarding strings. |
| `add-detection-rule` | Add a built-in detector or preset (domains, bundle IDs/exe names, file globs, document types) | model + user | Preset file format, `SOURCES.md` licensing for third-party lists, proptest for matchers, `just schema`, docs table |
| `add-trigger` | New trigger type (e.g. lid reopened, lock > N min) | model + user | Event source per OS, debounce, engine wiring, config schema plus migration, an ADR |
| `add-translation` | Add or modify UI strings or a new language | model + user | Key naming, plural suffixes, `references/hu-style.md`, run `just i18n-check` |
| `privacy-review` | Before a PR touching deps, logging, IPC, capabilities, CSP or storage | model + user | Checklist plus `just audit-network`, `cargo tree -i`, grep for logged titles |
| `write-adr` | Record a decision | model + user | Next number, MADR template in `assets/`, update `docs/adr/README.md` |
| `run-desktop` | Launch and smoke-test the app | model + user | `just dev`, granting TCC permissions, test fixtures. Claude's built-in `run` skill looks for a project skill like this first. |
| `release` | Cut a release | **user only** (`disable-model-invocation: true`) | Confirm CI is green, merge the release-please PR, watch release.yml, verify attestations, update the Homebrew cask and winget manifests |

Example (portable frontmatter; Claude-only keys are omitted where they aren't needed):

```markdown
---
name: add-platform-adapter
description: Add or extend an OS capability in crates/platform-* (window/tab/document enumeration, close or hide, unsaved-state detection, screen-share detection, sleep/lid/lock events). Use when a task touches crates/platform-macos, platform-windows, platform-linux or the Platform traits in crates/core.
license: GPL-3.0-or-later
---
1. Extend the trait in crates/core/src/platform/; unsupported means `Capability::Unsupported`, never a panic.
2. Implement it in crates/platform-fake and add a contract test in crates/core/tests/contract.rs.
3. Implement per OS; each `unsafe` gets `// SAFETY:`; no logging of titles or paths.
4. Update docs/platform/capability-matrix.md and docs/platform/<os>.md (permission, min OS, gaps).
5. A new OS permission needs write-adr, a PRIVACY.md row, and onboarding strings in locales/en and locales/hu.
6. Run `just check`; list the OSes you could not verify in the PR.
```

### 2.3 Subagents (`.claude/agents/*.md`, Claude-only)

Frontmatter requires `name` and `description`. Optional fields include `tools`, `disallowedTools`, `model`, `permissionMode`, `skills` (preloads the full skill content), `memory`, `isolation: worktree`, `effort` and `maxTurns` ([docs](https://code.claude.com/docs/en/sub-agents)). You already have generic `rust-reviewer`, `react-reviewer`, `security-reviewer` and `a11y-architect` globally, so the repo only needs project-specific ones. Make each a thin wrapper over a portable skill, so Codex and Cursor users get the same checklist.

- `privacy-auditor`: `tools: Read, Grep, Glob, Bash`, `skills: [privacy-review]`, read-only. Reports file:line findings against PRIVACY.md and ADR-0002. Plugin skills may need the namespaced name (`bezartam:privacy-review`); check this.
- `platform-reviewer`: reviews `crates/platform-*` diffs for unsafe/FFI soundness, permission handling and capability-matrix updates.
- `hu-copy-reviewer`: checks `locales/hu` for natural Hungarian. Optional, since you already have the `magyar-szoveg` skill personally; contributors don't.

Leave `memory:` off at first. Committed agent memory tends to turn into noise.

### 2.4 Project memory: what helps agents

| Artifact | Keep? | Why |
|---|---|---|
| `docs/adr/NNNN-title.md` in **MADR 4.0** format ([MADR](https://adr.github.io/madr/), 4.0.0, 2024-09-17) | **Yes** | Records the *why* that agents can't infer from code. Use `docs/adr/` rather than MADR's default `docs/decisions`, because your installed `domain-modeling` skill (mattpocock-skills) expects `docs/adr/` and `GLOSSARY.md`. |
| `GLOSSARY.md` | **Yes** | One vocabulary for Trigger, Detector, SensitiveItem, Rule, Action, UnsavedPolicy, Adapter, Capability. Prevents naming drift across Rust, TS and docs. |
| `docs/ARCHITECTURE.md` (a [matklad-style codemap](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html)) | **Yes, short** | Codemap plus invariants plus boundaries, for humans. Agents read it on demand. Don't inline it into AGENTS.md: the ETH study found overviews don't help when injected into context. |
| `docs/platform/*.md` and `capability-matrix.md` | **Yes** | Per-OS permissions, minimum versions and gaps. These are the facts agents most often get wrong. |
| `docs/research/YYYY-MM-DD-topic.md` | **Yes** | Dated notes with sources (like this report). Mark anything that may go stale. |
| `llms.txt` | **No** | Meant for websites. 2026 measurements show about 97% of llms.txt files get zero requests ([ppc.land on Ahrefs data](https://ppc.land/llms-txt-adoption-rises-8-8x-but-97-of-files-get-zero-ai-requests/), *secondary*). Revisit only if you build a docs site. |
| `docs/memory/` logs or session journals | **No** | They grow without bound and contradict the code. Claude's auto memory already lives per user under `~/.claude/projects/<project>/memory/`. |

Seed ADRs:
- 0001 Record decisions with MADR
- 0002 Local-only: no network, no telemetry, no auto-updater, and how it's enforced
- 0003 Tauri 2.x plus React/TS; stay on 2.x until 3.0 is stable
- 0004 Platform traits plus per-OS crates plus a fake platform
- 0005 GPL-3.0-or-later plus DCO
- 0006 i18n format shared by Rust and TS
- 0007 Browser tabs via WebExtension plus native messaging
- 0008 Semantics of the unsaved-document policy
- 0009 Config format, schema and storage location (the config file is itself sensitive)
- 0010 Release, signing and distribution channels

### 2.5 Making the repo verifiable by agents (higher payoff than more prose)

- **One command surface:** `just check`, `just test-*`, `just dev`. biome, goose and spacedrive all expose a justfile. Pin tool versions in `mise.toml` and `rust-toolchain.toml`.
- **`crates/platform-fake`:** an in-memory platform loaded from JSON fixtures. The whole trigger, detect, plan and execute pipeline then runs in tests on any OS, Linux CI included, so agents can verify behavior without macOS permissions.
- **Generated, committed contracts:** `schemas/config.schema.json` from Rust types via schemars 1.x, and TS IPC bindings via tauri-specta (2.0.0-rc.29; it's an rc, so ts-rs 12 is the stable fallback). Agents then can't drift the Rust and TS types apart.
- **`--dry-run` CLI:** prints the action plan without touching anything. It's the fastest feedback loop for rule changes.
- **Manual test plans** in `docs/platform/test-plans/` for what CI can't do (starting a share in Zoom, Teams or Meet; lid close). Agents can then say exactly what still needs a human.

---

## 3. Human OSS hygiene

### 3.1 README.md (English) plus README.hu.md

1. Name and one-line purpose.
2. A short demo GIF that contains no real sensitive content.
3. Three-line privacy promise, linking to PRIVACY.md and to "verify it yourself".
4. What it does and **what it doesn't do**:
   - It isn't a security boundary.
   - It can't un-share frames that were already captured.
   - Some capabilities are missing on Wayland.
5. Support matrix (trigger by OS, detector by OS), generated from `capability-matrix.md`.
6. Install: Homebrew cask, winget, Flathub/AppImage, build from source. Include how to verify a download with `gh attestation verify` and `gh release verify-asset` ([gh docs](https://cli.github.com/manual/gh_release_verify)).
7. Which OS permissions it asks for, and why.
8. Quick start with an example rule file.
9. Configuration, triggers, actions and the unsaved-document policy.
10. FAQ.
11. Contributing, translating and the AI policy.
12. License.

### 3.2 PRIVACY.md and the threat model

`PRIVACY.md` covers:
- What is read (per detector and OS), when, and why.
- What is stored: config path per OS, retention, and whether any local action log exists (redacted, off by default).
- What is never done.
- OS permissions table.
- The browser extension's permissions and native-messaging scope.
- **How the no-network rule is enforced** (section 3.11), with links.
- How to verify it yourself: LuLu or Little Snitch on macOS, Windows Firewall logging, OpenSnitch or `ss` on Linux.

`docs/privacy/threat-model.md` covers:
- **Assets:** the open sensitive items, plus **the config file itself**, since a list of "sensitive" domains reveals the user's habits.
- **In-scope adversaries:** the audience of a shared screen or projector, people near an unlocked laptop, a later viewer of a recording, a malicious dependency or fork.
- **Out of scope:** malware or an admin on the device, a compromised OS, forensics.
- **Failure modes:** the race at share start, detector false negatives, a crash while hiding, an unsaved-document prompt left on screen.
- **Mitigations:** pre-meeting "check now", the panic hotkey, fail-closed defaults where possible.

### 3.3 SECURITY.md

- Report through GitHub private vulnerability reporting.
- Scope: IPC/capability escalation, extension-to-host message spoofing, bypassing the no-network rule, leakage of sensitive data into logs, supply chain.
- Best-effort timelines, honestly stated for a solo maintainer: acknowledge within 7 days, 90-day disclosure.
- Supported versions: the latest minor.
- How to verify release integrity.

### 3.4 CONTRIBUTING.md, AI_POLICY.md, CODE_OF_CONDUCT.md

- **CONTRIBUTING:**
  - Setup: `mise install && just setup`.
  - Run `just check` before every PR.
  - Conventional commits, **DCO** (`git commit -s`), no CLA.
  - Say which OSes you tested on.
  - How to add a language.
  - Link to AGENTS.md: agent-facing docs are code and get reviewed like code.
- **AI_POLICY.md**, modeled on [Ghostty's](https://github.com/ghostty-org/ghostty/blob/main/AI_POLICY.md):
  - Disclose the tool and how much it was used.
  - The submitter must understand every line.
  - AI-assisted issues must be edited by a human.
  - No AI-generated media.
  - Maintainers are exempt. This doesn't conflict with your personal no-attribution rule, because the policy applies to outside contributions.
- **CODE_OF_CONDUCT:** Contributor Covenant **3.0** (August 2025, OES; [version page](https://www.contributor-covenant.org/version/3/0/)), with your own reporting contact.

### 3.5 License: GPL-3.0-or-later

| | MIT | Apache-2.0 | MPL-2.0 | **GPL-3.0-or-later** |
|---|---|---|---|---|
| A redistributed fork must publish source for what it ships | No | No | Only modified MPL files; new files can be closed | **Whole program** |
| Explicit patent grant | No | Yes | Yes | Yes |
| Compatible with the Tauri/Rust ecosystem (MIT/Apache deps) | Yes | Yes | Yes | Yes (Apache-2.0 flows one-way into GPLv3) |
| Others can embed your rules crate in closed apps | Yes | Yes | Yes (file-level) | No |
| Precedent among privacy/security desktop tools | - | - | Firefox | [Mullvad VPN app](https://github.com/mullvad/mullvadvpn-app) (GPL-3.0), [LuLu](https://github.com/objective-see/LuLu) (GPL-3.0), [Signal Desktop](https://github.com/signalapp/Signal-Desktop) (AGPL-3.0) |
| [SignPath Foundation](https://signpath.org/terms) free signing eligibility | OSI license, no commercial dual-licensing | same | same | same |

Why GPL-3.0-or-later:
- A privacy tool's value is user trust. Copyleft means anyone who redistributes a modified build, including one with telemetry bolted on, must publish the complete source. Under MPL, the telemetry could sit in a new, closed file.
- AGPL buys nothing here, because there's no network service.
- The usual GPL objection, the Mac App Store, doesn't apply: controlling other apps through the Accessibility API is incompatible with the App Sandbox, so Mac App Store distribution is off the table anyway ([Apple DTS](https://developer.apple.com/forums/thread/756130)).

Caveats:
- It discourages companies from embedding `crates/rules`. If you want that later, relicense that crate to MPL-2.0 **before** you accept outside contributions, because relicensing afterwards needs every contributor's consent.
- License the bundled third-party domain lists separately and record them in `rules/presets/SOURCES.md` (optionally with `REUSE.toml`).

### 3.6 Templates and ownership

- **Issue forms (`.github/ISSUE_TEMPLATE/*.yml`):** bug, false negative ("something sensitive wasn't closed", with a warning not to paste real URLs or titles), platform support, feature request. Add `config.yml` linking Discussions and the security policy.
- **PR template:** `just check` passed; OSes tested; privacy impact (none / new permission / new data read, which needs an ADR); en and hu strings added; docs updated; AI disclosure.
- **CODEOWNERS:** `* @<you>`, plus explicit entries for `deny.toml`, `apps/desktop/src-tauri/capabilities/`, `.github/workflows/`, `PRIVACY.md`, `.agents/`, `AGENTS.md`. It starts to matter once you add a co-maintainer and require code-owner review in rulesets.

### 3.7 Dependency updates

Dependabot is enough for a solo maintainer: it's native and supports `cargo`, `bun` (1.1.39+), `github-actions`, `groups`, `cooldown`, and `multi-ecosystem-groups` ([options reference](https://docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference)). Use weekly schedules, a 7-day `cooldown` against freshly published malicious releases, and group minor and patch updates.

The tricky part is Tauri: the `tauri*` crates and `@tauri-apps/*` npm packages must move together. Group them with `patterns` in each ecosystem, and add a CI step that fails if their major.minor versions differ. If two-PR drift becomes annoying, switch to Renovate, which handles cross-manager `groupName` well.

### 3.8 Commits, changelog, releases

- Conventional commits (you already require them globally), enforced by a `commit-msg` hook (lefthook 2.2 or prek 0.5) and a PR-title check.
- **release-please v5** (`googleapis/release-please-action@v5`) opens a release PR with CHANGELOG and version bump.
  - Single version source: `apps/desktop/package.json`, with `tauri.conf.json` set to `"version": "../package.json"`. Tauri accepts a path to a package.json there ([config reference](https://v2.tauri.app/reference/config/)).
  - Internal crates: `publish = false` with a fixed version.
  - release-please's Rust strategy reportedly can't rewrite inherited `version.workspace = true` (*secondary*), so don't rely on it. Use `release-type: simple` plus `extra-files` with the `json` updater ([customizing](https://github.com/googleapis/release-please/blob/main/docs/customizing.md)), and run a dry run first.
- Changesets is the alternative; biome uses it even for Rust. It costs a manual changeset file per PR, which a solo maintainer doesn't need.

### 3.9 GitHub Actions (all actions SHA-pinned with pinact; `permissions: {}` at the top, as in [Mullvad's supply-chain workflow](https://github.com/mullvad/mullvadvpn-app/blob/main/.github/workflows/rust-supply-chain.yml))

| Workflow | What |
|---|---|
| `ci.yml` | `lint` on ubuntu: rustfmt, clippy `-D warnings`, Biome/oxlint, `tsc`, typos, i18n-check, schema/bindings drift. `test` matrix **macos-latest (arm64), windows-latest, ubuntu-24.04**: nextest and vitest. `build` matrix: `tauri build --debug --no-bundle`. Uses Swatinem/rust-cache v2.9. |
| `supply-chain.yml` | `cargo-deny-action@v2` running `check all`; `actions/dependency-review-action@v5` on PRs; `bun audit` |
| `codeql.yml` | CodeQL for JS/TS (supports TS up to 7.0) and Rust (editions 2021/2024 are listed as supported, [CodeQL docs](https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/); confirm GA status in the Security tab) |
| `zizmor.yml` | Lints the workflows themselves (zizmor 1.32) |
| `scorecard.yml` | `ossf/scorecard-action@v2.5.0`, publishing results and the badge |
| `release-please.yml` | Opens or updates the release PR |
| `release.yml` | On release: `tauri-apps/tauri-action` (`action-v1.0.0`) matrix for macOS arm64 and x64, Windows, Linux (`uploadUpdaterJson: false`, since there's no updater). Then macOS signing and notarization, Windows signing, an SBOM (CycloneDX via cargo-cyclonedx and/or `anchore/sbom-action`), `actions/attest@v4` provenance plus `actions/attest-sbom`, and `SHA256SUMS`. Turn on **immutable releases** ([GA since 2025-10-28](https://github.blog/changelog/2025-10-28-immutable-releases-are-now-generally-available/)). |

Notes:
- `actions/attest-build-provenance` v4 is now a wrapper around `actions/attest` ([README](https://github.com/actions/attest-build-provenance)). Attestations are free for public repos.
- GitHub documents that doing the build *and* attestation inside a reusable workflow helps reach **SLSA v1.0 Build Level 3** ([docs](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/increase-security-rating)).
- Bit-for-bit reproducible builds aren't realistic for signed and notarized macOS/Windows bundles. Aim instead for pinned toolchains, `--locked`, `SOURCE_DATE_EPOCH`, CI-only builds, provenance and an SBOM. Mullvad only has a reproducible-build workflow for Android.

### 3.10 OpenSSF Scorecard

Checks per the [Scorecard docs](https://github.com/ossf/scorecard/blob/main/docs/checks.md):

| Check | How this repo satisfies it |
|---|---|
| Dangerous-Workflow, Token-Permissions | zizmor, `permissions: {}` defaults, no `pull_request_target` checkout of PR code |
| Pinned-Dependencies | Actions pinned by SHA (pinact), lockfiles, `--locked`/`--frozen-lockfile` |
| Dependency-Update-Tool, Vulnerabilities | Dependabot, cargo-deny advisories (RustSec), `bun audit`, dependency-review |
| SAST | CodeQL |
| Security-Policy, License | SECURITY.md, LICENSE |
| Signed-Releases, SBOM, Packaging | Attestations and SBOM published as release assets |
| CI-Tests, Branch-Protection | Required status checks via rulesets |
| Code-Review, Contributors | **Will score low while you're solo.** That's expected; say so in the README rather than gaming it. |
| Fuzzing | proptest in `crates/rules` counts as property-based testing |
| CII-Best-Practices | Optional: the OpenSSF Best Practices "passing" badge once docs are done |

### 3.11 Enforcing "no network" in layers (specific to this project)

1. **Rust:** `deny.toml [bans] deny = [reqwest, hyper, ureq, isahc, curl, surf, tungstenite, tokio-tungstenite, tauri-plugin-http, tauri-plugin-updater, sentry, opentelemetry-otlp, ...]`, modeled on Mullvad's `bans.deny`. Run `cargo tree -i <crate>` on the Tauri baseline before switching these from warn to deny.
2. **TS:** Biome/oxlint restricted globals (`fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `navigator.sendBeacon`), all fonts and assets bundled locally.
3. **Tauri:** a strict CSP (`connect-src` limited to IPC). Minimal `capabilities/*.json`: a window matching no capability gets no IPC access at all. No `http` or `updater` plugins.
4. **CI runtime check** (`just audit-network`, Linux): run the CLI dry run and the engine integration tests under `unshare -rn` plus `strace -f -e trace=socket,connect`, and fail on `AF_INET`/`AF_INET6`. Local IPC (`AF_UNIX`, D-Bus, native messaging) is allowed and documented.
5. **Agents:** the invariant in AGENTS.md, the `guard-deps.sh` hook, and the `privacy-auditor` subagent.

### 3.12 Signing and distribution (affects the repo layout)

- **macOS:**
  - Needs Developer ID signing plus notarization, which means the paid Apple Developer Program. Without it, users hit Gatekeeper warnings.
  - The App Store isn't possible (see 3.5).
  - Distribute as a DMG plus a Homebrew cask.
- **Windows:**
  - Azure Artifact Signing (formerly Trusted Signing) issues public-trust certificates to **individuals only in the US and Canada** ([quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart); forum reports are *secondary*). As a Hungarian individual you're probably not eligible.
  - **SignPath Foundation** signs for free but requires: an OSI license without commercial dual-licensing, an already released project, a code-signing policy page, MFA, and manual approval of every signing ([terms](https://signpath.org/terms)).
  - Plan: unsigned early releases, documented honestly, then SignPath once there's some reputation. Distribute via winget.
- **Linux:** AppImage, .deb and .rpm from tauri-action; Flathub later, since it builds from source.
- **No auto-updater:** updates come through package managers (brew, winget, Flathub). That respects the no-network rule.

---

## 4. Proposed directory tree

```text
bezartam/                                   # /Users/futozs/Developer/personal/bezartam
├── AGENTS.md                               # single cross-tool agent guide: invariants, commands, map of nested guides
├── README.md                               # English landing page: purpose, privacy promise, support matrix, install, verify
├── README.hu.md                            # Hungarian README (Hungarian-first audience)
├── PRIVACY.md                              # data inventory, OS permissions, how "no network" is enforced, how to verify
├── SECURITY.md                             # private vulnerability reporting, scope, timelines, release verification
├── CONTRIBUTING.md                         # setup, `just check`, conventional commits, DCO, per-OS testing expectations
├── AI_POLICY.md                            # disclosure and understanding rules for AI-assisted outside contributions
├── CODE_OF_CONDUCT.md                      # Contributor Covenant 3.0 with your contact
├── GLOSSARY.md                             # domain vocabulary shared by Rust, TS, docs and agents
├── CHANGELOG.md                            # generated by release-please
├── LICENSE                                 # GPL-3.0-or-later full text
├── Cargo.toml                              # Rust workspace: members, [workspace.dependencies], [workspace.lints] (unsafe_code=deny by default)
├── Cargo.lock                              # committed; CI builds with --locked
├── rust-toolchain.toml                     # pinned stable (1.100.x), rustfmt, clippy, macOS/Windows/Linux targets
├── package.json                            # bun workspaces root (apps/*); scripts only delegate to just
├── bun.lock                                # JS lockfile; CI uses --frozen-lockfile
├── justfile                                # the one command surface for humans, CI and agents
├── mise.toml                               # pins just, bun, cargo-deny, cargo-nextest, typos, etc.
├── deny.toml                               # cargo-deny: advisories, GPL-compatible license allowlist, network-crate bans, sources
├── biome.json                              # TS/JSON lint and format, including restricted network globals
├── typos.toml                              # spell checking with Hungarian words allowlisted
├── lefthook.yml                            # pre-commit fmt/typos, commit-msg conventional-commit check
├── release-please-config.json              # simple release type plus extra-files (apps/desktop/package.json)
├── .release-please-manifest.json           # current version
├── .editorconfig                           # whitespace and EOL rules for all editors and agents
├── .gitattributes                          # EOL normalization, generated files marked linguist-generated
├── .gitignore                              # target/, node_modules/, dist/, .claude/settings.local.json, CLAUDE.local.md
├── .agents/
│   ├── .claude-plugin/
│   │   ├── marketplace.json                # exposes .agents as a local Claude Code plugin marketplace (ruff pattern)
│   │   └── plugin.json                     # plugin manifest; skills appear as /bezartam:<skill>
│   └── skills/
│       ├── README.md                       # skill catalog and rules for adding skills
│       ├── add-platform-adapter/SKILL.md   # trait, fake, OS implementation, capability matrix workflow
│       ├── add-detection-rule/SKILL.md     # presets, matchers, schema, tests
│       ├── add-trigger/SKILL.md            # new trigger end to end
│       ├── add-translation/                # SKILL.md plus references/hu-style.md
│       ├── privacy-review/SKILL.md         # pre-PR privacy checklist plus audit commands
│       ├── write-adr/                      # SKILL.md plus assets/madr-template.md
│       ├── run-desktop/SKILL.md            # launch, grant permissions, smoke-test
│       └── release/                        # SKILL.md (user-only) plus agents/openai.yaml (no implicit invocation)
├── .claude/
│   ├── settings.json                       # shared permissions, hooks, local skills plugin
│   ├── rules/claude-code.md                # Claude-only notes; loads alongside AGENTS.md
│   ├── agents/
│   │   ├── privacy-auditor.md              # read-only privacy audit, preloads privacy-review
│   │   ├── platform-reviewer.md            # unsafe/FFI/permission review for crates/platform-*
│   │   └── hu-copy-reviewer.md             # Hungarian string quality (optional)
│   └── hooks/
│       ├── guard-deps.sh                   # blocks adding network-capable crates or packages (exit 2)
│       └── fmt.sh                          # Stop hook: cargo fmt plus biome format
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                          # lint, 3-OS test matrix, debug build
│   │   ├── supply-chain.yml                # cargo-deny, dependency-review, bun audit
│   │   ├── codeql.yml                      # SAST for TS and Rust
│   │   ├── zizmor.yml                      # workflow security lint
│   │   ├── scorecard.yml                   # OpenSSF Scorecard
│   │   ├── release-please.yml              # release PR automation
│   │   └── release.yml                     # tauri-action builds, signing, SBOM, attestations
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug.yml                         # bug report form
│   │   ├── false-negative.yml              # "something sensitive wasn't closed" (no real URLs or titles)
│   │   ├── platform-support.yml            # OS or app compatibility report
│   │   ├── feature.yml                     # feature request
│   │   └── config.yml                      # links to Discussions and the security policy
│   ├── PULL_REQUEST_TEMPLATE.md            # checks, OSes tested, privacy impact, i18n, AI disclosure
│   ├── CODEOWNERS                          # owner on everything, plus explicit critical paths
│   └── dependabot.yml                      # cargo, bun, github-actions; cooldown; tauri groups
├── crates/
│   ├── core/                               # domain types and Platform traits; no OS dependencies, no I/O
│   ├── rules/                              # pure rules engine (match, age, action plan) with AGENTS.md, proptest, insta
│   ├── config/                             # load/save/migrate, per-OS paths, schemars schema generation
│   ├── engine/                             # orchestrator: triggers, detect, plan, execute; dry-run; redacted logs
│   ├── platform-macos/                     # objc2 AX/AppKit/CoreGraphics adapter with AGENTS.md
│   ├── platform-windows/                   # windows crate / UI Automation / session events adapter with AGENTS.md
│   ├── platform-linux/                     # X11/Wayland/AT-SPI via zbus/atspi with AGENTS.md; gaps return Unsupported
│   ├── platform-fake/                      # deterministic in-memory platform plus JSON fixtures for tests and agents
│   ├── protocol/                           # serde types for IPC and native messaging; TS generated from here
│   ├── i18n/                               # Rust-side loader for locales/ (tray, notifications)
│   ├── native-host/                        # optional: native-messaging bridge, browser to app over local IPC
│   └── cli/                                # optional `bezartam` CLI: --dry-run, list items, validate config
├── apps/
│   ├── desktop/
│   │   ├── AGENTS.md                       # UI/IPC conventions, generated files, capability rules
│   │   ├── package.json                    # single source of the app version
│   │   ├── vite.config.ts                  # Vite 8 configuration
│   │   ├── tsconfig.json                   # TS 7 strict configuration
│   │   ├── src/                            # React UI: onboarding/permissions, rules editor, triggers, settings
│   │   │   └── bindings.ts                 # generated IPC types (never hand-edit)
│   │   └── src-tauri/                      # Tauri 2 app crate (workspace member)
│   │       ├── Cargo.toml                  # app crate manifest
│   │       ├── build.rs                    # tauri_build::build()
│   │       ├── tauri.conf.json             # identifier, strict CSP, "version": "../package.json"
│   │       ├── capabilities/default.json   # minimal IPC permissions (owner review required)
│   │       ├── icons/                      # generated by `tauri icon`
│   │       └── src/                        # main.rs (untouched), lib.rs, commands/, tray.rs, hotkey.rs
│   └── browser-extension/                  # optional WXT MV3 extension: tab detection via native messaging, with AGENTS.md
├── locales/
│   ├── AGENTS.md                           # key naming, plurals, Hungarian style rules
│   ├── README.md                           # how to add a language (Weblate later)
│   ├── en/                                 # app.json, tray.json, notifications.json (i18next JSON v4)
│   └── hu/                                 # Hungarian translations, same keys
├── rules/
│   └── presets/                            # built-in category packs (banking, medical, mail, adult) plus SOURCES.md licensing
├── schemas/
│   └── config.schema.json                  # generated JSON Schema for user configs (editor autocomplete, validation)
├── xtask/                                  # cargo xtask: i18n-check, schema/bindings generation, version alignment
└── docs/
    ├── ARCHITECTURE.md                     # short codemap, invariants, boundaries
    ├── adr/                                # MADR 4.0 decisions, NNNN-title.md, plus a README index
    ├── platform/                           # macos.md, windows.md, linux.md, capability-matrix.md, test-plans/
    ├── privacy/                            # threat-model.md, network-audit.md
    ├── research/                           # dated research notes with sources
    ├── user/                               # configuration, triggers, actions, unsaved-document policy, FAQ
    └── dev/                                # testing, release, signing, i18n pipeline
```

**i18n design.**
- Shared files in i18next JSON v4 under `locales/<lang>/<ns>.json`, with react-i18next 18 in the UI.
- Rust reads the same files through `crates/i18n` for the tray and notification strings. Either use a small serde loader or rust-i18n 4.x with a custom `Backend`; rust-i18n's default `%{name}` interpolation differs from i18next's `{{name}}`, so pick one syntax and check both sides in a spike.
- Avoid Fluent: `@fluent/react` hasn't shipped a release since 2023-08.
- Weblate supports i18next JSON v4 when you're ready for community translators ([formats](https://docs.weblate.org/en/latest/formats.html)).

---

## 5. Version snapshot (checked 2026-11-16)

| Item | Version |
|---|---|
| Rust | 1.100.0 (2026-11-12), edition 2024 |
| tauri crate | 2.12.5 (3.0.0-alpha.6 published 2026-11-06; no stable 3.0 announced. The [Tauri 2.12 announcement](https://v2.tauri.app/blog/) is dated 2026-09-26.) |
| @tauri-apps/cli and /api | 2.12.4 |
| tauri-action | action-v1.0.0 |
| Tauri plugins | global-shortcut 2.4.1, notification 2.8.1, autostart 2.7.0, single-instance 2.5.5, store 2.5.1, log 2.10.2 |
| Frontend | React 19.3.1, TypeScript 7.0.2, Vite 8.3.8, Vitest 5.1.0, Biome 2.6.8, oxlint 1.93.0, Bun 1.4.4, WXT 0.21.5 |
| i18n | i18next 27.1.3, react-i18next 18.0.2, rust-i18n 4.2.4 |
| Rust libraries | schemars 1.2.3, proptest 1.12.0, insta 1.49.0, tauri-specta 2.0.0-rc.29, ts-rs 12.0.1, objc2 0.6.5 / objc2-* 0.3.2, windows 0.62.2, uiautomation 0.25.1, atspi 0.30.0, zbus 6.3.0, x11rb 0.14.0 |
| Tooling | cargo-deny 0.20.2 (action v2.1.1), cargo-auditable 0.7.7, cargo-dist 0.34.0, typos 1.52.2, zizmor 1.32.0, pinact 5.0.2, just 1.59.0, mise 2026.11.9, lefthook 2.2.2, prek 0.5.11 |
| Actions | checkout v7.0.1, attest v4.2.2, attest-sbom v4.1.0, scorecard-action v2.5.0, release-please-action v5.0.0, dependency-review-action v5.0.0, rust-cache v2.9.3, anchore/sbom-action v0.24.3 |
| Claude Code | v2.1.325 (native AGENTS.md reading since 2.1.277) |

---

## 6. Suggested bootstrap order

1. **Day 1:**
   - Root files: LICENSE, AGENTS.md, justfile, mise.toml, rust-toolchain.toml, deny.toml.
   - Workspace with `core`, `rules`, `platform-fake`.
   - `ci.yml` and `supply-chain.yml`.
   - ADRs 0001-0005.
   - `.claude/settings.json` and `.agents/` with `write-adr` and `add-detection-rule`.
2. **macOS spike:**
   - `platform-macos` plus `docs/platform/macos.md` plus the `add-platform-adapter` skill.
   - CLI `--dry-run`.
3. **Product shape:**
   - Tauri app shell, permissions onboarding, rules editor.
   - i18n (en and hu) plus `add-translation`.
   - Config schema.
4. **Shipping:**
   - PRIVACY.md, SECURITY.md and the threat model finalized.
   - release-please and release.yml with attestations.
   - Scorecard, macOS notarization.
5. **Expansion:** Windows and Linux adapters, the browser extension plus native host, Weblate.
