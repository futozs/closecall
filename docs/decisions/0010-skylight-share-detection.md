---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# Private SkyLight API for macOS share detection, failing open

## Context and Problem Statement

macOS has no public API that tells an app another app started capturing the screen. Screen-share
start is our main trigger. How do we detect it?

## Considered Options

- Poll the private `SLSIsScreenWatcherPresent()` from SkyLight, loaded with `dlopen`/`dlsym`
- The private push callback `SLSRegisterNotifyProc` (events 1502/1503)
- Heuristics: meeting-app processes, localized share toolbars read over AX
- ScreenCaptureKit (needs Screen Recording, rejected by [0003](0003-never-request-screen-recording.md))

## Decision Outcome

Chosen option: "Poll `SLSIsScreenWatcherPresent()`", at 4 Hz while a meeting app runs or a call is
likely and 1 Hz otherwise, with `SLSRegisterNotifyProc` behind an experimental flag. It needs no
permission and the symbol exists on macOS 27.2.

- **Fail open.** If the symbol is missing, the signal source reports `Unavailable` and the capability
  report says "Screen-share detection unavailable on this macOS version". Projector detection, the
  hotkey, lock/sleep cleanup and the call-likely pre-flight keep working.
- **Noise calibration.** If the watcher reads true while no share is known (DisplayLink docks,
  always-on recorders, OBS with a display source), the source is marked `Noisy` and demoted to Low
  confidence, and the user is told why.

### Consequences

- Good, because no permission prompt and sub-second latency in meeting mode.
- Bad, because a private API can change in any macOS release. Re-check the symbol on every macOS beta
  as a release gate.
- Bad, because it is confirmed only for Zoom (macOS 26.6.1) and `screencapture -V`. Teams, Meet,
  Discord, OBS, QuickTime and AirPlay are unverified until the [test matrix](../test-matrix.md) runs.
  Mirroring is detected separately through public CoreGraphics display APIs.
- Bad, because detection is reactive: the first frames of a share can go out before cleanup finishes.

### Confirmation

The signal source has a unit test for the missing-symbol path. Device results are recorded per macOS
version in the test matrix.

## More Information

[platform-macos.md §5.2](../research/platform-macos.md), [feasibility-review.md](../research/feasibility-review.md),
[architecture-recommendation.md §7](../research/architecture-recommendation.md).
