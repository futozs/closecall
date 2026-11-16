use std::path::Path;

use super::*;

fn codes(config: &Config) -> Vec<(String, String)> {
    config
        .validate()
        .into_iter()
        .map(|issue| {
            (
                issue.field,
                issue.code.trim_start_matches("config.error.").to_owned(),
            )
        })
        .collect()
}

fn issues_in(toml: &str) -> Vec<(String, String)> {
    let config: Config = toml::from_str(toml).expect("test config parses");
    codes(&config)
}

fn pair(field: &str, code: &str) -> (String, String) {
    (field.to_owned(), code.to_owned())
}

#[test]
fn recommended_preset_has_the_documented_rules() {
    let config = Config::recommended();
    let ids: Vec<&str> = config.rules.iter().map(|rule| rule.id.0.as_str()).collect();
    assert_eq!(
        ids,
        [
            "share-close-sensitive-tabs",
            "private-windows-on-share",
            "share-hide-private-apps",
            "share-close-invoices",
            "lid-close-adult",
            "away-close-banking",
            "bank-tab-open-too-long",
        ]
    );
    for rule in &config.rules {
        assert_eq!(
            rule.name.as_deref(),
            Some(format!("preset.{}", rule.id.0).as_str())
        );
        assert_eq!(rule.on_unsaved, UnsavedPolicy::SaveThenClose);
        assert!(rule.enabled);
        assert_ne!(rule.action, Action::Quit);
    }
    assert_eq!(
        config.protect.categories,
        [CategoryId("meeting".to_owned())]
    );
    assert!(config.validate().is_empty(), "{:?}", config.validate());
}

#[test]
fn recommended_preset_round_trips_through_toml() {
    let config = Config::recommended();
    let text = config.to_toml();
    assert_eq!(Config::from_toml(&text).unwrap(), config);
    insta::assert_snapshot!(text);
}

#[test]
fn recommended_preset_round_trips_through_json() {
    let config = Config::recommended();
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(serde_json::from_str::<Config>(&json).unwrap(), config);
}

#[test]
fn documented_example_parses() {
    let text = r#"
version = 1
language = "system"
panic_hotkey = "Ctrl+Alt+Shift+B"

[protect]
categories = ["meeting"]

[[rule]]
id = "share-close-invoices"
when = ["panic", "screen_share", "display_mirrored"]
targets = ["document"]
match = { category = "invoices" }
action = "close"
on_unsaved = "hide"

[[rule]]
id = "away-close-banking"
when = [{ away_longer_than = "15m" }]
targets = ["tab"]
match = { any = [{ domain = "otpbank.hu" }, { all = [{ title_contains = "számla" }, { not = "private" }] }] }
except = { url_glob = "https://otpbank.hu/public/*" }
action = "close"

[[rule]]
id = "bank-tab-open-too-long"
when = [{ item_open_longer_than = "2h" }]
targets = ["tab"]
match = { category = "banking" }
action = "close"
confirm = { countdown = "60s" }
"#;
    let config = Config::from_toml(text).unwrap();
    assert_eq!(config.rules.len(), 3);
    assert_eq!(config.rules[0].on_unsaved, UnsavedPolicy::Hide);
    assert_eq!(
        config.rules[1].when,
        [Trigger::AwayLongerThan(Dur::from_mins(15))]
    );
    assert_eq!(
        config.rules[2].confirm,
        Confirm::Countdown(Dur::from_secs(60))
    );
    assert_eq!(
        config.untitled_save_dir,
        Config::default().untitled_save_dir
    );
}

#[test]
fn empty_file_is_the_default_config() {
    assert_eq!(Config::from_toml("").unwrap(), Config::default());
}

#[test]
fn syntax_and_type_errors_are_parse_errors() {
    for text in [
        "version = ",
        "[[rule]]\nid = \"x\"\nwhen = [\"sunset\"]\ntargets = [\"tab\"]\nmatch = \"private\"\naction = \"close\"",
        "[[rule]]\nid = \"x\"\nwhen = [{ idle_longer_than = \"15x\" }]\ntargets = [\"tab\"]\nmatch = \"private\"\naction = \"close\"",
        "[[rule]]\nid = \"x\"\nwhen = [\"panic\"]\ntargets = [\"tab\"]\nmatch = \"private\"\naction = \"explode\"",
        "[[rule]]\nid = \"x\"\nwhen = [\"panic\"]\ntargets = [\"tab\"]\naction = \"close\"",
        "language = \"de\"",
    ] {
        assert!(
            matches!(Config::from_toml(text), Err(ConfigError::Parse(_))),
            "{text}"
        );
    }
}

#[test]
fn newer_versions_are_refused_before_validation() {
    assert!(matches!(
        Config::from_toml(
            "version = 2\n[[rule]]\nid = \"x\"\nwhen = []\ntargets = []\nmatch = \"private\"\naction = \"close\""
        ),
        Err(ConfigError::TooNew(2))
    ));
}

#[test]
fn invalid_configs_report_every_issue() {
    let Err(ConfigError::Invalid(issues)) =
        Config::from_toml("version = 0\n[protect]\ndomains = [\"\"]")
    else {
        panic!("expected validation errors");
    };
    let fields: Vec<&str> = issues.iter().map(|issue| issue.field.as_str()).collect();
    assert_eq!(fields, ["version", "protect.domains[0]"]);
    assert_eq!(issues[0].code, "config.error.unsupported_version");
    assert_eq!(issues[0].rule, None);
}

const RULE: &str = "when = [\"panic\"]\ntargets = [\"tab\"]\naction = \"close\"\n";

