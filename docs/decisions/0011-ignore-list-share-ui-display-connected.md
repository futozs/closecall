---
status: proposed
date: 2026-11-16
decision-makers: futozs
---

# Ignore list for always-on capture and own monitors, sharing-bar detection, `display_connected`

## Context and Problem Statement

A DisplayLink dock drives its monitors by capturing the screen for as long as DisplayLink Manager
runs. `SLSIsScreenWatcherPresent()` is one bit with no app name, so on a docked Mac it reads true all
day. The launch-time noise calibration of [0010](0010-skylight-share-detection.md) only covered a
watcher that was on at launch with no meeting app running, and it was cleared after one false read:
a dock plugged in later, a replug or Slack running at launch gave a High-confidence share and kept
share-guard on all day. A DisplayLink monitor in mirror mode also counted as a projector. How do we
let the user say "this capture and these monitors are mine" and still catch real shares?

## Considered Options

- An ignore list in the config: capture apps and displays, matched by bundle id or name and by
  display name or EDID vendor and model
- Smarter noise calibration only (recalibrate on display changes and dock attach)
- SkyLight's per-process capture notification (`SLSScreenWatcherNotificationForEach` inside an
  `SLSRegisterNotifyProc` callback)

## Decision Outcome

Chosen option: "An ignore list", with a second share signal that does not depend on the watcher.

- `[ignore] capture_apps`: while a listed app runs, a capture the OS cannot attribute counts as that
  app's and starts nothing. The recommended preset lists DisplayLink only. Remote-desktop agents are
  left out on purpose: their sessions are real shares.
- `[ignore] displays`: listed monitors are never a projector or an external display.
- `macos.share_ui`: sharing bars of meeting apps and Chromium browsers, read from `CGWindowList`
  owners, layers and bounds (no titles, no Screen Recording, consistent with
  [0003](0003-never-request-screen-recording.md)), Zoom's `CptHost` process, and an AX title only for
  a bar out of its usual place. Medium confidence, with the capturing app named.
- New trigger `display_connected` for an external display that is not ignored. Not in the
  recommended preset; Settings can add it to rules armed by `display_mirrored`.

### Consequences

- Good, because a docked Mac no longer looks like a permanent share, and the user decides what is
  theirs, in Settings or in `config.toml`.
- Bad, because while an ignored capturer runs, every unattributed capture is ignored, even with no
  dock attached; only shares with a recognised sharing bar are caught then. Discord has no known bar.
- Bad, because the sharing-bar thresholds are unverified on a device.
- Neutral: the per-process notification would remove the first drawback; its event number is
  unverified, so it stays a spike in the [test matrix](../test-matrix.md).

### Confirmation

Engine scenarios in `crates/engine/tests/scenarios/ignore.rs` cover ignored captures and displays;
`share_ui` has unit tests for its cues; device results go in the test matrix.

## More Information

[docs/platforms/macos.md](../platforms/macos.md#displaylink-and-other-always-on-capture),
[docs/config.md](../config.md#ignore). This complements the noise calibration in
[0010](0010-skylight-share-detection.md); it does not replace it.
