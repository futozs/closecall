# Tech stack decision: "Did I close it?" / „Bezártam?”

*Research date: 2026-11-16. Version numbers were pulled live today from the crates.io API, the npm registry, the GitHub Releases API and the official release feeds (Electron releases.json, the Flutter release JSON, NuGet, go.dev, the Rust channel manifest). Anything I could not confirm from a primary source is marked **(unverified)**. I hit the web-search budget near the end, so a few secondary claims are only partly cross-checked. Section 15 lists them.*

---

## 1. Recommendation (TL;DR)

| Layer | Choice | Exact version (verified 2026-11-16) |
|---|---|---|
| App framework | **Tauri v2** (Rust core, system WebView). Do not start on v3 alpha. | `tauri` 2.12.5, `tauri-build` 2.7.4, `@tauri-apps/cli` 2.12.4, `@tauri-apps/api` 2.12.4 |
| Rust toolchain | stable, edition 2024, resolver 3 | Rust **1.100.0**. Tauri 2.12 MSRV is 1.90 |
| Core architecture | UI-agnostic Rust workspace: `core` (pure logic + platform traits) + `platform-{macos,windows,linux}` + thin Tauri app crate | see §12 |
| macOS bindings | objc2 family (AX via `objc2-application-services`), AppleScript via `osakit` | `objc2` 0.6.5, `objc2-app-kit` / `objc2-foundation` / `objc2-application-services` / `objc2-core-graphics` 0.3.2, `osakit` 0.3.1 |
| Windows bindings | Microsoft `windows` crate (UIA, WTS, power, Restart Manager) | `windows` 0.62.2, `windows-sys` 0.61.2 (optional `uiautomation` 0.25.1) |
| Linux bindings | zbus (logind, portals), AT-SPI, ashpd (GlobalShortcuts / ScreenCast portals) | `zbus` 6.3.0, `atspi` 0.30.0, `ashpd` 0.14.0 |
| Frontend | React + Vite + TypeScript + Tailwind + React Aria Components | `react` 19.3.1, `vite` 8.3.8, `@vitejs/plugin-react` 6.3.0, `typescript` 7.0.2, `tailwindcss` / `@tailwindcss/vite` 4.3.5, `react-aria-components` 1.22.1 |
| Lint/format (web) | Biome (it does not depend on the TS compiler API, so it works with TS 7) | `@biomejs/biome` 2.6.8 |
| Config | TOML file in the app config dir, owned and validated by Rust, JSON Schema generated | `toml` 1.2.1 (TOML spec 1.1), `serde` 1.0.229, `schemars` 1.2.3 |
| Rust→TS types | tauri-specta (pinned exactly, still RC). Fallback: ts-rs | `tauri-specta` 2.0.0-rc.29, `specta` 2.0.0-rc.28, `specta-typescript` 0.0.15; `ts-rs` 12.0.1 |
| i18n | **Fluent** `.ftl` files as the single source for Rust (tray, notifications) and web UI | `fluent-bundle` 0.16.0, `fluent-templates` 0.15.1, `fluent-langneg` 0.14.2, `sys-locale` 0.3.2; `@fluent/bundle` 0.19.1, `@fluent/langneg` 0.7.0 |
| Tests | cargo-nextest + insta + proptest; Vitest + Tauri `mockIPC`; WebdriverIO + `@wdio/tauri-service` (embedded driver, works on macOS) | `cargo-nextest` 0.9.152, `insta` 1.49.0, `proptest` 1.12.0, `vitest` 5.1.0, `webdriverio` / `@wdio/cli` 10.2.0, `@wdio/tauri-service` 1.5.1, `@playwright/test` 1.65.1 (UI-only) |
| Package manager | bun | `bun` 1.4.4 |
| Supply-chain tooling | cargo-deny, cargo-audit, cargo-auditable, SBOM, provenance | `cargo-deny` 0.20.2, `cargo-audit` 0.23.0, `cargo-auditable` 0.7.7, `cargo-cyclonedx` 0.5.10, `cargo-about` 0.9.3 |
| Updates | **No in-app updater in v1.** Use GitHub Releases plus package managers. See §6. | (`tauri-plugin-updater` 2.14.1 assessed and rejected for v1) |
| Browser extension | WXT + native messaging. Ship in **v1.1**, but define the protocol in v1. | `wxt` 0.21.5 |

**Why:** Tauri is the only option that scores high on every heavily weighted criterion at once. Rust gets direct, zero-FFI-layer access to AXUIElement, UI Automation and AT-SPI. The "no network" claim can be proven with standard tooling. Official plugins cover tray, hotkey, autostart, notifications and single-instance. The developer keeps React/TS for the settings window and learns Rust where it matters. The settings WebView can be destroyed when closed, so the always-on part is a small Rust process.

