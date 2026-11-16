# Test matrix

Manual device tests for what CI cannot do. Fill a cell with ✓ (works), ✗ (fails, link an issue) or a
short note. Record the macOS build, app commit and date at the top of each run. Run every row in
English and Hungarian system language where it involves dialogs or sheets.

## v0.1 (macOS)

Run date: · App commit: · Tester:

### Share detection (SkyLight watcher) and latency

Latency is from share start to the last conceal or close step, in milliseconds, meeting mode on.

| Source | 14 | 15 | 26 | 27 | Conceal ms | Tabs ms |
|---|---|---|---|---|---|---|
| Zoom | | | | | | |
| Microsoft Teams | | | | | | |
| Google Meet in Chrome | | | | | | |
| Google Meet in Safari | | | | | | |
| Discord | | | | | | |
| OBS (display capture) | | | | | | |
| QuickTime screen recording | | | | | | |
| Built-in screen recording (⌘⇧5) | | | | | | |
| macOS Screen Sharing (incoming) | | | | | | |
| AirPlay mirroring | | | | | | |
| HDMI projector | | | | | | |
| Sidecar | | | | | | |
| DisplayLink dock (expect a false positive, marked Noisy) | | | | | | |

### Browsers

| Browser | List tabs | URL of every tab | Close one tab | Private/incognito detected | Automation prompt at onboarding |
|---|---|---|---|---|---|
| Chrome | | | | | |
| Brave | | | | | |
| Edge | | | | | |
| Vivaldi | | | | | |
| Arc | | | | | |
| Safari | | | | | |
| Firefox (session file, hide only) | | | | | |
| Opera (hide only) | | | | | |

### Documents and unsaved policies

| App | Path detected | Dirty detected | save_then_close (file) | save_then_close (untitled) | hide | skip_and_notify | discard | Sheet buttons by role (EN / HU) |
|---|---|---|---|---|---|---|---|---|
| Preview | | | | | | | | |
| TextEdit | | | | | | | | |
| Pages | | | | | | | | |
| Keynote | | | | | | | | |
| Numbers | | | | | | | | |
| Word | | | | | | | | |
| Excel | | | | | | | | |
| PowerPoint | | | | | | | | |
| Acrobat Reader | | | | | | | | |

Any data loss in this table blocks the release.

### Other triggers

| Trigger | 14 | 15 | 26 | 27 | Notes |
|---|---|---|---|---|---|
| Panic hotkey | | | | | |
| Lid closed | | | | | |
| Will sleep | | | | | |
| Locked | | | | | |
| Away longer than N (fires before unlock) | | | | | |
| Idle longer than N | | | | | |
| Item open longer than N (countdown) | | | | | |
| Call likely (mic or camera in a meeting app) | | | | | |

### Spikes to settle

| Question | Answer | Date |
|---|---|---|
| Do AX and Apple Events work while the screen is locked? | | |
| Are Safari private windows enumerable, and can we tell them apart? | | |
| Does `NSRunningApplication.hide()` work on other apps without a TCC grant? | | |
| `SLSRegisterNotifyProc` callback signature | | |
| Minimum macOS version for per-process mic attribution | | |
| Opera and Dia AppleScript support | | |
| Office for Mac `close … saving` syntax | | |
| Does `saving no` really discard in autosave apps? | | |
