# "Did I close it?" / „Bezártam?”: macOS technical feasibility study (macOS 14 / 15 / 26, Apple Silicon)

**Date:** 2026-11-16. **Verified locally on:** macOS **27.2** (build 26B123), Mac16,12 (Apple Silicon), Xcode SDK `MacOSX27.2.sdk`, Swift 6.4.2. macOS 27 is the current release, so wherever I could, I checked behaviour on 27.2 directly and say so. Claims about 14, 15 and 26 come from the cited sources. Web search hit its per-turn budget near the end, so a few items are marked **unverified** below.

---

## 0. Verdict at a glance

| Capability | Feasible? | Main mechanism | Permission | Confidence |
|---|---|---|---|---|
| List running apps | Yes | `NSWorkspace.runningApplications` | none | High |
| Window titles of all apps | Yes | AX (`kAXWindowsAttribute` + `kAXTitleAttribute`) | Accessibility | High |
| Window titles via CGWindowList | Only with Screen Recording | `CGWindowListCopyWindowInfo` | Screen Recording | High (verified 27.2) |
| Open document file per window | Mostly | AX `kAXDocumentAttribute` + per-app AppleScript | Accessibility / Automation | Medium-High |
| All tabs (URL+title) in Chrome/Brave/Edge/Vivaldi/Arc/Safari | Yes | AppleScript/JXA | Automation (per browser) | High |
| Incognito tabs (Chromium) | Yes, flagged by `mode` | AppleScript | Automation | High |
| Safari private windows | Readable (probably), **no flag** to tell them apart | AppleScript | Automation | Medium (unverified) |
| Firefox all tabs | Not via AppleScript | WebExtension + native messaging (recommended); sessionstore file; AX | none extra / Accessibility | High for the extension route |
| Close a single tab | Yes (Chromium, Safari, Arc); Firefox via extension | AppleScript `close` / `tabs.remove` | Automation | High |
| Close a single document window, handle the save sheet | Yes | AppleScript `close … saving no`, or AX close button + sheet buttons | Automation / Accessibility | Medium-High |
| Detect unsaved state | Partly | `AXEdited` on the window's **close button**; AppleScript `modified`/`saved` | Accessibility / Automation | Medium |
| Sleep/wake, lid, lock, idle | Yes | NSWorkspace, IOKit, distributed notifications, CGEventSource | none | High |
| **Detect another app's screen sharing/recording** | **Yes, only via a private API** | `SLSIsScreenWatcherPresent()` (SkyLight) | none | Medium-High (private, used empirically) |
| Global panic hotkey | Yes | Carbon `RegisterEventHotKey` (via `global-hotkey`) | none | High |
| Mac App Store | **No** | Accessibility is unsupported in the sandbox | – | High |
| Developer ID + notarization | Yes | Hardened runtime + `com.apple.security.automation.apple-events` | – | High |

---

## 1. Enumerating apps, windows, titles and documents

### 1.1 Running apps (no permission)
- `NSWorkspace.shared.runningApplications` → `NSRunningApplication` (`bundleIdentifier`, `processIdentifier`, `activationPolicy == .regular`, `isHidden`, `launchDate`).
- Live updates come from `NSWorkspace.didLaunchApplicationNotification`, `didTerminateApplicationNotification`, `didActivateApplicationNotification`, `didHideApplicationNotification` and `didUnhideApplicationNotification`. These are posted on **`NSWorkspace.shared.notificationCenter`**, not the default center.

### 1.2 `CGWindowListCopyWindowInfo`: which fields need Screen Recording
I tested this on macOS 27.2 from a process **without** Screen Recording (`CGPreflightScreenCaptureAccess() == false`), looking at layer-0 windows:

| Key | Present without Screen Recording? |
|---|---|
| `kCGWindowNumber`, `kCGWindowOwnerPID`, `kCGWindowOwnerName`, `kCGWindowLayer`, `kCGWindowBounds`, `kCGWindowAlpha`, `kCGWindowIsOnscreen`, `kCGWindowMemoryUsage`, `kCGWindowStoreType` | **Yes** |
| `kCGWindowName` (title) | **No.** The key is absent. |
| `kCGWindowSharingState` | Present, but every window reported `0` (None), so the value is effectively redacted and tells you nothing. |

