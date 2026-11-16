# Decisions

Architecture decision records in [MADR 4.0](https://adr.github.io/madr/) format. They record why,
so nobody has to rediscover it from the code.

| # | Decision | Status |
|---|---|---|
| [0001](0001-tauri-2-and-react.md) | Tauri 2 with a React UI | accepted |
| [0002](0002-no-network-ever.md) | No network access, ever | accepted |
| [0003](0003-never-request-screen-recording.md) | Never request the macOS Screen Recording permission | accepted |
| [0004](0004-os-automation-before-extension.md) | OS automation for browser tabs before a browser extension | accepted |
| [0005](0005-gpl-3-or-later.md) | GPL-3.0-or-later | accepted |
| [0006](0006-toml-config-json-schema-ts-rs.md) | TOML config, generated JSON Schema, ts-rs bindings | accepted |
| [0007](0007-default-unsaved-policy.md) | Default unsaved-work policy is `save_then_close` | accepted |
| [0008](0008-name-close-call.md) | The name "Close Call" | accepted |
| [0009](0009-bundled-plain-text-lists.md) | Bundled plain-text lists, no runtime downloads | accepted |
| [0010](0010-skylight-share-detection.md) | Private SkyLight API for macOS share detection, failing open | accepted |
| [0011](0011-ignore-list-share-ui-display-connected.md) | Ignore list for always-on capture and own monitors, sharing-bar detection, `display_connected` | accepted |
| [0012](0012-logo-close-paren.md) | The logo: `close()` | accepted |

## Writing one

Write an ADR when you make or reverse a decision that someone would otherwise argue again: a new
OS permission, a new class of dependency, a new trigger type, a changed default or invariant.

1. Copy [template.md](template.md) to `NNNN-short-title.md` with the next number.
2. Fill it in. Keep it short; link to `docs/research/` for evidence.
3. Add a row to the table above.
4. To change an accepted decision, write a new ADR and set the old one's status to
   `superseded by NNNN`. Do not rewrite history.
