# "Bezártam?" / "Did I close it?": technical feasibility on Windows 10/11 and Linux

*Research date: 2026-11-16. Each claim is either linked to a primary source or marked **unverified**. Crate versions were read from the crates.io API on the research date.*

---

## 0. Executive summary

| Capability | Windows 10/11 | Linux X11 (any DE) | GNOME Wayland (Mutter 50/51) | KDE Plasma 6 Wayland | wlroots / Hyprland / niri / COSMIC |
|---|---|---|---|---|---|
| List top-level windows (title, app, PID) | **Full** (EnumWindows + WinEvent hooks) | **Full** (EWMH) | **Partial**: needs our own GNOME Shell extension; AT-SPI frames are a partial fallback | **Full via KWin script** (D-Bus loaded) | **Full** (wlr-foreign-toplevel / ext-foreign-toplevel-list) |
| Close or minimize a window | **Full** (WM_CLOSE / ShowWindowAsync); elevated windows blocked by UIPI | **Full** (_NET_CLOSE_WINDOW, WM_DELETE_WINDOW, iconify) | **Partial**: only through our extension | **Full via KWin script** (`closeWindow()`, `minimized=true`) | **Full** on wlr protocol (close/minimize); COSMIC uses its own protocol |
| Read browser tab titles | **Full** (UIA; Chromium 138+ native UIA, Firefox 143+ UIA rollout) | **Partial** (AT-SPI; Chromium needs a11y enabled at launch) | **Partial** (AT-SPI, same caveat) | **Partial** (AT-SPI) | **Partial** (AT-SPI) |
| Read URL of the active tab | **Partial** (omnibox ValuePattern: displayed text, scheme often elided) | Partial (AT-SPI) | Partial | Partial | Partial |
| Read URLs of background tabs | **Not possible via UIA** without activating the tab (visible). Possible with a companion WebExtension | Same | Same | Same | Same |
| Close a specific tab | **Full** (UIA Invoke on the tab's close button; layout-dependent) | Partial (AT-SPI action on close button) | Partial | Partial | Partial |
| Office documents (Word/Excel/PowerPoint) | **Full** via COM (`Documents`, `Workbooks`, `Presentations`) | n/a (LibreOffice: AT-SPI, window titles, lock files) | same | same | same |
| Handle "Save changes?" | **Partial** (Office COM is exact; other apps need UIA dialog detection) | Partial (AT-SPI) | Partial | Partial | Partial |
| Lock/unlock trigger | **Full** | **Full** (logind LockedHint / ScreenSaver signals) | Full | Full | Partial (depends on locker) |
| Sleep/resume, lid | **Full** (with Modern Standby caveats) | **Full** (logind PrepareForSleep + delay inhibitor, UPower LidIsClosed) | Full | Full | Full |
| Idle time | **Full** | Full (XScreenSaver ext.) | Full (org.gnome.Mutter.IdleMonitor) | Full (ext-idle-notify v2) | Full (ext-idle-notify) |
| **Screen sharing/recording started** | **Partial, good coverage**: undocumented ConsentStore registry + RegNotifyChangeKeyValue catches Windows Graphics Capture users (Chrome, Edge, Teams, Zoom, Discord, Slack, OBS-WGC). Misses DXGI Desktop Duplication, GDI and tab-only capture | **Not possible** in general (X11 capture is silent); heuristics only | **Partial, good**: all capture goes through portal → PipeWire screencast nodes can be monitored | Same as GNOME | Same as GNOME |
| Projector / display duplication | **Full** (WM_DISPLAYCHANGE + QueryDisplayConfig topology) | Full (RandR) | Full (wl_output / Mutter DisplayConfig) | Full | Full |
| Global hotkey | **Full** (RegisterHotKey) | **Full** (XGrabKey) | **Partial** (GlobalShortcuts portal, GNOME 48+, user approves) | Full (portal) | Hyprland yes; Sway/wlr portal **no** (weakly sourced) → CLI + compositor bind |
| Autostart | Full (HKCU Run / MSIX StartupTask) | Full (XDG autostart) | Full | Full | Full |

**Bottom line:** Windows is the best of the three platforms after macOS. Everything in the brief can be built there, and screen-share detection is decent if you accept an undocumented registry signal. Linux has two halves. X11 makes window control trivial but screen-capture detection impossible. Wayland is the reverse: detecting screen sharing is fairly clean through PipeWire, but window control differs for every compositor, and GNOME needs a shell extension. In 2026 Wayland is the main Linux target: GNOME 50 removed the X11 session, and Plasma 6.8 (released October 2026) is Wayland-only.

For browser tabs on both OSes, accessibility APIs give titles and the active URL only. **A companion WebExtension with native messaging is the only reliable way to get every tab's URL and close tabs by ID**, and it stays local because it talks over stdio. Plan for it in v1.x, with UIA/AT-SPI as the fallback.

---

## 1. Windows 10/11

### 1.1 Platform status
- Windows 10 is still a live target. Microsoft reportedly extended the free consumer ESU program to **October 12, 2027** ([Tom's Hardware](https://tomshardware.com/software/windows/microsoft-extends-free-windows-10-security-updates-for-a-second-year), [BleepingComputer](https://bleepingcomputer.com/news/microsoft/microsoft-quietly-extends-free-windows-10-esu-support-to-october-2027)). Several detection features below are Windows 11-only or unverified on Windows 10.
- Current Insider work is on 26H2 (Experimental build 26340.9482, Sept 2026). In that build the capture border changed from **yellow to red**; this is a visual change only and there is no API ([release notes](https://learn.microsoft.com/en-us/windows-insider/release-notes/experimental/preview-build-26340-9482)).

### 1.2 Enumerating windows and processes
- **EnumWindows** with filters: `IsWindowVisible`, no owner (`GetWindow(GW_OWNER)==NULL`), not `WS_EX_TOOLWINDOW`, and `DwmGetWindowAttribute(DWMWA_CLOAKED)==0`. The cloaked check drops suspended UWP windows and windows on other virtual desktops; keep those if you want to act on them.
- **GetWindowTextW** is safe on other processes' windows. For a foreign window it reads the caption from the system's internal store and does not send WM_GETTEXT, so a hung app can't block you ([docs](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getwindowtexta), [Raymond Chen](https://devblogs.microsoft.com/oldnewthing/20030821-00/?p=42833)). Browser windows carry the active tab title (`"<tab title> - Google Chrome"`), which is a useful fallback when UIA fails.
- **Process info:** `GetWindowThreadProcessId` → `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW`. This works for elevated processes as well. Command lines (for example `AcroRd32.exe "C:\…\bank.pdf"`) can be read with `sysinfo` (PEB read; same user, non-elevated targets).
- **UWP/WinUI quirk:** many Store apps show an `ApplicationFrameWindow` owned by `ApplicationFrameHost.exe`. To find the real PID, read the child `Windows.UI.Core.CoreWindow`.
- **Event-driven updates:** `SetWinEventHook(EVENT_OBJECT_CREATE/DESTROY/NAMECHANGE/SHOW, WINEVENT_OUTOFCONTEXT)` avoids polling and DLL injection. This is also how the "item open longer than N hours" timer gets accurate first-seen timestamps.
- **Which file is open?** No general API exists. In order of usefulness: window title parsing, process command line, Office COM `FullName`, and the Restart Manager (`RmStartSession`/`RmRegisterResources`/`RmGetList`), which answers "which processes hold this known file". Explorer windows can be enumerated with `IShellWindows`; how this behaves with Windows 11 Explorer tabs is **unverified**.

### 1.3 Browser tabs via UI Automation (IUIAutomation)
**Status in 2026**
- **Chromium (Chrome, Edge, Brave, Vivaldi, Opera):** native UIA has been on by default since **Chrome 138**. The policy `UiAutomationProviderEnabled` could revert it only through Chrome 146 ([Chrome blog, Aug 2025](https://developer.chrome.com/blog/windows-uia-support-update?hl=en)). RPA vendors reported breakage from the switch ([Blue Prism](https://community.blueprism.com/t5/Blogs/Chrome-147-Removes-UIA-Workaround-Update-Your-Blue-Prism/ba-p/125213)), so selectors must target the native tree.
- **Firefox:** native UIA shipped in **Firefox 143** (Sept 2025) as a staged rollout. The pref is `accessibility.uia.enable` (0 = off, 1 = on, 2 = on unless an incompatible client is detected) ([intent to ship](https://groups.google.com/a/mozilla.org/g/dev-platform/c/qljNUMTNtyA), [143 notes](https://www.firefox.com/en-US/firefox/143.0/releasenotes)). Where native UIA is off, UIA still reaches Firefox through the MSAA/IA2 proxy, and `LegacyIAccessiblePattern.DoDefaultAction` replaces `Invoke`.

**Reading tabs.** The expected tree is browser window (`Chrome_WidgetWin_1`) → `Tab` control → `TabItem` elements. `Name` is the page title, with localized decorations such as "audio playing". The active tab has `SelectionItemPattern.IsSelected`. **Confirm the exact tree per browser and version with Accessibility Insights for Windows or inspect.exe.** Edge vertical tabs, collapsed tab groups and pinned tabs change it. Keep an index that stays current through UIA `StructureChanged` and `PropertyChanged(Name)` events, so nothing has to be scanned at trigger time.

**Reading the URL.** Find the omnibox (`ControlType.Edit`; English name "Address and search bar", localized, so match structurally instead) and read `ValuePattern.Value` ([community examples](https://kaweah.freedombox.rocks/kiwix/content/stackoverflow.com_en_all_2023-11/a/40638519)). Caveats:
- It is the **displayed** text, so the scheme and `www.` may be elided.
- While the user is typing it returns the typed text.
- It is missing in full-screen mode.
- **Only the active tab's URL is exposed.** Background tabs give titles only, unless you select the tab first, which is visible to a meeting audience.

Workaround: cache each tab's URL whenever it becomes active, keyed by a runtime ID, and fall back to title rules (bank names, "Inbox", etc.).

**Closing a specific tab**
- **Preferred: UIA `InvokePattern.Invoke()` on the TabItem's child close Button.** It needs no focus, injects no input, and works on background windows. Identify the button structurally (the Button child of the TabItem), not by its localized name ("Close"/"Close tab"/Hungarian); the exact names are **unverified**.
- **Ctrl+W after focusing: avoid.** It needs `SetForegroundWindow`, which foreground-lock rules restrict, plus `SelectionItemPattern.Select()` on the tab (visible), plus `SendInput`, which goes to whatever window is in front at that instant (race-prone) and is blocked by UIPI for elevated targets. Use it only as a last resort when the tab is already active.
- **"Hide" for a tab** has no direct API. Options: select a neutral tab (visible switch) or minimize the whole browser window.

**Engineering notes**
- Run UIA on a dedicated MTA thread.
- Set `IUIAutomation2::ConnectionTimeout`/`TransactionTimeout` so a hung browser can't freeze you.
- Use `CacheRequest` to fetch Name, ControlType and RuntimeId in bulk.
- Chromium turns on accessibility modes when it detects a UIA client, which **may raise browser CPU and memory use**. Measure this.

**Better long-term option: WebExtension + native messaging.**
- `chrome.tabs.query` returns every tab's real URL, and `chrome.tabs.remove(id)` closes exactly one tab. Incognito access needs the user to allow it.
- Transport is stdio to a local host binary, registered at `HKCU\Software\Google\Chrome\NativeMessagingHosts\<name>` ([Chrome docs](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)). Edge and Firefox use their own registry keys.
- Costs: Chrome Web Store publication (stable Chrome on Windows does not allow off-store extensions without policy) and AMO signing for Firefox.
- It needs no network and stays auditable.

### 1.4 Closing apps: WM_CLOSE and unsaved changes
- Use `PostMessageW(hwnd, WM_CLOSE)`, never `SendMessage`, so you don't block. Equivalents are `WM_SYSCOMMAND/SC_CLOSE` and UIA `WindowPattern.Close()`. The app then runs its normal close path:
  - It may show a save prompt.
  - It may minimize to the tray (Teams, Discord, Slack).
  - It may close **all** tabs or documents in that window (browsers, Acrobat, tabbed Notepad).
  - It may ignore the message.
- Some modern apps keep unsaved content through session restore and close without asking. The Windows 11 Notepad behavior here is **unverified** and depends on the app's settings.
- **UIPI:** messages from a medium-integrity process to an elevated window are dropped silently, even though the APIs report success ([MS blog](https://learn.microsoft.com/en-us/archive/blogs/vishalsi/what-is-user-interface-privilege-isolation-uipi-on-vista)). UIA cannot reach elevated UI either. `uiAccess="true"` needs a signed binary installed under Program Files, and it still cannot cross into elevated UI for non-admin users ([UIA security](https://learn.microsoft.com/windows/win32/winauto/uiauto-securityoverview)). Rule: **skip elevated windows and tell the user**. Never run the app elevated.
- **Knowing the save state up front:** Windows has no general "document edited" flag (unlike macOS). Heuristics: a leading `*` in the title (Notepad, Paint), `●` (VS Code), or Office COM `Saved`.
- **Detecting and answering the prompt with UIA:** after WM_CLOSE, listen for `UIA_Window_WindowOpenedEventId` or poll `GetWindow(hwnd, GW_ENABLEDPOPUP)`, then check `WindowPattern.IsModal`.
  - Classic `#32770` dialogs and TaskDialogs expose the button control IDs as AutomationId (IDYES=6, IDNO=7, IDCANCEL=2), which is language-independent.
  - WinUI ContentDialogs commonly use `PrimaryButton`/`SecondaryButton`/`CloseButton`.
  - Both are **unverified per app**.
  - Office uses its own dialog framework, so use COM there.
- **Safe default rule:** if a modal prompt appears, press Cancel, then minimize, then notify. Pressing "Don't Save" should require an explicit per-rule opt-in.

### 1.5 Office COM and Acrobat
**Attaching.** `CLSIDFromProgID("Word.Application")` → `GetActiveObject` (ROT) → `IDispatch`. Gotchas:
- Office registers in the ROT **only after its window first loses focus** (KB238610) ([MS KB](https://support.microsoft.com/kb/238610)).
- Only the first instance of a ProgID is reachable through the ROT. For extra Excel instances, call `AccessibleObjectFromWindow(OBJID_NATIVEOM)` on each `EXCEL7` child window. This is a known pattern but **unverified here**.
- COM to an elevated Office instance fails.

**Word.** Enumerate `Documents` (`FullName`, `Saved`), then `Document.Close(SaveChanges, OriginalFormat, RouteDocument)`. SaveChanges takes `wdDoNotSaveChanges` (0), `wdSaveChanges` (−1) or `wdPromptToSaveChanges` (−2); a cancelled prompt raises error 4198 ([docs](https://learn.microsoft.com/en-us/office/vba/api/word.document.close(method))). **Documents opened in Protected View (email attachments, downloads) are not in `Documents`.** They are in `ProtectedViewWindows` ([docs](https://learn.microsoft.com/office/vba/api/word.protectedviewwindow.document)). To hide instead of close, set `Window.WindowState` to minimized.

**Excel.** `Workbooks` → `Workbook.Close(SaveChanges, FileName, RouteWorkbook)`. SaveChanges is ignored when there are no changes or when the workbook is open in other windows ([docs](https://learn.microsoft.com/en-us/office/vba/api/excel.workbook.close)).

**PowerPoint.** `Presentation.Close()` **closes without prompting and discards unsaved work**. Call `Save()` first, or set `Saved = True` to discard on purpose ([docs](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.close)). Check `SlideShowWindows` for a running slideshow.

**Acrobat**
- Acrobat Pro/Standard support OLE: `AcroExch.App` → AVDoc enumeration → `AVDoc.Close(bNoSave)`.
- **Acrobat Reader does not support OLE**, only DDE. `DocClose`/`CloseAllDocs` are documented, and DocClose closes **without saving or prompting** ([Adobe IAC](https://opensource.adobe.com/dc-acrobat-sdk-docs/library/interapp/index.html), [DDE messages](https://opensource.adobe.com/dc-acrobat-sdk-docs/library/interapp/IAC_API_DDE_Messages.html)). The DDE service name depends on the version (the `AcroViewR…` form is **unverified**). Rust: `windows` crate `Win32_System_DataExchange` (`DdeInitializeW`, `DdeConnect`, `DdeClientTransaction`).
- Fallbacks: UIA on Reader's document tabs, or WM_CLOSE on the window. PDFs opened in Edge are ordinary browser tabs.

**Rust COM.** With the `windows` crate, `IDispatch::GetIDsOfNames` and `Invoke` with VARIANTs are verbose but workable. Wrap them in a small helper module so they stay auditable.

### 1.6 Minimize vs hide (SW_HIDE risks)
- **Default: `ShowWindowAsync(hwnd, SW_MINIMIZE)` or `SW_SHOWMINNOACTIVE`.** The Async variant can't block on a hung target. UIA alternative: `WindowPattern.SetWindowVisualState(Minimized)`.
- Risks of `SW_HIDE` on another process's window:
  - The taskbar button and Alt+Tab entry disappear, so users may think their work was lost.
  - If your app crashes, the window is orphaned. Persist the list of hidden HWNDs and offer "Restore hidden windows" from the tray and on next start.
  - The taskbar can leave blank buttons when visibility and style change in the wrong order ([Old New Thing](https://devblogs.microsoft.com/oldnewthing/20031229-00/?p=41283)).
  - Tray apps may treat hidden as closed, and some apps re-show themselves.
  - UWP CoreWindows and elevated windows don't respond reliably.
- `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` and `DWMWA_CLOAK` only apply to **your own** windows, so they can't protect other apps' windows without injecting code (don't).

### 1.7 Triggers
| Trigger | API | Notes |
|---|---|---|
| Lock/unlock | `WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)` → `WM_WTSSESSION_CHANGE` (`WTS_SESSION_LOCK`/`UNLOCK`) | May fail with `RPC_S_INVALID_BINDING` if called before `Global\TermSrvReadyEvent` at boot ([docs](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsregistersessionnotification)). **Act on LOCK**, while the screen is hidden, so nothing is visible on unlock. |
| Sleep/resume | `PowerRegisterSuspendResumeNotification(DEVICE_NOTIFY_CALLBACK)` or `RegisterSuspendResumeNotification` → `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC`/`PBT_APMRESUMESUSPEND` | On **Modern Standby (S0ix)** laptops these are reported to be missing ([MS Q&A](https://learn.microsoft.com/en-us/answers/questions/882272/event-that-gets-triggered-on-modern-standby)). Also watch display power and detect wall-clock jumps. |
| Display off/on (Modern Standby entry/exit) | `RegisterPowerSettingNotification(GUID_SESSION_DISPLAY_STATUS / GUID_CONSOLE_DISPLAY_STATE)` | Data: `PowerMonitorOff`/`On`/`Dim` ([GUIDs](https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids)) |
| Lid | `RegisterPowerSettingNotification(GUID_LIDSWITCH_STATE_CHANGE)` → `WM_POWERBROADCAST/PBT_POWERSETTINGCHANGE` | Data 0 = closed, 1 = opened; no callback until a lid device is found and its state is known (same doc) |
| Locked/asleep longer than N minutes | Timestamp on LOCK/SUSPEND/display-off; compare on UNLOCK/RESUME/display-on | Use `GetSystemTimePreciseAsFileTime` plus `QueryUnbiasedInterruptTime` to measure sleep gaps |
| Idle | `GetLastInputInfo` (session-specific; tick not guaranteed monotonic) ([docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlastinputinfo)) or `GUID_SESSION_USER_PRESENCE` | |
| Projector / duplicated display | `WM_DISPLAYCHANGE` → `QueryDisplayConfig(QDC_DATABASE_CURRENT, …, &topologyId)` → `DISPLAYCONFIG_TOPOLOGY_CLONE/EXTEND/EXTERNAL` ([docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)) | Covers the school-projector case directly. Windows' built-in "Do not disturb when duplicating display" has no public API. |
| Sensitive item open > N hours | Your own first-seen bookkeeping (WinEvent hooks) | |

### 1.8 Detecting screen sharing and recording (no official API)
1. **CapabilityAccessManager ConsentStore (best signal, undocumented).**
   - Location: `HKCU\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\graphicsCaptureProgrammatic\NonPackaged\<path with \ replaced by #>` and `…\graphicsCaptureWithoutBorder\…`. Packaged apps are keyed by package family name.
   - Values: `LastUsedTimeStart`/`LastUsedTimeStop` (REG_QWORD FILETIME, UTC); **Stop = 0 while capture is running**.
   - Subscribe with `RegNotifyChangeKeyValue(REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_CHANGE_NAME, bWatchSubtree=TRUE)`; it reacts within a few hundred ms.
   - A working open-source implementation (hdr-switch) reports that this catches **Discord, Chrome, Edge, OBS, Teams, Zoom, Slack** ([hdr-switch](https://github.com/preunec-gmbh/hdr-switch)). Same key format as webcam/mic forensics ([forensics cheat sheet](https://www.windowsforensics.app/en/artifacts/capability-access-manager)).
   - Caveats:
     - **Undocumented by Microsoft**, so it can change in any update.
     - In Chromium, **opening the share picker already creates a record**. That is useful as an early warning, because the picker itself shows live window thumbnails.
     - Sharing **a single Chromium tab** creates no record.
     - It doesn't say which display or window is captured.
     - **DXGI Desktop Duplication**, GDI BitBlt and HDMI capture cards never register.
     - On Windows 10 it is **unverified**: the capability is documented from build 19645 and the API from 20348 ([uap11:Capability](https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-uap11-capability), [GraphicsCaptureAccessKind](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscaptureaccesskind?view=winrt-22621)).
2. **The capture border has no query API.** `GraphicsCaptureSession.IsBorderRequired` is per session in the capturing app. Borderless capture needs consent for `GraphicsCaptureAccessKind.Borderless` ([docs](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired?view=winrt-22621)). Settings → Privacy & security → "Screenshots and screen recording" lists recent activity mainly for Store apps.
3. **Process and window heuristics** (Zoom, Teams, Discord and OBS sharing toolbars, OBS process). These are fragile, version- and locale-specific, and have to be reverse-engineered per app (**unverified** class names). As a pre-emptive trigger, "meeting app launched" or "meeting window appeared" is more robust than "share started".
4. **The Microsoft Teams local third-party API (ws://localhost:8124) is gone.** Microsoft retired it on **June 30, 2026** (MC1266901) ([message center](https://mc.merill.net/message/MC1266901)).
5. **OBS:** obs-websocket on localhost:4455 exposes record/stream status. It is loopback-only, but it is still a socket, the user must opt in, and it needs a password. Treat it as an optional plugin, given the "no network calls" rule.
6. **ETW** (`Microsoft-Windows-Win32k`, which Velociraptor uses for screenshot detection; `Microsoft-Windows-Graphics-Capture-Server`) ([Velociraptor](https://docs.velociraptor.app/exchange/artifacts/pages/windows.etw.screenshottaken), [WinEvtKB](https://winevt-kb.readthedocs.io/en/latest/_sources/sources/eventlog-providers/Provider-Microsoft-Windows-Graphics-Capture-Server.md.txt)). Real-time sessions need admin or Performance Log Users rights and are noisy, so not recommended.
7. **Latency:** when the trigger fires, the first frames may already be shared. Design for **pre-emptive sweeps**: a hotkey, a meeting-app launch, the share-picker record, and LOCK/lid-close.

**Verdict:** partially possible with good practical coverage. Ship it with clear UI copy that detection is best-effort.

### 1.9 Hotkeys, autostart, signing, distribution
- **Global hotkey:** `RegisterHotKey(hwnd, id, MOD_CONTROL|MOD_ALT|MOD_NOREPEAT, vk)` → `WM_HOTKEY`. It fails if the combination is already taken, so offer rebinding. The `global-hotkey` crate supports Windows, macOS and Linux **X11 only** ([repo](https://github.com/tauri-apps/global-hotkey)).
- **Autostart:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (no admin; users can disable it in Task Manager). For MSIX use the `windows.startupTask` extension. `tauri-plugin-autostart` covers both.
- **Code signing and SmartScreen** ([Microsoft, updated 2026-10-29](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)):
  - **Microsoft Store (MSIX):** free, Microsoft re-signs the package, **no SmartScreen warnings**, and the developer account is free. This is the best option for a student. A full-trust (`runFullTrust`) desktop app can still use EnumWindows, UIA and RegisterHotKey; confirm store policy for an app that closes other apps' windows.
  - **Azure Artifact Signing** (formerly Trusted Signing): about $9.99/month, **individuals only in the USA/Canada**, so not available to an individual in Hungary (EU *organizations* qualify). Reputation builds over time.
  - **OV certificate:** $150–300/year, key on an HSM or token, reputation builds over time. **EV no longer bypasses SmartScreen (since 2024).**
  - **SignPath Foundation:** free for OSI-licensed open-source projects. The certificate is issued to SignPath Foundation, which appears as the publisher, and MFA is required ([SignPath](https://signpath.org/)).
- **Antivirus heuristics:** enumerating windows, reading browser URLs, reading registry privacy keys and closing other apps' windows looks like spyware or an infostealer. Signing, a public repository, reproducible builds and no network access reduce false positives but don't eliminate them.

### 1.10 Windows Rust crates
| Crate (version on 2026-11-16) | Use |
|---|---|
| `windows` **0.62.2** (last release Oct 2025) | Features: `Win32_UI_WindowsAndMessaging`, `Win32_UI_Accessibility` (UIA, WinEvent hooks), `Win32_System_RemoteDesktop` (WTS), `Win32_System_Power`, `Win32_System_Registry` (RegNotifyChangeKeyValue), `Win32_Devices_Display` (QueryDisplayConfig), `Win32_System_Com`/`Ole`/`Variant` (Office COM), `Win32_System_DataExchange` (Acrobat DDE), `Win32_System_RestartManager`, `Win32_Graphics_Dwm`, `Win32_System_Threading` |
| `uiautomation` **0.25.1** (Sept 2026, leexgone/uiautomation-rs) | Safe IUIAutomation wrapper: matchers, patterns (Invoke, Value, SelectionItem, Window), event handlers |
| `windows-registry` | Ergonomic ConsentStore reads |
| `sysinfo` **0.40.3** | Process list, executable path, command line, start time |
| `global-hotkey` / `tauri-plugin-global-shortcut` | Hotkeys (Windows, macOS, X11) |
| `tauri` **2.12.5** + `tauri-plugin-autostart` | App shell, tray, autostart, MSI/NSIS bundles |

---

## 2. Linux (GNOME and KDE, X11 and Wayland)

### 2.1 Landscape in late 2026
- **GNOME 50** removed the X11 session and Mutter's X11 backend; X11 apps keep running through XWayland. GNOME 49 had already disabled the session by default ([GNOME blog](https://blogs.gnome.org/alatiera/2025/06/08/the-x11-session-removal/), [Phoronix](https://phoronix.com/news/gnome-50-alpha), [Fedora change](https://fedoraproject.org/wiki/Changes/WaylandOnlyGNOME)). Ubuntu 26.04 LTS and Fedora 44 ship GNOME 50 (per news reports).
- **KDE Plasma 6.8** (released 14 October 2026) is **Wayland-only**. KDE supports the X11 session of Plasma 6.7 and earlier into early 2027 ([KDE blog](https://blogs.kde.org/2025/11/26/going-all-in-on-a-wayland-future/), [6.8 announcement](https://discuss.kde.org/t/plasma-6-8-is-out-its-kdes-30th-birthday-and-were-kicking-off-our-fundraiser/50936)).
- X11 still matters for Ubuntu 24.04 LTS, Xfce, Cinnamon, MATE and older installs. **Under a Wayland session, an X11 client (x11rb) sees only XWayland windows.**

### 2.2 X11: fully possible
- **List:** read `_NET_CLIENT_LIST` (and `_NET_CLIENT_LIST_STACKING`) on the root window. Per window: `_NET_WM_NAME` (UTF8_STRING), `WM_CLASS`, `_NET_WM_PID` (may be missing). Use `PropertyNotify` on the root and on each client for live updates.
- **Close:** send the `_NET_CLOSE_WINDOW` ClientMessage to the root (source indication 2 = pager), which lets the window manager deliver `WM_DELETE_WINDOW`. The app then runs its normal save prompt.
- **Minimize:** `WM_CHANGE_STATE` IconicState (what `XIconifyWindow` does), or toggle `_NET_WM_STATE_HIDDEN` through the window manager.
- **Activate:** `_NET_ACTIVE_WINDOW`.
- **xdotool** is X11-only (XTEST). Don't shell out to it; implement the calls directly with `x11rb`.
- **Idle:** MIT-SCREEN-SAVER `XScreenSaverQueryInfo`.
- **Hotkey:** `XGrabKey`.
- **Screen-capture detection is impossible:** any client can read the screen silently with `XGetImage`/XShm/XComposite.

### 2.3 Wayland: window list and control per compositor
| Compositor | Mechanism | List | Close / minimize | Notes |
|---|---|---|---|---|
| **GNOME (Mutter)** | No foreign-toplevel protocol. `org.gnome.Shell.Introspect.GetWindows` is **restricted to the portal allow-list** unless Shell runs in unsafe mode ([Discourse](https://discourse.gnome.org/t/unable-to-call-the-remote-getwindows-gnome-method-via-dbus/21201), [commit](https://lists.gnome.org/archives/commits-list/2021-September/msg01053.html)) | Through **our own GNOME Shell extension** exposing D-Bus (pattern: [Window Calls](https://github.com/ickyicky/window-calls) with `List`, `Close`, `Minimize`, …) | Through the extension (`metaWindow.delete()`, `.minimize()`) | Extension must be updated for each GNOME release (ESM since GNOME 45), go through extensions.gnome.org review, and **check the D-Bus sender** (similar extensions have no access control) |
| **KDE KWin 6** | **KWin scripting over D-Bus:** `org.kde.KWin /Scripting` `loadScript(path, name)` → `/Scripting/Script<N>` `run()` ([discussion](https://discuss.kde.org/t/run-kwin-script-from-command-line-with-argument/12984)) | `workspace.stackingOrder` (and `windowList()` in KWin 6, **not shown in the docs page**), `window.caption`, `resourceClass`, `pid`, `desktopFileName`, signals `windowAdded`/`windowRemoved` ([KWin API](https://develop.kde.org/docs/plasma/kwin/api/)) | `window.closeWindow()`, `window.minimized = true` | Send data back with `callDBus(...)` to our zbus service. Alternative `org_kde_plasma_window_management` is **privileged** (needs `X-KDE-Wayland-Interfaces` in the .desktop file, and "regular clients must not use it") ([wayland.app](https://wayland.app/protocols/kde-plasma-window-management)) |
| **wlroots family** (Sway, labwc, river, Wayfire, Hyprland, niri, Jay, Mango, Cage, Qtile) | `zwlr_foreign_toplevel_management_unstable_v1` (v3) ([wayland.app](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1)) | title, app_id, state events | `close()`, `set_minimized()`, `activate()` | Not supported by KWin, Mutter, COSMIC or Weston (per the wayland.app table). Sway IPC and `hyprctl` are alternatives |
| **ext-foreign-toplevel-list-v1** (staging) | COSMIC, Hyprland, Sway 1.11, niri, labwc, river, Mir, phoc, … ; **not KWin 6.7, not Mutter 51** ([wayland.app](https://wayland.app/protocols/ext-foreign-toplevel-list-v1)) | title, app_id, stable identifier | **No close or activate** (list only, by design) | |
| **COSMIC** | ext-foreign-toplevel-list + `cosmic-toplevel-management` | yes | yes | |

**Hiding:** on Wayland, minimizing through the mechanisms above is the only option. There is no equivalent of SW_HIDE.

**Input injection** (for Ctrl+W) is not possible for normal clients on Wayland. The RemoteDesktop portal/libei needs user consent; ydotool needs uinput access (root or `input` group). Avoid both.

### 2.4 AT-SPI2 (browser tabs, documents, dialogs)
- AT-SPI runs over D-Bus, so it works on both X11 and Wayland. On Wayland, screen coordinates aren't meaningful, but actions and names work.
- Desktop → applications → frames: **frame names are window titles**, a partial GNOME fallback without the extension.
- Use `Accessible` children and roles to find page tabs and the close button, then `Action.DoAction` ("press"/"click"). Subscribe to `object:property-change:accessible-name` and `object:children-changed` for live updates.
- **Chromium:**
  - It initializes its AT-SPI bridge only when accessibility is enabled. Its Linux code checks `org.a11y.Status.IsEnabled` and auto-detects Orca by scanning `/proc` ([Chromium source](https://chromium.googlesource.com/chromium/src/+/lkgr/content/browser/accessibility/browser_accessibility_state_impl_auralinux.cc), [Debian source](https://sources.debian.org/src/chromium/138.0.7204.183-1/ui/accessibility/platform/atk_util_auralinux.cc)).
  - Without it, apps expose only an empty skeleton. Workarounds are `--force-renderer-accessibility` or `ACCESSIBILITY_ENABLED=1` ([xa11y-linux notes](https://docs.rs/xa11y-linux/0.7.0/src/xa11y_linux/atspi.rs.html)).
  - Practical consequence: the user enables accessibility once (for example `gsettings set org.gnome.desktop.interface toolkit-accessibility true`) and **restarts the browser**.
  - Whether the tab strip is exposed without renderer accessibility is **unverified**; needs a test.
- **Firefox** exposes ATK/AT-SPI when a11y is enabled. Its tab close buttons are separate accessibles.
- Background-tab URLs have the same limitation as on Windows, so the WebExtension + native messaging host (`~/.config/google-chrome/NativeMessagingHosts/`, `~/.mozilla/native-messaging-hosts/`) is again the reliable path.
- **Documents on Linux:**
  - `/proc/<pid>/cmdline` often contains the opened file path (viewers launched by xdg-open).
  - `/proc/<pid>/stat` gives the start time.
  - `/proc/<pid>/fd` works for apps that keep files open.
  - LibreOffice lock files `.~lock.<name>#` sit next to the open document.
  - Okular has a per-instance D-Bus interface (details **unverified**). LibreOffice UNO can't attach to a running instance unless it was started with `--accept`.

### 2.5 Triggers (D-Bus via zbus)
| Trigger | API |
|---|---|
| Sleep/resume | `org.freedesktop.login1.Manager.PrepareForSleep(b)` (true before, false after). Take a **delay inhibitor**: `Inhibit("sleep", who, why, "delay")`, bounded by `InhibitDelayMaxUSec`. This lets the app close or hide things **before** the machine sleeps ([logind docs](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)) |
| Lock/unlock | Session property **`LockedHint`** (PropertiesChanged; set by the desktop's locker through `SetLockedHint`). Session `Lock`/`Unlock` signals are **requests to the locker**, not state, and GNOME doesn't emit logind `Lock` when it locks itself ([GNOME issue](https://gitlab.gnome.org/GNOME/gnome-shell/issues/741)). Also `org.gnome.ScreenSaver.ActiveChanged(b)` (GNOME) and `org.freedesktop.ScreenSaver.ActiveChanged(b)` (KDE) |
| Lid | UPower `org.freedesktop.UPower` `LidIsClosed`/`LidIsPresent` on the system bus; PowerDevil watches its changes ([UPower](https://upower.freedesktop.org/docs/UPower.html)). logind `Manager.LidClosed` also exists; whether it emits change signals is **unverified** |
| Idle | **GNOME:** `org.gnome.Mutter.IdleMonitor` at `/org/gnome/Mutter/IdleMonitor/Core` `GetIdletime` (session bus; watch-method names: verify against the Mutter XML). **KWin and wlroots:** `ext-idle-notify-v1`, where v2's `get_input_idle_notification` ignores inhibitors; **KWin 6.7 supports it, Mutter does not** ([wayland.app](https://wayland.app/protocols/ext-idle-notify-v1)). **X11:** XScreenSaverQueryInfo. Also logind `IdleHint`/`IdleSinceHint` |
| Projector / new display | X11 RandR `RRScreenChangeNotify`/output events; Wayland `wl_output` global add/remove; GNOME `org.gnome.Mutter.DisplayConfig` `MonitorsChanged` (private API); KDE KScreen |

### 2.6 Detecting screen sharing and recording
- **Wayland (GNOME, KDE, wlroots, Hyprland, COSMIC):** every capture goes through the xdg-desktop-portal ScreenCast or RemoteDesktop interface. The compositor then publishes a **PipeWire video node**.
  - Monitor the PipeWire registry (`pipewire` crate: registry `global`/`global_remove` listener) for `media.class = "Video/Source"` nodes from the compositor or portal. Observed `node.name` values: `gnome-shell` / `meta-screen-cast-src`, `kwin_wayland`, `xdg-desktop-portal-wlr`, and similar ([gist](https://gist.github.com/columbarius/50239ea3c4c70df8f240aa50f88e801a)). Then check for **links to a consumer stream** (`Stream/Input/Video`).
  - Exclude webcams (`device.api` v4l2/libcamera).
  - `media.role = "Screen"` exists in the spec ("recording or sharing the desktop screen"), but clients set it optionally. Only `pipewire.*` properties are trustworthy ([pipewire-props](https://docs.pipewire.org/page_man_pipewire-props_7.html)).
  - Prior art: PipeShare watches `pw-dump --monitor` for ScreenCast nodes ([repo](https://github.com/SavasTanriverdi/PipeShare)).
  - Remote-desktop sessions (gnome-remote-desktop, KRDP) also show up, which is a feature here.
  - The portal emits no public "session started" signal to third parties. `org.gnome.Mutter.ScreenCast` is private ([Mutter XML](https://sources.debian.org/src/mutter/49.4-1/data/dbus-interfaces/org.gnome.Mutter.ScreenCast.xml)), and the GNOME top-bar indicator has no public API.
  - **Verdict: partially possible, good coverage on Wayland. Verify node names per DE version.**
- **X11: not possible** to detect reliably. Use heuristics only: meeting-app processes and windows, the OBS process, or an optional obs-websocket plugin.

### 2.7 Global shortcuts and autostart
- **GlobalShortcuts portal** (`CreateSession`, `BindShortcuts`, `Activated`/`Deactivated`, v2 `ConfigureShortcuts`; the user typically confirms in a dialog) ([spec](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)).
  - Implemented by **GNOME 48+** ([GNOME 48 notes](https://release.gnome.org/48/developers/)), **KDE** and **Hyprland**.
  - Reported **not** implemented by xdg-desktop-portal-wlr (Sway) or COSMIC; that source is weak ([xfwl4 issue](https://gitlab.xfce.org/xfce/xfwl4/-/issues/93)).
  - Use the `ashpd` crate.
  - Universal fallback: ship a CLI (`bezartam panic`) and let users bind it in their DE or compositor.
- **X11:** `XGrabKey` (global-hotkey crate).
- **Autostart:** `~/.config/autostart/<id>.desktop` (XDG Autostart). In Flatpak use the Background portal `RequestBackground(autostart=true)` (ashpd). A systemd `--user` unit is an alternative.

### 2.8 Packaging
- **.deb, .rpm and AppImage** come from the Tauri bundler. Flatpak is not a Tauri target; build with flatpak-builder from the deb or binary. Recent bundler work pulls the linuxdeploy plugin to drop the libfuse requirement for AppImage ([Tauri releases](https://v2.tauri.app/release/tauri-bundler/all-versions/)).
- **Flatpak is a poor fit for v1** ([permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)):
  - The session bus is filtered, so KWin (`--talk-name=org.kde.KWin`) and the GNOME extension's D-Bus name need explicit holes.
  - Host processes are invisible (no `/proc` view, can't kill).
  - Wayland window control is impossible from inside the sandbox anyway. `--socket=x11` *does* allow full X11 window control, because X11 has no isolation.
  - **The AT-SPI bus has no access control**, so sandboxed access is problematic ([GNOME a11y wiki](https://wiki.gnome.org/Projects/GTK/Accessibility)). The Newton project aims to fix this but was not in GNOME Shell when last reported.
  - PipeWire graph visibility for Flatpak clients is portal-mediated and likely restricted (**unverified**).
  - logind and UPower work through `--system-talk-name`.
  - Flathub review will scrutinize all of this.
- **Recommendation:** .deb, .rpm, AppImage and AUR first. Flatpak later, possibly with features reduced.

### 2.9 Linux Rust crates
| Crate (version on 2026-11-16) | Use |
|---|---|
| `x11rb` **0.14.0** | EWMH list/close/iconify, RandR, ScreenSaver, XGrabKey |
| `wayland-client` **0.31.15** + `wayland-protocols` **0.32.13** (staging: ext-foreign-toplevel-list, ext-idle-notify) + `wayland-protocols-wlr` **0.3.12** (foreign-toplevel-management) + `wayland-protocols-plasma` (privileged KDE window management) | Wayland clients |
| `zbus` **6.3.0** | logind, UPower, ScreenSaver, Mutter IdleMonitor/DisplayConfig, KWin scripting, our own D-Bus service for the GNOME extension and KWin callbacks |
| `atspi` **0.30.0** (odilia-app, zbus-based) | Tab titles, close buttons, dialogs. Note: it appears to depend on **zbus 5.x** while `ashpd` 0.14 uses **zbus 6**, so expect two zbus versions in the dependency tree; check this |
| `ashpd` **0.14.0** | GlobalShortcuts and Background/autostart portals |
| `pipewire` **0.10.1** (pipewire-rs) | Screencast node and link monitoring |
| `sysinfo` 0.40.3, `procfs` | Processes, cmdline, start time |

---

## 3. Cross-platform architecture recommendations
1. **Rust core with a provider per platform:**
   - `WindowProvider`: Win32 / X11 / wlr / KWin-script / GNOME-extension.
   - `TabProvider`: UIA / AT-SPI / WebExtension.
   - `DocProvider`: Office COM / Acrobat DDE / title + cmdline heuristics.
   - `TriggerSource`.
   - Tauri 2 for UI, i18n (Hungarian first) and the tray.
2. **Keep the index live and event-driven** (WinEvent/UIA events, EWMH PropertyNotify, AT-SPI signals, KWin callbacks). The screen-share trigger then only has to issue close/minimize calls; there is no slow scan while the audience is already watching.
3. **Act on the way down:** LOCK, PrepareForSleep(true) with a delay inhibitor, display-off and lid-closed, rather than on resume.
4. **Safe defaults:** minimize rather than hide; with unsaved changes, Cancel, then minimize, then notify; skip elevated windows. Discard only through an explicit rule. Office: use COM, and remember that PowerPoint `Close()` discards silently.
5. **Browser companion extension** (Chrome, Edge, Firefox) over local native messaging for exact URLs and per-tab close. UIA/AT-SPI is the fallback and a "no extension installed" mode.
6. **Screen-share detection is best-effort and labelled as such:**
   - Windows: ConsentStore watcher plus display-topology watcher plus meeting-app heuristics.
   - Wayland: PipeWire watcher.
   - X11: heuristics only.
   - Always offer the panic hotkey and a pre-meeting sweep.
7. **Distribution:** Microsoft Store MSIX (free signing, no SmartScreen) or SignPath for the GitHub release. Linux: deb, rpm, AppImage. GNOME: separate extension on extensions.gnome.org.

## 4. Possibly outdated or unverified items
- The ConsentStore `graphicsCapture*` keys are undocumented; Windows 10 behavior is unverified.
- Exact UIA tree, control names and AutomationIds for Chromium and Firefox tabs and close buttons. Firefox 143 UIA was a staged rollout.
- Dialog AutomationIds (#32770 control IDs, ContentDialog IDs) and Notepad session-restore behavior.
- The Acrobat Reader DDE service name, and multi-instance Excel attachment through OBJID_NATIVEOM.
- KWin 6 `workspace.windowList()` (only `stackingOrder` is shown on the docs page).
- Mutter IdleMonitor watch-method names, and whether logind `LidClosed` emits change signals.
- PipeWire screencast node names per DE version.
- GlobalShortcuts support in Sway/xdg-desktop-portal-wlr and COSMIC.
- Whether the Chromium tab strip is exposed over AT-SPI without renderer a11y.
- The `windows` crate's latest release is 0.62.2 (Oct 2025). Check for newer windows-rs releases before starting.
- wayland.app compatibility tables (KWin 6.7, Mutter 51) may lag Plasma 6.8.
