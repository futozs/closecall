mod apps;
mod chromium;
mod firefox;
mod safari;
mod windows;

pub(crate) use apps::AppProvider;
pub(crate) use chromium::ChromiumProvider;
pub(crate) use firefox::FirefoxProvider;
pub(crate) use safari::SafariProvider;
pub(crate) use windows::WindowProvider;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use closecall_core::{Action, StepOutcome, Target, TargetId, TargetKind};

use crate::apps::{self as running, RunningApp};
use crate::keys;

/// Longer than any run; after that a remembered tab is gone from the index anyway.
const BATCH_MEMORY: Duration = Duration::from_secs(10);

fn done() -> StepOutcome {
    StepOutcome::Done
}

fn skipped(reason_key: &str) -> StepOutcome {
    StepOutcome::Skipped {
        reason_key: reason_key.to_owned(),
    }
}

fn failed(error: impl Into<String>) -> StepOutcome {
    StepOutcome::Failed {
        error: error.into(),
    }
}

fn fell_back(action: Action, reason_key: &str) -> StepOutcome {
    StepOutcome::FellBack {
        action,
        reason_key: reason_key.to_owned(),
    }
}

fn hide_app(pid: i32) -> StepOutcome {
    if running::hide(pid) {
        done()
    } else {
        failed("NSRunningApplication.hide failed")
    }
}

/// Hiding is lossless, so it is where a failed or impossible close ends up.
fn hide_instead(pid: i32, reason_key: &str) -> StepOutcome {
    if running::hide(pid) {
        fell_back(Action::Hide, reason_key)
    } else {
        failed("could not close, and hiding the app failed too")
    }
}

/// The app instance the target was found in, if it is still running. A relaunched app has a
/// new pid, and its targets count as gone.
fn same_instance(target: &Target) -> Option<RunningApp> {
    let pid = i32::try_from(target.app.pid?).ok()?;
    running::by_pid(pid).filter(|app| app.app_ref().id == target.app.id)
}

/// Never relaunches: an app that quit since is left alone.
fn unhide_app(target: &Target) -> StepOutcome {
    match same_instance(target) {
        Some(app) if running::unhide(app.pid) => done(),
        Some(_) => failed("NSRunningApplication.unhide failed"),
        None => skipped(keys::NOT_RUNNING),
    }
}

fn tab_window(target: &Target) -> Option<&str> {
    match &target.kind {
        TargetKind::BrowserTab(tab) => Some(&tab.window.0),
        _ => None,
    }
}

/// The other planned tabs in the same window of the same browser instance.
fn window_peers<'a>(target: &Target, planned: &'a [Target]) -> Vec<&'a Target> {
    let Some(window) = tab_window(target) else {
        return Vec::new();
    };
    planned
        .iter()
        .filter(|other| {
            other.id != target.id
                && other.provider == target.provider
                && other.app == target.app
                && tab_window(other) == Some(window)
        })
        .collect()
}

/// Lets a tab provider close all planned tabs of a window in one call. The engine still
/// sends a step for each of them, and it may already have run some (a Hide in the conceal
/// phase), so this remembers both. Relies on `ExecContext.planned` holding only targets that
/// are acted on: a notify-only tab in it would be closed with its window's batch.
#[derive(Default)]
struct TabBatch {
    closed_ahead: HashMap<TargetId, Instant>,
    acted: HashMap<TargetId, Instant>,
}

impl TabBatch {
    fn prune(&mut self) {
        let now = Instant::now();
        let fresh = |_: &TargetId, at: &mut Instant| now.duration_since(*at) < BATCH_MEMORY;
        self.closed_ahead.retain(fresh);
        self.acted.retain(fresh);
    }

    fn acted(&mut self, id: TargetId) {
        self.prune();
        self.acted.insert(id, Instant::now());
    }

    fn was_closed_ahead(&mut self, id: TargetId) -> bool {
        self.prune();
        self.closed_ahead.contains_key(&id)
    }

    fn take_closed_ahead(&mut self, id: TargetId) -> bool {
        self.prune();
        self.closed_ahead.remove(&id).is_some()
    }

    fn closed(&mut self, ids: impl IntoIterator<Item = TargetId>) {
        let now = Instant::now();
        self.closed_ahead
            .extend(ids.into_iter().map(|id| (id, now)));
    }

    fn peers<'a>(&mut self, target: &Target, planned: &'a [Target]) -> Vec<&'a Target> {
        self.prune();
        window_peers(target, planned)
            .into_iter()
            .filter(|peer| {
                !self.acted.contains_key(&peer.id) && !self.closed_ahead.contains_key(&peer.id)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use closecall_core::{AppRef, BrowserFamily, BrowserTab, Handle, Tri, UrlQuality};

    use super::*;
    use crate::poll;

    fn tab(id: u64, pid: u32, window: &str) -> Target {
        let mut target = poll::target(
            "macos.chromium",
            AppRef {
                id: "com.google.Chrome".into(),
                name: "Google Chrome".into(),
                pid: Some(pid),
            },
            TargetKind::BrowserTab(BrowserTab {
                family: BrowserFamily::Chromium,
                window: Handle(window.into()),
                tab: Handle(format!("i:{id}")),
                title: String::new(),
                url: None,
                url_quality: UrlQuality::Exact,
                private: Tri::No,
                active: false,
                audible: Tri::Unknown,
                pinned: Tri::Unknown,
            }),
        );
        target.id = TargetId(id);
        target
    }

    fn ids(targets: &[&Target]) -> Vec<u64> {
        targets.iter().map(|t| t.id.0).collect()
    }

    #[test]
    fn peers_share_the_window_and_the_browser_instance() {
        let own = tab(1, 10, "i:1");
        let mut other_provider = tab(5, 10, "i:1");
        other_provider.provider.0 = "macos.safari".into();
        let planned = [
            own.clone(),
            tab(2, 10, "i:1"),
            tab(3, 10, "i:2"),
            tab(4, 11, "i:1"),
            other_provider,
            tab(6, 10, "i:1"),
        ];
        assert_eq!(ids(&window_peers(&own, &planned)), [2, 6]);
        assert!(window_peers(&own, &[]).is_empty());
    }

    #[test]
    fn batch_skips_tabs_already_acted_on_and_reports_closed_ones_once() {
        let own = tab(1, 10, "i:1");
        let planned = [own.clone(), tab(2, 10, "i:1"), tab(3, 10, "i:1")];
        let mut batch = TabBatch::default();
        batch.acted(TargetId(2));
        assert_eq!(ids(&batch.peers(&own, &planned)), [3]);
        batch.closed([TargetId(3)]);
        assert!(batch.peers(&own, &planned).is_empty());
        assert!(batch.was_closed_ahead(TargetId(3)));
        assert!(batch.take_closed_ahead(TargetId(3)));
        assert!(!batch.take_closed_ahead(TargetId(3)));
        assert!(!batch.was_closed_ahead(TargetId(2)));
    }
}
