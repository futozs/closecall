# Roadmap

Order and scope, not dates. Details: [research/architecture-recommendation.md §16](research/architecture-recommendation.md).

## v0.1: macOS (in progress)

Goal: a student installs it, grants Accessibility and Automation once, and from then on a screen
share, a projector, the lid, a lock or the hotkey closes or hides their sensitive tabs, documents and
apps without losing work.

- **M1, skeleton**: workspace and crates; `core` model, matcher, planner, config and schema; `lists`
  snapshot pipeline; `engine` with `platform-fake`; menu-bar shell and settings window; English and
  Hungarian plumbing; CI (fmt, clippy, tests, cargo-deny, Biome, bindings drift, no-network runtime
  test); spike: do AX and Apple Events work while the screen is locked?
- **M2, inventory and actions**: apps, AX windows and documents with dirty state; JXA tabs for
  Chrome, Brave, Edge, Vivaldi, Arc; AppleScript for Safari; Firefox detection from the session file;
  document closing through AppleScript with an AX fallback; close, hide, minimize, notify, opt-in
  quit; onboarding and permission health.
- **M3, triggers**: panic (hotkey, tray, CLI); SkyLight watcher with adaptive polling and noise
  calibration; mirroring and projector; call-likely pre-flight; will-sleep, lid, lock, away, idle,
  item age with countdown; share-guard.
- **M4, hardening and release**: all four unsaved policies with role-based sheet handling; activity
  panel with "why was this closed?" and in-memory restore; capability report and `closecall diag`;
  [test matrix](test-matrix.md) run and latency published; Developer ID signing, notarization, DMG,
  Homebrew tap.

Exit criteria: the test matrix passes or has a documented limitation; zero `AF_INET`/`AF_INET6`
syscalls in CI; Hungarian strings reviewed by a native speaker; no data loss in any unsaved-document scenario.

Not in v0.1: Firefox per-tab close, the browser extension, calendar, image classification, a
persisted journal, Windows, Linux.

## v0.2: Windows 10 and 11

Win32 and UIA inventory, Office COM, ConsentStore share detection with share-picker early warning,
display topology, session and power triggers. Microsoft Store MSIX, SignPath-signed NSIS, winget.
See [platforms/windows.md](platforms/windows.md).

## v0.3: Linux X11, Wayland-ready

X11 control; logind, UPower, AT-SPI, `/proc`, PipeWire share detection and the CLI on every session.
On Wayland v0.3 runs degraded but useful. `.deb`, `.rpm`, AppImage, AUR. See [platforms/linux.md](platforms/linux.md).

## v0.4: companion browser extension

MV3 extension for Chromium browsers and Firefox with a native-messaging host over a local socket or
named pipe (never TCP): exact URLs for every tab, per-tab close, private windows when allowed, push
updates for share-guard, an extra share signal from `getDisplayMedia`, opt-in history cleanup.

## v0.5: Wayland window control

KWin script, then wlr-foreign-toplevel (Sway, Hyprland, niri, labwc), then COSMIC, then a GNOME Shell extension.

## v0.6: opt-in on-device image classification

Off by default. A bundled local model classifies image files open in viewers; no screen capture.

## Later

Safari Web Extension; calendar pre-flight; an encrypted persisted journal behind Touch ID or Windows
Hello; a read-only managed config for schools; more languages through Weblate.
