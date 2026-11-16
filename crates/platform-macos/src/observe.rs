//! AX notifications from browser windows. Switching or loading a tab changes the window
//! title, and rescanning on that lets share-guard see the new tab well within a second
//! instead of at the next poll. Polling stays the fallback.
//!
//! The observers run on their own thread with its own `CFRunLoop`: the engine keeps the
//! provider thread blocked on its channel, so that thread's run loop never spins. AX
//! elements and observers are created, used and released on this thread only.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use closecall_core::platform::ProviderError;
use objc2_application_services::{AXError, AXObserver, AXUIElement};
use objc2_core_foundation::{
    CFRetained, CFRunLoop, CFRunLoopMode, CFRunLoopRunResult, CFRunLoopSource, CFString,
    kCFRunLoopDefaultMode,
};

use crate::ax::{self, Ax};
use crate::sheet::Node as _;
use crate::{apps, automation};

/// How often windows and running browsers are re-read, so new windows get observed.
const RESYNC: Duration = Duration::from_secs(1);
const APP_NOTIFICATIONS: [&str; 3] = [
    "AXFocusedWindowChanged",
    "AXMainWindowChanged",
    "AXWindowCreated",
];
const WINDOW_NOTIFICATION: &str = "AXTitleChanged";

pub(crate) struct Observer {
    stop: Arc<AtomicBool>,
}

impl Drop for Observer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Sends on `wake` whenever a window of one of `bundle_ids` changes title or focus. Does
/// nothing until Accessibility is granted, and observes only apps we may script, since
/// only those are scanned.
pub(crate) fn spawn(
    name: &str,
    bundle_ids: Vec<&'static str>,
    wake: Sender<()>,
) -> Result<Observer, ProviderError> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    thread::Builder::new()
        .name(format!("{name}-ax"))
        .spawn(move || run(&bundle_ids, wake, &stopped))
        .map_err(|e| ProviderError::Other(e.to_string()))?;
    Ok(Observer { stop })
}

fn default_mode() -> Option<&'static CFRunLoopMode> {
    // SAFETY: an immutable framework constant.
    unsafe { kCFRunLoopDefaultMode }
}

unsafe extern "C-unwind" fn on_notification(
    _observer: NonNull<AXObserver>,
    _element: NonNull<AXUIElement>,
    _notification: NonNull<CFString>,
    refcon: *mut c_void,
) {
    // SAFETY: `refcon` points at the `Sender` that `run` keeps alive until every observer's
    // source is off the run loop, and callbacks only arrive on that thread while it runs.
    let wake = unsafe { &*refcon.cast::<Sender<()>>() };
    let _ = wake.send(());
}

fn run(bundle_ids: &[&str], wake: Sender<()>, stop: &AtomicBool) {
    let Some(run_loop) = CFRunLoop::current() else {
        return;
    };
    let wake = Box::new(wake);
    let refcon = ptr::from_ref::<Sender<()>>(&wake)
        .cast_mut()
        .cast::<c_void>();
    let mut watched: HashMap<i32, Watched> = HashMap::new();
    while !stop.load(Ordering::Relaxed) {
        if ax::is_trusted() {
            sync(&mut watched, bundle_ids, &run_loop, refcon);
        } else {
            watched.clear();
        }
        // With no source in the mode the run loop returns at once instead of waiting.
        let idle = watched.is_empty()
            || CFRunLoop::run_in_mode(default_mode(), RESYNC.as_secs_f64(), false)
                == CFRunLoopRunResult::Finished;
        if idle {
            thread::sleep(RESYNC);
        }
    }
    // The observers' sources hold `refcon`, so they leave the run loop before `wake` goes.
    drop(watched);
    drop(wake);
}

fn sync(
    watched: &mut HashMap<i32, Watched>,
    bundle_ids: &[&str],
    run_loop: &CFRetained<CFRunLoop>,
    refcon: *mut c_void,
) {
    let running: Vec<i32> = bundle_ids
        .iter()
        .filter_map(|bundle_id| apps::by_bundle(bundle_id))
        .filter(|app| app.bundle_id.as_deref().is_some_and(automation::granted))
        .map(|app| app.pid)
        .collect();
    watched.retain(|pid, _| running.contains(pid));
    for pid in running {
        let entry = match watched.entry(pid) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(slot) => match Watched::new(pid, run_loop) {
                Some(new) => slot.insert(new),
                None => continue,
            },
        };
        entry.sync(refcon);
    }
}

struct Watched {
    observer: CFRetained<AXObserver>,
    source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
    app: Ax,
    app_registered: bool,
    windows: Vec<Ax>,
}

impl Watched {
    fn new(pid: i32, run_loop: &CFRetained<CFRunLoop>) -> Option<Self> {
        let mut raw: *mut AXObserver = ptr::null_mut();
        // SAFETY: `on_notification` has the `AXObserverCallback` signature and `raw` is a
        // valid out-pointer.
        let error =
            unsafe { AXObserver::create(pid, Some(on_notification), NonNull::from(&mut raw)) };
        if error != AXError::Success {
            return None;
        }
        // SAFETY: Create rule: on success `raw` holds a +1 reference, which we take over.
        let observer = unsafe { CFRetained::from_raw(NonNull::new(raw)?) };
        // SAFETY: `observer` was just created and is valid.
        let source = unsafe { observer.run_loop_source() };
        run_loop.add_source(Some(&source), default_mode());
        Some(Self {
            observer,
            source,
            run_loop: CFRetained::clone(run_loop),
            app: Ax::application(pid),
            app_registered: false,
            windows: Vec::new(),
        })
    }

    fn add(&self, element: &Ax, notification: &str, refcon: *mut c_void) -> bool {
        let name = CFString::from_str(notification);
        // SAFETY: `refcon` outlives this observer's run loop source (see `run`).
        let error = unsafe { self.observer.add_notification(element.raw(), &name, refcon) };
        matches!(
            error,
            AXError::Success | AXError::NotificationAlreadyRegistered
        )
    }

    /// A freshly launched app may not answer yet; whatever fails is retried next time.
    fn sync(&mut self, refcon: *mut c_void) {
        if !self.app_registered {
            self.app_registered = APP_NOTIFICATIONS
                .iter()
                .all(|name| self.add(&self.app, name, refcon));
        }
        let Ok(current) = self.app.elements("AXWindows") else {
            return;
        };
        self.windows
            .retain(|known| current.iter().any(|window| window.same(known)));
        for window in current {
            let known = self.windows.iter().any(|w| w.same(&window));
            if !known && self.add(&window, WINDOW_NOTIFICATION, refcon) {
                self.windows.push(window);
            }
        }
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        self.run_loop
            .remove_source(Some(&self.source), default_mode());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    // Observes this test process, which has no windows, so it touches no other app and
    // works without Accessibility.
    #[test]
    fn observers_join_and_leave_the_run_loop() {
        let run_loop = CFRunLoop::current().unwrap();
        let (wake, _wakes) = mpsc::channel();
        let wake = Box::new(wake);
        let refcon = ptr::from_ref::<Sender<()>>(&wake)
            .cast_mut()
            .cast::<c_void>();
        let mut watched = Watched::new(apps::own_pid(), &run_loop).unwrap();
        let source = CFRetained::clone(&watched.source);
        assert!(run_loop.contains_source(Some(&source), default_mode()));
        watched.sync(refcon);
        assert!(watched.windows.is_empty());
        drop(watched);
        assert!(!run_loop.contains_source(Some(&source), default_mode()));
    }
}
