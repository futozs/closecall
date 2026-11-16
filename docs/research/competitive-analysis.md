# Competitive analysis: „Bezártam?” / "Did I close it?"

**Snapshot:** 2026-11-16. Built from this workflow's desktop and browser market reports, which used official docs, repositories, the AMO, GitHub and Hugging Face APIs, and a third-party Chrome Web Store mirror (the store itself redirected to a consent wall from Hungary). Prices, versions and maintenance status change fast. Treat anything here older than about three months as possibly outdated. Items marked *(vendor claim)* or *(unverified)* were not confirmed against a primary source. Recheck before quoting anything publicly.

## Summary

- About 60 products and built-in features overlap with the brief. After deduplication, **20 are real competitors**. The rest are ingredients (domain lists, NSFW models) or patterns worth copying (password-manager auto-lock settings).
- Each competitor covers one cell of the problem. Some hide everything (presentation modes, boss keys). Some silence notifications (Muzzle, Hush). Some quit idle apps (Quitter, AutoQuit, Raycast). Some exclude windows from capture (Invisiwind, Unshown). Some act on tabs inside a single browser (panic extensions, Tab Wrangler). **None of them chooses what to act on by content across tabs, documents and apps.**
- **Closest commercial rival: Stealthly** (macOS, $12.99). It reacts to screen sharing, but it hides every window, does not restore them, and ships Sentry crash reporting. **Closest browser-side idea: Stealth Browser Assistant**, which stashes tabs by keyword or domain. It is proprietary and has about one Firefox daily user.
- **Masking during a share is a losing position.**
  - macOS 15.4+: ScreenCaptureKit ignores `NSWindow.sharingType = .none`, and Apple DTS says there is no public API to prevent capture.
  - Windows: excluding another process's window requires DLL injection.
  - The durable strategy is to close or hide sensitive items **before exposure and at the moment a share starts**. That is our strategy.
- No competitor is cross-platform, open source, offline and localized all at once. Linux has almost nothing, and nobody ships Hungarian.

## Legend

● full · ◐ partial or limited · – none · ? not documented

What each column means:
- **Share:** reacts on its own when a screen share or recording starts.
- **Projector:** reacts to mirroring or an external display.
- **Wake/lock:** reacts to the lid, sleep, lock or unlock.
- **Age:** has a time rule (idle time or open too long).
- **Panic:** has a manual one-shot trigger.
- **Acts on:** the smallest unit the tool can single out.

## Capability matrix

