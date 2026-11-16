use std::path::PathBuf;
use std::sync::Arc;

use closecall_core::FiredTrigger;
use closecall_core::config::{Config, ValidationIssue};
use closecall_core::ipc::{CapabilityReport, CategoryInfo, Preset, RunReport, Status};
use closecall_core::platform::{Permission, PermissionState, Platform, RequestOutcome};

pub struct EngineSettings {
    /// `config.toml` lives here; created on first save.
    pub config_dir: PathBuf,
    pub home: PathBuf,
    pub app_version: String,
    /// Our own bundle id / exe, never acted on.
    pub own_app_id: String,
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

struct Inner;

impl Engine {
    pub fn new(
        platform: Box<dyn Platform>,
        settings: EngineSettings,
        sink: Arc<dyn EngineSink>,
    ) -> Result<Self, EngineError> {
        todo!(
            "closecall-engine: new {} {:?}",
            platform.name(),
            settings.config_dir
        )
    }

    /// Must be called on the main thread once its run loop is running (Tauri `setup`).
    pub fn start_signals(&self) {
        todo!("closecall-engine: start_signals")
    }

    pub fn config(&self) -> Config {
        todo!()
    }

    pub fn save_config(&self, config: Config) -> Result<Vec<ValidationIssue>, EngineError> {
        todo!("closecall-engine: save_config {}", config.rules.len())
    }

    pub fn reset_config(&self, preset: Preset) -> Result<Config, EngineError> {
        todo!("closecall-engine: reset_config {preset:?}")
    }

    pub fn status(&self) -> Status {
        todo!()
    }

    pub fn preview(&self, trigger: FiredTrigger) -> RunReport {
        todo!("closecall-engine: preview {trigger:?}")
    }

    pub fn panic(&self) -> RunReport {
        todo!()
    }

    pub fn simulate(&self, trigger: FiredTrigger) -> RunReport {
        todo!("closecall-engine: simulate {trigger:?}")
    }

    pub fn activity(&self) -> Vec<RunReport> {
        todo!()
    }

    pub fn restore(&self, run_id: u64) -> Result<RunReport, EngineError> {
        todo!("closecall-engine: restore {run_id}")
    }

    pub fn permissions(&self) -> Vec<PermissionState> {
        todo!()
    }

    pub fn request_permission(&self, permission: &Permission) -> RequestOutcome {
        todo!("closecall-engine: request_permission {permission:?}")
    }

    pub fn capabilities(&self) -> CapabilityReport {
        todo!()
    }

    pub fn categories(&self) -> Vec<CategoryInfo> {
        todo!()
    }

    pub fn pause(&self, minutes: u32) -> Status {
        todo!("closecall-engine: pause {minutes}")
    }

    pub fn resume(&self) -> Status {
        todo!()
    }

    /// Redacted capability and permission report for `closecall diag` and bug reports.
    pub fn diag(&self) -> String {
        todo!()
    }
}

/// The platform for the OS we were built for, or the scripted fake with feature `fake`.
pub fn default_platform() -> Box<dyn Platform> {
    todo!()
}
