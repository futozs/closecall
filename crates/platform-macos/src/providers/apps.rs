use std::time::Instant;

use closecall_core::platform::{
    Discovered, EventSender, ExecContext, InventoryEvent, Provider, ProviderError, Reverify,
};
use closecall_core::{
    Action, Capability, ProcessInfo, ProviderId, Step, StepOutcome, Target, TargetKind,
    UnsavedPolicy,
};
use objc2_application_services::AXError;

use super::{done, failed, hide_app, hide_instead, same_instance, skipped};
use crate::apps::{self, RunningApp};
use crate::ax::{self, Ax};
use crate::poll::{self, Poller};
use crate::{caps, doc, keys};

pub(crate) const ID: &str = "macos.apps";

pub(crate) struct AppProvider {
    poller: Option<Poller>,
}

impl AppProvider {
    pub(crate) fn new() -> Self {
        Self { poller: None }
    }
}

fn describe(app: &RunningApp) -> Discovered {
    Discovered {
        key: app.pid.to_string(),
        target: poll::target(
            ID,
            app.app_ref(),
            TargetKind::Process(ProcessInfo {
                pid: app.pid.cast_unsigned(),
                exe: app.exe.clone().unwrap_or_default(),
            }),
        ),
    }
}

fn scan() -> Vec<Discovered> {
    apps::regular()
        .iter()
        .filter(|app| !app.is_own())
        .map(describe)
        .collect()
}

/// Quitting an app with open documents brings up save prompts we can't answer safely, and
/// state restoration would reopen the documents anyway.
fn has_open_documents(app: &RunningApp) -> bool {
    let known_document_app = app
        .bundle_id
        .as_deref()
        .is_some_and(|id| doc::doc_app(id).is_some());
    if !ax::is_trusted() {
        return known_document_app;
    }
    match Ax::application(app.pid).elements("AXWindows") {
        Ok(windows) => windows.iter().any(|window| {
            window.string("AXDocument").is_some()
                || window
                    .element("AXCloseButton")
                    .and_then(|button| button.boolean("AXEdited"))
                    == Some(true)
        }),
        Err(AXError::CannotComplete) => true,
        Err(_) => known_document_app,
    }
}

fn quit(app: &RunningApp, policy: UnsavedPolicy) -> StepOutcome {
    if has_open_documents(app) {
        return match policy {
            UnsavedPolicy::SkipAndNotify => skipped(keys::UNSAVED_SKIPPED),
            _ => hide_instead(app.pid, keys::QUIT_WITH_DOCUMENTS_HIDDEN),
        };
    }
    if apps::terminate(app.pid) {
        done()
    } else {
        failed("NSRunningApplication.terminate failed")
    }
}

impl Provider for AppProvider {
    fn id(&self) -> ProviderId {
        ProviderId(ID.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        caps::apps(ID)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        self.poller = Some(poll::spawn(self.id(), apps::launches(), events, |_| {
            Ok(scan())
        })?);
        Ok(())
    }

    fn snapshot(&mut self, _deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
        Ok(scan())
    }

    fn reverify(&mut self, target: &Target, _deadline: Instant) -> Reverify {
        same_instance(target).map_or(Reverify::Gone, |app| {
            poll::compare(target, describe(&app).target)
        })
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
            Action::Quit => quit(&app, step.on_unsaved),
            Action::Close | Action::Minimize => skipped(keys::UNSUPPORTED),
        }
    }
}
