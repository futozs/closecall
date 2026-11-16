//! Lock/unlock, sleep/wake and the lid.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use block2::RcBlock;
use closecall_core::Confidence;
use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceWillSleepNotification,
};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFString};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSOperationQueue,
    NSString,
};
use objc2_io_kit::{
    IOObjectRelease, IORegistryEntryCreateCFProperty, IOServiceMatching, io_object_t,
    kIOMainPortDefault,
};

use super::{Shared, send, signal};

pub(crate) const ID: &str = "macos.session";
const LID_INTERVAL: Duration = Duration::from_secs(1);

struct Observer {
    center: Retained<NSNotificationCenter>,
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

// SAFETY: NSNotificationCenter is thread-safe, and the token is only handed back to it.
unsafe impl Send for Observer {}

impl Drop for Observer {
    fn drop(&mut self) {
        // SAFETY: the token came from this center's `addObserverForName:…`.
        unsafe { self.center.removeObserver(self.token.as_ref()) };
    }
}

fn observe(
    center: &Retained<NSNotificationCenter>,
    name: &NSString,
    queue: Option<&NSOperationQueue>,
    signals: &Shared,
    kind: SignalKind,
) -> Observer {
    let signals = Arc::clone(signals);
    let block = RcBlock::new(move |_: NonNull<NSNotification>| {
        send(&signals, signal(kind, Confidence::High, ID));
    });
    // SAFETY: valid name, optional queue and a block that only touches `Send` data.
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, queue, &block)
    };
    Observer {
        center: center.clone(),
        token,
    }
}

unsafe extern "C-unwind" {
    /// Consumes `matching`. Not exported by objc2-io-kit 0.3.2.
    fn IOServiceGetMatchingService(
        main_port: libc::mach_port_t,
        matching: *const CFDictionary,
    ) -> io_object_t;
}

/// `None` on Macs without a lid.
fn lid_closed() -> Option<bool> {
    // SAFETY: the name is a valid C string; the returned dictionary is consumed below.
    let matching = unsafe { IOServiceMatching(c"IOPMrootDomain".as_ptr()) }?;
    // SAFETY: framework constant; IOServiceGetMatchingService takes ownership of `matching`.
    let root = unsafe {
        IOServiceGetMatchingService(
            kIOMainPortDefault,
            CFRetained::into_raw(matching).as_ptr().cast(),
        )
    };
    if root == 0 {
        return None;
    }
    let key = CFString::from_static_str("AppleClamshellState");
    // SAFETY: `root` is a valid registry entry until released right after.
    let value = unsafe { IORegistryEntryCreateCFProperty(root, Some(&key), None, 0) };
    IOObjectRelease(root);
    Some(value?.downcast::<CFBoolean>().ok()?.as_bool())
}

pub(crate) struct SessionSource {
    observers: Vec<Observer>,
    stop: Arc<AtomicBool>,
}

impl SessionSource {
    pub(crate) fn new() -> Self {
        Self {
            observers: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for SessionSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl SignalSource for SessionSource {
    fn id(&self) -> String {
        ID.to_owned()
    }

    fn health(&self) -> SourceHealth {
        SourceHealth::Ok
    }

    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError> {
        let signals: Shared = Arc::new(Mutex::new(signals));

        // The lock notifications are undocumented but stable for over a decade.
        let distributed = NSDistributedNotificationCenter::defaultCenter().into_super();
        let main_queue = NSOperationQueue::mainQueue();
        for (name, kind) in [
            ("com.apple.screenIsLocked", SignalKind::Locked),
            ("com.apple.screenIsUnlocked", SignalKind::Unlocked),
        ] {
            let name = NSString::from_str(name);
            self.observers.push(observe(
                &distributed,
                &name,
                Some(&main_queue),
                &signals,
                kind,
            ));
        }

        let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: framework constants.
        let (will_sleep, did_wake) = unsafe {
            (
                NSWorkspaceWillSleepNotification,
                NSWorkspaceDidWakeNotification,
            )
        };
        self.observers.push(observe(
            &workspace,
            will_sleep,
            None,
            &signals,
            SignalKind::WillSleep,
        ));
        self.observers.push(observe(
            &workspace,
            did_wake,
            None,
            &signals,
            SignalKind::DidWake,
        ));

        // Polling the IORegistry instead of an IOKit interest notification: it needs no
        // run-loop port or undocumented message argument layout, a closed lid without an
        // external display already arrives as WillSleep, and one read per second is cheap.
        let stop = Arc::clone(&self.stop);
        thread::Builder::new()
            .name(format!("{ID}-lid"))
            .spawn(move || {
                let mut closed = lid_closed();
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(LID_INTERVAL);
                    let now = lid_closed();
                    if now == closed {
                        continue;
                    }
                    let kind = match (closed, now) {
                        (_, Some(true)) => SignalKind::LidClosed,
                        (Some(true), Some(false)) => SignalKind::LidOpened,
                        _ => {
                            closed = now;
                            continue;
                        }
                    };
                    closed = now;
                    if !send(&signals, signal(kind, Confidence::High, ID)) {
                        return;
                    }
                }
            })
            .map_err(|e| SignalError::Other(e.to_string()))?;
        Ok(())
    }
}
