//! Polling plus diffing into `InventoryEvent`s, and helpers shared by the providers.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use closecall_core::platform::{Discovered, EventSender, InventoryEvent, ProviderError, Reverify};
use closecall_core::{AppRef, ProviderId, Target, TargetId, TargetKind};

pub(crate) const INTERVAL: Duration = Duration::from_secs(2);
const SCAN_BUDGET: Duration = Duration::from_secs(5);
/// A page that rewrites its title every second wakes the poller constantly; scans still
/// leave the app this much room.
const MIN_GAP: Duration = Duration::from_millis(300);

/// The engine owns `id` and `first_seen_ms`; providers leave them zeroed.
pub(crate) fn target(provider: &str, app: AppRef, kind: TargetKind) -> Target {
    Target {
        id: TargetId(0),
        provider: ProviderId(provider.to_owned()),
        app,
        first_seen_ms: 0,
        kind,
    }
}

pub(crate) fn compare(old: &Target, mut fresh: Target) -> Reverify {
    fresh.id = old.id;
    fresh.first_seen_ms = old.first_seen_ms;
    if fresh == *old {
        Reverify::Same
    } else {
        Reverify::Changed(fresh)
    }
}

fn same_content(a: &Target, b: &Target) -> bool {
    a.provider == b.provider && a.app == b.app && a.kind == b.kind
}

pub(crate) fn diff(
    provider: &ProviderId,
    previous: &mut HashMap<String, Target>,
    current: Vec<Discovered>,
) -> Vec<InventoryEvent> {
    let mut events = Vec::new();
    let mut seen = HashSet::with_capacity(current.len());
    for found in current {
        seen.insert(found.key.clone());
        match previous.get(&found.key) {
            None => events.push(InventoryEvent::Added(found.clone())),
            Some(old) if !same_content(old, &found.target) => {
                events.push(InventoryEvent::Changed(found.clone()));
            }
            Some(_) => continue,
        }
        previous.insert(found.key, found.target);
    }
    previous.retain(|key, _| {
        let keep = seen.contains(key);
        if !keep {
            events.push(InventoryEvent::Removed {
                provider: provider.clone(),
                key: key.clone(),
            });
        }
        keep
    });
    events
}

