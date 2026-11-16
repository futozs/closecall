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
//! | `restore`            | `{ run_id: u64 }`             | `RunReport`                     |
//! | `get_permissions`    | –                             | `Vec<PermissionState>`          |
//! | `request_permission` | `{ permission: Permission }`  | `RequestOutcome`                |
//! | `get_capabilities`   | –                             | `CapabilityReport`              |
//! | `list_categories`    | –                             | `Vec<CategoryInfo>`             |
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
    pub paused_until_ms: Option<u64>,
    pub last_run: Option<RunSummary>,
    pub permissions: Vec<PermissionState>,
    pub signal_sources: Vec<SignalSourceStatus>,
    pub needs_onboarding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShareState {
    pub active: bool,
    pub confidence: Option<Confidence>,
    pub mirrored_display: bool,
    pub call_likely: bool,
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
    pub run_id: u64,
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
    pub run_id: u64,
    pub at_ms: u64,
    pub trigger: FiredTrigger,
    pub dry_run: bool,
    pub steps: Vec<StepReport>,
    pub restorable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StepReport {
    pub target: TargetId,
    pub summary: TargetSummary,
    pub action: Action,
    pub phase: Phase,
    pub because: Vec<Reason>,
    /// `None` in a dry run.
    pub outcome: Option<StepOutcome>,
}

/// Enough to recognise a target in the UI. Shown only inside our own window, never in
/// notifications or logs.
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
