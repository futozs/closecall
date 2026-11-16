//! Projectors, monitors, HDMI and AirPlay: display reconfiguration, then a diff of the external
//! displays and their mirroring. Each signal names its display, so the engine can ignore the
//! user's own monitors.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, PoisonError};

use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};
use closecall_core::{Confidence, DisplayInfo};
use objc2::MainThreadMarker;
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSScreen;
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayChangeSummaryFlags, CGDisplayIsBuiltin, CGDisplayIsInMirrorSet,
    CGDisplayModelNumber, CGDisplayRegisterReconfigurationCallback,
    CGDisplayRemoveReconfigurationCallback, CGDisplaySerialNumber, CGDisplayVendorNumber, CGError,
    CGGetOnlineDisplayList,
};
use objc2_foundation::{NSNumber, NSString};

use super::{Shared, send, signal};
use crate::keys;

pub(crate) const ID: &str = "macos.display";
const MAX_DISPLAYS: u32 = 32;

/// `NSScreen` is main-thread only, so names are read there (at start and on every
/// reconfiguration) and kept for other threads. A display that was mirrored from the moment
/// it connected never had an `NSScreen`, so it has no name.
static NAMES: Mutex<BTreeMap<CGDirectDisplayID, String>> = Mutex::new(BTreeMap::new());

fn refresh_names() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let key = NSString::from_str("NSScreenNumber");
    let found: Vec<(CGDirectDisplayID, String)> = autoreleasepool(|_| {
        NSScreen::screens(mtm)
            .iter()
            .filter_map(|screen| {
                let number = screen.deviceDescription().objectForKey(&key)?;
                let number = number.downcast::<NSNumber>().ok()?;
                Some((
                    number.unsignedIntValue(),
                    screen.localizedName().to_string(),
                ))
            })
            .collect()
    });
    NAMES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(found);
}

/// Every online display, the built-in one included. Safe to call from any thread; names are
/// fresh only on the main thread, cached elsewhere.
pub(crate) fn displays() -> Vec<DisplayInfo> {
    refresh_names();
    let mut ids = [0 as CGDirectDisplayID; MAX_DISPLAYS as usize];
    let mut count = 0u32;
    // SAFETY: the buffer holds MAX_DISPLAYS entries and `count` is a valid out-pointer.
    let error = unsafe { CGGetOnlineDisplayList(MAX_DISPLAYS, ids.as_mut_ptr(), &mut count) };
    if error != CGError::Success {
        return Vec::new();
    }
    let names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    ids.iter()
        .take(count as usize)
        .map(|&id| {
            let serial = CGDisplaySerialNumber(id);
            DisplayInfo {
                id,
                name: names.get(&id).cloned().unwrap_or_default(),
                vendor: CGDisplayVendorNumber(id),
                model: CGDisplayModelNumber(id),
                serial: (serial != 0).then_some(serial),
                builtin: CGDisplayIsBuiltin(id),
                mirrored: CGDisplayIsInMirrorSet(id),
            }
        })
        .collect()
}

fn about(kind: SignalKind, display: &DisplayInfo) -> Signal {
    Signal {
        display: Some(display.clone()),
        ..signal(kind, Confidence::High, ID)
    }
}

type Externals = BTreeMap<CGDirectDisplayID, DisplayInfo>;

/// The built-in panel sits in the mirror set too when it mirrors to a projector, but it is
/// never the projector, so only external displays are reported.
fn changes(before: &Externals, after: &Externals) -> Vec<Signal> {
    let mut signals = Vec::new();
    for (id, old) in before {
        if !after.contains_key(id) {
            if old.mirrored {
                signals.push(about(SignalKind::DisplayUnmirrored, old));
            }
            signals.push(about(SignalKind::DisplayDisconnected, old));
        }
    }
    for (id, new) in after {
        match before.get(id) {
            None => {
                signals.push(about(SignalKind::DisplayConnected, new));
                if new.mirrored {
                    signals.push(about(SignalKind::DisplayMirrored, new));
                }
            }
            Some(old) if old.mirrored != new.mirrored => {
                let kind = if new.mirrored {
                    SignalKind::DisplayMirrored
                } else {
                    SignalKind::DisplayUnmirrored
                };
                signals.push(about(kind, new));
            }
            Some(_) => {}
        }
    }
    signals
}

fn externals() -> Externals {
    displays()
        .into_iter()
        .filter(|display| !display.builtin)
        .map(|display| (display.id, display))
        .collect()
}

