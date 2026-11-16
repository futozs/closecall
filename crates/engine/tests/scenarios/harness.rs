use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::ipc::{RunReport, Status};
use closecall_core::platform::SignalKind;
use closecall_core::{
    Action, CategoryId, Confidence, Confirm, FiredTrigger, Matcher, Rule, RuleId, StepOutcome,
    TargetKindTag, Trigger, UnsavedPolicy,
};
use closecall_engine::{Engine, EngineOptions, EngineSettings, EngineSink, ManualClock, Notice};
use closecall_platform_fake::{FakeController, FakePlatform};

pub const HOME: &str = "/Users/demo";
pub const OWN_APP: &str = "hu.futozs.closecall";

#[derive(Default)]
pub struct Recorder {
    statuses: Mutex<Vec<Status>>,
    runs: Mutex<Vec<RunReport>>,
    notices: Mutex<Vec<Notice>>,
}

impl Recorder {
    pub fn runs(&self) -> Vec<RunReport> {
        self.runs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn notices(&self) -> Vec<Notice> {
        self.notices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn statuses(&self) -> Vec<Status> {
        self.statuses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn runs_for(&self, matches: impl Fn(&FiredTrigger) -> bool) -> Vec<RunReport> {
        self.runs()
            .into_iter()
            .filter(|run| matches(&run.trigger))
            .collect()
    }
}

impl EngineSink for Recorder {
    fn status_changed(&self, status: &Status) {
        self.statuses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(status.clone());
    }

    fn run_completed(&self, report: &RunReport) {
        self.runs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(report.clone());
    }

    fn notify(&self, notice: &Notice) {
        self.notices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(notice.clone());
    }
}

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "engine-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub struct Setup {
    pub demo: bool,
    pub config_file: Option<String>,
    pub options: EngineOptions,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            demo: true,
            config_file: None,
            options: EngineOptions::default(),
        }
    }
}

pub struct Harness {
    pub engine: Engine,
    pub fake: FakeController,
    pub clock: Arc<ManualClock>,
    pub sink: Arc<Recorder>,
    pub dir: TempDir,
}

impl Harness {
    /// The demo inventory and the recommended preset, with no config file on disk.
    pub fn demo() -> Self {
        Self::start(Setup::default())
    }

    pub fn with_config(config: &Config) -> Self {
        Self::start(Setup {
            config_file: Some(config.to_toml()),
            ..Setup::default()
        })
    }

    pub fn start(setup: Setup) -> Self {
        let dir = TempDir::new();
        if let Some(text) = &setup.config_file {
            fs::create_dir_all(dir.path()).expect("temp dir");
            fs::write(dir.path().join("config.toml"), text).expect("config file");
        }
        let platform = if setup.demo {
            FakePlatform::demo()
        } else {
            FakePlatform::new()
        };
        let fake = platform.controller();
        let clock = Arc::new(ManualClock::new());
        let sink = Arc::new(Recorder::default());
        let engine = Engine::with_options(
            Box::new(platform),
            EngineSettings {
                config_dir: dir.path().to_path_buf(),
                home: PathBuf::from(HOME),
                app_version: "0.1.0-test".to_owned(),
                own_app_id: OWN_APP.to_owned(),
            },
            sink.clone(),
            EngineOptions {
                clock: clock.clone(),
                background_ticks: false,
                ..setup.options
            },
        )
        .expect("engine starts");
        engine.start_signals();
        engine.settle();
        Self {
            engine,
            fake,
            clock,
            sink,
            dir,
        }
    }

    pub fn signal(&self, kind: SignalKind) {
        assert!(self.fake.signal(kind), "{kind:?} source not started");
        self.engine.settle();
    }

    pub fn share(&self, confidence: Confidence, capturing_app: Option<&str>) {
        assert!(self.fake.capture_started(confidence, capturing_app));
        self.engine.settle();
    }

    /// Moves both clocks, runs the timers and waits for what they caused.
    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
        self.engine.tick();
        self.engine.settle();
    }

    pub fn closed(&self, key: &str) -> bool {
        self.fake.executed().iter().any(|step| {
            step.key == key
                && matches!(step.action, Action::Close | Action::Quit)
                && step.outcome == StepOutcome::Done
        })
    }

    pub fn touched(&self, key: &str) -> bool {
        self.fake.executed().iter().any(|step| step.key == key)
    }

    pub fn config_on_disk(&self) -> Option<String> {
        fs::read_to_string(self.dir.path().join("config.toml")).ok()
    }
}

pub fn outcome_of<'a>(report: &'a RunReport, title: &str) -> Option<&'a StepOutcome> {
    report
        .steps
        .iter()
        .find(|step| step.summary.title == title)
        .and_then(|step| step.outcome.as_ref())
}

pub fn rule(
    id: &str,
    when: Vec<Trigger>,
    targets: Vec<TargetKindTag>,
    matcher: Matcher,
    action: Action,
) -> Rule {
    Rule {
        id: RuleId(id.to_owned()),
        name: None,
        enabled: true,
        when,
        targets,
        matcher,
        except: None,
        action,
        on_unsaved: UnsavedPolicy::default(),
        confirm: Confirm::Never,
    }
}

pub fn category(id: &str) -> Matcher {
    Matcher::Category(CategoryId(id.to_owned()))
}

pub fn recommended_with(extra: Vec<Rule>) -> Config {
    let mut config = Config::recommended();
    config.rules.extend(extra);
    config
}

pub fn skipped(reason_key: &str) -> StepOutcome {
    StepOutcome::Skipped {
        reason_key: reason_key.to_owned(),
    }
}

pub fn hidden_because(reason_key: &str) -> StepOutcome {
    StepOutcome::FellBack {
        action: Action::Hide,
        reason_key: reason_key.to_owned(),
    }
}
