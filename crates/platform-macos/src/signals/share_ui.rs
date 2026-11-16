//! Screen sharing seen from the controls that meeting apps and browsers put on screen while
//! they share, without Screen Recording: `CGWindowList` owners, layers and bounds (no titles),
//! Zoom's capture host process, and, only for a bar out of its usual place, its AX title.
//!
//! This catches shares the SkyLight watcher cannot tell apart while an ignored app (a
//! DisplayLink dock) keeps it on. Every cue is structural first; a title is matched only on a
//! window that already looks like a sharing bar.

use std::collections::HashMap;
use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use closecall_core::Confidence;
use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};
use closecall_core::text::fold;
use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CFType, CGRect};
use objc2_core_graphics::{
    CGDisplayBounds, CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo,
    CGWindowListOption, kCGWindowAlpha, kCGWindowBounds, kCGWindowLayer, kCGWindowNumber,
    kCGWindowOwnerPID,
};

use super::signal;
use crate::ax::{self, Ax};
use crate::{apps, keys};

pub(crate) const ID: &str = "macos.share_ui";
const INTERVAL: Duration = Duration::from_secs(1);
/// Polls in a row a cue has to hold (or be gone) before it counts, so a tooltip-sized flicker
/// is not a share.
const STEADY: u8 = 2;

const ZOOM: &str = "us.zoom.xos";
const ZOOM_CAPTURE_HOST: &str = "CptHost";

const BROWSERS: &[&str] = &[
    "com.google.Chrome",
    "com.google.Chrome.beta",
    "com.google.Chrome.dev",
    "com.google.Chrome.canary",
    "com.microsoft.edgemac",
    "com.microsoft.edgemac.Beta",
    "com.microsoft.edgemac.Dev",
    "com.microsoft.edgemac.Canary",
    "com.brave.Browser",
    "com.brave.Browser.beta",
    "com.brave.Browser.nightly",
    "company.thebrowser.Browser",
    "net.imput.helium",
    "com.vivaldi.Vivaldi",
    "org.chromium.Chromium",
];

const MEETING_APPS: &[&str] = &[
    ZOOM,
    "com.microsoft.teams2",
    "com.microsoft.teams",
    "com.tinyspeck.slackmacgap",
    "com.hnc.Discord",
    "com.cisco.webexmeetingsapp",
    "Cisco-Systems.Spark",
];

