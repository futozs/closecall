# Status

Living document. Update it at the end of every work session: change what is no longer true, add
what you learned, keep it short. Last updated: 2026-11-16.

## Current state

Pre-alpha. v0.1 (macOS), milestone M1 "skeleton" is code-complete on `main`; nothing is released and
nothing has run on a real share yet. The core contract, lists, engine with the scripted fake
platform, the macOS adapter, the Tauri shell, the settings UI, CI and the docs are merged, plus the
ignore list for always-on capturers and own monitors ([ADR-0011](decisions/0011-ignore-list-share-ui-display-connected.md), proposed).

## What works (checked in CI or locally on 2026-11-16)

- `cargo fmt`, `cargo clippy -D warnings` and `cargo test --workspace`: core 144, engine 8 unit and
  51 scenarios, lists 10, platform-macos 76 unit tests, app 5.
- UI: `bun run check`, Biome, Vitest (43), Playwright (48, desktop and mobile, EN and HU), the
  production build. `check-i18n` and `check-message-keys` pass: every key Rust can emit has text.
- `closecall diag` (stand-alone) waits for the providers' first inventory and reports real counts.
- Engine behaviour covered by scenarios: share, mirroring and display-connected guards; a guard that
  comes on without firing (debounced restart, pause ending, resume) sweeps what is open; batches
  only see targets with the same action, plus the run's protected targets; a left-open target is
  announced once per guard; restores are their own runs (`restore_of`), document restores get 10 s;
  activity forgets titles, URLs, paths and evidence after the restore window; a rejected
  `config.toml` shows a banner and is kept as `config.rejected.toml` on the first save; concurrent
  saves leave one whole file; permission requests return `not_while_sharing` during a share and never
  block the engine; a cold-start panic waits up to 3 s for the inventory.

## Unverified on a device

Nothing below has run against real apps with a stably signed bundle. The
[test matrix](test-matrix.md) lists the checks.

- SkyLight watcher for anything but Zoom and `screencapture`; sharing-bar cues of `macos.share_ui`
  (layers, sizes, positions for Chrome/Edge/Brave/Arc/Helium, Teams, Slack, Zoom, Webex; whether
  Zoom's `CptHost` runs only while sharing). Tune with `cargo run -p closecall-platform-macos
  --example share_ui 30`.
- The DisplayLink case end to end: dock attached, DisplayLink ignored, then a Zoom or Meet share.
  A read-only probe on the owner's Mac saw the watcher true while `com.displaylink.DisplayLinkUserAgent`
  runs, and two LG monitors with different model codes (23481, 23482).
- Sheet handling: the save-alert shape check (three buttons, default and cancel, no form controls),
  Terminal's terminate sheet, a pre-existing Print or Export sheet, Hungarian system language.
- The Chromium close script's own-tab check, the tab list's error re-raise, Safari's URL-based keys,
  per-element AX timeouts, the poller-cache snapshot.
- AX and Apple Events while the screen is locked; restore of tabs and documents.

## Known problems

- While an ignored capturer runs, every capture the OS can't attribute is ignored, even with no dock
  attached; only shares with a recognised sharing bar start a run then. Discord has no known bar.
  The fix is SkyLight's per-process capture notification; its event number is unverified.
- `KNOWN_CAPTURE_APPS` has DisplayLink only. Luna Display and Duet Display bundle ids are unchecked;
  remote-desktop agents are left out on purpose.
- `confirm = { countdown = … }` is a silent wait: no notice, no Keep or Do it now. The UI and docs say
  so; implementing the buttons needs IPC commands, a notice and a Status field.
- Will-sleep and lid runs are not waited for (no `IORegisterForSystemPower`); the Mac may sleep first.
- Without an Automation grant a browser is not covered at all, while its capability rows still read
  Full. Opera, Dia and other unsupported browsers are plain app windows: a Close rule on app windows
  would close a whole browser window.
- A tab Hide hides the whole browser (skipped when a protected target shares it); there is no
  per-window minimize for tabs yet.
- The single-instance socket is `/tmp/hu_futozs_closecall_si.sock` in the shared `/tmp` (plugin
  limit); another local user could claim it first. Needs a patched plugin or a peer-uid check.
- The opener scope allows the bundled lists' source URLs (raw.githubusercontent.com) besides the
  repo; the `verify-no-network` skill allows only the repo. The owner decides which one changes.
- No crash marker yet: apps hidden at a crash stay hidden until unhidden by hand.
- The tray reuses the "screen shared" string for an armed external display.
- Display names come from `NSScreen`; a display mirrored from the moment it connected has none and
  can be ignored by vendor and model only.
- `cargo deny check` was not run locally (not installed); CI runs it.
- Tauri 3 alphas depend on `reqwest` unconditionally; staying on 2.x is required by the no-network rule.
- CC BY-SA 4.0 list compatibility with GPL-3.0 needs confirming before the first release. No
  trademark search for "Close Call" yet.

## Next

1. M2 on a real Mac with a stably signed bundle: run the test matrix, starting with the DisplayLink
   case, the sharing-bar cues and the sheet rows; record results there and in
   [platforms/macos.md](platforms/macos.md).
2. Spike: AX and Apple Events while locked; SkyLight's per-process capture notification.
3. Countdown Keep / Do it now, `IORegisterForSystemPower` for sleep, Partial capabilities for
   browsers without a grant, a crash marker for hidden apps.
4. Owner decisions: ADR-0011 (proposed), the opener scope for list sources, the single-instance
   socket location.
