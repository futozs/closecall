use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::matcher::{compile_glob, parse_domain};
use crate::plan::trigger_allows_confirm;
use crate::{
    Action, AppSelector, CategoryDef, CategoryId, Confidence, Confirm, Dur, Matcher, Rule, RuleId,
    TargetKindTag, Trigger, UnsavedPolicy,
};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct Config {
    pub version: u32,
    pub language: Language,
    /// Empty means no hotkey: TOML has no null, so `None` would come back as the default.
    pub panic_hotkey: String,
    pub share_min_confidence: Confidence,
    pub untitled_save_dir: String,
    pub restore_window: Dur,
    pub protect: Protect,
    pub categories: Vec<CategoryDef>,
    #[serde(rename = "rule")]
    #[ts(rename = "rule")]
    pub rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: Language::System,
            panic_hotkey: "Ctrl+Alt+Shift+B".to_owned(),
            share_min_confidence: Confidence::Medium,
            untitled_save_dir: "~/Documents/Close Call".to_owned(),
            restore_window: Dur::from_hours(2),
            protect: Protect::default(),
            categories: Vec::new(),
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Language {
    #[default]
    System,
    En,
    Hu,
}

/// Targets that are never touched, whatever the rules say. Protection always wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct Protect {
    pub categories: Vec<CategoryId>,
    pub apps: Vec<AppSelector>,
    pub domains: Vec<String>,
    pub audible_tabs: bool,
    pub pinned_tabs: bool,
}

impl Default for Protect {
    fn default() -> Self {
        Self {
            categories: vec![CategoryId("meeting".to_owned())],
            apps: Vec::new(),
            domains: Vec::new(),
            audible_tabs: false,
            pinned_tabs: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidationIssue {
    pub rule: Option<RuleId>,
    pub field: String,
    pub code: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config has {} problem(s)", .0.len())]
    Invalid(Vec<ValidationIssue>),
    #[error("config version {0} is newer than this app understands ({CONFIG_VERSION})")]
    TooNew(u32),
}

impl Config {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text)?;
        if config.version > CONFIG_VERSION {
            return Err(ConfigError::TooNew(config.version));
        }
        let issues = config.validate();
        if issues.is_empty() {
            Ok(config)
        } else {
            Err(ConfigError::Invalid(issues))
        }
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("Config always serializes to TOML")
    }

    pub fn recommended() -> Self {
        let share = || {
            vec![
                Trigger::Panic,
                Trigger::ScreenShare,
                Trigger::DisplayMirrored,
            ]
        };
        let category = |id: &str| Matcher::Category(CategoryId(id.to_owned()));
        let any = |ids: &[&str]| Matcher::Any(ids.iter().map(|id| category(id)).collect());
        Self {
            rules: vec![
                preset_rule(
                    "share-close-sensitive-tabs",
                    share(),
                    vec![TargetKindTag::Tab],
                    any(&["adult", "banking", "webmail", "medical"]),
                    Action::Close,
                ),
                preset_rule(
                    "private-windows-on-share",
                    share(),
                    vec![TargetKindTag::Tab],
                    Matcher::Private,
                    Action::Close,
                ),
                preset_rule(
                    "share-hide-private-apps",
                    share(),
                    vec![TargetKindTag::AppWindow, TargetKindTag::Process],
                    any(&["password_managers", "messengers"]),
                    Action::Hide,
                ),
                preset_rule(
                    "share-close-invoices",
                    share(),
                    vec![TargetKindTag::Document],
                    category("invoices"),
                    Action::Close,
                ),
                preset_rule(
                    "lid-close-adult",
                    vec![Trigger::LidClosed, Trigger::WillSleep],
                    vec![TargetKindTag::Tab],
                    category("adult"),
                    Action::Close,
                ),
                preset_rule(
                    "away-close-banking",
                    vec![Trigger::AwayLongerThan(Dur::from_mins(15))],
                    vec![TargetKindTag::Tab],
                    category("banking"),
                    Action::Close,
                ),
                Rule {
                    confirm: Confirm::Countdown(Dur::from_secs(60)),
                    ..preset_rule(
                        "bank-tab-open-too-long",
                        vec![Trigger::ItemOpenLongerThan(Dur::from_hours(2))],
                        vec![TargetKindTag::Tab],
                        category("banking"),
                        Action::Close,
                    )
                },
            ],
            ..Self::default()
        }
    }

    /// Codes are i18n keys under `config.error.`; fields are paths like `rule[2].confirm`.
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Issues::default();
        if self.version != CONFIG_VERSION {
            issues.push(None, "version".to_owned(), "unsupported_version");
        }
        for (i, domain) in self.protect.domains.iter().enumerate() {
            if parse_domain(domain).is_none() {
                issues.push(None, format!("protect.domains[{i}]"), "invalid_domain");
            }
        }
        for (c, category) in self.categories.iter().enumerate() {
            for (i, domain) in category.domains.iter().enumerate() {
                if parse_domain(domain).is_none() {
                    issues.push(
                        None,
                        format!("categories[{c}].domains[{i}]"),
                        "invalid_domain",
                    );
                }
            }
            for (i, glob) in category.file_globs.iter().enumerate() {
                if compile_glob(glob).is_err() {
                    issues.push(
                        None,
                        format!("categories[{c}].file_globs[{i}]"),
                        "invalid_glob",
                    );
                }
            }
        }

        let mut seen = HashSet::new();
        for (i, rule) in self.rules.iter().enumerate() {
            let id = Some(&rule.id);
            if !seen.insert(&rule.id) {
                issues.push(id, format!("rule[{i}].id"), "duplicate_rule_id");
            }
            if rule.when.is_empty() {
                issues.push(id, format!("rule[{i}].when"), "empty_when");
            }
            if rule.targets.is_empty() {
                issues.push(id, format!("rule[{i}].targets"), "empty_targets");
            }
            if matches!(rule.confirm, Confirm::Countdown(_))
                && !rule.when.iter().all(trigger_allows_confirm)
            {
                issues.push(id, format!("rule[{i}].confirm"), "countdown_not_allowed");
            }
            issues.check_matcher(id, &rule.matcher, format!("rule[{i}].match"), true);
            if let Some(except) = &rule.except {
                issues.check_matcher(id, except, format!("rule[{i}].except"), false);
            }
        }
        issues.0
    }

