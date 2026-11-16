//! Commands are Tauri commands with these exact names; arguments and results are the
//! types below (exported to `app/src/bindings` with ts-rs). Events are emitted with
//! the names in `events`. `app/src/ipc.ts` is the only place in the UI that talks to Rust.
//! | command              | args                          | returns                         |
//! | `get_config`         | –                             | `Config`                        |
//! | `save_config`        | `{ config: Config }`          | `Vec<ValidationIssue>` (empty = saved) |
//! | `reset_config`       | `{ preset: Preset }`          | `Config`                        |
//! | `get_status`         | –                             | `Status`                        |
//! | `preview`            | `{ trigger: FiredTrigger }`   | `RunReport` (dry run, nothing touched) |
//! | `run_panic`          | –                             | `RunReport`                     |
//! | `simulate`           | `{ trigger: FiredTrigger }`   | `RunReport` (really acts)       |
//! | `get_activity`       | –                             | `Vec<RunReport>` (newest first, memory only) |
//! | `restore`            | `{ run_id: u64 }`             | `RunReport` (a new run, `restore_of = run_id`) |
//! | `get_permissions`    | –                             | `Vec<PermissionState>`          |
//! | `request_permission` | `{ permission: Permission }`  | `RequestOutcome` (`not_while_sharing` during a share) |
//! | `get_capabilities`   | –                             | `CapabilityReport`              |
//! | `list_categories`    | –                             | `Vec<CategoryInfo>`             |
//! | `list_displays`      | –                             | `Vec<DisplayInfo>`              |
//! | `list_running_apps`  | –                             | `Vec<AppRef>` (background agents included) |
//! | `pause`              | `{ minutes: u32 }`            | `Status`                        |
//! | `resume`             | –                             | `Status`                        |

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::platform::{PermissionState, SourceHealth};
use crate::{
    Action, Capability, CategoryId, Confidence, FiredTrigger, Phase, Reason, StepOutcome, TargetId,
    TargetKindTag,
};

pub mod events {
    pub const STATUS: &str = "closecall://status";
    pub const RUN: &str = "closecall://run";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Preset {
    Recommended,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Status {
    pub platform: String,
    pub app_version: String,
    pub share: ShareState,
    pub guard_active: bool,
    #[ts(type = "number | null")]
    pub paused_until_ms: Option<u64>,
    pub last_run: Option<RunSummary>,
    pub permissions: Vec<PermissionState>,
    pub signal_sources: Vec<SignalSourceStatus>,
    pub needs_onboarding: bool,
    /// Message key (`config.problem.*`) when `config.toml` was rejected and the recommended
    /// preset runs instead. The file is left as it is until the user saves.
    pub config_problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShareState {
    pub active: bool,
    pub confidence: Option<Confidence>,
    pub mirrored_display: bool,
    /// An external display that is not ignored is connected, and a rule treats that like a projector.
    pub external_display: bool,
    pub call_likely: bool,
    /// Name of the ignored app that holds the OS's capture signal, which therefore cannot tell
    /// a new share apart.
    pub ignored_capture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SignalSourceStatus {
    pub id: String,
    pub health: SourceHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunSummary {
    #[ts(type = "number")]
    pub run_id: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    pub trigger: FiredTrigger,
    pub done: u32,
    pub fell_back: u32,
    pub skipped: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunReport {
    #[ts(type = "number")]
    pub run_id: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    pub trigger: FiredTrigger,
    pub dry_run: bool,
    pub steps: Vec<StepReport>,
    pub restorable: bool,
    /// Set on the report of a restore: the run it undid. Its steps carry the original actions.
    #[ts(type = "number | null")]
    pub restore_of: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StepReport {
    pub target: TargetId,
    pub summary: TargetSummary,
    pub action: Action,
    pub phase: Phase,
    /// Empty once the run's restore window has ended: evidence can name the site that matched.
    pub because: Vec<Reason>,
    /// `None` in a dry run.
    pub outcome: Option<StepOutcome>,
}

/// Enough to recognise a target in the UI. Shown only inside our own window, never in
/// notifications or logs. Title, URL and path are cleared once the run's restore window ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TargetSummary {
    pub kind: TargetKindTag,
    pub app_name: String,
    pub title: String,
    pub url: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CapabilityReport {
    pub platform: String,
    pub capabilities: Vec<Capability>,
    pub signal_sources: Vec<SignalSourceStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CategoryInfo {
    pub id: CategoryId,
    pub builtin: bool,
    pub default_enabled: bool,
    pub domain_count: u32,
    pub app_count: u32,
    pub keyword_count: u32,
    pub sources: Vec<ListSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListSource {
    pub id: String,
    pub name: String,
    pub url: String,
    pub license: String,
    pub snapshot_date: String,
    pub entries: u32,
}
