use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::platform::{Permission, RequestOutcome, SignalKind};
use closecall_core::{
    Action, Confidence, FiredTrigger, Matcher, StepOutcome, TargetKindTag, Trigger,
};
use closecall_engine::Notice;
use closecall_platform_fake::{app_window, chrome_tab};

use crate::harness::{Harness, OWN_APP, category, recommended_with, rule};

const SLACK: &str = "com.tinyspeck.slackmacgap";

#[test]
fn share_closes_sensitive_tabs_and_spares_meet_the_capturing_app_and_ourselves() {
    let share = || vec![Trigger::ScreenShare];
    let h = Harness::with_config(&recommended_with(vec![
        rule(
            "everything-google",
            share(),
            vec![TargetKindTag::Tab],
            Matcher::Domain("google.com".to_owned()),
            Action::Close,
        ),
        rule(
            "hide-close-call",
            share(),
            vec![TargetKindTag::AppWindow],
            Matcher::TitleContains("Close Call".to_owned()),
            Action::Hide,
        ),
    ]));
    h.fake
        .add(app_window("app-slack", (SLACK, "Slack"), "Slack huddle"));
    h.fake
        .add(app_window("app-own", (OWN_APP, "Close Call"), "Close Call"));
    h.engine.settle();

    h.share(Confidence::High, Some(SLACK));

    for key in [
        "chrome-otp",
        "chrome-adult",
        "chrome-gmail",
        "chrome-incognito",
        "safari-eeszt",
        "preview-invoice",
        "word-payslip",
    ] {
        assert!(h.closed(key), "{key} should be closed");
    }
    assert!(h.fake.is_concealed("app-1password"));
    assert!(h.fake.is_concealed("app-signal"));
    for key in ["chrome-meet", "app-slack", "app-own"] {
        assert!(!h.touched(key), "{key} must stay untouched");
        assert!(h.fake.contains(key));
    }

    let runs = h
        .sink
        .runs_for(|t| matches!(t, FiredTrigger::ScreenShare { .. }));
    assert_eq!(runs.len(), 1);
    assert_eq!(
        h.sink.notices(),
        vec![
            Notice::ShareDetected,
            Notice::RunSummary {
                done: 7,
                hidden: 2,
                skipped: 0
            },
        ]
    );
    let status = h.engine.status();
    assert!(status.share.active && status.guard_active);
    assert!(
        h.sink
            .statuses()
            .iter()
            .any(|s| s.guard_active && s.share.confidence == Some(Confidence::High))
    );
}

#[test]
fn guard_closes_a_tab_that_navigates_to_a_bank_mid_share() {
    let h = Harness::demo();
    h.share(Confidence::High, None);
    h.fake.add(chrome_tab(
        "chrome-news",
        "https://index.hu/",
        "Index – Hírek",
    ));
    h.engine.settle();
    assert!(!h.touched("chrome-news"));

    assert!(h.fake.navigate(
        "chrome-news",
        "https://netbank.otpbank.hu/portal/hu/Szamlak",
        "OTP Bank – Internetbank",
    ));
    h.engine.settle();
    assert!(h.closed("chrome-news"));

    h.fake.add(chrome_tab(
        "chrome-mail-2",
        "https://mail.google.com/mail/u/1/",
        "Gmail",
    ));
    h.engine.settle();
    assert!(h.closed("chrome-mail-2"));

    let guard_runs = h
        .sink
        .runs_for(|t| matches!(t, FiredTrigger::ScreenShare { .. }));
    assert_eq!(guard_runs.len(), 3, "share sweep plus two guard runs");
}

#[test]
fn guard_stops_with_the_share_after_the_debounce() {
    let h = Harness::demo();
    h.share(Confidence::High, None);
    h.signal(SignalKind::CaptureStopped);
    h.advance(Duration::from_secs(1));
    assert!(h.engine.status().guard_active, "a stop is debounced 3 s");

    h.share(Confidence::High, None);
    assert_eq!(
        h.sink
            .runs_for(|t| matches!(t, FiredTrigger::ScreenShare { .. }))
            .len(),
        1,
        "a quick restart is the same share"
    );

    h.signal(SignalKind::CaptureStopped);
    h.advance(Duration::from_secs(3));
    assert!(!h.engine.status().share.active);

    h.fake.add(chrome_tab(
        "chrome-bank-later",
        "https://netbank.otpbank.hu/",
        "OTP",
    ));
    h.engine.settle();
    assert!(!h.touched("chrome-bank-later"));
}

#[test]
fn low_confidence_capture_runs_only_call_likely_rules() {
    let h = Harness::with_config(&recommended_with(vec![rule(
        "preflight-banking",
        vec![Trigger::CallLikely],
        vec![TargetKindTag::Tab],
        category("banking"),
        Action::NotifyOnly,
    )]));
    h.share(Confidence::Low, None);

    let runs = h.sink.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].trigger, FiredTrigger::CallLikely);
    assert_eq!(runs[0].steps.len(), 1);
    assert_eq!(runs[0].steps[0].action, Action::NotifyOnly);
    assert_eq!(runs[0].steps[0].outcome, Some(StepOutcome::Done));
    assert!(h.fake.executed().is_empty(), "nothing closed or hidden");
    assert!(!h.sink.notices().contains(&Notice::ShareDetected));

    let status = h.engine.status();
    assert!(!status.share.active && !status.guard_active);
    assert!(status.share.call_likely);
    assert_eq!(status.share.confidence, Some(Confidence::Low));
}

