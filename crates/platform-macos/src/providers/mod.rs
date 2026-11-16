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

use closecall_core::{Action, StepOutcome, Target};

use crate::apps::{self as running, RunningApp};

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
