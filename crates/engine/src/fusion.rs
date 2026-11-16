//! Turns signals and timers into fired triggers. Pure state, no threads or clock reads: the
//! caller passes `Now`, so every rule here is unit-testable.

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::ipc::ShareState;
use closecall_core::platform::{Signal, SignalKind};
use closecall_core::{Confidence, Dur, FiredTrigger, TargetId, Trigger};

pub(crate) const DEBOUNCE_MS: u64 = 10_000;
pub(crate) const SHARE_STOP_MS: u64 = 3_000;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Now {
    pub mono_ms: u64,
    pub unix_ms: u64,
}

#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub fire: Vec<FiredTrigger>,
    pub share_started: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Share,
    Mirrored,
    CallLikely,
    WillSleep,
    LidClosed,
    Locked,
    Away,
    Idle,
    ItemAge(TargetId),
}

#[derive(Debug)]
struct Capture {
    confidence: Confidence,
    capturing_app: Option<String>,
    stop_at: Option<u64>,
}

#[derive(Debug, Default)]
pub(crate) struct Fusion {
    capture: Option<Capture>,
    mirrored: bool,
    unmirror_at: Option<u64>,
    call: bool,
    picker_open: bool,
    locked: bool,
    asleep: bool,
    /// Wall clock, because the monotonic clock stops while a Mac sleeps.
    away_since_unix: Option<u64>,
    away_fired: BTreeSet<Dur>,
    idle_last: Option<Duration>,
    idle_fired: BTreeSet<Dur>,
    age_fired: HashMap<TargetId, BTreeSet<Dur>>,
    last_fired: HashMap<Key, u64>,
}

impl Fusion {
    pub fn share_active(&self, min: Confidence) -> bool {
        self.capture.as_ref().is_some_and(|c| c.confidence >= min)
    }

    pub fn share_state(&self, min: Confidence) -> ShareState {
        let low_capture = self.capture.as_ref().is_some_and(|c| c.confidence < min);
        ShareState {
            active: self.share_active(min),
            confidence: self.capture.as_ref().map(|c| c.confidence),
            mirrored_display: self.mirrored,
            call_likely: self.call || self.picker_open || low_capture,
        }
    }

    /// The triggers whose rules share-guard applies to new and changed targets.
    pub fn guard_triggers(&self, min: Confidence) -> Vec<FiredTrigger> {
        let mut triggers = Vec::new();
        if let Some(capture) = self.capture.as_ref().filter(|c| c.confidence >= min) {
            triggers.push(FiredTrigger::ScreenShare {
                confidence: capture.confidence,
            });
        }
        if self.mirrored {
            triggers.push(FiredTrigger::DisplayMirrored);
        }
        triggers
    }

    pub fn capturing_app(&self) -> Option<&str> {
        self.capture.as_ref()?.capturing_app.as_deref()
    }

    pub fn on_signal(
        &mut self,
        signal: &Signal,
        now: Now,
        config: &Config,
        paused: bool,
    ) -> Outcome {
        let min = config.share_min_confidence;
        let mut out = Outcome::default();
        match signal.kind {
            SignalKind::CaptureStarted => {
                let was_active = self.share_active(min);
                self.picker_open = false;
                let capture = self.capture.get_or_insert(Capture {
                    confidence: signal.confidence,
                    capturing_app: None,
                    stop_at: None,
                });
                capture.confidence = capture.confidence.max(signal.confidence);
                capture.stop_at = None;
                if signal.capturing_app.is_some() {
                    capture.capturing_app.clone_from(&signal.capturing_app);
                }
                if signal.confidence >= min {
                    if !was_active {
                        out.share_started = !paused;
                        self.fire(
                            FiredTrigger::ScreenShare {
                                confidence: signal.confidence,
                            },
                            now,
                            paused,
                            &mut out,
                        );
                    }
                } else {
                    self.fire(FiredTrigger::CallLikely, now, paused, &mut out);
                }
            }
            SignalKind::CaptureStopped => {
                if let Some(capture) = &mut self.capture {
                    capture.stop_at.get_or_insert(now.mono_ms + SHARE_STOP_MS);
                }
            }
            SignalKind::SharePickerOpened => {
                self.picker_open = true;
                self.fire(FiredTrigger::CallLikely, now, paused, &mut out);
            }
            SignalKind::CallLikely => {
                self.call = true;
                self.fire(FiredTrigger::CallLikely, now, paused, &mut out);
            }
            SignalKind::CallEnded => self.call = false,
            SignalKind::DisplayMirrored => {
                let was = self.mirrored;
                self.mirrored = true;
                self.unmirror_at = None;
                if !was {
                    self.fire(FiredTrigger::DisplayMirrored, now, paused, &mut out);
                }
            }
            SignalKind::DisplayUnmirrored => {
                if self.mirrored {
                    self.unmirror_at.get_or_insert(now.mono_ms + SHARE_STOP_MS);
                }
            }
            SignalKind::WillSleep => {
                self.asleep = true;
                self.away_since_unix.get_or_insert(now.unix_ms);
                self.fire(FiredTrigger::WillSleep, now, paused, &mut out);
            }
            SignalKind::DidWake => {
                self.asleep = false;
                self.check_away(now, config, paused, &mut out);
                // Without a password on wake there is no locked window to act in: act now.
                if !self.locked {
                    self.end_away();
                }
            }
            SignalKind::Locked => {
                self.locked = true;
                self.away_since_unix.get_or_insert(now.unix_ms);
                self.fire(FiredTrigger::Locked, now, paused, &mut out);
            }
            SignalKind::Unlocked => {
                self.locked = false;
                if !self.asleep {
                    self.end_away();
                }
            }
            SignalKind::LidClosed => {
                self.fire(FiredTrigger::LidClosed, now, paused, &mut out);
            }
            SignalKind::LidOpened => {}
            SignalKind::Panic => {
                self.fire(FiredTrigger::Panic, now, paused, &mut out);
            }
        }
        out
    }

