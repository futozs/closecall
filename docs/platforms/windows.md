# Windows

Planned for v0.2: Windows 10 2004+ (22H2 with extended security updates) and Windows 11. Nothing here
is implemented yet. Evidence and sources: [research/platform-windows-linux.md](../research/platform-windows-linux.md).

## Permissions

Windows has no consent prompts for what we do. Onboarding explains instead:

- Elevated (administrator) windows are skipped. Windows silently drops our messages to them (UIPI).
- Office documents become reachable once Office has been focused once.
- The built-in "Turn on do not disturb automatically when duplicating your display" setting.

## Capabilities

| Capability | Support | Mechanism and limits |
|---|---|---|
| Windows, titles, processes | Full | `EnumWindows` with filters, `SetWinEventHook` (out of context, no DLL injection), `QueryFullProcessImageNameW` |
| Tab titles | Full | UI Automation: Chromium 138+ native UIA, Firefox 143+ (staged rollout) |
| Tab URLs | Partial | Only the active tab's address bar; cached when a tab becomes active. Exact URLs for all tabs need the v0.4 extension. |
| Close one tab | Partial | UIA Invoke on the tab's close button, found by structure; depends on browser layout |
| Office documents | Full | COM: Word, Excel, PowerPoint, including Protected View. PowerPoint is saved with `Save()` before `Close()`, which otherwise discards. |
| Other documents | Partial | `WM_CLOSE` plus UIA dialog handling (buttons by AutomationId, not label); title markers `*`/`●` for unsaved state |
| Acrobat Reader | Partial | Minimize by default; close only through `WM_CLOSE` plus dialog handling |
| Hide | Full | Minimize every top-level window (`SW_MINIMIZE`). Never `SW_HIDE`, which can orphan windows. |
| Private windows | Partial | Title suffix only |

## Triggers

| Trigger | Mechanism | Confidence |
|---|---|---|
| Screen share | Undocumented ConsentStore registry keys (`graphicsCaptureProgrammatic`, `graphicsCaptureWithoutBorder`) watched with `RegNotifyChangeKeyValue`. Catches Windows Graphics Capture users (Chrome, Edge, Teams, Zoom, Discord, Slack, OBS). The Chromium share picker writes a record before you pick, which gives an early warning. The capturing app is protected. | Medium-High; unverified on Windows 10 |
| Projector | `WM_DISPLAYCHANGE` + `QueryDisplayConfig` topology | High |
| Call likely | ConsentStore microphone and webcam in use by a meeting app | Medium |
| Sleep | `PowerRegisterSuspendResumeNotification`; on Modern Standby also display-off | Partial |
| Lid | `GUID_LIDSWITCH_STATE_CHANGE` | Full |
| Lock / unlock | `WTSRegisterSessionNotification` | Full |
| Idle | `GetLastInputInfo` | Full |
| Panic | `RegisterHotKey`, tray, `closecall panic` | Full |

## Limits

- Not detected: DXGI Desktop Duplication, GDI capture, capture cards, sharing a single Chromium tab.
  The UI says so.
- We refuse DLL injection, so we cannot exclude other apps' windows from capture.
- Our own window uses `WDA_EXCLUDEFROMCAPTURE`, which works on Windows.
- Antivirus heuristics may flag window enumeration. Signed builds, no injection and no network help.
- Distribution: Microsoft Store MSIX, a SignPath-signed NSIS installer and winget.