| # | Product | OS | Licence / price | Status (Nov 2026) | Share | Projector | Wake/lock | Age | Panic | Acts on | Action | Restore |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Stealthly | macOS 14+ | Closed, $12.99 | Maintained, v1.5.1 | ● direct build only | ? | – | – | ◐ toggle | Whole desktop | Hide all windows, icons, Dock; DND | – |
| 2 | Screen Guard | macOS 14.6+ | Closed, Mac App Store | v1.1.0 (May 2025), slowing | – | ● | ● unlock + overlay | – | – | App | Hide | ? |
| 3 | PliimPRO / Pliim | macOS 10.15+ | $22 / CC0 | Low activity | – | – | – | – | ● toggle | Whole desktop | Hide apps and icons, DND, mute | ● |
| 4 | Muzzle / Hush | macOS | Free closed / MIT | Muzzle stale since Jun 2021; Hush tiny | ● | ◐ | – | – | – | Notifications only | DND / Focus | ● automatic |
| 5 | Quitter / Hocus Focus | macOS | Free, closed | Unmaintained / abandoned (Intel-only) | – | – | – | ◐ inactivity | – | App | Hide or quit | – |
| 6 | AutoQuit (rm335) / Aion | macOS 13+ / 14+ | GPL-3.0 / freemium | Hobby / new in 2026 | – | – | – | ◐ idle 1–48 h | – | App | Quit | – |
| 7 | Raycast (Auto Quit, Quit/Hide All Apps) | macOS, Windows | Closed, freemium | Active | – | – | – | ◐ idle up to 15 min | ● hotkey | App | Quit / hide | – |
| 8 | Tuple App Veil | macOS (inside Tuple) | Paid subscription | Active | ◐ Tuple sessions only | – | – | – | – | Window | Exclude from Tuple's stream | ● per window |
| 9 | Unshown / Cloak / WindowCloak | macOS | Closed / no licence / repo 404 | New 2025–26 | ◐ you must share their proxy | – | – | – | ◐ Cloak hotkey | App / window | Filter the outgoing stream | ● |
| 10 | Invisiwind / WinHider / HideMyWindows | Windows 10 2004+ | MIT / GPL-3.0 / MIT + Commons Clause | Active, small | ◐ always-on exclusion, no detection | ◐ | – | – | ● hotkey | Window | Capture exclusion via DLL injection | ● |
| 11 | Boss Key (MindGems) and legacy boss keys | Windows | Commercial / freeware | Sold / mostly legacy | – | – | – | – | ● | All windows | Hide, mute, cover app | ● |
| 12 | PanicMode | macOS, Windows (Tauri) | Unclear, freemium | Active 2026 | – | – | – | – | ● | Whole screen | Decoy overlay | ● |
| 13 | Hammerspoon / Keyboard Maestro / EventScripts / macOS Shortcuts | macOS | MIT / paid / paid / built-in | Active | ◐ DIY | ● DIY | ● DIY | ◐ DIY | ● | Whatever you script | Whatever you script | DIY |
| 14 | Stealth Browser Assistant | Firefox, Chrome, Edge | Proprietary, freemium | New, v1.0.9 (Sep 2026) | – | – | – | – | ● | Tab (keyword, domain) | Mute + stash window; closing is paid | ● |
| 15 | Panic-button extensions (Panic Button, Hide Tabs, Hide!!!, AE Panic Button) | Chrome, Firefox | Mostly closed | Maintained, low activity | – | – | – | – | ● | All tabs (Hide!!!: hand-picked tabs) | Close or hide + safe page | ● |
| 16 | Share-blur extensions (BlurrySite, DataBlur, Safe Screen Share) | Chrome, Firefox | GPL-3.0 / closed / closed | Active, small user bases | ◐ only shares started in that browser | – | – | – | ◐ | Page content | Blur | ● |
| 17 | AutoScreenShield | Chrome | Unverified | Little traction | ◐ web meetings only | – | – | – | – | Tab (domain list) | Mask title, move to minimized window | ● |
| 18 | Tab Wrangler / Auto Tab Discard | Chrome, Firefox | GPL-3.0 / MPL-2.0 | Active (TW v9.1.1, Oct 2026) | – | – | – | ● idle | – | Tab | Close to "Corral" / discard | ● Corral |
| 19 | Incognito Killer (two extensions) | Chrome | Closed | Stale / abandoned | – | – | ● lock; resume | ◐ idle | – | Incognito tabs | Close | – |
| 20 | LeechBlock NG | 6 browsers | MPL-2.0 | Active, ~114k Firefox daily users | – | – | – | ◐ time limits | ◐ lockdown | Site / tab | Block, blur or close at navigation | – |
| – | **Bezártam? (target state, v0.4)** | macOS, then Windows, then Linux | GPL-3.0, free | Planned | ● best effort, labelled with confidence | ● | ● incl. "away longer than N min" | ● idle and "open longer than N h" | ● | Tab, document window, app window, process | Close / hide / minimize / quit, with a per-rule unsaved-work policy | ● in memory |

On Tab Wrangler's licence, one report said MIT and the GitHub API says GPL-3.0. The API wins.

## What each does better and worse than us

