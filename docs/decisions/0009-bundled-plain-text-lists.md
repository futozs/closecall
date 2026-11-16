---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# Bundled plain-text lists, no runtime downloads

## Context and Problem Statement

Categories such as adult, banking and webmail need domain lists with tens of thousands of entries.
Ad blockers fetch such lists at runtime. We cannot ([0002](0002-no-network-ever.md)), and the lists
must stay reviewable and properly licensed.

## Considered Options

- Snapshot lists into the repo as plain text, compile them into the app, update only with releases
- Download lists at runtime from upstream
- Ship a binary blob produced outside the repo

## Decision Outcome

Chosen option: "Plain-text snapshots in `lists/`, compiled in", because every change is a reviewable
diff, the licence of each source is recorded next to it, and the app never fetches anything.

- `lists/` holds one domain per line per source, plus `lists/manifest.toml` with source URL, licence,
  snapshot date, SHA-256 and entry count.
- `crates/lists` turns them into built-in categories at build time; it may compress them, but the plain
  text stays the source of truth.
- A dev-time script or CI job refreshes snapshots and opens a PR with a diff summary. Downloading is
  allowed there, never in the app.
- Users can import their own list files (hosts or plain-domain format) through a file dialog.

### Consequences

- Good, because no network, and nothing changes behind the user's back.
- Good, because licence attribution is complete and visible (About → Licences).
- Bad, because lists age between releases.
- Bad, because the repo and the binary grow; each category has a size budget, and very large sources
  (UT1 adult, about 4.6M entries) are excluded.

### Confirmation

A test in `crates/lists` checks the manifest against the files (hash, count, licence present) and the
size budget. The `add-category` skill covers licence checks for new sources.

## More Information

[architecture-recommendation.md §4.5](../research/architecture-recommendation.md), [competitors-browser.md](../research/competitors-browser.md).
