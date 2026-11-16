//! The engine's shared state and its one fusion thread, which owns signal handling, inventory
//! updates, share-guard and the timers.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use closecall_core::config::Config;
use closecall_core::ipc::{RunReport, RunSummary, SignalSourceStatus, Status};
use closecall_core::platform::{
    Discovered, InventoryEvent, PermissionBroker, PermissionState, ProviderError, Signal,
    SignalKind, SignalSource, SourceHealth,
};
use closecall_core::{
    AppRef, AppSelector, Capability, DisplayInfo, Dur, FiredTrigger, TargetId, TargetKind,
};
use closecall_lists::Lists;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};

use crate::actor::{ProviderHandle, error_kind, is_permission_missing};
use crate::exec::{Countdowns, Journal, RunKind, RunRequest, forget_details, trigger_name};
use crate::fusion::{Fusion, Now, Outcome};
use crate::index::Index;
use crate::store::Problem;
use crate::{EngineOptions, EngineSettings, EngineSink, Notice};

const REFRESH_MS: u64 = 30_000;
const CAPTURE_IGNORED: &str = "reason.capture_ignored";
const IDLE_POLL_MS: u64 = 30_000;
const BACKGROUND_TICK: Duration = Duration::from_secs(1);
pub(crate) const ACTIVITY_CAP: usize = 50;

pub(crate) enum Msg {
    Inventory {
        provider: usize,
        event: InventoryEvent,
    },
    Snapshot {
        provider: usize,
        result: Result<Vec<Discovered>, ProviderError>,
        capabilities: Vec<Capability>,
    },
    Signal(Signal),
    /// The ignore list may have changed: put running captures down to ignored apps again.
    Reattribute,
    Tick(Option<Sender<()>>),
    Flush(Sender<()>),
    Shutdown,
}

pub(crate) struct Pending {
    pub fired: FiredTrigger,
    pub countdown: Dur,
    pub due_ms: u64,
}

pub(crate) struct State {
    pub config: Arc<Config>,
    pub lists: Arc<Lists>,
    pub config_problem: Option<Problem>,
    pub needs_onboarding: bool,
    pub index: Index,
    pub capabilities: Vec<Vec<Capability>>,
    pub permission_missing: Vec<bool>,
    pub fusion: Fusion,
    pub paused_until_ms: Option<u64>,
    pub pending: Vec<Pending>,
    pub journal: Journal,
    pub activity: VecDeque<RunReport>,
    pub last_run: Option<RunSummary>,
    pub last_run_id: u64,
    pub last_refresh_ms: u64,
    pub last_idle_poll_ms: Option<u64>,
    /// What each target looked like when a run last acted on it, so share-guard does not act
    /// again on an event that only echoes our own change.
    pub acted: HashMap<TargetId, TargetKind>,
    /// Targets reported as left open while the guard has been on.
    pub left_open: HashSet<TargetId>,
    /// The guard triggers that were on after the last signal or tick, by name.
    pub guarding: BTreeSet<&'static str>,
    /// Which providers have delivered their first snapshot.
    pub reported: Vec<bool>,
}

impl State {
    pub fn paused(&self, now: Now) -> bool {
        self.paused_until_ms
            .is_some_and(|until| until > now.unix_ms)
    }

    /// Automatic triggers stay off until the user has finished onboarding: a fresh install must
    /// not act on rules nobody has agreed to yet. Panic always works.
    pub fn automatic_off(&self, now: Now) -> bool {
        self.needs_onboarding || self.paused(now)
    }

    pub fn guard_active(&self, now: Now) -> bool {
        !self.automatic_off(now) && !self.fusion.guard_triggers(&self.config).is_empty()
    }

    pub fn next_run_id(&mut self) -> u64 {
        self.last_run_id += 1;
        self.last_run_id
    }

    /// The echo filter and the left-open set only make sense while the guard is on.
    fn forget_guard_episode(&mut self, now: Now) {
        if !self.guard_active(now) {
            self.acted.clear();
            self.left_open.clear();
        }
    }
}

