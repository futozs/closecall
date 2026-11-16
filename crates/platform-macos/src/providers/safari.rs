use std::collections::HashSet;
use std::sync::{Arc, mpsc};
use std::time::Instant;

use closecall_core::platform::{
    Discovered, EventSender, ExecContext, InventoryEvent, Provider, ProviderError, Reverify,
};
use closecall_core::{
    Action, BrowserFamily, BrowserTab, Capability, Handle, ProviderId, Step, StepOutcome, Target,
    TargetId, TargetKind, Tri, UrlQuality,
};

use super::{TabBatch, done, failed, hide_app, hide_instead, same_instance, skipped, unhide_app};
use crate::apps::{self, RunningApp};
use crate::observe::{self, Observer};
use crate::osa::{self, OsaError};
use crate::poll::{self, LastGood, Poller};
use crate::script::{self, SafariTab, ScriptId, ScriptWindow};
use crate::{automation, caps, keys};

pub(crate) const ID: &str = "macos.safari";

pub(crate) struct SafariProvider {
    last: Arc<LastGood<i32>>,
    poller: Option<Poller>,
    observer: Option<Observer>,
    batch: TabBatch,
}

impl SafariProvider {
    pub(crate) fn new() -> Self {
        Self {
            last: Arc::new(LastGood::new()),
            poller: None,
            observer: None,
            batch: TabBatch::default(),
        }
    }
}

/// Safari has no tab ids: the handle is the 1-based index at scan time, and actions find
/// the tab again by URL.
fn tabs(app: &RunningApp, windows: &[ScriptWindow]) -> Vec<Discovered> {
    let mut out = Vec::new();
    for window in windows {
        let window_handle = window.id.encode();
        for (i, tab) in window.tabs.iter().enumerate() {
            let index = i + 1;
            out.push(Discovered {
                key: format!("{}|{window_handle}|{index}", app.pid),
                target: poll::target(
                    ID,
                    app.app_ref(),
                    TargetKind::BrowserTab(BrowserTab {
                        family: BrowserFamily::Safari,
                        window: Handle(window_handle.clone()),
                        tab: Handle(index.to_string()),
                        title: tab.title.clone(),
                        url: tab.url.clone(),
                        url_quality: UrlQuality::Exact,
                        private: Tri::Unknown,
                        active: window.active_index == Some(index),
                        audible: Tri::Unknown,
                        pinned: Tri::Unknown,
                    }),
                ),
            });
        }
    }
    out
}

fn scan(last: &LastGood<i32>, deadline: Instant) -> Vec<Discovered> {
    let app = apps::by_bundle(caps::SAFARI).filter(|_| automation::granted(caps::SAFARI));
    let Some(app) = app else {
        last.retain(&HashSet::new());
        return Vec::new();
    };
    let result = osa::run_in_app(caps::SAFARI, script::SAFARI_LIST, deadline)
        .map(|value| tabs(&app, &script::parse_safari(&value)));
    if let Err(error) = &result {
        tracing::debug!(%error, "Safari tab scan failed");
    }
    let found = last.resolve(&app.pid, result);
    last.retain(&HashSet::from([app.pid]));
    found
}

fn location(tab: &BrowserTab) -> Option<(ScriptId, usize)> {
    Some((ScriptId::decode(&tab.window.0)?, tab.tab.0.parse().ok()?))
}

fn peer(target: &Target) -> Option<(TargetId, SafariTab<'_>)> {
    let TargetKind::BrowserTab(tab) = &target.kind else {
        return None;
    };
    Some((
        target.id,
        SafariTab {
            index: tab.tab.0.parse().ok()?,
            url: tab.url.as_deref(),
            title: Some(&tab.title),
        },
    ))
}

