@AGENTS.md

## Claude Code notes

- Read the nested `AGENTS.md` for the area (`crates/core`, `crates/engine`, `crates/platform-macos`,
  `app`) before editing there.
- Project skills live in `.claude/skills/`: `add-category`, `add-provider`, `add-translation`,
  `verify-no-network`, and the user-only `release`.
- Before finishing a change to dependencies, logging, IPC, Tauri capabilities, CSP or storage, run the
  `privacy-reviewer` subagent. For `crates/platform-*` diffs, run `platform-reviewer`.
- `.claude/settings.json` allows the routine build, test and lint commands; `cargo add`, `bun add` and
  `git push` ask first. Personal overrides go in `.claude/settings.local.json` (gitignored), not here.
- Hungarian copy: the `magyar-szoveg` skill helps if you have it; the rules in `AGENTS.md` apply either way.
