---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# OS automation for browser tabs before a browser extension

## Context and Problem Statement

Tabs are the most common sensitive item. A companion WebExtension with native messaging gives exact
URLs and per-tab close in every browser, but needs store publishing and a second install step. OS
automation (AppleScript/JXA on macOS, UIA on Windows, AT-SPI on Linux) needs no install, but is
uneven: titles everywhere, URLs of every tab only for Chromium and Safari on macOS. Which comes first?

## Considered Options

- OS automation in v0.1–v0.3, companion extension in v0.4
- Extension only, from v0.1
- Extension first, OS automation as fallback

## Decision Outcome

Chosen option: "OS automation first, extension in v0.4", because it ships value in v0.1 without a
store review, and on macOS it already covers Chrome, Brave, Edge, Vivaldi, Arc and Safari fully.
The native-messaging protocol types are defined early so the extension slots in later.

### Consequences

- Good, because v0.1 works with no extra install.
- Bad, because Firefox on macOS is detection-only in v0.1 (session file, about 15 s stale, no private
  windows) and the action is hiding the app. Selecting a tab and sending ⌘W would be visible to the audience.
- Bad, because on Windows and Linux background tabs give titles only until v0.4; URLs are cached when
  a tab becomes active, and rules fall back to title matching.
- Bad, because each browser needs its own macOS Automation consent.

### Confirmation

The capability report states per browser what works ("Firefox: titles only, no per-tab close").

## More Information

[architecture-recommendation.md §1 (decisions 5, 6), §9](../research/architecture-recommendation.md),
[platform-macos.md §2](../research/platform-macos.md), [platform-windows-linux.md §1.3, §2.4](../research/platform-windows-linux.md).
