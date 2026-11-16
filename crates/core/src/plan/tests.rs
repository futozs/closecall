use proptest::prelude::*;
use proptest::sample::select;

use super::*;
use crate::config::Config;
use crate::matcher::app_matches;
use crate::testing::{
    PROVIDER, StaticCategoryIndex, TargetExt, all_supported, app_window, document, process, tab,
};
use crate::{
    CategoryDef, CategoryId, Confidence, Dirty, DocKind, Dur, ProviderId, RuleId, TargetId,
};

const HOME: &str = "/Users/anna";

fn category_def(id: &str) -> CategoryDef {
    CategoryDef {
        id: CategoryId(id.to_owned()),
        ..CategoryDef::default()
    }
}

fn index() -> StaticCategoryIndex {
    let bundle = |id: &str| AppSelector::BundleId(id.to_owned());
    StaticCategoryIndex::new()
        .with_category(CategoryDef {
            lists: vec!["adult".to_owned()],
            ..category_def("adult")
        })
        .with_category(CategoryDef {
            domains: vec!["otpbank.hu".to_owned()],
            title_keywords: vec!["Internetbank".to_owned()],
            ..category_def("banking")
        })
        .with_category(CategoryDef {
            domains: vec!["mail.google.com".to_owned(), "freemail.hu".to_owned()],
            ..category_def("webmail")
        })
        .with_category(CategoryDef {
            domains: vec!["eeszt.gov.hu".to_owned()],
            ..category_def("medical")
        })
        .with_category(CategoryDef {
            apps: vec![bundle("com.1password.1password")],
            ..category_def("password_managers")
        })
        .with_category(CategoryDef {
            apps: vec![bundle("com.tinyspeck.slackmacgap")],
            ..category_def("messengers")
        })
        .with_category(CategoryDef {
            title_keywords: vec!["számla".to_owned(), "invoice".to_owned()],
            file_globs: vec!["**/*szamla*".to_owned()],
            ..category_def("invoices")
        })
        .with_category(CategoryDef {
            domains: vec!["meet.google.com".to_owned(), "zoom.us".to_owned()],
            apps: vec![bundle("us.zoom.xos")],
            ..category_def("meeting")
        })
        .with_list("adult", &["pornhub.com"])
}

struct Fixture {
    index: StaticCategoryIndex,
    protect: Protect,
    protect_apps: Vec<AppSelector>,
    capabilities: Vec<Capability>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            index: index(),
            protect: Protect::default(),
            protect_apps: Vec::new(),
            capabilities: all_supported(PROVIDER),
        }
    }

    fn ctx(&self) -> PlanContext<'_> {
        PlanContext {
            categories: &self.index,
            protect: &self.protect,
            protect_apps: &self.protect_apps,
            capabilities: &self.capabilities,
            now_ms: 0,
            home: HOME,
        }
    }

    fn plan(&self, fired: &FiredTrigger, snapshot: &[Target], rules: &[Rule]) -> Plan {
        plan(fired, snapshot, rules, &self.ctx())
    }
}

fn rule(id: &str, when: Vec<Trigger>, targets: Vec<TargetKindTag>, matcher: Matcher) -> Rule {
    Rule {
        id: RuleId(id.to_owned()),
        name: None,
        enabled: true,
        when,
        targets,
        matcher,
        except: None,
        action: Action::Close,
        on_unsaved: UnsavedPolicy::SaveThenClose,
        confirm: Confirm::Never,
    }
}

fn panic_rule(id: &str, matcher: Matcher) -> Rule {
    rule(
        id,
        vec![Trigger::Panic],
        vec![
            TargetKindTag::Tab,
            TargetKindTag::Document,
            TargetKindTag::AppWindow,
            TargetKindTag::Process,
        ],
        matcher,
    )
}

fn category(id: &str) -> Matcher {
    Matcher::Category(CategoryId(id.to_owned()))
}

fn domain(d: &str) -> Matcher {
    Matcher::Domain(d.to_owned())
}

fn targets_of(plan: &Plan) -> Vec<u64> {
    plan.steps.iter().map(|step| step.target.0).collect()
}

