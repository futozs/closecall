# macOS

Target for v0.1. macOS 14 and later, universal binary (Intel Macs top out at macOS 26). Not on the
Mac App Store: Accessibility is unsupported in the App Sandbox. Distributed as a notarized Developer
ID build. Evidence and sources: [research/platform-macos.md](../research/platform-macos.md).

## Permissions

| Permission | Why | When we ask |
|---|---|---|
| Accessibility | Read window titles and open documents, press close buttons, read unsaved state | Onboarding. There is no callback, so the app polls once a second until granted. |
| Automation, one per app | Read and close browser tabs (Chrome, Brave, Edge, Vivaldi, Arc, Helium, Safari) and close documents through AppleScript (Preview, TextEdit, Pages, Keynote, Numbers, Word, Excel, PowerPoint) | Onboarding for running apps; for others, the next time they launch, but never while a share or projector is active (the request returns `not_while_sharing`) |
| Notifications | Run summaries (counts only) | Onboarding |

Never requested: **Screen Recording** ([ADR-0003](../decisions/0003-never-request-screen-recording.md)),
Input Monitoring (the hotkey uses Carbon `RegisterEventHotKey`), Full Disk Access, Calendars.

Permission-free: NSWorkspace, `CGWindowList` owners, layers and bounds (no titles), the IOKit
clamshell state, idle time, the SkyLight watcher, CoreAudio "in use", display configuration and
EDID ids, the Firefox session file.

## Capabilities

| Capability | Support | Mechanism and limits |
|---|---|---|
| Apps and processes | Full | `NSWorkspace.runningApplications` and its notifications |
| Windows and titles | Full | AX. Some apps hide windows on other Spaces from AX; `CGWindowList` still shows they exist, without titles. |
| File behind a document window | Partial | AX `AXDocument`, AppleScript for scriptable apps. Missing for untitled documents and most Electron apps. |
| Unsaved state | Partial | AppleScript `modified`/`saved`, then `AXEdited` on the window's close button (only `true` is trusted) |
| Tabs: Chrome, Brave, Edge, Vivaldi, Helium | Full | AppleScript (it has `with timeout`, JXA does not): every tab's URL and title, incognito via window `mode`, close by tab id. The close script re-checks each tab's title and URL, the step's own tab included, and closes only unchanged ones. |
| Tabs: Arc | Partial (unverified) | Same script, ids are text; Little Arc windows are not exposed |
| Tabs: Safari | Partial | AppleScript has no tab id: keyed by window, URL and occurrence, found again by URL right before closing; no private-window flag |
| Tabs: Firefox | Partial | Detection from `recovery.jsonlz4` (about 15 s stale, no private windows); Close hides Firefox |
| Tabs: Opera, Dia and other browsers | None | No tab provider: their windows are app windows to the AX provider, so a Close rule on app windows would close a whole browser window. Don't target them with Close. |
| Hide | Full | `NSRunningApplication.hide()`: instant and lossless. For windows only when every visible window of the app is planned with the same action; for tabs it hides the whole browser. Never while a protected target (a meeting tab, the capturing app) shares the app instance: the step is skipped with `reason.protected_in_app`. |
| Minimize | Full | `AXMinimized`; last resort (genie animation, live Dock thumbnail). A failed Minimize is reported, never turned into a Hide. |
| Close a document | Full for scriptable apps, Partial via AX | AppleScript `close … saving yes/no`; otherwise the AX close button. A sheet is answered only when it has the shape of a save alert (three buttons with a default and a cancel, no text field or pop-up); anything else, and a sheet that was open before, is cancelled or left alone and the window concealed. |
| Quit (opt-in) | Full | Documents first, then `terminate()`. Never `forceTerminate()`. State restoration may reopen the document. |

Without an Automation grant a browser's tabs are not listed at all, and the AX provider skips
browsers that have a tab provider, so nothing of that browser is concealed while the capability table
still shows the tab rows. Granting Automation in onboarding is what makes browser rules work.

## Triggers

| Trigger | Mechanism | Confidence |
|---|---|---|
| Screen share | Private `SLSIsScreenWatcherPresent()` polled at 4 Hz while a meeting app runs, 1 Hz otherwise; fails open ([ADR-0010](../decisions/0010-skylight-share-detection.md)). It is one bit, not a count, and names no app. | High where confirmed (Zoom, `screencapture -V`); others unverified. Low when it was already on at launch with no meeting app running, until it reads false once. |
| Sharing bars (`macos.share_ui`) | `CGWindowList` owners, layers and bounds: Zoom's `CptHost` process, bar-shaped floating windows of Chromium browsers and meeting apps, meeting-app frames around a whole display, and an AX title match only for a bar out of its usual place. A cue must hold for 2 polls (1 s apart). | Medium, with the capturing app named. Thresholds unverified on a device. |
| Projector, mirroring | `CGDisplayRegisterReconfigurationCallback`, then a per-display diff: `DisplayConnected`/`Disconnected`/`Mirrored`/`Unmirrored` with name (from `NSScreen`) and EDID vendor, model and serial | High. Ignored displays and the built-in one never fire. |
| Call likely | CoreAudio: the default input device is running somewhere. No per-process attribution, no camera, no "meeting app launched". | Low |
| Will sleep | `NSWorkspaceWillSleepNotification`. Sleep is not delayed: the run races the system going to sleep. | Partial |
| Lid | `AppleClamshellState` from IOKit, polled once a second | Full (up to 1 s late) |
| Lock / unlock | Distributed notifications `com.apple.screenIsLocked` / `…Unlocked` (undocumented, widely used) | Full |
| Idle | `CGEventSourceSecondsSinceLastEventType` | Full |
| Panic | `tauri-plugin-global-shortcut`, tray, `closecall panic` | Full |