/// `NSMainMenuWindowLevel`: the menu bar, and above it status items and menus, all of which
/// apps own too.
const MAIN_MENU_LEVEL: i32 = 24;
/// `NSScreenSaverWindowLevel`, where Electron apps put always-on-top toolbars that must stay
/// above full-screen windows.
const SCREEN_SAVER_LEVEL: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    fn contains(&self, (x, y): (f64, f64)) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Global coordinates with the origin at the top left of the main display, like `CGWindowList`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Window {
    pub number: u32,
    pub pid: i32,
    /// The owner's bundle id, when it is an app.
    pub app: Option<String>,
    pub layer: i32,
    pub bounds: Rect,
    pub alpha: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cue {
    ZoomCaptureHost,
    /// A wide, short, floating window where a sharing bar sits.
    Bar,
    /// A bar somewhere else (they can be dragged) whose title talks about sharing.
    TitledBar,
    /// A floating frame around a whole display, drawn around the screen being shared.
    Border,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Detection {
    pub app: String,
    pub cue: Cue,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Browser,
    Meeting,
}

fn owner(window: &Window) -> Option<(&str, Owner)> {
    let app = window.app.as_deref()?;
    if BROWSERS.contains(&app) {
        Some((app, Owner::Browser))
    } else if MEETING_APPS.contains(&app) {
        Some((app, Owner::Meeting))
    } else {
        None
    }
}

fn floating(layer: i32) -> bool {
    (1..MAIN_MENU_LEVEL).contains(&layer) || layer == SCREEN_SAVER_LEVEL
}

fn visible(window: &Window) -> bool {
    window.alpha > 0.0 && window.bounds.width > 0.0 && window.bounds.height > 0.0
}

/// Wide and short like a toolbar. A picture-in-picture video or a mini meeting window is far
/// taller for its width.
pub(crate) fn is_bar(window: &Window) -> bool {
    let Rect { width, height, .. } = window.bounds;
    visible(window)
        && floating(window.layer)
        && (24.0..=80.0).contains(&height)
        && (200.0..=1000.0).contains(&width)
        && width >= 3.0 * height
}

/// Centered near the top or bottom edge of a display: where Chrome, Zoom and Teams put their
/// sharing bars. Browsers only use the bottom; their top-center bubbles are about full screen.
fn in_bar_spot(window: &Window, displays: &[Rect], owner: Owner) -> bool {
    let center = window.bounds.center();
    let Some(display) = displays.iter().find(|display| display.contains(center)) else {
        return false;
    };
    let margin = (display.height * 0.15).max(120.0);
    let centered = (center.0 - display.center().0).abs() <= (display.width * 0.1).max(60.0);
    let top_gap = window.bounds.y - display.y;
    let bottom_gap = display.y + display.height - (window.bounds.y + window.bounds.height);
    let near_top = owner == Owner::Meeting && (0.0..=margin).contains(&top_gap);
    let near_bottom = (-4.0..=margin).contains(&bottom_gap);
    centered && (near_top || near_bottom)
}

/// The top edge may sit below the menu bar.
fn frames_a_display(window: &Window, displays: &[Rect]) -> bool {
    let b = window.bounds;
    visible(window)
        && window.layer > 0
        && window.layer != MAIN_MENU_LEVEL
        && displays.iter().any(|d| {
            (b.x - d.x).abs() <= 16.0
                && (b.x + b.width - (d.x + d.width)).abs() <= 16.0
                && (b.y + b.height - (d.y + d.height)).abs() <= 16.0
                && (0.0..=48.0).contains(&(b.y - d.y))
        })
}

/// English and Hungarian, case- and accent-insensitive: Chrome's "… is sharing your screen",
/// Zoom's "You are screen sharing", „… megosztja a képernyőjét”, „Képernyőmegosztás”.
pub(crate) fn talks_about_sharing(title: &str) -> bool {
    let title = fold(title);
    ["sharing", "megoszt"]
        .iter()
        .any(|word| title.contains(word))
}

/// `title` reads a window's AX title; it is asked only about bars outside their usual spot.
pub(crate) fn detect(
    windows: &[Window],
    displays: &[Rect],
    capture_host: bool,
    title: &mut dyn FnMut(&Window) -> Option<String>,
) -> Option<Detection> {
    if capture_host {
        return Some(Detection {
            app: ZOOM.to_owned(),
            cue: Cue::ZoomCaptureHost,
        });
    }
    let found = |app: &str, cue| {
        Some(Detection {
            app: app.to_owned(),
            cue,
        })
    };
    for window in windows {
        let Some((app, kind)) = owner(window) else {
            continue;
        };
        if kind == Owner::Meeting && frames_a_display(window, displays) {
            return found(app, Cue::Border);
        }
        if !is_bar(window) {
            continue;
        }
        if in_bar_spot(window, displays, kind) {
            return found(app, Cue::Bar);
        }
        if title(window).is_some_and(|title| talks_about_sharing(&title)) {
            return found(app, Cue::TitledBar);
        }
    }
    None
}

fn number<T>(
    info: &CFDictionary<CFString, CFType>,
    key: &CFString,
    read: fn(&CFNumber) -> Option<T>,
) -> Option<T> {
    let value = info.get(key)?.downcast::<CFNumber>().ok()?;
    read(&value)
}

/// On-screen windows of every app. Titles are never read here; without Screen Recording the
/// list does not carry them anyway.
pub(crate) fn windows() -> Option<Vec<Window>> {
    let options =
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements;
    let list = CGWindowListCopyWindowInfo(options, 0)?;
    // SAFETY: the window list is an array of dictionaries.
    let list = unsafe { list.cast_unchecked::<CFDictionary>() };
    // SAFETY: framework constants.
    let (pid_key, layer_key, number_key, alpha_key, bounds_key) = unsafe {
        (
            kCGWindowOwnerPID,
            kCGWindowLayer,
            kCGWindowNumber,
            kCGWindowAlpha,
            kCGWindowBounds,
        )
    };
    let own = apps::own_pid();
    let mut bundles: HashMap<i32, Option<String>> = HashMap::new();
    let windows = list
        .iter()
        .filter_map(|info| {
            // SAFETY: window info dictionaries have string keys and CF values.
            let info = unsafe { info.cast_unchecked::<CFString, CFType>() };
            let pid = number(info, pid_key, CFNumber::as_i32)?;
            if pid == own {
                return None;
            }
            let bounds = info.get(bounds_key)?.downcast::<CFDictionary>().ok()?;
            let mut rect = CGRect::default();
            // SAFETY: `bounds` is the dictionary form of a CGRect; `rect` is a valid out-pointer.
            if !unsafe { CGRectMakeWithDictionaryRepresentation(Some(&*bounds), &mut rect) } {
                return None;
            }
            let app = bundles
                .entry(pid)
                .or_insert_with(|| apps::by_pid(pid).and_then(|app| app.bundle_id))
                .clone();
            Some(Window {
                number: number(info, number_key, CFNumber::as_i64)
                    .and_then(|n| u32::try_from(n).ok())?,
                pid,
                app,
                layer: number(info, layer_key, CFNumber::as_i32)?,
                bounds: Rect {
                    x: rect.origin.x,
                    y: rect.origin.y,
                    width: rect.size.width,
                    height: rect.size.height,
                },
                alpha: number(info, alpha_key, CFNumber::as_f64).unwrap_or(1.0),
            })
        })
        .collect();
    Some(windows)
}

pub(crate) fn display_rects() -> Vec<Rect> {
    super::display::displays()
        .iter()
        .map(|display| {
            let bounds = CGDisplayBounds(display.id);
            Rect {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.size.width,
                height: bounds.size.height,
            }
        })
        .collect()
}

/// Zoom starts `CptHost` from its own bundle for as long as it shares the screen.
pub(crate) fn zoom_capture_host() -> bool {
    if apps::by_bundle(ZOOM).is_none() {
        return false;
    }
    let mut pids = vec![0i32; 4096];
    let bytes = i32::try_from(pids.len() * size_of::<i32>()).unwrap_or(i32::MAX);
    // SAFETY: the buffer is `bytes` long; the call writes at most that many bytes.
    let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    let Ok(count) = usize::try_from(count) else {
        return false;
    };
    pids.iter().take(count).any(|&pid| {
        let mut name = [0 as c_char; 64];
        // SAFETY: `name` is 64 bytes and the call writes a NUL-terminated name into it.
        let written = unsafe { libc::proc_name(pid, name.as_mut_ptr().cast(), 64) };
        // SAFETY: on success the buffer holds a NUL-terminated string.
        written > 0
            && unsafe { CStr::from_ptr(name.as_ptr()) }.to_bytes() == ZOOM_CAPTURE_HOST.as_bytes()
    })
}

/// The AX title of `window`, matched by window number. Never logged.
pub(crate) fn ax_title(window: &Window) -> Option<String> {
    if !ax::is_trusted() {
        return None;
    }
    Ax::application(window.pid)
        .elements("AXWindows")
        .ok()?
        .into_iter()
        .find(|element| element.window_id() == Some(window.number))?
        .string("AXTitle")
}

pub(crate) fn poll() -> Option<Option<Detection>> {
    let windows = windows()?;
    Some(detect(
        &windows,
        &display_rects(),
        zoom_capture_host(),
        &mut ax_title,
    ))
}

/// Debounces raw readings into start and stop signals.
#[derive(Debug, Default)]
struct Tracker {
    sharing: Option<String>,
    candidate: Option<Option<String>>,
    streak: u8,
}

impl Tracker {
    fn observe(&mut self, app: Option<String>) -> Option<Signal> {
        if app == self.sharing {
            self.candidate = None;
            self.streak = 0;
            return None;
        }
        if self.candidate.as_ref() == Some(&app) {
            self.streak += 1;
        } else {
            self.candidate = Some(app.clone());
            self.streak = 1;
        }
        if self.streak < STEADY {
            return None;
        }
        self.candidate = None;
        self.streak = 0;
        self.sharing.clone_from(&app);
        Some(match app {
            Some(app) => Signal {
                capturing_app: Some(app),
                ..signal(SignalKind::CaptureStarted, Confidence::Medium, ID)
            },
            None => signal(SignalKind::CaptureStopped, Confidence::Medium, ID),
        })
    }
}

pub(crate) struct ShareUiSource {
    health: Arc<Mutex<SourceHealth>>,
    stop: Arc<AtomicBool>,
}

impl ShareUiSource {
    pub(crate) fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(SourceHealth::Ok)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for ShareUiSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn set(health: &Mutex<SourceHealth>, value: SourceHealth) {
    *health.lock().unwrap_or_else(PoisonError::into_inner) = value;
}

impl SignalSource for ShareUiSource {
    fn id(&self) -> String {
        ID.to_owned()
    }

    fn health(&self) -> SourceHealth {
        self.health
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError> {
        let (health, stop) = (Arc::clone(&self.health), Arc::clone(&self.stop));
        thread::Builder::new()
            .name(ID.to_owned())
            .spawn(move || {
                let mut tracker = Tracker::default();
                while !stop.load(Ordering::Relaxed) {
                    let reading = poll();
                    set(
                        &health,
                        match reading {
                            Some(_) => SourceHealth::Ok,
                            None => SourceHealth::Unavailable {
                                reason_key: keys::HEALTH_WINDOW_LIST_UNAVAILABLE.to_owned(),
                            },
                        },
                    );
                    if let Some(detection) = reading {
                        let app = detection.map(|detection| detection.app);
                        if let Some(signal) = tracker.observe(app)
                            && !signals.send(signal)
                        {
                            return;
                        }
                    }
                    thread::sleep(INTERVAL);
                }
            })
            .map_err(|e| SignalError::Other(e.to_string()))?;
        Ok(())
    }
}

/// For `examples/share_ui.rs`: what the source sees right now, with bundle ids, layers and
/// sizes but never a title.
pub(crate) fn report() -> String {
    let Some(windows) = windows() else {
        return "window list unavailable\n".to_owned();
    };
    let displays = display_rects();
    let capture_host = zoom_capture_host();
    let mut out = format!(
        "displays: {}, accessibility: {}, zoom capture host: {}\n",
        displays.len(),
        if ax::is_trusted() {
            "granted"
        } else {
            "not granted"
        },
        if capture_host {
            "running"
        } else {
            "not running"
        },
    );
    for (window, (app, kind)) in windows
        .iter()
        .filter_map(|window| owner(window).map(|found| (window, found)))
    {
        let bar = is_bar(window);
        let shape = if kind == Owner::Meeting && frames_a_display(window, &displays) {
            "display frame"
        } else if bar && in_bar_spot(window, &displays, kind) {
            "bar in a sharing-bar spot"
        } else if bar {
            "bar elsewhere"
        } else {
            "other"
        };
        let titled = bar
            && !in_bar_spot(window, &displays, kind)
            && ax_title(window).is_some_and(|title| talks_about_sharing(&title));
        out.push_str(&format!(
            "  {app}: layer {}, {:.0}x{:.0} at ({:.0}, {:.0}), alpha {:.2}: {shape}{}\n",
            window.layer,
            window.bounds.width,
            window.bounds.height,
            window.bounds.x,
            window.bounds.y,
            window.alpha,
            if titled { ", sharing title" } else { "" },
        ));
    }
    let detection = detect(&windows, &displays, capture_host, &mut ax_title);
    out.push_str(&match detection {
        Some(detection) => format!("detected: {} via {:?}\n", detection.app, detection.cue),
        None => "detected: nothing\n".to_owned(),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1512.0,
        height: 982.0,
    };
    const MONITOR: Rect = Rect {
        x: 1512.0,
        y: -200.0,
        width: 2560.0,
        height: 1440.0,
    };

    fn window(app: &str, layer: i32, (x, y, width, height): (f64, f64, f64, f64)) -> Window {
        Window {
            number: 42,
            pid: 7,
            app: Some(app.to_owned()),
            layer,
            bounds: Rect {
                x,
                y,
                width,
                height,
            },
            alpha: 1.0,
        }
    }

    fn found(windows: &[Window]) -> Option<(String, Cue)> {
        detect(windows, &[LAPTOP, MONITOR], false, &mut |_| None)
            .map(|detection| (detection.app, detection.cue))
    }

    fn found_with_title(windows: &[Window], title: &str) -> Option<(String, Cue)> {
        detect(windows, &[LAPTOP, MONITOR], false, &mut |_| {
            Some(title.to_owned())
        })
        .map(|detection| (detection.app, detection.cue))
    }

    fn chrome_main() -> Window {
        window("com.google.Chrome", 0, (0.0, 25.0, 1512.0, 957.0))
    }

    #[test]
    fn chrome_share_bar_at_the_bottom_center_of_a_display() {
        let bar = window(
            "com.google.Chrome",
            3,
            (1512.0 + 1000.0, 1180.0, 560.0, 44.0),
        );
        assert_eq!(
            found(&[chrome_main(), bar]),
            Some(("com.google.Chrome".to_owned(), Cue::Bar))
        );
        let helium = window("net.imput.helium", 3, (476.0, 920.0, 560.0, 44.0));
        assert_eq!(
            found(&[helium]),
            Some(("net.imput.helium".to_owned(), Cue::Bar))
        );
    }

    #[test]
    fn browser_bubbles_at_the_top_and_normal_windows_are_not_shares() {
        let fullscreen_hint = window("com.google.Chrome", 3, (556.0, 40.0, 400.0, 44.0));
        let pip = window("com.google.Chrome", 3, (1100.0, 700.0, 400.0, 225.0));
        let status_item = window("com.google.Chrome", 25, (656.0, 0.0, 200.0, 24.0));
        let docked = window("com.google.Chrome", 0, (476.0, 920.0, 560.0, 44.0));
        assert_eq!(
            found(&[chrome_main(), fullscreen_hint, pip, status_item, docked]),
            None
        );
    }

    #[test]
    fn a_dragged_bar_counts_only_with_a_sharing_title() {
        let dragged = window("com.google.Chrome", 3, (100.0, 300.0, 560.0, 44.0));
        assert_eq!(found(std::slice::from_ref(&dragged)), None);
        for title in [
            "meet.google.com is sharing your screen.",
            "A(z) meet.google.com megosztja a képernyőjét.",
            "KÉPERNYŐMEGOSZTÁS",
        ] {
            assert_eq!(
                found_with_title(std::slice::from_ref(&dragged), title),
                Some(("com.google.Chrome".to_owned(), Cue::TitledBar)),
                "{title}"
            );
        }
        assert_eq!(found_with_title(&[dragged], "Downloads"), None);
    }

    #[test]
    fn titles_are_never_asked_for_windows_that_are_not_bars() {
        let mut asked = 0;
        let windows = [
            chrome_main(),
            window("com.apple.Safari", 3, (100.0, 300.0, 560.0, 44.0)),
        ];
        let detection = detect(&windows, &[LAPTOP], false, &mut |_| {
            asked += 1;
            Some("is sharing your screen".to_owned())
        });
        assert_eq!(detection, None);
        assert_eq!(asked, 0);
    }

    #[test]
    fn meeting_toolbars_at_the_top_and_frames_around_a_display() {
        let teams_bar = window("com.microsoft.teams2", 3, (456.0, 30.0, 600.0, 48.0));
        assert_eq!(
            found(&[teams_bar]),
            Some(("com.microsoft.teams2".to_owned(), Cue::Bar))
        );
        let slack_bar = window(
            "com.tinyspeck.slackmacgap",
            SCREEN_SAVER_LEVEL,
            (2592.0, 1150.0, 400.0, 56.0),
        );
        assert_eq!(
            found(&[slack_bar]),
            Some(("com.tinyspeck.slackmacgap".to_owned(), Cue::Bar))
        );
        let border = window("us.zoom.xos", 5, (1512.0, -200.0, 2560.0, 1440.0));
        assert_eq!(
            found(&[border]),
            Some(("us.zoom.xos".to_owned(), Cue::Border))
        );
    }

    #[test]
    fn meeting_windows_that_are_not_sharing_controls_do_not_count() {
        let fullscreen_meeting = window("us.zoom.xos", 0, (0.0, 0.0, 1512.0, 982.0));
        let mini_window = window("com.microsoft.teams2", 3, (1180.0, 60.0, 320.0, 180.0));
        let notification = window("com.hnc.Discord", 3, (1150.0, 40.0, 340.0, 70.0));
        assert_eq!(
            found(&[fullscreen_meeting, mini_window, notification]),
            None
        );
    }

    #[test]
    fn other_apps_never_count_whatever_their_shape() {
        let bar = window("com.example.recorder", 3, (476.0, 920.0, 560.0, 44.0));
        let frame = window("com.example.recorder", 5, (0.0, 0.0, 1512.0, 982.0));
        let unknown = Window {
            app: None,
            ..bar.clone()
        };
        assert_eq!(found(&[bar, frame, unknown]), None);
    }

    #[test]
    fn the_zoom_capture_host_wins() {
        let detection = detect(&[], &[LAPTOP], true, &mut |_| None);
        assert_eq!(
            detection,
            Some(Detection {
                app: ZOOM.to_owned(),
                cue: Cue::ZoomCaptureHost
            })
        );
    }

    #[test]
    fn readings_must_hold_two_polls_before_they_start_or_stop_a_share() {
        let mut tracker = Tracker::default();
        let zoom = || Some(ZOOM.to_owned());
        assert_eq!(tracker.observe(None), None);
        assert_eq!(tracker.observe(zoom()), None);
        assert_eq!(tracker.observe(None), None, "a one-poll flicker");
        assert_eq!(tracker.observe(zoom()), None);
        let started = tracker.observe(zoom()).expect("steady for two polls");
        assert_eq!(started.kind, SignalKind::CaptureStarted);
        assert_eq!(started.confidence, Confidence::Medium);
        assert_eq!(started.capturing_app.as_deref(), Some(ZOOM));
        assert_eq!(started.source, ID);
        assert_eq!(tracker.observe(zoom()), None);
        assert_eq!(tracker.observe(None), None);
        let stopped = tracker.observe(None).expect("gone for two polls");
        assert_eq!(stopped.kind, SignalKind::CaptureStopped);
    }
}
