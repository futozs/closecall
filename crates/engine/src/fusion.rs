//! Turns signals and timers into fired triggers. Pure state, no threads or clock reads: the
//! caller passes `Now`, so every rule here is unit-testable.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::ipc::ShareState;
use closecall_core::platform::{Signal, SignalKind};
use closecall_core::{Confidence, DisplayInfo, Dur, FiredTrigger, TargetId, Trigger};

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

impl Outcome {
    pub fn merge(&mut self, other: Outcome) {
        self.fire.extend(other.fire);
        self.share_started |= other.share_started;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Share,
    Mirrored,
    DisplayConnected,
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
    /// The ignored always-on capturer this capture is put down to; it then counts for nothing.
    ignored_by: Option<String>,
    stop_at: Option<u64>,
}

#[derive(Debug)]
struct Shown {
    display: Option<DisplayInfo>,
    gone_at: Option<u64>,
}

/// Keyed by the OS display id; `None` for a display the OS did not identify.
type Displays = BTreeMap<Option<u32>, Shown>;

#[derive(Debug, Default)]
pub(crate) struct Fusion {
    /// By signal source, so one source stopping does not end another source's capture.
    captures: BTreeMap<String, Capture>,
    mirrored: Displays,
    external: Displays,
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
    fn counted(&self) -> impl Iterator<Item = &Capture> {
        self.captures
            .values()
            .filter(|capture| capture.ignored_by.is_none())
    }

    pub fn share_active(&self, min: Confidence) -> bool {
        self.counted().any(|capture| capture.confidence >= min)
    }

    pub fn mirrored(&self, config: &Config) -> bool {
        showing(&self.mirrored, config)
    }

    /// Only while a rule is armed by it: unarmed, a desk monitor would keep the guard on all day.
    pub fn external_display(&self, config: &Config) -> bool {
        showing(&self.external, config) && arms(config, &Trigger::DisplayConnected)
    }

    pub fn share_state(&self, config: &Config) -> ShareState {
        let min = config.share_min_confidence;
        let low_capture = self.counted().any(|capture| capture.confidence < min);
        ShareState {
            active: self.share_active(min),
            confidence: self.counted().map(|capture| capture.confidence).max(),
            mirrored_display: self.mirrored(config),
            external_display: self.external_display(config),
            call_likely: self.call || self.picker_open || low_capture,
            ignored_capture: self
                .captures
                .values()
                .find_map(|capture| capture.ignored_by.clone()),
        }
    }

    /// The triggers whose rules share-guard applies to new and changed targets.
    pub fn guard_triggers(&self, config: &Config) -> Vec<FiredTrigger> {
        let min = config.share_min_confidence;
        let mut triggers = Vec::new();
        if let Some(confidence) = self
            .counted()
            .map(|capture| capture.confidence)
            .filter(|confidence| *confidence >= min)
            .max()
        {
            triggers.push(FiredTrigger::ScreenShare { confidence });
        }
        if self.mirrored(config) {
            triggers.push(FiredTrigger::DisplayMirrored);
        }
        if self.external_display(config) {
            triggers.push(FiredTrigger::DisplayConnected);
        }
        triggers
    }

    pub fn capturing_apps(&self) -> Vec<&str> {
        self.counted()
            .filter_map(|capture| capture.capturing_app.as_deref())
            .collect()
    }

    /// Sources whose capture is put down to an ignored app.
    pub fn ignored_sources(&self) -> Vec<&str> {
        self.captures
            .iter()
            .filter(|(_, capture)| capture.ignored_by.is_some())
            .map(|(source, _)| source.as_str())
            .collect()
    }

    /// Captures that have not stopped, by source, with the app the OS put them down to.
    pub fn running_captures(&self) -> Vec<(String, Option<String>)> {
        self.captures
            .iter()
            .filter(|(_, capture)| capture.stop_at.is_none())
            .map(|(source, capture)| (source.clone(), capture.capturing_app.clone()))
            .collect()
    }

