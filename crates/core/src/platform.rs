//! Threading contract:
//! - The engine moves each `Provider` onto its own OS thread and talks to it over channels,
//!   so providers may block (with the given deadline) and keep thread-affine state
//!   (AX observers, COM apartments). Calls never come from the UI thread.
//! - `SignalSource::start` is called once on the main thread during app setup. It must not
//!   block; it registers observers on the main run loop or spawns its own threads.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    Action, AppRef, Capability, Confidence, DisplayInfo, ProviderId, Step, StepOutcome, Target,
};

pub trait EventSender<T>: Send {
    /// Returns `false` when the receiving side is gone and the source should stop.
    fn send(&self, value: T) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovered {
    pub key: String,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryEvent {
    Added(Discovered),
    Changed(Discovered),
    Removed { provider: ProviderId, key: String },
    Resync { provider: ProviderId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reverify {
    Same,
    Changed(Target),
    Gone,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("permission missing: {0:?}")]
    PermissionMissing(Permission),
    #[error("timed out")]
    Timeout,
    #[error("{0}")]
    Other(String),
}

pub struct ExecContext {
    /// Already expanded; may not exist yet.
    pub untitled_save_dir: PathBuf,
    /// This provider's targets in the current run whose step has the same action as the
    /// current one, the current target included, so the provider can hide a whole app instead
    /// of minimizing its windows one by one, or close tabs in one call per window. A target
    /// whose step is another action (a notify-only tab, a document that is only hidden) is never
    /// in it, so batching on this list never does more than each target's own step asked for.
    pub planned: Vec<Target>,
    /// The run's protected targets, from every provider (meeting tabs, the capturing app's
    /// windows). Hiding a whole app would conceal them too, so a provider never does that while
    /// one of them belongs to the same app instance.
    pub protected: Vec<Target>,
}

pub trait Provider: Send {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> Vec<Capability>;
    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError>;
    fn snapshot(&mut self, deadline: Instant) -> Result<Vec<Discovered>, ProviderError>;
    /// Re-read one target right before acting, so we never act on a tab that navigated away.
    fn reverify(&mut self, target: &Target, deadline: Instant) -> Reverify;
    fn execute(
        &mut self,
        step: &Step,
        target: &Target,
        ctx: &ExecContext,
        deadline: Instant,
    ) -> StepOutcome;
    /// Undo a step this provider performed: reopen a closed tab or document, unhide an app.
    /// `target` is the state captured before the step ran.
    fn restore(&mut self, target: &Target, action: Action, deadline: Instant) -> StepOutcome {
        let _ = (target, action, deadline);
        StepOutcome::Skipped {
            reason_key: "reason.restore_unsupported".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SignalKind {
    CaptureStarted,
    CaptureStopped,
    SharePickerOpened,
    DisplayMirrored,
    DisplayUnmirrored,
    DisplayConnected,
    DisplayDisconnected,
    CallLikely,
    CallEnded,
    WillSleep,
    DidWake,
    Locked,
    Unlocked,
    LidClosed,
    LidOpened,
    Panic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Signal {
    pub kind: SignalKind,
    pub confidence: Confidence,
    pub source: String,
    pub capturing_app: Option<String>,
    /// The display a `Display*` signal is about, when the OS identifies it.
    pub display: Option<DisplayInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "state")]
#[ts(export)]
pub enum SourceHealth {
    Ok,
    Unavailable { reason_key: String },
    Noisy { reason_key: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("{0}")]
    Other(String),
}

pub trait SignalSource: Send {
    fn id(&self) -> String;
    fn health(&self) -> SourceHealth;
    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export)]
pub enum Permission {
    Accessibility,
    Automation { bundle_id: String, app_name: String },
    Notifications,
    ToolkitAccessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    AppNotRunning,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PermissionState {
    pub permission: Permission,
    pub status: PermissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export)]
pub enum RequestOutcome {
    Granted,
    Denied,
    OpenedSettings,
    /// macOS can only ask for Automation consent while the target app runs.
    AppNotRunning,
    NotApplicable,
    /// A share or projector is active, so no prompt was shown: it would be on the shared screen.
    NotWhileSharing,
}

pub trait PermissionBroker: Send {
    fn status(&self) -> Vec<PermissionState>;
    fn request(&self, permission: &Permission) -> RequestOutcome;
}

pub trait Platform {
    fn name(&self) -> &'static str;
    fn providers(&self) -> Vec<Box<dyn Provider>>;
    fn signal_sources(&self) -> Vec<Box<dyn SignalSource>>;
    fn permissions(&self) -> Box<dyn PermissionBroker>;
    /// Time since the last keyboard or mouse input, for `IdleLongerThan`.
    fn idle_clock(&self) -> Box<dyn Fn() -> Option<Duration> + Send + Sync> {
        Box::new(|| None)
    }
    /// Running apps, background agents included, for the ignore list and to put a capture
    /// down to an ignored app.
    fn running_apps(&self) -> Box<dyn Fn() -> Vec<AppRef> + Send + Sync> {
        Box::new(Vec::new)
    }
    /// Connected displays, the built-in one included.
    fn displays(&self) -> Box<dyn Fn() -> Vec<DisplayInfo> + Send + Sync> {
        Box::new(Vec::new)
    }
}
