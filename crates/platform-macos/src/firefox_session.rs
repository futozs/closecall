//! Firefox's `recovery.jsonlz4`: mozLz4 framing (magic, u32 LE size, raw LZ4 block) around
//! the session-restore JSON. Firefox rewrites it about every 15 s and leaves private
//! windows out.

use std::path::{Path, PathBuf};

use serde::Deserialize;

const MAGIC: &[u8; 8] = b"mozLz40\0";
/// Larger than any real session; guards the allocation against a corrupt header.
const MAX_SIZE: usize = 256 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DecodeError {
    NotMozLz4,
    TooLarge(usize),
    Corrupt(String),
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let header = bytes.get(..12).ok_or(DecodeError::NotMozLz4)?;
    if &header[..8] != MAGIC {
        return Err(DecodeError::NotMozLz4);
    }
    let size = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    if size > MAX_SIZE {
        return Err(DecodeError::TooLarge(size));
    }
    lz4_flex::block::decompress(&bytes[12..], size).map_err(|e| DecodeError::Corrupt(e.to_string()))
}

#[derive(Debug, Default, Deserialize)]
struct Session {
    #[serde(default)]
    windows: Vec<Window>,
}

#[derive(Debug, Default, Deserialize)]
struct Window {
    #[serde(default)]
    tabs: Vec<Tab>,
    #[serde(default)]
    selected: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct Tab {
    #[serde(default)]
    entries: Vec<Entry>,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    pinned: Option<bool>,
    #[serde(default)]
    hidden: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct Entry {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionTab {
    pub window: usize,
    pub tab: usize,
    pub title: String,
    pub url: Option<String>,
    pub active: bool,
    pub pinned: bool,
}

pub(crate) fn parse(json: &[u8]) -> Result<Vec<SessionTab>, serde_json::Error> {
    let session: Session = serde_json::from_slice(json)?;
    let mut out = Vec::new();
    for (w, window) in session.windows.iter().enumerate() {
        for (t, tab) in window.tabs.iter().enumerate() {
            if tab.hidden == Some(true) {
                continue;
            }
            // `index` is 1-based into the back/forward history; the current page is there.
            let current = tab
                .index
                .and_then(|i| i.checked_sub(1))
                .and_then(|i| tab.entries.get(i))
                .or_else(|| tab.entries.last());
            let url = current.and_then(|e| e.url.clone());
            let title = current
                .and_then(|e| e.title.clone())
                .or_else(|| url.clone())
                .unwrap_or_default();
            out.push(SessionTab {
                window: w,
                tab: t,
                title,
                url,
                active: window.selected == Some(t + 1),
                pinned: tab.pinned == Some(true),
            });
        }
    }
    Ok(out)
}

/// The profile Firefox is writing to right now: the newest `recovery.jsonlz4`.
pub(crate) fn current_recovery_file(profiles_dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(profiles_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|profile| profile.path().join("sessionstore-backups/recovery.jsonlz4"))
        .filter_map(|file| {
            let modified = file.metadata().ok()?.modified().ok()?;
            Some((modified, file))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, file)| file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(json: &[u8]) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&u32::try_from(json.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&lz4_flex::block::compress(json));
        out
    }

    const SESSION: &str = r#"{
        "version": ["sessionrestore", 1],
        "windows": [
            {
                "selected": 2,
                "tabs": [
                    {"entries": [{"url": "https://news.example/", "title": "News"}], "index": 1, "pinned": true},
                    {"entries": [
                        {"url": "https://search.example/?q=bank", "title": "Search"},
                        {"url": "https://netbank.example/login", "title": "NetBank – Belépés"}
                    ], "index": 2},
                    {"entries": [{"url": "about:blank"}], "index": 1, "hidden": true}
                ]
            },
            {"tabs": [{"entries": [{"url": "https://mail.example/"}]}]}
        ]
    }"#;

    #[test]
    fn decodes_mozlz4_frames() {
        let json = SESSION.as_bytes();
        assert_eq!(decode(&frame(json)).unwrap(), json);
    }

    #[test]
    fn rejects_other_files() {
        assert_eq!(decode(b"{\"windows\":[]}"), Err(DecodeError::NotMozLz4));
        assert_eq!(decode(b"mozLz40"), Err(DecodeError::NotMozLz4));
        let mut huge = MAGIC.to_vec();
        huge.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(decode(&huge), Err(DecodeError::TooLarge(_))));
        let mut truncated = frame(SESSION.as_bytes());
        truncated.truncate(40);
        assert!(matches!(decode(&truncated), Err(DecodeError::Corrupt(_))));
    }

    #[test]
    fn reads_the_current_history_entry_of_each_visible_tab() {
        let tabs = parse(SESSION.as_bytes()).unwrap();
        assert_eq!(tabs.len(), 3);
        assert_eq!(tabs[0].title, "News");
        assert!(tabs[0].pinned && !tabs[0].active);
        assert_eq!(
            tabs[1].url.as_deref(),
            Some("https://netbank.example/login")
        );
        assert_eq!(tabs[1].title, "NetBank – Belépés");
        assert!(tabs[1].active);
        assert_eq!((tabs[2].window, tabs[2].tab), (1, 0));
        assert_eq!(tabs[2].title, "https://mail.example/");
    }

    #[test]
    fn tolerates_missing_fields() {
        assert!(parse(b"{}").unwrap().is_empty());
        assert!(
            parse(br#"{"windows":[{"tabs":[{}]}]}"#).unwrap()[0]
                .url
                .is_none()
        );
        assert!(parse(b"not json").is_err());
    }

    #[test]
    fn picks_the_newest_profile() {
        let root = std::env::temp_dir().join(format!("closecall-ff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for profile in ["old.default", "new.default-release"] {
            let dir = root.join(profile).join("sessionstore-backups");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("recovery.jsonlz4"), frame(b"{}")).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::fs::create_dir_all(root.join("empty.profile")).unwrap();
        let found = current_recovery_file(&root).unwrap();
        assert!(found.starts_with(root.join("new.default-release")));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
