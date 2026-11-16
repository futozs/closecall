# Privacy

Close Call runs entirely on your computer. It has no servers, no accounts, no telemetry, no crash
reporter and no update check. It never opens a network connection, and you can check that yourself.

## What it reads

| What | Why | How (macOS) |
|---|---|---|
| Running apps and their windows, window titles | To find sensitive items | NSWorkspace, Accessibility |
| Running apps including background agents (bundle ids and names) | To offer and apply the ignore list of always-on screen capturers, such as a DisplayLink dock | NSWorkspace |
| The file behind a document window, and whether it has unsaved changes | Rules by path or file type; not losing work | Accessibility, AppleScript |
| Browser tab titles and URLs, and whether a window is private | Rules by site; private windows count as sensitive | AppleScript per browser; the Firefox session file |
| Screen-share, display, lock, sleep, lid and idle state | Triggers | System notifications, IOKit, CoreGraphics, a private SkyLight call |
| Which app owns each on-screen window, and its layer and position (no titles, no pixels); for a window shaped like a sharing bar but out of its usual place, its title | Recognising the sharing bars of meeting apps and browsers | `CGWindowList`, Accessibility |
| Connected displays: name, EDID vendor, model and serial number, built-in or mirrored | Projector detection and the ignore list for your own monitors | CoreGraphics, NSScreen |
| Whether the default microphone is in use (not by which app) | "A call is likely" pre-flight | CoreAudio |

It does not read page contents, messages, the contents of your files, your screen pixels, keystrokes
or browsing history. (The planned v0.6 image classifier, off by default, would read image files that
are open in a viewer, on your machine only.) It never asks
for Screen Recording, Input Monitoring or Full Disk Access.

## What it stores

| Data | Where | Contains titles, URLs or paths? | Kept |
|---|---|---|---|
| `config.toml` | App config directory, see [docs/config.md](docs/config.md) | Your rules; may name sites or folders you consider sensitive | Until you delete it |
| `config.rejected.toml` | Next to `config.toml` | Your earlier rules | Written once, when you save over a `config.toml` that could not be used; until you delete it |
| Restore list of closed items | **Memory only** | Yes | Until you restore, quit, or the restore window ends (2 hours by default) |
| Activity history (the last 50 runs) | **Memory only** | Yes, until the run's restore window ends; then only the kind of item, the app and what happened | Until quit |
| When each item was first seen | Memory only | No | Until quit; item age restarts when the app restarts |
| Logs | Standard error only (no log file); a launch agent started at login sends it nowhere | **No**: rule ids, counts and outcomes | Not kept |
| Login item, when "Open at login" is on | `~/Library/LaunchAgents/`, a plist with the app's path | No | Until you turn it off |
| Single-instance socket | `/tmp/hu_futozs_closecall_si.sock` while the app runs | No; a second launch sends its command-line arguments through it | Removed on quit |
| Untitled documents saved under the default unsaved policy | `~/Documents/Close Call` (configurable) | They are your documents | Until you delete them |

Not implemented yet: importing your own list files, a marker that lets the next launch unhide apps
after a crash (until then, unhide them from the Dock or the app switcher), and a diagnostic log mode.
The single-instance socket sits in the shared `/tmp`, a limit of the Tauri plugin; on a Mac with
other local users, one of them could create that socket first and receive the arguments of your
launch.

Titles, URLs and paths appear only inside the Close Call window. Notifications show counts ("2 tabs
closed, 1 document hidden"), never what was closed, because they may show up on a shared screen.

## What leaves your computer

Nothing. Close Call never opens an internet socket, not even to localhost. Lists of sensitive sites
ship inside the app and change only with a new release.

Things outside our control:

- The system webview (WKWebView, WebView2, WebKitGTK) is part of your OS and updates itself. Close
  Call loads only files bundled with the app.
- macOS may check the app's signature with Apple at launch (Gatekeeper). That is the OS.
- Links you click in the app (the source code, the releases page, the source of a bundled list) open
  in your own browser.
- From v0.4, the optional browser extension lives in a browser that uses the network. The extension
  itself makes no requests and talks to the app only over a local pipe.

## Check it yourself

- **While it runs** (macOS): LuLu or Little Snitch will show no connection attempts. Or find the
  process id in Activity Monitor and run `lsof -nP -i -a -p <pid>`; it prints nothing.
- **Linux**: `strace -f -e trace=socket,connect <binary>` shows no `AF_INET` or `AF_INET6`.
- **In the source**: how the rule is enforced in CI is described in the README section
  [Check the privacy promise](README.md#check-the-privacy-promise) and [ADR-0002](docs/decisions/0002-no-network-ever.md).

Questions or a privacy problem: see [SECURITY.md](SECURITY.md) for private reporting.