## Inventory

- Browser tabs come from a poller every 2 s, woken at once by AX title-change observers on browser
  windows (their own thread and `CFRunLoop`, see `observe.rs`) and by app launches. Windows and
  documents come from a 2 s AX poll. After the first full scan, the engine's 30 s refresh reads the
  poller's cache instead of scanning on the provider thread, so a share's actions never queue
  behind a scan.
- Every AX element gets a 0.5 s messaging timeout, not only the application element.
- Re-verify before acting: a tab is read again by id (Chromium) or by URL (Safari) right before the
  step. Without an Automation grant no Apple Event is sent (it could prompt on a shared screen); the
  close script then checks the tab again itself.

## Restore

Restore never launches an app, except Launch Services opening a closed document in its app.

| Action undone | How |
|---|---|
| Hide | `unhide()` of the same app instance (same pid); a relaunched app is left alone |
| Minimize | `AXMinimized = false` on the same window |
| Close tab, Chromium | Reopens the URL in the tab's own window if it is a normal one, else any normal window, else a new one. Never into an incognito window. Private tabs are never reopened. |
| Close tab, Safari | Never: Safari can't tell private windows apart. History → Reopen Last Closed Tab can. |
| Close document | Launch Services opens the file in the app that had it (10 s deadline); untitled documents and moved files can't come back |
| Quit | Never relaunched |

## DisplayLink and other always-on capture

A DisplayLink dock (DisplayLink Manager, bundle id `com.displaylink.DisplayLinkUserAgent`, an
`LSUIElement` agent) drives its monitors by capturing the screen all day, so the SkyLight watcher
reads true for as long as it runs. Without help, every launch and every dock replug would look like a
High-confidence share and share-guard would stay on all day.

- `[ignore] capture_apps` in [config.md](../config.md) lists such apps (the recommended preset has
  DisplayLink). While a listed app runs, any capture the OS can't attribute counts as that app's and
  starts nothing. The status shows a note and the watcher's health reads "unreliable" with the reason.
- Real shares are then recognised by `macos.share_ui` from the meeting apps' and browsers' sharing
  bars. Discord has no known bar cue, so a Discord share is missed while an ignored capturer runs.
- Known gap: the ignore holds while the capturer process runs, even when it is not capturing (no dock
  attached). A precise fix needs SkyLight's per-process capture notification, whose event number is
  not verified.
- `[ignore] displays` lists the user's own monitors by name or EDID vendor and model; they never
  count as a projector or an external display. A display that was mirrored from the moment it
  connected has no `NSScreen`, so no name: ignore it by vendor and model.
- To tune the sharing-bar cues on a device: `cargo run -p closecall-platform-macos --example
  share_ui 30` during a Zoom, Meet in Chrome, Teams and Slack share. It prints owners, layers and
  bounds only.

## Limits

- The first frames of a share can go out before cleanup finishes. Use the pre-flight sweep, cleanup on
  lock and sleep, and share a window instead of the whole screen. See [threat-model.md](../threat-model.md).
- We cannot keep a window visible to you and hidden from the audience. `NSWindow.sharingType = .none`
  is not honoured by ScreenCaptureKit, so our own windows hide when a share starts.
- Whether AX and Apple Events work while the screen is locked is unverified. If they do not, away
  rules run at lock or will-sleep instead of before unlock.
- Will-sleep and lid runs are not waited for: the Mac may sleep before AppleScript calls finish.
  `IORegisterForSystemPower` with a delayed `IOAllowPowerChange` would fix that; not implemented.
- If the Mac does not require a password right after sleep, wake shows the desktop at once. Onboarding
  points to the setting.

## Development

- TCC grants are keyed to bundle id `hu.futozs.closecall` and the code signature. Ad-hoc signatures
  change on every build and silently break grants. Sign dev builds with a stable identity.
- `bun run dev` runs an unbundled binary; TCC attributes it to your terminal or IDE. Test permissions
  with a bundled `.app`.
- Reset: `tccutil reset Accessibility hu.futozs.closecall`, `tccutil reset AppleEvents hu.futozs.closecall`.
- Deep links: `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` and `…?Privacy_Automation`.
- Release builds need the hardened runtime, the entitlement `com.apple.security.automation.apple-events`
  and `NSAppleEventsUsageDescription` (localized in `en.lproj` and `hu.lproj`).
