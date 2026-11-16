# Status

Living document. Update it at the end of every work session: change what is no longer true, add
what you learned, keep it short. Last updated: 2026-11-16.

## Current state

Pre-alpha. v0.1 (macOS), milestone M1 "skeleton", in progress. Nothing is released.

The scaffold commit set up the workspace, the `crates/core` contract and an empty Tauri shell. A wave
of parallel work on top of it (core logic, lists, engine with the fake platform, the macOS adapter,
the UI, CI and tooling, docs) is being merged; update this file as those land.

## What works

- The Cargo workspace (Rust 1.99.1, edition 2024) and the bun workspace with `app/`.
- `crates/core` compiles and defines the full contract: model, config with defaults, platform traits,
  IPC types. `Dur` parsing and formatting are tested.
- `cargo test -p closecall-core` passes and writes the TypeScript bindings to `app/src/bindings/`
  (verified 2026-11-16, no diff against the committed files).
- Config TOML in the shape documented in [config.md](config.md) deserializes into `Config`
  (checked against the examples on 2026-11-16).
- `app/src-tauri/tauri.conf.json` sets the bundle id `hu.futozs.closecall`, a strict CSP,
  `withGlobalTauri: false` and `freezePrototype`. The app build itself was not checked in this session.

## What doesn't work yet

- `text::fold`, `matcher::evaluate`, `matcher::app_matches`, `plan::armed`, `plan::plan`,
  `Config::validate` and `Config::recommended` are `todo!()` and panic. `Config::from_toml` calls
  `validate`, so it panics too.
- `lists`, `engine`, `platform-*` and `platform-fake` are empty.
- No UI beyond a placeholder, no tray, no hotkey, no locales.
- No CI, `deny.toml`, `clippy.toml`, Biome config or generated `schema/config.schema.json` yet.

## Next

1. Finish M1: core logic with proptest invariants, list snapshots and manifest, engine on the fake
   platform, i18n plumbing, CI including the no-network runtime test.
2. Spike: do AX and Apple Events work while the screen is locked? The away-before-unlock design depends on it.
3. Start M2 on a real Mac with a stably signed bundled build (see [platforms/macos.md](platforms/macos.md)).

## Known problems and open questions

- Tauri 3 alphas depend on `reqwest` unconditionally; staying on 2.x is required by the no-network rule.
- Linux will need both zbus 5 and 6 (atspi, the notification plugin, ashpd); allow both in cargo-deny.
- CC BY-SA 4.0 list compatibility with GPL-3.0 needs confirming before the first release.
- No trademark search for "Close Call" yet.
- Every device question in the "Spikes to settle" table of the [test matrix](test-matrix.md) is open.
- The config path per OS in [config.md](config.md) assumes the engine uses the bundle id as the
  directory name; confirm once the engine's config loader lands.