/// Closes the tab and the given peers of its window in one script. Returns the outcome for
/// the tab and the peers that were closed with it.
fn close_tabs(
    pid: i32,
    tab: &BrowserTab,
    peers: &[&Target],
    deadline: Instant,
) -> (StepOutcome, Vec<TargetId>) {
    if !automation::granted(caps::SAFARI) {
        return (hide_instead(pid, keys::AUTOMATION_MISSING), Vec::new());
    }
    let Some((window, index)) = location(tab) else {
        return (failed("malformed tab handle"), Vec::new());
    };
    let own = SafariTab {
        index,
        url: tab.url.as_deref(),
        title: None,
    };
    let (peer_ids, peers): (Vec<TargetId>, Vec<SafariTab<'_>>) =
        peers.iter().filter_map(|target| peer(target)).unzip();
    let tabs: Vec<SafariTab<'_>> = std::iter::once(own).chain(peers).collect();
    let script = script::safari_close_tabs(&window, &tabs);
    match osa::run_in_app(caps::SAFARI, &script, deadline) {
        Ok(value) => {
            let found = script::parse_safari_closed(&value, tabs.len()).unwrap_or_default();
            let Some((own_found, peers_found)) = found.split_first() else {
                return (skipped(keys::TARGET_GONE), Vec::new());
            };
            let closed = peer_ids
                .into_iter()
                .zip(peers_found)
                .filter_map(|(id, at)| at.map(|_| id))
                .collect();
            let outcome = if own_found.is_some() {
                done()
            } else {
                skipped(keys::TARGET_GONE)
            };
            (outcome, closed)
        }
        Err(error) if error.is_missing_object() => (skipped(keys::TARGET_GONE), Vec::new()),
        Err(OsaError::NotPermitted | OsaError::NeedsConsent) => {
            automation::forget(caps::SAFARI);
            (hide_instead(pid, keys::AUTOMATION_MISSING), Vec::new())
        }
        Err(_) => (hide_instead(pid, keys::CLOSE_FAILED), Vec::new()),
    }
}

impl Provider for SafariProvider {
    fn id(&self) -> ProviderId {
        ProviderId(ID.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        caps::safari(ID)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        let (wake, wakes) = mpsc::channel();
        apps::notify_launches(wake.clone());
        self.observer = Some(observe::spawn(ID, vec![caps::SAFARI], wake)?);
        let last = Arc::clone(&self.last);
        self.poller = Some(poll::spawn(self.id(), wakes, events, move |deadline| {
            Ok(scan(&last, deadline))
        })?);
        Ok(())
    }

    fn snapshot(&mut self, deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
        Ok(scan(&self.last, deadline))
    }

    fn reverify(&mut self, target: &Target, deadline: Instant) -> Reverify {
        let TargetKind::BrowserTab(tab) = &target.kind else {
            return Reverify::Gone;
        };
        // See the Chromium provider.
        if self.batch.was_closed_ahead(target.id) {
            return Reverify::Same;
        }
        let (Some(_), Some((window, index))) = (same_instance(target), location(tab)) else {
            return Reverify::Gone;
        };
        let script = script::safari_tab_info(&window, index, tab.url.as_deref());
        match osa::run_in_app(caps::SAFARI, &script, deadline) {
            Ok(value) => match script::parse_safari_tab_info(&value) {
                Some((index, title, url)) => {
                    let mut fresh = target.clone();
                    if let TargetKind::BrowserTab(tab) = &mut fresh.kind {
                        tab.tab = Handle(index.to_string());
                        tab.title = title;
                        tab.url = url;
                    }
                    poll::compare(target, fresh)
                }
                None => Reverify::Gone,
            },
            // See the Chromium provider: an unresponsive Safari is concealed, not ignored.
            Err(_) => Reverify::Same,
        }
    }

    fn execute(
        &mut self,
        step: &Step,
        target: &Target,
        ctx: &ExecContext,
        deadline: Instant,
    ) -> StepOutcome {
        let TargetKind::BrowserTab(tab) = &target.kind else {
            return skipped(keys::UNSUPPORTED);
        };
        self.batch.acted(target.id);
        if step.action == Action::Close && self.batch.take_closed_ahead(target.id) {
            return done();
        }
        let Some(app) = same_instance(target) else {
            return skipped(keys::NOT_RUNNING);
        };
        match step.action {
            Action::NotifyOnly => done(),
            Action::Hide => hide_app(app.pid),
            Action::Close => {
                let peers = self.batch.peers(target, &ctx.planned);
                let (outcome, closed) = close_tabs(app.pid, tab, &peers, deadline);
                self.batch.closed(closed);
                outcome
            }
            Action::Minimize | Action::Quit => skipped(keys::UNSUPPORTED),
        }
    }

    /// Safari can't tell private windows apart, so no tab is reopened: it might have been
    /// private. Safari's own History → Reopen Last Closed Tab can.
    fn restore(&mut self, target: &Target, action: Action, _deadline: Instant) -> StepOutcome {
        match action {
            Action::NotifyOnly => done(),
            Action::Hide => unhide_app(target),
            Action::Close => skipped(keys::RESTORE_PRIVATE),
            Action::Minimize | Action::Quit => skipped(keys::RESTORE_UNSUPPORTED),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tabs_are_addressed_by_window_and_index() {
        let app = RunningApp {
            pid: 7,
            bundle_id: Some(caps::SAFARI.into()),
            name: "Safari".into(),
            exe: None,
        };
        let windows =
            script::parse_safari(&json!([[3, 2, ["A", "B"], ["https://a/", "https://b/"]]]));
        let found = tabs(&app, &windows);
        assert_eq!(found[1].key, "7|i:3|2");
        let TargetKind::BrowserTab(tab) = &found[1].target.kind else {
            panic!("expected a tab");
        };
        assert!(tab.active);
        assert_eq!(tab.private, Tri::Unknown);
        assert_eq!(location(tab), Some((ScriptId::Int(3), 2)));
    }
}
