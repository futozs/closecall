use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::ipc::Preset;
use closecall_core::platform::{Permission, PermissionStatus, SourceHealth};
use closecall_core::{CategoryId, Confidence};
use closecall_engine::Notice;
use closecall_platform_fake::{CHROMIUM, PROVIDERS, SOURCES};

use crate::harness::{Harness, Setup};

#[test]
fn needs_onboarding_until_the_first_save() {
    let h = Harness::demo();
    assert!(h.engine.status().needs_onboarding);
    assert_eq!(h.engine.config(), Config::recommended());
    assert!(h.config_on_disk().is_none());

    let mut config = Config::recommended();
    config.share_min_confidence = Confidence::High;
    assert_eq!(h.engine.save_config(config.clone()).expect("saved"), vec![]);

    assert!(!h.engine.status().needs_onboarding);
    assert!(
        h.sink
            .statuses()
            .last()
            .is_some_and(|status| !status.needs_onboarding)
    );
    let on_disk = h.config_on_disk().expect("written");
    assert_eq!(Config::from_toml(&on_disk).expect("valid"), config);
    assert_eq!(h.engine.config(), config);
}

#[test]
fn an_invalid_save_returns_issues_and_leaves_the_file_alone() {
    let h = Harness::with_config(&Config::recommended());
    let before = h.config_on_disk().expect("config file");

    let mut config = Config::recommended();
    let duplicate = config.rules[0].clone();
    config.rules.push(duplicate);
    let issues = h.engine.save_config(config).expect("validated");

    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "config.error.duplicate_rule_id")
    );
    assert_eq!(h.config_on_disk().as_deref(), Some(before.as_str()));
    assert_eq!(h.engine.config(), Config::recommended());
}

#[test]
fn an_invalid_file_runs_the_recommended_preset_and_is_never_overwritten() {
    let text = "version = 1\n[[rule]]\nid = \"broken\"\n";
    let h = Harness::start(Setup {
        config_file: Some(text.to_owned()),
        ..Setup::default()
    });
    assert_eq!(h.engine.config(), Config::recommended());
    assert!(!h.engine.status().needs_onboarding);
    assert!(h.engine.diag().contains("could not be parsed"));

    h.signal(closecall_core::platform::SignalKind::LidClosed);
    h.advance(Duration::from_secs(60));
    assert_eq!(h.config_on_disk().as_deref(), Some(text));
    assert!(h.closed("chrome-adult"), "the recommended preset is live");
}

#[test]
fn reset_returns_a_preset_without_saving() {
    let h = Harness::demo();
    assert_eq!(
        h.engine.reset_config(Preset::Empty).expect("preset"),
        Config::default()
    );
    assert_eq!(
        h.engine.reset_config(Preset::Recommended).expect("preset"),
        Config::recommended()
    );
    assert!(h.config_on_disk().is_none());
}

#[test]
fn saved_user_categories_take_effect_at_once() {
    let h = Harness::demo();
    let mut config = Config::recommended();
    config.categories.push(closecall_core::CategoryDef {
        id: CategoryId("banking".to_owned()),
        domains: vec!["index.hu".to_owned()],
        ..closecall_core::CategoryDef::default()
    });
    assert!(h.engine.save_config(config).expect("saved").is_empty());
    h.fake.add(closecall_platform_fake::chrome_tab(
        "chrome-index",
        "https://index.hu/",
        "Index",
    ));
    h.engine.settle();
    h.engine.panic();
    assert!(h.closed("chrome-index"));
}

#[test]
fn capabilities_categories_and_status_describe_the_platform() {
    let h = Harness::demo();
    let report = h.engine.capabilities();
    assert_eq!(report.platform, "fake");
    for provider in PROVIDERS {
        assert!(report.capabilities.iter().any(|c| c.provider.0 == provider));
    }
    assert_eq!(report.signal_sources.len(), SOURCES.len());

    h.fake.set_health(
        "share_watcher",
        SourceHealth::Noisy {
            reason_key: "reason.watcher_noisy".to_owned(),
        },
    );
    let status = h.engine.status();
    assert_eq!(status.platform, "fake");
    assert_eq!(status.app_version, "0.1.0-test");
    assert!(
        status
            .signal_sources
            .iter()
            .any(|s| s.id == "share_watcher" && matches!(s.health, SourceHealth::Noisy { .. }))
    );
    assert!(!status.permissions.is_empty());

    let categories = h.engine.categories();
    assert!(categories.iter().any(|c| c.id.0 == "banking" && c.builtin));
}

#[test]
fn diag_is_redacted() {
    let h = Harness::demo();
    let diag = h.engine.diag();
    assert!(diag.contains("platform: fake"));
    assert!(diag.contains("fake.chromium: 5 tabs"));
    assert!(diag.contains("fake.documents: 2 documents"));
    for secret in [
        "otpbank",
        "Internetbank",
        "/Users/demo",
        "szamla",
        "Bérjegyzék",
        "YouTube",
    ] {
        assert!(!diag.contains(secret), "diag leaks {secret}:\n{diag}");
    }
}

#[test]
fn a_provider_losing_its_permission_notifies_once() {
    let h = Harness::demo();
    h.fake.deny_provider(CHROMIUM, Permission::Accessibility);
    h.fake
        .set_permission(&Permission::Accessibility, PermissionStatus::Denied);
    h.engine.settle();
    h.advance(Duration::from_secs(31));
    let missing = h
        .sink
        .notices()
        .iter()
        .filter(|n| **n == Notice::PermissionMissing)
        .count();
    assert_eq!(missing, 1);
    assert!(h.engine.diag().contains("permission missing"));
    assert!(
        h.engine
            .permissions()
            .iter()
            .any(|p| p.permission == Permission::Accessibility
                && p.status == PermissionStatus::Denied)
    );

    h.fake.allow_provider(CHROMIUM);
    h.engine.settle();
    assert!(!h.engine.diag().contains("permission missing"));
}
