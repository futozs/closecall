---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# Never request the macOS Screen Recording permission

## Context and Problem Statement

On macOS, window titles from `CGWindowListCopyWindowInfo` and any screen capture need the Screen
Recording permission. Screen-share detection with ScreenCaptureKit would need it too. Do we ask for it?

## Considered Options

- Never request it; get titles from Accessibility (AX)
- Request it for `CGWindowList` titles
- Request it for ScreenCaptureKit-based detection

## Decision Outcome

Chosen option: "Never request it", because AX already gives titles and documents. Screen Recording
brings recurring re-consent prompts and the purple capture indicator on our own app, and capturing
the screen ourselves would trip our own share detector ([0010](0010-skylight-share-detection.md)).

### Consequences

- Good, because one fewer scary permission, and no capture indicator.
- Good, because our share signal is not polluted by our own capture.
- Bad, because some apps hide off-Space windows from AX. `CGWindowList` still shows that such windows
  exist (owner PID, no title), so we can at least report them.
- Bad, because we cannot classify on-screen pixels; the v0.6 image classifier reads image files instead.

### Confirmation

Code review of `crates/platform-macos`: no `CGRequestScreenCaptureAccess`, no ScreenCaptureKit, no
`CGWindowListCreateImage`. The permission list in [platforms/macos.md](../platforms/macos.md) has no Screen Recording row.

## More Information

[platform-macos.md §1.2, §6](../research/platform-macos.md).
