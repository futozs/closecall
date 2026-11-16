# Privacy

Close Call runs entirely on your computer. It has no servers, no accounts, no telemetry, no crash
reporter and no update check. It never opens a network connection, and you can check that yourself.

## What it reads

| What | Why | How (macOS) |
|---|---|---|
| Running apps and their windows, window titles | To find sensitive items | NSWorkspace, Accessibility |
| The file behind a document window, and whether it has unsaved changes | Rules by path or file type; not losing work | Accessibility, AppleScript |
| Browser tab titles and URLs, and whether a window is private | Rules by site; private windows count as sensitive | AppleScript per browser; the Firefox session file |
| Screen-share, display, lock, sleep, lid and idle state | Triggers | System notifications, IOKit, CoreGraphics, a private SkyLight call |
| Whether the microphone or camera is in use, and by which app | "A call is likely" pre-flight | CoreAudio, CoreMediaIO |

It does not read page contents, messages, the contents of your files, your screen pixels, keystrokes
or browsing history. (The planned v0.6 image classifier, off by default, would read image files that
are open in a viewer, on your machine only.) It never asks
for Screen Recording, Input Monitoring or Full Disk Access.

## What it stores

| Data | Where | Contains titles, URLs or paths? | Kept |
|---|---|---|---|
| `config.toml` | App config directory, see [docs/config.md](docs/config.md) | Your rules; may name sites or folders you consider sensitive | Until you delete it |
| Imported list files | App data directory | Domains | Until you remove them |
| Restore list of closed items | **Memory only** | Yes | Until you restore, quit, or the restore window ends (2 hours by default) |
| Hidden-apps marker | App data directory | No: bundle ids only | Until restored; lets the next launch unhide apps after a crash |
| Logs | App log directory | **No**, unless you turn on diagnostic mode, which warns you and switches itself off after 24 hours | 7 days |
| When each item was first seen | Memory only | No | Until quit; item age restarts when the app restarts |
| Untitled documents saved under the default unsaved policy | `~/Documents/Close Call` (configurable) | They are your documents | Until you delete them |

Titles, URLs and paths appear only inside the Close Call window. Notifications show counts ("2 tabs
closed, 1 document hidden"), never what was closed, because they may show up on a shared screen.

## What leaves your computer

Nothing. Close Call never opens an internet socket, not even to localhost. Lists of sensitive sites
ship inside the app and change only with a new release or a file you import.

Things outside our control:

- The system webview (WKWebView, WebView2, WebKitGTK) is part of your OS and updates itself. Close
  Call loads only files bundled with the app.
- macOS may check the app's signature with Apple at launch (Gatekeeper). That is the OS.
- Links you click in the app (the source code, the releases page) open in your own browser.
- From v0.4, the optional browser extension lives in a browser that uses the network. The extension
  itself makes no requests and talks to the app only over a local pipe.

## Check it yourself

- **While it runs** (macOS): LuLu or Little Snitch will show no connection attempts. Or find the
  process id in Activity Monitor and run `lsof -nP -i -a -p <pid>`; it prints nothing.
- **Linux**: `strace -f -e trace=socket,connect <binary>` shows no `AF_INET` or `AF_INET6`.
- **In the source**: how the rule is enforced in CI is described in the README section
  [Check the privacy promise](README.md#check-the-privacy-promise) and [ADR-0002](docs/decisions/0002-no-network-ever.md).

Questions or a privacy problem: see [SECURITY.md](SECURITY.md) for private reporting.
