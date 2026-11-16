use std::sync::Mutex;

use closecall_core::ipc::Status;
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Wry};

use crate::native::TrayStrings;
use crate::shell::{self, View};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Sharing,
    Paused,
}

impl Mode {
    pub fn of(status: &Status) -> Self {
        if status.paused_until_ms.is_some() {
            Self::Paused
        } else if shell::is_shared(status) {
            Self::Sharing
        } else {
            Self::Idle
        }
    }

    fn icon(self) -> Image<'static> {
        match self {
            Self::Idle => tauri::include_image!("icons/tray@2x.png"),
            Self::Sharing => tauri::include_image!("icons/tray-sharing@2x.png"),
            Self::Paused => tauri::include_image!("icons/tray-paused@2x.png"),
        }
    }

    fn line(self, strings: &TrayStrings) -> &str {
        match self {
            Self::Idle => &strings.status_idle,
            Self::Sharing => &strings.status_sharing,
            Self::Paused => &strings.status_paused,
        }
    }
}

const OPEN: &str = "open";
const CLEAN_NOW: &str = "clean_now";
const PAUSE_15: &str = "pause_15";
const PAUSE_60: &str = "pause_60";
const RESUME: &str = "resume";
const QUIT: &str = "quit";

fn accelerator(hotkey: &str) -> Option<&str> {
    Some(hotkey).filter(|hotkey| !hotkey.is_empty())
}

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem<Wry>,
    open: MenuItem<Wry>,
    clean_now: MenuItem<Wry>,
    pause_15: MenuItem<Wry>,
    pause_60: MenuItem<Wry>,
    resume: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    shown: Mutex<View>,
}

impl Tray {
    pub fn build(app: &AppHandle, strings: &TrayStrings, view: &View) -> tauri::Result<Self> {
        let item = |id: &str, text: &str, enabled: bool, accelerator: Option<&str>| {
            MenuItem::with_id(app, id, text, enabled, accelerator)
        };
        let status = item("status", view.mode.line(strings), false, None)?;
        let open = item(OPEN, &strings.open, true, None)?;
        let clean_now = item(
            CLEAN_NOW,
            &strings.clean_now,
            true,
            accelerator(&view.hotkey),
        )?;
        let pause_15 = item(PAUSE_15, &strings.pause_15, true, None)?;
        let pause_60 = item(PAUSE_60, &strings.pause_60, true, None)?;
        let resume = item(RESUME, &strings.resume, view.mode == Mode::Paused, None)?;
        let quit = item(QUIT, &strings.quit, true, None)?;
        let menu = Menu::with_items(
            app,
            &[
                &status,
                &PredefinedMenuItem::separator(app)?,
                &open,
                &clean_now,
                &PredefinedMenuItem::separator(app)?,
                &pause_15,
                &pause_60,
                &resume,
                &PredefinedMenuItem::separator(app)?,
                &quit,
            ],
        )?;
        let icon = TrayIconBuilder::with_id("main")
            .icon(view.mode.icon())
            .icon_as_template(true)
            .tooltip(&app.package_info().name)
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(on_menu_event)
            .build(app)?;
        Ok(Self {
            icon,
            status,
            open,
            clean_now,
            pause_15,
            pause_60,
            resume,
            quit,
            shown: Mutex::new(view.clone()),
        })
    }

    /// Touches only what changed since the last render.
    pub fn render(&self, strings: &TrayStrings, view: &View) -> tauri::Result<()> {
        let mut shown = shell::lock(&self.shown);
        let relabel = shown.lang != view.lang;
        if relabel {
            self.open.set_text(&strings.open)?;
            self.clean_now.set_text(&strings.clean_now)?;
            self.pause_15.set_text(&strings.pause_15)?;
            self.pause_60.set_text(&strings.pause_60)?;
            self.resume.set_text(&strings.resume)?;
            self.quit.set_text(&strings.quit)?;
        }
        if relabel || shown.mode != view.mode {
            self.status.set_text(view.mode.line(strings))?;
        }
        if shown.mode != view.mode {
            self.icon
                .set_icon_with_as_template(Some(view.mode.icon()), true)?;
            self.resume.set_enabled(view.mode == Mode::Paused)?;
        }
        if shown.hotkey != view.hotkey {
            self.clean_now.set_accelerator(accelerator(&view.hotkey))?;
        }
        *shown = view.clone();
        Ok(())
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        OPEN => shell::show_window(app),
        CLEAN_NOW => shell::clean_up_now(app),
        PAUSE_15 => shell::in_background(app, |engine| engine.pause(15)),
        PAUSE_60 => shell::in_background(app, |engine| engine.pause(60)),
        RESUME => shell::in_background(app, |engine| engine.resume()),
        QUIT => app.exit(0),
        _ => {}
    }
}
