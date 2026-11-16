use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;

use closecall_core::matcher::CategoryIndex;
use closecall_core::{AppSelector, CategoryDef, CategoryId};
use sha2::{Digest, Sha256};

use crate::{Lists, bundled, categories, contains_host};

const MAX_COMPRESSED_BYTES: usize = 4 * 1024 * 1024;
const GPL3_COMPATIBLE: &[&str] = &[
    "MIT",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "GPL-3.0",
    "GPL-3.0-or-later",
];

fn lists_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../lists")
}

fn id(text: &str) -> CategoryId {
    CategoryId(text.to_owned())
}

fn in_category(lists: &Lists, category: &str, host: &str) -> bool {
    let def = lists.category(&id(category)).expect("built-in category");
    def.lists.iter().any(|list| lists.list_contains(list, host))
}

#[test]
fn hits_and_misses_per_category() {
    let lists = Lists::bundled();
    let samples = [
        ("adult", "pornhub.com", "wikipedia.org"),
        ("adult", "www.xvideos.com", "github.com"),
        ("banking", "internetbank.otpbank.hu", "index.hu"),
        ("banking", "www.paypal.com", "otpbank.hu.example.com"),
        ("webmail", "mail.google.com", "google.com"),
        ("webmail", "freemail.hu", "drive.google.com"),
        ("medical", "eeszt.gov.hu", "gov.hu"),
        ("financial", "boursorama.com", "example.com"),
        ("dating", "tinder.com", "example.org"),
        ("gambling", "www.bet365.com", "example.net"),
    ];
    for (category, hit, miss) in samples {
        assert!(
            in_category(&lists, category, hit),
            "{hit} should be {category}"
        );
        assert!(
            !in_category(&lists, category, miss),
            "{miss} should not be {category}"
        );
    }
}

#[test]
fn every_bundled_list_matches_its_own_entries_and_their_subdomains() {
    let lists = Lists::bundled();
    for entry in bundled::manifest() {
        let text = fs::read_to_string(lists_dir().join(format!("{}.txt", entry.id))).unwrap();
        let first = text.lines().next().unwrap();
        let last = text.lines().last().unwrap();
        assert!(
            lists.list_contains(&entry.id, first),
            "{}: {first}",
            entry.id
        );
        assert!(
            lists.list_contains(&entry.id, &format!("deep.sub.{last}")),
            "{}: {last}",
            entry.id
        );
        assert!(!lists.list_contains(&entry.id, "closecall-never-listed.example"));
    }
    assert!(!lists.list_contains("no-such-list", "pornhub.com"));
}

#[test]
fn parent_domains_match_down_to_the_registrable_domain() {
    let domains: HashSet<Box<str>> = ["otpbank.hu", "blogspot.com", "sub.example.co.uk"]
        .into_iter()
        .map(Box::from)
        .collect();
    assert!(contains_host(&domains, "otpbank.hu"));
    assert!(contains_host(&domains, "a.b.otpbank.hu"));
    assert!(contains_host(&domains, "otpbank.hu."));
    assert!(contains_host(&domains, "x.sub.example.co.uk"));
    assert!(!contains_host(&domains, "nototpbank.hu"));
    assert!(!contains_host(&domains, "otpbank.hu.example.com"));
    assert!(!contains_host(&domains, "example.co.uk"));
    assert!(!contains_host(&domains, "someblog.blogspot.com"));
    assert!(contains_host(&domains, "blogspot.com"));
    assert!(!contains_host(&domains, "localhost"));
}

#[test]
fn user_category_with_builtin_id_extends_it() {
    let user = [
        CategoryDef {
            id: id("banking"),
            name: Some("Banks".to_owned()),
            domains: vec!["mybank.example".to_owned()],
            lists: vec!["seed-banking".to_owned(), "ut1-financial".to_owned()],
            apps: vec![AppSelector::BundleId("com.example.bank".to_owned())],
            title_keywords: vec!["NetBank".to_owned(), "Számlatörténet".to_owned()],
            file_globs: vec!["~/Bank/**".to_owned()],
        },
        CategoryDef {
            id: id("work"),
            domains: vec!["intranet.example".to_owned(), "intranet.example".to_owned()],
            ..CategoryDef::default()
        },
    ];
    let lists = Lists::bundled().with_user_categories(&user);

    let banking = lists.category(&id("banking")).unwrap();
    assert_eq!(banking.name.as_deref(), Some("Banks"));
    assert_eq!(banking.domains, ["mybank.example"]);
    assert_eq!(banking.lists, ["seed-banking", "ut1-bank", "ut1-financial"]);
    assert_eq!(
        banking.apps,
        [AppSelector::BundleId("com.example.bank".to_owned())]
    );
    assert_eq!(
        banking.title_keywords,
        ["Internetbank", "NetBank", "e-bank", "Számlatörténet"]
    );
    assert_eq!(banking.file_globs, ["~/Bank/**"]);
    assert!(in_category(&lists, "banking", "boursorama.com"));

    let work = lists.category(&id("work")).unwrap();
    assert_eq!(work.domains, ["intranet.example"]);

    let infos = lists.infos();
    let ids: Vec<_> = infos.iter().map(|info| info.id.0.as_str()).collect();
    assert_eq!(ids.last(), Some(&"work"));
    assert_eq!(ids.len(), categories::BUILTINS.len() + 1);
    let work_info = infos.iter().find(|info| info.id == id("work")).unwrap();
    assert!(!work_info.builtin);
    assert_eq!(work_info.domain_count, 1);
    let banking_info = infos.iter().find(|info| info.id == id("banking")).unwrap();
    assert!(banking_info.builtin);
    assert_eq!(banking_info.sources.len(), 3);
    assert_eq!(banking_info.app_count, 1);
}