#[test]
fn exact_triggers_arm_only_their_own_kind() {
    let pairs = [
        (Trigger::Panic, FiredTrigger::Panic),
        (
            Trigger::ScreenShare,
            FiredTrigger::ScreenShare {
                confidence: Confidence::Low,
            },
        ),
        (Trigger::DisplayMirrored, FiredTrigger::DisplayMirrored),
        (Trigger::CallLikely, FiredTrigger::CallLikely),
        (Trigger::WillSleep, FiredTrigger::WillSleep),
        (Trigger::LidClosed, FiredTrigger::LidClosed),
        (Trigger::Locked, FiredTrigger::Locked),
    ];
    for (i, (trigger, _)) in pairs.iter().enumerate() {
        let r = rule(
            "r",
            vec![trigger.clone()],
            vec![TargetKindTag::Tab],
            Matcher::Private,
        );
        for (j, (_, fired)) in pairs.iter().enumerate() {
            assert_eq!(armed(&r, fired), i == j, "{trigger:?} vs {fired:?}");
        }
    }
}

#[test]
fn duration_triggers_arm_at_or_above_their_threshold() {
    let away = rule(
        "away",
        vec![Trigger::AwayLongerThan(Dur::from_mins(15))],
        vec![TargetKindTag::Tab],
        Matcher::Private,
    );
    assert!(!armed(
        &away,
        &FiredTrigger::Away {
            away: Dur::from_mins(14)
        }
    ));
    assert!(armed(
        &away,
        &FiredTrigger::Away {
            away: Dur::from_mins(15)
        }
    ));
    assert!(armed(
        &away,
        &FiredTrigger::Away {
            away: Dur::from_hours(3)
        }
    ));
    assert!(!armed(
        &away,
        &FiredTrigger::Idle {
            idle: Dur::from_hours(3)
        }
    ));

    let idle = rule(
        "idle",
        vec![Trigger::IdleLongerThan(Dur::from_mins(5))],
        vec![TargetKindTag::Tab],
        Matcher::Private,
    );
    assert!(armed(
        &idle,
        &FiredTrigger::Idle {
            idle: Dur::from_mins(5)
        }
    ));
    assert!(!armed(
        &idle,
        &FiredTrigger::Idle {
            idle: Dur::from_secs(299)
        }
    ));

    let age = rule(
        "age",
        vec![Trigger::ItemOpenLongerThan(Dur::from_hours(2))],
        vec![TargetKindTag::Tab],
        Matcher::Private,
    );
    let fired = |age| FiredTrigger::ItemAge {
        target: TargetId(1),
        age,
    };
    assert!(armed(&age, &fired(Dur::from_hours(2))));
    assert!(!armed(&age, &fired(Dur::from_mins(119))));
}

#[test]
fn disabled_rules_never_arm_and_any_trigger_in_when_arms() {
    let mut r = rule(
        "r",
        vec![Trigger::Locked, Trigger::Panic],
        vec![TargetKindTag::Tab],
        Matcher::Private,
    );
    assert!(armed(&r, &FiredTrigger::Panic));
    assert!(armed(&r, &FiredTrigger::Locked));
    r.enabled = false;
    assert!(!armed(&r, &FiredTrigger::Panic));
}

#[test]
fn only_armed_rules_select_targets() {
    let f = Fixture::new();
    let bank = tab("https://otpbank.hu/", "").with_id(1);
    let locked = rule(
        "locked",
        vec![Trigger::Locked],
        vec![TargetKindTag::Tab],
        category("banking"),
    );
    assert!(
        f.plan(
            &FiredTrigger::Panic,
            std::slice::from_ref(&bank),
            std::slice::from_ref(&locked)
        )
        .steps
        .is_empty()
    );
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Locked, &[bank], &[locked])),
        [1]
    );
}

#[test]
fn selection_respects_target_kinds() {
    let f = Fixture::new();
    let snapshot = [
        tab("https://a.hu/", "Számla").with_id(1),
        document(Some("/x/szamla.pdf"), "Számla", Dirty::Clean).with_id(2),
        app_window("com.apple.Mail", "Mail", "Számla").with_id(3),
    ];
    let docs_only = rule(
        "docs",
        vec![Trigger::Panic],
        vec![TargetKindTag::Document],
        category("invoices"),
    );
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Panic, &snapshot, &[docs_only])),
        [2]
    );
}

#[test]
fn except_removes_matches() {
    let f = Fixture::new();
    let snapshot = [
        tab("https://mail.google.com/mail/u/0/", "Work inbox").with_id(1),
        tab("https://mail.google.com/mail/u/1/", "Private inbox").with_id(2),
    ];
    let mut webmail = panic_rule("webmail", category("webmail"));
    webmail.except = Some(Matcher::UrlGlob(
        "https://mail.google.com/mail/u/0/*".to_owned(),
    ));
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Panic, &snapshot, &[webmail.clone()])),
        [2]
    );

    webmail.except = Some(Matcher::Not(Box::new(Matcher::TitleContains(
        "private".to_owned(),
    ))));
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Panic, &snapshot, &[webmail])),
        [2]
    );
}

