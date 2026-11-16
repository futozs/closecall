---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# GPL-3.0-or-later

## Context and Problem Statement

The value of a privacy tool is that people can trust what it does. Which licence keeps that true for
redistributed builds, and lets us bundle the domain lists we need?

## Considered Options

- GPL-3.0-or-later
- MPL-2.0
- MIT / Apache-2.0

## Decision Outcome

Chosen option: "GPL-3.0-or-later" for all code, because anyone who redistributes a modified build,
for example one with telemetry added, must publish the complete source. It also lets us bundle
GPL-3.0 lists (HaGeZi) and learn from GPL code (Tab Wrangler, AutoQuit, BlurrySite, KeePassXC).
Bundled lists keep their upstream licences, recorded in `lists/manifest.toml` and shown in the app.

### Consequences

- Good, because forks cannot go closed and add tracking.
- Good, because the usual objection, the Mac App Store, does not apply: Accessibility is unsupported
  in the App Sandbox, so there is no App Store build anyway.
- Bad, because companies cannot embed `closecall-core` in closed products. If that is ever wanted,
  relicense that crate before accepting outside contributions to it.
- Open: CC BY-SA 4.0 lists (UT1) are believed one-way compatible into GPLv3; confirm with a licence
  FAQ before the first release.

### Confirmation

`license = "GPL-3.0-or-later"` in the workspace `Cargo.toml`; `cargo-deny` enforces a licence
allowlist for dependencies; list licences are checked when a source is added (`add-category` skill).

## More Information

[architecture-recommendation.md §1 (decision 11)](../research/architecture-recommendation.md),
[repo-layout.md §3.5](../research/repo-layout.md), [LICENSE](../../LICENSE).
