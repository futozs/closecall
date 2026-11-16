---
name: add-provider
description: Add or extend a platform provider, signal source or capability (enumerate tabs/windows/documents, close/hide/minimize, unsaved-state detection, share/lock/sleep/lid/idle signals, permissions) in crates/platform-macos, platform-windows, platform-linux or platform-fake. Use when a task implements or changes the Provider, SignalSource or PermissionBroker traits for an OS, or adds support for a new browser or document app.
---

# Add a provider or capability

1. **The traits are the contract.** `crates/core/src/platform.rs` (`Provider`, `SignalSource`,
   `PermissionBroker`, `Platform`) and the model types are fixed. If you need a change, stop and
   propose it to the owner instead of editing `core`.
2. **Read the area guide**: `crates/platform-macos/AGENTS.md` for macOS, and the facts in
   `docs/platforms/<os>.md` and `docs/research/platform-*.md`.
3. **Implement it** in `crates/platform-<os>`:
   - The provider that discovers a target is the one that acts on it; `Handle`s are opaque to everyone else.
   - Respect the deadline you are given. `reverify` re-reads one target; `execute` returns a
     `StepOutcome` and never panics.
   - Unsupported means `Support::Unsupported`. Partial means `Support::Partial { note_key }` with a key
     added to `locales/en` and `locales/hu` ("Firefox: titles only, no per-tab close").
   - Never prompt for a permission outside `PermissionBroker::request`. Never log titles, URLs or paths.
4. **Fake it too**: add the same capability to `crates/platform-fake` and an engine scenario that
   exercises it, so CI covers the behaviour on any OS.
5. **A new OS permission** needs an ADR in `docs/decisions/`, a row in `PRIVACY.md` and
   `docs/platforms/<os>.md`, onboarding strings in both languages, and the `Permission` enum in core
   (a contract change: ask first).
6. **Docs**: update the capability table in `docs/platforms/<os>.md`, add rows to
   `docs/test-matrix.md`, and make sure `closecall diag` reports the new capability.
7. **Verify**: `cargo clippy -p closecall-platform-<os> --all-targets -- -D warnings`, the crate's
   tests, `cargo test` for engine scenarios. Device behaviour needs a real machine (on macOS, a
   stably signed bundled `.app`); say in the PR what you could not run.
