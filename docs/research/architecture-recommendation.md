# Architecture recommendation: „Bezártam?” / "Did I close it?"

**Snapshot:** 2026-11-16. Versions come from crates.io and the npm registry, queried on that date. Local toolchain: macOS 27.2 on Apple Silicon, Rust 1.99.1, bun 1.3.14, Node 26.13.1.

**Freshness flags:**
- The `objc2-*` framework crates (0.3.2) and `windows` (0.62.2) were last released in Oct 2025. Check for 0.4 / 0.63 releases before starting.
- Tauri 3 is in alpha. Stay on 2.x.
- TypeScript 7 is the new Go-based compiler.
- Anything marked *(verify)* is unconfirmed on a real device.

**Project location:** `~/Developer/personal/bezartam`, per the author's request. The repo is public on GitHub.

---

## 1. Decisions at a glance

Where the research reports disagreed, this table records the pick and the reason.

| # | Decision | Choice | Why | Rejected |
|---|---|---|---|---|
| 1 | App framework | **Tauri 2.12 (Rust core + React UI)** | One codebase for three OSes. Rust reaches AX, Win32/UIA/COM and D-Bus/Wayland directly. Small binaries, an explicit capability model for the webview, and it fits an author who knows TypeScript and React and wants to learn Rust. PanicMode shows Tauri works for this class of app. | **Swift:** macOS only, would mean two more rewrites. **Electron:** ships Chromium and Node with a full network stack in-process, which is harder to audit. **Tauri 3:** alpha. |
| 2 | Frontend build | **Vite 8 + React 19 SPA** | Tauri needs static assets. Next.js App Router server features are dead weight inside a webview. This departs from the author's default stack (Next.js) on purpose. | Next.js static export |
| 3 | macOS bindings | **objc2 family, in Rust** | One language, one build, auditable. Covers AX, NSWorkspace, IOKit, CoreGraphics, OSAKit and CoreAudio. | A Swift helper via swift-rs (second toolchain); `accessibility` crate (patchy safe layer) |
| 4 | Screen Recording permission | **Never request it** | Not needed. It brings Sequoia's monthly re-consent prompts and the purple indicator, and it would trip our own capture detector. Window titles come from AX. | CGWindowList titles, ScreenCaptureKit |
| 5 | Browser tabs | **OS automation first (v0.1–v0.3), companion extension in v0.4** | AppleScript/JXA on macOS and UIA/AT-SPI elsewhere need no install and ship value in v0.1. The macOS report wanted the extension early; the Windows/Linux report put it at "v1.x". The extension is the only way to get exact URLs for every tab on Windows and Linux, so it comes right after the three OS ports. The native-messaging protocol is defined now so it slots in later. | Extension-only (needs store publishing before any value ships) |
| 6 | Firefox on macOS in v0.1 | **Detect via `recovery.jsonlz4`; act by hiding or minimizing** | Firefox has no tab dictionary. Selecting a tab and sending ⌘W is visible to the audience. The session file is permission-free (about 15 s stale, no private windows). | AX ⌘W fallback |
| 7 | macOS share signal | **`SLSIsScreenWatcherPresent` poll is canonical; `SLSRegisterNotifyProc(1502/1503)` behind an experimental flag** | The poll has been confirmed by several 2025–26 apps and verified to exist on 27.2. The push callback has one source and an unverified signature. | Push-only |
| 8 | Loopback integrations (obs-websocket, Teams localhost API) | **Excluded** | "No network calls" means no AF_INET/AF_INET6 sockets at all, loopback included. The Teams local API was retired on 30 Jun 2026 anyway. | "Optional plugin" (Windows/Linux report) |
| 9 | Conceal action | **One abstract `Hide` action, mapped per OS** | macOS `NSRunningApplication.hide()` is instant and lossless. On Windows and Linux the mapping is minimize, because `SW_HIDE` can orphan windows. Both reports agree once the action is abstracted. | `SW_HIDE`, macOS window minimize as the default (genie animation, live Dock thumbnail) |
| 10 | Linux order | **v0.3 X11, plus the session-independent Linux pieces (logind, UPower, AT-SPI, PipeWire watcher, CLI). Wayland window control in v0.5** | The brief asks for X11 first, and X11 control is cheap. But GNOME 50 dropped X11 and Plasma 6.8 is Wayland-only, so v0.3 must already detect shares on Wayland through PipeWire and degrade gracefully there. | Pure X11 v0.3 |
| 11 | Licence | **GPL-3.0-or-later** for code. Data lists keep their upstream licences. | Forks can't go closed-source and add telemetry, which matters for a privacy tool. Lets us bundle GPL-3.0 lists (HaGeZi) and study GPL code (Tab Wrangler, AutoQuit, BlurrySite, KeePassXC). CC BY-SA 4.0 is one-way compatible into GPLv3 *(verify with a licence FAQ before the first release)*. | MIT/Apache-2.0 |
| 12 | Rust→TS types | **ts-rs 12.0.1** | Stable. Derives TS types from the domain model. | tauri-specta / specta (still `2.0.0-rc`) |
| 13 | Config format | **TOML** with a generated JSON Schema | Easy for people to read and diff. The UI and docs use the schema. | JSON, YAML |
| 14 | Hotkey | **tauri-plugin-global-shortcut** on macOS, Windows and X11; **GlobalShortcuts portal** (ashpd) on Wayland; CLI `bezartam panic` everywhere | Carbon `RegisterEventHotKey` needs no Input Monitoring. Wayland needs the portal. | CGEventTap |
| 15 | macOS permissions code | **Our own ~150-line module** | `tauri-plugin-macos-permissions` (last release May 2025) has no Automation support. | That plugin |
| 16 | Windows distribution | **Microsoft Store MSIX (primary) + SignPath-signed NSIS on GitHub + winget** | Free signing without SmartScreen warnings. Azure Artifact Signing isn't available to individuals in Hungary, and EV no longer bypasses SmartScreen. | OV certificate |
| 17 | NSFW model (v0.6) | **Marqo nsfw-image-detection-384 → ONNX, run with tract-onnx 0.23.10** | Apache-2.0, 5.6M parameters, and tract is pure Rust (no native runtime, nothing phoning home). `ort` is still `2.0.0-rc.15`. | NSFWJS in the webview, Falconsai ViT-base (too large) |
| 18 | Calendar pre-trigger | **Deferred** | One more TCC prompt, and the mic/camera/share-picker signals cover the same case. | EventKit in v0.1 |

---

## 2. Stack with exact versions

### Rust

| Area | Crate | Version | Notes |
|---|---|---|---|
| Toolchain | Rust | **1.99.1**, edition 2024 | Pin in `rust-toolchain.toml` |
| App shell | `tauri` / `tauri-build` | **2.12.5** | Pin `~2.12`. No `updater` and no `http` plugin, ever. |
| | `tauri-plugin-global-shortcut` | 2.4.1 | Wraps `global-hotkey` 0.10.0 |
| | `tauri-plugin-autostart` | 2.7.0 | macOS: `MacosLauncher::LaunchAgent`, never the AppleScript launcher (it adds a System Events Automation prompt) |
| | `tauri-plugin-single-instance` | 2.5.5 | Forwards `panic` / `status` CLI arguments to the running instance |
| | `tauri-plugin-notification` | 2.8.1 | Summaries only. Never titles, URLs or paths. |
| | `tauri-plugin-dialog` | 2.8.4 | Importing list files |
| | `tauri-plugin-opener` | 2.7.3 | System Settings deep links, repo link |
| Core | `serde` 1.0.229, `toml` 1.2.1, `schemars` 1.2.3, `thiserror` 2.0.21, `regex` 1.13.1, `globset` 0.4.21, `url` 2.5.8, `psl` 2.1.255 (embedded Public Suffix List, no fetching), `unicode-normalization` (latest) | | Pure, no OS dependencies |
| Lists | `lz4_flex` 0.14.2 | | Compressed list blobs. Also decodes Firefox mozLz4. |
| Engine | `crossbeam-channel` 0.5.17, `tracing` 0.1.44, `tracing-subscriber` 0.3.23, `tracing-appender` 0.2.5, `dirs` 7.0.0, `sys-locale` 0.3.2, `sysinfo` 0.40.3 | | Threads and channels. Tokio (1.54.1) only arrives through Tauri. |
| macOS | `objc2` 0.6.5; `objc2-foundation`, `objc2-app-kit`, `objc2-application-services`, `objc2-core-foundation`, `objc2-core-graphics`, `objc2-io-kit`, `objc2-osa-kit`, `objc2-core-audio` **0.3.2**; `libc` | | Check for 0.4 first |
| Windows (v0.2) | `windows` **0.62.2**, `uiautomation` 0.25.1, `windows-registry` 0.100.0 | | `windows-registry`'s version number jumped; confirm it belongs to the same windows-rs family release |
| Linux (v0.3/v0.5) | `x11rb` 0.14.0, `zbus` 6.3.0, `atspi` 0.30.0, `ashpd` 0.14.0, `pipewire` 0.10.1, `wayland-client` 0.31.15, `wayland-protocols` 0.32.13, `wayland-protocols-wlr` 0.3.12 | | `atspi` probably pulls zbus 5 while `ashpd` uses zbus 6. Accept two zbus versions, or wait for an atspi release. |
| Native host (v0.4) | `interprocess` 2.4.8 | | Unix socket / named pipe, never TCP |
| ML (v0.6) | `tract-onnx` 0.23.10 | | CPU only |
| Bindings | `ts-rs` 12.0.1 | | Writes `app/src/bindings/*.ts` |
| Tests | `insta` 1.49.0, `proptest` 1.12.0 | | |
| Supply chain | `cargo-deny` 0.20.2, `cargo-auditable` 0.7.7 | | Run in CI |
| Later | `keyring` 4.2.0 | | Key for the encrypted journal |

