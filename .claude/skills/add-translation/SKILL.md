---
name: add-translation
description: Add a new UI language, or add or change user-visible strings in English and Hungarian. Use when a task touches locales/, adds a message key that Rust or the UI emits, changes onboarding, tray or notification text, or edits Hungarian copy.
---

# Add a language or strings

Strings live in `locales/<lang>/ui.json` (React, i18next) and `locales/<lang>/native.json` (tray menu
and notifications, embedded by `app/src-tauri`). English is the reference; every key exists in every
language.

## New or changed strings

1. Rust never builds prose. Emit a key plus parameters (`reason_key`, `note_key`, `ValidationIssue.code`)
   and add the text to `locales/en` and `locales/hu` in the same change.
2. Placeholders use i18next syntax, `{{count}}`. Plurals use `_one` / `_other` suffixes through
   `Intl.PluralRules`. Never glue case endings onto a placeholder in Hungarian (vowel harmony):
   write „Bezárva: {{count}} lap”, not „{{app}}-ban”.
3. Notification text never contains a title, URL or path. Counts only.

## New language

1. Copy `locales/en/` to `locales/<code>/` and translate every value; keep keys and placeholders.
2. Add the variant to `Language` in `crates/core/src/config.rs`. That is a contract change: regenerate
   bindings (`cargo test -p closecall-core export_bindings`) and the schema, and update `docs/config.md`.
3. Add `app/src-tauri/<code>.lproj/InfoPlist.strings` for the macOS permission text.
4. Run the tests; the parity check fails on missing or unused keys.

## Hungarian copy rules

- Write it as a native speaker would say it, not translated English. Read it aloud.
- „…” quotes, correct ő and ű (never õ or û), Hungarian capitalization (only the first word of a title).
- Singular noun after a numeral: „3 lap”, „2 dokumentum”.
- Pick tegezés or magázás once for the whole UI and keep it.
- Keep button labels short; check every screen at the minimum window size (720×520) in Hungarian.
