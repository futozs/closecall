use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use closecall_core::platform::{
    Discovered, EventSender, ExecContext, InventoryEvent, Provider, ProviderError, Reverify,
};
use closecall_core::{
    Action, BrowserFamily, BrowserTab, Capability, Handle, ProviderId, Step, StepOutcome, Target,
    TargetKind, Tri, UrlQuality,
};

use super::{done, hide_app, hide_instead, same_instance, skipped, unhide_app};
use crate::apps::{self, RunningApp};
use crate::firefox_session::{self, SessionTab};
use crate::poll::{self, LastGood, Poller};
use crate::{caps, keys};

pub(crate) const ID: &str = "macos.firefox";

pub(crate) struct FirefoxProvider {
    last: Arc<LastGood<i32>>,
    poller: Option<Poller>,
}

impl FirefoxProvider {
    pub(crate) fn new() -> Self {
        Self {
            last: Arc::new(LastGood::new()),
            poller: None,
        }
    }
}

fn profiles_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("Firefox/Profiles"))
}

/// A recovery file older than the running Firefox belongs to the previous session and lists
/// tabs that may no longer be open.
fn read_session(app: &RunningApp) -> Result<Vec<SessionTab>, String> {
    let file = profiles_dir()
        .and_then(|dir| firefox_session::current_recovery_file(&dir))
        .ok_or("no recovery.jsonlz4")?;
    let modified = file
        .metadata()
        .and_then(|m| m.modified())
        .map_err(|e| e.to_string())?;
    if apps::launched_at(app.pid).is_some_and(|launched| modified < launched) {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    let json = firefox_session::decode(&bytes).map_err(|e| format!("{e:?}"))?;
    firefox_session::parse(&json).map_err(|e| e.to_string())
}

fn tabs(app: &RunningApp, session: &[SessionTab]) -> Vec<Discovered> {
    let mut seen: HashMap<(usize, Option<&str>), usize> = HashMap::new();
    session
        .iter()
        .map(|tab| {
            let n = seen.entry((tab.window, tab.url.as_deref())).or_default();
            *n += 1;
            Discovered {
                key: format!(
                    "{}|{}|{}|{n}",
                    app.pid,
                    tab.window,
                    tab.url.as_deref().unwrap_or_default()
                ),
                target: poll::target(
                    ID,
                    app.app_ref(),
                    TargetKind::BrowserTab(BrowserTab {
                        family: BrowserFamily::Firefox,
                        window: Handle(tab.window.to_string()),
                        tab: Handle(format!("{}:{}", tab.window, tab.tab)),
                        title: tab.title.clone(),
                        url: tab.url.clone(),
                        url_quality: UrlQuality::Stale,
                        private: Tri::No,
                        active: tab.active,
                        audible: Tri::Unknown,
                        pinned: if tab.pinned { Tri::Yes } else { Tri::No },
                    }),
                ),
            }
        })
        .collect()
}

fn running_firefox() -> Option<RunningApp> {
    caps::FIREFOX
        .iter()
        .find_map(|bundle_id| apps::by_bundle(bundle_id))
}

fn scan(last: &LastGood<i32>) -> Vec<Discovered> {
    let Some(app) = running_firefox() else {
        last.retain(&HashSet::new());
        return Vec::new();
    };
    let result = read_session(&app).map(|session| tabs(&app, &session));
    if let Err(error) = &result {
        tracing::debug!(%error, "Firefox session read failed");
    }
    let found = last.resolve(&app.pid, result);
    last.retain(&HashSet::from([app.pid]));
    found
}

impl Provider for FirefoxProvider {
    fn id(&self) -> ProviderId {
        ProviderId(ID.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        caps::firefox(ID)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        let last = Arc::clone(&self.last);
        self.poller = Some(poll::spawn(
            self.id(),
            apps::launches(),
            events,
            move |_| Ok(scan(&last)),
        )?);
        Ok(())
    }

    fn snapshot(&mut self, _deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
        Ok(scan(&self.last))
    }

    fn reverify(&mut self, target: &Target, _deadline: Instant) -> Reverify {
        let Some(app) = same_instance(target) else {
            return Reverify::Gone;
        };
        let Ok(session) = read_session(&app) else {
            return Reverify::Same;
        };
        let key_of = |t: &Target| match &t.kind {
            TargetKind::BrowserTab(tab) => Some((tab.window.clone(), tab.url.clone())),
            _ => None,
        };
        let wanted = key_of(target);
        tabs(&app, &session)
            .into_iter()
            .find(|found| key_of(&found.target) == wanted)
            .map_or(Reverify::Gone, |found| poll::compare(target, found.target))
    }

    fn execute(
        &mut self,
        step: &Step,
        target: &Target,
        _ctx: &ExecContext,
        _deadline: Instant,
    ) -> StepOutcome {
        let Some(app) = same_instance(target) else {
            return skipped(keys::NOT_RUNNING);
        };
        match step.action {
            Action::NotifyOnly => done(),
            Action::Hide => hide_app(app.pid),
            // Closing one Firefox tab would mean selecting it and sending ⌘W in front of the
            // audience; hiding the app is the honest fallback.
            Action::Close => hide_instead(app.pid, keys::CLOSE_UNSUPPORTED),
            Action::Minimize | Action::Quit => skipped(keys::UNSUPPORTED),
        }
    }

    fn restore(&mut self, target: &Target, action: Action, _deadline: Instant) -> StepOutcome {
        match action {
            Action::NotifyOnly => done(),
            Action::Hide => unhide_app(target),
            Action::Close | Action::Minimize | Action::Quit => skipped(keys::RESTORE_UNSUPPORTED),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_urls_in_a_window_get_distinct_keys() {
        let app = RunningApp {
            pid: 9,
            bundle_id: Some("org.mozilla.firefox".into()),
            name: "Firefox".into(),
            exe: None,
        };
        let tab = |t, url: &str| SessionTab {
            window: 0,
            tab: t,
            title: "T".into(),
            url: Some(url.into()),
            active: false,
            pinned: t == 0,
        };
        let found = tabs(
            &app,
            &[
                tab(0, "https://a/"),
                tab(1, "https://a/"),
                tab(2, "https://b/"),
            ],
        );
        let keys: Vec<&str> = found.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(
            keys,
            ["9|0|https://a/|1", "9|0|https://a/|2", "9|0|https://b/|1"]
        );
        let TargetKind::BrowserTab(first) = &found[0].target.kind else {
            panic!("expected a tab");
        };
        assert_eq!(first.url_quality, UrlQuality::Stale);
        assert_eq!(first.pinned, Tri::Yes);
    }
}
