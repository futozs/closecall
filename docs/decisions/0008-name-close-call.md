---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# The name "Close Call"

## Context and Problem Statement

The working title was „Bezártam?” ("Did I close it?"), which English speakers cannot pronounce. The
project needs a name that works in English, in a CLI and as a repo slug, and that is free on the
usual registries.

## Considered Options

- Close Call (`closecall`)
- Coast Clear (`coastclear`)
- Did I Close? (`didiclose`)
- Bezártam (`bezartam`)

## Decision Outcome

Chosen option: "Close Call", because it carries three meanings: *close* is what the app does, *call*
is the meeting that triggers it, and *a close call* is the moment you nearly projected the open tab.
It is short, has no accents, and the slug was free on crates.io, PyPI and Homebrew on 2026-11-16.

- Repo `futozs/closecall`, CLI `closecall`, crates `closecall-*`, bundle id `hu.futozs.closecall`.
- The Hungarian UI keeps „Bezártam?” as a subtitle.

### Consequences

- Good, because one memorable word pair in both languages' UI.
- Bad, because `closecall` is taken on npm (use `close-call` or a scoped name if a package is ever needed).
- Bad, because it can read as a call-blocking app at first glance; the README's first sentence fixes that.
- Open: no trademark search yet; run an EUIPO search in class 9 before a public release.
- The bundle id is permanent: macOS TCC grants are keyed to it.

## More Information

[naming.md](../research/naming.md).
