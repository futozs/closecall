use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync + 'static {
    /// Milliseconds since an arbitrary start. On macOS it stands still while the machine
    /// sleeps, so durations that must include sleep use [`Clock::unix_ms`].
    fn mono_ms(&self) -> u64;
    fn unix_ms(&self) -> u64;
}

pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn mono_ms(&self) -> u64 {
        millis(self.start.elapsed())
    }

    fn unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, millis)
    }
}

/// A clock that moves only when told to, for deterministic tests and scripted scenarios.
pub struct ManualClock {
    mono: AtomicU64,
    unix: AtomicU64,
}

impl ManualClock {
    /// Starts at monotonic 0 and 2026-11-16 09:00 UTC.
    pub fn new() -> Self {
        Self {
            mono: AtomicU64::new(0),
            unix: AtomicU64::new(1_794_819_600_000),
        }
    }

    pub fn advance(&self, by: Duration) {
        self.mono.fetch_add(millis(by), Ordering::SeqCst);
        self.unix.fetch_add(millis(by), Ordering::SeqCst);
    }

    /// Moves only the wall clock, like a Mac asleep with its monotonic clock stopped.
    pub fn sleep(&self, by: Duration) {
        self.unix.fetch_add(millis(by), Ordering::SeqCst);
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn mono_ms(&self) -> u64 {
        self.mono.load(Ordering::SeqCst)
    }

    fn unix_ms(&self) -> u64 {
        self.unix.load(Ordering::SeqCst)
    }
}

pub(crate) fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