### Frontend and tooling

| Package | Version | Notes |
|---|---|---|
| bun | 1.3.x | Package manager and scripts (`bun tauri dev`) |
| `@tauri-apps/cli`, `@tauri-apps/api` | 2.12.4 | |
| `vite` / `@vitejs/plugin-react` | 8.3.8 / 6.3.0 | |
| `react`, `react-dom` / `@types/react` | 19.3.1 / 19.3.0 | |
| `typescript` | 7.0.2 | The Go-based compiler, used for typechecking only. Pin 6.0.x if a tool in the chain breaks. |
| `tailwindcss`, `@tailwindcss/vite` | 4.3.5 | Compiles to static CSS, so the CSP needs no `unsafe-inline` |
| `i18next` / `react-i18next` | 27.1.3 / 18.0.2 | |
| `vitest` | 5.1.0 | |
| `@playwright/test` | 1.65.1 | UI tests against the web build with `@tauri-apps/api/mocks` |
| `@biomejs/biome` | 2.6.8 | Lint and format. `noRestrictedGlobals` bans `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`. |

No state library and no data-fetching library: Rust owns the state, and the UI subscribes to Tauri events.

---

## 3. Repository and crate layout

```
~/Developer/personal/bezartam/
├─ Cargo.toml                 # workspace; edition 2024; resolver 3
├─ rust-toolchain.toml        # 1.99.1
├─ deny.toml                  # licence allowlist + banned network crates
├─ clippy.toml                # disallowed-types: std::net::*, tokio::net::Tcp*/Udp*
├─ package.json, bun.lock, biome.json
├─ crates/
│  ├─ core/                   # bezartam-core: model, matcher, planner, config schema, extension protocol types
│  │                          #   no I/O, no threads, no OS deps; `now` is always passed in
│  ├─ lists/                  # bezartam-lists: category definitions + compressed domain blobs (build.rs)
│  ├─ engine/                 # bezartam-engine: inventory index, trigger fusion, scheduler, executor,
│  │                          #   journal, config load/save, capability report
│  ├─ platform-macos/         # v0.1
│  ├─ platform-windows/       # v0.2
│  ├─ platform-linux/         # v0.3 (X11 + logind/UPower/AT-SPI/PipeWire), v0.5 (Wayland backends)
│  ├─ platform-fake/          # scripted inventory + signals: engine tests, UI dev, demos, screenshots
│  └─ host/                   # v0.4: native-messaging host binary (stdio <-> local socket)
├─ app/
│  ├─ src-tauri/              # the only crate that depends on Tauri: tray, windows, commands,
│  │   ├─ capabilities/       #   hotkey, autostart, single instance, CLI args, Info.plist, entitlements
│  │   ├─ Info.plist          # LSUIElement, NSAppleEventsUsageDescription
│  │   ├─ hu.lproj/InfoPlist.strings, en.lproj/InfoPlist.strings
│  │   └─ Entitlements.plist  # com.apple.security.automation.apple-events
│  └─ src/                    # React UI: onboarding, rules, lists, activity, settings, capability report
│      ├─ bindings/           # generated by ts-rs (committed, checked in CI)
│      ├─ ipc.ts              # typed invoke/listen wrappers, the only place that talks to Rust
│      └─ i18n.ts
├─ locales/{en,hu}/{ui,native}.json   # one source for UI and native strings
├─ lists/                     # snapshots + manifest.toml (source URL, licence, date, sha256, entry count)
├─ extension/                 # v0.4: MV3 WebExtension (TypeScript), Chromium + Firefox
├─ integrations/              # v0.5: kwin/ script, gnome-shell/ extension
└─ docs/                      # architecture, threat model, test matrix (asked for by the brief)
```

**Dependency rules (CI checks them with `cargo tree`):**
- `core` depends on nothing OS-specific.
- `platform-*` depend only on `core`.
- `engine` depends on `core`, `lists` and exactly one `platform-*` (chosen by `cfg(target_os)`). Feature `fake` swaps in `platform-fake`.
- `app/src-tauri` depends on `engine`.
- `host` depends only on `core` (protocol types).

There is no separate CLI crate. The app binary accepts `panic`, `status`, `explain [--trigger X]` and `diag`. `single-instance` forwards them to the running app, and `explain` and `diag` also work stand-alone as dry runs.

**Bundle identifier:** a reverse-DNS ID you control, for example `hu.futozs.bezartam`. Fix it forever, because TCC grants are keyed to it.

---

## 4. Core domain model

### 4.1 Types (`crates/core`)

```rust
// ---------- Rules ----------
pub struct Rule {
    pub id: RuleId,
    pub name: String,                 // user text, or an i18n key for built-in presets
    pub enabled: bool,
    pub when: Vec<Trigger>,           // any of these arms the rule
    pub targets: TargetKinds,         // bitflags: TAB | DOCUMENT | APP_WINDOW | PROCESS
    pub matcher: Matcher,
    pub except: Option<Matcher>,
    pub action: Action,
    pub on_unsaved: UnsavedPolicy,    // default: Hide
    pub confirm: Confirm,             // default: Never
}

// ---------- Triggers ----------
pub enum Trigger {
    Panic,                                       // hotkey, tray, CLI
    ScreenShare { min_confidence: Confidence },  // default Medium
    DisplayMirrored,                             // projector, HDMI/AirPlay mirroring
    CallLikely,                                  // pre-flight: meeting app on mic/camera, share picker opened
    WillSleep,
    LidClosed,
    Locked,
    AwayLongerThan(Duration),                    // locked/asleep for N; fires before unlock when possible
    IdleLongerThan(Duration),
    ItemOpenLongerThan(Duration),                // evaluated per target
}
pub enum Confidence { Low, Medium, High }

// ---------- Matchers ----------
pub enum Matcher {
    Category(CategoryId),     // built-in or user list: adult, banking, webmail, medical,
                              // password_managers, messengers, invoices, meeting, ...
    Domain(String),           // host suffix: "otpbank.hu" matches "netbank.otpbank.hu"
    UrlGlob(String),          // "https://mail.google.com/mail/u/1/*"
    TitleContains(String),    // case- and diacritic-folded: "szamla" matches "Számla"
    App(AppSelector),         // bundle id | exe name/path | desktop id / WM_CLASS
    FileGlob(String),         // "~/Documents/Bank/**", "**/*bérjegyzék*.pdf"
    DocKind(DocKind),         // Pdf, Image, Word, Excel, PowerPoint, Pages, Keynote, Numbers, Text, Other
    Private,                  // incognito/private window, only where the platform can tell
    Any(Vec<Matcher>),
    All(Vec<Matcher>),
    Not(Box<Matcher>),
}

// A category is a composite: domains + apps + title keywords + file globs.
pub struct CategoryDef {
    pub id: CategoryId,
    pub domains: Vec<ListRef>,
    pub apps: Vec<AppSelector>,
    pub title_keywords: Vec<String>,
    pub file_globs: Vec<String>,
}

// ---------- Actions ----------
pub enum Action { Close, Hide, Minimize, Quit, NotifyOnly }      // Quit is never in a preset
pub enum UnsavedPolicy { Hide, SaveThenClose, Discard, SkipAndNotify }
pub enum Confirm { Never, Countdown(Duration) }                   // countdown: offers "Keep" / "Do it now"

// ---------- Targets ----------
pub struct Target {
    pub id: TargetId,                 // stable for the target's lifetime in our index
    pub provider: ProviderId,         // the adapter that found it also acts on it
    pub app: AppRef,                  // app id, display name, pid
    pub first_seen: Instant,          // in memory only
    pub kind: TargetKind,
}
pub enum TargetKind {
    BrowserTab(BrowserTab),
    DocumentWindow(DocumentWindow),
    AppWindow(AppWindow),
    Process(ProcessInfo),
}
pub struct BrowserTab {
    pub browser: BrowserFamily,       // Chromium(variant) | Safari | Firefox | Arc
    pub window: WindowHandle,
    pub tab: TabHandle,               // opaque: Chromium int id, Arc UUID, Safari (window id, URL), UIA RuntimeId
    pub title: String,
    pub url: Option<Url>,
    pub url_quality: UrlQuality,      // Exact | CachedWhenActive | Stale(Duration) | None
    pub private: Tri,                 // Yes | No | Unknown
    pub active: bool,
    pub audible: Tri,
}
pub struct DocumentWindow {
    pub window: WindowHandle,
    pub title: String,
    pub path: Option<PathBuf>,        // None for untitled documents
    pub kind: DocKind,
    pub dirty: Dirty,                 // Clean | Dirty | Unknown (Unknown is treated as Dirty)
}
pub struct AppWindow { pub window: WindowHandle, pub title: String, pub minimized: bool }
pub struct ProcessInfo { pub pid: u32, pub exe: PathBuf, pub app_id: AppId, pub started_at: SystemTime }
```