    /// `ignored_by` is worked out by the caller for `CaptureStarted`: it needs the running apps.
    pub fn on_signal(
        &mut self,
        signal: &Signal,
        ignored_by: Option<String>,
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
                let capture = self
                    .captures
                    .entry(signal.source.clone())
                    .or_insert(Capture {
                        confidence: signal.confidence,
                        capturing_app: None,
                        ignored_by: None,
                        stop_at: None,
                    });
                capture.confidence = capture.confidence.max(signal.confidence);
                capture.stop_at = None;
                if signal.capturing_app.is_some() {
                    capture.capturing_app.clone_from(&signal.capturing_app);
                }
                capture.ignored_by = ignored_by;
                if capture.ignored_by.is_some() {
                    return out;
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
                if let Some(capture) = self.captures.get_mut(&signal.source) {
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
                let display = signal.display.as_ref();
                if show(&mut self.mirrored, display) && counts(display, config) {
                    self.fire(FiredTrigger::DisplayMirrored, now, paused, &mut out);
                }
            }
            SignalKind::DisplayUnmirrored => {
                hide(&mut self.mirrored, signal.display.as_ref(), now);
            }
            SignalKind::DisplayConnected => {
                let display = signal.display.as_ref();
                if show(&mut self.external, display) && counts(display, config) {
                    self.fire(FiredTrigger::DisplayConnected, now, paused, &mut out);
                }
            }
            SignalKind::DisplayDisconnected => {
                hide(&mut self.external, signal.display.as_ref(), now);
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

    /// After the ignore list changed: a capture that no longer counts as an ignored app's
    /// starts a share like a fresh one would.
    pub fn reattribute(
        &mut self,
        source: &str,
        ignored_by: Option<String>,
        now: Now,
        config: &Config,
        paused: bool,
    ) -> Outcome {
        let min = config.share_min_confidence;
        let mut out = Outcome::default();
        let was_active = self.share_active(min);
        let Some(capture) = self.captures.get_mut(source) else {
            return out;
        };
        capture.ignored_by = ignored_by;
        let confidence = capture.confidence;
        if !was_active && self.share_active(min) {
            out.share_started = !paused;
            self.fire(
                FiredTrigger::ScreenShare { confidence },
                now,
                paused,
                &mut out,
            );
        }
        out
    }

    /// Ends debounced shares, mirroring and disconnects, and fires away thresholds crossed
    /// while locked.
    pub fn on_tick(&mut self, now: Now, config: &Config, paused: bool) -> Outcome {
        let mut out = Outcome::default();
        let ended = |at: Option<u64>| at.is_some_and(|at| at <= now.mono_ms);
        self.captures.retain(|_, capture| !ended(capture.stop_at));
        self.mirrored.retain(|_, shown| !ended(shown.gone_at));
        self.external.retain(|_, shown| !ended(shown.gone_at));
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
        FiredTrigger::DisplayConnected => Key::DisplayConnected,
        FiredTrigger::CallLikely => Key::CallLikely,
        FiredTrigger::WillSleep => Key::WillSleep,
        FiredTrigger::LidClosed => Key::LidClosed,
        FiredTrigger::Locked => Key::Locked,
        FiredTrigger::Away { .. } => Key::Away,
        FiredTrigger::Idle { .. } => Key::Idle,
        FiredTrigger::ItemAge { target, .. } => Key::ItemAge(*target),
    })
}

/// True when the display is new, not back within the disconnect debounce.
fn show(displays: &mut Displays, display: Option<&DisplayInfo>) -> bool {
    match displays.entry(display.map(|display| display.id)) {
        Entry::Occupied(mut entry) => {
            let shown = entry.get_mut();
            shown.gone_at = None;
            shown.display = display.cloned();
            false
        }
        Entry::Vacant(entry) => {
            entry.insert(Shown {
                display: display.cloned(),
                gone_at: None,
            });
            true
        }
    }
}

fn hide(displays: &mut Displays, display: Option<&DisplayInfo>, now: Now) {
    if let Some(shown) = displays.get_mut(&display.map(|display| display.id)) {
        shown.gone_at.get_or_insert(now.mono_ms + SHARE_STOP_MS);
    }
}

/// Checked against the config of the moment, so editing the ignore list takes effect at once.
fn counts(display: Option<&DisplayInfo>, config: &Config) -> bool {
    display.is_none_or(|display| !display.builtin && !config.ignore.display(display))
}

fn showing(displays: &Displays, config: &Config) -> bool {
    displays
        .values()
        .any(|shown| counts(shown.display.as_ref(), config))
}

pub(crate) fn arms(config: &Config, trigger: &Trigger) -> bool {
    config
        .rules
        .iter()
        .any(|rule| rule.enabled && rule.when.contains(trigger))
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
            display: None,
        }
    }