pub(crate) struct Core {
    pub state: Mutex<State>,
    /// Signalled when a provider delivers its first snapshot.
    pub ready: Condvar,
    pub run_lock: Mutex<()>,
    /// Serializes writes of `config.toml` with the in-memory config they install.
    pub save_lock: Mutex<()>,
    pub providers: Vec<ProviderHandle>,
    pub sources: Mutex<Vec<Box<dyn SignalSource>>>,
    pub signals_started: AtomicBool,
    pub broker: Mutex<Box<dyn PermissionBroker>>,
    pub permissions: Mutex<Vec<PermissionState>>,
    pub idle_clock: Box<dyn Fn() -> Option<Duration> + Send + Sync>,
    pub running_apps: Box<dyn Fn() -> Vec<AppRef> + Send + Sync>,
    pub displays: Box<dyn Fn() -> Vec<DisplayInfo> + Send + Sync>,
    pub sink: Arc<dyn EngineSink>,
    pub settings: EngineSettings,
    pub home: String,
    pub platform: &'static str,
    pub options: EngineOptions,
    pub tx: Sender<Msg>,
    pub processed: AtomicU64,
    pub last_status: Mutex<Option<Status>>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Core {
    pub fn now(&self) -> Now {
        Now {
            mono_ms: self.options.clock.mono_ms(),
            unix_ms: self.options.clock.unix_ms(),
        }
    }

    /// Our own app always, plus the apps capturing the screen where a source names them.
    pub fn protect_apps(&self, capturing_apps: &[&str]) -> Vec<AppSelector> {
        std::iter::once(self.settings.own_app_id.as_str())
            .chain(capturing_apps.iter().copied())
            .flat_map(|app| {
                [
                    AppSelector::BundleId(app.to_owned()),
                    AppSelector::Exe(app.to_owned()),
                ]
            })
            .collect()
    }

    pub fn status(&self) -> Status {
        let permissions = lock(&self.permissions).clone();
        let signal_sources = self.source_statuses();
        let now = self.now();
        let state = lock(&self.state);
        let paused_until_ms = state.paused_until_ms.filter(|until| *until > now.unix_ms);
        Status {
            platform: self.platform.to_owned(),
            app_version: self.settings.app_version.clone(),
            share: state.fusion.share_state(&state.config),
            guard_active: state.guard_active(now),
            paused_until_ms,
            last_run: state.last_run.clone(),
            permissions,
            signal_sources,
            needs_onboarding: state.needs_onboarding,
            config_problem: state.config_problem.as_ref().map(|p| p.key().to_owned()),
        }
    }

    /// A source whose capture is put down to an ignored app cannot see a new share start, so
    /// it reports as noisy with the reason, whatever it says itself.
    pub fn source_statuses(&self) -> Vec<SignalSourceStatus> {
        let ignored: Vec<String> = lock(&self.state)
            .fusion
            .ignored_sources()
            .into_iter()
            .map(str::to_owned)
            .collect();
        lock(&self.sources)
            .iter()
            .map(|source| {
                let id = source.id();
                let health = if ignored.contains(&id) {
                    SourceHealth::Noisy {
                        reason_key: CAPTURE_IGNORED.to_owned(),
                    }
                } else {
                    source.health()
                };
                SignalSourceStatus { id, health }
            })
            .collect()
    }

    /// Emits only when something changed. The sink is called without any lock held, since it
    /// may call back into the engine.
    pub fn publish_status(&self) {
        let status = self.status();
        {
            let mut last = lock(&self.last_status);
            if last.as_ref() == Some(&status) {
                return;
            }
            *last = Some(status.clone());
        }
        self.sink.status_changed(&status);
    }

    /// Keeps the last answer while a permission prompt is open: the request holds the broker
    /// until the user answers, and neither the fusion thread nor the window may wait for that.
    pub fn refresh_permissions(&self) {
        let broker = match self.broker.try_lock() {
            Ok(broker) => broker,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };
        let fresh = broker.status();
        drop(broker);
        *lock(&self.permissions) = fresh;
    }

    /// Waits until every provider has delivered its first snapshot, or `max` has passed.
    pub fn wait_ready(&self, max: Duration) -> bool {
        let until = Instant::now() + max;
        let mut state = lock(&self.state);
        loop {
            if state.reported.iter().all(|done| *done) {
                return true;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            state = self
                .ready
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    pub fn sharing(&self) -> bool {
        let state = lock(&self.state);
        let share = state.fusion.share_state(&state.config);
        share.active || share.mirrored_display || share.external_display
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        lock(&self.state).capabilities.concat()
    }

    /// Runs until `Shutdown`; every message is handled in order, so a `Flush` answered here
    /// means everything sent before it has been dealt with.
    pub fn fusion_loop(&self, rx: &Receiver<Msg>) {
        let mut guard_ids = BTreeSet::new();
        loop {
            let msg = if self.options.background_ticks {
                match rx.recv_timeout(BACKGROUND_TICK) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => Msg::Tick(None),
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return,
                }
            };
            self.processed.fetch_add(1, Ordering::SeqCst);
            match msg {
                Msg::Inventory { provider, event } => {
                    guard_ids.extend(self.on_inventory(provider, event));
                }
                Msg::Snapshot {
                    provider,
                    result,
                    capabilities,
                } => {
                    guard_ids.extend(self.on_snapshot(provider, result, capabilities));
                }
                Msg::Signal(signal) => {
                    self.guard(&mut guard_ids);
                    self.on_signal(&signal);
                }
                Msg::Reattribute => {
                    self.guard(&mut guard_ids);
                    self.reattribute();
                }
                Msg::Tick(ack) => {
                    self.guard(&mut guard_ids);
                    self.tick();
                    if let Some(ack) = ack {
                        let _ = ack.send(());
                    }
                }
                Msg::Flush(ack) => {
                    self.guard(&mut guard_ids);
                    let _ = ack.send(());
                }
                Msg::Shutdown => return,
            }
            if rx.is_empty() {
                self.guard(&mut guard_ids);
            }
        }
    }

    fn on_inventory(&self, provider: usize, event: InventoryEvent) -> Option<TargetId> {
        let handle = self.providers.get(provider)?;
        let now = self.now();
        let mut state = lock(&self.state);
        match event {
            InventoryEvent::Added(found) | InventoryEvent::Changed(found) => {
                let changed = state.index.upsert(provider, &handle.id, found, now.mono_ms);
                changed.filter(|_| state.guard_active(now))
            }
            InventoryEvent::Removed { key, .. } => {
                state.index.remove_key(provider, &key);
                None
            }
            InventoryEvent::Resync { .. } => {
                drop(state);
                handle.refresh();
                None
            }
        }
    }

    fn on_snapshot(
        &self,
        provider: usize,
        result: Result<Vec<Discovered>, ProviderError>,
        capabilities: Vec<Capability>,
    ) -> Vec<TargetId> {
        let Some(handle) = self.providers.get(provider) else {
            return Vec::new();
        };
        let now = self.now();
        let mut notify = false;
        let changed = {
            let mut state = lock(&self.state);
            if let Some(slot) = state.capabilities.get_mut(provider) {
                *slot = capabilities;
            }
            if let Some(reported) = state.reported.get_mut(provider)
                && !*reported
            {
                *reported = true;
                self.ready.notify_all();
            }
            let missing = result.as_ref().err().is_some_and(is_permission_missing);
            if let Some(flag) = state.permission_missing.get_mut(provider) {
                notify = missing && !*flag;
                *flag = missing;
            }
            match result {
                Ok(found) => {
                    let changed = state
                        .index
                        .reconcile(provider, &handle.id, found, now.mono_ms);
                    if state.guard_active(now) {
                        changed
                    } else {
                        Vec::new()
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        provider = %handle.id.0,
                        error = error_kind(&error),
                        "inventory snapshot failed"
                    );
                    Vec::new()
                }
            }
        };
        if notify {
            self.sink.notify(&Notice::PermissionMissing);
        }
        changed
    }

    fn on_signal(&self, signal: &Signal) {
        let config = Arc::clone(&lock(&self.state).config);
        let ignored_by = (signal.kind == SignalKind::CaptureStarted)
            .then(|| self.ignored_capturer(&config, signal.capturing_app.as_deref()))
            .flatten();
        let now = self.now();
        let out = {
            let mut state = lock(&self.state);
            let paused = state.automatic_off(now);
            let out = state
                .fusion
                .on_signal(signal, ignored_by, now, &config, paused);
            state.forget_guard_episode(now);
            out
        };
        tracing::debug!(kind = ?signal.kind, fired = out.fire.len(), "signal");
        self.apply(out);
    }

    /// The ignored app a capture is put down to: the one the source names, or, when the OS
    /// cannot tell, any ignored capturer that is running.
    fn ignored_capturer(&self, config: &Config, capturing_app: Option<&str>) -> Option<String> {
        let ignore = &config.ignore;
        if ignore.capture_apps.is_empty() {
            return None;
        }
        let running = (self.running_apps)();
        match capturing_app {
            Some(id) => {
                let app = running
                    .into_iter()
                    .find(|app| app.id == id)
                    .unwrap_or_else(|| AppRef {
                        id: id.to_owned(),
                        name: id.to_owned(),
                        pid: None,
                    });
                ignore.capture_app(&app).then_some(app.name)
            }
            None => running
                .into_iter()
                .find(|app| ignore.capture_app(app))
                .map(|app| app.name),
        }
    }

    fn reattribute(&self) {
        let (config, captures) = {
            let state = lock(&self.state);
            (Arc::clone(&state.config), state.fusion.running_captures())
        };
        let attributed: Vec<(String, Option<String>)> = captures
            .into_iter()
            .map(|(source, app)| {
                let ignored_by = self.ignored_capturer(&config, app.as_deref());
                (source, ignored_by)
            })
            .collect();
        let now = self.now();
        let out = {
            let mut state = lock(&self.state);
            let paused = state.automatic_off(now);
            let mut out = Outcome::default();
            for (source, ignored_by) in attributed {
                out.merge(
                    state
                        .fusion
                        .reattribute(&source, ignored_by, now, &config, paused),
                );
            }
            out
        };
        self.apply(out);
    }

    /// Share detection reaches the app before the run starts, so it can hide its own windows.
    fn apply(&self, mut out: Outcome) {
        self.sweep_new_guards(&mut out);
        if out.share_started {
            self.sink.notify(&Notice::ShareDetected);
        }
        self.publish_status();
        for fired in out.fire {
            let kind = if fired == FiredTrigger::Panic {
                RunKind::Panic
            } else {
                RunKind::Automatic
            };
            self.run(RunRequest::new(fired, kind));
        }
    }

    /// A guard that comes on without its trigger firing still sweeps what is already open:
    /// a share that restarts within the debounce, a pause that ends mid-share, a rule that
    /// newly arms a connected display.
    fn sweep_new_guards(&self, out: &mut Outcome) {
        let now = self.now();
        let mut state = lock(&self.state);
        let current = if state.automatic_off(now) {
            Vec::new()
        } else {
            state.fusion.guard_triggers(&state.config)
        };
        let names: BTreeSet<&'static str> = current.iter().map(trigger_name).collect();
        for fired in current {
            let name = trigger_name(&fired);
            let firing = out.fire.iter().any(|other| trigger_name(other) == name);
            if !firing && !state.guarding.contains(name) {
                out.fire.push(fired);
            }
        }
        state.guarding = names;
    }

    pub fn tick(&self) {
        let now = self.now();
        let (mut out, due, refresh, poll_idle) = {
            let mut state = lock(&self.state);
            if state
                .paused_until_ms
                .is_some_and(|until| until <= now.unix_ms)
            {
                state.paused_until_ms = None;
            }
            let paused = state.automatic_off(now);
            let config = Arc::clone(&state.config);
            let mut out = state.fusion.on_tick(now, &config, paused);
            let ages = state.index.ages();
            out.fire
                .extend(state.fusion.on_ages(&ages, now, &config, paused).fire);
            let (due, pending): (Vec<_>, Vec<_>) = std::mem::take(&mut state.pending)
                .into_iter()
                .partition(|pending| pending.due_ms <= now.mono_ms);
            state.pending = pending;
            let refresh = now.mono_ms.saturating_sub(state.last_refresh_ms) >= REFRESH_MS;
            if refresh {
                state.last_refresh_ms = now.mono_ms;
            }
            let poll_idle = state
                .last_idle_poll_ms
                .is_none_or(|last| now.mono_ms.saturating_sub(last) >= IDLE_POLL_MS);
            if poll_idle {
                state.last_idle_poll_ms = Some(now.mono_ms);
            }
            state.journal.prune(now.unix_ms);
            let restore_ms = config.restore_window.as_millis();
            forget_details(&mut state.activity, now.unix_ms, restore_ms);
            state.forget_guard_episode(now);
            let State {
                acted,
                left_open,
                index,
                ..
            } = &mut *state;
            acted.retain(|id, _| index.contains(*id));
            left_open.retain(|id| index.contains(*id));
            (out, due, refresh, poll_idle)
        };
        if refresh {
            for provider in &self.providers {
                provider.refresh();
            }
            self.refresh_permissions();
        }
        let idle = (poll_idle || !due.is_empty())
            .then(|| (self.idle_clock)())
            .flatten();
        if poll_idle && let Some(idle) = idle {
            let mut state = lock(&self.state);
            let paused = state.automatic_off(now);
            let config = Arc::clone(&state.config);
            out.fire
                .extend(state.fusion.on_idle(idle, now, &config, paused).fire);
        }
        self.apply(out);
        for pending in due {
            if lock(&self.state).automatic_off(now) {
                continue;
            }
            let back_at_keyboard = matches!(pending.fired, FiredTrigger::Idle { .. })
                && idle.is_some_and(|idle| idle < pending.countdown.0);
            if back_at_keyboard {
                continue;
            }
            self.run(RunRequest {
                countdowns: Countdowns::Due(pending.countdown),
                ..RunRequest::new(pending.fired, RunKind::Automatic)
            });
        }
        self.publish_status();
    }

    /// Re-plans new and changed targets against the rules armed by the active share or
    /// projector, and acts at once.
    fn guard(&self, ids: &mut BTreeSet<TargetId>) {
        if ids.is_empty() {
            return;
        }
        let ids = std::mem::take(ids);
        let now = self.now();
        let (triggers, mut scope) = {
            let state = lock(&self.state);
            if state.automatic_off(now) {
                return;
            }
            let triggers = state.fusion.guard_triggers(&state.config);
            let scope: BTreeSet<TargetId> = ids
                .into_iter()
                .filter(|id| {
                    state
                        .index
                        .get(*id)
                        .is_some_and(|entry| state.acted.get(id) != Some(&entry.target.kind))
                })
                .collect();
            (triggers, scope)
        };
        for fired in triggers {
            if scope.is_empty() {
                break;
            }
            let (_, planned) = self.run(RunRequest {
                scope: Some(scope.clone()),
                ..RunRequest::new(fired, RunKind::Automatic)
            });
            scope.retain(|id| !planned.contains(id));
        }
    }

    /// Waits until the providers and the fusion thread have worked through everything sent
    /// so far, including the runs and events that causes.
    pub fn settle(&self) {
        for _ in 0..50 {
            let before = self.processed.load(Ordering::SeqCst);
            for provider in &self.providers {
                provider.flush(Duration::from_secs(10));
            }
            let (ack, done) = bounded(1);
            if self.tx.send(Msg::Flush(ack)).is_err() || done.recv().is_err() {
                return;
            }
            if self.processed.load(Ordering::SeqCst) == before + 1 {
                return;
            }
        }
    }

    pub fn request_tick(&self) {
        let (ack, done) = bounded(1);
        if self.tx.send(Msg::Tick(Some(ack))).is_ok() {
            let _ = done.recv();
        }
    }
}
