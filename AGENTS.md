# AGENTS.md

Close Call is a local-only desktop utility (Rust + Tauri 2 + React/TypeScript) that closes or hides
sensitive browser tabs, documents and apps when a screen share starts, a projector is connected, the
lid closes, the machine was locked for long, an item was open too long, or on a panic hotkey.
macOS first (v0.1), then Windows (v0.2) and Linux (v0.3).

## Invariants

Never break these. If a task seems to need it, stop and ask the owner.

- **No network, ever.** No HTTP clients, sockets (loopback included), telemetry, crash reporters,
  update checks, remote fonts or CDNs, in Rust or in the webview. IPC is stdio, Unix sockets or named
  pipes only. See [ADR-0002](docs/decisions/0002-no-network-ever.md).
- **Never log, persist or notify titles, URLs or file paths.** They may appear only inside our own
  window and in the user's own config. The restore journal lives in memory.
- **Never lose unsaved work.** `Dirty::Unknown` counts as dirty. Never drive a Save panel, never
  force-quit, never escalate a fallback (Close → Hide → Minimize → NotifyOnly only).
  See [ADR-0007](docs/decisions/0007-default-unsaved-policy.md).
- **Never request Screen Recording** on macOS. See [ADR-0003](docs/decisions/0003-never-request-screen-recording.md).
- **Never prompt for a permission while a share or projector is active.**
- **Protected targets always win** (meeting apps and sites, the capturing app, our own windows).

## Map

Read the nested `AGENTS.md` before editing under a path that has one.

| Path | What | Guide |
|---|---|---|
| `crates/core` | Pure domain model, matcher, planner, config, platform traits, IPC types. The contract. | [crates/core/AGENTS.md](crates/core/AGENTS.md) |
| `crates/lists` | Built-in categories and bundled domain lists | - |
| `crates/engine` | Inventory index, trigger fusion, scheduler, executor, journal | [crates/engine/AGENTS.md](crates/engine/AGENTS.md) |
| `crates/platform-macos` | macOS adapter (AX, Apple Events, SkyLight, IOKit) | [crates/platform-macos/AGENTS.md](crates/platform-macos/AGENTS.md) |
| `crates/platform-windows`, `crates/platform-linux` | v0.2 / v0.3 adapters, stubs for now | - |
| `crates/platform-fake` | Scripted inventory and signals for tests, UI dev and demos | - |
| `app/src-tauri`, `app/src` | Tauri shell (tray, commands, hotkey) and React settings UI | [app/AGENTS.md](app/AGENTS.md) |
| `locales/{en,hu}` | UI and native strings, one source for both | - |
| `lists/` | Plain-text list snapshots plus `manifest.toml` (source, licence, date, sha256) | - |
| `schema/config.schema.json` | Generated from `Config`, never hand-edited | - |
| `docs/` | Architecture, config reference, per-OS facts, decisions, status | - |

## Dependency rules

- `core` depends on nothing OS-specific: no I/O, no threads, no clock, no `cfg(target_os)`.
- `platform-*` depend only on `core`.
- `engine` depends on `core`, `lists` and exactly one `platform-*`, picked by `cfg(target_os)`;
  feature `fake` swaps in `platform-fake`.
- `app/src-tauri` depends on `engine` and is the only crate that depends on Tauri.
- New Rust deps go in the crate's own `Cargo.toml` with an explicit version. Do not edit the root
  `Cargo.toml` unless the task is about the workspace. No new dependency when std or an existing one covers it.

## Commands

```sh
bun install                              # JS deps (root workspace + app)
bun run dev                              # Tauri dev build with the UI on localhost:1420
bun run check                            # TypeScript typecheck of app/
cargo build                              # default members: core, lists, engine, platform-fake
cargo test                               # same members
cargo test -p closecall-core export_bindings   # rewrite app/src/bindings/*.ts (ts-rs)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo deny check                         # licences + banned network crates
bun x biome check .                      # lint + format for TS/JSON
bun x vitest run                         # UI unit tests (from app/)
bun x playwright test                    # UI tests against the web build with mocked IPC (from app/)
```

`cargo build -p closecall-platform-macos` and `-p closecall-app` build the non-default members.

## Conventions

- **Comments only where the why is not obvious.** No comments that restate the code, no section
  dividers, no doc comment on every item. A short doc comment on a public item only when its name and
  type do not already say it. `// SAFETY:` on every `unsafe` block is required (that is a why).
- **Rust never builds user-facing prose.** Engine events, outcomes and errors carry message keys and
  parameters (`reason_key`, `note_key`, `ValidationIssue.code`); the UI renders them from `locales/`.
- **Every user-visible string lives in `locales/en` and `locales/hu`.** Hungarian copy is written in
  natural Hungarian, not translated: „…” quotes, correct ő/ű, singular noun after a numeral („3 lap”).
- **Conventional commits:** `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, `perf:`, `ci:`.
  No attribution lines.
- **Capabilities are honest.** Unsupported means `Support::Unsupported`, never a fake success.

## Verify a change

1. `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`.
2. Touched core types: regenerate bindings and the schema, commit the diff.
3. Touched `app/`: `bun run check`, Biome, Vitest; for UI changes look at it at desktop width and at
   the window's minimum size, in English and Hungarian.
4. Touched dependencies, logging, IPC, capabilities, CSP or storage: run the checks in
   [.claude/skills/verify-no-network/SKILL.md](.claude/skills/verify-no-network/SKILL.md).
5. Say which OSes you could not verify. Platform behaviour needs a real device; record it in
   [docs/test-matrix.md](docs/test-matrix.md).

## Gotchas

- **macOS TCC grants are keyed to bundle id `hu.futozs.closecall` and the code signature.** `bun run dev`
  runs an unbundled binary, so macOS attributes permissions to your terminal or IDE. Test permissions
  with a bundled `.app` signed with a stable identity (Apple Development or self-signed), never ad hoc.
  Reset with `tccutil reset Accessibility hu.futozs.closecall` and `tccutil reset AppleEvents hu.futozs.closecall`.
- Never send an Apple Event to an app that is not running: it launches the app.
- AX and Apple Events never run on the main thread; always set a timeout.
- The owner's dev machine has little free disk. In a git worktree, point `CARGO_TARGET_DIR` at the
  main checkout's `target/` so all worktrees share one build cache. Never `cargo clean`; no release
  builds unless asked. Cargo may wait on the target-dir lock while another build runs; that is expected.
- `crates/core` functions still marked `todo!()` panic; check [docs/status.md](docs/status.md) before relying on one.

## Project memory

| File | Update it when |
|---|---|
| [docs/status.md](docs/status.md) | At the end of every work session: what works, what's next, known problems. |
| [docs/decisions/](docs/decisions/README.md) | A decision is made or reversed that someone would otherwise re-argue: new permission, new dependency class, new trigger type, a changed default or invariant. New MADR file, next number; never rewrite an accepted one, supersede it. |
| [docs/platforms/](docs/platforms/macos.md) | A capability, permission or limit is verified, found or changed on an OS. |
| [docs/test-matrix.md](docs/test-matrix.md) | After a device test run. |
| [docs/config.md](docs/config.md) | A config field, default or enum value changes. |
| [PRIVACY.md](PRIVACY.md) | What the app reads or stores changes. |

Background research (dated, may be stale) is in [docs/research/](docs/research/README.md); the
architecture recommendation there is the design source of truth, [docs/architecture.md](docs/architecture.md) the condensed current version.