| # | Product | Better than us | Worse than us | What we take from it |
|---|---|---|---|---|
| 1 | Stealthly | Polished, signed and notarized. Also hides desktop icons, wallpaper, Dock and menu-bar icons, which we leave alone in v0.x. Has named per-app workflows (Zoom, OBS) and a schedule trigger. | Hides every window whatever its content, and does not restore hidden windows. The App Store build has no detection. Sends Sentry telemetry. Closed source, macOS only, no wake/lock or tab handling. Its own docs say it cannot stop you opening something sensitive mid-share; our share-guard mode can. | Per-meeting-app awareness, which becomes our protected "meeting" category. Desktop clutter goes on the later list as an OS-settings checklist. |
| 2 | Screen Guard | Unlock overlay, weekday "Active Hours", external-display trigger. Covers the classroom projector case. | Works on whole apps only. No Zoom/Teams/Meet trigger. Closed source, macOS only, last changelog May 2025. | Schedule as a later rule condition. We don't need the overlay because we act before unlock. |
| 3 | PliimPRO / Pliim | One click covers icons, notifications, wallpaper and audio, then restores the previous state. | Manual and all-or-nothing. Closes nothing. Low activity. | Treat restoring the previous state as a first-class feature. |
| 4 | Muzzle / Hush | No configuration. Hush's detection code is open (MIT) and worth reading. | Silences notifications only. Muzzle relies on the pre-Monterey DND model. Hush has about 4 stars. | We don't toggle Focus, because there is no public API. Onboarding points users to macOS's own "notifications off while sharing" switch. |
| 5 | Quitter / Hocus Focus | Very simple. Hiding by default means nothing is lost. | Whole apps only, inactivity only, unmaintained, Intel-only. | Hide is the safe default action. |
| 6 | AutoQuit / Aion | Per-app timers in hours. Skips busy apps (media, downloads, calls). A 60 s warning offers Keep or Quit now. | Whole apps only. Unsaved work either gets the app's own prompt or is force-discarded. No privacy triggers. | Countdown with Keep for the item-age trigger. Skip targets that are playing audio. |
| 7 | Raycast | Large user base, polish, macOS and Windows. Hotkey Quit/Hide All with an exclusion list. Leaves apps alone while they record audio or video. | Built for memory and focus, not privacy. Whole apps only. Idle limit tops out at 15 min. Closed source, with account and cloud features. | Exclusion-list UX. Users will compare our panic hotkey's speed with Raycast's. |
| 8 | Tuple App Veil | Real per-window exclusion inside its own stream. A good default sensitive-app list. Clear visual cue (dashed border) and per-window Reveal. | Only works during Tuple sessions. | Default app list: Messages, Passwords, Keychain Access, 1Password, Bitwarden, Signal, WhatsApp, Telegram, Slack. Per-item "never touch this" override. |
| 9 | Unshown / Cloak / WindowCloak | The audience never sees excluded apps, even ones opened mid-share. Works on macOS 15+ because filtering happens in their own ScreenCaptureKit stream. | You have to share their proxy window every time. Needs Screen Recording. Cloak is unsigned and unlicensed, WindowCloak's repo returns 404, both are new. | Onboarding recommends window-level sharing. We don't compete on in-stream filtering. |
| 10 | Invisiwind / WinHider / HideMyWindows | The window stays visible to you and invisible to capture. HideMyWindows has rules by process and regex. | DLL injection triggers antivirus false positives. Windows only. Window picking is mostly manual. Content stays open. Microsoft says exclusion is not a security guarantee. | Confirms that users think in process and regex rules. We refuse injection. |
| 11 | Boss Key (MindGems) | Instant. Also hides windows created while hide mode is on. Covers mute, taskbar, tray and desktop icons. | Manual only, hides instead of closing, no content awareness, the demo nags. | "Hide new windows while active" becomes our share-guard mode. |
| 12 | PanicMode | macOS and Windows, multi-display, decoy workspaces, FR/DE/ES. Same Tauri stack as ours. | Covers content instead of removing it. Mechanism and licence unclear. | Shows that Tauri works for this class of app. A decoy is not our model. |
| 13 | Hammerspoon / Keyboard Maestro / EventScripts / Shortcuts | Can build any trigger and action. | Every user builds it from scratch. Hammerspoon's caffeinate watcher uses private APIs. No ready-made content rules. | Our CLI (`bezartam panic`, `bezartam explain`) lets power users wire us into these tools. |
| 14 | Stealth Browser Assistant | Hides tabs in a stash window without killing them. Keyword matching runs locally. | Proprietary, about 1 daily user, closing tabs costs money, no automatic triggers, needs access to all sites. | A stash window as "hide" for tabs in the v0.4 extension. |
| 15 | Panic-button extensions | No setup, instant, decoy safe page. Hide!!! scrubs history so the address bar doesn't autocomplete sensitive URLs. Panic Button has 10K+ users. | All-or-nothing, or hand-picked tabs. No triggers. Restore sessions sit in plain extension storage. Mostly closed source. | Opt-in history scrub in the extension. Never keep a plaintext restore list. |
| 16 | Share-blur extensions | Blur PII inside pages (emails, card numbers) and tab titles. BlurrySite reacts to browser-started shares on its own. | Only see shares started from a web page in that browser. Blur instead of close. Tiny user bases. DataBlur collects usage stats unless you opt out. | The `getDisplayMedia` hook as an extra signal in the v0.4 extension. In-page PII blur stays out of scope. |
| 17 | AutoScreenShield | Domain list, title masking, and moving tabs to a minimized window on web-meeting share. | Chrome only, browser meetings only, little traction, details unverified. | Confirms the design on the tab side. |
| 18 | Tab Wrangler / Auto Tab Discard | Mature and active. Good lock and allowlist UX. Protects pinned, audible and grouped tabs. The Corral restores closed tabs. | Works on inactivity, not sensitivity. The Corral is a readable list of closed URLs. Browser only. A discarded tab still shows its title and favicon. | Protect audible and pinned tabs by default. Restore UI. Keep the restore list in memory only. |
| 19 | Incognito Killer | Tiny, and does exactly one job: close incognito tabs on lock. | Incognito only, Chrome only, no rules, stale. | A lock trigger plus a `private` matcher. |
| 20 | LeechBlock NG | The richest open rule engine: 30 block sets, wildcards, keywords, time windows, lockdown, password-protected options, and blur/close actions. Well loved. | Acts at navigation time and is built for productivity. No share, wake or lock triggers. No sensitive-category lists. | Rule-editor UX and a per-rule choice of action. |

