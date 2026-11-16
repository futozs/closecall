use std::sync::Arc;
use std::time::Duration;

use closecall_core::platform::{
    Discovered, InventoryEvent, Permission, PermissionStatus, Signal, SignalKind, SourceHealth,
};
use closecall_core::{AppRef, Confidence, Dirty, DisplayInfo, ProviderId, Target, TargetKind};

use crate::provider::send;
use crate::targets::key_of;
use crate::{Executed, Item, Restored, Shared};

/// Drives a [`crate::FakePlatform`] while the engine runs on it. Changes go out as inventory
/// events, like a real provider's observers, unless the method says `silently`.
#[derive(Clone)]
pub struct FakeController {
    shared: Arc<Shared>,
}

impl FakeController {
    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }

    /// High confidence, no capturing app. `false` when the source was not started.
    pub fn signal(&self, kind: SignalKind) -> bool {
        self.send_signal(Signal {
            kind,
            confidence: Confidence::High,
            source: source_for(kind).to_owned(),
            capturing_app: None,
            display: None,
        })
    }

    pub fn capture_started(&self, confidence: Confidence, capturing_app: Option<&str>) -> bool {
        self.capture_from(
            source_for(SignalKind::CaptureStarted),
            SignalKind::CaptureStarted,
            confidence,
            capturing_app,
        )
    }

    /// A capture signal labelled as coming from `source`, like a second share detector
    /// (`macos.share_ui` next to `macos.skylight`) would send it.
    pub fn capture_from(
        &self,
        source: &str,
        kind: SignalKind,
        confidence: Confidence,
        capturing_app: Option<&str>,
    ) -> bool {
        self.send_signal(Signal {
            kind,
            confidence,
            source: source.to_owned(),
            capturing_app: capturing_app.map(str::to_owned),
            display: None,
        })
    }

    /// A `Display*` signal about `display`, which also joins or leaves `displays()`.
    pub fn display_signal(&self, kind: SignalKind, display: DisplayInfo) -> bool {
        {
            let mut state = self.shared.state();
            state.displays.retain(|known| known.id != display.id);
            if kind != SignalKind::DisplayDisconnected {
                state.displays.push(display.clone());
            }
        }
        self.send_signal(Signal {
            kind,
            confidence: Confidence::High,
            source: source_for(kind).to_owned(),
            capturing_app: None,
            display: Some(display),
        })
    }

    /// Goes out through the source that owns `signal.kind`, whatever `signal.source` says.
    pub fn send_signal(&self, signal: Signal) -> bool {
        let state = self.shared.state();
        state
            .signals
            .get(source_for(signal.kind))
            .is_some_and(|sender| sender.send(signal))
    }

    /// Returns the key, which is also the target's handle.
    pub fn add(&self, target: Target) -> String {
        let key = key_of(&target);
        let mut state = self.shared.state();
        let provider = target.provider.0.clone();
        send(
            &state,
            &provider,
            InventoryEvent::Added(Discovered {
                key: key.clone(),
                target: target.clone(),
            }),
        );
        state.closed.remove(&key);
        state.items.insert(key.clone(), Item::new(target));
        key
    }

    pub fn navigate(&self, key: &str, url: &str, title: &str) -> bool {
        self.update(key, true, |target| set_page(target, url, title))
    }

    /// The engine only finds out when it re-verifies the tab.
    pub fn navigate_silently(&self, key: &str, url: &str, title: &str) -> bool {
        self.update(key, false, |target| set_page(target, url, title))
    }

    pub fn set_dirty(&self, key: &str, dirty: Dirty) -> bool {
        self.update(key, true, |target| {
            if let TargetKind::DocumentWindow(doc) = &mut target.kind {
                doc.dirty = dirty;
            }
        })
    }

    pub fn remove(&self, key: &str) -> bool {
        let mut state = self.shared.state();
        let Some(item) = state.items.remove(key) else {
            return false;
        };
        send(
            &state,
            &item.target.provider.0,
            InventoryEvent::Removed {
                provider: item.target.provider.clone(),
                key: key.to_owned(),
            },
        );
        true
    }

    /// The engine only finds out when it re-verifies the target or takes a snapshot.
    pub fn remove_silently(&self, key: &str) -> bool {
        self.shared.state().items.remove(key).is_some()
    }

    /// Every later `reverify`, `execute` and `restore` of `provider` blocks until `release`.
    pub fn hang(&self, provider: &str) {
        self.shared.set_hung(provider, true);
    }

    pub fn release(&self, provider: &str) {
        self.shared.set_hung(provider, false);
    }

    pub fn set_permission(&self, permission: &Permission, status: PermissionStatus) {
        let mut state = self.shared.state();
        match state
            .permissions
            .iter_mut()
            .find(|row| row.permission == *permission)
        {
            Some(row) => row.status = status,
            None => state
                .permissions
                .push(closecall_core::platform::PermissionState {
                    permission: permission.clone(),
                    status,
                }),
        }
    }

    /// `start` and `snapshot` of `provider` fail with `PermissionMissing` from now on; a
    /// `Resync` asks the engine to look.
    pub fn deny_provider(&self, provider: &str, permission: Permission) {
        let mut state = self.shared.state();
        state
            .denied_providers
            .insert(provider.to_owned(), permission);
        send(
            &state,
            provider,
            InventoryEvent::Resync {
                provider: ProviderId(provider.to_owned()),
            },
        );
    }

    pub fn allow_provider(&self, provider: &str) {
        let mut state = self.shared.state();
        state.denied_providers.remove(provider);
        send(
            &state,
            provider,
            InventoryEvent::Resync {
                provider: ProviderId(provider.to_owned()),
            },
        );
    }

    pub fn set_health(&self, source: &str, health: SourceHealth) {
        self.shared.state().health.insert(source.to_owned(), health);
    }

    pub fn set_idle(&self, idle: Option<Duration>) {
        self.shared.state().idle = idle;
    }

    pub fn set_running_apps(&self, apps: Vec<AppRef>) {
        self.shared.state().running_apps = apps;
    }

    pub fn executed(&self) -> Vec<Executed> {
        self.shared.state().executed.clone()
    }

    pub fn restored(&self) -> Vec<Restored> {
        self.shared.state().restored.clone()
    }

    pub fn permission_requests(&self) -> Vec<Permission> {
        self.shared.state().requested.clone()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.shared.state().items.contains_key(key)
    }

    pub fn is_concealed(&self, key: &str) -> bool {
        self.shared
            .state()
            .items
            .get(key)
            .is_some_and(|item| item.concealed)
    }

    pub fn keys(&self) -> Vec<String> {
        self.shared.state().items.keys().cloned().collect()
    }

    fn update(&self, key: &str, announce: bool, change: impl FnOnce(&mut Target)) -> bool {
        let mut state = self.shared.state();
        let Some(item) = state.items.get_mut(key) else {
            return false;
        };
        change(&mut item.target);
        let target = item.target.clone();
        if announce {
            send(
                &state,
                &target.provider.0.clone(),
                InventoryEvent::Changed(Discovered {
                    key: key.to_owned(),
                    target,
                }),
            );
        }
        true
    }
}

fn set_page(target: &mut Target, url: &str, title: &str) {
    if let TargetKind::BrowserTab(tab) = &mut target.kind {
        tab.url = Some(url.to_owned());
        tab.title = title.to_owned();
    }
}

fn source_for(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::CaptureStarted | SignalKind::CaptureStopped | SignalKind::SharePickerOpened => {
            "share_watcher"
        }
        SignalKind::CallLikely | SignalKind::CallEnded => "call_audio",
        SignalKind::DisplayMirrored
        | SignalKind::DisplayUnmirrored
        | SignalKind::DisplayConnected
        | SignalKind::DisplayDisconnected => "display",
        SignalKind::WillSleep
        | SignalKind::DidWake
        | SignalKind::LidClosed
        | SignalKind::LidOpened => "power",
        SignalKind::Locked | SignalKind::Unlocked => "lock",
        SignalKind::Panic => "hotkey",
    }
}