#[test]
fn protection_always_wins() {
    let mut f = Fixture::new();
    f.protect.domains = vec!["example.com".to_owned()];
    f.protect.apps = vec![AppSelector::Name("signal".to_owned())];
    f.protect.audible_tabs = true;
    f.protect.pinned_tabs = true;
    f.protect_apps = vec![AppSelector::BundleId(
        "com.obsproject.obs-studio".to_owned(),
    )];
    let snapshot = [
        tab("https://meet.google.com/abc", "Meet")
            .private()
            .with_id(1),
        tab("https://docs.example.com/", "Docs")
            .private()
            .with_id(2),
        tab("https://a.hu/", "Music").private().audible().with_id(3),
        tab("https://b.hu/", "Pinned").private().pinned().with_id(4),
        app_window("org.whispersystems.signal-desktop", "Signal", "Chats").with_id(5),
        app_window("com.obsproject.obs-studio", "OBS", "Studio").with_id(6),
        tab("https://c.hu/", "Other").private().with_id(7),
        app_window("com.apple.TextEdit", "TextEdit", "Notes").with_id(8),
    ];
    let everything = panic_rule(
        "everything",
        Matcher::Any(vec![
            Matcher::Private,
            Matcher::App(AppSelector::Exe("x".to_owned())),
            Matcher::TitleContains("s".to_owned()),
        ]),
    );
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Panic, &snapshot, &[everything])),
        [7, 8]
    );
}

#[test]
fn audible_and_pinned_tabs_are_fair_game_unless_protected() {
    let f = Fixture::new();
    let snapshot = [
        tab("https://a.hu/", "").private().audible().with_id(1),
        tab("https://b.hu/", "").private().pinned().with_id(2),
    ];
    let private = panic_rule("private", Matcher::Private);
    assert_eq!(
        targets_of(&f.plan(&FiredTrigger::Panic, &snapshot, &[private])),
        [1, 2]
    );
}

#[test]
fn item_age_only_considers_its_own_target() {
    let f = Fixture::new();
    let snapshot = [
        tab("https://otpbank.hu/a", "").with_id(1),
        tab("https://otpbank.hu/b", "").with_id(2),
    ];
    let age = rule(
        "age",
        vec![Trigger::ItemOpenLongerThan(Dur::from_hours(2))],
        vec![TargetKindTag::Tab],
        category("banking"),
    );
    let fired = FiredTrigger::ItemAge {
        target: TargetId(2),
        age: Dur::from_hours(3),
    };
    let rules = [age];
    assert_eq!(targets_of(&f.plan(&fired, &snapshot, &rules)), [2]);
    let gone = FiredTrigger::ItemAge {
        target: TargetId(9),
        age: Dur::from_hours(3),
    };
    assert!(f.plan(&gone, &snapshot, &rules).steps.is_empty());
}

#[test]
fn merge_takes_the_strongest_action_and_safest_unsaved_policy() {
    let f = Fixture::new();
    let doc = document(Some("/x/szamla.pdf"), "Számla", Dirty::Dirty).with_id(1);
    let mut hide = panic_rule("a-hide", category("invoices"));
    hide.action = Action::Hide;
    hide.on_unsaved = UnsavedPolicy::Discard;
    let mut close = panic_rule("b-close", Matcher::DocKind(DocKind::Pdf));
    close.action = Action::Close;
    close.on_unsaved = UnsavedPolicy::SkipAndNotify;
    let mut notify = panic_rule("c-notify", Matcher::TitleContains("szamla".to_owned()));
    notify.action = Action::NotifyOnly;
    notify.on_unsaved = UnsavedPolicy::SaveThenClose;

    let plan = f.plan(&FiredTrigger::Panic, &[doc], &[notify, close, hide]);
    let [step] = plan.steps.as_slice() else {
        panic!("one step expected: {plan:?}");
    };
    assert_eq!(step.action, Action::Close);
    assert_eq!(step.on_unsaved, UnsavedPolicy::SkipAndNotify);
    assert_eq!(step.phase, Phase::CloseDocuments);
    assert_eq!(step.fell_back_from, None);
    let rules: Vec<&str> = step
        .because
        .iter()
        .map(|reason| reason.rule.0.as_str())
        .collect();
    assert_eq!(rules, ["a-hide", "b-close", "c-notify"]);
    assert_eq!(
        step.because[1].evidence,
        Evidence::DocKind {
            doc_kind: DocKind::Pdf
        }
    );
}

