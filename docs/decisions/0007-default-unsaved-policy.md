---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# Default unsaved-work policy is `save_then_close`

## Context and Problem Statement

A rule may close a document that has unsaved changes. The research recommended `hide` as the default:
never close a dirty document, conceal it and notify. Hiding leaves the content one click away, and a
student who closes the lid wants the payslip gone, not minimized. What should happen by default?

## Considered Options

- `save_then_close`: save through the app, then close
- `hide`: leave it open, conceal it, notify
- `skip_and_notify`: leave it open and visible, notify
- `discard`: close without saving

## Decision Outcome

Chosen option: "`save_then_close`", decided by the owner on 2026-11-16, because it removes the item
without losing work. It comes with these safety fallbacks:

- `Dirty::Unknown` is treated as dirty. Only a clean document closes without a save step.
- File-backed documents are saved through the app's own API (AppleScript `close … saving yes`, Word
  `Close(wdSaveChanges)`, Excel `Close(SaveChanges=True)`, PowerPoint `Save()` then `Close()`, since
  PowerPoint's `Close()` discards silently).
- Untitled documents are saved into `Config.untitled_save_dir` (default `~/Documents/Close Call`)
  where the app supports saving to a path through its API. Otherwise they are hidden. We never drive a
  Save panel.
- If a save fails, or a sheet appears whose buttons cannot be identified by role, we press Cancel,
  conceal the document and notify.
- `discard` needs an explicit opt-in in the rule, and a second one for documents that were never saved.
- When several rules hit one document, the most data-preserving policy wins:
  hide > skip_and_notify > save_then_close > discard.

### Consequences

- Good, because the sensitive document is gone and the work is on disk.
- Bad, because edits the user had not meant to keep are written to the file. For macOS autosave apps
  (Pages, Keynote, Numbers, TextEdit, Preview) closing saves in place anyway.
- Bad, because untitled documents appear as files in `untitled_save_dir`; the run report tells the
  user where, inside our own window only.
- Bad, because saving takes longer than hiding, so documents are acted on after instant conceal steps
  (phase order: Conceal, CloseTabs, CloseDocuments, Notify).

### Confirmation

`UnsavedPolicy::default()` is `SaveThenClose` in `crates/core/src/model.rs`. Planner property tests
check that `discard` never appears without a rule asking for it. The v0.1 exit criteria require no
data loss in any unsaved-document scenario of the [test matrix](../test-matrix.md).

## More Information

[architecture-recommendation.md §10](../research/architecture-recommendation.md) (written with `hide`
as default; the rest of that section still applies), [platform-macos.md §3](../research/platform-macos.md).
