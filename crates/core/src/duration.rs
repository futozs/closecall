use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(export, type = "string")]
pub struct Dur(pub Duration);

impl Dur {
    pub const fn from_secs(secs: u64) -> Self {
        Self(Duration::from_secs(secs))
    }

    pub const fn from_mins(mins: u64) -> Self {
        Self(Duration::from_secs(mins * 60))
    }

    pub const fn from_hours(hours: u64) -> Self {
        Self(Duration::from_secs(hours * 3600))
    }

    pub fn as_millis(&self) -> u64 {
        u64::try_from(self.0.as_millis()).unwrap_or(u64::MAX)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid duration {0:?}: use a number followed by s, m, h or d, for example 15m")]
pub struct ParseDurError(pub String);

impl FromStr for Dur {
    type Err = ParseDurError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let split = trimmed
            .find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| ParseDurError(s.to_owned()))?;
        let (number, unit) = trimmed.split_at(split);
        let n: u64 = number.parse().map_err(|_| ParseDurError(s.to_owned()))?;
        let secs = match unit {
            "s" => Some(n),
            "m" => n.checked_mul(60),
            "h" => n.checked_mul(3600),
            "d" => n.checked_mul(86_400),
            _ => None,
        }
        .ok_or_else(|| ParseDurError(s.to_owned()))?;
        Ok(Self(Duration::from_secs(secs)))
    }
}

impl TryFrom<String> for Dur {
    type Error = ParseDurError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Dur> for String {
    fn from(value: Dur) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Dur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.as_secs();
        if secs != 0 && secs % 86_400 == 0 {
            write!(f, "{}d", secs / 86_400)
        } else if secs != 0 && secs % 3600 == 0 {
            write!(f, "{}h", secs / 3600)
        } else if secs != 0 && secs % 60 == 0 {
            write!(f, "{}m", secs / 60)
        } else {
            write!(f, "{secs}s")
        }
    }
}

impl schemars::JsonSchema for Dur {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Duration".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[0-9]+[smhd]$",
            "examples": ["90s", "15m", "2h", "1d"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_common_values() {
        for s in ["90s", "15m", "2h", "1d", "0s"] {
            assert_eq!(s.parse::<Dur>().unwrap().to_string(), s);
        }
        assert_eq!("120m".parse::<Dur>().unwrap().to_string(), "2h");
    }

    #[test]
    fn rejects_garbage() {
        for s in ["", "m", "15", "15x", "-1m", "1.5h"] {
            assert!(s.parse::<Dur>().is_err(), "{s} should not parse");
        }
    }
}