#[test]
fn opt_in_categories_are_off_by_default() {
    for category in ["financial", "dating", "gambling"] {
        assert!(!Lists::default_enabled(&id(category)), "{category}");
    }
    for category in [
        "adult",
        "banking",
        "webmail",
        "medical",
        "password_managers",
    ] {
        assert!(Lists::default_enabled(&id(category)), "{category}");
    }
    for category in ["messengers", "invoices", "meeting", "my_own"] {
        assert!(Lists::default_enabled(&id(category)), "{category}");
    }
    assert_eq!(Lists::builtin_ids().len(), categories::BUILTINS.len());
}

#[test]
fn builtin_categories_are_unique_and_reference_bundled_lists() {
    let mut seen = BTreeSet::new();
    let mut used = BTreeSet::new();
    for builtin in categories::BUILTINS {
        assert!(seen.insert(builtin.id), "duplicate category {}", builtin.id);
        for list in builtin.lists {
            assert!(
                bundled::manifest_entry(list).is_some(),
                "{} uses unknown list {list}",
                builtin.id
            );
            used.insert(*list);
        }
    }
    let bundled: BTreeSet<_> = bundled::BLOBS.iter().map(|(id, _)| *id).collect();
    assert_eq!(used, bundled, "every bundled list belongs to a category");
}

#[test]
fn manifest_matches_files_and_blobs() {
    let manifest: toml::Table = toml::from_str(bundled::MANIFEST_TOML).unwrap();
    let recorded: BTreeMap<&str, &str> = manifest["list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|list| {
            (
                list["id"].as_str().unwrap(),
                list["sha256"].as_str().unwrap(),
            )
        })
        .collect();
    let blob_ids: BTreeSet<_> = bundled::BLOBS.iter().map(|(id, _)| *id).collect();
    assert_eq!(recorded.keys().copied().collect::<BTreeSet<_>>(), blob_ids);

    for (list_id, blob) in &bundled::BLOBS {
        let entry = bundled::manifest_entry(list_id).unwrap();
        let file = fs::read(lists_dir().join(format!("{list_id}.txt"))).unwrap();
        let sha256: String = Sha256::digest(&file)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            sha256, recorded[list_id],
            "{list_id}: sha256 differs, rerun the snapshot script"
        );
        assert_eq!(
            bundled::decompress(blob).as_bytes(),
            file,
            "{list_id}: stale blob"
        );
        assert_eq!(
            file.iter().filter(|b| **b == b'\n').count(),
            entry.entries as usize
        );
    }
}

#[test]
fn list_files_are_sorted_unique_lowercase_hostnames() {
    for entry in bundled::manifest() {
        let text = fs::read_to_string(lists_dir().join(format!("{}.txt", entry.id))).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert!(
            lines.windows(2).all(|pair| pair[0] < pair[1]),
            "{}: not sorted or has duplicates",
            entry.id
        );
        for line in lines {
            assert!(
                line.contains('.')
                    && line.bytes().all(|b| b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b'.' | b'-' | b'_')),
                "{}: bad entry {line:?}",
                entry.id
            );
        }
    }
}

#[test]
fn every_list_has_a_compatible_licence_and_an_attribution() {
    let licences = fs::read_to_string(lists_dir().join("LICENSES.md")).unwrap();
    for entry in bundled::manifest() {
        assert!(
            GPL3_COMPATIBLE.contains(&entry.license.as_str()),
            "{}: {}",
            entry.id,
            entry.license
        );
        assert!(
            licences.contains(&format!("`{}`", entry.id)),
            "{} missing from LICENSES.md",
            entry.id
        );
        assert!(entry.url.starts_with("https://"), "{}", entry.id);
        assert_eq!(entry.snapshot_date.len(), 10, "{}", entry.id);
        assert!(!entry.name.is_empty());
    }
}

#[test]
fn compressed_lists_fit_the_size_budget() {
    let total: usize = bundled::BLOBS.iter().map(|(_, blob)| blob.len()).sum();
    assert!(
        total <= MAX_COMPRESSED_BYTES,
        "bundled lists are {total} bytes compressed"
    );
}