#[test]
fn unsaved_policy_order_is_hide_skip_save_discard() {
    let f = Fixture::new();
    let doc = document(Some("/x/a.pdf"), "a", Dirty::Dirty).with_id(1);
    let with_policy = |id: &str, policy| Rule {
        on_unsaved: policy,
        ..panic_rule(id, Matcher::DocKind(DocKind::Pdf))
    };
    let merged = |policies: &[UnsavedPolicy]| {
        let rules: Vec<Rule> = policies
            .iter()
            .enumerate()
            .map(|(i, policy)| with_policy(&format!("r{i}"), *policy))
            .collect();
        f.plan(&FiredTrigger::Panic, std::slice::from_ref(&doc), &rules)
            .steps[0]
            .on_unsaved
    };
    use UnsavedPolicy::*;
    assert_eq!(merged(&[Discard, Hide]), Hide);
    assert_eq!(merged(&[SkipAndNotify, SaveThenClose]), SkipAndNotify);
    assert_eq!(merged(&[Discard, SaveThenClose]), SaveThenClose);
    assert_eq!(merged(&[Discard]), Discard);
}

#[test]
fn merge_keeps_the_longest_countdown() {
    let f = Fixture::new();
    let bank = tab("https://otpbank.hu/", "").with_id(1);
    let idle = |id: &str, confirm| Rule {
        confirm,
        ..rule(
            id,
            vec![Trigger::IdleLongerThan(Dur::from_mins(10))],
            vec![TargetKindTag::Tab],
            category("banking"),
        )
    };
    let fired = FiredTrigger::Idle {
        idle: Dur::from_mins(20),
    };
    let rules = [
        idle("a", Confirm::Countdown(Dur::from_secs(30))),
        idle("b", Confirm::Never),
        idle("c", Confirm::Countdown(Dur::from_secs(90))),
    ];
    let plan = f.plan(&fired, std::slice::from_ref(&bank), &rules);
    assert_eq!(
        plan.steps[0].confirm,
        Confirm::Countdown(Dur::from_secs(90))
    );
    let plan = f.plan(&fired, &[bank], &rules[1..2]);
    assert_eq!(plan.steps[0].confirm, Confirm::Never);
}

fn capability(
    kind: TargetKindTag,
    action: Action,
    app: Option<&str>,
    support: Support,
) -> Capability {
    Capability {
        provider: ProviderId(PROVIDER.to_owned()),
        app: app.map(str::to_owned),
        target_kind: kind,
        action,
        support,
    }
}

fn single_step(f: &Fixture, target: Target, action: Action) -> Step {
    let r = Rule {
        action,
        ..panic_rule(
            "r",
            Matcher::Any(vec![
                Matcher::Private,
                Matcher::App(AppSelector::Name("Firefox".to_owned())),
            ]),
        )
    };
    let plan = f.plan(&FiredTrigger::Panic, &[target], &[r]);
    assert_eq!(plan.steps.len(), 1, "{plan:?}");
    plan.steps.into_iter().next().unwrap()
}

#[test]
fn unsupported_actions_fall_back_toward_less_destructive() {
    let mut f = Fixture::new();
    f.capabilities = vec![
        capability(TargetKindTag::Tab, Action::Hide, None, Support::Full),
        capability(
            TargetKindTag::Tab,
            Action::Close,
            None,
            Support::Unsupported,
        ),
    ];
    let step = single_step(
        &f,
        tab("https://a.hu/", "").private().with_id(1),
        Action::Quit,
    );
    assert_eq!(step.action, Action::Hide);
    assert_eq!(step.fell_back_from, Some(Action::Quit));
    assert_eq!(step.phase, Phase::Conceal);
}

#[test]
fn fallback_never_escalates() {
    let mut f = Fixture::new();
    f.capabilities = vec![
        capability(TargetKindTag::Tab, Action::Close, None, Support::Full),
        capability(TargetKindTag::Tab, Action::Hide, None, Support::Full),
    ];
    let step = single_step(
        &f,
        tab("https://a.hu/", "").private().with_id(1),
        Action::Minimize,
    );
    assert_eq!(step.action, Action::NotifyOnly);
    assert_eq!(step.fell_back_from, Some(Action::Minimize));
    assert_eq!(step.phase, Phase::Notify);
}

