---
name: add-category
description: Add or change a built-in sensitive category (adult, banking, webmail, medical, password_managers, messengers, invoices, meeting, ...) or a domain-list source behind one. Use when a task touches lists/, lists/manifest.toml, crates/lists, or the built-in category definitions, including adding Hungarian banks, app bundle ids, title keywords or file globs.
---

# Add a category or list source

1. **Check the licence first.** Acceptable: MIT, BSD, Apache-2.0, CC0, CC BY 4.0, CC BY-SA 4.0 (one-way
   into GPL-3.0; still to be confirmed before the first release), GPL-3.0. Reject anything unlicensed,
   NonCommercial, or without upstream attribution. If unsure, stop and ask the owner.
2. **Snapshot, never fetch at runtime** ([ADR-0009](../../../docs/decisions/0009-bundled-plain-text-lists.md)).
   Download in a dev-time script, normalize to one lowercase, punycoded domain per line, sort,
   deduplicate, and commit the plain-text file under `lists/`.
3. **Record it in `lists/manifest.toml`**: id, name, source URL, licence, snapshot date, SHA-256 of
   the committed file, entry count. The app's About screen and `ListSource` in `crates/core/src/ipc.rs` show these.
4. **Wire the category** in `crates/lists`: domains via list ids, plus `apps` (bundle id, exe, desktop
   id), `title_keywords` (they are folded, so write `szamla` or `Számla`, both match) and `file_globs`.
   Hand-curated seeds (for example Hungarian banks) are ours; check each domain by hand.
5. **Tests in `crates/lists`**: a few hit and miss domains (a subdomain of a listed domain must hit),
   the manifest matches the files (hash, count, licence present), and the size budget still holds.
   Very large sources (millions of entries) are out.
6. **Strings**: a new category id needs a display name in `locales/en` and `locales/hu`.
7. **Docs**: the category list in [docs/config.md](../../../docs/config.md) and, if it is on by default,
   the README's rule list.

Run `cargo test -p closecall-lists` and `cargo deny check`. Never add a category to `protect` defaults
without the owner.
