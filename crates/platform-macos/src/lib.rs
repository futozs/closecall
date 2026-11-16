//! macOS adapter. Everything is behind `cfg(target_os = "macos")`, so on other systems
//! this crate is empty and the workspace still builds.

#[cfg(target_os = "macos")]
mod apps;
#[cfg(target_os = "macos")]
mod automation;
#[cfg(target_os = "macos")]
mod ax;
#[cfg(target_os = "macos")]
mod caps;
#[cfg(target_os = "macos")]
mod doc;
#[cfg(target_os = "macos")]
mod firefox_session;
#[cfg(target_os = "macos")]
mod idle;
#[cfg(target_os = "macos")]
pub mod keys;
#[cfg(target_os = "macos")]
mod observe;
#[cfg(target_os = "macos")]
mod osa;
#[cfg(target_os = "macos")]
mod permissions;
#[cfg(target_os = "macos")]
mod poll;
#[cfg(target_os = "macos")]
mod providers;
#[cfg(target_os = "macos")]
mod script;
#[cfg(target_os = "macos")]
mod sheet;
#[cfg(target_os = "macos")]
mod signals;

#[cfg(target_os = "macos")]
pub use platform::MacPlatform;

/// What the `macos.share_ui` source sees right now, for `examples/share_ui.rs`: bundle ids,
/// window layers and sizes, never a title.
#[cfg(target_os = "macos")]
pub fn share_ui_report() -> String {
    signals::share_ui_report()
}

#[cfg(target_os = "macos")]
mod platform {
    use std::time::Duration;

    use closecall_core::platform::{PermissionBroker, Platform, Provider, SignalSource};
    use closecall_core::{AppRef, DisplayInfo};

    use crate::apps;
    use crate::permissions::MacPermissions;
    use crate::providers::{
        AppProvider, ChromiumProvider, FirefoxProvider, SafariProvider, WindowProvider,
    };
    use crate::signals::{
        CallSource, DisplaySource, ScreenWatcher, SessionSource, ShareUiSource, displays,
    };

    #[derive(Debug, Default, Clone, Copy)]
    pub struct MacPlatform;

    impl MacPlatform {
        pub fn new() -> Self {
            Self
        }
    }

    impl Platform for MacPlatform {
        fn name(&self) -> &'static str {
            "macos"
        }

        /// "macos.chromium", "macos.safari", "macos.firefox", "macos.ax", "macos.apps".
        fn providers(&self) -> Vec<Box<dyn Provider>> {
            vec![
                Box::new(ChromiumProvider::new()),
                Box::new(SafariProvider::new()),
                Box::new(FirefoxProvider::new()),
                Box::new(WindowProvider::new()),
                Box::new(AppProvider::new()),
            ]
        }

        /// "macos.skylight", "macos.share_ui", "macos.display", "macos.session", "macos.call".
        fn signal_sources(&self) -> Vec<Box<dyn SignalSource>> {
            vec![
                Box::new(ScreenWatcher::new()),
                Box::new(ShareUiSource::new()),
                Box::new(DisplaySource::new()),
                Box::new(SessionSource::new()),
                Box::new(CallSource::new()),
            ]
        }

        fn permissions(&self) -> Box<dyn PermissionBroker> {
            Box::new(MacPermissions)
        }

        fn idle_clock(&self) -> Box<dyn Fn() -> Option<Duration> + Send + Sync> {
            Box::new(crate::idle::since_last_input)
        }

        fn running_apps(&self) -> Box<dyn Fn() -> Vec<AppRef> + Send + Sync> {
            Box::new(|| {
                apps::listed()
                    .iter()
                    .map(apps::RunningApp::app_ref)
                    .collect()
            })
        }

        fn displays(&self) -> Box<dyn Fn() -> Vec<DisplayInfo> + Send + Sync> {
            Box::new(displays)
        }
    }
}
