//! A short scripted scenario on the fake platform: a share starts, a tab navigates to a bank
//! mid-share, the share ends, the machine locks and sleeps for 20 minutes. Prints the redacted
//! run summaries (trigger, counts, rule ids) and fails if the engine did not act.
//!
//! CI runs it under strace and without a network namespace (scripts/ci/no-network.sh).

#![allow(clippy::print_stdout, reason = "the summary on stdout is the point")]

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use closecall_core::Confidence;
use closecall_core::ipc::{RunReport, Status};
use closecall_core::platform::SignalKind;
use closecall_engine::{Engine, EngineOptions, EngineSettings, EngineSink, ManualClock, Notice};
use closecall_platform_fake::{FakePlatform, chrome_tab};

#[derive(Default)]
struct Printer {
    runs: Mutex<Vec<RunReport>>,
}

impl EngineSink for Printer {
    fn status_changed(&self, _status: &Status) {}

    fn run_completed(&self, report: &RunReport) {
        let mut rules: Vec<&str> = report
            .steps
            .iter()
            .flat_map(|step| step.because.iter().map(|reason| reason.rule.0.as_str()))
            .collect();
        rules.sort_unstable();
        rules.dedup();
        let count = |kind: &str| {
            report
                .steps
                .iter()
                .filter_map(|step| step.outcome.as_ref())
                .filter(|outcome| outcome_kind(outcome) == kind)
                .count()
        };
        println!(
            "run {} {}: {} steps, {} done, {} fell back, {} skipped, {} failed; rules: {}",
            report.run_id,
            trigger_label(&report.trigger),
            report.steps.len(),
            count("done"),
            count("fell_back"),
            count("skipped"),
            count("failed"),
            rules.join(", ")
        );
        self.runs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(report.clone());
    }

    fn notify(&self, notice: &Notice) {
        println!("notice: {notice:?}");
    }
}

fn outcome_kind(outcome: &closecall_core::StepOutcome) -> &'static str {
    match outcome {
        closecall_core::StepOutcome::Done => "done",
        closecall_core::StepOutcome::FellBack { .. } => "fell_back",
        closecall_core::StepOutcome::Skipped { .. } => "skipped",
        closecall_core::StepOutcome::Failed { .. } => "failed",
    }
}

fn trigger_label(trigger: &closecall_core::FiredTrigger) -> String {
    use closecall_core::FiredTrigger as F;
    match trigger {
        F::Panic => "panic".to_owned(),
        F::ScreenShare { confidence } => format!("screen_share ({confidence:?})"),
        F::DisplayMirrored => "display_mirrored".to_owned(),
        F::CallLikely => "call_likely".to_owned(),
        F::WillSleep => "will_sleep".to_owned(),
        F::LidClosed => "lid_closed".to_owned(),
        F::Locked => "locked".to_owned(),
        F::Away { away } => format!("away ({away})"),
        F::Idle { idle } => format!("idle ({idle})"),
        F::ItemAge { age, .. } => format!("item_age ({age})"),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let platform = FakePlatform::demo();
    let fake = platform.controller();
    let clock = Arc::new(ManualClock::new());
    let sink = Arc::new(Printer::default());
    let scratch = std::env::temp_dir().join(format!("closecall-scenario-{}", std::process::id()));
    let engine = Engine::with_options(
        Box::new(platform),
        EngineSettings {
            config_dir: scratch,
            home: "/Users/demo".into(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            own_app_id: "hu.futozs.closecall".to_owned(),
        },
        sink.clone(),
        EngineOptions {
            clock: clock.clone(),
            background_ticks: false,
            ..EngineOptions::default()
        },
    )?;
    engine.start_signals();
    engine.settle();

    fake.capture_started(Confidence::High, None);
    engine.settle();
    fake.add(chrome_tab("chrome-news", "https://index.hu/", "Index"));
    engine.settle();
    fake.navigate("chrome-news", "https://netbank.otpbank.hu/", "OTP Bank");
    engine.settle();
    fake.signal(SignalKind::CaptureStopped);
    engine.settle();
    clock.advance(Duration::from_secs(4));
    engine.tick();

    fake.add(chrome_tab(
        "chrome-otp-2",
        "https://netbank.otpbank.hu/",
        "OTP Bank",
    ));
    fake.add(chrome_tab(
        "chrome-adult-2",
        "https://www.pornhub.com/",
        "x",
    ));
    engine.settle();
    fake.signal(SignalKind::Locked);
    fake.signal(SignalKind::WillSleep);
    engine.settle();
    clock.sleep(Duration::from_secs(20 * 60));
    fake.signal(SignalKind::DidWake);
    engine.settle();
    fake.signal(SignalKind::Unlocked);
    engine.settle();

    println!("{}", engine.diag());

    let runs = sink
        .runs
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .len();
    let closed = fake
        .executed()
        .iter()
        .filter(|step| step.outcome == closecall_core::StepOutcome::Done)
        .count();
    if runs < 4 || closed < 10 {
        return Err(format!(
            "expected at least 4 runs and 10 closed items, got {runs} and {closed}"
        )
        .into());
    }
    Ok(())
}