## Built-ins we integrate with instead of duplicating

| Built-in | What it covers | Our stance |
|---|---|---|
| macOS window/app sharing picker (Sonoma+), presenter preview and on-the-fly window add/remove (Sequoia), selective AirPlay/HDMI mirroring (15.2) | Sharing only what you choose | Onboarding recommends sharing a window instead of the whole screen. That is the cheapest real protection. |
| macOS "Allow notifications when mirroring or sharing the display" | Notification previews during a share | We don't toggle Focus (no public API). Onboarding opens the setting. Our own notifications never contain titles or URLs. |
| Safari Locked Private Browsing (Safari 17+) | Private windows lock on lock, sleep or screensaver | Validates our wake/lock trigger. We say plainly that Safari private windows protect themselves, and we cannot tell them apart over AppleScript. |
| macOS Tahoe Shortcuts automations | DIY triggers (display connected, app opened) | Users can call our CLI from a Shortcut. |
| Windows automatic DND when duplicating the display; taskbar "Share this window" | Projector notifications; window sharing | Recommended in onboarding. Nothing in Windows closes content. |
| Windows `WDA_EXCLUDEFROMCAPTURE` | Hides **your own** windows from capture | Applied to our own settings window. We never inject it into other processes. |
| KDE Plasma 6.6 per-window capture exclusion and DND while sharing | Manual window exclusion on Wayland | Mentioned in Linux onboarding. It complements us, because it excludes but doesn't close. |
| Zoom / Teams / Meet window or tab sharing; Chrome hides web-notification content while sharing | Choosing what to share | Recommended. People still share the whole screen for slides, demos and school projectors, and that is where we matter. |

## Gaps nobody fills

