use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::{Action, FiredTrigger, Matcher, StepOutcome, TargetKindTag, Trigger};
use closecall_engine::{EngineError, EngineOptions, Notice};
use closecall_platform_fake::{DOCUMENTS, SAFARI};

use crate::harness::{Harness, Setup, outcome_of, rule, skipped};

fn timed_out() -> StepOutcome {
    StepOutcome::Failed {
        error: "reason.timeout".to_owned(),
    }
}

#[test]
fn a_hung_provider_fails_its_steps_and_the_others_carry_on() {
    let h = Harness::start(Setup {
        options: EngineOptions {
            document_deadline: Duration::from_millis(60),
            ..EngineOptions::default()
        },
        ..Setup::default()
    });
    h.fake.hang(DOCUMENTS);
    let report = h.engine.panic();
    h.fake.release(DOCUMENTS);

    assert_eq!(
        outcome_of(&report, "szamla-2026-10.pdf"),
        Some(&timed_out())
    );
    assert_eq!(
        outcome_of(&report, "Bérjegyzék 2026. október.docx"),
        Some(&timed_out())
    );
    assert_eq!(
        outcome_of(&report, "OTP Bank – Internetbank"),
        Some(&StepOutcome::Done)
    );
    assert_eq!(outcome_of(&report, "1Password"), Some(&StepOutcome::Done));
    assert!(h.closed("chrome-otp") && h.fake.is_concealed("app-signal"));
    assert!(h.sink.notices().contains(&Notice::RunSummary {
        done: 5,
        hidden: 2,
        skipped: 2
    }));

    h.engine.settle();
    let again = h.engine.panic();
    assert_eq!(
        outcome_of(&again, "szamla-2026-10.pdf"),
        Some(&StepOutcome::Done),
        "the provider answers again once it is no longer hung"
    );
}

#[test]
fn reverify_skips_targets_that_are_gone_or_no_longer_match() {
    let h = Harness::demo();
    assert!(h.fake.remove_silently("chrome-adult"));
    assert!(
        h.fake
            .navigate_silently("chrome-gmail", "https://index.hu/", "Index – Hírek")
    );
    let report = h.engine.panic();

    assert_eq!(
        outcome_of(&report, "Pornhub"),
        Some(&skipped("reason.gone"))
    );
    assert_eq!(
        outcome_of(&report, "Index – Hírek"),
        Some(&skipped("reason.changed_before_action"))
    );
    assert!(!h.touched("chrome-gmail") && h.fake.contains("chrome-gmail"));
    assert!(h.closed("chrome-otp"));
    assert!(h.sink.notices().contains(&Notice::RunSummary {
        done: 5,
        hidden: 2,
        skipped: 1
    }));
}

#[test]
fn preview_plans_without_touching_anything() {
    let h = Harness::demo();
    let report = h.engine.preview(FiredTrigger::Panic);
    assert!(report.dry_run && !report.restorable);
    assert_eq!(report.steps.len(), 9);
    assert!(report.steps.iter().all(|step| step.outcome.is_none()));
    assert!(h.fake.executed().is_empty());
    assert!(h.sink.runs().is_empty() && h.sink.notices().is_empty());
    assert!(h.engine.activity().is_empty());
}

#[test]
fn restore_undoes_a_run_in_reverse_order_except_private_tabs() {
    let h = Harness::demo();
    let report = h.engine.panic();
    assert!(report.restorable);
    assert!(h.engine.activity()[0].restorable);

    let restored = h.engine.restore(report.run_id).expect("restorable");
    assert!(!restored.restorable);
    assert_eq!(restored.restore_of, Some(report.run_id));
    assert_ne!(restored.run_id, report.run_id);
    assert_eq!(
        outcome_of(&restored, "ajándék – YouTube"),
        Some(&skipped("reason.private_not_restorable"))
    );
    assert_eq!(
        outcome_of(&restored, "EESZT – Lakossági portál"),
        Some(&skipped("reason.private_not_restorable")),
        "Safari cannot tell private tabs apart, so its tabs are never reopened"
    );
    for title in ["OTP Bank – Internetbank", "szamla-2026-10.pdf", "1Password"] {
        assert_eq!(
            outcome_of(&restored, title),
            Some(&StepOutcome::Done),
            "{title}"
        );
    }

    let executed: Vec<String> = h
        .fake
        .executed()
        .into_iter()
        .filter(|step| step.provider != SAFARI && step.key != "chrome-incognito")
        .map(|step| step.key)
        .collect();
    let mut reverse: Vec<String> = h.fake.restored().into_iter().map(|step| step.key).collect();
    reverse.reverse();
    assert_eq!(reverse, executed);
    assert!(h.fake.contains("chrome-otp") && !h.fake.is_concealed("app-1password"));

    assert!(matches!(
        h.engine.restore(report.run_id),
        Err(EngineError::NotRestorable(_))
    ));
    assert!(!h.engine.activity().iter().any(|run| run.restorable));
}

#[test]
fn restore_is_refused_after_the_window() {
    let h = Harness::demo();
    let report = h.engine.panic();
    h.advance(Duration::from_secs(2 * 3600 + 1));
    assert!(!h.engine.activity()[0].restorable);
    assert!(matches!(
        h.engine.restore(report.run_id),
        Err(EngineError::NotRestorable(id)) if id == report.run_id
    ));
    assert!(h.fake.restored().is_empty());
}

#[test]
fn activity_forgets_titles_urls_and_paths_after_the_restore_window() {
    let h = Harness::demo();
    h.engine.panic();
    let fresh = &h.engine.activity()[0];
    assert!(fresh.steps.iter().any(|step| step.summary.url.is_some()));

    h.advance(Duration::from_secs(2 * 3600 + 1));
    let old = &h.engine.activity()[0];
    assert_eq!(old.steps.len(), fresh.steps.len());
    for step in &old.steps {
        assert!(step.summary.title.is_empty() && step.summary.url.is_none());
        assert!(step.summary.path.is_none() && step.because.is_empty());
        assert!(!step.summary.app_name.is_empty());
    }
}

#[test]
fn a_run_that_only_closed_private_tabs_is_not_restorable() {
    let mut config = Config::default();
    config.rules.push(rule(
        "panic-private",
        vec![Trigger::Panic],
        vec![TargetKindTag::Tab],
        Matcher::Private,
        Action::Close,
    ));
    let h = Harness::with_config(&config);
    let report = h.engine.panic();
    assert_eq!(report.steps.len(), 1);
    assert!(!report.restorable);
    assert!(h.engine.restore(report.run_id).is_err());
    assert!(h.fake.restored().is_empty());
}

#[test]
fn activity_is_newest_first_and_capped() {
    let h = Harness::demo();
    let mut last = 0;
    for _ in 0..55 {
        h.fake.add(closecall_platform_fake::chrome_tab(
            "again",
            "https://www.pornhub.com/",
            "again",
        ));
        h.engine.settle();
        last = h.engine.panic().run_id;
    }
    let activity = h.engine.activity();
    assert_eq!(activity.len(), 50);
    assert_eq!(activity[0].run_id, last);
    assert!(
        activity
            .windows(2)
            .all(|pair| pair[0].run_id > pair[1].run_id)
    );
}
