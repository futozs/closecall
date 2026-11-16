//! The engine's shared state and its one fusion thread, which owns signal handling, inventory
//! updates, share-guard and the timers.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::ipc::{RunReport, RunSummary, SignalSourceStatus, Status};
use closecall_core::platform::{
    Discovered, InventoryEvent, PermissionBroker, PermissionState, ProviderError, Signal,
    SignalSource,
};
use closecall_core::{AppSelector, Capability, Dur, FiredTrigger, TargetId, TargetKind};
use closecall_lists::Lists;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};

use crate::actor::{ProviderHandle, error_kind, is_permission_missing};
use crate::exec::{Countdowns, Journal, RunKind, RunRequest};
use crate::fusion::{Fusion, Now, Outcome};
use crate::index::Index;
use crate::store::Problem;
use crate::{EngineOptions, EngineSettings, EngineSink, Notice};

const REFRESH_MS: u64 = 30_000;
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
}

impl State {
    pub fn paused(&self, now: Now) -> bool {
        self.paused_until_ms
            .is_some_and(|until| until > now.unix_ms)
    }

    pub fn guard_active(&self, now: Now) -> bool {
        !self.paused(now)
            && !self
                .fusion
                .guard_triggers(self.config.share_min_confidence)
                .is_empty()
    }

    pub fn next_run_id(&mut self) -> u64 {
        self.last_run_id += 1;
        self.last_run_id
    }
}

pub(crate) struct Core {
    pub state: Mutex<State>,
    pub run_lock: Mutex<()>,
    pub providers: Vec<ProviderHandle>,
    pub sources: Mutex<Vec<Box<dyn SignalSource>>>,
    pub signals_started: AtomicBool,
    pub broker: Mutex<Box<dyn PermissionBroker>>,
    pub permissions: Mutex<Vec<PermissionState>>,
    pub idle_clock: Box<dyn Fn() -> Option<Duration> + Send + Sync>,
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

    /// Our own app always, plus the app that is capturing the screen when the OS names it.
    pub fn protect_apps(&self, capturing_app: Option<&str>) -> Vec<AppSelector> {
        std::iter::once(self.settings.own_app_id.as_str())
            .chain(capturing_app)
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
            share: state.fusion.share_state(state.config.share_min_confidence),
            guard_active: state.guard_active(now),
            paused_until_ms,
            last_run: state.last_run.clone(),
            permissions,
            signal_sources,
            needs_onboarding: state.needs_onboarding,
        }
    }

    pub fn source_statuses(&self) -> Vec<SignalSourceStatus> {
        lock(&self.sources)
            .iter()
            .map(|source| SignalSourceStatus {
                id: source.id(),
                health: source.health(),
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

    pub fn refresh_permissions(&self) {
        let fresh = lock(&self.broker).status();
        *lock(&self.permissions) = fresh;
    }

    pub fn sharing(&self) -> bool {
        let state = lock(&self.state);
        let share = state.fusion.share_state(state.config.share_min_confidence);
        share.active || share.mirrored_display
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
        let now = self.now();
        let out = {
            let mut state = lock(&self.state);
            let paused = state.paused(now);
            let config = Arc::clone(&state.config);
            let out = state.fusion.on_signal(signal, now, &config, paused);
            if !state.guard_active(now) {
                state.acted.clear();
            }
            out
        };
        tracing::debug!(kind = ?signal.kind, fired = out.fire.len(), "signal");
        self.apply(out);
    }

    /// Share detection reaches the app before the run starts, so it can hide its own windows.
    fn apply(&self, out: Outcome) {
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
            let paused = state.paused(now);
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
            if !state.guard_active(now) {
                state.acted.clear();
            }
            let State { acted, index, .. } = &mut *state;
            acted.retain(|id, _| index.contains(*id));
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
            let paused = state.paused(now);
            let config = Arc::clone(&state.config);
            out.fire
                .extend(state.fusion.on_idle(idle, now, &config, paused).fire);
        }
        self.apply(out);
        for pending in due {
            if lock(&self.state).paused(now) {
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
            if state.paused(now) {
                return;
            }
            let triggers = state
                .fusion
                .guard_triggers(state.config.share_min_confidence);
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