---

## 2. Framework comparison (weighted)

Weights: (1) native a11y/automation APIs **25**, (2) small, auditable, provably offline **20**, (3) tray-first features **15**, (4) i18n **5**, (5) signing, installers, updates **10**, (6) developer fit (TS/React, Rust-curious) **10**, (7) GitHub Actions CI **5**, (8) idle footprint **10**. Each criterion is scored 0–5, and the total is normalised to 100.

| Option (current version) | (1) | (2) | (3) | (4) | (5) | (6) | (7) | (8) | **Score** |
|---|---|---|---|---|---|---|---|---|---|
| **Tauri 2.12.5** (Rust + system WebView) | 5 | 5 | 5 | 4 | 5 | 5 | 5 | 4 | **97** |
| Pure Rust UI: **Slint 1.19.0** (best of group) | 5 | 5 | 4 | 3 | 3 | 2 | 4 | 5 | 84 |
| Native per OS (Swift + C# + GTK) | 5 | 3 | 5 | 2 | 4 | 1 | 3 | 5 | 77 |
| Wails **v3.1.0** (Go) | 3 | 4 | 4 | 3 | 3 | 2 | 4 | 4 | 68 |
| Avalonia **12.1.6** / .NET 10.0.15 LTS | 3 | 3 | 4 | 4 | 3 | 2 | 4 | 3 | 63 |
| Electron **45.3.3** | 2 | 2 | 4 | 4 | 4 | 5 | 4 | 1 | 58 |
| Flutter **3.50.0** (Dart 3.14.0) | 2 | 3 | 3 | 4 | 3 | 2 | 4 | 3 | 55 |
| .NET MAUI 11.0.10 | (no Linux desktop target, so excluded) | | | | | | | | n/a |

### Notes per option

- **Tauri v2.** Stable line is 2.12.x ([2.12 announced 2026-09-26](https://v2.tauri.app/blog/tauri-2.12); 2.12.5 on crates.io 2026-11-06). **Tauri 3.0.0-alpha.6** is out (2026-11-06). The [v3 alpha.0 notes](https://github.com/tauri-apps/tauri/releases/tag/tauri-v3.0.0-alpha.0) list:
  - runtime selection via separate crates (`tauri-runtime-wry` or the new `tauri-runtime-cef`)
  - GTK3/GTK4 features
  - Linux tray switched to `ksni` (dropping libayatana-appindicator)
  - MSRV 1.95

  No v3 stable date or v2 end-of-life is published. **Start on 2.12 and keep Tauri glue in one crate** so a v3 migration stays cheap. Security model ([capabilities](https://v2.tauri.app/security/capabilities/)): IPC is reachable only from bundled code by default ("remote" URLs are denied unless configured). Capabilities can be scoped per window and per platform. CSP gets automatic hashes and nonces ([CSP docs](https://v2.tauri.app/security/csp/)). An optional [Isolation pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/) adds AES-GCM-checked IPC against compromised npm deps.

- **Electron 45.3.3** (2026-11-14; bundles Chromium 156.0.8078.100 and Node 24.22.3, per [releases.json](https://releases.electronjs.org/releases.json)). Built-in tray, globalShortcut, notifications and single-instance are good. But AX, UIA and AT-SPI need native Node addons, which in practice means napi-rs, i.e. writing Rust anyway. Chromium plus Node plus a large npm graph is much harder to audit and to prove offline. Chromium's browser and GPU processes stay resident even with no windows. Benchmarks vary widely: [Better Stack, Jul 2026](https://betterstack.com/community/guides/scaling-nodejs/tauri-vs-electron-vs-deno-vs-electrobun/) measured Tauri 109 MB vs Electron 128 MB idle with one window open. For a tray app, the decisive difference is that Tauri can drop its WebView entirely, while Electron cannot drop Chromium.

- **Wails v3.** **v3.0.0 went GA on 2026-10-26**, and v3.1.0 followed on 2026-11-01 with experimental CGO-free macOS/Linux backends ([GitHub releases](https://github.com/wailsapp/wails/releases)). The betas added layout-aware global shortcuts (beta.29), Linux single-instance (beta.23) and updater fixes (beta.30). It requires Go 1.26+ (current Go 1.27.4). It is credible, but v3 is three weeks old as GA. Go access to AX, UIA COM and AT-SPI goes through cgo, syscall or godbus with thinner bindings than Rust's. And Go is a third language for this developer.

- **Pure Rust UI.**
  - **Slint 1.19.0** was released today and has had a declarative tray icon since [1.17 (June 2026)](https://slint.dev/blog/slint-1.17-released). It is the strongest "no WebView at all" story, but you lose React/TS, and its i18n is gettext-style. Licensing is tri-licensed (GPLv3, royalty-free, commercial) **(check current terms)**.
  - `egui`/`eframe` 0.37.0 and `iced` 0.14.0 (Dec 2025) need manual wiring of `tray-icon` 0.26.1, `global-hotkey` 0.10.0, `notify-rust` 4.19.0 and `auto-launch`.
  - Dioxus 0.7.10 (0.8 alpha) desktop uses wry, the same WebView as Tauri, with a smaller plugin ecosystem.
  - **This is the fallback:** the recommended workspace keeps `core` UI-agnostic, so a Slint front-end could replace the WebView later if the "no WebView" argument ever wins.

- **Native per OS** gives the best API access and footprint, but three codebases and three i18n systems hurt auditability, and it is a poor fit for a solo TS developer.

- **Avalonia 12.1.6 / .NET 10 LTS.** It has a TrayIcon and good P/Invoke, but it is C#, the .NET runtime is larger to audit, and hotkeys and AT-SPI need community libraries. **MAUI 11** has no Linux desktop target.

- **Flutter 3.50.0** (2026-11-11). Tray and hotkeys come from community packages (`tray_manager` 0.9.0, `hotkey_manager` 0.4.0). Every OS API needs platform-channel or FFI code in Swift, C++ or C. Dart is a new language.

---

## 3. Tray-first mechanics in Tauri 2.12

| Need | Tauri solution (version) | Caveats |
|---|---|---|
| Menu-bar/tray icon + menu | core `tray-icon` feature ([docs](https://v2.tauri.app/learn/system-tray/)) | **Linux: tray click events are not emitted** (only the context menu works) on the v2 libayatana backend. v3 alpha moves to `ksni`. Vanilla GNOME needs the AppIndicator extension to show tray icons **(unverified for GNOME 49/50)**. |
| No Dock icon on macOS | `app.set_activation_policy(ActivationPolicy::Accessory)` | |
| Settings window on demand | `"windows": []` in config; create with `WebviewWindowBuilder` from the tray; **destroy (not hide) on close**; keep the process alive with `RunEvent::ExitRequested { code: None, api } → api.prevent_exit()` | A hidden window keeps WebKit processes resident: one report shows ~159 MB RSS in WebKit helpers after "closing" a merely hidden window ([DEV, tinyforge](https://dev.to/tinyforge/tauri-cold-launch-was-1s-heres-how-i-got-it-to-feel-instant-32lh)). Also see [issue #13511](https://github.com/tauri-apps/tauri/issues/13511) on telling quit apart from last-window-close. |
| Global hotkey (panic button) | `tauri-plugin-global-shortcut` 2.4.1 (on `global-hotkey` 0.10.0) | `global-hotkey` is **"Linux (X11 Only)"** ([repo](https://github.com/tauri-apps/global-hotkey)). On Wayland, use the xdg-desktop-portal **GlobalShortcuts** portal via `ashpd` 0.14.0 (GNOME 48+ per [TWIG #189](https://thisweek.gnome.org/posts/2025/02/twig-189), KDE). macOS Carbon hotkeys need no Input Monitoring permission. |
| Autostart | `tauri-plugin-autostart` 2.7.0 (on `auto-launch` 0.6) | macOS: LaunchAgent (default) or AppleScript login item ([docs.rs](https://docs.rs/auto-launch/latest/auto_launch/)). Windows: tries the HKLM Run key first and falls back to HKCU. Prefer the per-user mode explicitly. Linux: XDG autostart. |
| Notifications | `tauri-plugin-notification` 2.8.1 (`notify-rust` 4.18+, `zbus` 5 on Linux) | Send from Rust so they work with no WebView alive. |
| Single instance | `tauri-plugin-single-instance` 2.5.5 | |
| Logging | `tauri-plugin-log` 2.10.2 | Log rule IDs and counts, never URLs or titles (§8). |
| Open release page | `tauri-plugin-opener` 2.7.3 | The OS browser does the request, not the app. |
| **Do not include** | `tauri-plugin-http` 2.9.0, `-shell` 2.6.0, `-fs` 2.7.1, `-updater` 2.14.1, `-store` 2.5.1 | Config I/O happens in Rust, and no network crate enters the graph. |

---

## 4. "Provably no network": layered design

Fact checked in the crate source: **`tauri` 2.12.5 depends on `reqwest` only for `cfg(any(target_os = "android", all(target_vendor = "apple", not(target_os = "macos"))))`**, i.e. mobile. On desktop, the dependency graph can be completely HTTP-client-free.

| Layer | Mechanism | Enforced by |
|---|---|---|
| Rust dependency graph | `deny.toml` `[bans] deny = [reqwest, hyper, hyper-util, h2, h3, quinn, ureq, isahc, curl, curl-sys, attohttpc, tungstenite, tokio-tungstenite, async-tungstenite, rustls, native-tls, openssl, openssl-sys]` with a `reason` on each, plus `[graph] targets` limited to the 6 desktop triples so mobile-only edges don't count. `multiple-versions = "deny"` (or warn), `wildcards = "deny"`, `[bans.build] allow-build-scripts` allow-list ([cargo-deny bans docs](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html)). | `EmbarkStudios/cargo-deny-action` v2.1.1 in CI |
| Our Rust code | `clippy.toml` `disallowed-types`: `std::net::{TcpStream,TcpListener,UdpSocket}`, `tokio::net::{TcpStream,TcpListener,UdpSocket}`; `#![forbid(unsafe_code)]` in `core` (unsafe lives only in platform crates). | `cargo clippy -- -D warnings` |
| Tauri surface | No http/websocket/upload/updater/shell plugins. One capability file for the `settings` window only, no `remote` block. App commands get explicit permissions through `tauri_build`'s app manifest, so even our own commands are allow-listed. | Code review; capabilities are small JSON files |
| WebView | CSP along the lines of `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self'; object-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'`. `connect-src` follows the official [CSP example](https://v2.tauri.app/security/csp/). Only bundled assets are loaded. | E2E test asserts `fetch('https://example.com')` rejects |
| Frontend source | Biome `noRestrictedGlobals` for `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `navigator.sendBeacon` | `biome ci` |
| npm supply chain | bun is "default-secure": it only runs lifecycle scripts for allow-listed packages ([bun docs](https://bun.com/docs/pm/lifecycle)). Add `install.minimumReleaseAge` (seconds) in `bunfig.toml` ([bunfig](https://bun.com/docs/runtime/bunfig)) and `--frozen-lockfile`. Keep UI deps tiny. Consider the Isolation pattern. | CI |
| **Runtime egress test** | Linux CI job: launch the release binary under `xvfb-run` with `strace -f -e trace=connect,sendto,sendmsg`, drive triggers through a test hook, and **fail on any AF_INET/AF_INET6 socket**. Run a second pass inside `unshare -rn` / `bwrap --unshare-net` to show full function offline. | CI artifact (strace log) |
| Release transparency | `cargo auditable build` (dependency list embedded in the binary), CycloneDX SBOM, SHA256SUMS, `actions/attest-build-provenance` v4.2.2 (users run `gh attestation verify`) | Release workflow |
| User-verifiable | README section: verify with LuLu or Little Snitch (macOS), Resource Monitor (Windows), `ss`/strace (Linux) | Docs |

**Scope the claim honestly:** "the app's code makes no network connections." OS components can still talk to the network on their own: the WebView2 runtime's own updates, Gatekeeper/notarization checks at launch, and WebView2 SmartScreen, which is **on by default** (`IsReputationCheckingRequired`, [MS docs](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2settings.isreputationcheckingrequired)). Since only local content is loaded, set that to `false` via `with_webview` / `ICoreWebView2Settings8`. On macOS, App Sandbox (no `network.client` entitlement) **cannot** be used as an OS-level guarantee, because the sandbox blocks the Accessibility APIs this app needs ([Apple DTS on the forums](https://developer.apple.com/forums/thread/810677)).

---

## 5. Signing, installers, distribution

| OS | Path | Facts |
|---|---|---|
| macOS | Developer ID Application cert, Hardened Runtime, **notarization** with an App Store Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or Apple ID (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`) ([Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)). Ship `.app` + `.dmg`, universal (aarch64 + x86_64). Entitlement `com.apple.security.automation.apple-events` + `NSAppleEventsUsageDescription`. **Not sandboxed, so no Mac App Store.** | A free Apple account **cannot notarize**. Ad-hoc signing (`-`) is fine for development. TCC grants (Accessibility, Automation) are tied to the code signature, bundle ID and path, so keep a stable dev signing identity or you will re-grant after every rebuild. Homebrew 5.0 deprecated casks that fail Gatekeeper and announced disablement for Sept 2026 ([Homebrew 5.0.0](https://brew.sh/2025/11/12/homebrew-5.0.0); [6.0.0](https://brew.sh/2026/06/11/homebrew-6.0.0/)), so a Homebrew cask needs notarization. |
| Windows | NSIS (`-setup.exe`) + MSI. Sign via `bundle.windows.signCommand` ([Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)). | **Azure Artifact Signing (formerly Trusted Signing): individuals must be in the US or Canada.** Organizations in the EU are eligible ([MS quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart), updated 2026-10-08), so a Hungarian student cannot use it as an individual. Best fit: **SignPath Foundation** free OSS signing. Requirements: OSI license, no proprietary parts, CI-verifiable builds, MFA, per-release manual approval, a "Code signing policy" page ([terms](https://signpath.org/terms)). Otherwise a commercial OV cert. EV no longer gives instant SmartScreen reputation. Smart App Control blocks unsigned apps. |
| Linux | AppImage + `.deb` + `.rpm` from the Tauri bundler. SHA256SUMS + attestations. | Flatpak and Snap later: sandbox and portal semantics complicate AT-SPI, logind and window control. Build on `ubuntu-22.04` for a lower glibc baseline. |

**CI runners** ([actions/runner-images](https://github.com/actions/runner-images)):
- `macos-26` (= `macos-latest`, arm64), `macos-15`, `macos-15-intel`
- `windows-2025` (= `windows-latest`), `windows-11-arm`
- `ubuntu-24.04` (= `ubuntu-latest`), `ubuntu-22.04`, `ubuntu-24.04-arm`, `ubuntu-26.04`
- **macOS 14 images were retired on 2026-11-02.**

Actions:
- `tauri-apps/tauri-action` (tag `action-v1.0.0`, 2026-06-29)
- `Swatinem/rust-cache` v2.9.3
- `oven-sh/setup-bun` v2.2.0
- `EmbarkStudios/cargo-deny-action` v2.1.1
- `actions/attest-build-provenance` v4.2.2

Pin actions by commit SHA. Standard runners are free for public repos.

---

## 6. Updater assessment (Tauri updater + static JSON on GitHub Releases)

- **Not a backend.** It is a static `latest.json` (`version`, `platforms.{os-arch}.url`, `signature`) that tauri-action generates and uploads to GitHub Releases. Signatures are **mandatory** ("cannot be disabled"). Keys come from `tauri signer generate`, and CI holds `TAURI_SIGNING_PRIVATE_KEY`. Endpoints must be HTTPS. 2.12's CLI binds the app version into updater signatures ([updater docs](https://v2.tauri.app/plugin/updater/); [2.12 notes](https://v2.tauri.app/blog/tauri-2.12)). Update artifacts: `.app.tar.gz`, NSIS/MSI, AppImage. **No deb/rpm self-update.**
- **But it breaks the hard constraint.** `tauri-plugin-updater` 2.14.1 depends on `reqwest ^0.13` plus TLS (verified on crates.io). Every check is an outbound request to github.com and its CDN that reveals IP, timing and app version. It also defeats the cargo-deny ban that makes "no network" provable.
- **Recommendation for v1:**
  - Ship **no updater code**.
  - A tray item "Check for updates…" opens `https://github.com/<org>/<repo>/releases/latest` in the user's browser via `tauri-plugin-opener`. That is user-initiated and done by the browser.
  - Distribute through channels that update themselves: Homebrew cask, winget manifest, `.deb`/`.rpm`, AUR (community), later Flathub.
  - If auto-update is ever wanted, make it a **separate opt-in build flavor** (Cargo feature `updater`, separate artifact name, separate cargo-deny config) and keep the default artifact provably offline.

---

## 7. OS integration map (detectors, triggers, actuators)

Put this behind traits in `core` (`TriggerSource`, `Inventory`, `Actuator`, `PermissionProbe`) with a `FakePlatform` for tests. Prefer events over polling.

| Concern | macOS | Windows | Linux |
|---|---|---|---|
| Will sleep (act **before** sleep, the best defence for the lid-open case) | IOKit `IORegisterForSystemPower`, `kIOMessageSystemWillSleep`. Sleep is delayed up to 30 s if not acknowledged ([QA1340](https://developer.apple.com/library/archive/qa/qa1340/_index.html)). Also `NSWorkspaceWillSleepNotification` (on NSWorkspace's own notification center). | `WM_POWERBROADCAST` / `PBT_APMSUSPEND` (Modern Standby behaves differently, so test it) | logind `PrepareForSleep(true)` + **delay inhibitor lock** ([logind D-Bus API](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)) |
| Wake / lid | `NSWorkspaceDidWakeNotification`; clamshell state from the IORegistry `AppleClamshellState` **(not formally documented)** | `RegisterPowerSettingNotification(GUID_LIDSWITCH_STATE_CHANGE)`, where 0 = closed and 1 = open ([MS docs](https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids)) | `PrepareForSleep(false)`; logind `LidClosed` |
| Lock / unlock, idle | Distributed notifications `com.apple.screenIsLocked` / `…Unlocked` **(undocumented, widely used)** | `WTSRegisterSessionNotification` → `WTS_SESSION_LOCK` / `UNLOCK`; `GUID_SESSION_USER_PRESENCE` | logind session `Lock` / `Unlock` signals, `LockedHint`, `IdleHint` / `IdleSinceHint` |
| Projector / mirroring (reliable public signal) | `CGDisplayRegisterReconfigurationCallback` + `CGDisplayIsInMirrorSet` | `WM_DISPLAYCHANGE` + `QueryDisplayConfig` (clone topology) | Mutter DisplayConfig D-Bus / RandR (X11) |
| **Screen sharing / recording (primary trigger)** | **No public API** ([Apple forums](https://developer.apple.com/forums/thread/669127)). Options: (a) heuristic window/process rules (share toolbars of Zoom, Teams, Chrome "is sharing your screen" bar, OBS) read through AX; (b) private `CGSIsScreenWatcherPresent` (avoid or make opt-in); (c) `screencaptureui`/`screencapture` processes for built-in capture ([example](https://github.com/TomasHubelbauer/macos-screen-recording-detect)) | **No public detection API**. Windows Graphics Capture only shows a capture border to the user ([WGC docs](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture)). Use window-title and process heuristics via `SetWinEventHook` | PipeWire screencast stream nodes (`pipewire` 0.10.1) / portal ScreenCast sessions **(heuristic, needs a spike)** |
| Window, document and tab inventory | AXUIElement (`AXDocument` gives the file URL of document windows); AppleScript for Safari and Chromium family (Chrome, Edge, Brave, Arc) tab URLs and incognito mode; Office/iWork AppleScript for save/close. Avoid the Screen Recording permission (only needed for `CGWindowList` titles). | UI Automation (window titles, tab items); `RmGetList` (Restart Manager: which process holds a file); Office COM `Document.Close(SaveChanges)` | AT-SPI over D-Bus. Chromium exposes its tree only when a11y is enabled (`org.a11y.Status IsEnabled`, GNOME `toolkit-accessibility`, or `--force-renderer-accessibility`, per [Chromium a11y docs](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/accessibility/overview.md)); `/proc/*/fd` for open files |
| Actuators (closing safely) | `NSRunningApplication.hide()` (safest universal action), AX `AXMinimized`, press AX close, AppleScript `close tab` / `save` | `ShowWindow(SW_MINIMIZE)`, UIA `WindowPattern.Close`, Office COM | AT-SPI actions. **Wayland forbids cross-app window control** except via compositor-specific protocols, so expect weaker Linux behavior. |
| "Open longer than N hours" | First-seen timestamps **kept in RAM only** | same | same |

**Unsaved-document policy (per rule):** `hide` is the default for screen-share triggers. `save` only where a scripting API exists (Office, iWork). `close` triggers the app's own save prompt. `discard` requires explicit per-rule opt-in. Unsupported cases fall back to `hide + notify`. Add a **dry-run / "show me what would happen"** mode; it is both a trust feature and a testing aid.

**Privacy of the app itself:** the audit log should record rule IDs, counts and timestamps, never URLs, titles or file names (opt-in verbose mode at most). Otherwise the leak-prevention tool becomes a leak.

---

## 8. Frontend

- **React 19.3.1 + Vite 8.3.8 (Rolldown-based) + TypeScript 7.0.2.** The developer is React-fluent. Framework choice barely affects memory, since the WebView process dominates and is destroyed when closed. Svelte 5.57.3 would ship less JS, but that doesn't matter for local assets. **Solid 2.0 is still RC (2.0.0-rc.22)**, so avoid it.
- **TypeScript 7.0** (Go-native, [GA 2026-07-08](https://devblogs.microsoft.com/typescript/announcing-typescript-7.0/)) ships **without a programmatic API**. `typescript-eslint` 8.72.2 peers `typescript <6.1.0`. So use `tsc --noEmit` from TS 7 plus **Biome 2.6.8** for lint and format. If a tool needs the old API, `@typescript/typescript6` 6.0.2 provides `tsc6` side by side.
- **Styling:** Tailwind 4.3.5 via `@tailwindcss/vite`.
- **Components:** **React Aria Components 1.22.1**. It is accessible, unstyled, and ships localized strings including **`hu-HU`** ([React Aria i18n](https://react-aria.adobe.com/quality#internationalization)). Alternative: Base UI `@base-ui/react` 1.9.0.
- No router. A single settings window with tabs is enough. State: server state is the Rust config (typed commands + events). Local UI state uses React state; `zustand` 5.1.0 only if needed.

## 9. Config and state

- **TOML** (`toml` 1.2.1) at `app_config_dir()/config.toml`, i.e. `~/Library/Application Support/<id>/`, `%APPDATA%\<id>\`, `~/.config/<id>/`.
  - Human-editable and diff-friendly, and it allows comments, which suits a transparency-focused tool.
  - Rust owns the types: `serde` with `deny_unknown_fields`, a `schema_version` field and explicit migrations (snapshot-test them with `insta`).
  - Atomic writes (temp file + rename).
  - Optional hot-reload of manual edits with `notify` 8.2.2.
- **JSON Schema** generated by `schemars` 1.2.3 into `schema/config.schema.json`, committed and diff-checked in CI. Users get editor validation through Taplo/Even Better TOML's `#:schema` directive **(Taplo directive support unverified for current versions)**.
- **TS types:** `tauri-specta` 2.0.0-rc.29 generates typed commands **and events**. Pin with `=`, because it has been RC since 2024. If RC churn hurts, switch to `ts-rs` 12.0.1 (stable) with a hand-written typed `invoke` wrapper. No runtime validation library is needed in the UI because Rust validates. Don't use `tauri-plugin-store`.

## 10. i18n

- **Fluent** is the only option with mature runtimes on **both** sides, so one set of `.ftl` files serves Rust (tray menu, notifications, permission prompts, which must work with no WebView) and React.
  - Rust: `fluent-templates` 0.15.1 (compile-time embedded, fallback chains) on `fluent-bundle` 0.16.0; locale from `sys-locale` 0.3.2 plus `fluent-langneg` 0.14.2.
  - Web: `@fluent/bundle` 0.19.1 + `@fluent/langneg` 0.7.0, importing the same files via Vite `?raw`, behind a ~30-line hook. `@fluent/react` 0.15.2 works but was last released 2023-08.
- Fluent's upstream is slow but alive: fluent-rs pushed 2026-09, fluent.js 2026-05.
- **i18next 27.1.3 / react-i18next 18.0.2** and **Paraglide JS 3.2.2** are JS-only, so Rust would need a second, duplicated system.
- Locales: `en-US` base, `hu-HU` first translation. Hungarian plural categories are `one`/`other`, which Fluent and `Intl.PluralRules` handle. Avoid gluing case suffixes onto variables (vowel harmony). Write „Bezárva: { $count } lap” rather than suffixing `{ $app }`.
- CI: an xtask parses every `.ftl` and fails on missing or extra message IDs across locales.

## 11. Testing

| Level | Tooling | Notes |
|---|---|---|
| Rust core | `cargo nextest` (0.9.152), `insta` 1.49.0 (config migration snapshots), `proptest` 1.12.0 (domain/glob matchers) | Core is pure, so fast and deterministic tests run against `FakePlatform` |
| Platform crates | Integration tests on each OS runner (open TextEdit/Notepad/gedit, detect, hide) | The macOS runner needs an Accessibility TCC grant; **may not be possible on hosted runners**, so keep a manual checklist / self-hosted Mac fallback |
| UI unit | Vitest 5.1.0 + Testing Library + `mockIPC` / `mockWindows` / `clearMocks` from `@tauri-apps/api/mocks` ([docs](https://v2.tauri.app/develop/tests/mocking/)) | |
| Real-app E2E | WebdriverIO 10.2.0 + `@wdio/tauri-service` 1.5.1, **embedded provider** (works on macOS, Windows, Linux) with the Rust plugin compiled only under an `e2e` Cargo feature ([service docs](https://github.com/webdriverio/desktop-mobile/tree/main/packages/tauri-service)) | `tauri-driver` 2.1.0 works only on Windows and Linux ("macOS has no WKWebView driver tool", per [Tauri WebDriver docs](https://v2.tauri.app/develop/tests/webdriver/)). CrabNebula's driver needs a paid key on macOS. Community `tauri-plugin-webdriver` 0.2.4 is debug-only. A CI check must ensure release builds contain none of these. |
| UI in a browser | Playwright 1.65.1 against the Vite dev server with mocked IPC | Playwright cannot drive WKWebView or WebKitGTK inside Tauri |
| Egress | strace / unshare job (§4) | |

## 12. Workspace layout

```
bezartam/                      # working title; crate prefix e.g. bezartam-*
├─ Cargo.toml                  # [workspace] resolver="3", edition 2024, [workspace.lints], shared deps
├─ rust-toolchain.toml         # channel = "1.100.0"
├─ deny.toml  clippy.toml  biome.json  bunfig.toml  package.json (bun workspaces)  bun.lock
├─ locales/{en-US,hu-HU}/*.ftl # single i18n source (Rust + web)
├─ schema/config.schema.json   # generated, CI-diffed
├─ crates/
│  ├─ core/                    # rules, matchers, trigger state machine, action planner,
│  │                           # config types (+ schemars/specta derives), platform TRAITS,
│  │                           # FakePlatform (feature "test-support"); #![forbid(unsafe_code)]; no tauri
│  ├─ platform-macos/          # objc2*, AX, osakit, IOKit power, CGDisplay
│  ├─ platform-windows/        # windows 0.62 (UIA, WTS, power, RestartManager, WinEvent hooks)
│  ├─ platform-linux/          # zbus (logind), atspi, ashpd (GlobalShortcuts, ScreenCast)
│  ├─ i18n/                    # fluent-templates loader shared by tray/notifications
│  └─ native-host/             # v1.1: browser native-messaging stdio bridge → local socket
│                              #       (interprocess 2.4.8: Unix socket / named pipe, no TCP)
├─ apps/desktop/
│  ├─ src-tauri/               # Tauri glue only: tray, windows, commands, capabilities/, tauri.conf.json
│  └─ ui/                      # React + Vite settings window
├─ extension/                  # v1.1: WXT project (Chromium + Firefox)
└─ xtask/                      # codegen (TS bindings, JSON schema), .ftl lint, egress test runner
```

Note a version-skew detail: `atspi` 0.30.0 and Tauri's notification and single-instance plugins use **zbus 5**, while `ashpd` 0.14.0 uses **zbus 6**. Either accept two zbus versions (relax cargo-deny `multiple-versions` for zbus) or pin an ashpd release built on zbus 5.

## 13. Package manager

**bun 1.4.4** for JS (Tauri CLI via `bunx tauri` / `bun tauri`), with the text lockfile `bun.lock`, `--frozen-lockfile` in CI, `minimumReleaseAge` (for example 259200 s = 3 days) and default-blocked lifecycle scripts. Cargo stays the Rust package manager. Commit `Cargo.lock` and use Dependabot or Renovate for both ecosystems.

## 14. Browser extension: v1 or later?

**Later (v1.1), but design for it in v1.**

- **v1 without an extension:** macOS covers Safari and the Chromium family through AppleScript (list and close tabs by URL, including incognito windows). Firefox gets window-level hide only. On Windows and Linux, tab-level control via UIA or AT-SPI is fragile, so window-level hide is the fallback.
- **v1.1 extension:**
  - **WXT 0.21.5** (MIT, active). Not Plasmo: npm `plasmo` 0.90.5 has had no release since 2025-05.
  - WXT defaults to MV3 for Chromium and MV2 for Firefox/Safari ([WXT docs](https://wxt.dev/guide/essentials/target-different-browsers)).
  - `chrome.tabs` gives exact URLs and close/discard. A **MAIN-world content script that wraps `getDisplayMedia`** on Meet, Teams and Zoom web would give a precise screen-share trigger for web meetings, the best signal available for the primary trigger.
  - Native messaging: a stdio host with 32-bit length-prefixed JSON, **max 1 MB host→browser and 64 MiB browser→host**, per-browser manifest locations (registry on Windows) ([Chrome docs](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)). No network involved.
- **Why not v1:** store reviews (Chrome Web Store, AMO), per-browser host registration on 3 OSes, and an extra release pipeline.
- **Safari is v1.2+:** it requires an Xcode containing app. Developer ID distribution outside the App Store is supported ([Apple](https://developer.apple.com/documentation/safariservices/distributing-your-safari-web-extension)).
- Define the message schema (Rust types → TS via specta) in v1 so the extension slots in later.

## 15. Possibly outdated or unverified (re-check before relying on it)

- Wails v3 went GA on 2026-10-26 and is moving fast. Re-evaluate if Go appeals.
- The Tauri v3 timeline is unknown (alpha.6 on 2026-11-06).
- Homebrew's September 2026 disablement of Gatekeeper-failing casks was announced; I did not confirm it was enforced.
- Memory numbers come from third-party benchmarks with differing methods. Measure your own tray-only RSS per OS in CI.
- I did not verify whether WKWebView or WebKitGTK helper processes fully exit after the last Tauri webview is destroyed.
- GNOME tray visibility without the AppIndicator extension, and GlobalShortcuts portal reliability, still need checking: an August 2026 bug report showed failures on GNOME 48.7.
- Undocumented macOS keys and notifications (`AppleClamshellState`, `com.apple.screenIsLocked`, `CGSIsScreenWatcherPresent`) could change in future macOS releases.

## 16. Note for the next workflow step

The user asked (in Hungarian) for the project to be created in their **developer → personal** folder. This research step wrote no files. The scaffolding step should resolve that folder (for example `~/Developer/personal/<repo>`) and confirm the exact path with the user if it is ambiguous.
