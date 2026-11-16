use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Dur;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS, JsonSchema,
)]
#[ts(export)]
pub struct RuleId(pub String);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    TS,
    JsonSchema,
)]
#[ts(export)]
pub struct CategoryId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export, type = "number")]
pub struct TargetId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProviderId(pub String);

/// Only the provider that created a handle may interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Handle(pub String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Tri {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DocKind {
    Pdf,
    Image,
    Word,
    Excel,
    PowerPoint,
    Pages,
    Keynote,
    Numbers,
    Text,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Trigger {
    Panic,
    ScreenShare,
    DisplayMirrored,
    CallLikely,
    WillSleep,
    LidClosed,
    Locked,
    AwayLongerThan(Dur),
    IdleLongerThan(Dur),
    ItemOpenLongerThan(Dur),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export)]
pub enum FiredTrigger {
    Panic,
    ScreenShare { confidence: Confidence },
    DisplayMirrored,
    CallLikely,
    WillSleep,
    LidClosed,
    Locked,
    Away { away: Dur },
    Idle { idle: Dur },
    ItemAge { target: TargetId, age: Dur },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AppSelector {
    BundleId(String),
    Exe(String),
    DesktopId(String),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Matcher {
    Category(CategoryId),
    Domain(String),
    UrlGlob(String),
    TitleContains(String),
    App(AppSelector),
    FileGlob(String),
    DocKind(DocKind),
    Private,
    Any(Vec<Matcher>),
    All(Vec<Matcher>),
    Not(Box<Matcher>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export)]
pub enum Evidence {
    Category { id: CategoryId, via: Box<Evidence> },
    Domain { domain: String },
    UrlGlob { glob: String },
    TitleContains { keyword: String },
    App { app: String },
    FileGlob { glob: String },
    DocKind { doc_kind: DocKind },
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct CategoryDef {
    pub id: CategoryId,
    pub name: Option<String>,
    pub domains: Vec<String>,
    pub lists: Vec<String>,
    pub apps: Vec<AppSelector>,
    pub title_keywords: Vec<String>,
    pub file_globs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Action {
    Close,
    Hide,
    Minimize,
    Quit,
    NotifyOnly,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TS, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum UnsavedPolicy {
    /// Untitled documents go to `Config::untitled_save_dir` where the app allows it, otherwise they are hidden.
    #[default]
    SaveThenClose,
    Hide,
    SkipAndNotify,
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Confirm {
    #[default]
    Never,
    /// Only valid for idle and item-age triggers: on a share or lock there is no time to ask,
    /// and a dialog would show up on the shared screen.
    Countdown(Dur),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TargetKindTag {
    Tab,
    Document,
    AppWindow,
    Process,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct Rule {
    pub id: RuleId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub when: Vec<Trigger>,
    pub targets: Vec<TargetKindTag>,
    #[serde(rename = "match")]
    #[ts(rename = "match")]
    pub matcher: Matcher,
    #[serde(default)]
    pub except: Option<Matcher>,
    pub action: Action,
    #[serde(default)]
    pub on_unsaved: UnsavedPolicy,
    #[serde(default)]
    pub confirm: Confirm,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Target {
    pub id: TargetId,
    pub provider: ProviderId,
    pub app: AppRef,
    /// Engine monotonic clock; item age restarts when the app restarts.
    #[ts(type = "number")]
    pub first_seen_ms: u64,
    pub kind: TargetKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppRef {
    pub id: String,
    pub name: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "type")]
#[ts(export)]
pub enum TargetKind {
    BrowserTab(BrowserTab),
    DocumentWindow(DocumentWindow),
    AppWindow(AppWindow),
    Process(ProcessInfo),
}

impl TargetKind {
    pub fn tag(&self) -> TargetKindTag {
        match self {
            Self::BrowserTab(_) => TargetKindTag::Tab,
            Self::DocumentWindow(_) => TargetKindTag::Document,
            Self::AppWindow(_) => TargetKindTag::AppWindow,
            Self::Process(_) => TargetKindTag::Process,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BrowserFamily {
    Chromium,
    Safari,
    Firefox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum UrlQuality {
    Exact,
    CachedWhenActive,
    Stale,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowserTab {
    pub family: BrowserFamily,
    pub window: Handle,
    pub tab: Handle,
    pub title: String,
    pub url: Option<String>,
    pub url_quality: UrlQuality,
    pub private: Tri,
    pub active: bool,
    pub audible: Tri,
    pub pinned: Tri,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Dirty {
    Clean,
    Dirty,
    /// Treated as `Dirty`: we never risk losing work on a guess.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DocumentWindow {
    pub window: Handle,
    pub title: String,
    pub path: Option<String>,
    pub doc_kind: DocKind,
    pub dirty: Dirty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppWindow {
    pub window: Handle,
    pub title: String,
    pub minimized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProcessInfo {
    pub pid: u32,
    pub exe: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Phase {
    Conceal,
    CloseTabs,
    CloseDocuments,
    Notify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Reason {
    pub rule: RuleId,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Step {
    pub target: TargetId,
    pub action: Action,
    pub on_unsaved: UnsavedPolicy,
    pub phase: Phase,
    pub fell_back_from: Option<Action>,
    pub confirm: Confirm,
    pub because: Vec<Reason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Plan {
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export)]
pub enum StepOutcome {
    Done,
    FellBack { action: Action, reason_key: String },
    Skipped { reason_key: String },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "level")]
#[ts(export)]
pub enum Support {
    Full,
    Partial { note_key: String },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Capability {
    pub provider: ProviderId,
    pub app: Option<String>,
    pub target_kind: TargetKindTag,
    pub action: Action,
    pub support: Support,
}
