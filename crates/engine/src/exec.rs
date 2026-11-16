//! Runs: plan, re-verify each target, execute phase by phase with deadlines, report, journal.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use closecall_core::config::Config;
use closecall_core::ipc::{RunReport, RunSummary, StepReport, TargetSummary};
use closecall_core::plan::{PlanContext, plan, protected};
use closecall_core::platform::{ExecContext, Reverify};
use closecall_core::{
    Action, AppSelector, Capability, Confirm, Dirty, Dur, FiredTrigger, Phase, Reason, Step,
    StepOutcome, Target, TargetId, TargetKind, Tri,
};
use closecall_lists::Lists;

use crate::actor::Command;
use crate::runtime::{ACTIVITY_CAP, Core, Pending, lock};
use crate::store::expand;
use crate::{EngineError, Notice};

pub(crate) const REASON_GONE: &str = "reason.gone";
pub(crate) const REASON_CHANGED: &str = "reason.changed_before_action";
pub(crate) const REASON_TIMEOUT: &str = "reason.timeout";
pub(crate) const REASON_PRIVATE: &str = "reason.private_not_restorable";
/// Reopening a document waits for Launch Services and the app, which can take seconds.
const DOCUMENT_RESTORE_DEADLINE: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunKind {
    Automatic,
    Panic,
    /// `simulate` from the settings window: acts, but the user is looking, so no notification.
    Manual,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Countdowns {
    /// Steps with a countdown wait for it; the rest act now.
    Defer,
    /// A countdown ran out: re-plan and act on the steps that waited for this one.
    Due(Dur),
}

pub(crate) struct RunRequest {
    pub fired: FiredTrigger,
    pub kind: RunKind,
    pub scope: Option<BTreeSet<TargetId>>,
    pub countdowns: Countdowns,
}

impl RunRequest {
    /// An item-age run only ever plans its own target, so it only snapshots that one.
    pub fn new(fired: FiredTrigger, kind: RunKind) -> Self {
        let scope = match &fired {
            FiredTrigger::ItemAge { target, .. } => Some(BTreeSet::from([*target])),
            _ => None,
        };
        Self {
            fired,
            kind,
            scope,
            countdowns: Countdowns::Defer,
        }
    }
}

/// Pre-step targets of Done and FellBack steps, in memory only, until `restore_window` ends.
#[derive(Default)]
pub(crate) struct Journal {
    entries: Vec<JournalEntry>,
}

struct JournalEntry {
    run_id: u64,
    trigger: FiredTrigger,
    provider: usize,
    target: Target,
    action: Action,
    phase: Phase,
    because: Vec<Reason>,
    expires_unix_ms: u64,
}

impl Journal {
    pub fn prune(&mut self, now_unix_ms: u64) {
        self.entries
            .retain(|entry| entry.expires_unix_ms > now_unix_ms);
    }

    pub fn restorable(&self, run_id: u64, now_unix_ms: u64) -> bool {
        self.entries.iter().any(|entry| {
            entry.run_id == run_id
                && entry.expires_unix_ms > now_unix_ms
                && !never_restored(&entry.target)
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn take(&mut self, run_id: u64, now_unix_ms: u64) -> Vec<JournalEntry> {
        self.prune(now_unix_ms);
        let (taken, kept) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|entry| entry.run_id == run_id);
        self.entries = kept;
        taken
    }
}

/// Reopening a private tab in a normal window would write it to history; Safari cannot tell
/// private from normal, so `Unknown` counts as private.
fn never_restored(target: &Target) -> bool {
    matches!(&target.kind, TargetKind::BrowserTab(tab) if tab.private != Tri::No)
}

struct RunData {
    config: Arc<Config>,
    lists: Arc<Lists>,
    capabilities: Vec<Capability>,
    protect_apps: Vec<AppSelector>,
    untitled_save_dir: PathBuf,
    now_ms: u64,
}

impl RunData {
    fn context<'a>(&'a self, home: &'a str) -> PlanContext<'a> {
        PlanContext {
            categories: &*self.lists,
            protect: &self.config.protect,
            protect_apps: &self.protect_apps,
            capabilities: &self.capabilities,
            now_ms: self.now_ms,
            home,
        }
    }

    fn plan(&self, fired: &FiredTrigger, targets: &[Target], home: &str) -> Vec<Step> {
        plan(fired, targets, &self.config.rules, &self.context(home)).steps
    }
}

struct Planned {
    step: Step,
    provider: usize,
    target: Target,
}

enum IndexUpdate {
    Remove(TargetId),
    Replace(Target),
}

struct Executed {
    target: Target,
    action: Action,
    outcome: StepOutcome,
    update: Option<IndexUpdate>,
}

impl Core {
    /// Returns the report and every target the plan covered, deferred countdown steps included.
    pub fn run(&self, request: RunRequest) -> (RunReport, BTreeSet<TargetId>) {
        let serial = lock(&self.run_lock);
        let now = self.now();
        let (data, snapshot) = {
            let state = lock(&self.state);
            let data = RunData {
                config: Arc::clone(&state.config),
                lists: Arc::clone(&state.lists),
                capabilities: state.capabilities.concat(),
                protect_apps: self.protect_apps(&state.fusion.capturing_apps()),
                untitled_save_dir: expand(&state.config.untitled_save_dir, &self.settings.home),
                now_ms: now.mono_ms,
            };
            (data, state.index.targets(request.scope.as_ref()))
        };
        let targets: Vec<Target> = snapshot.iter().map(|(_, t)| t.clone()).collect();
        let providers: BTreeMap<TargetId, usize> =
            snapshot.iter().map(|(p, t)| (t.id, *p)).collect();
        let steps = data.plan(&request.fired, &targets, &self.home);
        let covered: BTreeSet<TargetId> = steps.iter().map(|step| step.target).collect();
        let kept_visible: Vec<Target> = if steps.is_empty() {
            Vec::new()
        } else {
            let ids: BTreeSet<TargetId> = protected(&targets, &data.context(&self.home))
                .into_iter()
                .collect();
            targets
                .iter()
                .filter(|target| ids.contains(&target.id))
                .cloned()
                .collect()
        };

        let mut report = RunReport {
            run_id: 0,
            at_ms: now.unix_ms,
            trigger: request.fired.clone(),
            dry_run: request.kind == RunKind::Preview,
            steps: Vec::new(),
            restorable: false,
            restore_of: None,
        };
        let by_id: BTreeMap<TargetId, &Target> = targets.iter().map(|t| (t.id, t)).collect();
        let planned: Vec<Planned> = steps
            .into_iter()
            .filter_map(|step| {
                Some(Planned {
                    provider: *providers.get(&step.target)?,
                    target: (*by_id.get(&step.target)?).clone(),
                    step,
                })
            })
            .collect();

        if request.kind == RunKind::Preview {
            report.steps = planned
                .iter()
                .map(|p| step_report(&p.step, &p.target, p.step.action, None))
                .collect();
            report.run_id = lock(&self.state).next_run_id();
            return (report, covered);
        }

        let planned = self.split_countdowns(&request, planned, now.mono_ms);
        let executed = self.execute(&request.fired, &data, &planned, &kept_visible);
        report.steps = planned
            .iter()
            .zip(&executed)
            .map(|(p, done)| {
                step_report(
                    &p.step,
                    &done.target,
                    done.action,
                    Some(done.outcome.clone()),
                )
            })
            .collect();

        let restore_ms = data.config.restore_window.as_millis();
        // An automatic run that found nothing to do leaves no trace, not even a run id.
        let kept = !report.steps.is_empty() || request.kind != RunKind::Automatic;
        let repeated: Vec<bool>;
        {
            let mut state = lock(&self.state);
            if kept {
                report.run_id = state.next_run_id();
            }
            let run_id = report.run_id;
            // While the guard is on, a target left open is reported once, not again on every
            // title change that re-plans it.
            let guard_on = state.guard_active(now);
            repeated = executed
                .iter()
                .map(|done| {
                    guard_on
                        && left_open(done.action, &done.outcome)
                        && !state.left_open.insert(done.target.id)
                })
                .collect();
            for (p, done) in planned.iter().zip(&executed) {
                match &done.update {
                    Some(IndexUpdate::Remove(id)) => state.index.remove(*id),
                    Some(IndexUpdate::Replace(target)) => state.index.replace(target.clone()),
                    None => {}
                }
                let Some(action) = performed(done.action, &done.outcome) else {
                    continue;
                };
                state.acted.insert(done.target.id, done.target.kind.clone());
                if matches!(done.action, Action::Close | Action::Quit)
                    && done.outcome == StepOutcome::Done
                {
                    state.index.remove(done.target.id);
                }
                if action != Action::NotifyOnly {
                    state.journal.entries.push(JournalEntry {
                        run_id,
                        trigger: request.fired.clone(),
                        provider: p.provider,
                        target: done.target.clone(),
                        action,
                        phase: p.step.phase,
                        because: p.step.because.clone(),
                        expires_unix_ms: now.unix_ms.saturating_add(restore_ms),
                    });
                }
            }
            report.restorable = state.journal.restorable(run_id, now.unix_ms);
            if !report.steps.is_empty() {
                state.last_run = Some(summary(&report));
                state.activity.push_front(report.clone());
                state.activity.truncate(ACTIVITY_CAP);
            }
        }
        drop(serial);
        if kept {
            tracing::info!(
                run_id = report.run_id,
                trigger = trigger_name(&request.fired),
                steps = report.steps.len(),
                "run finished"
            );
        }
        if !report.steps.is_empty() {
            self.sink.run_completed(&report);
            if matches!(request.kind, RunKind::Automatic | RunKind::Panic) {
                self.notify_run(&report, &executed, &repeated);
            }
        }
        self.publish_status();
        (report, covered)
    }

    fn split_countdowns(
        &self,
        request: &RunRequest,
        planned: Vec<Planned>,
        now_ms: u64,
    ) -> Vec<Planned> {
        match request.countdowns {
            Countdowns::Due(countdown) => planned
                .into_iter()
                .filter(|p| p.step.confirm == Confirm::Countdown(countdown))
                .collect(),
            Countdowns::Defer => {
                let mut waiting: BTreeSet<Dur> = BTreeSet::new();
                let now: Vec<Planned> = planned
                    .into_iter()
                    .filter(|p| match p.step.confirm {
                        Confirm::Countdown(countdown) => {
                            waiting.insert(countdown);
                            false
                        }
                        Confirm::Never => true,
                    })
                    .collect();
                if !waiting.is_empty() {
                    let mut state = lock(&self.state);
                    for countdown in waiting {
                        let duplicate = state.pending.iter().any(|pending| {
                            pending.fired == request.fired && pending.countdown == countdown
                        });
                        if !duplicate {
                            state.pending.push(Pending {
                                fired: request.fired.clone(),
                                countdown,
                                due_ms: now_ms.saturating_add(countdown.as_millis()),
                            });
                        }
                    }
                }
                now
            }
        }
    }

    /// Phases run in order. Within a phase each provider works through its own steps on a
    /// thread of its own, so one slow app does not hold up the others.
    fn execute(
        &self,
        fired: &FiredTrigger,
        data: &RunData,
        planned: &[Planned],
        protected: &[Target],
    ) -> Vec<Executed> {
        let mut results: Vec<Option<Executed>> = planned.iter().map(|_| None).collect();
        let mut start = 0;
        while start < planned.len() {
            let phase = planned[start].step.phase;
            let end = planned[start..]
                .iter()
                .position(|p| p.step.phase != phase)
                .map_or(planned.len(), |offset| start + offset);
            let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for (i, p) in planned.iter().enumerate().take(end).skip(start) {
                groups.entry(p.provider).or_default().push(i);
            }
            let finished: Vec<(usize, Executed)> = std::thread::scope(|scope| {
                let workers: Vec<_> = groups
                    .iter()
                    .map(|(provider, indices)| {
                        scope.spawn(move || {
                            indices
                                .iter()
                                .map(|&i| {
                                    let job = Job {
                                        fired,
                                        data,
                                        planned,
                                        protected,
                                        provider: *provider,
                                    };
                                    (i, self.act(&job, i))
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                workers
                    .into_iter()
                    .flat_map(|worker| worker.join().unwrap_or_default())
                    .collect()
            });
            for (i, done) in finished {
                results[i] = Some(done);
            }
            start = end;
        }
        results
            .into_iter()
            .zip(planned)
            .map(|(done, p)| {
                done.unwrap_or_else(|| Executed {
                    target: p.target.clone(),
                    action: p.step.action,
                    outcome: failed(REASON_TIMEOUT),
                    update: None,
                })
            })
            .collect()
    }

    fn act(&self, job: &Job<'_>, i: usize) -> Executed {
        let Job {
            fired,
            data,
            planned,
            protected,
            provider,
        } = *job;
        let p = &planned[i];
        let mut done = Executed {
            target: p.target.clone(),
            action: p.step.action,
            outcome: StepOutcome::Done,
            update: None,
        };
        let Some(handle) = self.providers.get(provider) else {
            done.outcome = skipped(REASON_GONE);
            return done;
        };
        let budget = self.deadline(p.step.phase);
        let deadline = Instant::now() + budget;
        let reverified = handle.request(
            |reply| Command::Reverify {
                target: p.target.clone(),
                deadline,
                reply,
            },
            deadline + self.options.reply_grace,
        );
        let step = match reverified {
            None => {
                done.outcome = failed(REASON_TIMEOUT);
                return done;
            }
            Some(Reverify::Gone) => {
                done.outcome = skipped(REASON_GONE);
                done.update = Some(IndexUpdate::Remove(p.target.id));
                return done;
            }
            Some(Reverify::Same) => p.step.clone(),
            Some(Reverify::Changed(fresh)) => {
                let fresh = Target {
                    id: p.target.id,
                    first_seen_ms: p.target.first_seen_ms,
                    provider: p.target.provider.clone(),
                    ..fresh
                };
                done.update = Some(IndexUpdate::Replace(fresh.clone()));
                done.target = fresh;
                let replanned = data.plan(fired, std::slice::from_ref(&done.target), &self.home);
                match replanned
                    .into_iter()
                    .find(|step| step.target == p.target.id)
                {
                    Some(step) => step,
                    None => {
                        done.outcome = skipped(REASON_CHANGED);
                        return done;
                    }
                }
            }
        };
        done.action = step.action;
        if step.action == Action::NotifyOnly {
            return done;
        }
        let ctx = ExecContext {
            untitled_save_dir: data.untitled_save_dir.clone(),
            planned: std::iter::once(done.target.clone())
                .chain(
                    planned
                        .iter()
                        .enumerate()
                        .filter(|&(j, other)| {
                            j != i && other.provider == provider && other.step.action == step.action
                        })
                        .map(|(_, other)| other.target.clone()),
                )
                .collect(),
            protected: protected.to_vec(),
        };
        let target = done.target.clone();
        let deadline = Instant::now() + budget;
        done.outcome = handle
            .request(
                |reply| Command::Execute {
                    step,
                    target,
                    ctx,
                    deadline,
                    reply,
                },
                deadline + self.options.reply_grace,
            )
            .unwrap_or_else(|| failed(REASON_TIMEOUT));
        done
    }

    fn deadline(&self, phase: Phase) -> Duration {
        match phase {
            Phase::Conceal | Phase::Notify => self.options.conceal_deadline,
            Phase::CloseTabs => self.options.tab_deadline,
            Phase::CloseDocuments => self.options.document_deadline,
        }
    }

    /// `repeated` marks targets already reported as left open while this guard is on.
    fn notify_run(&self, report: &RunReport, executed: &[Executed], repeated: &[bool]) {
        let mut done = 0;
        let mut hidden = 0;
        let mut skipped = 0;
        let mut unsaved = 0;
        for ((step, result), repeated) in report.steps.iter().zip(executed).zip(repeated) {
            if *repeated {
                continue;
            }
            match &result.outcome {
                StepOutcome::Done => match step.action {
                    Action::Close | Action::Quit => done += 1,
                    Action::Hide | Action::Minimize => hidden += 1,
                    Action::NotifyOnly => skipped += 1,
                },
                StepOutcome::FellBack { action, .. } => {
                    match action {
                        Action::Close | Action::Quit => done += 1,
                        Action::Hide | Action::Minimize => hidden += 1,
                        Action::NotifyOnly => skipped += 1,
                    }
                    if unsaved_document(&result.target) {
                        unsaved += 1;
                    }
                }
                StepOutcome::Skipped { reason_key } if reason_key == REASON_GONE => {}
                StepOutcome::Skipped { .. } | StepOutcome::Failed { .. } => skipped += 1,
            }
        }
        if done + hidden + skipped > 0 {
            self.sink.notify(&Notice::RunSummary {
                done,
                hidden,
                skipped,
            });
        }
        if unsaved > 0 {
            self.sink.notify(&Notice::UnsavedHidden { count: unsaved });
        }
    }

    pub fn restore(&self, run_id: u64) -> Result<RunReport, EngineError> {
        let serial = lock(&self.run_lock);
        let now = self.now();
        let (entries, new_id) = {
            let mut state = lock(&self.state);
            if !state.journal.restorable(run_id, now.unix_ms) {
                return Err(EngineError::NotRestorable(run_id));
            }
            let entries = state.journal.take(run_id, now.unix_ms);
            (entries, state.next_run_id())
        };
        let Some(trigger) = entries.first().map(|entry| entry.trigger.clone()) else {
            return Err(EngineError::NotRestorable(run_id));
        };
        let steps = entries
            .iter()
            .rev()
            .map(|entry| {
                let outcome = if never_restored(&entry.target) {
                    skipped(REASON_PRIVATE)
                } else {
                    self.restore_one(entry)
                };
                StepReport {
                    target: entry.target.id,
                    summary: summarize(&entry.target),
                    action: entry.action,
                    phase: entry.phase,
                    because: entry.because.clone(),
                    outcome: Some(outcome),
                }
            })
            .collect();
        let report = RunReport {
            run_id: new_id,
            at_ms: now.unix_ms,
            trigger,
            dry_run: false,
            steps,
            restorable: false,
            restore_of: Some(run_id),
        };
        {
            let mut state = lock(&self.state);
            state.activity.push_front(report.clone());
            state.activity.truncate(ACTIVITY_CAP);
        }
        drop(serial);
        tracing::info!(run_id, steps = report.steps.len(), "restore finished");
        self.sink.run_completed(&report);
        self.publish_status();
        Ok(report)
    }

    fn restore_one(&self, entry: &JournalEntry) -> StepOutcome {
        let Some(handle) = self.providers.get(entry.provider) else {
            return skipped(REASON_GONE);
        };
        let budget = match (&entry.target.kind, entry.action) {
            (TargetKind::DocumentWindow(_), Action::Close | Action::Quit) => {
                DOCUMENT_RESTORE_DEADLINE
            }
            _ => self.options.document_deadline,
        };
        let deadline = Instant::now() + budget;
        handle
            .request(
                |reply| Command::Restore {
                    target: entry.target.clone(),
                    action: entry.action,
                    deadline,
                    reply,
                },
                deadline + self.options.reply_grace,
            )
            .unwrap_or_else(|| failed(REASON_TIMEOUT))
    }
}

struct Job<'a> {
    fired: &'a FiredTrigger,
    data: &'a RunData,
    planned: &'a [Planned],
    protected: &'a [Target],
    provider: usize,
}

fn left_open(action: Action, outcome: &StepOutcome) -> bool {
    match outcome {
        StepOutcome::Done => action == Action::NotifyOnly,
        StepOutcome::Skipped { reason_key } => reason_key != REASON_GONE,
        StepOutcome::FellBack { .. } | StepOutcome::Failed { .. } => false,
    }
}

/// Once a run can no longer be restored, its history keeps what happened but not to what:
/// titles, URLs and paths go, and so does the evidence, which can name the site that matched.
pub(crate) fn forget_details(
    activity: &mut std::collections::VecDeque<RunReport>,
    now_unix_ms: u64,
    restore_ms: u64,
) {
    for report in activity
        .iter_mut()
        .filter(|report| report.at_ms.saturating_add(restore_ms) <= now_unix_ms)
    {
        for step in &mut report.steps {
            step.summary.title.clear();
            step.summary.url = None;
            step.summary.path = None;
            step.because.clear();
        }
    }
}

/// What the provider actually did, if anything a restore could undo.
fn performed(planned: Action, outcome: &StepOutcome) -> Option<Action> {
    match outcome {
        StepOutcome::Done => Some(planned),
        StepOutcome::FellBack { action, .. } => Some(*action),
        StepOutcome::Skipped { .. } | StepOutcome::Failed { .. } => None,
    }
}

fn unsaved_document(target: &Target) -> bool {
    matches!(&target.kind, TargetKind::DocumentWindow(doc) if doc.dirty != Dirty::Clean)
}

fn step_report(
    step: &Step,
    target: &Target,
    action: Action,
    outcome: Option<StepOutcome>,
) -> StepReport {
    StepReport {
        target: step.target,
        summary: summarize(target),
        action,
        phase: step.phase,
        because: step.because.clone(),
        outcome,
    }
}

pub(crate) fn summarize(target: &Target) -> TargetSummary {
    let (title, url, path) = match &target.kind {
        TargetKind::BrowserTab(tab) => (tab.title.clone(), tab.url.clone(), None),
        TargetKind::DocumentWindow(doc) => (doc.title.clone(), None, doc.path.clone()),
        TargetKind::AppWindow(window) => (window.title.clone(), None, None),
        TargetKind::Process(_) => (target.app.name.clone(), None, None),
    };
    TargetSummary {
        kind: target.kind.tag(),
        app_name: target.app.name.clone(),
        title,
        url,
        path,
    }
}

pub(crate) fn summary(report: &RunReport) -> RunSummary {
    let mut summary = RunSummary {
        run_id: report.run_id,
        at_ms: report.at_ms,
        trigger: report.trigger.clone(),
        done: 0,
        fell_back: 0,
        skipped: 0,
        failed: 0,
    };
    for step in &report.steps {
        match &step.outcome {
            Some(StepOutcome::Done) => summary.done += 1,
            Some(StepOutcome::FellBack { .. }) => summary.fell_back += 1,
            Some(StepOutcome::Skipped { .. }) => summary.skipped += 1,
            Some(StepOutcome::Failed { .. }) => summary.failed += 1,
            None => {}
        }
    }
    summary
}

pub(crate) fn trigger_name(fired: &FiredTrigger) -> &'static str {
    match fired {
        FiredTrigger::Panic => "panic",
        FiredTrigger::ScreenShare { .. } => "screen_share",
        FiredTrigger::DisplayMirrored => "display_mirrored",
        FiredTrigger::DisplayConnected => "display_connected",
        FiredTrigger::CallLikely => "call_likely",
        FiredTrigger::WillSleep => "will_sleep",
        FiredTrigger::LidClosed => "lid_closed",
        FiredTrigger::Locked => "locked",
        FiredTrigger::Away { .. } => "away",
        FiredTrigger::Idle { .. } => "idle",
        FiredTrigger::ItemAge { .. } => "item_age",
    }
}

fn skipped(reason_key: &str) -> StepOutcome {
    StepOutcome::Skipped {
        reason_key: reason_key.to_owned(),
    }
}

fn failed(error: &str) -> StepOutcome {
    StepOutcome::Failed {
        error: error.to_owned(),
    }
}
