use std::sync::Arc;

use closecall_core::ipc::{RunReport, Status, events};
use closecall_engine::{EngineSink, Notice};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::shell::{self, Shell};
use crate::tray::Mode;

/// Forwards engine events to the window, the tray and the OS. Called from engine threads.
pub struct TauriSink {
    app: AppHandle,
    shell: Arc<Shell>,
}

impl TauriSink {
    pub fn new(app: AppHandle, shell: Arc<Shell>) -> Self {
        Self { app, shell }
    }
}

impl EngineSink for TauriSink {
    fn status_changed(&self, status: &Status) {
        if shell::is_shared(status) {
            shell::hide_window(&self.app);
        }
        if let Err(err) = self.app.emit(events::STATUS, status) {
            tracing::warn!("could not send the status to the window: {err}");
        }
        self.shell.set_mode(&self.app, Mode::of(status));
    }

    fn run_completed(&self, report: &RunReport) {
        if let Err(err) = self.app.emit(events::RUN, report) {
            tracing::warn!("could not send a run report to the window: {err}");
        }
    }

    fn notify(&self, notice: &Notice) {
        let Some(message) = self
            .shell
            .strings()
            .notice(notice, &self.app.package_info().name)
        else {
            return;
        };
        let shown = self
            .app
            .notification()
            .builder()
            .title(message.title)
            .body(message.body)
            .show();
        if let Err(err) = shown {
            tracing::warn!("could not show a notification: {err}");
        }
    }
}

/// For `closecall diag`, which has no window or tray.
pub struct SilentSink;

impl EngineSink for SilentSink {
    fn status_changed(&self, _: &Status) {}
    fn run_completed(&self, _: &RunReport) {}
    fn notify(&self, _: &Notice) {}
}
