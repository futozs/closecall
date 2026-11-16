mod actor;
pub mod clock;
mod diag;
mod exec;
mod fusion;
mod index;
mod runtime;
mod store;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use closecall_core::config::{Config, ValidationIssue};
use closecall_core::ipc::{CapabilityReport, CategoryInfo, Preset, RunReport, Status};
use closecall_core::platform::{
    EventSender, Permission, PermissionState, Platform, RequestOutcome, Signal, SignalError,
};
use closecall_core::{AppRef, DisplayInfo, FiredTrigger};
use closecall_lists::Lists;
use crossbeam_channel::{Sender, unbounded};

pub use clock::{Clock, ManualClock, SystemClock};

use crate::actor::ProviderHandle;
use crate::exec::{Journal, RunKind, RunRequest};
use crate::fusion::Fusion;
use crate::index::Index;
use crate::runtime::{Core, Msg, State, lock};
use crate::store::Loaded;

pub struct EngineSettings {
    /// `config.toml` lives here; created on first save.
    pub config_dir: PathBuf,
    pub home: PathBuf,
    pub app_version: String,
    /// Our own bundle id / exe, never acted on.
    pub own_app_id: String,
}

/// How long a panic right after start-up waits for the first inventory: without it a
/// `closecall panic` on a cold start would plan against nothing.
const PANIC_READY_WAIT: Duration = Duration::from_secs(3);

/// Defaults are what the app runs with; tests swap in a [`ManualClock`] and short deadlines.
pub struct EngineOptions {
    pub clock: Arc<dyn Clock>,
    pub conceal_deadline: Duration,
    pub tab_deadline: Duration,
    pub document_deadline: Duration,
    /// How long past a step's deadline the engine still waits for the provider's answer.
    pub reply_grace: Duration,
    /// Off: timers only run on [`Engine::tick`], so a test decides when time passes.
    pub background_ticks: bool,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            clock: Arc::new(SystemClock::new()),
            conceal_deadline: Duration::from_millis(300),
            tab_deadline: Duration::from_secs(1),
            document_deadline: Duration::from_secs(2),
            reply_grace: Duration::from_millis(250),
            background_ticks: true,
        }
    }
}

/// Notifications carry counts only, never titles, URLs or paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    RunSummary {
        done: u32,
        hidden: u32,
        skipped: u32,
    },
    UnsavedHidden {
        count: u32,
    },
    PermissionMissing,
    ShareDetected,
}

/// Implemented by the app to forward engine events to the UI and the OS.
pub trait EngineSink: Send + Sync + 'static {
    fn status_changed(&self, status: &Status);
    fn run_completed(&self, report: &RunReport);
    fn notify(&self, notice: &Notice);
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("could not read or write the config: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] closecall_core::config::ConfigError),
    #[error("run {0} is unknown or no longer restorable")]
    NotRestorable(u64),
}

/// Cheap to clone; all methods may block for up to a few seconds (a run), so call them
/// off the UI thread.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

/// Dropping the last `Engine` stops the fusion thread; the provider threads end with it.
struct Inner {
    core: Arc<Core>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        let _ = self.core.tx.send(Msg::Shutdown);
    }
}

struct SignalForward {
    tx: Sender<Msg>,
}

impl EventSender<Signal> for SignalForward {
    fn send(&self, signal: Signal) -> bool {
        self.tx.send(Msg::Signal(signal)).is_ok()
    }
}

impl Engine {
    pub fn new(
        platform: Box<dyn Platform>,
        settings: EngineSettings,
        sink: Arc<dyn EngineSink>,
    ) -> Result<Self, EngineError> {
        Self::with_options(platform, settings, sink, EngineOptions::default())
    }