### 4.2 Planner (pure, the most-tested function in the repo)

```rust
pub fn plan(trigger: &FiredTrigger, snapshot: &[Target], rules: &[Rule], ctx: &PlanContext) -> Plan;

pub struct PlanContext<'a> {
    pub lists: &'a Lists,
    pub protected: &'a ProtectedSet,     // meeting category + capturing app (when known) + ourselves
    pub capabilities: &'a CapabilityTable,
    pub now: Instant,
}
pub struct Plan { pub steps: Vec<Step> }  // ordered by Phase
pub struct Step {
    pub target: TargetId,
    pub action: Action,                  // after resolution and fallback
    pub on_unsaved: UnsavedPolicy,
    pub phase: Phase,                    // Conceal → CloseTabs → CloseDocuments → Notify
    pub because: Vec<Reason>,            // rule id + matcher path ("banking via domain otpbank.hu")
}
```

Semantics, in order:

1. **Arm.** Only rules whose `when` contains the fired trigger are considered. `ItemOpenLongerThan` fires per target, and the planner scopes it to that target.
2. **Select.** A candidate must match the rule's `targets` kinds and its `matcher`, and must not match `except` or be protected. **Protected always wins.** It covers the meeting category (`meet.google.com`, `teams.microsoft.com`, `teams.live.com`, `*.zoom.us`, Discord/Zoom/Teams/FaceTime/Webex/OBS apps), the capturing process when the OS tells us (Windows ConsentStore, PipeWire), and our own windows.
3. **Merge.** When several rules hit one target:
   - The **strongest action** wins: Quit > Close > Hide > Minimize > NotifyOnly.
   - The **most data-preserving unsaved policy** wins: Hide > SkipAndNotify > SaveThenClose > Discard.
4. **Resolve.** Check the provider's capability for (target kind, action). If it isn't supported, fall back **only toward less destructive actions**: Close → Hide → Minimize → NotifyOnly. Never escalate.
5. **Order.**
   - Phase `Conceal`: instant app hides, plus our own UI.
   - Then tab closes, batched into one call per browser window.
   - Then document closes, which may involve save sheets.
   - Then notifications.
6. **Confirm.** `Confirm::Countdown` is only allowed for `ItemOpenLongerThan` and `IdleLongerThan`. Config validation rejects it on Panic, ScreenShare, DisplayMirrored, Locked, WillSleep, LidClosed and AwayLongerThan. There is no time for a prompt on those triggers, and a dialog would appear on the shared screen.

The executor **re-verifies every target right before acting**: the title, URL or path must still match. This handles a tab that navigated away, or a Safari tab whose index shifted. Each step has a deadline: 300 ms for conceal steps, 1 s per tab batch, 2 s per document with a sheet. Every step produces a `StepOutcome`: `Done`, `FellBack(Action)`, `Skipped(reason)` or `Failed(err)`.

### 4.3 Engine pipeline

```
SignalSources ──Signal{kind, confidence}──▶ TriggerFusion ──FiredTrigger──┐
Scheduler (1/min: ItemAge, Idle, Away timers) ─────────────▶ TriggerFusion │
                                                                          ▼
Providers ──InventoryEvent(Added/Changed/Removed)──▶ Inventory index ──▶ plan() ──▶ Executor ──▶ Journal ──▶ UI / notification
```

- **The index stays warm.** It is driven by AXObserver, WinEvent/UIA, EWMH PropertyNotify and AT-SPI events, plus a full refresh every 30 s. A trigger never waits for a scan.
- **Share-guard mode.** While a share or mirroring is active, every `Added` or `Changed` inventory event is re-planned against the rules armed by `ScreenShare`/`DisplayMirrored`, and acted on immediately. For example, a tab that navigates to a bank mid-share gets closed. On macOS this hangs off `kAXTitleChangedNotification` on browser windows, which then re-queries that window's active tab. The page is visible between its load and our close (about 0.3–1 s, *verify*), and the docs say so.
- **Debounce.** The same trigger won't fire again within 10 s, except Panic, which always fires. Share stop is debounced by 3 s.
- **Away semantics.**
  - On `Locked` or `WillSleep`, record `t0` and start a timer.
  - If the timer reaches N while the machine is still locked, fire.
  - If the machine was asleep, then on wake, *while it is still locked*, compute `now − t0` and fire before the user unlocks.
  - If the OS doesn't require a password on wake, fire at wake and tell the user to turn that requirement on.

### 4.4 Config example (TOML, recommended preset)

```toml
version = 1
language = "system"                 # "system" | "en" | "hu"
panic_hotkey = "Ctrl+Alt+Shift+B"   # ⌃⌥⇧B on macOS

[protect]
categories = ["meeting"]

[[rule]]
id = "share-close-sensitive-tabs"
when = ["panic", "screen_share", "display_mirrored"]
targets = ["tab"]
match = { any = [{ category = "adult" }, { category = "banking" }, { category = "webmail" }, { category = "medical" }] }
action = "close"

[[rule]]
id = "share-hide-private-apps"
when = ["panic", "screen_share", "display_mirrored"]
targets = ["app_window", "process"]
match = { any = [{ category = "password_managers" }, { category = "messengers" }] }
action = "hide"

[[rule]]
id = "share-close-invoices"
when = ["panic", "screen_share", "display_mirrored"]
targets = ["document"]
match = { category = "invoices" }
action = "close"
on_unsaved = "hide"

[[rule]]
id = "lid-close-adult"
when = ["lid_closed", "will_sleep"]
targets = ["tab"]
match = { category = "adult" }
action = "close"

[[rule]]
id = "away-close-banking"
when = [{ away_longer_than = "15m" }]
targets = ["tab"]
match = { category = "banking" }
action = "close"

[[rule]]
id = "bank-tab-open-too-long"
when = [{ item_open_longer_than = "2h" }]
targets = ["tab"]
match = { category = "banking" }
action = "close"
confirm = { countdown = "60s" }
```

### 4.5 Built-in categories and lists

Lists are snapshotted by a CI job into `lists/`, with source, licence, date, SHA-256 and entry count in `manifest.toml`. Each refresh is a reviewed PR with a diff summary. The app **never downloads lists**. Updates arrive only with a release or a user-imported file (hosts or plain-domain format).

| Category | Source | Licence | Size | Default |
|---|---|---|---|---|
| adult | StevenBlack `alternates/porn-only` + HaGeZi NSFW, merged and deduplicated | MIT + CC BY 4.0 (attribution) / GPL-3.0 | 76,811 + 83,101 before dedup; lz4 blob about 1–2 MB (*measure*) | On |
| banking | UT1 `bank` + curated Hungarian seed (OTP, K&H, Erste, MBH, Raiffeisen, UniCredit, CIB, Gránit, Revolut, Wise; domains checked by hand) + title keywords ("Internetbank", "NetBank") | CC BY-SA 4.0 / ours | 6,646 + seed | On |
| webmail | UT1 `webmail` + seed (freemail.hu, citromail.hu, Gmail, Outlook, Proton) | CC BY-SA 4.0 / ours | 413 + seed | On |
| medical | Hand-curated seed (for example `eeszt.gov.hu`) + title keywords. No open, licensed health list exists. | ours | small | On |
| financial, dating, gambling | UT1 | CC BY-SA 4.0 | 473 / 10,564 / 47,823 | Off |
| password_managers, messengers | App selectors, starting from Tuple App Veil's defaults (Passwords, Keychain Access, 1Password, Bitwarden, Messages, Signal, WhatsApp, Telegram, Slack) | ours | about 30 apps | On |
| invoices | File globs and title keywords (számla, invoice, bérjegyzék, payslip, bankszámlakivonat, adóbevallás) | ours | small | On |
| meeting (protected) | Meeting domains and apps | ours | small | Always |

