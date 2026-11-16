use std::path::Path;

use closecall_core::config::Config;
use closecall_core::{
    Action, AppSelector, Dirty, Matcher, StepOutcome, TargetKindTag, Trigger, UnsavedPolicy,
};
use closecall_engine::Notice;
use closecall_platform_fake::document;

use crate::harness::{Harness, Setup, hidden_because, outcome_of, rule, skipped};

const PREVIEW: (&str, &str) = ("com.apple.Preview", "Preview");

fn run_policy(policy: UnsavedPolicy) -> (Harness, closecall_core::ipc::RunReport) {
    let mut config = Config::default();
    config.rules.push(closecall_core::Rule {
        on_unsaved: policy,
        ..rule(
            "preview-docs",
            vec![Trigger::Panic],
            vec![TargetKindTag::Document],
            Matcher::App(AppSelector::BundleId(PREVIEW.0.to_owned())),
            Action::Close,
        )
    });
    let h = Harness::start(Setup {
        demo: false,
        config_file: Some(config.to_toml()),
        ..Setup::default()
    });
    h.fake.add(document(
        "doc-dirty",
        PREVIEW,
        Some("/Users/demo/Documents/dirty.pdf"),
        "dirty.pdf",
        Dirty::Dirty,
    ));
    h.fake.add(document(
        "doc-unknown",
        PREVIEW,
        Some("/Users/demo/Documents/unknown.pdf"),
        "unknown.pdf",
        Dirty::Unknown,
    ));
    h.fake.add(document(
        "doc-untitled",
        PREVIEW,
        None,
        "Untitled",
        Dirty::Dirty,
    ));
    h.fake.add(document(
        "doc-clean",
        PREVIEW,
        Some("/Users/demo/Documents/clean.pdf"),
        "clean.pdf",
        Dirty::Clean,
    ));
    h.engine.settle();
    let report = h.engine.panic();
    (h, report)
}

fn assert_outcomes(report: &closecall_core::ipc::RunReport, expected: [(&str, StepOutcome); 4]) {
    for (title, outcome) in expected {
        assert_eq!(outcome_of(report, title), Some(&outcome), "{title}");
    }
}

fn unsaved_notice(h: &Harness) -> Option<u32> {
    h.sink.notices().iter().find_map(|notice| match notice {
        Notice::UnsavedHidden { count } => Some(*count),
        _ => None,
    })
}

#[test]
fn hide_policy_hides_every_document_with_unsaved_work() {
    let (h, report) = run_policy(UnsavedPolicy::Hide);
    assert_outcomes(
        &report,
        [
            ("dirty.pdf", hidden_because("reason.unsaved_hidden")),
            ("unknown.pdf", hidden_because("reason.unsaved_hidden")),
            ("Untitled", hidden_because("reason.unsaved_hidden")),
            ("clean.pdf", StepOutcome::Done),
        ],
    );
    assert_eq!(unsaved_notice(&h), Some(3));
    assert!(h.fake.is_concealed("doc-dirty") && h.fake.is_concealed("doc-untitled"));
}

#[test]
fn skip_and_notify_leaves_unsaved_documents_open() {
    let (h, report) = run_policy(UnsavedPolicy::SkipAndNotify);
    assert_outcomes(
        &report,
        [
            ("dirty.pdf", skipped("reason.unsaved_skipped")),
            ("unknown.pdf", skipped("reason.unsaved_skipped")),
            ("Untitled", skipped("reason.unsaved_skipped")),
            ("clean.pdf", StepOutcome::Done),
        ],
    );
    assert_eq!(unsaved_notice(&h), None);
    assert!(h.fake.contains("doc-dirty") && !h.fake.is_concealed("doc-dirty"));
    assert!(h.sink.notices().contains(&Notice::RunSummary {
        done: 1,
        hidden: 0,
        skipped: 3
    }));
}

#[test]
fn save_then_close_saves_file_backed_documents_and_hides_untitled_ones() {
    let (h, report) = run_policy(UnsavedPolicy::SaveThenClose);
    assert_outcomes(
        &report,
        [
            ("dirty.pdf", StepOutcome::Done),
            ("unknown.pdf", StepOutcome::Done),
            ("Untitled", hidden_because("reason.untitled_hidden")),
            ("clean.pdf", StepOutcome::Done),
        ],
    );
    assert_eq!(unsaved_notice(&h), Some(1));
    let executed = h.fake.executed();
    assert!(executed.iter().all(|step| {
        step.untitled_save_dir == Path::new("/Users/demo/Documents/Close Call")
            && step.planned.len() == 4
    }));
}

#[test]
fn discard_closes_file_backed_documents_but_never_discards_untitled_ones() {
    let (h, report) = run_policy(UnsavedPolicy::Discard);
    assert_outcomes(
        &report,
        [
            ("dirty.pdf", StepOutcome::Done),
            ("unknown.pdf", StepOutcome::Done),
            ("Untitled", hidden_because("reason.unsaved_hidden")),
            ("clean.pdf", StepOutcome::Done),
        ],
    );
    assert_eq!(unsaved_notice(&h), Some(1));
}