#[test]
fn notify_only_needs_no_capability() {
    let mut f = Fixture::new();
    f.capabilities = Vec::new();
    let step = single_step(
        &f,
        tab("https://a.hu/", "").private().with_id(1),
        Action::NotifyOnly,
    );
    assert_eq!(step.action, Action::NotifyOnly);
    assert_eq!(step.fell_back_from, None);
}

#[test]
fn partial_support_counts_and_other_providers_do_not() {
    let mut f = Fixture::new();
    f.capabilities = vec![
        Capability {
            provider: ProviderId("other".to_owned()),
            ..capability(TargetKindTag::Tab, Action::Close, None, Support::Full)
        },
        capability(
            TargetKindTag::Tab,
            Action::Hide,
            None,
            Support::Partial {
                note_key: "capability.note.whole_app".to_owned(),
            },
        ),
    ];
    let step = single_step(
        &f,
        tab("https://a.hu/", "").private().with_id(1),
        Action::Close,
    );
    assert_eq!(step.action, Action::Hide);
}

#[test]
fn app_specific_capability_rows_win_over_generic_ones() {
    let mut f = Fixture::new();
    f.capabilities = vec![
        capability(TargetKindTag::Tab, Action::Close, None, Support::Full),
        capability(
            TargetKindTag::Tab,
            Action::Close,
            Some("org.mozilla.firefox"),
            Support::Unsupported,
        ),
        capability(
            TargetKindTag::Tab,
            Action::Hide,
            Some("org.mozilla.firefox"),
            Support::Full,
        ),
    ];
    let firefox = tab("https://a.hu/", "")
        .with_app("org.mozilla.firefox", "Firefox")
        .with_id(1);
    let step = single_step(&f, firefox, Action::Close);
    assert_eq!(
        (step.action, step.fell_back_from),
        (Action::Hide, Some(Action::Close))
    );

    let chrome = tab("https://a.hu/", "").private().with_id(2);
    let step = single_step(&f, chrome, Action::Close);
    assert_eq!((step.action, step.fell_back_from), (Action::Close, None));

    f.capabilities = vec![
        capability(
            TargetKindTag::Tab,
            Action::Close,
            None,
            Support::Unsupported,
        ),
        capability(
            TargetKindTag::Tab,
            Action::Close,
            Some("org.mozilla.firefox"),
            Support::Full,
        ),
    ];
    let firefox = tab("https://a.hu/", "")
        .with_app("org.mozilla.firefox", "Firefox")
        .with_id(1);
    assert_eq!(
        single_step(&f, firefox, Action::Close).action,
        Action::Close
    );
}

#[test]
fn steps_are_ordered_by_phase_then_target() {
    let f = Fixture::new();
    let snapshot = [
        document(Some("/x/szamla.pdf"), "szamla", Dirty::Clean).with_id(1),
        tab("https://otpbank.hu/", "").with_id(2),
        app_window("com.1password.1password", "1Password", "").with_id(9),
        tab("https://mail.google.com/", "").with_id(3),
        process(
            "com.tinyspeck.slackmacgap",
            "Slack",
            "/Applications/Slack.app",
        )
        .with_id(4),
    ];
    let plan = f.plan(
        &FiredTrigger::Panic,
        &snapshot,
        &Config::recommended().rules,
    );
    let order: Vec<(Phase, u64)> = plan.steps.iter().map(|s| (s.phase, s.target.0)).collect();
    assert_eq!(
        order,
        [
            (Phase::Conceal, 4),
            (Phase::Conceal, 9),
            (Phase::CloseTabs, 2),
            (Phase::CloseTabs, 3),
            (Phase::CloseDocuments, 1),
        ]
    );
}

#[test]
fn close_on_windows_and_processes_is_a_document_phase() {
    assert_eq!(
        phase(Action::Close, TargetKindTag::AppWindow),
        Phase::CloseDocuments
    );
    assert_eq!(
        phase(Action::Close, TargetKindTag::Process),
        Phase::CloseDocuments
    );
    assert_eq!(phase(Action::Quit, TargetKindTag::Process), Phase::Conceal);
    assert_eq!(phase(Action::Minimize, TargetKindTag::Tab), Phase::Conceal);
}

