# crates/engine

Turns signals into runs: inventory index, trigger fusion, scheduler, planner calls, executor, journal.
Depends on `core`, `lists` and exactly one `platform-*` chosen by `cfg(target_os)`; feature `fake`
uses `platform-fake`. No Tauri dependency: the app crate wraps the engine, not the other way round.

## Rules

- The engine owns the clock and passes `now_ms` into `core`. Item age restarts when the app restarts.
- Each `Provider` runs on its own OS thread and is driven over channels. Never call a provider from
  the UI thread. `SignalSource::start` runs once on the main thread and must not block.
- The index stays warm (provider events plus a full refresh every 30 s). A trigger never waits for a scan.
- Re-verify every target right before acting; skip it if it is gone or no longer matches.
- `ExecContext.planned` holds only the provider's targets whose step has the current step's action;
  `ExecContext.protected` holds the run's protected targets, so no provider hides an app that shows one.
- Step deadlines: 300 ms for Conceal, 1 s per tab batch, 2 s per document with a sheet.
- Debounce: the same trigger does not fire again within 10 s, except Panic. Share stop is debounced 3 s.
- Share-guard: while a share or mirroring is active, every added or changed target is re-planned
  against the rules armed by `screen_share`/`display_mirrored` and acted on at once; the same for
  `display_connected` while a display that is not ignored is connected and a rule arms it. Whenever
  a guard trigger comes on without firing (debounced restart, pause ending, `resume()`, a newly armed
  rule), the engine sweeps what is open. A target left open is announced once per guard.
- `config.ignore`: a capture put down to an ignored app, and an ignored display, count for nothing.
  An unattributed capture is put down to an ignored app whenever one runs.
- Never prompt for permissions while a share or projector is active (`RequestOutcome::NotWhileSharing`).
  The broker lock may be held while a prompt waits for the user: the fusion thread and the window
  only `try_lock` it.
- The restore journal stays in memory and expires after `restore_window`; so do the titles, URLs,
  paths and evidence in the activity history. Logs carry rule ids, counts and outcomes, never titles,
  URLs or paths.
- A rejected `config.toml` is never overwritten silently: `Status.config_problem` names why, and the
  first save keeps it as `config.rejected.toml`.
- Engine events carry message keys and parameters, never prose.

## Testing

Write scenarios against `platform-fake`: share starts mid-navigation, an unsaved document under each
policy, lock then sleep then wake while locked, a noisy watcher, a provider timeout. These run on any
OS, so CI covers them; they are also what the no-network runtime test drives.
