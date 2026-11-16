# Configuration

Close Call reads one TOML file. The settings window edits the same file, so hand edits and UI edits
are interchangeable. The types are defined in `crates/core/src/config.rs` and `crates/core/src/model.rs`;
the JSON Schema generated from them is [schema/config.schema.json](../schema/config.schema.json).

## Where it lives

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/hu.futozs.closecall/config.toml` |
| Windows | `%APPDATA%\hu.futozs.closecall\config.toml` |
| Linux | `$XDG_CONFIG_HOME/hu.futozs.closecall/config.toml` (usually `~/.config/…`) |

The file may name your bank or doctor. It stays on your machine; treat it like any private document.

Editors that understand JSON Schema for TOML (Taplo, Even Better TOML; support for the directive
varies by version) can validate it if you add this as the first line:

```toml
#:schema https://raw.githubusercontent.com/futozs/closecall/main/schema/config.schema.json
```

The editor fetches that URL, not Close Call. Point it at a local copy of the schema if you prefer.

## Top level

| Key | Type | Default | Meaning |
|---|---|---|---|
| `version` | integer | `1` | Config format version. A file newer than the app understands is refused, not guessed at. |
| `language` | `"system"`, `"en"`, `"hu"` | `"system"` | UI language. |
| `panic_hotkey` | string | `"Ctrl+Alt+Shift+B"` | Global hotkey for the panic trigger (⌃⌥⇧B on macOS). |
| `share_min_confidence` | `"low"`, `"medium"`, `"high"` | `"medium"` | Weakest screen-share signal that fires `screen_share` rules. Low signals only start the pre-flight check. |
| `untitled_save_dir` | path | `"~/Documents/Close Call"` | Where never-saved documents are saved under `save_then_close`, when the app allows it. |
| `restore_window` | duration | `"2h"` | How long closed items can be restored. The list is kept in memory only. After this, Activity keeps what happened but forgets titles, URLs, paths and the matching evidence. |
| `[protect]` | table | see below | Things never touched, whatever the rules say. |
| `[ignore]` | table | see below | Screen capture and displays that never count as a share or a projector. |
| `[[categories]]` | array of tables | none | Your own categories, next to the built-in ones. |
| `[[rule]]` | array of tables | none | The rules. |

Durations are a whole number followed by `s`, `m`, `h` or `d`: `90s`, `15m`, `2h`, `1d`.

## `[protect]`

Protection always wins over every rule.

| Key | Type | Default |
|---|---|---|
| `categories` | category ids | `["meeting"]` (Zoom, Teams, Meet, FaceTime, Webex, Discord, OBS and their sites) |
| `apps` | app selectors | `[]` |
| `domains` | domains (a domain covers its subdomains) | `[]` |
| `audible_tabs` | bool | `false` |
| `pinned_tabs` | bool | `false` |

The app that is capturing the screen, when the OS tells us, and Close Call's own windows are always protected.

## `[ignore]`

`protect` spares items from a run; `ignore` keeps the run from starting. Settings has an "Ignored
capture and displays" section that adds entries from the running apps and the connected displays.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `capture_apps` | app selectors | recommended preset: `[{ bundle_id = "com.displaylink.DisplayLinkUserAgent" }]`; empty preset: `[]` | Screen capture by these apps is not a share. |
| `displays` | display selectors | `[]` | These displays are never a projector, mirrored or not. |

Some apps capture the screen for as long as they run, so the OS reports a share all day. DisplayLink
Manager does this to drive the monitors on a DisplayLink USB dock. While an app from `capture_apps`
runs, a capture the OS cannot attribute to an app is put down to it and ignored. macOS cannot tell a
second capture apart while the first one holds the signal, so shares are then recognised from the
sharing bars of Zoom, Teams, Slack, Webex, Discord and Chromium browsers instead. Remote-access
agents such as AnyDesk, Splashtop or TeamViewer are not in the preset on purpose: they capture only
during a remote session, and a remote session is a real share.

A display selector is either part of the name, matched case- and accent-insensitively, or the
display's EDID vendor and model codes, optionally with its serial number:

```toml
[ignore]
capture_apps = [{ bundle_id = "com.displaylink.DisplayLinkUserAgent" }]
displays = [
  { name = "LG HDR" },                                   # every display whose name contains this
  { id = { vendor = 7789, model = 23481 } },             # every display of this model
  { id = { vendor = 4675, model = 2, serial = 16843009 } }, # this one unit
]
```

`vendor` must be 1–65535: EDID packs the three-letter manufacturer code (`GSM` for LG) into 16 bits,
and anything else is the OS saying it does not know. A display mirrored from the moment it was
connected has no name on macOS, so use its codes.

## `[[categories]]`

| Key | Type | Meaning |
|---|---|---|
| `id` | string | Used in `{ category = "…" }`. |
| `name` | string, optional | Shown in the UI. |
| `domains` | strings | Host suffixes: `otpbank.hu` matches `netbank.otpbank.hu`. |
| `lists` | strings | Ids of bundled or imported domain lists. |
| `apps` | app selectors | Apps that belong to the category. |
| `title_keywords` | strings | Matched case- and accent-insensitively: `szamla` matches `Számla`. |
| `file_globs` | strings | Paths of documents: `~/Documents/Bank/**`. |

Built-in categories: `adult`, `banking`, `webmail`, `medical`, `password_managers`, `messengers` and
`invoices` (on in the recommended preset); `financial`, `dating` and `gambling` (available, off);
`meeting` (protected). Their sources and licences are listed in [lists/manifest.toml](../lists/manifest.toml).

## `[[rule]]`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Unique. Shown in "why was this closed?". |
| `name` | string | none | Display name. |
| `enabled` | bool | `true` | |
| `when` | triggers | required | Any of these arms the rule. |
| `targets` | `"tab"`, `"document"`, `"app_window"`, `"process"` | required | Kinds of things the rule may act on. |
| `match` | matcher | required | What counts as sensitive. |
| `except` | matcher | none | Matches here are left alone. |
| `action` | action | required | What to do. |
| `on_unsaved` | unsaved policy | `"save_then_close"` | What to do with unsaved documents when the action is `close` or `quit`. |
| `confirm` | `"never"` or `{ countdown = "60s" }` | `"never"` | Wait this long before acting. For an `idle_longer_than` rule, keyboard or mouse input during the wait calls it off. There is no "Keep" or "Do it now" button yet, and no notice while it waits. |

### Triggers

| Value | Fires when |
|---|---|
| `"panic"` | The hotkey, the tray item or `closecall panic`. |
| `"screen_share"` | A screen share or recording starts (best effort, per OS). |
| `"display_mirrored"` | A projector or mirrored display is connected, unless it is in `ignore.displays`. |
| `"display_connected"` | An external display that is not in `ignore.displays` is connected, mirrored or not. Not in the recommended preset: a desk monitor would set it off every day. While such a display stays connected, share-guard keeps acting on its rules. |
| `"call_likely"` | A meeting app uses the mic or camera, or a share picker opens. Meant for notify-only pre-flight rules. |
| `"will_sleep"` | The machine is about to sleep. |
| `"lid_closed"` | The lid closes. |
| `"locked"` | The screen locks. |
| `{ away_longer_than = "15m" }` | Locked or asleep for this long. Fires before unlock when the OS allows it. |
| `{ idle_longer_than = "10m" }` | No input for this long. |
| `{ item_open_longer_than = "2h" }` | A matching item has been open this long. Age restarts when Close Call restarts. |

### Matchers

| Matcher | Example |
|---|---|
| `category` | `{ category = "banking" }` |
| `domain` | `{ domain = "otpbank.hu" }` |
| `url_glob` | `{ url_glob = "https://mail.google.com/mail/u/1/*" }` |
| `title_contains` | `{ title_contains = "bérjegyzék" }` |
| `app` | `{ app = { bundle_id = "com.apple.MobileSMS" } }`, also `exe`, `desktop_id`, `name` |
| `file_glob` | `{ file_glob = "**/*bérjegyzék*.pdf" }` |
| `doc_kind` | `{ doc_kind = "pdf" }`: `pdf`, `image`, `word`, `excel`, `power_point`, `pages`, `keynote`, `numbers`, `text`, `other` |
| `private` | `"private"`: a private or incognito window, where the browser lets us tell (Chromium yes, Safari no) |
| `any`, `all` | `{ any = [{ category = "adult" }, "private"] }` |
| `not` | `{ not = { domain = "docs.google.com" } }` |

