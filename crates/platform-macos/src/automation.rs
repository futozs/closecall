//! Automation (Apple Events) consent per target app.

use std::collections::HashMap;
use std::ptr;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use closecall_core::platform::PermissionStatus;
use objc2_core_services::{
    AECreateDesc, AEDesc, AEDeterminePermissionToAutomateTarget, AEDisposeDesc,
    errAEEventNotPermitted, errAEEventWouldRequireUserConsent, typeApplicationBundleID, typeNull,
    typeWildCard,
};

const PROC_NOT_FOUND: i32 = -600;
const CACHE_TTL: Duration = Duration::from_secs(5);

/// `ask` shows the consent prompt and blocks until the user answers; only `request()` passes it.
pub(crate) fn status(bundle_id: &str, ask: bool) -> PermissionStatus {
    let mut target = AEDesc {
        descriptorType: typeNull,
        dataHandle: ptr::null_mut(),
    };
    // SAFETY: AECreateDesc copies `bundle_id`'s bytes into `target`, which is disposed below.
    let created = unsafe {
        AECreateDesc(
            typeApplicationBundleID,
            bundle_id.as_ptr().cast(),
            bundle_id.len() as _,
            &mut target,
        )
    };
    if created != 0 {
        return PermissionStatus::Denied;
    }
    // SAFETY: `target` is a valid address descriptor created above.
    let result = unsafe {
        let result =
            AEDeterminePermissionToAutomateTarget(&target, typeWildCard, typeWildCard, ask);
        AEDisposeDesc(&mut target);
        result
    };
    match result {
        0 => PermissionStatus::Granted,
        PROC_NOT_FOUND => PermissionStatus::AppNotRunning,
        r if r == errAEEventWouldRequireUserConsent => PermissionStatus::NotDetermined,
        r if r == errAEEventNotPermitted => PermissionStatus::Denied,
        _ => PermissionStatus::Denied,
    }
}

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, bool)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether we may send Apple Events to a running app without causing a prompt.
pub(crate) fn granted(bundle_id: &str) -> bool {
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&(at, granted)) = cache.get(bundle_id)
        && at.elapsed() < CACHE_TTL
    {
        return granted;
    }
    let granted = status(bundle_id, false) == PermissionStatus::Granted;
    cache.insert(bundle_id.to_owned(), (Instant::now(), granted));
    granted
}

pub(crate) fn forget(bundle_id: &str) {
    CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(bundle_id);
}
