//! Screen capture detection through the private `SLSIsScreenWatcherPresent`. There is no
//! public API; confirmed so far for Zoom and `screencapture`, everything else is on the
//! manual test list.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use closecall_core::Confidence;
use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};

use super::signal;
use crate::{apps, keys};

pub(crate) const ID: &str = "macos.skylight";

const MEETING_APPS: &[&str] = &[
    "us.zoom.xos",
    "com.microsoft.teams2",
    "com.microsoft.teams",
    "com.hnc.Discord",
    "com.tinyspeck.slackmacgap",
    "com.cisco.webexmeetingsapp",
    "Cisco-Systems.Spark",
    "com.apple.FaceTime",
    "com.skype.skype",
    "com.loom.desktop",
    "com.obsproject.obs-studio",
];
const IDLE_INTERVAL: Duration = Duration::from_secs(1);
const MEETING_INTERVAL: Duration = Duration::from_millis(250);
const MEETING_RECHECK: Duration = Duration::from_secs(2);

type WatcherFn = unsafe extern "C-unwind" fn() -> bool;

fn watcher_fn() -> Option<WatcherFn> {
    // SAFETY: dlopen/dlsym only load and look up; the handle stays open for the process.
    unsafe {
        let handle = libc::dlopen(
            c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight".as_ptr(),
            libc::RTLD_LAZY,
        );
        if handle.is_null() {
            return None;
        }
        let symbol = libc::dlsym(handle, c"SLSIsScreenWatcherPresent".as_ptr());
        // SAFETY: declared `bool SLSIsScreenWatcherPresent(void)` in every known header.
        (!symbol.is_null()).then(|| std::mem::transmute::<*mut c_void, WatcherFn>(symbol))
    }
}

fn meeting_app_running() -> bool {
    apps::regular().iter().any(|app| {
        app.bundle_id
            .as_deref()
            .is_some_and(|id| MEETING_APPS.contains(&id))
    })
}

pub(crate) struct ScreenWatcher {
    health: Arc<Mutex<SourceHealth>>,
    stop: Arc<AtomicBool>,
}

impl ScreenWatcher {
    pub(crate) fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(SourceHealth::Ok)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for ScreenWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn set(health: &Mutex<SourceHealth>, value: SourceHealth) {
    *health.lock().unwrap_or_else(PoisonError::into_inner) = value;
}

fn noisy() -> SourceHealth {
    SourceHealth::Noisy {
        reason_key: keys::HEALTH_WATCHER_NOISY.to_owned(),
    }
}

/// Noise calibration: a watcher that is already on at launch with no meeting app running
/// is a DisplayLink dock, an always-on recorder or similar. Its signals drop to Low until
/// it reads false once.
fn watch(
    present: WatcherFn,
    signals: Box<dyn EventSender<Signal>>,
    health: Arc<Mutex<SourceHealth>>,
    stop: Arc<AtomicBool>,
) {
    // SAFETY: no arguments, no preconditions.
    let read = || unsafe { present() };
    let mut capturing = read();
    let mut meeting = meeting_app_running();
    let mut noise = capturing && !meeting;
    if noise {
        set(&health, noisy());
    }
    let confidence = |noise: bool| {
        if noise {
            Confidence::Low
        } else {
            Confidence::High
        }
    };
    if capturing && !signals.send(signal(SignalKind::CaptureStarted, confidence(noise), ID)) {
        return;
    }
    let mut checked = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(if meeting {
            MEETING_INTERVAL
        } else {
            IDLE_INTERVAL
        });
        if checked.elapsed() >= MEETING_RECHECK {
            meeting = meeting_app_running();
            checked = Instant::now();
        }
        let now = read();
        if now == capturing {
            continue;
        }
        capturing = now;
        let kind = if now {
            SignalKind::CaptureStarted
        } else {
            SignalKind::CaptureStopped
        };
        if !signals.send(signal(kind, confidence(noise), ID)) {
            return;
        }
        if !now && noise {
            noise = false;
            set(&health, SourceHealth::Ok);
        }
    }
}

impl SignalSource for ScreenWatcher {
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
        let Some(present) = watcher_fn() else {
            set(
                &self.health,
                SourceHealth::Unavailable {
                    reason_key: keys::HEALTH_WATCHER_MISSING.to_owned(),
                },
            );
            return Err(SignalError::Unavailable(
                "SLSIsScreenWatcherPresent not found".to_owned(),
            ));
        };
        let (health, stop) = (Arc::clone(&self.health), Arc::clone(&self.stop));
        thread::Builder::new()
            .name(ID.to_owned())
            .spawn(move || watch(present, signals, health, stop))
            .map_err(|e| SignalError::Other(e.to_string()))?;
        Ok(())
    }
}
