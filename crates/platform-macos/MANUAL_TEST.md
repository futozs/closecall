# platform-macos manual test

Run before each release on macOS 14, 15, 26 and 27. Use throwaway tabs and documents in a
test folder (for example `~/Desktop/cc-test/`), never real work.

TCC attributes permissions to the responsible app: from a terminal that is the terminal, in
the bundled app it is Close Call. Test permissions with the bundled `.app`, signed with a
stable identity. Reset between runs:

```sh
tccutil reset Accessibility hu.futozs.closecall
tccutil reset AppleEvents hu.futozs.closecall
```

The two examples are the quickest check from a terminal:

```sh
cargo run -p closecall-platform-macos --example inventory            # read-only snapshot as JSON
cargo run -p closecall-platform-macos --example inventory -- --redact # same, safe to paste in an issue
cargo run -p closecall-platform-macos --example signals -- 60         # prints signals for 60 s
```

## Permissions

- [ ] Fresh install: `status()` reports Accessibility `denied`, every running browser and document app `not_determined`, apps that aren't running `app_not_running`. No prompt appears.
- [ ] `request(Accessibility)` shows the system prompt and opens Privacy & Security → Accessibility. After enabling, `status()` reports `granted` without a restart.
- [ ] `request(Automation)` for a running Chrome shows one prompt naming Close Call; Allow → `granted`, Don't Allow → `denied`.
- [ ] `request(Automation)` for an app that was denied opens Privacy & Security → Automation instead of prompting.
- [ ] `request(Automation)` for an app that isn't running returns `not_applicable` and does not launch it.
- [ ] Screen Recording and Input Monitoring are never requested (check System Settings after a full run).
- [ ] Revoke Accessibility while running: `macos.ax` snapshot fails with `PermissionMissing(Accessibility)`; Hide/Close on windows fall back to hiding the app.

## Browsers

For each of Chrome, Brave, Edge, Vivaldi, Arc, Helium and Safari: open three tabs in a normal
window and one in a private/incognito window, then run `inventory`.

- [ ] Every tab is listed with title and exact URL; the active tab has `active: true`.
- [ ] Chromium incognito tabs have `private: "yes"`, normal ones `"no"`. Arc: note what it reports.
- [ ] Safari: are private-window tabs listed at all? Record the answer; `private` is always `"unknown"`.
- [ ] Close one tab by step: only that tab closes, in the right window, including after reordering tabs.
- [ ] Safari: close a tab after closing a tab to its left (index shifted): the right tab closes.
- [ ] Safari: navigate the tab elsewhere before closing: reverify reports `Gone`, nothing closes.
- [ ] Hide: the whole browser hides instantly (`hides_whole_app`).
- [ ] Quit the browser: no Apple Event relaunches it during the next 10 s of polling.
- [ ] Without Automation for a browser: its tabs are not listed, Close falls back to Hide with `automation_missing`.
- [ ] Hung browser (pause it with `kill -STOP <pid>`): scans keep the previous tabs, Close falls back to Hide within about 2 s. `kill -CONT` afterwards.
- [ ] Firefox: tabs appear within about 15 s of opening them, `url_quality: "stale"`, private windows absent; Close hides the app (`close_unsupported`). Right after launching Firefox, tabs from the previous session are not listed.

## Documents

Run every row twice: system language English, then Hungarian (System Settings → General →
Language & Region, log out and in). The save sheet must be answered by role in both.

Apps: Preview, TextEdit, Pages, Keynote, Numbers, Word, Excel, PowerPoint, Acrobat Reader, one
non-scriptable editor (for example Visual Studio Code or Pixelmator).

- [ ] `inventory` lists a file-backed document as `document_window` with the right `path` and `doc_kind`; an untitled one as `app_window`.
- [ ] Dirty flag: unsaved edit → `dirty`; after saving → `clean` (scriptable apps) or `unknown` (others).
- [ ] Close, clean document: closes, no sheet.
- [ ] Close, dirty, `save_then_close`: changes are on disk afterwards, window gone. PowerPoint keeps the edit (it is saved before closing).
- [ ] Close, dirty untitled, `save_then_close`: TextEdit/Pages/Keynote/Numbers save to `untitled_save_dir` as `<title>-<date>`; open the file to confirm the content. Office and Preview hide instead (`untitled_hidden`), no "Grant File Access" dialog.
- [ ] Close, dirty, `hide`: nothing closes; the app hides, or only the window minimizes when the app has other visible windows.
- [ ] Close, dirty, `skip_and_notify`: nothing changes, outcome `skipped`.
- [ ] Close, dirty, `discard` (non-autosave app such as Word or the non-scriptable editor): changes are gone, window closed. Autosave apps (TextEdit, Preview, iWork) hide instead.
- [ ] Non-scriptable editor, dirty, `save_then_close`: the sheet's Save button is pressed (not Don't Save) in both languages.
- [ ] A sheet with an unexpected layout (more than three buttons) is cancelled and the window concealed (`sheet_unrecognized_hidden`).
- [ ] Two documents with the same file name in different folders: the right one closes, or the step falls back to the close button.
- [ ] Office `full name`: note whether it is POSIX or HFS on this version.

## Apps and windows

- [ ] Minimize a window: genie animation, only that window.
- [ ] Hide on the only visible window hides the app; with two visible windows only the target minimizes (`other_windows_visible`).
- [ ] Quit an app without documents (for example Spotify): it quits normally, never force-quits.
- [ ] Quit an app with an open document: it is hidden instead (`quit_with_documents_hidden`).
- [ ] Close Call's own windows and the Finder desktop never appear in the inventory.

## Signals (`--example signals`)

- [ ] Lock (⌃⌘Q) and unlock: `locked`, `unlocked`.
- [ ] Sleep from the Apple menu and wake: `will_sleep`, `did_wake`.
- [ ] Close and open the lid on a laptop: `lid_closed`, `lid_opened` (with an external display attached, too).
- [ ] Mirror to a projector, HDMI display and AirPlay: `display_mirrored` (high), then `display_unmirrored`.
- [ ] Screen share in Zoom, Teams, Meet in Chrome, Meet in Safari, Discord, Slack huddle; recording with ⌘⇧5, QuickTime, OBS; macOS Screen Sharing; Sidecar; a DisplayLink dock. Record for each whether `capture_started` fires and the delay.
- [ ] Start the example while something is already capturing and no meeting app runs: health `noisy`, signals `low`; it returns to `ok` after the capture stops.
- [ ] Join a call so the microphone turns on: `call_likely` (medium), then `call_ended`.
- [ ] A macOS where `SLSIsScreenWatcherPresent` is missing: `macos.skylight` reports `unavailable` and the other sources keep working.

## Locked screen (v0.1 spike)

- [ ] With the screen locked, do AX reads and Apple Events still work? Schedule a `simulate` run 30 s after locking and check the outcome when back.
