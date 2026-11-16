use std::time::Duration;

use closecall_core::platform::SignalKind;
use closecall_core::{Action, Confidence, Dur, FiredTrigger, TargetKindTag, Trigger};
use closecall_engine::Notice;
use closecall_platform_fake::chrome_tab;

use crate::harness::{Harness, category, recommended_with, rule};

const MIN: u64 = 60;

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn otp_tab() -> closecall_core::Target {
    chrome_tab(
        "chrome-otp",
        "https://netbank.otpbank.hu/portal/hu/Szamlak",
        "OTP Bank – Internetbank",
    )
}

#[test]
fn lock_sleep_wake_while_locked_fires_the_away_rule_before_unlock() {
    let h = Harness::demo();
    h.signal(SignalKind::Locked);
    assert!(
        h.fake.executed().is_empty(),
        "no rule is armed by a plain lock"
    );

    h.signal(SignalKind::WillSleep);
    assert!(
        h.closed("chrome-adult"),
        "lid-close-adult runs at will-sleep"
    );
    assert!(!h.closed("chrome-otp"));

    h.clock.sleep(secs(20 * MIN));
    h.signal(SignalKind::DidWake);
    assert!(h.closed("chrome-otp"), "away rule fired while still locked");
    let away = h.sink.runs_for(|t| matches!(t, FiredTrigger::Away { .. }));
    assert_eq!(away.len(), 1);
    assert!(matches!(away[0].trigger, FiredTrigger::Away { away } if away >= Dur::from_mins(20)));

    h.advance(secs(30));
    h.signal(SignalKind::Unlocked);
    h.advance(secs(30 * MIN));
    assert_eq!(
        h.sink
            .runs_for(|t| matches!(t, FiredTrigger::Away { .. }))
            .len(),
        1
    );
}

#[test]
fn away_fires_on_the_scheduler_while_locked_without_sleep() {
    let h = Harness::demo();
    h.signal(SignalKind::Locked);
    h.advance(secs(14 * MIN));
    assert!(!h.closed("chrome-otp"));
    h.advance(secs(MIN + 15));
    assert!(h.closed("chrome-otp"));
}

#[test]
fn away_after_unlock_starts_from_zero() {
    let h = Harness::demo();
    h.signal(SignalKind::Locked);
    h.advance(secs(10 * MIN));
    h.signal(SignalKind::Unlocked);
    h.signal(SignalKind::Locked);
    h.advance(secs(10 * MIN));
    assert!(!h.closed("chrome-otp"));
}

#[test]
fn idle_fires_once_per_threshold_per_idle_period() {
    let h = Harness::with_config(&recommended_with(vec![rule(
        "idle-banking",
        vec![Trigger::IdleLongerThan(Dur::from_mins(5))],
        vec![TargetKindTag::Tab],
        category("banking"),
        Action::Close,
    )]));
    let idle_runs = || {
        h.sink
            .runs_for(|t| matches!(t, FiredTrigger::Idle { .. }))
            .len()
    };

    h.fake.set_idle(Some(secs(4 * MIN)));
    h.advance(secs(30));
    assert!(!h.closed("chrome-otp"));

    h.fake.set_idle(Some(secs(6 * MIN)));
    h.advance(secs(30));
    assert!(h.closed("chrome-otp"));
    assert_eq!(idle_runs(), 1);

    h.fake.add(otp_tab());
    h.fake.set_idle(Some(secs(7 * MIN)));
    h.advance(secs(30));
    assert_eq!(idle_runs(), 1, "same idle period");

    h.fake.set_idle(Some(secs(5)));
    h.advance(secs(30));
    h.fake.set_idle(Some(secs(6 * MIN)));
    h.advance(secs(30));
    assert_eq!(idle_runs(), 2, "input started a new idle period");
}