### Actions

| Action | Effect |
|---|---|
| `"close"` | Close the tab or document window. |
| `"hide"` | Conceal without closing: app hide on macOS, minimize on Windows and Linux. |
| `"minimize"` | Minimize the window. |
| `"quit"` | Quit the app after handling its documents. Never in a preset; never a force quit. |
| `"notify_only"` | Tell you, touch nothing. |

If the platform cannot do the action for that target, Close Call falls back only toward less
destructive actions: close → hide → minimize → notify only.

### Unsaved policies

| Policy | Clean document | Unsaved, has a file | Unsaved, never saved |
|---|---|---|---|
| `"save_then_close"` (default) | close | save through the app, then close | save into `untitled_save_dir` where the app allows it, otherwise hide |
| `"hide"` | close | hide and notify | hide and notify |
| `"skip_and_notify"` | close | leave it open, notify | leave it open, notify |
| `"discard"` | close | close without saving | hide, unless you also opted in for never-saved documents |

When the app cannot say whether a document has unsaved changes, it counts as unsaved. If a save fails
or an unexpected dialog appears, Close Call cancels, hides the document and tells you. It never fills
in a Save dialog. See [ADR-0007](decisions/0007-default-unsaved-policy.md).

When several rules hit the same item, the strongest action wins (quit > close > hide > minimize >
notify only) and the most careful unsaved policy wins (hide > skip_and_notify > save_then_close > discard).

