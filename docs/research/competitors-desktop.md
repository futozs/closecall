# Market map: "Did I close it?" / „Bezártam?”

*Research date: 2026-11-16. Facts come from official docs, repos and changelogs wherever they could be reached; vendor-only claims are flagged. Prices and versions change fast, so recheck before publishing anything.*

## Röviden
- 45 overlapping products and built-in features made the map. The niche is real but crowded at the shallow end: plenty of tools **hide everything** (boss keys, presentation modes) or **silence notifications** (Muzzle, Hush). **No tool found looks at what is open (tab domain, document type) and closes only that.**
- The closest competitors: **Stealthly** (macOS, $12.99, hides the whole desktop when sharing starts), **Screen Guard** (macOS, hides selected apps on mirroring, external display or unlock), **Quitter / AutoQuit / Raycast Auto Quit** (idle-based quitting), and **Tuple App Veil** (per-window hiding, only inside Tuple).
- Important technical finding: on macOS 15.4+ the window-level capture exclusion (`NSWindow.sharingType = .none`) does not work with ScreenCaptureKit. Apple DTS says "no public APIs for preventing screen capture". On Windows it only works through DLL injection. **Closing or hiding before the share starts** is therefore more robust than masking during the share. That is the product's core differentiator.
- None of the tools is cross-platform, local-only, open source and localized at the same time. Linux is nearly empty.

## 1. Landscape by category

| Category | Representative products | What they lack relative to the brief |
|---|---|---|
| Auto-hide / auto-quit idle apps | Quitter, Hocus Focus (dead), Raycast Auto Quit, AutoQuit (OSS), Aion | Whole apps only, inactivity only, no privacy triggers, crude unsaved-work handling |
| Screen-share notification silencers | Muzzle (last release 2021), Hush (OSS), CleanShot X (own recordings only), KDE built-in, GNOME extension, Chrome 88+ | Only notifications; open windows and tabs are untouched |
| "Clean desktop" presentation modes | PliimPRO/Pliim, Stealthly, Zen Mode, One Switch, HiddenMe, TopNotch (cosmetic) | Hide **everything**, no content awareness, often manual, restore is partial |
| Trigger-based app hiders | Screen Guard (mirroring / external display / unlock), Stealthly (share / app-launch / schedule) | App-level only, macOS only, closed source |
| Per-window capture exclusion | Invisiwind, WinHider, HideMyWindows, Cloakly, DisplayGuard (Windows, DLL injection); WindowCloak, Cloak, Unshown (macOS); KDE Plasma 6.6; Tuple App Veil | Manual window picking; fragile on macOS 15.4+; antivirus flags on Windows; content stays open |
| Boss key / panic button | MindGems Boss Key, Clicky Gone (2013), Magic Boss Key et al., AutoHotkey scripts, Mole, PanicMode (Tauri, mac+win), Raycast "Quit All Apps", GNOME Show Desktop Plus, xdotool | Manual only, hide or cover instead of close, no domain or file rules |
| Browser-level | Panic Button (AE Creations), AutoScreenShield, Tab Wrangler (OSS), URL-regex auto-close extensions, Safari Locked Private Browsing | One browser only; no OS triggers (wake, lid, desktop meeting apps) |
| DIY automation substrate | Hammerspoon (OSS), Keyboard Maestro, EventScripts, macOS 26 Shortcuts automations, AutoHotkey | Every user has to build it themselves |
| OS / meeting-app built-ins | macOS picker + presenter preview + "notifications off when sharing"; Windows DND-on-duplicate + taskbar "Share this window"; Zoom/Teams/Meet window/tab sharing | Depend on choosing correctly every time; nothing cleans up what is already open |

## 2. Closest competitors in detail
- **Stealthly** (macOS 14+, $12.99, closed, v1.5.1). Starts automatically on screen share/recording (direct version only), on app launch, or on a schedule. Hides desktop icons, wallpaper, Dock, menu bar icons and *all* active windows, and turns on DND. Hidden windows are not restored. Uses Sentry crash reporting, so it is not fully offline.
- **Screen Guard** (macOS 14.6+, Mac App Store, v1.1.0 May 2025). Hides chosen apps on mirroring, external-display connect and unlock; unlock overlay; Active Hours. This is the classroom-projector and wake case. No Zoom/Teams trigger listed.
- **PliimPRO** (macOS, $22). One-click presentation mode with restore. Manual, all-or-nothing.
- **Quitter / AutoQuit / Aion / Raycast Auto Quit.** Time-based quitting. AutoQuit (GPL-3.0) is the only one with an hours-scale timer (1-48 h), a warning dialog and busy-app protection. Its "force quit without saving" option is the opposite of a safe unsaved-work policy.
- **Tuple App Veil.** The best UX reference: a default sensitive-app list (Messages, Passwords, Keychain, Slack, Signal, WhatsApp, Telegram, 1Password, Bitwarden...), a dashed orange border on hidden windows, and per-window "Reveal". It only works inside Tuple.
- **AutoScreenShield** (Chrome). Domain-list tab protection that starts on web-meeting share. It shows the tab-side design, but it is limited to Chrome and browser meetings.
- **PanicMode** (mac+win, Tauri). Hotkey decoy workspace. Same tech stack as the planned app, but it covers content instead of closing it.