fn scripted_inventory() -> Vec<Target> {
    vec![
        tab("https://netbank.otpbank.hu/belepes", "OTP NetBank").with_id(1),
        tab("https://www.pornhub.com/view_video.php", "Video").with_id(2),
        tab("https://www.example.com/", "Example Domain")
            .private()
            .with_id(3),
        tab("https://meet.google.com/abc-defg-hij", "Meet – Weekly")
            .private()
            .with_id(4),
        document(
            Some("/Users/anna/Documents/Számlák/szamla_2026_10.pdf"),
            "szamla_2026_10.pdf",
            Dirty::Dirty,
        )
        .with_id(5),
        app_window("com.1password.1password", "1Password", "Personal").with_id(6),
        process(
            "com.tinyspeck.slackmacgap",
            "Slack",
            "/Applications/Slack.app/Contents/MacOS/Slack",
        )
        .with_id(7),
        tab("https://docs.rs/url", "url - Rust").with_id(8),
    ]
}

#[test]
fn recommended_preset_on_panic() {
    let f = Fixture::new();
    let plan = f.plan(
        &FiredTrigger::Panic,
        &scripted_inventory(),
        &Config::recommended().rules,
    );
    insta::assert_yaml_snapshot!(plan);
}

#[test]
fn recommended_preset_on_screen_share() {
    let f = Fixture::new();
    let fired = FiredTrigger::ScreenShare {
        confidence: Confidence::High,
    };
    let plan = f.plan(&fired, &scripted_inventory(), &Config::recommended().rules);
    insta::assert_yaml_snapshot!(plan);
}

const URLS: &[&str] = &[
    "https://netbank.otpbank.hu/",
    "https://www.pornhub.com/",
    "https://meet.google.com/x",
    "https://mail.google.com/",
    "https://example.com/",
    "https://us02web.zoom.us/j/1",
];
const TITLES: &[&str] = &["Számla 2026", "Internetbank", "Inbox", "Notes", ""];
const PATHS: &[Option<&str>] = &[
    Some("/Users/anna/szamla.pdf"),
    Some("/Users/anna/notes.txt"),
    None,
];
const APPS: &[(&str, &str)] = &[
    ("com.1password.1password", "1Password"),
    ("com.tinyspeck.slackmacgap", "Slack"),
    ("us.zoom.xos", "zoom.us"),
    ("com.apple.TextEdit", "TextEdit"),
];
const CATEGORIES: &[&str] = &[
    "adult",
    "banking",
    "webmail",
    "invoices",
    "password_managers",
    "messengers",
    "meeting",
];
const KINDS: &[TargetKindTag] = &[
    TargetKindTag::Tab,
    TargetKindTag::Document,
    TargetKindTag::AppWindow,
    TargetKindTag::Process,
];
const ACTIONS: &[Action] = &[
    Action::Quit,
    Action::Close,
    Action::Hide,
    Action::Minimize,
    Action::NotifyOnly,
];
const POLICIES: &[UnsavedPolicy] = &[
    UnsavedPolicy::Hide,
    UnsavedPolicy::SkipAndNotify,
    UnsavedPolicy::SaveThenClose,
    UnsavedPolicy::Discard,
];

fn arb_tri() -> impl Strategy<Value = Tri> {
    select(vec![Tri::Yes, Tri::No, Tri::Unknown])
}

fn arb_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        (
            select(URLS),
            select(TITLES),
            arb_tri(),
            arb_tri(),
            arb_tri()
        )
            .prop_map(|(url, title, private, audible, pinned)| {
                let mut target = tab(url, title);
                if let TargetKind::BrowserTab(tab) = &mut target.kind {
                    tab.private = private;
                    tab.audible = audible;
                    tab.pinned = pinned;
                }
                target
            }),
        (
            select(PATHS),
            select(TITLES),
            select(vec![Dirty::Clean, Dirty::Dirty, Dirty::Unknown])
        )
            .prop_map(|(path, title, dirty)| document(path, title, dirty)),
        (select(APPS), select(TITLES)).prop_map(|((id, name), title)| app_window(id, name, title)),
        select(APPS).prop_map(|(id, name)| process(id, name, "/Applications/X.app")),
    ]
}

fn arb_leaf() -> impl Strategy<Value = Matcher> {
    prop_oneof![
        select(CATEGORIES).prop_map(category),
        select(vec!["otpbank.hu", "google.com", "zoom.us", "example.com"]).prop_map(domain),
        select(vec!["számla", "inbox", "notes"])
            .prop_map(|keyword| Matcher::TitleContains(keyword.to_owned())),
        select(APPS).prop_map(|(id, _)| Matcher::App(AppSelector::BundleId(id.to_owned()))),
        Just(Matcher::Private),
        Just(Matcher::DocKind(DocKind::Pdf)),
    ]
}

