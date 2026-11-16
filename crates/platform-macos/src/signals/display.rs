//! Projectors, HDMI and AirPlay mirroring: display reconfiguration plus `CGDisplayIsInMirrorSet`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use closecall_core::Confidence;
use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayChangeSummaryFlags, CGDisplayIsInMirrorSet,
    CGDisplayRegisterReconfigurationCallback, CGDisplayRemoveReconfigurationCallback, CGError,
    CGGetOnlineDisplayList,
};

use super::{Shared, send, signal};
use crate::keys;

pub(crate) const ID: &str = "macos.display";
const MAX_DISPLAYS: u32 = 32;

fn any_mirrored() -> bool {
    let mut displays = [0 as CGDirectDisplayID; MAX_DISPLAYS as usize];
    let mut count = 0u32;
    // SAFETY: the buffer holds MAX_DISPLAYS entries and `count` is a valid out-pointer.
    let error = unsafe { CGGetOnlineDisplayList(MAX_DISPLAYS, displays.as_mut_ptr(), &mut count) };
    error == CGError::Success
        && displays
            .iter()
            .take(count as usize)
            .any(|&display| CGDisplayIsInMirrorSet(display))
}

struct State {
    signals: Shared,
    mirrored: AtomicBool,
}

impl State {
    fn update(&self) {
        let now = any_mirrored();
        if self.mirrored.swap(now, Ordering::Relaxed) != now {
            let kind = if now {
                SignalKind::DisplayMirrored
            } else {
                SignalKind::DisplayUnmirrored
            };
            send(&self.signals, signal(kind, Confidence::High, ID));
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
            mirrored: AtomicBool::new(false),
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
