use std::collections::HashSet;
use std::sync::OnceLock;

use closecall_core::ipc::ListSource;
use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/blobs.rs"));

pub(crate) const MANIFEST_TOML: &str = include_str!("../../../lists/manifest.toml");

#[derive(Debug, Deserialize)]
pub(crate) struct ManifestEntry {
    pub id: String,
    pub name: String,
    pub url: String,
    pub license: String,
    pub snapshot_date: String,
    pub entries: u32,
}

impl ManifestEntry {
    pub fn source(&self) -> ListSource {
        ListSource {
            id: self.id.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
            license: self.license.clone(),
            snapshot_date: self.snapshot_date.clone(),
            entries: self.entries,
        }
    }
}

pub(crate) fn manifest() -> &'static [ManifestEntry] {
    #[derive(Deserialize)]
    struct Manifest {
        list: Vec<ManifestEntry>,
    }
    static PARSED: OnceLock<Vec<ManifestEntry>> = OnceLock::new();
    PARSED.get_or_init(|| {
        toml::from_str::<Manifest>(MANIFEST_TOML)
            .expect("lists/manifest.toml is written by scripts/lists/snapshot.ts")
            .list
    })
}

pub(crate) fn manifest_entry(id: &str) -> Option<&'static ManifestEntry> {
    manifest().iter().find(|entry| entry.id == id)
}

/// Decompressed on first use; a list no enabled category touches never costs memory.
pub(crate) fn domains(list_id: &str) -> Option<&'static HashSet<Box<str>>> {
    static SETS: [OnceLock<HashSet<Box<str>>>; LIST_COUNT] =
        [const { OnceLock::new() }; LIST_COUNT];
    let index = BLOBS.iter().position(|(id, _)| *id == list_id)?;
    Some(SETS[index].get_or_init(|| decompress(BLOBS[index].1).lines().map(Box::from).collect()))
}

pub(crate) fn decompress(blob: &[u8]) -> String {
    let bytes = lz4_flex::decompress_size_prepended(blob).expect("blob was written by build.rs");
    String::from_utf8(bytes).expect("list files are ASCII")
}