fn arb_matcher() -> impl Strategy<Value = Matcher> {
    prop_oneof![
        arb_leaf(),
        prop::collection::vec(arb_leaf(), 1..3).prop_map(Matcher::Any),
        (arb_leaf(), arb_leaf())
            .prop_map(|(a, b)| Matcher::All(vec![a, Matcher::Not(Box::new(b))])),
    ]
}

fn arb_trigger() -> impl Strategy<Value = Trigger> {
    select(vec![
        Trigger::Panic,
        Trigger::ScreenShare,
        Trigger::Locked,
        Trigger::AwayLongerThan(Dur::from_mins(15)),
        Trigger::ItemOpenLongerThan(Dur::from_hours(1)),
    ])
}

fn arb_rule() -> impl Strategy<Value = Rule> {
    (
        prop::collection::vec(arb_trigger(), 1..3),
        prop::collection::vec(select(KINDS), 1..4),
        arb_matcher(),
        prop::option::weighted(0.3, arb_leaf()),
        select(ACTIONS),
        select(POLICIES),
        prop::option::weighted(0.3, select(vec![30u64, 60, 90])),
        prop::bool::weighted(0.9),
    )
        .prop_map(
            |(when, targets, matcher, except, action, on_unsaved, countdown, enabled)| Rule {
                id: RuleId(String::new()),
                name: None,
                enabled,
                when,
                targets,
                matcher,
                except,
                action,
                on_unsaved,
                confirm: countdown.map_or(Confirm::Never, |secs| {
                    Confirm::Countdown(Dur::from_secs(secs))
                }),
            },
        )
}

fn arb_capability() -> impl Strategy<Value = Capability> {
    (
        select(KINDS),
        select(ACTIONS),
        prop::option::weighted(0.3, select(APPS)),
        select(vec![
            Support::Full,
            Support::Partial {
                note_key: "n".to_owned(),
            },
            Support::Unsupported,
        ]),
    )
        .prop_map(|(kind, action, app, support)| {
            capability(kind, action, app.map(|(id, _)| id), support)
        })
}

fn arb_fired() -> impl Strategy<Value = FiredTrigger> {
    prop_oneof![
        Just(FiredTrigger::Panic),
        Just(FiredTrigger::ScreenShare {
            confidence: Confidence::Medium
        }),
        Just(FiredTrigger::Locked),
        (0u64..40).prop_map(|mins| FiredTrigger::Away {
            away: Dur::from_mins(mins)
        }),
        (0u64..8).prop_map(|id| FiredTrigger::ItemAge {
            target: TargetId(id),
            age: Dur::from_hours(2)
        }),
    ]
}

#[derive(Debug, Clone)]
struct Scenario {
    fixture_protect: Protect,
    protect_apps: Vec<AppSelector>,
    capabilities: Vec<Capability>,
    fired: FiredTrigger,
    snapshot: Vec<Target>,
    rules: Vec<Rule>,
}

impl Scenario {
    fn fixture(&self) -> Fixture {
        Fixture {
            index: index(),
            protect: self.fixture_protect.clone(),
            protect_apps: self.protect_apps.clone(),
            capabilities: self.capabilities.clone(),
        }
    }

    /// Rules that select `target`, computed without the planner's merge.
    fn matching_rules<'a>(&'a self, f: &Fixture, target: &Target) -> Vec<&'a Rule> {
        let ctx = MatchContext {
            categories: &f.index,
            home: HOME,
        };
        self.rules
            .iter()
            .filter(|rule| armed(rule, &self.fired) && super::select(rule, target, &ctx).is_some())
            .collect()
    }
}