## 3. Technical facts that shape the product
1. **macOS capture exclusion is broken for third parties.** On macOS 15.4+ a window with `sharingType = .none` still shows up in ScreenCaptureKit streams. Apple DTS (Jul 2025): "At this time there are no public APIs for preventing screen capture." Zoom, new Teams, QuickTime and system screenshots ignore the flag; Chrome-based Meet still honors it on the legacy path (third-party testing, early 2026). Any "invisible window" product on macOS is fragile, which makes "close or hide before sharing" the sound strategy.
2. **Windows exclusion only works on your own windows.** `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` (Win10 2004+) only accepts windows of the calling process, and Microsoft says it is not a security guarantee. That is why Invisiwind and WinHider inject DLLs and get antivirus false positives. An open-source privacy tool should avoid injection.
3. **The macOS picker changes the threat model.** SCContentSharingPicker (public since Sonoma) and Sequoia's presenter preview let users share single windows. Apps that bypass the picker get monthly re-approval prompts. This tool should not need Screen Recording permission at all: detect sharing from other signals and act through Accessibility/Apple Events.
4. **Linux.** KDE Plasma 6.6 (17 Feb 2026) has window-rule capture exclusion and DND-while-sharing (Wayland only). GNOME has neither built in; an extension exists. xdotool does not work on Wayland, so Linux window control must go through compositor-specific paths (KWin scripts, GNOME Shell extension, or the xdg-desktop-portal / PipeWire state).
5. **Safari already locks private windows** after inactivity, lock, sleep or screensaver (Safari 17+, on by default). Treat this as validation of the "wake/lock" trigger, and as a built-in feature to integrate with rather than duplicate.

## 4. Market gaps this product can fill
See `market_gaps` for the full list. The short version:
1. **Content-aware cleanup.** No product decides by URL domain, file type or document path. Everything else acts on whole apps or the whole desktop.
2. **Cross-app tab and document coverage**: Chrome/Edge/Firefox/Safari/Arc tabs plus Preview/Acrobat PDFs, Office, Pages/Keynote, Photos. Today that needs one browser extension per browser plus manual work.
3. **The trigger matrix in one tool.** Share start, mirroring/projector, wake/lid open, locked > N min, item open > N h, and a panic hotkey are spread across 5+ products.
4. **An explicit unsaved-work policy** per rule (save / discard / hide instead of close / skip and notify), plus a restore or undo log after the meeting.
5. **Pre-emptive instead of reactive.** A "meeting app launched / call joined" trigger and a pre-share checklist ("3 sensitive items open: close?") avoid the race against the first shared frame and the broken macOS exclusion API.
6. **Cross-platform, local-only, open source, auditable.** The Accessibility and Automation permissions such a tool needs demand trust. Existing open-source options are tiny, unsigned, unlicensed or Windows-only with DLL injection, and the commercial ones are closed (one sends Sentry telemetry).
7. **Maintenance vacuum.** Muzzle (2021), Hocus Focus (2021, Intel-only), Quitter (re-signed only), Clicky Gone (2013) and Pliim (no releases) still get recommended.
8. **Linux and Wayland** have almost nothing beyond KDE's built-ins.
9. **Presets for sensitive categories** (banking, adult, medical, private mail, password managers), stored as local lists; Tuple's default list shows the value.
10. **Localization and the student/classroom audience.** No competitor offers Hungarian. Existing tools target developers and sales calls, not students on school projectors.

## 5. Positioning suggestion
"The pre-flight privacy check for your screen": detect what is sensitive, close or hide it *before* anyone sees it, and handle unsaved work safely. Built in the open, runs offline, works on all three OSes. Avoid competing on "invisible windows during the share", which is fragile on macOS 15.4+ and needs DLL injection on Windows. Integrate with the OS features (window picker, Safari private lock, KDE exclusion) instead of reimplementing them.

## 6. Caveats
- Several 2025-2026 entrants (Unshown, Cloakly, PanicMode, Aion, WindowCloak, NoCapture, PrivacyStage, ScreenVeil, Cloaky) are known mainly from Product Hunt or vendor pages. Their features are vendor claims and their survival is uncertain. NoCapture, PrivacyStage, ScreenVeil and Cloaky were seen but not verified, so they are not in the product list.
- WindowCloak's GitHub repo returned 404. HiddenMe's App Store page rate-limited the fetch. The AMO page of Panic Button and the Chrome Web Store page of AutoScreenShield did not load, so some details come from search snippets.
- The macOS 15 capture-exclusion behavior per meeting app comes from a third-party test (early 2026). Apple's statement only confirms that no public API exists. Re-test on Tahoe 26.x / macOS 27 before relying on it.
- No files were written. The user's instruction to create the project in the personal folder under Developer applies to a later scaffolding step.