    #[test]
    fn share_stop_is_debounced_and_a_quick_restart_is_the_same_share() {
        let config = Config::recommended();
        let mut fusion = Fusion::default();
        let started = fusion.on_signal(
            &signal(SignalKind::CaptureStarted, Confidence::High),
            None,
            now(0, 0),
            &config,
            false,
        );
        assert!(started.share_started);
        fusion.on_signal(
            &signal(SignalKind::CaptureStopped, Confidence::High),
            None,
            now(1, 1),
            &config,
            false,
        );
        fusion.on_tick(now(2, 2), &config, false);
        assert!(fusion.share_active(Confidence::Medium));
        let again = fusion.on_signal(
            &signal(SignalKind::CaptureStarted, Confidence::High),
            None,
            now(3, 3),
            &config,
            false,
        );
        assert!(again.fire.is_empty() && !again.share_started);
        fusion.on_signal(
            &signal(SignalKind::CaptureStopped, Confidence::High),
            None,
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
            None,
            now(0, 0),
            &config,
            false,
        );
        fusion.on_signal(
            &signal(SignalKind::WillSleep, Confidence::High),
            None,
            now(1, 1),
            &config,
            false,
        );
        let woke = fusion.on_signal(
            &signal(SignalKind::DidWake, Confidence::High),
            None,
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

    fn capture(source: &str, kind: SignalKind) -> Signal {
        Signal {
            source: source.to_owned(),
            ..signal(kind, Confidence::High)
        }
    }

    #[test]
    fn one_source_stopping_does_not_end_another_sources_capture() {
        let config = Config::recommended();
        let mut fusion = Fusion::default();
        fusion.on_signal(
            &capture("skylight", SignalKind::CaptureStarted),
            None,
            now(0, 0),
            &config,
            false,
        );
        let second = fusion.on_signal(
            &capture("share_ui", SignalKind::CaptureStarted),
            None,
            now(1, 1),
            &config,
            false,
        );
        assert!(second.fire.is_empty(), "already sharing");
        fusion.on_signal(
            &capture("share_ui", SignalKind::CaptureStopped),
            None,
            now(2, 2),
            &config,
            false,
        );
        fusion.on_tick(now(10, 10), &config, false);
        assert!(fusion.share_active(Confidence::Medium));
        fusion.on_signal(
            &capture("skylight", SignalKind::CaptureStopped),
            None,
            now(11, 11),
            &config,
            false,
        );
        fusion.on_tick(now(20, 20), &config, false);
        assert!(!fusion.share_active(Confidence::Low));
    }

    #[test]
    fn an_ignored_capture_counts_for_nothing_until_reattributed() {
        let config = Config::recommended();
        let mut fusion = Fusion::default();
        let out = fusion.on_signal(
            &capture("skylight", SignalKind::CaptureStarted),
            Some("DisplayLink Manager".to_owned()),
            now(0, 0),
            &config,
            false,
        );
        assert!(out.fire.is_empty() && !out.share_started);
        let state = fusion.share_state(&config);
        assert!(!state.active && !state.call_likely);
        assert_eq!(state.confidence, None);
        assert_eq!(
            state.ignored_capture.as_deref(),
            Some("DisplayLink Manager")
        );
        assert_eq!(fusion.ignored_sources(), ["skylight"]);
        assert!(fusion.guard_triggers(&config).is_empty());

        let ui = fusion.on_signal(
            &Signal {
                capturing_app: Some("us.zoom.xos".to_owned()),
                confidence: Confidence::Medium,
                ..capture("share_ui", SignalKind::CaptureStarted)
            },
            None,
            now(1, 1),
            &config,
            false,
        );
        assert!(ui.share_started);
        assert_eq!(fusion.capturing_apps(), ["us.zoom.xos"]);

        let mut fusion = Fusion::default();
        fusion.on_signal(
            &capture("skylight", SignalKind::CaptureStarted),
            Some("DisplayLink Manager".to_owned()),
            now(0, 0),
            &config,
            false,
        );
        let out = fusion.reattribute("skylight", None, now(5, 5), &config, false);
        assert!(out.share_started);
        assert!(matches!(
            out.fire.as_slice(),
            [FiredTrigger::ScreenShare { .. }]
        ));
    }

    fn display(id: u32, name: &str) -> DisplayInfo {
        DisplayInfo {
            id,
            name: name.to_owned(),
            vendor: 7789,
            model: id,
            serial: None,
            builtin: false,
            mirrored: false,
        }
    }

    fn about(kind: SignalKind, display: DisplayInfo) -> Signal {
        Signal {
            display: Some(display),
            ..signal(kind, Confidence::High)
        }
    }

    #[test]
    fn ignored_displays_never_fire_and_others_fire_once_each() {
        let mut config = Config::recommended();
        config.ignore.displays = vec![closecall_core::DisplaySelector::Name("LG".to_owned())];
        let mut fusion = Fusion::default();
        let lg = display(2, "LG HDR 4K");
        let projector = display(3, "EPSON PJ");

        for kind in [SignalKind::DisplayConnected, SignalKind::DisplayMirrored] {
            let out = fusion.on_signal(&about(kind, lg.clone()), None, now(0, 0), &config, false);
            assert!(out.fire.is_empty(), "{kind:?}");
        }
        assert!(!fusion.mirrored(&config));
        assert!(fusion.guard_triggers(&config).is_empty());

        let connected = fusion.on_signal(
            &about(SignalKind::DisplayConnected, projector.clone()),
            None,
            now(1, 1),
            &config,
            false,
        );
        assert_eq!(connected.fire, [FiredTrigger::DisplayConnected]);
        assert!(
            !fusion.external_display(&config),
            "no rule is armed by display_connected"
        );

        let mirrored = fusion.on_signal(
            &about(SignalKind::DisplayMirrored, projector.clone()),
            None,
            now(2, 2),
            &config,
            false,
        );
        assert_eq!(mirrored.fire, [FiredTrigger::DisplayMirrored]);
        assert_eq!(
            fusion.guard_triggers(&config),
            [FiredTrigger::DisplayMirrored]
        );

        fusion.on_signal(
            &about(SignalKind::DisplayUnmirrored, projector.clone()),
            None,
            now(3, 3),
            &config,
            false,
        );
        fusion.on_tick(now(4, 4), &config, false);
        assert!(fusion.mirrored(&config), "unmirroring is debounced");
        fusion.on_tick(now(7, 7), &config, false);
        assert!(!fusion.mirrored(&config));

        config.ignore.displays.clear();
        assert!(
            fusion.share_state(&config).mirrored_display,
            "the LG counts as soon as it is no longer ignored"
        );
    }
}