## Validation

The app rejects a config with problems instead of running half of it, and lists each problem with
the rule and field. For example, `confirm = { countdown = … }` is allowed only on rules armed by
`idle_longer_than` or `item_open_longer_than`: on a share, a projector, the lid or a lock there is no
time to wait.

If `config.toml` can't be read or parsed, was written by a newer version, or has problems, Close Call
leaves the file as it is, runs the recommended preset instead, and says so in a banner at the top of
the settings window (and in `closecall diag`). Fix the file and restart, or change something in the
window: the first save then keeps your file as `config.rejected.toml` next to the new `config.toml`.

## Examples

The rules from the README, plus a few more:

```toml
version = 1
language = "hu"

[protect]
categories = ["meeting"]
apps = [{ bundle_id = "com.apple.Keynote" }]
domains = ["docs.google.com"]
pinned_tabs = true

[[categories]]
id = "my_bank"
name = "My bank"
domains = ["otpbank.hu"]
title_keywords = ["internetbank"]

[[rule]]
id = "payslips-on-lid-close"
when = ["lid_closed", "will_sleep"]
targets = ["document"]
match = { all = [{ doc_kind = "pdf" }, { file_glob = "**/*bérjegyzék*.pdf" }] }
action = "close"

[[rule]]
id = "bank-tab-open-too-long"
when = [{ item_open_longer_than = "2h" }]
targets = ["tab"]
match = { category = "my_bank" }
except = { url_glob = "https://www.otpbank.hu/portal/hu/*" }
action = "close"
confirm = { countdown = "60s" }

[[rule]]
id = "messages-when-away"
name = "Hide Messages after 30 minutes away"
when = [{ away_longer_than = "30m" }]
targets = ["app_window"]
match = { app = { bundle_id = "com.apple.MobileSMS" } }
action = "hide"
```

Try a rule without risk: the settings window has a dry run ("this is what I would do right now"), and
`closecall explain --trigger screen_share` prints the plan without touching anything.
