//! What each provider can do, reported honestly. `NotifyOnly` doubles as "how well we can
//! see this target", since `Capability` has no separate detection dimension.

use closecall_core::{Action, Capability, ProviderId, Support, TargetKindTag};

use crate::doc::DOC_APPS;
use crate::keys;

pub(crate) const CHROMIUM: &[(&str, &str)] = &[
    ("com.google.Chrome", "Google Chrome"),
    ("com.brave.Browser", "Brave Browser"),
    ("com.microsoft.edgemac", "Microsoft Edge"),
    ("com.vivaldi.Vivaldi", "Vivaldi"),
    ("company.thebrowser.Browser", "Arc"),
    ("net.imput.helium", "Helium"),
];
pub(crate) const ARC: &str = "company.thebrowser.Browser";
pub(crate) const SAFARI: &str = "com.apple.Safari";
pub(crate) const FIREFOX: &[&str] = &[
    "org.mozilla.firefox",
    "org.mozilla.firefoxdeveloperedition",
    "org.mozilla.nightly",
];

pub(crate) fn has_tab_provider(bundle_id: &str) -> bool {
    bundle_id == SAFARI
        || CHROMIUM.iter().any(|(id, _)| *id == bundle_id)
        || FIREFOX.contains(&bundle_id)
}

fn partial(note: &str) -> Support {
    Support::Partial {
        note_key: note.to_owned(),
    }
}

fn row(
    provider: &str,
    app: Option<&str>,
    target_kind: TargetKindTag,
    action: Action,
    support: Support,
) -> Capability {
    Capability {
        provider: ProviderId(provider.to_owned()),
        app: app.map(str::to_owned),
        target_kind,
        action,
        support,
    }
}

fn table(
    provider: &str,
    app: Option<&str>,
    kind: TargetKindTag,
    support: impl Fn(Action) -> Support,
) -> Vec<Capability> {
    [
        Action::Close,
        Action::Hide,
        Action::Minimize,
        Action::Quit,
        Action::NotifyOnly,
    ]
    .into_iter()
    .map(|action| row(provider, app, kind, action, support(action)))
    .collect()
}

pub(crate) fn chromium(provider: &str) -> Vec<Capability> {
    CHROMIUM
        .iter()
        .flat_map(|&(bundle_id, _)| {
            table(
                provider,
                Some(bundle_id),
                TargetKindTag::Tab,
                |action| match action {
                    Action::Close if bundle_id == ARC => partial(keys::NOTE_ARC_UNVERIFIED),
                    Action::Close | Action::NotifyOnly => Support::Full,
                    Action::Hide => partial(keys::NOTE_HIDES_WHOLE_APP),
                    Action::Minimize | Action::Quit => Support::Unsupported,
                },
            )
        })
        .collect()
}

pub(crate) fn safari(provider: &str) -> Vec<Capability> {
    table(
        provider,
        Some(SAFARI),
        TargetKindTag::Tab,
        |action| match action {
            Action::Close => Support::Full,
            Action::NotifyOnly => partial(keys::NOTE_SAFARI_PRIVATE_UNKNOWN),
            Action::Hide => partial(keys::NOTE_HIDES_WHOLE_APP),
            Action::Minimize | Action::Quit => Support::Unsupported,
        },
    )
}

pub(crate) fn firefox(provider: &str) -> Vec<Capability> {
    FIREFOX
        .iter()
        .flat_map(|&bundle_id| {
            table(
                provider,
                Some(bundle_id),
                TargetKindTag::Tab,
                |action| match action {
                    Action::NotifyOnly => partial(keys::NOTE_FIREFOX_SESSION_FILE),
                    Action::Hide => partial(keys::NOTE_HIDES_WHOLE_APP),
                    Action::Close | Action::Minimize | Action::Quit => Support::Unsupported,
                },
            )
        })
        .collect()
}