pub(crate) struct Poller {
    stop: Arc<AtomicBool>,
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Scans every `INTERVAL`, or sooner when `wake` fires (an app launched or quit, a browser
/// window changed title). A failed scan keeps the previous state, so a busy app doesn't look
/// like it closed everything.
pub(crate) fn spawn(
    provider: ProviderId,
    wake: Receiver<()>,
    events: Box<dyn EventSender<InventoryEvent>>,
    mut scan: impl FnMut(Instant) -> Result<Vec<Discovered>, ProviderError> + Send + 'static,
) -> Result<Poller, ProviderError> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    thread::Builder::new()
        .name(format!("{}-poll", provider.0))
        .spawn(move || {
            let mut known = HashMap::new();
            while !stopped.load(Ordering::Relaxed) {
                let started = Instant::now();
                match scan(started + SCAN_BUDGET) {
                    Ok(found) => {
                        for event in diff(&provider, &mut known, found) {
                            if !events.send(event) {
                                return;
                            }
                        }
                    }
                    Err(error) => tracing::debug!(provider = %provider.0, %error, "scan failed"),
                }
                match wake.recv_timeout(INTERVAL) {
                    Ok(()) => {
                        thread::sleep(MIN_GAP.saturating_sub(started.elapsed()));
                        while wake.try_recv().is_ok() {}
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => thread::sleep(INTERVAL),
                }
            }
        })
        .map_err(|e| ProviderError::Other(e.to_string()))?;
    Ok(Poller { stop })
}

/// Last successful per-app result, reused when that app times out once. After the first full
/// scan it is also the provider's snapshot: the poller keeps it fresh, and a scan on the
/// provider's command thread would make a share's actions queue behind it.
pub(crate) struct LastGood<K> {
    by_app: Mutex<HashMap<K, Vec<Discovered>>>,
    complete: AtomicBool,
}

impl<K: Eq + Hash + Clone> LastGood<K> {
    pub(crate) fn new() -> Self {
        Self {
            by_app: Mutex::new(HashMap::new()),
            complete: AtomicBool::new(false),
        }
    }

    /// Everything the last full scan found, once there has been one.
    pub(crate) fn cached(&self) -> Option<Vec<Discovered>> {
        if !self.complete.load(Ordering::SeqCst) {
            return None;
        }
        let map = self.by_app.lock().unwrap_or_else(PoisonError::into_inner);
        Some(map.values().flatten().cloned().collect())
    }

    pub(crate) fn resolve<E>(
        &self,
        key: &K,
        result: Result<Vec<Discovered>, E>,
    ) -> Vec<Discovered> {
        let mut map = self.by_app.lock().unwrap_or_else(PoisonError::into_inner);
        match result {
            Ok(found) => {
                map.insert(key.clone(), found.clone());
                found
            }
            Err(_) => map.get(key).cloned().unwrap_or_default(),
        }
    }

    /// Ends a full scan.
    pub(crate) fn retain(&self, alive: &HashSet<K>) {
        self.by_app
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|key, _| alive.contains(key));
        self.complete.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use closecall_core::{AppWindow, Handle};

    use super::*;

    fn found(key: &str, title: &str) -> Discovered {
        Discovered {
            key: key.to_owned(),
            target: target(
                "macos.ax",
                AppRef {
                    id: "com.example".into(),
                    name: "Example".into(),
                    pid: Some(1),
                },
                TargetKind::AppWindow(AppWindow {
                    window: Handle(key.to_owned()),
                    title: title.to_owned(),
                    minimized: false,
                }),
            ),
        }
    }

    fn kinds(events: &[InventoryEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                InventoryEvent::Added(d) => format!("+{}", d.key),
                InventoryEvent::Changed(d) => format!("~{}", d.key),
                InventoryEvent::Removed { key, .. } => format!("-{key}"),
                InventoryEvent::Resync { .. } => "resync".into(),
            })
            .collect()
    }

    #[test]
    fn diff_reports_added_changed_and_removed() {
        let provider = ProviderId("macos.ax".into());
        let mut known = HashMap::new();
        let first = diff(
            &provider,
            &mut known,
            vec![found("a", "A"), found("b", "B")],
        );
        assert_eq!(kinds(&first), ["+a", "+b"]);
        let second = diff(
            &provider,
            &mut known,
            vec![found("a", "A"), found("b", "B2"), found("c", "C")],
        );
        assert_eq!(kinds(&second), ["~b", "+c"]);
        let mut third = kinds(&diff(&provider, &mut known, vec![found("c", "C")]));
        third.sort();
        assert_eq!(third, ["-a", "-b"]);
        assert!(diff(&provider, &mut known, vec![found("c", "C")]).is_empty());
    }

    #[test]
    fn compare_keeps_engine_owned_fields() {
        let mut old = found("a", "A").target;
        old.id = TargetId(9);
        old.first_seen_ms = 1234;
        assert_eq!(compare(&old, found("a", "A").target), Reverify::Same);
        match compare(&old, found("a", "Other").target) {
            Reverify::Changed(t) => {
                assert_eq!((t.id, t.first_seen_ms), (TargetId(9), 1234));
            }
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    #[test]
    fn last_good_survives_one_failure() {
        let cache = LastGood::new();
        assert_eq!(cache.resolve::<()>(&1, Ok(vec![found("a", "A")])).len(), 1);
        assert!(cache.cached().is_none(), "no full scan yet");
        cache.retain(&HashSet::from([1]));
        assert_eq!(cache.cached().map(|all| all.len()), Some(1));
        assert_eq!(cache.resolve(&1, Err(())).len(), 1);
        assert!(cache.resolve(&2, Err(())).is_empty());
        cache.retain(&HashSet::from([2]));
        assert!(cache.resolve(&1, Err(())).is_empty());
    }
}
