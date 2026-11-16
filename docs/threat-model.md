# Threat model

Close Call reduces embarrassing and harmful exposure of things you left open. It is not a security
boundary, and it cannot take back frames that were already shared.

## What we protect

- The open sensitive items: tabs, documents, app windows and their titles, URLs and file paths.
- The config file. A list of "sensitive" domains and keywords reveals what the user considers private.
- The run reports and the restore journal, which name what was closed.

## Who we protect against

| In scope | Example |
|---|---|
| The audience of a shared screen | Classmates during a Zoom presentation, a client on a Teams call |
| The room in front of a projector | A lecture hall, a meeting room |
| Someone at an unlocked or just-woken laptop | Opening the lid in class, a colleague at your desk after lunch |
| A later viewer of a recording | A recorded lecture or demo |
| A malicious dependency or a hostile fork | A crate that adds telemetry, a fork that adds tracking |

| Out of scope | Why |
|---|---|
| Malware or an administrator on the same machine | They can read everything we can, and more. |
| A compromised OS or webview runtime | We run on top of them. |
| Forensics on the disk | Closing a tab does not erase history, caches or swap. |
| Network observers | We send nothing; other apps are not our concern. |

## Failure modes and what we do about them

| Failure | Mitigation | What remains |
|---|---|---|
| **The first frames of a share leak.** Detection is reactive; capture can start before cleanup finishes. | Pre-flight sweep when a call is likely; cleanup on lock, sleep and lid close; panic hotkey; share-guard during the share; advice to share a window, not the screen. Latency targets: 300 ms to conceal indexed apps, 700 ms for tabs on macOS in meeting mode. These are targets to measure, not promises. | A fraction of a second of exposure. We will publish measured latency in the [test matrix](test-matrix.md). |
| **Detection misses a share.** | Per-OS confidence levels, an honest capability report, the hotkey. | Undetectable on X11; on Windows DXGI, GDI and capture cards; on macOS any source the private watcher does not see. |
| **The private macOS API changes or disappears.** | `dlsym` with fail-open; projector, lock and hotkey keep working; re-check on every macOS beta. | No automatic share trigger on that macOS version until fixed. |
| **A tab navigates to a sensitive page mid-share.** | Share-guard re-plans changed targets and acts at once. | The page is visible between load and close (about 0.3–1 s, unverified). |
| **A rule is too broad and closes the wrong thing.** | Dry run, "why was this closed?", restore within the restore window, protected meeting apps and sites, re-verification before acting. | Closing a private/incognito tab cannot be undone. |
| **Unsaved work.** | Unknown counts as unsaved; save through the app's API or hide; never drive a Save panel; never force-quit ([ADR-0007](decisions/0007-default-unsaved-policy.md)). | `save_then_close` writes edits you may not have wanted to keep. |
| **A permission prompt or dialog appears on the shared screen.** | All prompts at onboarding or launch, never during a share; countdowns only on idle and item-age rules; notifications without titles. | OS dialogs we do not control. |
| **The app itself leaks.** | No titles, URLs or paths in logs or notifications; restore journal in memory only; our windows hide on share. | The config file on disk names your sensitive sites. |
| **Crash while apps are hidden.** | A marker with bundle ids only lets the next launch unhide them. | |

## Trust boundaries we depend on

- **The OS webview runtime** (WKWebView, WebView2, WebKitGTK) updates itself and is not ours. We load
  only bundled assets under a strict CSP. On Windows, WebView2's own SmartScreen check is a network
  feature of the runtime; keep Tauri's defaults that disable it.
- **Private and undocumented APIs**: SkyLight on macOS, the ConsentStore registry layout on Windows,
  the `com.apple.screenIsLocked` notification names. Each one fails open and is re-checked per OS release.
- **Accessibility and Automation grants.** Anything with those grants can read your windows. We ask
  for them because the job needs them, and we publish the source so you can check what we do with them.
- **The OS itself** for Gatekeeper and notarization checks at launch, which may contact Apple. That is
  the OS, not Close Call.

## Not goals

- Hiding a window from capture while keeping it visible to you. No public API allows it on macOS, and
  on Windows it needs DLL injection, which we refuse.
- Blurring content inside pages.
- Changing OS settings for you. We point to them.