    /// Ends debounced shares and mirroring, and fires away thresholds crossed while locked.
    pub fn on_tick(&mut self, now: Now, config: &Config, paused: bool) -> Outcome {
        let mut out = Outcome::default();
        if self
            .capture
            .as_ref()
            .and_then(|c| c.stop_at)
            .is_some_and(|at| at <= now.mono_ms)
        {
            self.capture = None;
        }
        if self.unmirror_at.is_some_and(|at| at <= now.mono_ms) {
            self.mirrored = false;
            self.unmirror_at = None;
        }
        if self.locked || self.asleep {
            self.check_away(now, config, paused, &mut out);
        }
        out
    }

    /// One fire per newly crossed threshold set; a new idle period (input seen) starts over.
    pub fn on_idle(&mut self, idle: Duration, now: Now, config: &Config, paused: bool) -> Outcome {
        let mut out = Outcome::default();
        if self.idle_last.is_some_and(|last| idle < last) {
            self.idle_fired.clear();
        }
        self.idle_last = Some(idle);
        let crossed = crossed(config, idle_threshold, idle, &self.idle_fired);
        if !crossed.is_empty()
            && self.fire(
                FiredTrigger::Idle { idle: Dur(idle) },
                now,
                paused,
                &mut out,
            )
        {
            self.idle_fired.extend(crossed);
        }
        out
    }

    pub fn on_ages(
        &mut self,
        ages: &[(TargetId, u64)],
        now: Now,
        config: &Config,
        paused: bool,
    ) -> Outcome {
        let mut out = Outcome::default();
        self.age_fired
            .retain(|id, _| ages.iter().any(|(alive, _)| alive == id));
        for &(target, first_seen_ms) in ages {
            let age = Duration::from_millis(now.mono_ms.saturating_sub(first_seen_ms));
            static NONE: BTreeSet<Dur> = BTreeSet::new();
            let fired = self.age_fired.get(&target).unwrap_or(&NONE);
            let crossed = crossed(config, age_threshold, age, fired);
            if crossed.is_empty() {
                continue;
            }
            let trigger = FiredTrigger::ItemAge {
                target,
                age: Dur(age),
            };
            if self.fire(trigger, now, paused, &mut out) {
                self.age_fired.entry(target).or_default().extend(crossed);
            }
        }
        out
    }

    fn check_away(&mut self, now: Now, config: &Config, paused: bool, out: &mut Outcome) {
        let Some(since) = self.away_since_unix else {
            return;
        };
        let away = Duration::from_millis(now.unix_ms.saturating_sub(since));
        let crossed = crossed(config, away_threshold, away, &self.away_fired);
        if !crossed.is_empty()
            && self.fire(FiredTrigger::Away { away: Dur(away) }, now, paused, out)
        {
            self.away_fired.extend(crossed);
        }
    }

    fn end_away(&mut self) {
        self.away_since_unix = None;
        self.away_fired.clear();
    }

    fn fire(&mut self, fired: FiredTrigger, now: Now, paused: bool, out: &mut Outcome) -> bool {
        let admitted = self.admit(&fired, now, paused);
        if admitted {
            out.fire.push(fired);
        }
        admitted
    }

