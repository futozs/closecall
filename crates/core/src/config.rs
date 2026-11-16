use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AppSelector, CategoryDef, CategoryId, Confidence, Dur, Rule, RuleId};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(default)]
#[ts(export)]
pub struct Config {
    pub version: u32,
    pub language: Language,
    pub panic_hotkey: Option<String>,
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
            panic_hotkey: Some("Ctrl+Alt+Shift+B".to_owned()),
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
#[serde(default)]
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
        todo!("closecall-core: recommended preset")
    }

    pub fn validate(&self) -> Vec<ValidationIssue> {
        todo!("closecall-core: validation")
    }

    pub fn json_schema() -> schemars::Schema {
        schemars::schema_for!(Config)
    }
}
