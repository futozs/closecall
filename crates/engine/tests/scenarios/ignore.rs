use std::time::Duration;

use closecall_core::config::Config;
use closecall_core::platform::{Permission, RequestOutcome, SignalKind, SourceHealth};
use closecall_core::{
    Action, AppSelector, Confidence, DisplayId, DisplaySelector, FiredTrigger, TargetKindTag,
    Trigger,
};
use closecall_engine::Notice;
use closecall_platform_fake::{
    DISPLAYLINK, app_window, demo_running_apps, external_display, running_app,
};

use crate::harness::{Harness, category, recommended_with, rule};

const SHARE_UI: &str = "macos.share_ui";
const ZOOM: &str = "us.zoom.xos";

fn with_displaylink(h: &Harness) {
    let mut apps = demo_running_apps();
    apps.push(running_app(DISPLAYLINK, "DisplayLink Manager", 900));
    h.fake.set_running_apps(apps);
}

fn share_runs(h: &Harness) -> usize {
    h.sink
        .runs_for(|t| matches!(t, FiredTrigger::ScreenShare { .. }))
        .len()
}

#[test]
fn a_capture_held_by_an_ignored_dock_agent_does_nothing() {
    let h = Harness::demo();
    with_displaylink(&h);
    h.share(Confidence::High, None);

    assert!(h.sink.runs().is_empty(), "no share, not even call-likely");
    assert!(h.fake.executed().is_empty());
    assert!(!h.sink.notices().contains(&Notice::ShareDetected));
    let status = h.engine.status();
    assert!(!status.share.active && !status.guard_active && !status.share.call_likely);
    assert_eq!(
        status.share.ignored_capture.as_deref(),
        Some("DisplayLink Manager")
    );
    let watcher = status
        .signal_sources
        .iter()
        .find(|source| source.id == "share_watcher")
        .expect("fake share watcher");
    assert_eq!(
        watcher.health,
        SourceHealth::Noisy {
            reason_key: "reason.capture_ignored".to_owned()
        }
    );
    assert!(
        h.engine
            .diag()
            .contains("capture put down to an ignored app: yes")
    );
}

#[test]
fn the_share_ui_source_still_starts_a_share_while_the_dock_holds_the_watcher() {
    let h = Harness::demo();
    with_displaylink(&h);
    h.fake
        .add(app_window("app-zoom", (ZOOM, "zoom.us"), "Zoom Meeting"));
    h.share(Confidence::High, None);
    assert!(h.sink.runs().is_empty());

    assert!(h.fake.capture_from(
        SHARE_UI,
        SignalKind::CaptureStarted,
        Confidence::Medium,
        Some(ZOOM)
    ));
    h.engine.settle();

    assert_eq!(share_runs(&h), 1);
    assert!(h.closed("chrome-otp"));
    assert!(h.fake.is_concealed("app-1password"));
    assert!(!h.touched("app-zoom"), "the capturing app is protected");
    assert!(h.sink.notices().contains(&Notice::ShareDetected));
    let status = h.engine.status();
    assert!(status.share.active && status.guard_active);
    assert_eq!(status.share.confidence, Some(Confidence::Medium));

    assert!(h.fake.capture_from(
        SHARE_UI,
        SignalKind::CaptureStopped,
        Confidence::Medium,
        None
    ));
    h.engine.settle();
    h.advance(Duration::from_secs(4));
    let status = h.engine.status();
    assert!(!status.share.active && !status.guard_active);
    assert_eq!(
        status.share.ignored_capture.as_deref(),
        Some("DisplayLink Manager"),
        "the dock still holds the watcher"
    );
}

#[test]
fn without_the_dock_agent_running_an_unattributed_capture_is_a_share() {
    let h = Harness::demo();
    h.share(Confidence::High, None);
    assert_eq!(share_runs(&h), 1);
    assert_eq!(h.engine.status().share.ignored_capture, None);
}

#[test]
fn a_capture_named_after_an_ignored_app_is_ignored_even_when_others_run() {
    let mut config = Config::recommended();
    config
        .ignore
        .capture_apps
        .push(AppSelector::Name("OBS".to_owned()));
    let h = Harness::with_config(&config);
    h.fake
        .set_running_apps(vec![running_app("com.obsproject.obs-studio", "OBS", 77)]);
    h.share(Confidence::High, Some("com.obsproject.obs-studio"));
    assert!(h.sink.runs().is_empty());
    assert_eq!(
        h.engine.status().share.ignored_capture.as_deref(),
        Some("OBS")
    );
}