#[test]
fn duplicate_rule_ids_point_at_the_later_rule() {
    let text = format!(
        "[[rule]]\nid = \"a\"\nmatch = \"private\"\n{RULE}[[rule]]\nid = \"b\"\nmatch = \"private\"\n{RULE}[[rule]]\nid = \"a\"\nmatch = \"private\"\n{RULE}"
    );
    let config: Config = toml::from_str(&text).unwrap();
    let issues = config.validate();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].rule, Some(RuleId("a".to_owned())));
    assert_eq!(issues[0].field, "rule[2].id");
    assert_eq!(issues[0].code, "config.error.duplicate_rule_id");
}

#[test]
fn empty_when_and_targets() {
    let text =
        "[[rule]]\nid = \"a\"\nwhen = []\ntargets = []\nmatch = \"private\"\naction = \"close\"";
    assert_eq!(
        issues_in(text),
        [
            pair("rule[0].when", "empty_when"),
            pair("rule[0].targets", "empty_targets")
        ]
    );
}

#[test]
fn countdowns_only_on_idle_and_item_age() {
    let rule = |when: &str| {
        format!(
            "[[rule]]\nid = \"a\"\nwhen = {when}\ntargets = [\"tab\"]\nmatch = \"private\"\naction = \"close\"\nconfirm = {{ countdown = \"30s\" }}"
        )
    };
    assert!(issues_in(&rule("[{ idle_longer_than = \"10m\" }]")).is_empty());
    assert!(
        issues_in(&rule(
            "[{ idle_longer_than = \"10m\" }, { item_open_longer_than = \"1h\" }]"
        ))
        .is_empty()
    );
    for when in [
        "[\"panic\"]",
        "[\"screen_share\"]",
        "[\"locked\"]",
        "[{ away_longer_than = \"15m\" }]",
        "[{ idle_longer_than = \"10m\" }, \"lid_closed\"]",
    ] {
        assert_eq!(
            issues_in(&rule(when)),
            [pair("rule[0].confirm", "countdown_not_allowed")],
            "{when}"
        );
    }
}

fn matcher_issues(matcher: &str) -> Vec<(String, String)> {
    issues_in(&format!("[[rule]]\nid = \"a\"\nmatch = {matcher}\n{RULE}"))
}

#[test]
fn empty_composites() {
    assert_eq!(
        matcher_issues("{ any = [] }"),
        [pair("rule[0].match", "empty_composite")]
    );
    assert_eq!(
        matcher_issues("{ any = [\"private\", { all = [] }] }"),
        [pair("rule[0].match.any[1]", "empty_composite")]
    );
}

#[test]
fn matchers_that_cannot_explain_a_match() {
    assert_eq!(
        matcher_issues("{ not = \"private\" }"),
        [pair("rule[0].match", "lone_not")]
    );
    assert_eq!(
        matcher_issues("{ all = [{ not = \"private\" }, { not = { domain = \"a.hu\" } }] }"),
        [pair("rule[0].match", "lone_not")]
    );
    assert_eq!(
        matcher_issues("{ any = [{ domain = \"a.hu\" }, { not = \"private\" }] }"),
        [pair("rule[0].match.any[1]", "lone_not")]
    );
    assert!(matcher_issues("{ all = [{ domain = \"a.hu\" }, { not = \"private\" }] }").is_empty());
    assert!(
        matcher_issues("{ all = [{ any = [{ not = \"private\" }] }, { title_contains = \"x\" }] }")
            .is_empty()
    );
    assert!(issues_in(&format!(
        "[[rule]]\nid = \"a\"\nmatch = \"private\"\nexcept = {{ not = {{ domain = \"a.hu\" }} }}\n{RULE}"
    ))
    .is_empty());
}

#[test]
fn bad_domains_and_globs() {
    assert_eq!(
        matcher_issues("{ domain = \"https://bank.hu\" }"),
        [pair("rule[0].match", "invalid_domain")]
    );
    assert_eq!(
        matcher_issues("{ any = [{ domain = \"bank .hu\" }, { file_glob = \"**/[a-\" }] }"),
        [
            pair("rule[0].match.any[0]", "invalid_domain"),
            pair("rule[0].match.any[1]", "invalid_glob")
        ]
    );
    assert!(
        matcher_issues(
            "{ any = [{ domain = \"*.bücher.de\" }, { file_glob = \"~/Documents/Bank/**\" }] }"
        )
        .is_empty()
    );
    assert_eq!(
        issues_in(&format!(
            "[[rule]]\nid = \"a\"\nmatch = \"private\"\nexcept = {{ domain = \"a:b\" }}\n{RULE}"
        )),
        [pair("rule[0].except", "invalid_domain")]
    );
}

#[test]
fn bad_category_and_protect_entries() {
    let text = r#"
[protect]
domains = ["ok.hu", "bad/one"]

[[categories]]
id = "mine"
domains = ["ok.hu", ""]
file_globs = ["**/*.pdf", "{a,b"]
"#;
    assert_eq!(
        issues_in(text),
        [
            pair("protect.domains[1]", "invalid_domain"),
            pair("categories[0].domains[1]", "invalid_domain"),
            pair("categories[0].file_globs[1]", "invalid_glob"),
        ]
    );
}

#[test]
fn json_schema_file_is_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/config.schema.json");
    let fresh = serde_json::to_string_pretty(&Config::json_schema()).unwrap() + "\n";
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &fresh).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        current == fresh,
        "schema/config.schema.json is stale: run `UPDATE_SCHEMA=1 cargo test -p closecall-core json_schema`"
    );
}
