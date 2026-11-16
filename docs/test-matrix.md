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
| DisplayLink dock with DisplayLink on the ignore list (expect no share, watcher marked Unreliable) | | | | | | |
| Zoom share while the DisplayLink dock is attached (caught by the sharing bar) | | | | | | |

### Sharing bars (`macos.share_ui`)

Run `cargo run -p closecall-platform-macos --example share_ui 30` during each share and note the
cue it saw (layer, size, position) so the thresholds in `signals/share_ui.rs` can be tuned.

| Share | Cue seen | Starts within 2 s | Ends within 4 s after stopping |
|---|---|---|---|
| Zoom (whole screen; `CptHost` only while sharing?) | | | |
| Meet in Chrome, Edge, Brave, Arc, Helium | | | |
| Teams | | | |
| Slack huddle | | | |
| Webex | | | |
| Discord (no known cue) | | | |

### Browsers

| Browser | List tabs | URL of every tab | Close one tab | Private/incognito detected | Automation prompt at onboarding |
|---|---|---|---|---|---|
| Chrome | | | | | |
| Brave | | | | | |
| Edge | | | | | |
| Vivaldi | | | | | |
| Arc | | | | | |
| Helium | | | | | |
| Safari | | | | | |
| Firefox (session file, hide only) | | | | | |
| Opera, Dia (no tab provider; only app windows) | | | | | |

Also check: a notify-only tab next to a closed tab in the same window stays open; a tab that navigated
to a meeting site just before the close is left open; with a meeting tab in the same browser, a Hide
rule skips with "protected" instead of hiding the browser.

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

Also: a Terminal window whose title matches a document rule shows "Terminate running processes?"
and must be cancelled and hidden, never terminated. A window with a Print or Export sheet already
open must be hidden without touching the sheet.

Any data loss in this table blocks the release.

### Other triggers

| Trigger | 14 | 15 | 26 | 27 | Notes |
|---|---|---|---|---|---|
| Panic hotkey | | | | | |
| Lid closed | | | | | |
| Will sleep (the run is not waited for; does it finish before sleep?) | | | | | |
| Locked | | | | | |
| Away longer than N (fires before unlock) | | | | | |
| Idle longer than N | | | | | |
| Item open longer than N (silent wait, no Keep button) | | | | | |
| Call likely (default microphone in use) | | | | | |

### Spikes to settle

| Question | Answer | Date |
|---|---|---|
| Do AX and Apple Events work while the screen is locked? | | |
| Are Safari private windows enumerable, and can we tell them apart? | | |
| Does `NSRunningApplication.hide()` work on other apps without a TCC grant? | | |
| `SLSRegisterNotifyProc` callback signature, and the event number that carries per-process capture data | | |
| Minimum macOS version for per-process mic attribution | | |
| Opera and Dia AppleScript support | | |
| Office for Mac `close … saving` syntax | | |
| Does `saving no` really discard in autosave apps? | | |