pub(crate) fn windows(provider: &str) -> Vec<Capability> {
    let generic = |action| match action {
        Action::Close => partial(keys::NOTE_AX_SAVE_SHEET),
        Action::Hide => partial(keys::NOTE_HIDE_OR_MINIMIZE),
        Action::Minimize | Action::NotifyOnly => Support::Full,
        Action::Quit => Support::Unsupported,
    };
    let mut out = table(provider, None, TargetKindTag::Document, generic);
    out.extend(table(provider, None, TargetKindTag::AppWindow, generic));
    for app in DOC_APPS {
        out.push(row(
            provider,
            Some(app.bundle_id),
            TargetKindTag::Document,
            Action::Close,
            Support::Full,
        ));
    }
    out
}

pub(crate) fn apps(provider: &str) -> Vec<Capability> {
    table(
        provider,
        None,
        TargetKindTag::Process,
        |action| match action {
            Action::Hide | Action::Quit | Action::NotifyOnly => Support::Full,
            Action::Close | Action::Minimize => Support::Unsupported,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support(
        caps: &[Capability],
        app: Option<&str>,
        kind: TargetKindTag,
        action: Action,
    ) -> Support {
        caps.iter()
            .find(|c| c.app.as_deref() == app && c.target_kind == kind && c.action == action)
            .map(|c| c.support.clone())
            .unwrap_or_else(|| panic!("no row for {app:?} {kind:?} {action:?}"))
    }

    #[test]
    fn firefox_cannot_close_tabs() {
        let caps = firefox("macos.firefox");
        for bundle_id in FIREFOX {
            assert_eq!(
                support(&caps, Some(bundle_id), TargetKindTag::Tab, Action::Close),
                Support::Unsupported
            );
            assert!(matches!(
                support(
                    &caps,
                    Some(bundle_id),
                    TargetKindTag::Tab,
                    Action::NotifyOnly
                ),
                Support::Partial { .. }
            ));
        }
    }

    #[test]
    fn safari_private_detection_is_partial() {
        let caps = safari("macos.safari");
        assert_eq!(
            support(&caps, Some(SAFARI), TargetKindTag::Tab, Action::NotifyOnly),
            Support::Partial {
                note_key: keys::NOTE_SAFARI_PRIVATE_UNKNOWN.to_owned()
            }
        );
        assert_eq!(
            support(&caps, Some(SAFARI), TargetKindTag::Tab, Action::Close),
            Support::Full
        );
    }

    #[test]
    fn chromium_lists_every_browser_and_flags_arc() {
        let caps = chromium("macos.chromium");
        assert_eq!(caps.len(), CHROMIUM.len() * 5);
        assert_eq!(
            support(
                &caps,
                Some("com.google.Chrome"),
                TargetKindTag::Tab,
                Action::Close
            ),
            Support::Full
        );
        assert!(matches!(
            support(&caps, Some(ARC), TargetKindTag::Tab, Action::Close),
            Support::Partial { .. }
        ));
    }

    #[test]
    fn processes_are_never_closed_or_minimized() {
        let caps = apps("macos.apps");
        assert_eq!(
            support(&caps, None, TargetKindTag::Process, Action::Close),
            Support::Unsupported
        );
        assert_eq!(
            support(&caps, None, TargetKindTag::Process, Action::Quit),
            Support::Full
        );
    }

    #[test]
    fn scriptable_document_apps_close_fully() {
        let caps = windows("macos.ax");
        assert_eq!(
            support(
                &caps,
                Some("com.microsoft.Word"),
                TargetKindTag::Document,
                Action::Close
            ),
            Support::Full
        );
        assert!(matches!(
            support(&caps, None, TargetKindTag::Document, Action::Close),
            Support::Partial { .. }
        ));
    }

    #[test]
    fn tab_browsers_are_recognized() {
        assert!(has_tab_provider("com.brave.Browser"));
        assert!(has_tab_provider(SAFARI));
        assert!(has_tab_provider("org.mozilla.firefox"));
        assert!(!has_tab_provider("com.apple.Preview"));
    }
}