struct State {
    signals: Shared,
    externals: Mutex<Externals>,
}

impl State {
    fn update(&self) {
        let now = externals();
        let signals = {
            let mut last = self
                .externals
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let signals = changes(&last, &now);
            *last = now;
            signals
        };
        for signal in signals {
            send(&self.signals, signal);
        }
    }
}

unsafe extern "C-unwind" fn reconfigured(
    _display: CGDirectDisplayID,
    flags: CGDisplayChangeSummaryFlags,
    info: *mut c_void,
) {
    if flags.contains(CGDisplayChangeSummaryFlags::BeginConfigurationFlag) {
        return;
    }
    // SAFETY: `info` is the `State` registered in `start`, which is never freed.
    let state = unsafe { &*info.cast::<State>() };
    state.update();
}

struct Registration(*mut State);

// SAFETY: the pointer is only used to unregister the callback; the `State` behind it is
// itself `Sync` and is never freed.
unsafe impl Send for Registration {}

pub(crate) struct DisplaySource {
    health: SourceHealth,
    registration: Option<Registration>,
}

impl DisplaySource {
    pub(crate) fn new() -> Self {
        Self {
            health: SourceHealth::Ok,
            registration: None,
        }
    }
}

impl Drop for DisplaySource {
    fn drop(&mut self) {
        if let Some(Registration(state)) = self.registration.take() {
            // SAFETY: same callback and pointer as registered. The `State` is leaked on
            // purpose: a callback may be running on the main thread right now.
            unsafe { CGDisplayRemoveReconfigurationCallback(Some(reconfigured), state.cast()) };
        }
    }
}

impl SignalSource for DisplaySource {
    fn id(&self) -> String {
        ID.to_owned()
    }

    fn health(&self) -> SourceHealth {
        self.health.clone()
    }

    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError> {
        let state = Box::new(State {
            signals: Arc::new(Mutex::new(signals)),
            externals: Mutex::new(Externals::new()),
        });
        state.update();
        let state = Box::into_raw(state);
        // SAFETY: `reconfigured` matches the callback type and `state` outlives the registration.
        let error =
            unsafe { CGDisplayRegisterReconfigurationCallback(Some(reconfigured), state.cast()) };
        if error != CGError::Success {
            // SAFETY: registration failed, so nothing else holds the pointer.
            drop(unsafe { Box::from_raw(state) });
            self.health = SourceHealth::Unavailable {
                reason_key: keys::HEALTH_DISPLAY_CALLBACK_FAILED.to_owned(),
            };
            return Err(SignalError::Other(format!(
                "CGDisplayRegisterReconfigurationCallback failed: {error:?}"
            )));
        }
        self.registration = Some(Registration(state));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(id: u32, mirrored: bool) -> DisplayInfo {
        DisplayInfo {
            id,
            name: format!("Display {id}"),
            vendor: 7789,
            model: 30471,
            serial: None,
            builtin: false,
            mirrored,
        }
    }

    fn map(displays: &[DisplayInfo]) -> Externals {
        displays
            .iter()
            .map(|display| (display.id, display.clone()))
            .collect()
    }

    fn kinds(signals: &[Signal]) -> Vec<(SignalKind, u32)> {
        signals
            .iter()
            .map(|signal| {
                let display = signal.display.as_ref().expect("names its display");
                (signal.kind, display.id)
            })
            .collect()
    }

    #[test]
    fn connecting_mirroring_and_unplugging_are_reported_per_display() {
        let none = map(&[]);
        let monitor = map(&[display(4, false)]);
        let projector = map(&[display(4, false), display(9, true)]);
        assert_eq!(
            kinds(&changes(&none, &monitor)),
            [(SignalKind::DisplayConnected, 4)]
        );
        assert_eq!(
            kinds(&changes(&monitor, &projector)),
            [
                (SignalKind::DisplayConnected, 9),
                (SignalKind::DisplayMirrored, 9)
            ]
        );
        assert_eq!(
            kinds(&changes(
                &projector,
                &map(&[display(4, true), display(9, true)])
            )),
            [(SignalKind::DisplayMirrored, 4)]
        );
        assert_eq!(
            kinds(&changes(&projector, &monitor)),
            [
                (SignalKind::DisplayUnmirrored, 9),
                (SignalKind::DisplayDisconnected, 9)
            ]
        );
        assert!(changes(&projector, &projector).is_empty());
        assert!(
            changes(&none, &monitor)
                .iter()
                .all(|signal| signal.source == ID && signal.confidence == Confidence::High)
        );
    }
}
