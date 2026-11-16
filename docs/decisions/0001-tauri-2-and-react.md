---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# Tauri 2 with a React UI

## Context and Problem Statement

The app has to read and act on windows, tabs and documents through each OS's accessibility and
automation APIs (AX and Apple Events, Win32/UIA/COM, D-Bus/AT-SPI/Wayland) on three operating
systems, with a small settings UI. The author knows TypeScript and React and wants to learn Rust.
Which application stack?

## Considered Options

- Tauri 2.12 (Rust core, React + Vite UI in the system webview)
- Swift / AppKit (macOS only)
- Electron
- Tauri 3 (alpha)
- Next.js static export as the UI layer

## Decision Outcome

Chosen option: "Tauri 2.12 with React 19 and Vite 8", because Rust reaches every OS API we need
directly, one codebase covers all three OSes, the webview's capabilities are explicit and small, and
the binaries stay small. Vite builds a static SPA; Next.js server features are dead weight in a webview.

### Consequences

- Good, because the domain core is plain Rust that compiles and tests on any OS.
- Good, because Tauri's CSP and capability files let us state the webview's reach in a few lines.
- Bad, because the UI runs in the OS webview runtime (WKWebView, WebView2, WebKitGTK), which updates
  itself outside our control. See [threat-model.md](../threat-model.md).
- Bad, because Swift would be the more natural fit on macOS alone; we accept FFI through `objc2`.
- Stay on 2.x: every Tauri 3 alpha depends on `reqwest` unconditionally, which [0002](0002-no-network-ever.md) bans.

### Confirmation

`app/src-tauri` is the only crate that depends on `tauri`; reviewed in PRs.

## More Information

[stack.md](../research/stack.md), [architecture-recommendation.md §1–2](../research/architecture-recommendation.md),
[feasibility-review.md](../research/feasibility-review.md) (the Tauri 3 `reqwest` finding).
