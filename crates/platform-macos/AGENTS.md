# crates/platform-macos

Implements the `crates/core` platform traits for macOS 14+. Depends only on `closecall-core` and the
`objc2` family; no Swift helper, no `accessibility` crate.

## Rules

- Every `unsafe` block gets a `// SAFETY:` comment.
- AX and Apple Events never run on the main thread. Set `AXUIElementSetMessagingTimeout` (0.5 s) on
  every element (`Ax` does it for every element it hands out); wrap every script in a 2 s timeout.
  `CannotComplete` means busy, never "closed" or "gone".
- Never send an Apple Event to an app that is not running (it launches it). Check first.
- Never request Screen Recording. `CGWindowList` is for owner PIDs, window counts, layers and bounds
  only (none of them needs the permission); titles come from AX
  ([ADR-0003](../../docs/decisions/0003-never-request-screen-recording.md)).
- Never prompt for a permission from provider or signal code. Prompts happen only through
  `PermissionBroker::request`, which the app calls during onboarding or at launch, never mid-share.
- Pick sheet and dialog buttons by role (`AXDefaultButton`, `AXCancelButton`, the remaining button),
  never by localized title. The Hungarian system language is a first-class target. Answer only a
  sheet shaped like a save alert (three buttons, default and cancel, no form controls) that our own
  close press brought up; cancel or leave anything else and conceal.
- Read dirty state from the window's `AXCloseButton` `AXEdited`; trust only `true`. AppleScript
  `modified`/`saved` comes first where a dictionary exists.
- Hide an app only if every visible window of it is in `ExecContext.planned` (which holds only
  targets with the current step's action), and never while a target in `ExecContext.protected`
  shares the app instance; otherwise act per window or skip with `reason.protected_in_app`. Never
  escalate a failed Minimize to Hide. Never `forceTerminate()`.
- AX observers live on their own thread with its own `CFRunLoop` (`observe.rs`): the engine blocks
  the provider thread on a channel, so that thread's run loop never spins. Polling stays the fallback.
- Restore never launches an app, except Launch Services opening a closed document in its app. Private
  tabs, and Safari tabs (privacy unknown), are never reopened.
- SkyLight symbols go through `dlopen`/`dlsym` and fail open: a missing symbol is
  `SourceHealth::Unavailable`, not a crash ([ADR-0010](../../docs/decisions/0010-skylight-share-detection.md)).
- Report capabilities honestly: `Support::Partial { note_key }` with a key that exists in `locales/`.
  Every key in `keys.rs` is a full locales key (`reason.*`, `note.*`), `StepOutcome::Failed` carries a
  key too, and `bun scripts/ci/check-message-keys.ts` fails on a missing one.
- Never log titles, URLs or paths.

## Testing

Unit-test parsing and mapping code without TCC. Anything that needs Accessibility or Automation must
run from a bundled `.app` signed with a stable identity; `bun run dev` attributes TCC to the terminal.
Reset grants with `tccutil reset Accessibility hu.futozs.closecall` (and `AppleEvents`). Record device
results in [docs/test-matrix.md](../../docs/test-matrix.md) and capability facts in
[docs/platforms/macos.md](../../docs/platforms/macos.md). If you could not run something on a Mac, say so.