fn arb_scenario() -> impl Strategy<Value = Scenario> {
    (
        prop::collection::vec(select(vec!["meeting", "banking", "messengers"]), 0..3),
        prop::option::weighted(0.3, Just("example.com")),
        prop::option::weighted(0.3, select(APPS)),
        any::<(bool, bool)>(),
        prop::option::weighted(0.3, select(APPS)),
        prop::collection::vec(arb_capability(), 0..30),
        arb_fired(),
        prop::collection::vec(arb_target(), 0..8),
        prop::collection::vec(arb_rule(), 0..6),
    )
        .prop_map(
            |(
                categories,
                protect_domain,
                protect_app,
                (audible_tabs, pinned_tabs),
                run_app,
                capabilities,
                fired,
                snapshot,
                rules,
            )| {
                let bundle = |(id, _): (&str, &str)| AppSelector::BundleId(id.to_owned());
                Scenario {
                    fixture_protect: Protect {
                        categories: categories
                            .into_iter()
                            .map(|c| CategoryId(c.to_owned()))
                            .collect(),
                        apps: protect_app.into_iter().map(bundle).collect(),
                        domains: protect_domain.into_iter().map(str::to_owned).collect(),
                        audible_tabs,
                        pinned_tabs,
                    },
                    protect_apps: run_app.into_iter().map(bundle).collect(),
                    capabilities,
                    fired,
                    snapshot: snapshot
                        .into_iter()
                        .enumerate()
                        .map(|(i, target)| target.with_id(i as u64))
                        .collect(),
                    rules: rules
                        .into_iter()
                        .enumerate()
                        .map(|(i, rule)| Rule {
                            id: RuleId(format!("r{i}")),
                            ..rule
                        })
                        .collect(),
                }
            },
        )
}

fn independently_protected(f: &Fixture, target: &Target) -> bool {
    let ctx = MatchContext {
        categories: &f.index,
        home: HOME,
    };
    let protect = &f.protect;
    let tab_flag = match &target.kind {
        TargetKind::BrowserTab(tab) => {
            (protect.audible_tabs && tab.audible == Tri::Yes)
                || (protect.pinned_tabs && tab.pinned == Tri::Yes)
        }
        _ => false,
    };
    tab_flag
        || protect
            .categories
            .iter()
            .any(|id| evaluate(&Matcher::Category(id.clone()), target, &ctx).is_some())
        || protect
            .domains
            .iter()
            .any(|d| evaluate(&Matcher::Domain(d.clone()), target, &ctx).is_some())
        || protect
            .apps
            .iter()
            .chain(&f.protect_apps)
            .any(|selector| app_matches(selector, target))
}

proptest! {
    #[test]
    fn prop_fallback_never_escalates(scenario in arb_scenario()) {
        let f = scenario.fixture();
        let plan = f.plan(&scenario.fired, &scenario.snapshot, &scenario.rules);
        for step in &plan.steps {
            let target = &scenario.snapshot[step.target.0 as usize];
            let strongest = scenario
                .matching_rules(&f, target)
                .iter()
                .map(|rule| severity(rule.action))
                .max()
                .expect("a step always has a matching rule");
            prop_assert!(severity(step.action) <= strongest);
            match step.fell_back_from {
                Some(from) => {
                    prop_assert!(severity(step.action) < severity(from));
                    prop_assert_eq!(severity(from), strongest);
                }
                None => prop_assert_eq!(severity(step.action), strongest),
            }
        }
    }

    #[test]
    fn prop_protected_targets_never_appear(scenario in arb_scenario()) {
        let f = scenario.fixture();
        let plan = f.plan(&scenario.fired, &scenario.snapshot, &scenario.rules);
        for step in &plan.steps {
            let target = &scenario.snapshot[step.target.0 as usize];
            prop_assert!(!independently_protected(&f, target), "{target:?} is protected");
        }
    }

    #[test]
    fn prop_discard_needs_a_matching_discard_rule(scenario in arb_scenario()) {
        let f = scenario.fixture();
        let plan = f.plan(&scenario.fired, &scenario.snapshot, &scenario.rules);
        for step in plan.steps.iter().filter(|step| step.on_unsaved == UnsavedPolicy::Discard) {
            let target = &scenario.snapshot[step.target.0 as usize];
            prop_assert!(scenario
                .matching_rules(&f, target)
                .iter()
                .any(|rule| rule.on_unsaved == UnsavedPolicy::Discard));
        }
    }

    #[test]
    fn prop_plan_ignores_input_order(
        (scenario, snapshot, rules) in arb_scenario().prop_flat_map(|scenario| {
            let snapshot = Just(scenario.snapshot.clone()).prop_shuffle();
            let rules = Just(scenario.rules.clone()).prop_shuffle();
            (Just(scenario), snapshot, rules)
        })
    ) {
        let f = scenario.fixture();
        let expected = f.plan(&scenario.fired, &scenario.snapshot, &scenario.rules);
        prop_assert_eq!(f.plan(&scenario.fired, &snapshot, &rules), expected.clone());
        prop_assert!(expected.steps.is_sorted_by_key(|step| (step.phase, step.target)));
    }
}