Excluded lists:
- **Bon-Appetit:** 6.6 MB, ShareAlike, no upstream attribution.
- **UT1 adult:** 4.6M entries.
- **OISD:** no plain format since 2024, and redundant with HaGeZi.
- **Block List Project:** archived 2026-10-27.

Matching:
- Hostnames are lowercased and punycoded.
- Walk up the labels to the registrable domain (`psl`) and check each against a `HashSet<Box<str>>`, built lazily the first time a list is used.
- Titles go through NFKD, combining marks are stripped, then case-folded.

---

## 5. Platform adapter traits and threading

```rust
/// Finds targets of the kinds it understands and acts on them. The adapter that
/// found a target is the one that acts on it, so opaque handles never cross adapters.
pub trait Provider: Send {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> Vec<Capability>;  // (TargetKind, Action) -> Full | Partial(note) | Unsupported
    fn start(&mut self, events: Sender<InventoryEvent>) -> Result<(), ProviderError>;
    fn snapshot(&mut self, deadline: Instant) -> Result<Vec<Target>, ProviderError>;
    fn reverify(&mut self, target: &Target, deadline: Instant) -> Reverify; // Same | Changed(Target) | Gone
    fn execute(&mut self, step: &Step, target: &Target, deadline: Instant) -> StepOutcome;
}

pub trait SignalSource: Send {
    fn id(&self) -> SignalSourceId;
    fn health(&self) -> SourceHealth;           // Ok | Unavailable(reason) | Noisy(reason)
    fn start(&mut self, tx: Sender<Signal>) -> Result<(), SignalError>;
}
pub struct Signal { pub kind: SignalKind, pub confidence: Confidence, pub at: Instant, pub detail: SignalDetail }
pub enum SignalKind {
    CaptureStarted, CaptureStopped, SharePickerOpened, DisplayMirrored, DisplayUnmirrored,
    CallLikely, CallEnded, WillSleep, DidWake, Locked, Unlocked, LidClosed, LidOpened, Panic,
}
// SignalDetail can carry the capturing process (Windows/Linux), which feeds ProtectedSet.

pub trait PermissionBroker: Send {
    fn status(&self) -> Vec<PermissionState>;   // Accessibility, Automation(per bundle id), Notifications, ...
    fn request(&self, p: &Permission) -> RequestOutcome;   // may open System Settings
}

pub trait Platform {
    fn providers(&self) -> Vec<Box<dyn Provider>>;
    fn signal_sources(&self) -> Vec<Box<dyn SignalSource>>;
    fn permissions(&self) -> Box<dyn PermissionBroker>;
}
```

All trait calls block with explicit deadlines. Each provider runs as an actor on its own OS thread, and the engine talks to it over channels. This fits the OS rules: AX and Apple Events must stay off the main thread, UIA wants MTA, Office COM wants an STA with a message pump.

### Threads per OS