#[test]
fn mirroring_arms_the_guard_like_a_share() {
    let h = Harness::demo();
    h.signal(SignalKind::DisplayMirrored);
    assert!(h.closed("chrome-otp"));
    assert!(h.engine.status().guard_active);
    h.fake.add(chrome_tab(
        "chrome-adult-2",
        "https://www.pornhub.com/categories",
        "Categories",
    ));
    h.engine.settle();
    assert!(h.closed("chrome-adult-2"));
}

#[test]
fn never_prompts_for_a_permission_while_sharing() {
    let h = Harness::demo();
    h.share(Confidence::High, None);
    assert_eq!(
        h.engine.request_permission(&Permission::Accessibility),
        RequestOutcome::NotWhileSharing
    );
    assert!(h.fake.permission_requests().is_empty());

    h.signal(SignalKind::CaptureStopped);
    h.advance(Duration::from_secs(4));
    assert_eq!(
        h.engine.request_permission(&Permission::Accessibility),
        RequestOutcome::Granted
    );
    assert_eq!(h.fake.permission_requests().len(), 1);
}

#[test]
fn a_batch_close_is_only_told_about_targets_closed_the_same_way() {
    let share = || vec![Trigger::ScreenShare];
    let h = Harness::with_config(&Config {
        rules: vec![
            rule(
                "webmail-notify",
                share(),
                vec![TargetKindTag::Tab],
                category("webmail"),
                Action::NotifyOnly,
            ),
            rule(
                "banking-close",
                share(),
                vec![TargetKindTag::Tab],
                category("banking"),
                Action::Close,
            ),
            rule(
                "adult-hide",
                share(),
                vec![TargetKindTag::Tab],
                category("adult"),
                Action::Hide,
            ),
        ],
        ..Config::default()
    });
    h.share(Confidence::High, None);

    assert!(h.closed("chrome-otp"));
    assert!(
        !h.touched("chrome-gmail"),
        "notify-only never reaches the provider"
    );
    let executed = h.fake.executed();
    let step = |key: &str| {
        executed
            .iter()
            .find(|step| step.key == key)
            .unwrap_or_else(|| panic!("{key} was not executed"))
    };
    assert_eq!(step("chrome-otp").planned, ["chrome-otp"]);
    assert_eq!(step("chrome-adult").planned, ["chrome-adult"]);
    assert!(
        step("chrome-otp")
            .protected
            .contains(&"chrome-meet".to_owned()),
        "the provider learns which of its targets must stay visible"
    );
}

#[test]
fn a_share_that_restarts_inside_the_debounce_still_sweeps() {
    let h = Harness::demo();
    h.share(Confidence::High, None);
    h.signal(SignalKind::CaptureStopped);
    h.advance(Duration::from_secs(4));
    assert!(!h.engine.status().share.active);

    h.fake.add(chrome_tab(
        "chrome-bank-2",
        "https://netbank.otpbank.hu/portal/hu/Kartyak",
        "OTP Bank – Kártyák",
    ));
    h.engine.settle();
    assert!(!h.touched("chrome-bank-2"));

    h.share(Confidence::High, None);
    assert!(
        h.closed("chrome-bank-2"),
        "the second share is inside the 10 s debounce but still sweeps"
    );
    assert!(h.engine.status().guard_active);
}

#[test]
fn a_pause_that_ends_mid_share_sweeps_what_is_open() {
    let h = Harness::demo();
    h.engine.pause(15);
    h.share(Confidence::High, None);
    assert!(!h.touched("chrome-otp"));
    assert!(!h.engine.status().guard_active);

    h.advance(Duration::from_secs(15 * 60 + 1));
    assert!(h.closed("chrome-otp"));
    assert!(h.engine.status().guard_active);
}

#[test]
fn resuming_mid_share_sweeps_what_is_open() {
    let h = Harness::demo();
    h.engine.pause(60);
    h.share(Confidence::High, None);
    assert!(!h.touched("chrome-otp"));

    h.engine.resume();
    h.engine.settle();
    assert!(h.closed("chrome-otp"));
}

#[test]
fn a_tab_left_open_is_reported_once_per_share() {
    let h = Harness::with_config(&Config {
        rules: vec![rule(
            "webmail-notify",
            vec![Trigger::ScreenShare],
            vec![TargetKindTag::Tab],
            category("webmail"),
            Action::NotifyOnly,
        )],
        ..Config::default()
    });
    h.share(Confidence::High, None);
    let left_open = Notice::RunSummary {
        done: 0,
        hidden: 0,
        skipped: 1,
    };
    assert_eq!(h.sink.notices(), [Notice::ShareDetected, left_open.clone()]);

    for unread in 4..7 {
        assert!(h.fake.navigate(
            "chrome-gmail",
            "https://mail.google.com/mail/u/0/#inbox",
            &format!("Beérkező levelek ({unread}) – Gmail"),
        ));
        h.engine.settle();
    }
    assert_eq!(
        h.sink
            .runs_for(|t| matches!(t, FiredTrigger::ScreenShare { .. }))
            .len(),
        4,
        "each change is still re-planned"
    );
    assert_eq!(
        h.sink.notices(),
        [Notice::ShareDetected, left_open],
        "but not announced again on the shared screen"
    );
}