#[test]
fn taking_the_dock_off_the_ignore_list_turns_its_capture_into_a_share() {
    let h = Harness::demo();
    with_displaylink(&h);
    h.share(Confidence::High, None);
    assert_eq!(share_runs(&h), 0);

    let mut config = h.engine.config();
    config.ignore.capture_apps.clear();
    assert!(h.engine.save_config(config).unwrap().is_empty());
    h.engine.settle();

    assert_eq!(share_runs(&h), 1);
    assert!(h.closed("chrome-otp"));
    assert_eq!(h.engine.status().share.ignored_capture, None);
}

fn lg(id: u32) -> closecall_core::DisplayInfo {
    external_display(id, "LG HDR 4K", 7789, 30471)
}

fn projector() -> closecall_core::DisplayInfo {
    external_display(9, "EPSON PJ", 4675, 2)
}

fn display_connected_config() -> Config {
    let mut config = recommended_with(vec![rule(
        "projector-close-banking",
        vec![Trigger::DisplayConnected],
        vec![TargetKindTag::Tab],
        category("banking"),
        Action::Close,
    )]);
    config.ignore.displays = vec![
        DisplaySelector::Name("lg hdr".to_owned()),
        DisplaySelector::Id(DisplayId {
            vendor: 1552,
            model: 41006,
            serial: None,
        }),
    ];
    config
}

#[test]
fn an_ignored_display_never_fires_mirrored_or_connected() {
    let h = Harness::with_config(&display_connected_config());
    for kind in [SignalKind::DisplayConnected, SignalKind::DisplayMirrored] {
        assert!(h.fake.display_signal(kind, lg(4)));
        assert!(h.fake.display_signal(kind, lg(5)));
    }
    h.engine.settle();

    assert!(h.sink.runs().is_empty());
    assert!(h.fake.executed().is_empty());
    let status = h.engine.status();
    assert!(!status.share.mirrored_display && !status.share.external_display);
    assert!(!status.guard_active);
    let lgs = h
        .engine
        .displays()
        .into_iter()
        .filter(|display| display.name == "LG HDR 4K")
        .count();
    assert_eq!(lgs, 2);
}

#[test]
fn a_display_that_is_not_ignored_runs_display_connected_rules_and_arms_the_guard() {
    let h = Harness::with_config(&display_connected_config());
    assert!(
        h.fake
            .display_signal(SignalKind::DisplayConnected, projector())
    );
    h.engine.settle();

    let runs = h
        .sink
        .runs_for(|t| matches!(t, FiredTrigger::DisplayConnected));
    assert_eq!(runs.len(), 1);
    assert!(h.closed("chrome-otp"));
    assert!(!h.touched("chrome-gmail"), "only the banking rule is armed");
    let status = h.engine.status();
    assert!(status.share.external_display && status.guard_active);
    assert_eq!(
        h.engine.request_permission(&Permission::Accessibility),
        RequestOutcome::NotWhileSharing,
        "a projector counts as sharing for permission prompts"
    );

    assert!(
        h.fake
            .display_signal(SignalKind::DisplayDisconnected, projector())
    );
    h.engine.settle();
    h.advance(Duration::from_secs(4));
    let status = h.engine.status();
    assert!(!status.share.external_display && !status.guard_active);
}

#[test]
fn the_recommended_preset_keeps_mirroring_armed_but_ignores_a_plain_connect() {
    let h = Harness::demo();
    assert!(
        h.fake
            .display_signal(SignalKind::DisplayConnected, projector())
    );
    h.engine.settle();
    assert!(h.fake.executed().is_empty());
    let status = h.engine.status();
    assert!(!status.share.external_display && !status.guard_active);

    assert!(
        h.fake
            .display_signal(SignalKind::DisplayMirrored, projector())
    );
    h.engine.settle();
    assert!(h.closed("chrome-otp"));
    assert!(h.engine.status().share.mirrored_display);
}

#[test]
fn running_apps_and_displays_come_from_the_platform() {
    let h = Harness::demo();
    with_displaylink(&h);
    assert!(
        h.engine
            .running_apps()
            .iter()
            .any(|app| app.id == DISPLAYLINK)
    );
    assert!(h.engine.displays().iter().any(|display| display.builtin));
}
