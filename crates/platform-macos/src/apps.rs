//! Running applications through `NSWorkspace` / `NSRunningApplication`. No permission needed.

use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, Once, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use block2::RcBlock;
use closecall_core::AppRef;
use objc2::rc::{Retained, autoreleasepool};
use objc2_app_kit::{
    NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace,
    NSWorkspaceDidLaunchApplicationNotification, NSWorkspaceDidTerminateApplicationNotification,
    NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{NSArray, NSError, NSNotification, NSString, NSURL};

pub(crate) const OWN_BUNDLE_ID: &str = "hu.futozs.closecall";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunningApp {
    pub pid: i32,
    pub bundle_id: Option<String>,
    pub name: String,
    pub exe: Option<String>,
}

impl RunningApp {
    pub(crate) fn app_ref(&self) -> AppRef {
        AppRef {
            id: self
                .bundle_id
                .clone()
                .or_else(|| self.exe.clone())
                .unwrap_or_else(|| self.name.clone()),
            name: self.name.clone(),
            pid: u32::try_from(self.pid).ok(),
        }
    }

    pub(crate) fn is_own(&self) -> bool {
        self.pid == own_pid() || self.bundle_id.as_deref() == Some(OWN_BUNDLE_ID)
    }
}

pub(crate) fn own_pid() -> i32 {
    std::process::id().cast_signed()
}

fn describe(app: &NSRunningApplication) -> RunningApp {
    let bundle_id = app.bundleIdentifier().map(|id| id.to_string());
    let exe = app
        .executableURL()
        .and_then(|url| url.path())
        .map(|path| path.to_string());
    let name = app
        .localizedName()
        .map(|name| name.to_string())
        .or_else(|| bundle_id.clone())
        .unwrap_or_default();
    RunningApp {
        pid: app.processIdentifier(),
        bundle_id,
        name,
        exe,
    }
}

fn process_exists(pid: i32) -> bool {
    // SAFETY: signal 0 only checks that the process exists; nothing is delivered.
    unsafe { libc::kill(pid, 0) == 0 || *libc::__error() == libc::EPERM }
}

/// Apps with a Dock icon and a menu bar, i.e. the ones that own user-visible windows.
/// `runningApplications` is refreshed only while the main run loop spins, so quit apps can
/// linger in it; the pid check drops them.
pub(crate) fn regular() -> Vec<RunningApp> {
    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace()
            .runningApplications()
            .iter()
            .filter(|app| {
                app.activationPolicy() == NSApplicationActivationPolicy::Regular
                    && !app.isTerminated()
                    && process_exists(app.processIdentifier())
            })
            .map(|app| describe(&app))
            .collect()
    })
}

fn running(pid: i32) -> Option<Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .filter(|app| !app.isTerminated() && process_exists(pid))
}

pub(crate) fn by_pid(pid: i32) -> Option<RunningApp> {
    autoreleasepool(|_| running(pid).map(|app| describe(&app)))
}

pub(crate) fn by_bundle(bundle_id: &str) -> Option<RunningApp> {
    autoreleasepool(|_| {
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
            bundle_id,
        ))
        .iter()
        .find(|app| !app.isTerminated() && process_exists(app.processIdentifier()))
        .map(|app| describe(&app))
    })
}

pub(crate) fn launched_at(pid: i32) -> Option<SystemTime> {
    autoreleasepool(|_| {
        let seconds = running(pid)?.launchDate()?.timeIntervalSince1970();
        (seconds > 0.0).then(|| UNIX_EPOCH + Duration::from_secs_f64(seconds))
    })
}

/// True when the app is hidden afterwards, also when it already was: a run hides one app
/// once per planned window.
pub(crate) fn hide(pid: i32) -> bool {
    autoreleasepool(|_| running(pid).is_some_and(|app| app.isHidden() || app.hide()))
}

pub(crate) fn unhide(pid: i32) -> bool {
    autoreleasepool(|_| running(pid).is_some_and(|app| !app.isHidden() || app.unhide()))
}

/// Polite quit: the app may still ask about unsaved documents. Never `forceTerminate`.
pub(crate) fn terminate(pid: i32) -> bool {
    autoreleasepool(|_| running(pid).is_some_and(|app| app.terminate()))
}

pub(crate) fn is_installed(bundle_id: &str) -> bool {
    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle_id))
            .is_some()
    })
}

/// Domain and code only: the localized description names the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenError {
    NoSuchApp,
    Failed { domain: String, code: isize },
    Timeout,
}

/// Opens a file with the app that has `bundle_id`, launching it if needed, and waits for
/// Launch Services to confirm until `deadline`.
pub(crate) fn open_with(path: &str, bundle_id: &str, deadline: Instant) -> Result<(), OpenError> {
    let (sender, receiver) = mpsc::channel();
    autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle_id))
            .ok_or(OpenError::NoSuchApp)?;
        let file = NSURL::fileURLWithPath(&NSString::from_str(path));
        let done = RcBlock::new(move |_: *mut NSRunningApplication, error: *mut NSError| {
            // SAFETY: Launch Services passes either null or a valid `NSError` for the
            // duration of the call.
            let error = unsafe { error.as_ref() }.map(|error| OpenError::Failed {
                domain: error.domain().to_string(),
                code: error.code(),
            });
            let _ = sender.send(error);
        });
        workspace.openURLs_withApplicationAtURL_configuration_completionHandler(
            &NSArray::from_retained_slice(&[file]),
            &app,
            &NSWorkspaceOpenConfiguration::configuration(),
            Some(&done),
        );
        Ok(())
    })?;
    match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(None) => Ok(()),
        Ok(Some(error)) => Err(error),
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => Err(OpenError::Timeout),
    }
}

pub(crate) fn open_url(url: &str) -> bool {
    autoreleasepool(|_| {
        NSURL::URLWithString(&NSString::from_str(url))
            .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url))
    })
}

static SUBSCRIBERS: Mutex<Vec<Sender<()>>> = Mutex::new(Vec::new());
static OBSERVING: Once = Once::new();

/// Fires whenever an app launches or quits. Workspace notifications are posted on the main
/// thread, so they only arrive while the main run loop runs (it does inside the app).
pub(crate) fn launches() -> Receiver<()> {
    let (sender, receiver) = mpsc::channel();
    notify_launches(sender);
    receiver
}

pub(crate) fn notify_launches(sender: Sender<()>) {
    SUBSCRIBERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(sender);
    OBSERVING.call_once(observe_launches);
}

fn observe_launches() {
    let block = RcBlock::new(|_: NonNull<NSNotification>| {
        SUBSCRIBERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|subscriber| subscriber.send(()).is_ok());
    });
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    // SAFETY: framework constants.
    let names = unsafe {
        [
            NSWorkspaceDidLaunchApplicationNotification,
            NSWorkspaceDidTerminateApplicationNotification,
        ]
    };
    for name in names {
        // SAFETY: valid name and block. The center keeps block observers registered until
        // `removeObserver:`, so dropping the returned token leaves the registration in place
        // for the life of the process, which is what we want here.
        let _token = unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)
        };
    }
}
