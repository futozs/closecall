# Browser-side landscape for "Did I close it?" / "Bezártam?"

Research date: 2026-11-16. Store numbers come from the official AMO API (Firefox), GitHub API (`gh api`), Hugging Face API and chromeboard.com, a third-party Chrome Web Store mirror. The Chrome Web Store itself redirected to a Google consent wall from Hungary, so Chrome user counts and dates are second-hand. The UT1 official site returned HTTP 503, so UT1 figures come from its daily GitHub mirror. Anything marked "unverified" was not confirmed against a primary source.

## Bottom line

1. No existing product does what the brief describes. Browser tools split into separate niches: panic buttons that close everything, inactivity closers, blur-on-share tools, incognito killers, and content filters. None of them combines **sensitive categories × automatic triggers × close/hide/save actions**, and none reaches outside the browser to PDFs, Office files or apps.
2. The main trigger, screen sharing, can't be detected reliably from a browser extension. Every share-aware extension (BlurrySite, DataBlur) wraps `getDisplayMedia` in the page, so it only sees shares started from a web page in that browser. Zoom, Teams, Discord and OBS desktop apps, and a projector, are invisible to it. Wake, lid and lock detection is uneven too: Chrome's `idle` API reports `locked`, Firefox's never does (Bugzilla 1395199, 1299775), and Safari has no `idle` WebExtension API (MDN browser-compat-data). **So a native desktop app has to own the triggers.** A browser extension at most acts as an enumerator and executor inside the browser.
3. The closest browser-side prior art:
   - **Stealth Browser Assistant**: keyword/domain matching plus a panic hotkey that hides tabs in a stash window. Proprietary, with about one user.
   - **BlurrySite**: GPL-3.0, blurs other tabs automatically on share.
   - **LeechBlock NG**: MPL-2.0 rule engine that already has 'blur instead of block' and 'close tab instead of block' actions.
   - **Tab Wrangler**: GPL-3.0, time-based closing with lock and allowlist UX.

## Product map (by category)

| Category | Products | Open source? | Auto triggers |
|---|---|---|---|
| Panic button | Hide Tabs (Panic Button), Panic Button (Ctrl/Cmd+Shift+Y), Hide!!!, Stealth Browser Assistant | Only Hide Tabs declares MPL-2.0 (no repo linked) | None, hotkey only |
| Auto-close / discard | Tab Wrangler (GPL-3.0), Auto Tab Discard (MPL-2.0), Safari "Automatically close tabs" (1 day/week/month), Chrome Android Tab Declutter | Yes / built-in | Inactivity only |
| Screen-share blur | BlurrySite (GPL-3.0), DataBlur, Safe Screen Share, Streamer Mode (AGPL-3.0), BCBlur (MIT claimed, no repo, 0 users) | Mixed | Only shares started from the browser (getDisplayMedia hook), or meeting URL (Safe Screen Share, mechanism undocumented) |
| Incognito auto-close / lock | Incognito Killer (screen lock/restart), Chrome Incognito Killer (2016, 15-min idle), Safari Locked Private Browsing, Firefox Android 140/141 private lock, Chrome iOS Incognito lock | No / built-in | Lock, sleep, screensaver, app switch |
| Blockers | LeechBlock NG (MPL-2.0, 114k Firefox daily users) | Yes | Time-based only |
| On-device NSFW | NSFWJS, NSFW Filter, Wingman Jr., HaramBlur, NudeNet, Falconsai ViT, Marqo-384, Apple SensitiveContentAnalysis | Mostly yes | n/a |

Detail per product, including version, date, licence, strengths and weaknesses, is in the structured `products` array.

## Technical findings that shape the design

