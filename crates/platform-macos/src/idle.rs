//! Time since the last keyboard or mouse input. Reading it needs no permission.

use std::time::Duration;

use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};

/// `kCGAnyInputEventType` (`~0`) is not in the bindings.
const ANY_INPUT: CGEventType = CGEventType(u32::MAX);

pub(crate) fn since_last_input() -> Option<Duration> {
    let seconds = CGEventSource::seconds_since_last_event_type(
        CGEventSourceStateID::CombinedSessionState,
        ANY_INPUT,
    );
    Duration::try_from_secs_f64(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_idle_time_without_permission() {
        assert!(since_last_input().is_some());
    }
}
