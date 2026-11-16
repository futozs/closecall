use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use closecall_core::config::{Config, ConfigError};

const FILE: &str = "config.toml";
/// Where a rejected `config.toml` goes before the first save replaces it.
pub(crate) const REJECTED: &str = "config.rejected.toml";

/// Why the config on disk is not in use. The file itself is never touched until the user saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Problem {
    Unreadable,
    Unparsable,
    TooNew(u32),
    Invalid(usize),
}

impl Problem {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Unreadable => "config.problem.unreadable",
            Self::Unparsable => "config.problem.unparsable",
            Self::TooNew(_) => "config.problem.too_new",
            Self::Invalid(_) => "config.problem.invalid",
        }
    }
}

pub(crate) enum Loaded {
    Config(Box<Config>),
    Missing,
    Rejected(Problem),
}

pub(crate) fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

pub(crate) fn load(dir: &Path) -> Loaded {
    let text = match fs::read_to_string(path(dir)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Loaded::Missing,
        Err(_) => return Loaded::Rejected(Problem::Unreadable),
    };
    match Config::from_toml(&text) {
        Ok(config) => Loaded::Config(Box::new(config)),
        Err(ConfigError::Parse(_)) => Loaded::Rejected(Problem::Unparsable),
        Err(ConfigError::TooNew(version)) => Loaded::Rejected(Problem::TooNew(version)),
        Err(ConfigError::Invalid(issues)) => Loaded::Rejected(Problem::Invalid(issues.len())),
    }
}

/// Write to a temporary file next to the target, then rename over it, so a crash never
/// leaves half a config behind. The file may name the user's bank, so only they can read it.
pub(crate) fn save(dir: &Path, config: &Config) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fs::create_dir_all(dir)?;
    let temp = dir.join(format!(
        ".{FILE}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = write_private(&temp, config.to_toml().as_bytes())
        .and_then(|()| fs::rename(&temp, path(dir)));
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written
}

/// Keeps the user's own file, which the engine could not use, next to the one about to replace it.
pub(crate) fn keep_rejected(dir: &Path) -> io::Result<()> {
    match fs::read(path(dir)) {
        Ok(bytes) => write_private(&dir.join(REJECTED), &bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// `~` and `~/…` are relative to the user's home; so is any other relative path.
pub(crate) fn expand(raw: &str, home: &Path) -> PathBuf {
    match raw.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') || rest.starts_with('\\') => home.join(&rest[1..]),
        _ => {
            let path = PathBuf::from(raw);
            if path.is_absolute() {
                path
            } else {
                home.join(path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home_relative_paths() {
        let home = Path::new("/Users/demo");
        assert_eq!(
            expand("~/Documents/Close Call", home),
            PathBuf::from("/Users/demo/Documents/Close Call")
        );
        assert_eq!(expand("~", home), PathBuf::from("/Users/demo"));
        assert_eq!(expand("/tmp/x", home), PathBuf::from("/tmp/x"));
        assert_eq!(expand("Saved", home), PathBuf::from("/Users/demo/Saved"));
    }
}
