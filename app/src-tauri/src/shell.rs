use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use closecall_core::config::Language;
use closecall_core::ipc::Status;
use closecall_engine::Engine;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::native::{Lang, Native, Strings};
use crate::tray::{Mode, Tray};

pub const MAIN_WINDOW: &str = "main";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub lang: Lang,
    pub mode: Mode,
    pub hotkey: String,
}

/// The native side of the UI: tray and notification strings. The engine sink calls in from
/// engine threads, so nothing here may wait on the main thread.
pub struct Shell {
    native: Native,
    view: Mutex<View>,
    tray: OnceLock<Tray>,
}

impl Shell {
    pub fn new(native: Native) -> Self {
        Self {
            native,
            view: Mutex::new(View {
                lang: Lang::resolve(Language::System),
                mode: Mode::Idle,
                hotkey: String::new(),
            }),
            tray: OnceLock::new(),
        }
    }

    pub fn strings(&self) -> &Strings {
        self.native.get(lock(&self.view).lang)
    }

    pub fn view(&self) -> View {
        lock(&self.view).clone()
    }

    pub fn set_mode(self: &Arc<Self>, app: &AppHandle, mode: Mode) {
        self.update(app, |view| view.mode = mode);
    }

    pub fn set_config(self: &Arc<Self>, app: &AppHandle, language: Language, hotkey: &str) {
        self.update(app, |view| {
            view.lang = Lang::resolve(language);
            hotkey.clone_into(&mut view.hotkey);
        });
    }

    fn update(self: &Arc<Self>, app: &AppHandle, change: impl FnOnce(&mut View)) {
        {
            let mut view = lock(&self.view);
            let before = view.clone();
            change(&mut view);
            if *view == before {
                return;
            }
        }
        if self.tray.get().is_none() {
            return;
        }
        // The render reads the view again when it runs, so out-of-order dispatches still
        // end on the newest state.
        let shell = Arc::clone(self);
        if let Err(err) = app.run_on_main_thread(move || shell.render()) {
            tracing::warn!("could not schedule a tray update: {err}");
        }
    }

    /// Catches up with any change made while the tray was being built.
    pub fn attach_tray(&self, tray: Tray) {
        if self.tray.set(tray).is_err() {
            tracing::warn!("tray attached twice; keeping the first");
        }
        self.render();
    }

    fn render(&self) {
        let Some(tray) = self.tray.get() else {
            return;
        };
        let view = self.view();
        if let Err(err) = tray.render(&self.native.get(view.lang).tray, &view) {
            tracing::warn!("could not update the tray: {err}");
        }
    }
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn is_shared(status: &Status) -> bool {
    status.share.active || status.share.mirrored_display
}

pub fn show_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let shown = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    if let Err(err) = shown {
        tracing::warn!("could not show the settings window: {err}");
    }
}

pub fn hide_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW)
        && let Err(err) = window.hide()
    {
        tracing::warn!("could not hide the settings window: {err}");
    }
}

/// Runs an engine call on the blocking pool; its effects reach the UI through the sink.
pub fn in_background<T>(app: &AppHandle, call: impl FnOnce(&Engine) -> T + Send + 'static) {
    let Some(engine) = app.try_state::<Engine>() else {
        return;
    };
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        call(&engine);
    });
}

/// The panic path (hotkey, tray, `closecall panic`). Our own window goes first: it lists
/// what was closed.
pub fn clean_up_now(app: &AppHandle) {
    hide_window(app);
    in_background(app, Engine::panic);
}

pub fn apply_config(app: &AppHandle, language: Language, hotkey: &str) {
    if let Some(shell) = app.try_state::<Arc<Shell>>() {
        shell.set_config(app, language, hotkey);
    }
    apply_hotkey(app, hotkey);
}

fn apply_hotkey(app: &AppHandle, hotkey: &str) {
    let shortcuts = app.global_shortcut();
    if let Err(err) = shortcuts.unregister_all() {
        tracing::warn!("could not release the panic hotkey: {err}");
    }
    if hotkey.is_empty() {
        return;
    }
    let registered = shortcuts.on_shortcut(hotkey, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            clean_up_now(app);
        }
    });
    if let Err(err) = registered {
        tracing::warn!(hotkey, "could not register the panic hotkey: {err}");
    }
}

pub fn second_instance(app: &AppHandle, argv: &[String]) {
    if argv.iter().skip(1).any(|arg| arg == "panic") {
        clean_up_now(app);
    } else {
        show_window(app);
    }
}
