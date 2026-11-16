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
use serde_json::Value;

use super::{TabBatch, done, failed, hide_app, hide_instead, same_instance, skipped, unhide_app};
use crate::apps::{self, RunningApp};
use crate::observe::{self, Observer};
use crate::osa::{self, OsaError};
use crate::poll::{self, LastGood, Poller};
use crate::script::{self, ChromiumPeer, ScriptId, ScriptWindow};
use crate::{automation, caps, keys};

pub(crate) const ID: &str = "macos.chromium";

pub(crate) struct ChromiumProvider {
    last: Arc<LastGood<String>>,
    poller: Option<Poller>,
    observer: Option<Observer>,
    batch: TabBatch,
}

impl ChromiumProvider {
    pub(crate) fn new() -> Self {
        Self {
            last: Arc::new(LastGood::new()),
            poller: None,
            observer: None,
            batch: TabBatch::default(),
        }
    }
}

fn tabs(app: &RunningApp, windows: &[ScriptWindow]) -> Vec<Discovered> {
    let mut out = Vec::new();
    for window in windows {
        for (i, tab) in window.tabs.iter().enumerate() {
            let Some(tab_id) = &tab.id else { continue };
            let (window_handle, tab_handle) = (window.id.encode(), tab_id.encode());
            out.push(Discovered {
                key: format!("{}|{window_handle}|{tab_handle}", app.pid),
                target: poll::target(
                    ID,
                    app.app_ref(),
                    TargetKind::BrowserTab(BrowserTab {
                        family: BrowserFamily::Chromium,
                        window: Handle(window_handle),
                        tab: Handle(tab_handle),
                        title: tab.title.clone(),
                        url: tab.url.clone(),
                        url_quality: UrlQuality::Exact,
                        private: window.private,
                        active: window.active_index == Some(i + 1),
                        audible: Tri::Unknown,
                        pinned: Tri::Unknown,
                    }),
                ),
            });
        }
    }
    out
}

fn scan(last: &LastGood<String>, deadline: Instant) -> Vec<Discovered> {
    let mut found = Vec::new();
    let mut alive = HashSet::new();
    for &(bundle_id, _) in caps::CHROMIUM {
        let Some(app) = apps::by_bundle(bundle_id) else {
            continue;
        };
        if !automation::granted(bundle_id) {
            continue;
        }
        let key = format!("{bundle_id}|{}", app.pid);
        let result = osa::run_in_app(bundle_id, script::CHROMIUM_LIST, deadline)
            .map(|value| tabs(&app, &script::parse_chromium(&value)));
        if let Err(error) = &result {
            tracing::debug!(bundle_id, %error, "tab scan failed");
        }
        found.extend(last.resolve(&key, result));
        alive.insert(key);
    }
    last.retain(&alive);
    found
}

fn ids(tab: &BrowserTab) -> Option<(ScriptId, ScriptId)> {
    Some((
        ScriptId::decode(&tab.window.0)?,
        ScriptId::decode(&tab.tab.0)?,
    ))
}

fn peer(target: &Target) -> Option<(TargetId, ChromiumPeer<'_>)> {
    let TargetKind::BrowserTab(tab) = &target.kind else {
        return None;
    };
    let id = ScriptId::decode(&tab.tab.0)?;
    Some((
        target.id,
        ChromiumPeer {
            id,
            title: &tab.title,
            url: tab.url.as_deref(),
        },
    ))
}