    /// A missing config means the recommended preset and onboarding; an invalid one is left on
    /// disk untouched and reported in `Status.config_problem` and by `diag` while the
    /// recommended preset runs. The first save keeps it as `config.rejected.toml`.
    pub fn with_options(
        platform: Box<dyn Platform>,
        settings: EngineSettings,
        sink: Arc<dyn EngineSink>,
        options: EngineOptions,
    ) -> Result<Self, EngineError> {
        let (config, config_problem, needs_onboarding) = match store::load(&settings.config_dir) {
            Loaded::Config(config) => (*config, None, false),
            Loaded::Missing => (Config::recommended(), None, true),
            Loaded::Rejected(problem) => {
                tracing::warn!(?problem, "config not used, running the recommended preset");
                (Config::recommended(), Some(problem), false)
            }
        };
        let lists = Lists::bundled().with_user_categories(&config.categories);

        let (tx, rx) = unbounded();
        let providers = platform
            .providers()
            .into_iter()
            .enumerate()
            .map(|(index, provider)| ProviderHandle::spawn(index, provider, tx.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let provider_count = providers.len();
        let now_ms = options.clock.mono_ms();
        let core = Arc::new(Core {
            state: Mutex::new(State {
                config: Arc::new(config),
                lists: Arc::new(lists),
                config_problem,
                needs_onboarding,
                index: Index::default(),
                capabilities: vec![Vec::new(); provider_count],
                permission_missing: vec![false; provider_count],
                fusion: Fusion::default(),
                paused_until_ms: None,
                pending: Vec::new(),
                journal: Journal::default(),
                activity: VecDeque::new(),
                last_run: None,
                last_run_id: 0,
                last_refresh_ms: now_ms,
                last_idle_poll_ms: None,
                acted: std::collections::HashMap::new(),
                left_open: std::collections::HashSet::new(),
                guarding: std::collections::BTreeSet::new(),
                reported: vec![false; provider_count],
            }),
            ready: Condvar::new(),
            run_lock: Mutex::new(()),
            save_lock: Mutex::new(()),
            providers,
            sources: Mutex::new(platform.signal_sources()),
            signals_started: AtomicBool::new(false),
            broker: Mutex::new(platform.permissions()),
            permissions: Mutex::new(Vec::new()),
            idle_clock: platform.idle_clock(),
            running_apps: platform.running_apps(),
            displays: platform.displays(),
            sink,
            home: settings.home.to_string_lossy().into_owned(),
            settings,
            platform: platform.name(),
            options,
            tx,
            processed: AtomicU64::new(0),
            last_status: Mutex::new(None),
        });
        core.refresh_permissions();
        let fusion = Arc::clone(&core);
        thread::Builder::new()
            .name("closecall-engine".to_owned())
            .spawn(move || fusion.fusion_loop(&rx))?;
        Ok(Self {
            inner: Arc::new(Inner { core }),
        })
    }

    fn core(&self) -> &Core {
        &self.inner.core
    }

    /// Must be called on the main thread once its run loop is running (Tauri `setup`).
    pub fn start_signals(&self) {
        let core = self.core();
        if core
            .signals_started
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        for source in lock(&core.sources).iter_mut() {
            let sender = Box::new(SignalForward {
                tx: core.tx.clone(),
            });
            if let Err(error) = source.start(sender) {
                let error = match error {
                    SignalError::Unavailable(_) => "unavailable",
                    SignalError::Other(_) => "other",
                };
                tracing::warn!(source = %source.id(), error, "signal source did not start");
            }
        }
        core.publish_status();
    }

    pub fn config(&self) -> Config {
        Config::clone(&lock(&self.core().state).config)
    }

    /// Issues come back without touching the file; an empty list means it was written.
    pub fn save_config(&self, config: Config) -> Result<Vec<ValidationIssue>, EngineError> {
        let issues = config.validate();
        if !issues.is_empty() {
            return Ok(issues);
        }
        let core = self.core();
        let lists = Lists::bundled().with_user_categories(&config.categories);
        {
            let _serial = lock(&core.save_lock);
            let rejected = lock(&core.state).config_problem.is_some();
            if rejected {
                store::keep_rejected(&core.settings.config_dir)?;
            }
            store::save(&core.settings.config_dir, &config)?;
            let mut state = lock(&core.state);
            state.config = Arc::new(config);
            state.lists = Arc::new(lists);
            state.config_problem = None;
            state.needs_onboarding = false;
        }
        let _ = core.tx.send(Msg::Reattribute);
        core.publish_status();
        Ok(Vec::new())
    }

    /// Not saved: the settings window edits the result and saves it.
    pub fn reset_config(&self, preset: Preset) -> Result<Config, EngineError> {
        Ok(match preset {
            Preset::Recommended => Config::recommended(),
            Preset::Empty => Config::default(),
        })
    }

    pub fn status(&self) -> Status {
        let core = self.core();
        core.refresh_permissions();
        core.publish_status();
        core.status()
    }

    pub fn preview(&self, trigger: FiredTrigger) -> RunReport {
        self.core()
            .run(RunRequest::new(trigger, RunKind::Preview))
            .0
    }

    /// Right after start-up this first waits (briefly) for the providers' first inventory.
    pub fn panic(&self) -> RunReport {
        let core = self.core();
        core.wait_ready(PANIC_READY_WAIT);
        core.run(RunRequest::new(FiredTrigger::Panic, RunKind::Panic))
            .0
    }

    /// Waits until every provider has delivered its first inventory, at most `max`. `false`
    /// when one has not by then.
    pub fn wait_ready(&self, max: Duration) -> bool {
        self.core().wait_ready(max)
    }

    pub fn simulate(&self, trigger: FiredTrigger) -> RunReport {
        self.core().run(RunRequest::new(trigger, RunKind::Manual)).0
    }

    /// Newest first, at most 50, memory only.
    pub fn activity(&self) -> Vec<RunReport> {
        let core = self.core();
        let now = core.now();
        let state = lock(&core.state);
        state
            .activity
            .iter()
            .map(|report| RunReport {
                restorable: !report.dry_run && state.journal.restorable(report.run_id, now.unix_ms),
                ..report.clone()
            })
            .collect()
    }

    pub fn restore(&self, run_id: u64) -> Result<RunReport, EngineError> {
        self.core().restore(run_id)
    }

    pub fn permissions(&self) -> Vec<PermissionState> {
        let core = self.core();
        core.refresh_permissions();
        core.publish_status();
        lock(&core.permissions).clone()
    }

    /// Never prompts while a share or projector is active: the prompt would be on screen.
    /// The request may block until the user answers; the engine keeps running meanwhile.
    pub fn request_permission(&self, permission: &Permission) -> RequestOutcome {
        let core = self.core();
        if core.sharing() {
            return RequestOutcome::NotWhileSharing;
        }
        let outcome = lock(&core.broker).request(permission);
        core.refresh_permissions();
        core.publish_status();
        outcome
    }

    pub fn capabilities(&self) -> CapabilityReport {
        let core = self.core();
        CapabilityReport {
            platform: core.platform.to_owned(),
            capabilities: core.capabilities(),
            signal_sources: core.source_statuses(),
        }
    }

    pub fn categories(&self) -> Vec<CategoryInfo> {
        lock(&self.core().state).lists.infos()
    }

    /// Connected displays, the built-in one included.
    pub fn displays(&self) -> Vec<DisplayInfo> {
        (self.core().displays)()
    }

    /// Running apps, background agents included, to pick from for the ignore list.
    pub fn running_apps(&self) -> Vec<AppRef> {
        (self.core().running_apps)()
    }

    /// `0` minutes resumes.
    pub fn pause(&self, minutes: u32) -> Status {
        if minutes == 0 {
            return self.resume();
        }
        let core = self.core();
        let until = core
            .now()
            .unix_ms
            .saturating_add(u64::from(minutes) * 60_000);
        lock(&core.state).paused_until_ms = Some(until);
        core.publish_status();
        core.status()
    }

    /// A share that is already on gets its sweep once the fusion thread sees the pause gone.
    pub fn resume(&self) -> Status {
        let core = self.core();
        lock(&core.state).paused_until_ms = None;
        let _ = core.tx.send(Msg::Tick(None));
        core.publish_status();
        core.status()
    }

    /// Redacted capability and permission report for `closecall diag` and bug reports.
    pub fn diag(&self) -> String {
        self.core().diag()
    }

    /// Runs the timers now (share-stop debounce, away, idle, item age, countdowns, the 30 s
    /// refresh) and returns once they and the runs they caused are done.
    pub fn tick(&self) {
        self.core().request_tick();
    }

    /// Returns once the providers and the engine have handled everything sent so far.
    pub fn settle(&self) {
        self.core().settle();
    }
}

/// The platform for the OS we were built for, or the scripted fake with feature `fake`.
#[cfg(feature = "fake")]
pub fn default_platform() -> Box<dyn Platform> {
    Box::new(closecall_platform_fake::FakePlatform::demo())
}

/// The platform for the OS we were built for, or the scripted fake with feature `fake`.
#[cfg(all(not(feature = "fake"), target_os = "macos"))]
pub fn default_platform() -> Box<dyn Platform> {
    Box::new(closecall_platform_macos::MacPlatform::new())
}

/// The Windows (v0.2) and Linux (v0.3) adapters expose no platform yet, so the engine runs on
/// an empty fake there: no providers, no signal sources, nothing acted on.
#[cfg(all(not(feature = "fake"), not(target_os = "macos")))]
pub fn default_platform() -> Box<dyn Platform> {
    Box::new(closecall_platform_fake::FakePlatform::empty())
}