1. **Content-aware cleanup.** Nothing decides by URL domain, file path or type, or title category. Everything else acts on whole apps or the whole desktop.
2. **Cross-app coverage from one rule set.** Chromium-family browsers, Safari, Firefox, Preview/Acrobat PDFs, Office, Pages/Keynote/Numbers and messenger apps. Today you need one extension per browser plus manual work.
3. **The full trigger matrix in one tool.** Share start, projector, lid/sleep/lock, idle and panic are spread across five or more products. "Locked or asleep longer than N minutes" and "sensitive item open longer than N hours" are not shipped ready-made anywhere. AutoQuit comes closest, and only for whole apps.
4. **An explicit unsaved-work policy.** Today tools either hide (content stays one click away), force-quit without saving (AutoQuit's option), or hide without restoring (Stealthly).
5. **Acting before exposure.** Nobody cleans up on will-sleep, lid close or lock, or does a pre-flight sweep when a call starts, so they all race the first shared frame.
6. **Cross-platform, open, offline, auditable.** The permissions this kind of tool needs (Accessibility, Automation) demand trust. The open-source options are tiny, unsigned, unlicensed or rely on DLL injection, and the commercial ones are closed (one ships Sentry).
7. **A maintenance vacuum.** Muzzle (2021), Hocus Focus (2021, Intel-only), Quitter (only re-signed), Clicky Gone (2013) and Pliim (no releases) still get recommended.
8. **Linux, especially Wayland.** There is close to nothing beyond KDE's built-ins.
9. **Curated offline categories** for banking, adult, webmail, medical, password managers, messengers and invoices.
10. **Hungarian, and the student/classroom audience.** Competitors target developers, streamers and sales calls.

## Positioning

> For students and professionals who share their screen in classes, meetings and on projectors, **Bezártam? (Did I close it?)** is a free, open-source desktop utility. It finds the sensitive tabs, documents and apps you left open and closes or hides only those, before the share starts or the lid opens, following a rule you choose for unsaved work. Presentation modes hide everything, and capture-exclusion tools depend on fragile APIs or DLL injection. Bezártam? acts on content (domain, file, app), runs on macOS, Windows and Linux with one rule format, and makes no network connections, which anyone can verify in the code.

Short form for the README and store listings: **"The pre-flight privacy check for your screen. It closes the bank tab and the invoice PDF and leaves your slides alone."**

## Differentiators

| # | Differentiator | Why it is hard to copy |
|---|---|---|
| 1 | **Content-aware.** Closes the banking tab and the payslip PDF, leaves the slides and the meeting alone. | Needs a live inventory across browsers, documents and apps through accessibility and automation APIs on three OSes. Single-purpose tools would have to be rebuilt to do this. |
| 2 | **One rule set across four target kinds**: browser tab, document window, app window, process. | The domain model and the adapter traits are the core of the codebase, not a bolt-on. |
| 3 | **Full trigger matrix**, including "away longer than N minutes" and "open longer than N hours". | Needs native lock, sleep, lid and idle plumbing on every OS. Browser extensions cannot do this (Firefox never reports "locked", Safari has no idle API). |
| 4 | **Acts before exposure.** Cleans up on will-sleep, lid close and lock. Runs a pre-flight sweep when a call is likely. Keeps a share-guard mode running for the whole share. | It is a design choice that competitors built on masking can't adopt without a rewrite. |
| 5 | **Per-rule unsaved-work policy** (hide / save then close / discard / skip and notify). Defaults to hide. Never force-quits. Never drives a Save panel. | Needs per-app knowledge (AppleScript dictionaries, Office COM, AX and UIA dialog handling). |
| 6 | **Verifiably offline.** No network code, enforced in CI by dependency bans, lints and a syscall-level test. No telemetry, accounts or crash reporter. | Closed competitors can only say "trust us". |
| 7 | **Restore after the meeting** without writing a plaintext list of sensitive URLs to disk. | Every panic tool and the Tab Wrangler Corral store that list in plaintext. |
| 8 | **macOS, Windows and Linux** (Wayland later) with one config file. | Every competitor runs on a single OS, except PanicMode, which only covers the screen. |
| 9 | **Hungarian first**, i18n-ready, designed with the classroom projector in mind. | No competitor ships Hungarian. |
| 10 | **Honest capability report.** Shows what it can and cannot see on this machine (for example "Firefox: titles only, no per-tab close"). | Most tools fail silently. Showing this builds trust and cuts support load. |
| 11 | **No DLL injection and no Screen Recording permission.** | Fewer antivirus flags on Windows, no monthly re-consent prompts and no purple capture indicator on macOS. |

## Where we are weaker, and the response

| Weakness | Response |
|---|---|
| Detection is reactive: the first 0.2–1.5 s of a share can go out before cleanup finishes. | Pre-flight sweep on "call likely" (meeting app using mic or camera, or the share picker opening on Windows), cleanup on lock and sleep, the panic hotkey, and an honest latency figure published only after we measure it. |
| macOS share detection relies on a private SkyLight symbol. | Load it with `dlsym` and fail open. Public fallbacks (display mirroring, mic/camera in use). The UI shows when the signal is unavailable. |
| We can't keep a window visible to you and hidden from the audience (Unshown and Invisiwind can). | By design. Onboarding recommends window sharing. |
| macOS needs Accessibility plus per-app Automation consent, and there will be no Mac App Store build. | Ask for every consent up front in onboarding, never prompt during a share, ship a notarized Developer ID build and a Homebrew cask. |
| Until the v0.4 extension: Windows and Linux see background-tab titles only, and Firefox on macOS is hide-only. | Match on titles, cache URLs when a tab becomes active, and say so in the capability report. |
| No in-page PII blur and no desktop-icon hiding in v0.x. | Out of scope. We point to OS settings. |
| A solo student project against funded products, in a category whose tools died of neglect. | Keep the scope tight, keep the core pure and tested, ship few dependencies, and publish a test matrix per release. |
| Windows antivirus heuristics: enumerating windows and reading browser URLs looks like spyware. | Signed builds (Microsoft Store / SignPath), no injection, no network, public source, and submitting builds to Microsoft for analysis. |

## Claims we will and won't make

- **Will:** "acts on content", "works offline, verifiably", "best-effort screen-share detection", "cleans up before you lock or sleep". Latency figures only after we measure them.
- **Won't:** "guaranteed", "invisible to screen sharing", "never leaks", "AI-powered" (the optional v0.6 image classifier is a local model, off by default). We won't promise detection we can't do: X11 screen capture can't be detected, and on Windows, DXGI and capture-card capture can't either.