/// Closes the tab and the given peers of its window in one script. Returns the outcome for
/// the tab and the peers that were closed with it.
fn close_tabs(
    bundle_id: &str,
    pid: i32,
    tab: &BrowserTab,
    peers: &[&Target],
    deadline: Instant,
) -> (StepOutcome, Vec<TargetId>) {
    if !automation::granted(bundle_id) {
        return (hide_instead(pid, keys::AUTOMATION_MISSING), Vec::new());
    }
    let Some((window, tab)) = ids(tab) else {
        return (failed("malformed tab handle"), Vec::new());
    };
    let (peer_ids, peers): (Vec<TargetId>, Vec<ChromiumPeer<'_>>) =
        peers.iter().filter_map(|target| peer(target)).unzip();
    let script = script::chromium_close_tabs(&window, &tab, &peers);
    match osa::run_in_app(bundle_id, &script, deadline) {
        Ok(Value::Null) => (skipped(keys::NOT_RUNNING), Vec::new()),
        Ok(value) => {
            let closed = script::parse_chromium_closed(&value, peer_ids.len())
                .map(|closed| {
                    peer_ids
                        .into_iter()
                        .zip(closed)
                        .filter_map(|(id, closed)| closed.then_some(id))
                        .collect()
                })
                .unwrap_or_default();
            (done(), closed)
        }
        Err(error) if error.is_missing_object() => (skipped(keys::TARGET_GONE), Vec::new()),
        Err(OsaError::NotPermitted | OsaError::NeedsConsent) => {
            automation::forget(bundle_id);
            (hide_instead(pid, keys::AUTOMATION_MISSING), Vec::new())
        }
        Err(_) => (hide_instead(pid, keys::CLOSE_FAILED), Vec::new()),
    }
}

/// Only tabs we know were not private come back, and only into a running browser: the
/// script checks again that it runs, so this never launches it.
fn reopen_tab(target: &Target, deadline: Instant) -> StepOutcome {
    let TargetKind::BrowserTab(tab) = &target.kind else {
        return skipped(keys::RESTORE_UNSUPPORTED);
    };
    if tab.private != Tri::No {
        return skipped(keys::RESTORE_PRIVATE);
    }
    let Some(url) = tab.url.as_deref().filter(|url| !url.is_empty()) else {
        return skipped(keys::RESTORE_UNSUPPORTED);
    };
    let bundle_id = target.app.id.as_str();
    if apps::by_bundle(bundle_id).is_none() {
        return skipped(keys::NOT_RUNNING);
    }
    if !automation::granted(bundle_id) {
        return skipped(keys::AUTOMATION_MISSING);
    }
    let window = ScriptId::decode(&tab.window.0);
    match osa::run_in_app(
        bundle_id,
        &script::chromium_reopen(window.as_ref(), url),
        deadline,
    ) {
        Ok(Value::String(result)) if result == "opened" => done(),
        Ok(_) => skipped(keys::NOT_RUNNING),
        Err(OsaError::NotPermitted | OsaError::NeedsConsent) => {
            automation::forget(bundle_id);
            skipped(keys::AUTOMATION_MISSING)
        }
        // The message can quote the URL; the number is enough to debug.
        Err(OsaError::Script { number, .. }) => failed(format!("AppleScript error {number}")),
        Err(error) => failed(error.to_string()),
    }
}

impl Provider for ChromiumProvider {
    fn id(&self) -> ProviderId {
        ProviderId(ID.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        caps::chromium(ID)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        let (wake, wakes) = mpsc::channel();
        apps::notify_launches(wake.clone());
        let bundle_ids = caps::CHROMIUM.iter().map(|&(id, _)| id).collect();
        self.observer = Some(observe::spawn(ID, bundle_ids, wake)?);
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
        // Closed in the batch of an earlier step, unchanged at that moment; `execute`
        // reports it done.
        if self.batch.was_closed_ahead(target.id) {
            return Reverify::Same;
        }
        let (Some(_), Some((window, tab_id))) = (same_instance(target), ids(tab)) else {
            return Reverify::Gone;
        };
        let script = script::chromium_tab_info(&window, &tab_id);
        match osa::run_in_app(&target.app.id, &script, deadline) {
            Ok(value) => match script::parse_tab_info(&value) {
                Some((title, url)) => {
                    let mut fresh = target.clone();
                    if let TargetKind::BrowserTab(tab) = &mut fresh.kind {
                        tab.title = title;
                        tab.url = url;
                    }
                    poll::compare(target, fresh)
                }
                None => Reverify::Gone,
            },
            // Unverifiable is not gone: a hung browser must still be concealed, and `execute`
            // falls back to hiding the app when the tab can't be addressed.
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
                let (outcome, closed) = close_tabs(&target.app.id, app.pid, tab, &peers, deadline);
                self.batch.closed(closed);
                outcome
            }
            Action::Minimize | Action::Quit => skipped(keys::UNSUPPORTED),
        }
    }

    fn restore(&mut self, target: &Target, action: Action, deadline: Instant) -> StepOutcome {
        match action {
            Action::NotifyOnly => done(),
            Action::Hide => unhide_app(target),
            Action::Close => reopen_tab(target, deadline),
            Action::Minimize | Action::Quit => skipped(keys::RESTORE_UNSUPPORTED),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keys_are_per_process_and_handles_round_trip() {
        let app = RunningApp {
            pid: 42,
            bundle_id: Some("com.brave.Browser".into()),
            name: "Brave Browser".into(),
            exe: None,
        };
        let windows = script::parse_chromium(&json!([[
            "w1",
            "incognito",
            2,
            ["t1", "t2"],
            ["A", "B"],
            ["https://a/", "https://b/"]
        ]]));
        let found = tabs(&app, &windows);
        assert_eq!(found[1].key, "42|s:w1|s:t2");
        let TargetKind::BrowserTab(tab) = &found[1].target.kind else {
            panic!("expected a tab");
        };
        assert!(tab.active);
        assert_eq!(tab.private, Tri::Yes);
        assert_eq!(
            ids(tab),
            Some((ScriptId::Text("w1".into()), ScriptId::Text("t2".into())))
        );
        assert_eq!(found[1].target.app.id, "com.brave.Browser");
    }
}
