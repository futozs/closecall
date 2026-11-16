# crates/core

The contract every other crate and the UI build on. Public API changes ripple into `engine`,
`platform-*`, `app/src-tauri`, the TS bindings and the config schema, so treat them as breaking.

## Rules

- Pure: no I/O, no threads, no `std::env`, no clock reads, no OS dependencies, no `cfg(target_os)`.
  Time comes in as a parameter (`now_ms`); planning is deterministic.
- Config field names and enum spellings (`snake_case` via serde) are user-facing API. Renaming one
  breaks existing `config.toml` files: bump `CONFIG_VERSION` and add a migration with a snapshot test.
- Errors and outcomes carry keys (`reason_key`, `note_key`, `ValidationIssue.code`), never prose.
- Text matching goes through `text::fold` (NFKD, combining marks stripped, lowercased), so `szamla`
  matches `Számla`. Hostnames are lowercased and punycoded before list lookups.

## Planner invariants

Each one has a proptest; keep them passing and add one when you add a rule.

- A protected target never appears in a plan.
- A fallback never gets more destructive: Close → Hide → Minimize → NotifyOnly.
- When several rules hit one target, the strongest action wins (Quit > Close > Hide > Minimize >
  NotifyOnly) and the most data-preserving unsaved policy wins (Hide > SkipAndNotify > SaveThenClose > Discard).
- `Discard` appears only when a rule asked for it.
- `Confirm::Countdown` is valid only for `IdleLongerThan` and `ItemOpenLongerThan` (`trigger_allows_confirm`).
- Steps are ordered by `Phase`: Conceal, CloseTabs, CloseDocuments, Notify.

## After changing a type

```sh
cargo test -p closecall-core                    # unit, proptest, insta snapshots
cargo test -p closecall-core export_bindings    # rewrites app/src/bindings/*.ts
```

Regenerate `schema/config.schema.json` from `Config::json_schema()` when a config type changes, and
update [docs/config.md](../../docs/config.md). Review insta snapshot changes before accepting them.
