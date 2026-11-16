# Architecture

The condensed, current picture. For the reasoning and the evidence behind it, read
[research/architecture-recommendation.md](research/architecture-recommendation.md) (written under
the working title „Bezártam?”; names and the default unsaved policy have changed since, see
[decisions/](decisions/README.md)).

## The idea in one paragraph

Providers keep a warm index of what is open: browser tabs, document windows, app windows, processes.
Signal sources report what is happening: a share started, a display was mirrored, the lid closed, the
screen locked. Trigger fusion turns signals and timers into a fired trigger. The pure planner matches
the index against the user's rules and returns an ordered plan. The executor re-checks each target
and asks the provider that found it to act. A run report goes to the UI; the journal for "restore"
stays in memory.

```
SignalSources ──Signal──▶ TriggerFusion ──FiredTrigger──┐
Scheduler (item age, idle, away timers) ──▶ TriggerFusion│
                                                         ▼
Providers ──InventoryEvent──▶ Inventory index ──▶ core::plan() ──▶ Executor ──▶ Journal ──▶ UI / notification
```

## Crates

| Crate | Role | May depend on |
|---|---|---|
| `closecall-core` (`crates/core`) | Model, matcher, planner, config and schema, platform traits, IPC types. No I/O, no threads, no clock, no OS code. | serde, toml, schemars, regex, globset, url, psl, ts-rs |
| `closecall-lists` (`crates/lists`) | Built-in categories, bundled domain lists from `lists/` | core |
| `closecall-engine` (`crates/engine`) | Index, fusion, scheduler, executor, journal, config load/save, capability report | core, lists, one platform crate |
| `closecall-platform-macos` | AX, Apple Events (OSAKit), NSWorkspace, IOKit, CoreGraphics, SkyLight | core, objc2 family |
| `closecall-platform-windows` (v0.2) | Win32, UIA, Office COM, WTS, ConsentStore | core, windows |
| `closecall-platform-linux` (v0.3) | X11, logind, UPower, AT-SPI, PipeWire; Wayland backends in v0.5 | core |
| `closecall-platform-fake` | Scripted inventory and signals for tests, UI work and demos | core |
| `closecall-app` (`app/src-tauri`) | Tauri shell: tray, settings window, commands, hotkey, autostart, CLI args | engine |

The engine picks its platform crate with `cfg(target_os)`; feature `fake` swaps in `platform-fake`.
The UI is React 19 + Vite 8 in `app/src`, talking to Rust only through `app/src/ipc.ts`, with types
generated into `app/src/bindings/` by ts-rs. The command table is in `crates/core/src/ipc.rs`.

## Model

- **Target**: something we can act on. `TargetKind` is `BrowserTab`, `DocumentWindow`, `AppWindow`
  or `Process`. Each target carries its `ProviderId`; only that provider may interpret its opaque
  `Handle`s and act on it.
- **Rule**: `when` (triggers) + `targets` (kinds) + `match` / `except` (matchers) + `action` +
  `on_unsaved` + `confirm`. Reference: [config.md](config.md).
- **Category**: domains, list ids, apps, title keywords and file globs under one id.
- **Plan**: ordered `Step`s, each with the target, the resolved action, the phase, an optional
  `fell_back_from`, and `because` (rule id plus evidence, for "why was this closed?").
- **Capability**: (provider, app, target kind, action) → `Full`, `Partial { note_key }` or
  `Unsupported`. The engine shows these as the capability report ("What I can see on this computer").

## Planner semantics

1. **Arm**: only rules whose `when` contains the fired trigger. `item_open_longer_than` is scoped to
   the one target that aged.
2. **Select**: target kind and `match` hit, `except` does not, and the target is not protected.
   Protection always wins.
3. **Merge**: strongest action wins (Quit > Close > Hide > Minimize > NotifyOnly); most
   data-preserving unsaved policy wins (Hide > SkipAndNotify > SaveThenClose > Discard).
4. **Resolve**: if the provider cannot do the action, fall back only toward less destructive ones
   (Close → Hide → Minimize → NotifyOnly).
5. **Order** by phase: Conceal (instant app hides, our own UI), CloseTabs (batched per browser
   window), CloseDocuments (may involve sheets), Notify.
6. **Confirm**: countdowns only on idle and item-age triggers.

## Engine behaviour

- The index is event-driven (AXObserver, WinEvent/UIA, EWMH, AT-SPI) with a full refresh every 30 s.
- Each provider is an actor on its own OS thread; calls carry deadlines (300 ms conceal, 1 s per tab
  batch, 2 s per document with a sheet). Every step ends `Done`, `FellBack`, `Skipped` or `Failed`.
- **Re-verify** each target right before acting (title, URL or path still match).
- **Debounce**: no repeat of the same trigger within 10 s, except Panic; share stop debounced 3 s.
- **Share-guard**: while a share or mirroring is active, new or changed targets are planned and acted
  on immediately, so a tab that navigates to a bank mid-share gets closed.
- **Away**: on lock or will-sleep, record the time; fire when the threshold passes while still locked,
  or on wake before unlock. If the OS does not require a password on wake, fire at wake and say so.

## Unsaved documents

`Dirty` is `Clean`, `Dirty` or `Unknown`, and `Unknown` counts as dirty. The default policy is
`save_then_close` with fallbacks to hiding ([ADR-0007](decisions/0007-default-unsaved-policy.md)).
Sheet and dialog buttons are picked by role, never by localized label. A Save panel is never driven.

## Strings

Rust never builds user-facing prose. Events, outcomes and capability notes carry keys and parameters;
`locales/{en,hu}/ui.json` (React, i18next) and `locales/{en,hu}/native.json` (tray, notifications) hold
the text. Notifications show counts only.

## No network

No `AF_INET`/`AF_INET6` socket, ever. Enforced by cargo-deny bans, Clippy `disallowed-types`, Biome
restricted globals, the webview CSP, the absence of network Tauri plugins, and a runtime `strace` test
in CI. See [ADR-0002](decisions/0002-no-network-ever.md) and [PRIVACY.md](../PRIVACY.md).

## Per-OS detail

[platforms/macos.md](platforms/macos.md), [platforms/windows.md](platforms/windows.md),
[platforms/linux.md](platforms/linux.md). What we defend against: [threat-model.md](threat-model.md).
