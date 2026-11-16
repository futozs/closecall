//! A scripted platform for engine tests, UI development and demos. `FakePlatform::demo()` starts
//! with a fixed inventory; a [`FakeController`] changes it, injects signals and records what the
//! engine asked the providers to do.

mod controller;
mod provider;
mod targets;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use closecall_core::platform::{
    EventSender, InventoryEvent, Permission, PermissionBroker, PermissionState, PermissionStatus,
    Platform, Provider, RequestOutcome, Signal, SignalError, SignalSource, SourceHealth,
};
use closecall_core::{Action, StepOutcome, Target, UnsavedPolicy};

pub use controller::FakeController;
pub use targets::{
    app_window, chrome_incognito_tab, chrome_tab, demo_inventory, document, process, safari_tab,
};

pub const CHROMIUM: &str = "fake.chromium";
pub const SAFARI: &str = "fake.safari";
pub const DOCUMENTS: &str = "fake.documents";
pub const APPS: &str = "fake.apps";
pub const PROVIDERS: [&str; 4] = [CHROMIUM, SAFARI, DOCUMENTS, APPS];

/// Ids of the fake signal sources; each carries the signal kinds of one real source.
pub const SOURCES: [&str; 6] = [
    "share_watcher",
    "call_audio",
    "display",
    "power",
    "lock",
    "hotkey",
];

/// One `execute` call as the provider saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executed {
    pub provider: String,
    pub key: String,
    pub action: Action,
    pub on_unsaved: UnsavedPolicy,
    pub outcome: StepOutcome,
    pub untitled_save_dir: std::path::PathBuf,
    /// Keys of every target this provider was told about for the run.
    pub planned: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub provider: String,
    pub key: String,
    pub action: Action,
    pub outcome: StepOutcome,
}

pub struct FakePlatform {
    shared: Arc<Shared>,
    with_providers: bool,
}

impl FakePlatform {
    /// Providers and signal sources, but nothing open yet.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared::default()),
            with_providers: true,
        }
    }

    /// The scripted demo inventory: sensitive tabs, a meeting tab, invoices and private apps.
    pub fn demo() -> Self {
        let platform = Self::new();
        {
            let mut state = platform.shared.state();
            for target in demo_inventory() {
                let key = targets::key_of(&target);
                state.items.insert(key, Item::new(target));
            }
        }
        platform
    }

    /// No providers and no signal sources: what the engine runs on where no adapter exists yet.
    pub fn empty() -> Self {
        Self {
            shared: Arc::new(Shared::default()),
            with_providers: false,
        }
    }

    pub fn controller(&self) -> FakeController {
        FakeController::new(Arc::clone(&self.shared))
    }
}

impl Default for FakePlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for FakePlatform {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn providers(&self) -> Vec<Box<dyn Provider>> {
        if !self.with_providers {
            return Vec::new();
        }
        PROVIDERS
            .iter()
            .map(|id| {
                Box::new(provider::FakeProvider::new(id, Arc::clone(&self.shared)))
                    as Box<dyn Provider>
            })
            .collect()
    }

    fn signal_sources(&self) -> Vec<Box<dyn SignalSource>> {
        if !self.with_providers {
            return Vec::new();
        }
        SOURCES
            .iter()
            .map(|id| {
                Box::new(FakeSource {
                    id,
                    shared: Arc::clone(&self.shared),
                }) as Box<dyn SignalSource>
            })
            .collect()
    }

    fn permissions(&self) -> Box<dyn PermissionBroker> {
        Box::new(FakeBroker {
            shared: Arc::clone(&self.shared),
        })
    }

    fn idle_clock(&self) -> Box<dyn Fn() -> Option<Duration> + Send + Sync> {
        let shared = Arc::clone(&self.shared);
        Box::new(move || shared.state().idle)
    }
}

pub(crate) struct Item {
    pub target: Target,
    pub concealed: bool,
}

impl Item {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            concealed: false,
        }
    }
}

pub(crate) struct State {
    pub items: BTreeMap<String, Item>,
    pub closed: HashMap<String, Target>,
    pub inventory: HashMap<String, Box<dyn EventSender<InventoryEvent>>>,
    pub signals: HashMap<String, Box<dyn EventSender<Signal>>>,
    pub health: HashMap<String, SourceHealth>,
    pub permissions: Vec<PermissionState>,
    pub requested: Vec<Permission>,
    pub denied_providers: HashMap<String, Permission>,
    pub idle: Option<Duration>,
    pub executed: Vec<Executed>,
    pub restored: Vec<Restored>,
}

impl Default for State {
    fn default() -> Self {
        let automation = |bundle_id: &str, app_name: &str| PermissionState {
            permission: Permission::Automation {
                bundle_id: bundle_id.to_owned(),
                app_name: app_name.to_owned(),
            },
            status: PermissionStatus::Granted,
        };
        Self {
            items: BTreeMap::new(),
            closed: HashMap::new(),
            inventory: HashMap::new(),
            signals: HashMap::new(),
            health: HashMap::new(),
            permissions: vec![
                PermissionState {
                    permission: Permission::Accessibility,
                    status: PermissionStatus::Granted,
                },
                automation("com.google.Chrome", "Google Chrome"),
                automation("com.apple.Safari", "Safari"),
                PermissionState {
                    permission: Permission::Notifications,
                    status: PermissionStatus::Granted,
                },
            ],
            requested: Vec::new(),
            denied_providers: HashMap::new(),
            idle: None,
            executed: Vec::new(),
            restored: Vec::new(),
        }
    }
}

#[derive(Default)]
pub(crate) struct Shared {
    state: Mutex<State>,
    hung: Mutex<HashSet<String>>,
    released: Condvar,
}

impl Shared {
    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn hung(&self) -> MutexGuard<'_, HashSet<String>> {
        self.hung.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_hung(&self, provider: &str, hung: bool) {
        let mut set = self.hung();
        if hung {
            set.insert(provider.to_owned());
        } else {
            set.remove(provider);
        }
        self.released.notify_all();
    }

    /// Blocks while `provider` is hung, at most `limit`, like an app that stopped answering.
    pub fn wait_while_hung(&self, provider: &str, limit: Duration) {
        let set = self.hung();
        let _unused = self
            .released
            .wait_timeout_while(set, limit, |set| set.contains(provider))
            .unwrap_or_else(PoisonError::into_inner);
    }
}

struct FakeSource {
    id: &'static str,
    shared: Arc<Shared>,
}

impl SignalSource for FakeSource {
    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn health(&self) -> SourceHealth {
        self.shared
            .state()
            .health
            .get(self.id)
            .cloned()
            .unwrap_or(SourceHealth::Ok)
    }

    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError> {
        self.shared
            .state()
            .signals
            .insert(self.id.to_owned(), signals);
        Ok(())
    }
}

struct FakeBroker {
    shared: Arc<Shared>,
}

impl PermissionBroker for FakeBroker {
    fn status(&self) -> Vec<PermissionState> {
        self.shared.state().permissions.clone()
    }

    fn request(&self, permission: &Permission) -> RequestOutcome {
        let mut state = self.shared.state();
        state.requested.push(permission.clone());
        match state
            .permissions
            .iter_mut()
            .find(|row| row.permission == *permission)
        {
            Some(row) => {
                row.status = PermissionStatus::Granted;
                RequestOutcome::Granted
            }
            None => RequestOutcome::NotApplicable,
        }
    }
}