    /// Panic always fires; everything else waits out a pause and the 10 s debounce.
    fn admit(&mut self, fired: &FiredTrigger, now: Now, paused: bool) -> bool {
        let Some(key) = key(fired) else {
            return true;
        };
        if paused {
            return false;
        }
        if self
            .last_fired
            .get(&key)
            .is_some_and(|&at| now.mono_ms < at + DEBOUNCE_MS)
        {
            return false;
        }
        self.last_fired.insert(key, now.mono_ms);
        true
    }
}

fn key(fired: &FiredTrigger) -> Option<Key> {
    Some(match fired {
        FiredTrigger::Panic => return None,
        FiredTrigger::ScreenShare { .. } => Key::Share,
        FiredTrigger::DisplayMirrored => Key::Mirrored,
        FiredTrigger::CallLikely => Key::CallLikely,
        FiredTrigger::WillSleep => Key::WillSleep,
        FiredTrigger::LidClosed => Key::LidClosed,
        FiredTrigger::Locked => Key::Locked,
        FiredTrigger::Away { .. } => Key::Away,
        FiredTrigger::Idle { .. } => Key::Idle,
        FiredTrigger::ItemAge { target, .. } => Key::ItemAge(*target),
    })
}

fn away_threshold(trigger: &Trigger) -> Option<Dur> {
    match trigger {
        Trigger::AwayLongerThan(min) => Some(*min),
        _ => None,
    }
}

fn idle_threshold(trigger: &Trigger) -> Option<Dur> {
    match trigger {
        Trigger::IdleLongerThan(min) => Some(*min),
        _ => None,
    }
}

fn age_threshold(trigger: &Trigger) -> Option<Dur> {
    match trigger {
        Trigger::ItemOpenLongerThan(min) => Some(*min),
        _ => None,
    }
}

fn crossed(
    config: &Config,
    threshold: fn(&Trigger) -> Option<Dur>,
    elapsed: Duration,
    fired: &BTreeSet<Dur>,
) -> Vec<Dur> {
    let thresholds: BTreeSet<Dur> = config
        .rules
        .iter()
        .filter(|rule| rule.enabled)
        .flat_map(|rule| rule.when.iter().filter_map(threshold))
        .collect();
    thresholds
        .into_iter()
        .filter(|min| min.0 <= elapsed && !fired.contains(min))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(mono_s: u64, unix_s: u64) -> Now {
        Now {
            mono_ms: mono_s * 1000,
            unix_ms: unix_s * 1000,
        }
    }

    fn signal(kind: SignalKind, confidence: Confidence) -> Signal {
        Signal {
            kind,
            confidence,
            source: "test".to_owned(),
            capturing_app: None,
        }
    }

    #[test]
    fn share_stop_is_debounced_and_a_quick_restart_is_the_same_share() {
        let config = Config::recommended();
        let mut fusion = Fusion::default();
        let started = fusion.on_signal(
            &signal(SignalKind::CaptureStarted, Confidence::High),
            now(0, 0),
            &config,
            false,
        );
        assert!(started.share_started);
        fusion.on_signal(
            &signal(SignalKind::CaptureStopped, Confidence::High),
            now(1, 1),
            &config,
            false,
        );
        fusion.on_tick(now(2, 2), &config, false);
        assert!(fusion.share_active(Confidence::Medium));
        let again = fusion.on_signal(
            &signal(SignalKind::CaptureStarted, Confidence::High),
            now(3, 3),
            &config,
            false,
        );
        assert!(again.fire.is_empty() && !again.share_started);
        fusion.on_signal(
            &signal(SignalKind::CaptureStopped, Confidence::High),
            now(5, 5),
            &config,
            false,
        );
        fusion.on_tick(now(8, 8), &config, false);
        assert!(!fusion.share_active(Confidence::Medium));
    }

    #[test]
    fn away_uses_the_wall_clock_across_sleep() {
        let config = Config::recommended();
        let mut fusion = Fusion::default();
        fusion.on_signal(
            &signal(SignalKind::Locked, Confidence::High),
            now(0, 0),
            &config,
            false,
        );
        fusion.on_signal(
            &signal(SignalKind::WillSleep, Confidence::High),
            now(1, 1),
            &config,
            false,
        );
        let woke = fusion.on_signal(
            &signal(SignalKind::DidWake, Confidence::High),
            now(2, 20 * 60),
            &config,
            false,
        );
        assert!(
            matches!(woke.fire.as_slice(), [FiredTrigger::Away { away }] if *away >= Dur::from_mins(15))
        );
        let tick = fusion.on_tick(now(30, 21 * 60), &config, false);
        assert!(tick.fire.is_empty());
    }
}