#[test]
fn item_age_waits_out_the_countdown_then_closes() {
    let h = Harness::demo();
    h.advance(secs(2 * 60 * MIN));
    assert!(!h.closed("chrome-otp"), "the 60 s countdown is running");
    h.advance(secs(30));
    assert!(!h.closed("chrome-otp"));
    h.advance(secs(30));
    assert!(h.closed("chrome-otp"));

    let runs = h
        .sink
        .runs_for(|t| matches!(t, FiredTrigger::ItemAge { .. }));
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].steps.len(), 1);
    assert_eq!(runs[0].steps[0].summary.title, "OTP Bank – Internetbank");
}

#[test]
fn item_age_countdown_replans_and_spares_a_tab_that_left_the_bank() {
    let h = Harness::demo();
    h.advance(secs(2 * 60 * MIN));
    assert!(
        h.fake
            .navigate("chrome-otp", "https://index.hu/", "Index – Hírek")
    );
    h.engine.settle();
    h.advance(secs(61));
    assert!(!h.touched("chrome-otp"));
    assert!(
        h.sink
            .runs_for(|t| matches!(t, FiredTrigger::ItemAge { .. }))
            .is_empty()
    );
}

#[test]
fn the_same_trigger_is_debounced_for_ten_seconds_but_panic_always_fires() {
    let h = Harness::demo();
    h.signal(SignalKind::LidClosed);
    assert!(h.closed("chrome-adult"));

    let second = chrome_tab(
        "chrome-adult-2",
        "https://www.pornhub.com/categories",
        "Categories",
    );
    h.fake.add(second);
    h.advance(secs(5));
    h.signal(SignalKind::LidClosed);
    assert!(!h.closed("chrome-adult-2"), "debounced");
    h.advance(secs(6));
    h.signal(SignalKind::LidClosed);
    assert!(h.closed("chrome-adult-2"));

    h.signal(SignalKind::Panic);
    h.fake.add(otp_tab());
    h.engine.settle();
    h.signal(SignalKind::Panic);
    assert_eq!(
        h.fake
            .executed()
            .iter()
            .filter(|step| step.key == "chrome-otp")
            .count(),
        2,
        "two panics a moment apart both act"
    );
}

#[test]
fn pause_blocks_automatic_triggers_but_not_panic() {
    let h = Harness::demo();
    let status = h.engine.pause(10);
    assert!(status.paused_until_ms.is_some());

    h.share(Confidence::High, None);
    h.signal(SignalKind::LidClosed);
    assert!(h.fake.executed().is_empty());
    assert!(!h.sink.notices().contains(&Notice::ShareDetected));
    let status = h.engine.status();
    assert!(status.share.active && !status.guard_active);

    h.fake.add(chrome_tab(
        "chrome-adult-2",
        "https://www.pornhub.com/x",
        "x",
    ));
    h.engine.settle();
    assert!(!h.touched("chrome-adult-2"), "share-guard is paused too");

    h.signal(SignalKind::Panic);
    assert!(h.closed("chrome-adult"));
    h.fake.add(otp_tab());
    h.engine.settle();
    let report = h.engine.panic();
    assert!(!report.dry_run);
    assert!(report.steps.iter().any(|step| {
        step.summary.title == "OTP Bank – Internetbank"
            && step.outcome == Some(closecall_core::StepOutcome::Done)
    }));

    h.advance(secs(11 * MIN));
    let status = h.engine.status();
    assert_eq!(status.paused_until_ms, None, "the pause ran out");
    assert!(status.guard_active);
    h.fake.add(chrome_tab(
        "chrome-adult-3",
        "https://www.pornhub.com/y",
        "y",
    ));
    h.engine.settle();
    assert!(h.closed("chrome-adult-3"));
}

#[test]
fn resume_ends_a_pause_at_once() {
    let h = Harness::demo();
    h.engine.pause(60);
    let status = h.engine.resume();
    assert_eq!(status.paused_until_ms, None);
    h.signal(SignalKind::LidClosed);
    assert!(h.closed("chrome-adult"));
}