    pub fn json_schema() -> schemars::Schema {
        schemars::schema_for!(Config)
    }
}

fn preset_rule(
    id: &str,
    when: Vec<Trigger>,
    targets: Vec<TargetKindTag>,
    matcher: Matcher,
    action: Action,
) -> Rule {
    Rule {
        id: RuleId(id.to_owned()),
        name: Some(format!("preset.{id}")),
        enabled: true,
        when,
        targets,
        matcher,
        except: None,
        action,
        on_unsaved: UnsavedPolicy::default(),
        confirm: Confirm::Never,
    }
}

#[derive(Default)]
struct Issues(Vec<ValidationIssue>);

impl Issues {
    fn push(&mut self, rule: Option<&RuleId>, field: String, code: &str) {
        self.0.push(ValidationIssue {
            rule: rule.cloned(),
            field,
            code: format!("config.error.{code}"),
        });
    }

    /// `needs_evidence` holds where a match has to explain itself: the rule's top-level
    /// matcher and the branches of an `any` under it. `except` only filters.
    fn check_matcher(
        &mut self,
        rule: Option<&RuleId>,
        matcher: &Matcher,
        path: String,
        needs_evidence: bool,
    ) {
        match matcher {
            Matcher::Domain(domain) if parse_domain(domain).is_none() => {
                self.push(rule, path, "invalid_domain");
            }
            Matcher::FileGlob(glob) if compile_glob(glob).is_err() => {
                self.push(rule, path, "invalid_glob");
            }
            Matcher::Any(children) | Matcher::All(children) if children.is_empty() => {
                self.push(rule, path, "empty_composite");
            }
            Matcher::Any(children) => {
                for (i, child) in children.iter().enumerate() {
                    self.check_matcher(rule, child, format!("{path}.any[{i}]"), needs_evidence);
                }
            }
            Matcher::All(children) => {
                if needs_evidence && !children.iter().any(always_has_evidence) {
                    self.push(rule, path.clone(), "lone_not");
                }
                for (i, child) in children.iter().enumerate() {
                    self.check_matcher(rule, child, format!("{path}.all[{i}]"), false);
                }
            }
            Matcher::Not(inner) => {
                if needs_evidence {
                    self.push(rule, path.clone(), "lone_not");
                }
                self.check_matcher(rule, inner, format!("{path}.not"), false);
            }
            _ => {}
        }
    }
}

fn always_has_evidence(matcher: &Matcher) -> bool {
    match matcher {
        Matcher::Not(_) => false,
        Matcher::Any(children) => !children.is_empty() && children.iter().all(always_has_evidence),
        Matcher::All(children) => children.iter().any(always_has_evidence),
        _ => true,
    }
}

#[cfg(test)]
mod tests;