- **getDisplayMedia hook (BlurrySite `src/main_world_bridge.js`)**: wraps `MediaDevices.prototype.getDisplayMedia` in the MAIN world, then listens for track `ended` and stream `inactive`. This only works for browser-based meetings. A companion extension could use the same hook as an extra signal, and the native app would cover native capture.
- **chrome.idle**: states are `active`, `idle` and `locked`. The default threshold is 60 s and `getAutoLockDelay` is ChromeOS-only. Incognito Killer relies on `locked`.
- **Firefox idle**: never emits `locked` (Bugzilla 1395199 is still open), so lock and wake must come from the native side on Firefox.
- **Safari**: no `idle` API in Safari WebExtensions (MDN BCD). Safari's own locked private browsing is the UX to imitate: lock on Mac lock, sleep or screensaver, or after Safari has been inactive for a while; Touch ID to unlock; skip while media plays.
- **Desktop Chrome has no built-in redaction during screen share**: Google only experimented with an Android flag in 2024 that redacts sensitive form fields while sharing.
- **Leaks the existing tools miss**: tab titles during load (Streamer Mode's known issue), the URL bar and its autocomplete history (only Hide!!! and the paid tier of Panic Button scrub history), and saved restore lists. Tab Wrangler's Corral and the panic tools' sessions are plain-text lists of the sensitive URLs.

## Open domain lists (adult and other sensitive categories)

| List | Licence | Size | Fit |
|---|---|---|---|
| StevenBlack porn-only | MIT repo; upstreams MIT plus CC BY 4.0 (tiuxo, brijrajparmar27), so attribution is needed | 76,811 domains | Small and permissive; good default to bundle |
| HaGeZi NSFW | GPL-3.0 | 83,101 | Very active; copyleft |
| OISD NSFW / NSFW small | GPL-3.0 | not published; "small" = also in Tranco top 1M | Small variant fits bundling; no plain hosts or domains format since 2024-01-01 |
| Bon-Appetit porn-domains | CC BY-SA 4.0 | 433,847 lines + 15,109 allowlist | Large; the allowlist helps with false positives |
| UT1 Toulouse | CC BY-SA 4.0 | adult 4.6M; **bank 6,646; webmail 413; financial 473; dating 10,564** | Only open list with bank and webmail categories; adult set is too big to bundle whole |
| Block List Project porn | Unlicense | n/a | **Archived 2026-10-27**, a frozen snapshot |
| Shallalist | n/a | n/a | **Dead since about 2023**; tools have migrated to UT1 or OISD |

No open, licensed **medical/health** domain list turned up, so medical sensitivity has to come from user rules, keywords in title or URL, or a small hand-curated seed list.

Licensing note: shipping GPL-3.0 lists (HaGeZi, OISD) inside the app pushes the app towards GPL-3.0. CC BY-SA 4.0 is generally treated as one-way compatible with GPLv3, but I did not re-verify that here. Because the app can't make network calls, lists must be bundled at build time (CI snapshot) and refreshed only through app releases or a file the user imports. That tension deserves a decision.

## On-device NSFW classification

- **NSFWJS** (MIT, v4.4.0, 2026-08-04): the easiest fit for a TypeScript stack. About 2.6-3.5 MB, 5 classes, claimed accuracy about 90-93%. Bundle the weights yourself, because the README says the CDN copy moved over hotlinking.
- **Apple SensitiveContentAnalysis** (macOS 14+): needs the `com.apple.developer.sensitivecontentanalysis.client` entitlement, and the policy is `.disabled` unless the user has turned on Sensitive Content Warning or Communication Safety. Use it as an optional macOS backend, not the default.
- **ViT options under Apache-2.0**: Falconsai/nsfw_image_detection (1.58M downloads, updated 2026-09) and Marqo nsfw-image-detection-384 (5.6M params). Both need conversion to ONNX or CoreML.
- **Copyleft or stale**: NudeNet and HaramBlur are AGPL-3.0, and HaramBlur looks stale since 2024-08. Wingman Jr. has CC0 code, an Apache-2.0 model and a private dataset. Yahoo open_nsfw is archived. Bumble and LAION have not been updated since 2023.
- No panic or auto-close tool uses a classifier to find unknown sensitive tabs. Classifiers only exist as image filters.

## Patterns to borrow

- **Bitwarden**: trigger × action matrix. Triggers: interval, immediately, on system idle, sleep, lock or restart, on browser restart, never. Actions: lock or log out. Warns about "Never".
- **KeePassXC** (exact option names): "Lock databases after inactivity of", "Lock databases when session is locked or lid is closed", "Lock databases when switching user", "Lock databases after minimizing the window", "Clear clipboard after". Its Qt/C++ lid and session-lock detection is a reference for all three OSes.
- **1Password**: the desktop app is the authority and the extension follows its settings; the extension always locks when the browser quits.
- **Banking (PSD2 RTS Art. 4(3)(d))**: at most 5 minutes of inactivity after authentication. OWASP recommends 2-5 minutes for high-value apps and notes that close handlers can't enforce logout. A sensible default is to close bank tabs after 5 idle minutes, since the session is dead anyway and the page only leaks.
- **Safari Locked Private Browsing**: lock on lock, sleep or screensaver, biometric unlock, skip while media plays.
- **Discord Streamer Mode**: turns on automatically when OBS or XSplit is detected and live. This is the precedent for the native capture trigger.
- **LeechBlock NG**: per-rule action (block, blur 1-32 px, grayscale, mute, close), with keywords matched against the page title.

## Suggested direction (for the author)

1. A native app (Tauri/Rust or Swift) owns triggers: screen capture, display mirroring or projector, wake, lid, lock duration, tab age.
2. A small companion WebExtension (Chrome, Firefox, maybe Safari) talks to the app over native messaging. It lists tabs and executes close, hide or discard-and-lock, and adds the getDisplayMedia hook as an extra signal.
3. Bundle StevenBlack porn-only (MIT/CC BY) plus UT1 bank and webmail (CC BY-SA). Offer OISD-small or HaGeZi if the app is GPL-3.0.
4. Optional NSFWJS classification of tab screenshots for unknown domains.
5. Restore lists encrypted or locked behind OS authentication, unlike every existing panic tool.