This matches the forum history: titles have been gated since Catalina ([Apple forum 126860](https://developer.apple.com/forums/thread/126860), [Mozilla bug 1580900](https://bugzilla.mozilla.org/show_bug.cgi?id=1580900)). Apple does not document which keys are gated ([forum 706187](https://developer.apple.com/forums/thread/706187)). `CGWindowListCreateImage` is marked `SCREEN_CAPTURE_OBSOLETE(10.5,14.0,15.0)` in the 27.2 SDK.

**Recommendation:** use CGWindowList only to find which PIDs own windows (it covers all Spaces with `.optionAll`). Get titles from AX. **Don't request Screen Recording at all.** On Sequoia it brings recurring re-consent prompts (weekly in beta, monthly at release; [TidBITS](https://tidbits.com/2024/08/19/apple-reduces-excessive-sequoia-permission-requests-shifts-to-monthly)). It also puts the purple capture indicator on your own app, and capturing the screen yourself would trip the screen-watcher signal from §5.2.

### 1.3 Accessibility (AX): the main source of truth
Header names below come from the 27.2 SDK (`HIServices/AXAttributeConstants.h`, `AXUIElement.h`).

```
app   = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5…1.0)       // default is several seconds; a hung app blocks you
windows = app[kAXWindowsAttribute]                  // "AXWindows"
for w in windows:
  w[kAXTitleAttribute]        "AXTitle"
  w[kAXDocumentAttribute]     "AXDocument"  -> file:// URL *string* (percent-encoded)
  w[kAXRoleAttribute]/[kAXSubroleAttribute]   AXWindow / AXStandardWindow vs AXDialog, AXFloatingWindow
  w[kAXMinimizedAttribute]    "AXMinimized" (settable)
  w[kAXMainAttribute], w[kAXFocusedAttribute]
  w[kAXCloseButtonAttribute]  "AXCloseButton" -> AXButton (press with kAXPressAction)
  closeButton[kAXEditedAttribute] "AXEdited"   -> dirty bit (see §3.2)
app[kAXHiddenAttribute] "AXHidden" (settable -> hide/unhide app)
```

- **`AXDocument`** exists only if the app ties a file to the window: the represented URL / NSDocument in Cocoa, or the proxy icon in Carbon ([accessibility-dev 2006](https://lists.apple.com/archives/accessibility-dev/2006/Apr/msg00012.html)). It is **missing** for untitled documents, for most Electron and cross-platform apps, and for non-document windows. Fall back to matching filenames in `AXTitle`.
- **Events instead of polling:** create `AXObserverCreate(pid, cb)` per app and register `kAXWindowCreatedNotification`, `kAXTitleChangedNotification`, `kAXUIElementDestroyedNotification`, `kAXFocusedWindowChangedNotification` and `kAXSheetCreatedNotification`. Add `AXObserverGetRunLoopSource` to a dedicated worker thread's CFRunLoop.
- **Spaces:** for many apps, `kAXWindowsAttribute` returns only windows on the **current Space**, plus minimized ones. This has been the case since 10.5 ([accessibility-dev 2009](https://lists.apple.com/archives/accessibility-dev/2009/May/msg00001.html)), and AltTab reports it on 12.2+ ([alt-tab #1324](https://github.com/lwouis/alt-tab-macos/issues/1324)). On macOS 14+, SwiftUI windows on another Space have their AX detail stripped; AppKit windows are not affected ([Cua driver limits](https://cua.ai/docs/cua-driver/reference/limits)). Use CGWindowList (all Spaces, owner PID) to notice that an app has windows you can't see through AX.
- **CGWindowID mapping:** the private `_AXUIElementGetWindow(AXUIElementRef, CGWindowID*)` is what yabai, Rectangle and AltTab use to join AX windows to CGWindowList entries.
- **Chromium and Electron apps** build their AX tree lazily. Setting `AXManualAccessibility = true` (Electron) or `AXEnhancedUserInterface = true` (Chrome) on the app element exposes it. `AXEnhancedUserInterface` is known to interfere with window animations, so avoid it unless you need web content.
- **Hard rules:** never make AX calls on the UI thread, and always set a messaging timeout.

---

## 2. Browser tabs

### 2.1 Matrix

| Browser (bundle id) | Read all tabs (URL + title) | Private/incognito | Close one tab | Notes |
|---|---|---|---|---|
| Chrome `com.google.Chrome` | AppleScript/JXA, Chromium suite | Visible; window `mode` = `"incognito"`/`"normal"` | `close tab id T of window id W` | IDs are stable integers. `"Allow JavaScript from Apple Events"` is **not** needed for URL, title or close. |
| Brave `com.brave.Browser` | Same suite. Verified with `sdef` on 27.2: window `id, mode, active tab index, minimized, visible`; tab `id, title, URL, loading`; commands `close, reload, execute` | Same `mode` | Same | |
| Edge `com.microsoft.edgemac` | Chromium suite | `mode` | Same | Not tested locally. |
| Vivaldi `com.vivaldi.Vivaldi` | Chromium suite ([Vivaldi forum](https://forum.vivaldi.net/topic/73155/scriptable-vivaldi-from-command-line)) | `mode` | Same | Test on install. |
| Opera `com.operasoftware.Opera` | **Uncertain.** Older sources say AppleScript support was removed. | ? | ? | Plan an extension or AX fallback. |
| Arc `company.thebrowser.Browser` | JXA works. Tab and window IDs are UUID **strings**; object references from `windows()`/`tabs()` don't resolve (-1700), so address tabs `byId` ([mcp-chrome-tabs arc.ts](https://github.com/pokutuna/mcp-chrome-tabs)) | Little Arc windows are **not exposed** | By id (verify) | Arc is in maintenance mode. |
| Dia `company.thebrowser.dia` | Since 1.7.0 (2025-12-01): "query windows and tabs or focus them" ([changelog](https://www.diabrowser.com/changelog/1-7-0)) | ? | Not stated | Check URL/close support in its dictionary. |
| Safari `com.apple.Safari` | AppleScript. The 27.2 dictionary shows tab properties `name, URL, index, visible, source, text, pid` and window `current tab`. **No tab id.** | **No private-window property** in the dictionary ([alexwlchan](https://alexwlchan.net/2020/using-applescript-to-open-a-url-in-private-browsing-in-safari/)). Private windows are believed to be enumerable but can't be told apart. **Unverified.** | `close tab N of window id W`. Re-resolve by URL immediately before closing, because indexes shift. | |
| Firefox `org.mozilla.firefox` | **No tab dictionary.** Only window `name` (= the active tab's title) ([bug 125419](https://bugzilla.mozilla.org/show_bug.cgi?id=125419)) | – | – | See 2.3. |

The Chromium object model is described in [chromium.org: AppleScript](https://www.chromium.org/developers/design-documents/applescript/): windows come from the browser list, which includes incognito windows and all profiles. Closing incognito windows by `mode` is confirmed in practice ([KM forum](https://forum.keyboardmaestro.com/t/find-and-close-incognito-chrome-windows/20467)).

### 2.2 Code (JXA, one Apple Event per property per window)
```js
// osakit / OSAScript in-process, language "JavaScript"
const c = Application('com.google.Chrome');
if (!c.running()) JSON.stringify([]);   // never send events to a non-running app: it launches it
else JSON.stringify(c.windows().map(w => {
  const ids = w.tabs.id(), urls = w.tabs.url(), titles = w.tabs.title();
  return { win: w.id(), mode: w.mode(), active: w.activeTabIndex(),
           tabs: ids.map((id, i) => ({ id, url: urls[i], title: titles[i] })) };
}));
// close one tab:
c.windows.byId(winId).tabs.byId(tabId).close();
```
```applescript
tell application id "com.apple.Safari"
  close (every tab of window id 1234 whose URL starts with "https://netbank.example")
end tell
```
Practical points:
- Bulk-read `w.tabs.url()` instead of one event per tab.
- Re-check IDs to catch tabs that change during the read (mcp-chrome-tabs does this).
- Wrap each call in a short deadline. A hung browser blocks for the Apple Event timeout, which defaults to about 2 minutes.
- `Application('Google Chrome')` with several same-bundle processes (for example a headless Chrome) may resolve to the wrong instance.
- **Lossiness:** tabs closed in normal windows can be restored with Reopen Closed Tab or History. Closing incognito or private tabs is **irreversible**.
- **"Open longer than N hours":** AppleScript gives no open time, so track first-seen timestamps yourself (in memory). The extension route gives `tab.lastAccessed`.

### 2.3 Firefox workarounds (in order of preference)
1. **Companion WebExtension + native messaging (recommended; also worth doing for Chromium).**
   - The extension uses `tabs.query({})`, `tabs.remove(id)`, `windows.getAll()` and `tabs.onUpdated`/`onCreated`. That makes it **push-based**: no polling and no per-browser Automation prompt.
   - The native host manifest goes in `~/Library/Application Support/Mozilla/NativeMessagingHosts/<name>.json`; the filename must equal `name` ([MDN native manifests](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_manifests)). Chrome's is `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/`; Brave and Edge have their own directories.
   - Private windows are visible only if the user enables "Run in Private Windows" (Firefox) or "Allow in Incognito" (Chromium). Both are off by default.
   - The Firefox extension must be AMO-signed (an unlisted listing is fine).
   - The host binary, which the browser spawns, should talk to the main app over a **Unix domain socket or XPC, not TCP**, so the no-network rule holds and can be audited.
   - Safari would need a Safari Web Extension inside an Xcode-built container app.
2. **Session file (read-only, delayed):** `~/Library/Application Support/Firefox/Profiles/<p>/sessionstore-backups/recovery.jsonlz4`.
   - Format: mozLz4 = 8-byte magic `mozLz40\0` + u32 LE uncompressed size + raw LZ4 block ([Foxton](https://www.foxtonforensics.com/blog/post/analysing-firefox-session-restore-data-mozlz4-jsonlz4)).
   - Firefox rewrites it about every 15 s (`browser.sessionstore.interval`). Private windows are not persisted.
   - No Full Disk Access is needed: this folder is not TCC-protected.
   - Good for detection only.
3. **AX tree:** tab-strip `AXRadioButton`s give every tab's title. The address-bar `AXTextField` or web-area `AXURL` gives the active tab's URL only. Private windows can typically be spotted by the "Private Browsing" title suffix (unverified). Closing a background tab means selecting it and posting ⌘W, which the user sees, so use this only as a fallback.

Firefox remote debugging and Marionette work over localhost TCP, which breaks the no-network rule. Exclude them.

---

## 3. Documents

### 3.1 Per-app matrix

| App | Which file is open | Close one doc without UI | Dirty flag | Status |
|---|---|---|---|---|
| Preview `com.apple.Preview` | AX `AXDocument`; AppleScript `path of document` | `close (every document whose path is P) saving no` | AppleScript `modified` | Dictionary **verified via `sdef` on 27.2** (document `name, path, modified`; window `document`; `close … saving ask/no/yes`) |
| TextEdit | Same standard suite | Same | `modified` | Same suite |
| Word `com.microsoft.Word` | AppleScript document `full name`, `path` (verified in sdef); AX likely | `close document "x.docx" saving no` (verify syntax) | `saved` (false = dirty) | Properties verified on 27.2 |
| PowerPoint | `presentation` `full name/path/saved` (verified) | `close presentation … saving no` (verify) | `saved` | Properties verified |
| Excel | `workbook` `full name/path/saved` (verified) | `close workbook … saving no` (verify) | `saved` | Properties verified |
| Pages / Keynote / Numbers | AppleScript document `file`, `modified` | `close document 1 saving no` | `modified` | Not installed here; **unverified** |
| Acrobat / Reader | Limited AppleScript; AX title / `AXDocument` | AX close button | AX | **Unverified** |
| Other image viewers | `AXDocument`, else title match | AX close | AX | – |

### 3.2 Unsaved state
- Window-level `AXEdited` returns `kAXErrorAttributeUnsupported` in AppKit. **Read `AXEdited` from the window's `AXCloseButton` instead**; it mirrors `NSWindow.isDocumentEdited` live ([trycua/cua PR #3795](https://github.com/trycua/cua/pull/3795), macOS 26.1).
- Treat only `true` as evidence. Autosave-in-place apps can hold unsaved text while the flag is clear.
- `NSAccessibilityEditedAttribute` ("is it dirty?") is declared in the 27.2 AppKit headers.
- The AppleScript `modified`/`saved` properties are the second signal.

### 3.3 Handling the save sheet with AX (localization-independent)
1. Press `kAXCloseButtonAttribute` with `kAXPressAction` (or AppleScript `close`).
2. Wait for `kAXSheetCreatedNotification` on the window, or poll its `AXChildren` for role `AXSheet` for 1–2 s.
3. In the sheet, `kAXDefaultButtonAttribute` is **Save** and `kAXCancelButtonAttribute` is **Cancel**. The one remaining `AXButton` is **Don't Save** (or **Delete** for an untitled autosaved document: "Do you want to keep this new document?"). Press it.
4. Don't match button titles: they're localized, and the Hungarian UI is a first-class target. Labels also vary by app ("Don't Save", "Delete", "Revert Changes"; [KM forum](https://forum.keyboardmaestro.com/t/close-and-dont-save-file/3491)). ⌘⌫ is the Don't Save shortcut since 10.9, but it needs focus.
5. A "Save" rule should apply only to documents that have a file (`AXDocument` present). Saving an untitled document opens a Save panel, which you can't drive safely.

Apply rules in this order: **unsaved → hide (default) | save-and-close (if it has a file) | discard (explicit opt-in) | skip and notify.** Use AppleScript `saving no/yes` where a dictionary exists; it avoids UI entirely.

---

## 4. Hide vs close vs minimize vs quit

| Action | API | Speed | Lossless? | Granularity | Safe during a full-screen share? | Caveats |
|---|---|---|---|---|---|---|
| **Hide app** | `NSRunningApplication.hide()` (no TCC expected; verify) or AX `AXHidden = true` | Instant, no animation | **Yes** | Whole app | Yes. Also removes it from Stage Manager thumbnails. | Hiding Chrome also hides a Meet tab running in Chrome. |
| Switch active tab | Chromium `set active tab index` | Instant | Yes | Per window | Partly: the title stays visible in the tab strip | – |
| Minimize window | AX `AXMinimized = true` | ~0.3 s genie animation | Yes | Per window | **Risky:** the Dock shows a live thumbnail | Avoid. |
| Close tab | AppleScript / extension | ~10–100 ms | Normal windows: restorable via history. **Incognito: no.** | Per tab | Yes | Form state is lost. |
| Close document window | AppleScript `saving` / AX | Fast | Only if saved | Per window | Yes | Save sheet (§3.3). |
| Quit app | `NSRunningApplication.terminate()` | Slow (save prompts) | Only if saved | Whole app | Yes | **State restoration reopens the sensitive document on next launch** unless "Close windows when quitting an application" is on. `forceTerminate()` loses data. Sandboxed callers get `false` ([Apple](https://developer.apple.com/documentation/appkit/nsrunningapplication/terminate())). |
| Cover with an overlay | Own borderless window | Instant | Yes | Per window | Only for whole-display shares; **window-level shares capture the window underneath** | This is how nyx works. |

Default policy: **hide** non-browser apps, **close** browser tabs (or hide the browser if it isn't hosting the meeting), close documents with the save rules from §3.3, and **never quit by default**.

---

## 5. Triggers

### 5.1 Sleep/wake, lid, lock, idle (all permission-free)

| Event | API | Notes / verification |
|---|---|---|
| Will sleep / did wake | `NSWorkspaceWillSleepNotification`, `NSWorkspaceDidWakeNotification` (workspace center), or `IORegisterForSystemPower` → `kIOMessageSystemWillSleep`, `kIOMessageSystemHasPoweredOn` | If you don't ack with `IOAllowPowerChange`, sleep is delayed by **up to 30 s**. Forced sleep (lid close) can only be **delayed, not prevented** ([QA1340](https://developer.apple.com/library/archive/qa/qa1340/_index.html)). |
| Display sleep/wake | `NSWorkspaceScreensDidSleepNotification`, `…DidWake` | |
| Lid | IORegistry `IOPMrootDomain` property `"AppleClamshellState"` (true = closed) and `"AppleClamshellCausesSleep"`. Events via `IOServiceAddInterestNotification(port, rootDomain, kIOGeneralInterest, cb)`, message `kIOPMMessageClamshellStateChange` = `iokit_family_msg(sub_iokit_powermanagement, 0x100)` = **0xE0034100**, arg bit0 = closed, bit1 = causes sleep | SDK header verified. `ioreg` on this Mac shows both keys (`CausesSleep = No` while in clamshell-capable mode). |
| Lock / unlock | `NSDistributedNotificationCenter` `"com.apple.screenIsLocked"` / `"com.apple.screenIsUnlocked"` (undocumented). Also `com.apple.screensaver.didstart`/`didstop`, `NSWorkspaceSessionDidResignActive`/`BecomeActive` (fast user switching). | Hammerspoon's `hs.caffeinate.watcher` still uses these names ([source](https://github.com/Hammerspoon/hammerspoon/blob/master/extensions/caffeinate/libcaffeinate_watcher.m)). For state polling, `CGSessionCopyCurrentDictionary()["CGSSessionScreenIsLocked"]`: on 27.2 the key is **absent while unlocked** (verified). |
| Idle time | `CGEventSourceSecondsSinceLastEventType(kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType)` (Swift: `CGEventSource.secondsSinceLastEventType`), or IOKit `IOHIDSystem` `HIDIdleTime` | **Verified on 27.2 without any permission.** |

**Design consequence:** act **before** exposure.
- Clean up on lid-close, will-sleep or lock while you still have up to 30 s.
- For "locked or asleep longer than N minutes", record the timestamp. On `DidWake`, if the screen is still locked (password required), run the cleanup **before** the user unlocks.
- If the Mac doesn't require a password, the desktop shows immediately on wake. In that case only a will-sleep cleanup protects the user.

### 5.2 Detecting that screen sharing or recording has started

There is **no public macOS API** for this:
- Unanswered threads: [forum 667549](https://developer.apple.com/forums/thread/667549), [773613](https://developer.apple.com/forums/thread/773613), [132798](https://developer.apple.com/forums/thread/132798).
- Feedback FB14920414 asks for an `NSScreen.isCaptured` ([feedback-assistant #546](https://github.com/feedback-assistant/reports/issues/546)).
- SwiftUI `isSceneCaptured` is iOS/Catalyst only. **Verified:** the 27.2 SDK declares `@available(macOS, unavailable)`.
- ScreenCaptureKit only sees your own streams.
- `CGDisplayIsCaptured` is deprecated.
- Apple DTS says `kCGWindowSharingState` doesn't affect most recorders ([forum 760234](https://developer.apple.com/forums/thread/760234)), and `sharingType = .none` is reported as ignored by ScreenCaptureKit on 15.4+ ([forum 792152](https://developer.apple.com/forums/thread/792152)).

| # | Approach | What it catches | Latency | Permission | Reliability |
|---|---|---|---|---|---|
| A | **`SLSIsScreenWatcherPresent()`** (SkyLight; old name `CGSIsScreenWatcherPresent`, declared `bool (void)` in [CGSInternal](https://github.com/rentzsch/ForceQuitUnresponsiveApps/blob/master/CGSInternal/CGSMisc.h)), via `dlopen` + `dlsym` | Any capture in the session: sharing, recording, mirroring/AirPlay. Confirmed with Zoom on macOS 26.6.1 ([maclens #29](https://github.com/amrakshay/maclens/issues/29), [RESEARCH.md](https://github.com/amrakshay/maclens/blob/main/RESEARCH.md)) and with `screencapture -V` ([headsup PR #1](https://github.com/ion05/headsup/pull/1)). Shipped in sandboxed Nubrix on macOS 14+ ([Nubrix 0.4.0](https://github.com/murathanbakti/nubrix/releases/tag/v0.4.0)). **I verified that the symbol exists on macOS 27.2 and returns `false` when nothing is capturing.** | Poll at 1–2 Hz | none | **High (empirical) but private.** Gives yes/no only: no app name, no window-vs-display. False positives come from DisplayLink docks ([ghostty #9056](https://github.com/ghostty-org/ghostty/discussions/9056)), always-on recorders, OBS with a display source loaded, and your own capture. |
| B | `SLSRegisterNotifyProc(cb, 1502/1503, ctx)` | Capture start/stop pushed by WindowServer ([nyx PR #4](https://github.com/aristocrat71/nyx/pull/4)). CGSInternal names 1502/1503 `kCGSessionRemoteConnect`/`Disconnect` ([CGSEvent.h](https://github.com/NUIKit/CGSInternal/blob/master/CGSEvent.h)). I confirmed the symbol exists on 27.2. | Immediate | none | **Medium:** one source, and the callback signature is unverified. Keep A as a backstop. |
| C | Display reconfiguration: `CGDisplayRegisterReconfigurationCallback`, `CGDisplayIsInMirrorSet`, `CGDisplayIsBuiltin`, `NSApplicationDidChangeScreenParametersNotification` | School projector or HDMI connected, mirroring | Immediate | none | **High for projectors**, public. Doesn't see app-level capture. |
| D | Mic/camera in use: `kAudioDevicePropertyDeviceIsRunningSomewhere`, `kCMIODevicePropertyDeviceIsRunningSomewhere`; per process via `kAudioHardwarePropertyProcessObjectList` + `kAudioProcessPropertyIsRunningInput` / `BundleID` (macOS 14+; verify) | "A call is starting": a **pre-trigger** | Immediate | none | High for "in a call", not for "sharing". |
| E | Process heuristics: Zoom `CptHost` (in Zoom's Frameworks; Zoom staff link it to sharing, [Zoom community](https://community.zoom.com/t5/Meetings/Zoom-screen-share-failed-to-start-Please-try-again-later-Error/m-p/65483)); `screencapture`/`screencaptureui` plus a temp `.mov` in `~/Library/ScreenRecordings/` for the built-in recorder ([repo](https://github.com/TomasHubelbauer/macos-screen-recording-detect)); `screensharingd` for incoming macOS Screen Sharing | Specific apps | Poll | none | Low-Medium: version-fragile. Teams, Meet and Discord have no stable signature. |
| F | AX/window heuristics: Zoom's share toolbar, Chrome's "… is sharing your screen" bar, Control Center's screen-recording menu extra | Specific apps | AXObserver | Accessibility | Low-Medium: the labels are **localized**. |
| G | Calendar (EventKit, `requestFullAccessToEvents` on 14+) | Scheduled meetings and classes | Proactive | Calendars | Medium. Works well as a "pre-meeting sweep". |
| H | Purple menu-bar / Control Center indicator | – | – | – | No API. Scraping it through AX is variant F. |

**Recommendation:** use A (+B) as the primary signal, C for projectors, and D/G for a proactive sweep. Detection is inherently reactive: the first frames of a share can go out before the cleanup finishes (≈0.2–1.5 s). So also offer a "pre-meeting" mode that cleans up when a meeting app's mic or camera turns on, when a calendar event starts, or when the user clicks Share. Keep the private API behind `dlsym` and fail open: treat a missing symbol as "unknown", tell the user, and fall back to C/D/G.

---

## 6. Permissions, TCC pitfalls, distribution

| Need | TCC service | Check | Request | Required plist / entitlement |
|---|---|---|---|---|
| AX read/act | Accessibility | `AXIsProcessTrusted()` | `AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})` opens System Settings. There is no callback, so poll. | – |
| AppleScript to browsers and doc apps | Automation (AppleEvents), **one prompt per target app** | `AEDeterminePermissionToAutomateTarget(desc, typeWildCard, typeWildCard, false)` → `errAEEventWouldRequireUserConsent` (-1744) / `errAEEventNotPermitted` (-1743) | Same call with `true`, or the first real event | `NSAppleEventsUsageDescription` + `com.apple.security.automation.apple-events` (hardened runtime; [forum 130949](https://developer.apple.com/forums/thread/130949)) |
| Window titles via CG | Screen Recording | `CGPreflightScreenCaptureAccess` | `CGRequestScreenCaptureAccess` (denied once → never re-prompts) | **Avoid** |
| Listen-only event tap | Input Monitoring | `IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)` / `CGPreflightListenEventAccess` | `IOHIDRequestAccess` / `CGRequestListenEventAccess` | **Not needed:** the panic hotkey uses Carbon `RegisterEventHotKey` |
| Posting ⌘W etc. | Accessibility (PostEvent) | `CGPreflightPostEventAccess` | – | Only for the Firefox AX fallback |
| Calendar pre-trigger | Calendars | `EKEventStore.authorizationStatus` | `requestFullAccessToEvents` | `NSCalendarsFullAccessUsageDescription` |

**Permission-free:** NSWorkspace, CGWindowList (without titles), IOKit power/clamshell, idle time, SkyLight watcher, CoreAudio/CoreMediaIO in-use, display config, Firefox session file. **Full Disk Access is not needed**; don't read `~/Library/Safari`.

**TCC pitfalls:**
1. Grants are keyed to bundle id + code-signing requirement. Ad-hoc signatures change on every build, so the Accessibility toggle stays on while `AXIsProcessTrusted()` returns false. Sign dev builds with a **stable** certificate (Apple Development from a free personal team, or a self-signed one) and use `tccutil reset Accessibility|AppleEvents <bundle-id>`.
2. `cargo tauri dev` runs a bare binary, so TCC attributes it to Terminal or the IDE (the responsible process). Test permissions with bundled `.app` builds.
3. Do **all** Automation prompts during onboarding. A prompt that first appears during a screen-share trigger blocks the Apple Event and shows a modal while you're sharing.
4. `AEDeterminePermissionToAutomateTarget` returns `procNotFound` (-600) if the target isn't running, and it can block, so call it off the main thread ([mjtsai](https://mjtsai.com/blog/2018/08/31/aedeterminepermissiontoautomatetarget-added-but-aepocalyse-still-looms/)).
5. Prefer in-process OSA (OSAKit / `NSAppleScript`) over spawning `osascript`. It's faster and the prompt clearly names your app.
6. Deep link for onboarding: `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` (and `…?Privacy_Automation`).
7. `tauri-plugin-autostart`'s `MacosLauncher::AppleScript` scripts System Events, which adds another Automation prompt. Use `LaunchAgent` (or SMAppService).

**Sandbox / Mac App Store: effectively impossible.**
- Apple DTS: Accessibility-gated APIs "are not compatible with the App Sandbox", and "We do not support the Accessibility APIs in sandboxed apps" ([forum 820594](https://developer.apple.com/forums/thread/820594)).
- Apple Events need `scripting-targets` or a temporary-exception entitlement, and whether those are allowed is entirely App Review's call ([QA1888](https://developer.apple.com/library/content/qa/qa1888/_index.html), [forum 823455](https://developer.apple.com/forums/thread/823455)).
- `NSRunningApplication.terminate()` doesn't work from the sandbox.
- **Ship outside the App Store** with Developer ID.

**Developer ID + notarization:**
- Apple Developer Program, $99/year; free accounts can't notarize ([Tauri docs](https://v2.tauri.app/distribute/sign/macos/)).
- Needs a "Developer ID Application" certificate, hardened runtime (Tauri default `hardenedRuntime: true`, [config.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-utils/src/config.rs)) and the entitlement `com.apple.security.automation.apple-events`.
- `dlsym` into SkyLight needs no extra entitlement. Notarization scans for malware and checks signing; it doesn't police private APIs.
- Notarization env vars: `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` or `APPLE_API_ISSUER`/`APPLE_API_KEY`/`APPLE_API_KEY_PATH`.
- Without notarization:
  - Sequoia+ users must use System Settings → Privacy & Security → "Open Anyway".
  - Ad-hoc signing breaks TCC grants on every update.
  - Homebrew 5.0 deprecated casks that fail Gatekeeper, with a disable date of 2026-09-01 ([Workbrew](https://workbrew.com/blog/homebrew-5-0-0)).
- **Budget the $99.**

---

## 7. Calling this from Rust / Tauri v2

### 7.1 Crates (versions from crates.io on 2026-11-16)

| Crate | Version | Use |
|---|---|---|
| `tauri` | **2.12.5** (3.0.0-alpha.6 exists; stay on v2) | App shell |
| `objc2` | 0.6.5 | ObjC runtime |
| `objc2-app-kit`, `objc2-foundation` | 0.3.2 | NSWorkspace, NSRunningApplication, NSDistributedNotificationCenter |
| `objc2-application-services` | 0.3.2 | **AX:** features `HIServices`, `AXUIElement`, `AXError`, `AXAttributeConstants`, `AXActionConstants`, `AXNotificationConstants`, `AXRoleConstants`, `libc`. `AXUIElement::new_application(pid)`, `copy_attribute_value`, `perform_action`, `set_attribute_value`, `set_messaging_timeout`, `AXObserver`, free fn `AXIsProcessTrustedWithOptions(Option<&CFDictionary>) -> bool` ([docs.rs](https://docs.rs/objc2-application-services/0.3.2/objc2_application_services/struct.AXUIElement.html)) |
| `objc2-core-foundation`, `objc2-core-graphics`, `objc2-io-kit` | 0.3.2 | CF types; CGWindowList / CGEventSource / CGDisplay; power and clamshell |
| `objc2-osa-kit` 0.3.2 or `osakit` 0.3.1; `objc2-scripting-bridge` 0.3.2 | – | In-process AppleScript/JXA |
| `objc2-core-audio`, `objc2-core-media-io`, `objc2-event-kit` | 0.3.2 | Pre-triggers |
| `accessibility-sys` / `accessibility` | 0.2.0 (Mar 2025) | Alternative AX bindings. The safe layer is "pretty spotty" ([eiz/accessibility](https://github.com/eiz/accessibility)). |
| `core-foundation` / `core-graphics` | 0.10.1 / 0.25.0 | Older servo crates. Prefer `objc2-*`. |
| `tauri-plugin-global-shortcut` / `global-hotkey` | 2.4.1 / 0.10.0 | Uses Carbon `RegisterEventHotKey`; a CGEventTap only for media keys ([source](https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/macos/mod.rs)), so **no Input Monitoring** |
| `tauri-plugin-macos-permissions` | 2.3.0 | Check/request Accessibility, Screen Recording, Input Monitoring, FDA, camera, mic. **No Automation**, so write that yourself. |
| `sysinfo` | 0.40.3 | Process heuristics |
| `cidre` | 0.30.1 | Alternative Apple bindings |

Note: the `objc2-*` framework crates were last released in Oct 2025, so check for a 0.4 before starting. Avoid `x-win`, `active-win-pos-rs` and `xcap` for titles: they rely on CGWindowList or capture and pull in Screen Recording.

### 7.2 Sketches
```rust
// Private screen-watcher probe (fail open)
type WatcherFn = unsafe extern "C" fn() -> bool;
fn screen_watcher_present() -> Option<bool> {
    unsafe {
        let h = libc::dlopen(c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight".as_ptr(), libc::RTLD_LAZY);
        if h.is_null() { return None; }
        let sym = libc::dlsym(h, c"SLSIsScreenWatcherPresent".as_ptr());
        (!sym.is_null()).then(|| std::mem::transmute::<*mut libc::c_void, WatcherFn>(sym)())
    }
}
```
```rust
// AX attribute read (objc2-application-services 0.3.2; check exact CF helper names)
unsafe fn ax_attr(el: &AXUIElement, name: &str) -> Option<CFRetained<CFType>> {
    let mut out: *const CFType = std::ptr::null();
    let err = el.copy_attribute_value(&CFString::from_str(name), NonNull::from(&mut out));
    (err == AXError::Success).then(|| NonNull::new(out as *mut CFType)).flatten()
        .map(|p| CFRetained::from_raw(p))
}
```

### 7.3 Threading and Tauri integration
- **Threads:**
  - Main thread: Tauri's NSApplication loop; register NSWorkspace and distributed observers there via `app.run_on_main_thread`.
  - One **AX worker thread** with its own CFRunLoop for AXObservers and all AX calls (timeout 0.5–1 s).
  - One **OSA worker** for Apple Events, with per-call deadlines.
  - An IOKit notification port on a run loop for clamshell and power.
  - A 1–2 Hz watcher poll.
- **Tauri config:**
  - `app.set_activation_policy(ActivationPolicy::Accessory)` for a menu-bar app; tray via `TrayIconBuilder`.
  - `src-tauri/Info.plist` (merged) with `NSAppleEventsUsageDescription` (localized through `hu.lproj/InfoPlist.strings`); `bundle.macOS.entitlements`.
  - `bundle.macOS.minimumSystemVersion: "14.0"`.
  - Set `contentProtected` on the app's own window, but **also hide the app's own UI when the watcher fires**, because sharingType is not honoured by ScreenCaptureKit on 15.4+.
- **Auditability:** no updater, `http` plugin or remote assets; strict CSP; a `cargo-deny` ban on `reqwest`/`hyper`/`ureq` in CI. The native-messaging bridge goes over a Unix socket only.

---

## 8. Suggested MVP order
1. Rust core:
   - running apps + AX window/title/document scan;
   - Chromium + Safari AppleScript tab scan/close;
   - rules engine with **hide** as the default action;
   - panic hotkey;
   - onboarding for Accessibility + Automation.
2. Triggers: lock, sleep, clamshell, idle, display reconfiguration, then the SkyLight watcher with poll + notify.
3. Unsaved-document flow (§3.3), AppleScript-first.
4. Companion WebExtension (Firefox first, then Chromium) over native messaging.
5. Pre-meeting signals (mic/camera per process, EventKit).

## 9. Must-verify-on-device test matrix
- Watcher (A/B) with Teams, Meet in Chrome, Discord, OBS, QuickTime, the built-in Screen Sharing app, AirPlay, Sidecar and DisplayLink, on 14, 15, 26 and 27.
- Whether Safari private windows are enumerable, and how to detect them.
- Opera's and Dia's dictionaries.
- `AXDocument` in Word, Acrobat and Pages.
- That AX and Apple Events work while the screen is locked.
- `NSRunningApplication.hide()` on other apps without TCC.
- The Firefox private-title suffix.
- Shortcuts "Automations" on Tahoe as an optional user-configurable trigger bridge (not researched; search budget exhausted).