| OS | Threads |
|---|---|
| macOS | **main:** Tauri/NSApplication, NSWorkspace observers (launch, terminate, sleep, wake), distributed notifications (`com.apple.screenIsLocked`/`Unlocked`), `CGDisplayRegisterReconfigurationCallback`, hotkey. **ax:** own CFRunLoop with AXObservers for regular apps and all AX calls; `AXUIElementSetMessagingTimeout` 0.5 s. **osa:** in-process OSAKit JXA/AppleScript, every script wrapped in a 2 s `with timeout`, and never sent to an app that isn't running (that would launch it). **iokit:** clamshell interest notification (`0xE0034100`) and `IORegisterForSystemPower`, acknowledged with `IOAllowPowerChange` after cleanup or at 20 s at most. **watcher:** SkyLight poll and CoreAudio/CoreMediaIO "running somewhere" listeners. **engine:** fusion, planner, executor dispatch. |
| Windows | **ui:** hidden message window for `WM_POWERBROADCAST`, `WM_WTSSESSION_CHANGE`, `WM_DISPLAYCHANGE`, plus power-setting registrations. **winevent:** `SetWinEventHook(WINEVENT_OUTOFCONTEXT)` with its own message loop. **uia:** MTA, `CacheRequest`, `ConnectionTimeout`/`TransactionTimeout` set. **com:** STA with a message pump for Office automation. **registry:** `RegNotifyChangeKeyValue` on ConsentStore. |
| Linux | **dbus:** zbus executor (logind, UPower, ScreenSaver, Mutter, KWin callbacks, our GNOME extension's interface). **pipewire:** main loop with registry listener. **x11:** x11rb event loop. **atspi:** event stream. |

---

## 6. Per-OS capability matrix

Full = works for the common case. Partial = works with stated limits. Impossible = no acceptable mechanism. "(v0.5)" marks a Wayland backend that is not in v0.3.

| Capability | macOS 14+ | Windows 10 2004+ / 11 | Linux X11 | KDE Plasma 6 Wayland | GNOME 48+ Wayland | wlroots / Hyprland / niri |
|---|---|---|---|---|---|---|
| List apps and processes | Full | Full | Full | Full | Full | Full |
| List windows with titles | Full (AX; some apps hide off-Space windows, which CGWindowList still reveals without titles) | Full | Full (EWMH) | Full (KWin script, v0.5) | Partial (our Shell extension, v0.5; AT-SPI frame names as fallback) | Full (wlr-foreign-toplevel, v0.5) |
| File behind a document window | Partial (AXDocument, AppleScript for scriptable apps) | Partial (Office COM exact; title or command line otherwise) | Partial (`/proc` cmdline and fds, LibreOffice lock files) | Partial | Partial | Partial |
| Unsaved-state detection | Partial (AppleScript `modified`/`saved`; `AXEdited` on the close button) | Partial (Office COM `Saved`; `*`/`●` title markers) | Impossible in general (title markers only) | same | same | same |
| Close a document under the save policy | Full for scriptable apps; Partial via AX sheet | Full for Office; Partial via UIA dialogs | Partial (close + AT-SPI dialog) | Partial | Partial (extension) | Partial |
| Conceal an app or window | Full (instant app hide; per-window minimize) | Full (minimize) | Full (iconify) | Full (v0.5) | Partial (extension, v0.5) | Full (v0.5; COSMIC has its own protocol) |
| Tab titles, all tabs | Full for Chromium and Safari; Partial for Firefox (session file, AX tab strip) | Full (UIA; Chromium 138+, Firefox 143+) | Partial (AT-SPI after enabling a11y and restarting the browser) | Partial | Partial | Partial |
| URL of every tab, no extension | Full for Chromium and Safari; Partial for Firefox (about 15 s stale, no private windows) | Partial (active tab only, cached when activated) | Partial (same) | Partial | Partial | Partial |
| Close one tab, no extension | Full for Chromium, Arc and Safari; Impossible for Firefox without a visible ⌘W | Partial (UIA Invoke on the tab's close button, depends on layout) | Partial (AT-SPI action) | Partial | Partial | Partial |
| Tabs with the v0.4 extension | Full (Chromium, Firefox; Safari later) | Full | Full | Full | Full | Full |
| Private/incognito detection | Full for Chromium (`mode`); Impossible for Safari (no flag); Firefox private windows not visible | Partial (title suffix) | Partial | Partial | Partial | Partial |
| Screen share / recording detection | Partial, high confidence (private SkyLight API) | Partial, medium-high (undocumented ConsentStore; Windows Graphics Capture only) | **Impossible** (heuristics only) | Partial, medium-high (PipeWire) | Partial, medium-high (PipeWire) | Partial, medium-high (PipeWire) |
| Projector / mirroring | Full | Full (display topology) | Full (RandR) | Full | Full | Full |
| Lock / unlock | Full (undocumented notification names) | Full | Partial (depends on the locker) | Full | Full | Partial |
| Act before sleep | Full (up to 30 s) | Partial (Modern Standby gaps; also watch display-off) | Full (logind delay inhibitor) | Full | Full | Full |
| Lid | Full | Full | Full (UPower) | Full | Full | Full |
| Idle time | Full | Full | Full | Full (ext-idle-notify) | Full (Mutter IdleMonitor) | Full (ext-idle-notify) |
| Global panic hotkey | Full (no Input Monitoring) | Full | Full | Full (portal) | Partial (portal; user approves) | Partial (Hyprland portal; Sway: bind the CLI) |
| Keep another app's window visible to you but hidden from capture | Impossible (no public API; ignored on 15.4+) | Impossible without DLL injection (we refuse) | Impossible | Plasma 6.6 built-in, manual | Impossible | Impossible |
| Sandboxed store distribution | Impossible (Accessibility not supported in the App Sandbox) | Possible (MSIX full trust) | Flatpak: poor fit (filtered D-Bus, no host processes, no Wayland control) | | | |

The engine turns each provider's `capabilities()` into a **runtime capability report** shown in Settings → "What I can see on this computer", for example "Firefox: titles only, no per-tab close" or "Screen-share detection: unavailable on X11, use the hotkey".

---

## 7. Screen-share detection per OS

**Confidence levels:**
- **High:** the OS capture path itself sets the signal, it has been observed on the major meeting apps, and false positives are rare and known.
- **Medium:** catches most real shares but misses some capture paths, or has known false positives.
- **Low:** a heuristic. Used only to start a pre-flight or to corroborate another signal.

Rules default to `min_confidence = Medium`. Low signals only ever start the pre-flight check, which runs the rules armed by `CallLikely`, and by default those act with a notification ("3 sensitive items are open: clean up now?"). When the share actually starts, the `ScreenShare` rules act without asking.

### macOS

| Signal | Mechanism | Catches | Latency | Permission | Confidence | Role |
|---|---|---|---|---|---|---|
| M1 | `SLSIsScreenWatcherPresent()` from SkyLight via `dlopen`/`dlsym`, polled at **4 Hz while a meeting app runs or a call is likely, 1 Hz otherwise** | Any capture in the session: Zoom confirmed on 26.6.1, `screencapture -V` confirmed; AirPlay/Teams/Meet/Discord/OBS *(verify)* | ≤ 250 ms in meeting mode | none | **High** (empirical, private API) | Primary |
| M2 | `SLSRegisterNotifyProc` events 1502/1503 | Capture start/stop, pushed | Immediate | none | Medium (signature unverified) | Experimental flag; speeds up M1 |
| M3 | `CGDisplayRegisterReconfigurationCallback`, `CGDisplayIsInMirrorSet`, new non-built-in display | Projector, HDMI, AirPlay mirroring | Immediate | none | **High** for mirroring; Medium for "external display connected" (could be a desk monitor; the user chooses) | Primary for projectors |
| M4 | CoreAudio `kAudioDevicePropertyDeviceIsRunningSomewhere`, CoreMediaIO equivalent, and per-process input attribution through process objects (macOS 14.x, *verify the minimum minor version*) | A call is in progress | Immediate | none | Medium | `CallLikely` pre-flight |
| M5 | Meeting app launched (NSWorkspace) or meeting-domain tab active | A meeting is about to start | Immediate | none | Low | Pre-flight, raises M1 poll rate |

Not used:
- ScreenCaptureKit / Screen Recording (permission and self-trigger).
- AX scraping of localized "You are sharing" toolbars (fragile).
- Calendar (deferred).

**Fail open.** If the M1 symbol is missing, the report says "Screen-share detection unavailable on this macOS version". M3–M5, the hotkey and lock-time cleanup keep working.

**Noise calibration.** At onboarding and at each launch, if M1 reads `true` while no share is known, mark it `Noisy`. Causes include DisplayLink docks, always-on recorders and OBS with a display source loaded. In that state M1 is demoted to Low and the user is told why.

### Windows

| Signal | Mechanism | Catches | Confidence | Role |
|---|---|---|---|---|
| W1 | `RegNotifyChangeKeyValue` on `HKCU\…\CapabilityAccessManager\ConsentStore\graphicsCaptureProgrammatic` and `graphicsCaptureWithoutBorder` (NonPackaged and packaged subkeys); `LastUsedTimeStop == 0` means active | Windows Graphics Capture users: Chrome, Edge, Teams, Zoom, Discord, Slack, OBS-WGC. **The Chromium share picker creates a record before the user picks**, which gives us an early warning. The subkey name is the capturing exe, which feeds ProtectedSet. | Medium-High (undocumented; unverified on Windows 10) | Primary + `SharePickerOpened` |
| W2 | `WM_DISPLAYCHANGE` → `QueryDisplayConfig` topology `CLONE`/`EXTEND` | Projector, duplicated display | High | Primary for projectors |
| W3 | ConsentStore `microphone` and `webcam` with `LastUsedTimeStop == 0` for a meeting exe | A call is in progress | Medium | `CallLikely` |
| W4 | Meeting process launched; share-toolbar windows | App-specific | Low | Corroborating only |

Blind spots, stated in the UI:
- DXGI Desktop Duplication, GDI capture, capture cards.
- Sharing a single Chromium tab (no record is created).

Excluded:
- ETW (needs admin, noisy).
- obs-websocket and the retired Teams local API (network).

### Linux

| Signal | Mechanism | Catches | Confidence | Role |
|---|---|---|---|---|
| L1 | `pipewire` registry: `media.class = Video/Source` node from the compositor or portal (`gnome-shell`/`meta-screen-cast-src`, `kwin_wayland`, `xdg-desktop-portal-wlr`/`-hyprland`, COSMIC), **linked to a consumer** stream; exclude v4l2/libcamera devices | Every Wayland capture, including remote desktop. The consumer's `application.process.binary` feeds ProtectedSet. | Medium-High *(node names per DE version: verify)* | Primary on Wayland |
| L2 | RandR `RRScreenChangeNotify` / `wl_output` / Mutter `DisplayConfig.MonitorsChanged` | Projector, mirroring | High | Primary for projectors |
| L3 | PipeWire `Stream/Input/Audio` from a meeting binary | A call is in progress | Medium | `CallLikely` |
| L4 | X11 session | **Nothing.** Any X client can read the screen silently. | – | Hotkey + pre-flight (L3) + projector (L2). The UI says so. |

### Latency target

Concealing already-indexed apps should finish within **300 ms of capture start** on macOS in meeting mode, and tabs within **700 ms**. These are targets to measure in the v0.1 test matrix, not promises. The first frames can still go out, which is why the pre-flight sweep, lock-time cleanup and window-sharing advice are part of the product, not extras.

Our own windows:
- Hidden when any share signal fires.
- Set `contentProtected` (effective on Windows; ignored by ScreenCaptureKit on macOS 15.4+).
- Notifications never include titles, URLs or paths.

---

## 8. Other triggers per OS

| Trigger | macOS | Windows | Linux |
|---|---|---|---|
| WillSleep | `NSWorkspaceWillSleepNotification` + `IORegisterForSystemPower` (delay ack, up to 30 s) | `PowerRegisterSuspendResumeNotification`; on Modern Standby also `GUID_CONSOLE_DISPLAY_STATE` off | logind `PrepareForSleep(true)` + `Inhibit("sleep", …, "delay")` |
| LidClosed | `IOPMrootDomain` `AppleClamshellState`, message `0xE0034100` | `GUID_LIDSWITCH_STATE_CHANGE` | UPower `LidIsClosed` |
| Locked / Unlocked | `com.apple.screenIsLocked` / `…Unlocked`; state via `CGSessionCopyCurrentDictionary` | `WTSRegisterSessionNotification` | logind `LockedHint`, `org.{gnome,freedesktop}.ScreenSaver.ActiveChanged` |
| Idle | `CGEventSource.secondsSinceLastEventType` | `GetLastInputInfo` | Mutter IdleMonitor / ext-idle-notify v2 / XScreenSaver |
| Panic | tauri-plugin-global-shortcut, tray, CLI | same | X11: plugin; Wayland: GlobalShortcuts portal; CLI everywhere |

None of these needs a TCC permission on macOS. **Whether AX and Apple Events keep working while the screen is locked is unverified.** It is the first spike in v0.1, because the "clean before unlock" design depends on it. If they don't work, the away rule runs at will-sleep or lock time instead.

---

## 9. Action semantics per OS

| Abstract action | macOS | Windows | Linux X11 | Wayland (v0.5) |
|---|---|---|---|---|
| Close tab | JXA `close` by tab id (Chromium, Arc via `byId`); Safari: re-resolve by URL immediately before `close`. Extension: `tabs.remove`. | UIA `Invoke` on the tab's close button (found by structure, not by localized name); extension | AT-SPI action on the close button; extension | same as X11 |
| Hide tab | Extension: move to a minimized stash window. Without it: minimize the tab's window if it holds no protected tab; otherwise, if the tab is active, activate a non-sensitive neighbour; otherwise notify. | same | same | same |
| Close document | AppleScript `close … saving yes/no` where a dictionary exists; otherwise AX close button + sheet handling (§10) | Office COM; otherwise `PostMessage(WM_CLOSE)` + UIA dialog handling | `_NET_CLOSE_WINDOW` + AT-SPI dialog | KWin `closeWindow()` / wlr `close()` / GNOME extension |
| Hide app | `NSRunningApplication.hide()`, only if **every** visible window of that app is a match. Otherwise act per window. | `ShowWindowAsync(SW_MINIMIZE)` on every top-level window of the process (never `SW_HIDE`) | Iconify every window of the PID | Minimize through the backend |
| Minimize window | `AXMinimized = true` (genie animation, live Dock thumbnail: last resort) | `ShowWindowAsync(SW_MINIMIZE)` | `WM_CHANGE_STATE` Iconic | backend |
| Quit app (opt-in only) | Close documents under the policy first, then `terminate()`. Never `forceTerminate()`. Warn that state restoration may reopen the document. | `WM_CLOSE` to every top-level window. Never `TerminateProcess`. | close all windows | same |

**Global rules:**
- Skip elevated windows on Windows (UIPI drops our messages silently) and tell the user.
- Never act on a target that fails re-verification.
- Never touch protected targets.

---

## 10. Unsaved-document policy

**Dirty detection** gives a tri-state: `Clean`, `Dirty` or `Unknown`. **`Unknown` is treated as `Dirty`.**

- **macOS:** in order of trust:
  1. AppleScript `modified` (Preview, TextEdit, Pages/Keynote/Numbers) or `saved` (Word, Excel, PowerPoint).
  2. `AXEdited` read from the window's **`AXCloseButton`**. The window itself doesn't support the attribute.
  3. Unknown.

  Only `true` is trusted from AX, because autosave apps can hold edits while the flag is clear.
- **Windows:** Office COM `Saved` is exact. Otherwise `*` or `●` in the title means Dirty, and anything else is Unknown.
- **Linux:** title markers, otherwise Unknown.
- **Browser tabs** count as Clean. Unsaved form input is unknowable. If a `beforeunload` dialog appears, it is handled like an unexpected sheet.

**Policy table** (applies only when the action is Close or Quit):

| `on_unsaved` | Clean | Dirty or Unknown, file-backed | Dirty, untitled (never saved) |
|---|---|---|---|
| **Hide** (default) | Close | Don't close. Conceal (app hide, or minimize the window) and notify "1 document with unsaved changes was hidden". | Same |
| **SaveThenClose** | Close | Save through the app's API, then close: AppleScript `close … saving yes`, Word `Close(wdSaveChanges)`, Excel `Close(SaveChanges=True)`, PowerPoint **`Save()` then `Close()`** (PowerPoint's `Close()` discards silently). If a sheet appears, press its default button. | Not supported: fall back to Hide. We never drive a Save panel. |
| **Discard** (explicit opt-in) | Close | Close without saving: `saving no`, `wdDoNotSaveChanges`, PowerPoint `Saved = True` then `Close()`, or the sheet's "Don't Save" button | Needs a second opt-in ("also discard documents that were never saved"); otherwise Hide |
| **SkipAndNotify** | Close | Leave it open and notify with a count | Same |

**Unexpected sheet or dialog** (we believed the document was Clean, or the state was Unknown):
- Wait up to 1.5 s for `kAXSheetCreatedNotification` (macOS) or a modal window (Windows UIA `WindowOpened` with `IsModal`), then apply the policy.
- **Pick buttons by role, never by localized title**, because the Hungarian UI is a first-class target:
  - macOS: `kAXDefaultButtonAttribute` = Save, `kAXCancelButtonAttribute` = Cancel, the one remaining `AXButton` = Don't Save or Delete.
  - Windows `#32770`/TaskDialog: AutomationId `6` = Yes/Save, `7` = No/Don't Save, `2` = Cancel.
- If the buttons can't be identified, press Cancel (or Escape), then conceal and notify.

**Per-app notes:**
- **Windows Word:** documents in Protected View live in `ProtectedViewWindows`, not `Documents`. They are read-only, so close them directly.
- **Windows Acrobat Reader:** has no OLE, and DDE `DocClose` discards without asking. Default to minimize. Close only through `WM_CLOSE` plus dialog handling, and use DDE only when the policy is Discard.
- **macOS autosave apps** (Pages, Keynote, Numbers, TextEdit, Preview): closing a file-backed document saves it in place. Whether `saving no` really discards in these apps is *(verify per app)*. Until it is verified, Discard in these apps falls back to Hide.
- **Restore never reopens a private or incognito tab into a normal window**, which would write it to history. Tabs with `private = Unknown` (Safari) aren't offered for restore. We point to Safari's own "Reopen Last Closed Tab".

---

## 11. Permissions onboarding

### macOS (v0.1)

1. **Welcome.** One screen: what the app does, "This app never connects to the internet", a link to the source, the language choice (Hungarian preselected when the system locale is `hu`), and the preset (Recommended or Start empty).
2. **Accessibility.**
   - Explain why: to read window titles and open documents and to close them.
   - The button calls `AXIsProcessTrustedWithOptions(prompt: true)` and opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`.
   - Poll once a second until granted. There is no callback.
3. **Automation, one consent per app.**
   - List installed browsers and document apps (found by bundle id through LaunchServices) that the active rules touch.
   - For each running one, "Allow" calls `AEDeterminePermissionToAutomateTarget(…, askUserIfNeeded: true)` on the OSA thread.
   - For apps that aren't running, show "Will ask the next time you open Chrome". On the next launch (`didLaunchApplication`), ask then, **but only if no share or projector is active**.
   - While a share is active, an ungranted app falls back to app-level Hide. We never prompt mid-share.
4. **Notifications** (for run summaries).
5. **Recommended OS settings.** Checklist with deep links; we never change OS settings ourselves:
   - Turn off notifications when mirroring or sharing.
   - Require a password immediately after sleep. Without it, wake shows the desktop at once.
   - Share a window instead of the whole screen in Zoom, Teams and Meet.
6. **Dry run.** "This is what I would do right now", using the planner on the live inventory. It builds trust and catches over-broad rules. "Simulate a screen share" injects a synthetic signal.
7. **Login item** through LaunchAgent (tauri-plugin-autostart), on by default and switchable.

After onboarding, a **Permission health** panel re-checks at every launch. A revoked grant shows a warning badge on the tray icon and a clear capability downgrade.

**Development pitfalls:**
- Sign dev builds with a **stable** identity (Apple Development certificate or a self-signed one). Ad-hoc signatures change every build and silently break TCC grants.
- Test permissions with bundled `.app` builds. `bun tauri dev` attributes TCC to Terminal or the IDE.
- Reset with `tccutil reset Accessibility|AppleEvents <bundle-id>`.

**Never requested:** Screen Recording, Input Monitoring, Full Disk Access, Calendars (until the later calendar feature).

### Windows (v0.2)

There are no consent prompts. Onboarding covers:
- What elevated windows mean (they get skipped).
- That Office documents become reachable once Office has been focused once (ROT registration, KB238610).
- The built-in "Turn on do not disturb automatically when duplicating your display" toggle.

### Linux (v0.3 / v0.5)

- **Accessibility for tabs:** show the exact command (`gsettings set org.gnome.desktop.interface toolkit-accessibility true`) with a "Run it for me" button that needs explicit consent, then "restart your browser".
- **Hotkey:** the GlobalShortcuts portal approval dialog on Wayland. Otherwise offer to bind `bezartam panic` in the desktop's own shortcut settings.
- **v0.5:** install the KWin script (loaded over D-Bus, no root) or the GNOME Shell extension (from extensions.gnome.org).

---

## 12. The no-network guarantee

**Statement:** the Bezártam? process and its helper binaries never open an AF_INET or AF_INET6 socket. Inter-process communication uses only stdio, Unix domain sockets and Windows named pipes with per-user ACLs.

Enforcement layers, all in CI:

| Layer | Mechanism |
|---|---|
| Dependencies | `cargo-deny` bans `reqwest`, `hyper`, `h2`, `ureq`, `isahc`, `curl`, `surf`, `attohttpc`, `tungstenite`, `quinn`, `rustls`, `native-tls`, `openssl`. A licence allowlist is also enforced. |
| Our Rust code | Clippy `disallowed-types` for `std::net::{TcpStream, TcpListener, UdpSocket}` and `tokio::net::{TcpStream, TcpListener, UdpSocket}`; `#![deny(clippy::disallowed_types)]` in every crate |
| Frontend code | Biome `noRestrictedGlobals`: `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`. Plus a grep for `navigator.sendBeacon`. |
| Webview | CSP `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost`. `withGlobalTauri: false`, `freezePrototype: true`, no remote URLs or remote fonts, devtools off in release. Tauri capabilities list only our commands and the named plugin permissions. The opener is scoped to `x-apple.systempreferences:*`, `ms-settings:*` and the repo URL (opening a link hands it to the user's browser; our process doesn't fetch it). |
| Tauri plugins | No `updater`, `http`, `websocket` or `upload` plugin. The plugin list is reviewed in PRs. |
| Runtime test | Linux CI runs the release binary with `platform-fake` scripted scenarios under `strace -f -e trace=socket,connect,sendto` and fails on any `AF_INET`/`AF_INET6`. A second run inside `unshare -rn` (no network namespace) must give identical results. macOS CI runs the same scenarios under a `sandbox-exec` profile with `(deny network*)` *(the tool is deprecated but still functions; verify on the runner)*. |
| Release integrity | `cargo auditable build` embeds the dependency list. GitHub artifact attestations (`actions/attest-build-provenance`). SHA256SUMS. Reproducible builds as an aim, not a claim. |
| Data refresh | Lists and models are bundled. Updates come only through a release or a user-imported file. No update checks, telemetry, crash reporter or accounts. Crash dumps stay local, and users can attach them to an issue by hand. |

**Honest caveats in the privacy doc:**
- The OS webview runtime (WKWebView, WebView2, WebKitGTK) is a system component that updates itself. We only load bundled assets.
- On Windows, keep Tauri's default WebView2 arguments, which already disable `msSmartScreenProtection` (*check `additionalBrowserArgs` in the Tauri config docs*). Anyone overriding `additionalBrowserArgs` has to keep that flag.
- The browser extension (v0.4) runs inside a browser that obviously uses the network. The extension itself requests no host permissions except content scripts on meeting domains (for the `getDisplayMedia` hook), and it never calls `fetch`.

### Data at rest

| Data | Where | Sensitive? | Retention |
|---|---|---|---|
| `config.toml` | app config dir | User rules (may name their bank) | Until the user deletes it |
| Imported lists | app data dir | Domain lists | Until removed |
| Restore journal (titles, URLs, paths of closed items) | **Memory only** | Yes | Until restored, quit, or 2 h (configurable) |
| Hidden-apps marker | app data dir | Bundle ids only | Cleared on restore. Lets a crash recovery unhide. |
| Logs | app log dir | **No titles, URLs or paths** unless the user enables diagnostic mode, which warns and turns itself off after 24 h | 7-day rotation |
| `first_seen` timestamps | Memory | No | Process lifetime. Item age restarts when the app restarts (documented). |

Later: an encrypted persisted journal (key in the OS keychain via `keyring`, viewing gated by Touch ID or Windows Hello).

---

## 13. Internationalization

- **One source of truth:** `locales/{en,hu}/ui.json` (React through i18next) and `locales/{en,hu}/native.json` (tray menu and OS notifications). `build.rs` in `app/src-tauri` embeds `native.json` as a small lookup table.
- **Rust never builds user-facing prose.** Engine events carry message keys and parameters. The UI renders them.
- **Plurals:** `Intl.PluralRules` through i18next (Hungarian has `one`/`other`; nouns after numerals stay singular, so write "3 lap" and never "3 lapok"). Dates and times through `Intl.DateTimeFormat`.
- **Locale detection:** the OS locale through `sys-locale` on the Rust side and `navigator.language` in the webview. A manual override lives in config.
- **macOS permission strings** are localized in `hu.lproj/InfoPlist.strings` and `en.lproj/InfoPlist.strings` (`NSAppleEventsUsageDescription`).
- **Hungarian copy** follows Hungarian typography („…” quotes, proper ő/ű) and is written natively, not translated.
- **CI fails** on missing keys in `hu` or `en`, and on unused keys.
- **Later:** community languages through hosted Weblate (libre plan for open-source projects). It is a translator tool and never part of the app.

---

## 14. Distribution and signing

| OS | Channel | Signing | Notes |
|---|---|---|---|
| macOS | DMG on GitHub Releases + own Homebrew tap (`brew install --cask futozs/tap/bezartam`); the official cask later | **Developer ID + notarization ($99/year Apple Developer Program, budget for it)**, hardened runtime, `com.apple.security.automation.apple-events` | Universal binary (Intel Macs top out at macOS 26). `minimumSystemVersion: "14.0"`. No Mac App Store build (AX is unsupported in the sandbox). Homebrew 5 disables casks that fail Gatekeeper. |
| Windows | **Microsoft Store MSIX** (free account, Microsoft re-signs, no SmartScreen warning) + NSIS installer signed through **SignPath Foundation** + winget manifest | Store / SignPath | Confirm Store policy for an app that closes other apps' windows before submitting. Submit builds to Microsoft's malware analysis portal to reduce Defender false positives. |
| Linux | `.deb`, `.rpm`, AppImage (Tauri bundler) + AUR | GPG-signed checksums | No Flatpak in v0.x (filtered D-Bus, no view of host processes, no Wayland window control). GNOME Shell extension on extensions.gnome.org (v0.5). |

There is no in-app updater. The app shows its version, and the user checks for updates. A later option is an opt-in "check GitHub for updates" button that opens the releases page in the user's browser. Our process still doesn't fetch it.

---

## 15. Testing strategy

| Layer | How |
|---|---|
| `core` (planner, matcher, merge, fallback, config validation) | Unit tests; `proptest` invariants ("fallbacks never get more destructive", "protected targets never appear in a plan", "Discard never appears without opt-in"); `insta` snapshots of plans for the recommended preset against scripted inventories |
| `lists` | Licence-manifest test; sample-domain hit and miss tests; size budget |
| `engine` | `platform-fake` scenarios: share starts mid-navigation, unsaved document hits each policy, lock then sleep then wake-while-locked, noisy watcher, provider timeout |
| Platform adapters | Manual test matrix per release (below), plus `bezartam diag`, which prints a **redacted** capability and permission report users can paste into issues |
| UI | Vitest for components and i18n; Playwright against the web build with `@tauri-apps/api/mocks`, at mobile and desktop widths; screenshots for both languages |
| No-network | §12 runtime test on every PR |

**v0.1 device test matrix (macOS 14, 15, 26, 27):**
- **Share sources:** Zoom, Teams, Meet in Chrome, Meet in Safari, Discord, OBS, QuickTime, built-in screen recording, macOS Screen Sharing, AirPlay mirroring, HDMI projector, Sidecar, a DisplayLink dock (false positive).
- **Browsers:** Chrome, Brave, Edge, Vivaldi, Arc, Safari (are private windows enumerable?), Firefox.
- **Documents:** Preview, TextEdit, Pages, Keynote, Numbers, Word, Excel, PowerPoint, Acrobat Reader.
- **Each in English and Hungarian system language**, for sheet-button handling.
- **Measure** signal-to-conceal latency and record it in `docs/test-matrix.md`.

---

## 16. Roadmap

### v0.1: macOS MVP (macOS 14+, universal binary)

**Goal:** a student installs it, grants Accessibility and Automation once, and from then on screen sharing, a projector, the lid, lock or the hotkey closes or hides their sensitive tabs, documents and apps without losing work.

- **M1, skeleton.**
  - Workspace and crate layout.
  - `core` with model, matcher, planner and config + schema.
  - `lists` with the snapshot pipeline.
  - `engine` with `platform-fake`.
  - Tauri menu-bar shell (LSUIElement), settings window, EN+HU i18n plumbing.
  - CI: fmt, clippy, tests, cargo-deny, Biome, ts-rs drift check, no-network runtime test.
  - Spike: do AX and Apple Events work while the screen is locked?
- **M2, inventory and actions.**
  - NSWorkspace apps.
  - AX windows, titles, `AXDocument` and dirty state, with AXObservers.
  - JXA tabs for Chrome, Brave, Edge, Vivaldi and Arc; AppleScript for Safari.
  - Firefox detection via `recovery.jsonlz4` (act = hide app).
  - Opera and Dia: hide app only, until their dictionaries are verified.
  - Documents through AppleScript (Preview, TextEdit, Pages/Keynote/Numbers, Word/Excel/PowerPoint); AX fallback for everything else.
  - Actions: Close, Hide, Minimize, NotifyOnly; Quit opt-in.
  - Onboarding with Accessibility and Automation, plus permission health.
- **M3, triggers.**
  - Panic (hotkey, tray, CLI).
  - SkyLight watcher with adaptive polling and noise calibration; notify-proc behind a flag.
  - Display mirroring and projector detection.
  - CoreAudio/CoreMediaIO `CallLikely` with the pre-flight panel.
  - Will-sleep, lid, lock, away > N, idle > N, item age > N with a countdown.
  - Share-guard mode.
- **M4, hardening and release.**
  - All four unsaved policies with role-based sheet handling.
  - Activity panel with redacted run reports, "why was this closed?" and in-memory restore (reopen normal tabs, reopen document paths, unhide apps).
  - Capability report and `diag`.
  - Test matrix run and latency published.
  - Developer ID signing, notarization, DMG, Homebrew tap. Tag `v0.1.0`.

**Not in v0.1:** Firefox per-tab close, the extension, Calendar, NSFW, persisted journal, Windows, Linux.

**Exit criteria:**
- The test matrix passes or has a documented limitation.
- Zero AF_INET/AF_INET6 syscalls in CI.
- Hungarian strings reviewed by a native speaker.
- No data loss in any unsaved-document scenario.

### v0.2: Windows 10 (2004+) and 11

- `platform-windows`:
  - EnumWindows with filters, `SetWinEventHook`, `sysinfo` command lines.
  - UIA tab titles and close buttons for Chromium 138+ and Firefox 143+.
  - URL cache filled when a tab becomes active.
  - Office COM (Word, Excel, PowerPoint, Protected View); minimize-first for Acrobat Reader.
  - Generic `WM_CLOSE` + UIA dialog handling.
  - Minimize as Hide; skip elevated windows.
- Triggers:
  - WTS lock/unlock; suspend/resume plus display state for Modern Standby; lid; idle.
  - Display topology for projectors.
  - ConsentStore watcher for capture, mic and webcam (with `SharePickerOpened` early warning and capturing-exe protection).
  - Hotkey and autostart.
- Packaging: MSIX for the Store, SignPath-signed NSIS, winget.
- **Exit criteria:** test matrix on current Windows 11 and Windows 10 22H2 (ESU runs to Oct 2027); Defender scan clean; SmartScreen behaviour documented per channel.

### v0.3: Linux X11, plus Wayland-ready foundations

- X11 through `x11rb`: EWMH list, close and iconify; RandR projector; XScreenSaver idle; XGrabKey hotkey.
- Session-independent pieces:
  - logind `PrepareForSleep` with a delay inhibitor, `LockedHint`, ScreenSaver `ActiveChanged`, UPower lid.
  - AT-SPI tabs and dialogs.
  - `/proc` command lines and LibreOffice lock files.
  - **PipeWire screencast and mic watcher.**
  - CLI panic.
- On a Wayland session v0.3 runs **degraded but useful**: share detection, notifications, XWayland windows only, CLI panic. The capability report says exactly that.
- Packaging: deb, rpm, AppImage, AUR.
- **Exit criteria:** Ubuntu 24.04 Xorg session, Linux Mint 22 Cinnamon and Xfce fully tested. Fedora / Ubuntu 26.04 GNOME Wayland verified in degraded mode.

### v0.4: Companion browser extension

- MV3 extension for the Chromium family (Chrome, Edge, Brave, Vivaldi, Opera) and Firefox (AMO, listed). TypeScript, no framework.
- **Permissions:** `tabs`, `nativeMessaging`; optional `history` for scrubbing; content scripts only on meeting domains.
- `bezartam-host` native-messaging host:
  - Talks stdio to the browser and a Unix socket or named pipe to the app.
  - Host manifests in per-browser `NativeMessagingHosts` locations, or HKCU registry keys on Windows.
- Features:
  - Exact URLs for every tab; `tabs.remove`; stash-window Hide.
  - Private windows, when the user allows the extension there.
  - Push updates through `tabs.onUpdated`, so share-guard reacts to navigation.
  - Protection for audible and pinned tabs.
  - `getDisplayMedia` hook as an extra `CaptureStarted` signal.
  - Opt-in `history.deleteUrl` for closed sensitive URLs (the address-bar autocomplete leak).
- The app shows per-browser extension status and upgrades the capability report automatically.
- Safari Web Extension stays deferred (it needs an Xcode container app).

### v0.5: Wayland window control

In this order, cheapest and most stable first:
1. KWin script backend (Plasma 6; 6.8 is Wayland-only).
2. wlr-foreign-toplevel backend (Sway, Hyprland, niri, labwc, river, Wayfire).
3. COSMIC toplevel management.
4. GNOME Shell extension (ESM; D-Bus interface with a **sender check**; extensions.gnome.org review; needs maintaining for every GNOME release).

Also: GlobalShortcuts portal through `ashpd`, Mutter IdleMonitor and ext-idle-notify v2.

### v0.6: Opt-in on-device NSFW image analysis (off by default)

- A new matcher, `ImageClass { label: "nsfw", min_score }`, plus a derived category `adult_detected`.
- **Sources:**
  - Image documents open in viewers (path from AX, command line or `/proc`). The file is read and classified, **with no screen capture**.
  - With the extension, and only after explicit opt-in: the active tab's visible area via `captureVisibleTab`, only for domains that match no list, at most every 30 s, kept in memory only.
- **Model:** Marqo nsfw-image-detection-384 (Apache-2.0, 5.6M parameters) converted to ONNX and bundled (about 22 MB in F32, less quantized: *measure*), run with `tract-onnx`.
- **Optional macOS backend:** SensitiveContentAnalysis, available only if Apple grants the entitlement and the user has Sensitive Content Warning on.

### Later (unordered)

- Safari Web Extension.
- Calendar pre-flight (EventKit and equivalents).
- Encrypted persisted journal behind Touch ID / Windows Hello.
- An "OS hygiene" checklist (desktop icons, notification previews) that opens settings and never changes them.
- **School deployment:** a read-only managed config in a system location, plus a macOS configuration profile.
- More languages through Weblate.
- A Flatpak with reduced features, if it's ever worth it.
- Re-validating the SkyLight symbol on every new macOS major and ConsentStore on every Windows feature update, as a release gate.

---

## 17. Risks and must-verify list

| Risk | Impact | Mitigation |
|---|---|---|
| Apple removes or changes `SLSIsScreenWatcherPresent` | Loses the macOS primary share signal | `dlsym` with fail-open; public fallbacks; checked as a release gate on each macOS beta |
| Microsoft changes the ConsentStore layout | Loses the Windows share signal | Same approach; display topology and hotkey keep working |
| Reactive detection leaks the first frames | The core promise weakens | Pre-flight, cleanup at lock/sleep, window-sharing advice, an honest published latency |
| AX / Apple Events stop working while the screen is locked | The "clean before unlock" design breaks | M1 spike; fall back to acting at will-sleep or lock |
| A TCC prompt appears mid-share | Blocks automation and shows a modal on the shared screen | All prompts at onboarding or app launch; never while a share is active |
| Per-app quirks (Arc IDs, Safari private windows, Opera/Dia dictionaries, Office close syntax on macOS, autosave `saving no`) | Wrong or no action | Re-verify before acting, fall back toward less destructive actions, per-app test matrix |
| Windows antivirus heuristics | Install friction | Signing, no injection, no network, public source, submit builds to Microsoft |
| GNOME extension upkeep | Breaks with each GNOME release | Ship GNOME last; KWin and wlroots first |
| List licensing (GPL / CC BY-SA mix) | Licence compliance | GPL-3.0 app, `manifest.toml` attributions, an About → Licences screen |
| `objc2-*` 0.4 / `windows` 0.63 API breaks | Rework | Check before starting; pin exact versions; keep FFI isolated in platform crates |
| Two zbus versions (atspi on zbus 5, ashpd on zbus 6) | Build size, possible type friction | Accept for v0.3; drop when atspi updates |

**Must verify on a real device before relying on it:**
- Which share sources flip the SkyLight watcher on macOS 14, 15, 26 and 27.
- Whether Safari private windows are enumerable over AppleScript.
- Whether `NSRunningApplication.hide()` works on other apps without TCC.
- The `SLSRegisterNotifyProc` callback signature.
- The minimum macOS version for per-process mic attribution.
- Opera and Dia AppleScript support.
- Office for Mac `close … saving` syntax.
- The Windows 10 ConsentStore behaviour.
- The UIA tab-close button structure per browser version.
- PipeWire node names per desktop version.
- Whether Chromium exposes its tab strip over AT-SPI without renderer accessibility.
- Whether CC BY-SA 4.0 lists can be bundled with a GPL-3.0 app.
