# macOS

Target for v0.1. macOS 14 and later, universal binary (Intel Macs top out at macOS 26). Not on the
Mac App Store: Accessibility is unsupported in the App Sandbox. Distributed as a notarized Developer
ID build. Evidence and sources: [research/platform-macos.md](../research/platform-macos.md).

## Permissions

| Permission | Why | When we ask |
|---|---|---|
| Accessibility | Read window titles and open documents, press close buttons, read unsaved state | Onboarding. There is no callback, so the app polls once a second until granted. |
| Automation, one per app | Read and close browser tabs (Chrome, Brave, Edge, Vivaldi, Arc, Safari) and close documents through AppleScript (Preview, TextEdit, Pages, Keynote, Numbers, Word, Excel, PowerPoint) | Onboarding for running apps; for others, the next time they launch, but never while a share or projector is active |
| Notifications | Run summaries (counts only) | Onboarding |

Never requested: **Screen Recording** ([ADR-0003](../decisions/0003-never-request-screen-recording.md)),
Input Monitoring (the hotkey uses Carbon `RegisterEventHotKey`), Full Disk Access, Calendars.

Permission-free: NSWorkspace, `CGWindowList` without titles, IOKit power and clamshell, idle time,
the SkyLight watcher, CoreAudio/CoreMediaIO "in use", display configuration, the Firefox session file.

## Capabilities

| Capability | Support | Mechanism and limits |
|---|---|---|
| Apps and processes | Full | `NSWorkspace.runningApplications` and its notifications |
| Windows and titles | Full | AX. Some apps hide windows on other Spaces from AX; `CGWindowList` still shows they exist, without titles. |
| File behind a document window | Partial | AX `AXDocument`, AppleScript for scriptable apps. Missing for untitled documents and most Electron apps. |
| Unsaved state | Partial | AppleScript `modified`/`saved`, then `AXEdited` on the window's close button (only `true` is trusted) |
| Tabs: Chrome, Brave, Edge, Vivaldi | Full | JXA: every tab's URL and title, incognito via window `mode`, close by tab id |
| Tabs: Arc | Full (verify) | JXA by UUID with `byId`; Little Arc windows are not exposed |
| Tabs: Safari | Partial | AppleScript has no tab id (re-resolve by URL right before closing) and no private-window flag |
| Tabs: Firefox | Partial | Detection from `recovery.jsonlz4` (about 15 s stale, no private windows); action is hiding the app |
| Tabs: Opera, Dia | Partial | Hide app only, until their dictionaries are verified |
| Hide | Full | `NSRunningApplication.hide()`: instant and lossless, only when every visible window of the app matches |
| Minimize | Full | `AXMinimized`; last resort (genie animation, live Dock thumbnail) |
| Close a document | Full for scriptable apps, Partial via AX | AppleScript `close … saving yes/no`; otherwise the AX close button plus role-based sheet handling |
| Quit (opt-in) | Full | Documents first, then `terminate()`. Never `forceTerminate()`. State restoration may reopen the document. |

## Triggers

| Trigger | Mechanism | Confidence |
|---|---|---|
| Screen share | Private `SLSIsScreenWatcherPresent()` polled at 4 Hz in meeting mode, 1 Hz otherwise; fails open ([ADR-0010](../decisions/0010-skylight-share-detection.md)) | High where confirmed (Zoom, `screencapture -V`); others unverified |
| Projector, mirroring | `CGDisplayRegisterReconfigurationCallback`, `CGDisplayIsInMirrorSet` | High for mirroring; Medium for "external display connected" |
| Call likely | CoreAudio/CoreMediaIO device running somewhere; per-process mic attribution (macOS 14.x, minimum minor unverified); meeting app launched | Medium / Low |
| Will sleep | `NSWorkspaceWillSleepNotification` + `IORegisterForSystemPower`; sleep waits up to 30 s for our ack | Full |
| Lid | IOKit clamshell interest notification (`0xE0034100`) | Full |
| Lock / unlock | Distributed notifications `com.apple.screenIsLocked` / `…Unlocked` (undocumented, widely used) | Full |
| Idle | `CGEventSourceSecondsSinceLastEventType` | Full |
| Panic | `tauri-plugin-global-shortcut`, tray, `closecall panic` | Full |

## Limits

- The first frames of a share can go out before cleanup finishes. Use the pre-flight sweep, cleanup on
  lock and sleep, and share a window instead of the whole screen. See [threat-model.md](../threat-model.md).
- We cannot keep a window visible to you and hidden from the audience. `NSWindow.sharingType = .none`
  is not honoured by ScreenCaptureKit, so our own windows hide when a share starts.
- Whether AX and Apple Events work while the screen is locked is unverified. If they do not, away
  rules run at lock or will-sleep instead of before unlock.
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
