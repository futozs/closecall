//! Document apps we can script, and the pure decisions around closing a document.

use closecall_core::{Dirty, DocKind, UnsavedPolicy};

use crate::keys;
use crate::script::DocDictionary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocApp {
    pub bundle_id: &'static str,
    pub name: &'static str,
    pub dict: DocDictionary,
    /// The dictionary's path property is plain text we can compare in a `whose`-style test.
    pub match_by_path: bool,
    /// Closing a file-backed document saves it in place, so `saving no` may not discard.
    pub autosaves: bool,
    /// Extension for `save … in` of an untitled document; `None` = we don't try.
    pub untitled_extension: Option<&'static str>,
    pub close_discards: bool,
}

const STANDARD: DocDictionary = DocDictionary {
    element: "document",
    path_property: "path",
    dirty_property: "modified",
    dirty_when: true,
};

const IWORK: DocDictionary = DocDictionary {
    element: "document",
    path_property: "file",
    dirty_property: "modified",
    dirty_when: true,
};

const fn office(element: &'static str) -> DocDictionary {
    DocDictionary {
        element,
        path_property: "full name",
        dirty_property: "saved",
        dirty_when: false,
    }
}

const fn app(bundle_id: &'static str, name: &'static str, dict: DocDictionary) -> DocApp {
    DocApp {
        bundle_id,
        name,
        dict,
        match_by_path: false,
        autosaves: false,
        untitled_extension: None,
        close_discards: false,
    }
}

// Office apps are sandboxed: saving an untitled document to a folder they were never
// granted pops a "Grant File Access" dialog, which must not appear on a shared screen.
pub(crate) const DOC_APPS: &[DocApp] = &[
    DocApp {
        match_by_path: true,
        autosaves: true,
        ..app("com.apple.Preview", "Preview", STANDARD)
    },
    DocApp {
        match_by_path: true,
        autosaves: true,
        untitled_extension: Some(""),
        ..app("com.apple.TextEdit", "TextEdit", STANDARD)
    },
    DocApp {
        autosaves: true,
        untitled_extension: Some("pages"),
        ..app("com.apple.iWork.Pages", "Pages", IWORK)
    },
    DocApp {
        autosaves: true,
        untitled_extension: Some("key"),
        ..app("com.apple.iWork.Keynote", "Keynote", IWORK)
    },
    DocApp {
        autosaves: true,
        untitled_extension: Some("numbers"),
        ..app("com.apple.iWork.Numbers", "Numbers", IWORK)
    },
    app("com.microsoft.Word", "Microsoft Word", office("document")),
    app("com.microsoft.Excel", "Microsoft Excel", office("workbook")),
    DocApp {
        close_discards: true,
        ..app(
            "com.microsoft.Powerpoint",
            "Microsoft PowerPoint",
            office("presentation"),
        )
    },
];

pub(crate) fn doc_app(bundle_id: &str) -> Option<&'static DocApp> {
    DOC_APPS
        .iter()
        .find(|app| app.bundle_id.eq_ignore_ascii_case(bundle_id))
}

pub(crate) fn doc_kind(path: &str) -> DocKind {
    let name = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path);
    let Some((_, ext)) = name.rsplit_once('.') else {
        return DocKind::Other;
    };
    match ext.to_ascii_lowercase().as_str() {
        "pdf" => DocKind::Pdf,
        "png" | "jpg" | "jpeg" | "gif" | "heic" | "heif" | "tif" | "tiff" | "bmp" | "webp"
        | "svg" | "psd" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "raw" => DocKind::Image,
        "doc" | "docx" | "docm" | "dot" | "dotx" | "odt" => DocKind::Word,
        "xls" | "xlsx" | "xlsm" | "xlsb" | "xlt" | "xltx" | "ods" | "csv" => DocKind::Excel,
        "ppt" | "pptx" | "pptm" | "pps" | "ppsx" | "pot" | "potx" | "odp" => DocKind::PowerPoint,
        "pages" => DocKind::Pages,
        "key" => DocKind::Keynote,
        "numbers" => DocKind::Numbers,
        "txt" | "text" | "md" | "markdown" | "rtf" | "rtfd" | "log" | "json" | "xml" | "yaml"
        | "yml" | "toml" | "tex" => DocKind::Text,
        _ => DocKind::Other,
    }
}

/// Percent-decodes an `AXDocument` value (`file:///Users/a/My%20Doc.pdf`) into a path.
pub(crate) fn file_url_to_path(url: &str) -> Option<String> {
    let rest = url.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = bytes
                .get(i + 1..i + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    let path = String::from_utf8(out).ok()?;
    let trimmed = path.trim_end_matches('/');
    Some(if trimmed.is_empty() {
        path
    } else {
        trimmed.to_owned()
    })
}

/// Office reports `full name` as a POSIX path or an HFS path (`Macintosh HD:Users:a:x.docx`)
/// depending on version, and the boot volume's name is unknown here, so HFS paths are
/// compared by everything after the volume.
pub(crate) fn same_file(posix: &str, reported: &str) -> bool {
    if reported.starts_with('/') {
        return posix.trim_end_matches('/') == reported.trim_end_matches('/');
    }
    let Some((_, rest)) = reported.split_once(':') else {
        return false;
    };
    let rest = rest.trim_end_matches(':').replace(':', "/");
    !rest.is_empty() && posix.trim_end_matches('/').ends_with(&format!("/{rest}"))
}

/// Turns a reported path into a POSIX path; `volume_exists` decides whether an HFS path's
/// first component is a mounted volume other than the boot disk.
pub(crate) fn posix_path(reported: &str, volume_exists: impl Fn(&str) -> bool) -> Option<String> {
    if reported.starts_with('/') {
        return Some(reported.to_owned());
    }
    let (volume, rest) = reported.split_once(':')?;
    let rest = rest.trim_end_matches(':').replace(':', "/");
    if volume_exists(volume) {
        Some(format!("/Volumes/{volume}/{rest}"))
    } else {
        Some(format!("/{rest}"))
    }
}

/// Window titles decorate the document name ("Report.docx — Edited", "Deck - Compatibility
/// Mode") or drop its extension, so a prefix match on a word boundary is accepted.
pub(crate) fn title_matches(title: &str, doc_name: &str) -> bool {
    if doc_name.is_empty() {
        return false;
    }
    let stem = doc_name
        .rsplit_once('.')
        .map_or(doc_name, |(stem, _)| stem)
        .trim();
    [doc_name, stem].into_iter().any(|name| {
        !name.is_empty()
            && title.strip_prefix(name).is_some_and(|rest| {
                rest.is_empty() || rest.starts_with([' ', '-', '—', '–', '.', '(', '['])
            })
    })
}

/// Dirty state of a window from the facts we could read. AX `AXEdited` is only trusted when
/// true, because autosaving apps keep it clear while holding edits.
pub(crate) fn dirty_state(
    script_dirty: Option<bool>,
    ax_edited: Option<bool>,
    document_app: bool,
) -> Dirty {
    match (script_dirty, ax_edited) {
        (Some(true), _) | (_, Some(true)) => Dirty::Dirty,
        (Some(false), _) => Dirty::Clean,
        _ if document_app => Dirty::Unknown,
        _ => Dirty::Clean,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CloseRoute {
    /// Save through the dictionary, then close (PowerPoint: `save`, check, `close`).
    ScriptSave,
    ScriptDiscard,
    ScriptSaveUntitled,
    /// Press the close button; a sheet that shows up is answered by `sheet::choose`.
    Button,
    Conceal(&'static str),
    Skip(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CloseFacts {
    pub dirty: Dirty,
    pub has_path: bool,
    /// The app is in `DOC_APPS`, we have Automation for it, and the probe found exactly
    /// this document.
    pub scriptable: bool,
    pub autosaves: bool,
    pub saves_untitled: bool,
}

pub(crate) fn close_route(policy: UnsavedPolicy, facts: CloseFacts) -> CloseRoute {
    if facts.dirty == Dirty::Clean {
        return CloseRoute::Button;
    }
    match policy {
        UnsavedPolicy::Hide => CloseRoute::Conceal(keys::UNSAVED_HIDDEN),
        UnsavedPolicy::SkipAndNotify => CloseRoute::Skip(keys::UNSAVED_SKIPPED),
        UnsavedPolicy::SaveThenClose if facts.has_path => {
            if facts.scriptable {
                CloseRoute::ScriptSave
            } else {
                CloseRoute::Button
            }
        }
        UnsavedPolicy::SaveThenClose if facts.scriptable && facts.saves_untitled => {
            CloseRoute::ScriptSaveUntitled
        }
        UnsavedPolicy::SaveThenClose => CloseRoute::Conceal(keys::UNTITLED_HIDDEN),
        UnsavedPolicy::Discard if !facts.has_path => {
            CloseRoute::Conceal(keys::DISCARD_UNTITLED_HIDDEN)
        }
        UnsavedPolicy::Discard if facts.autosaves => {
            CloseRoute::Conceal(keys::AUTOSAVE_DISCARD_HIDDEN)
        }
        UnsavedPolicy::Discard if facts.scriptable => CloseRoute::ScriptDiscard,
        UnsavedPolicy::Discard => CloseRoute::Button,
    }
}

pub(crate) fn safe_file_stem(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | ':' | '\\' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').trim();
    let cut: String = cleaned.chars().take(80).collect();
    if cut.is_empty() {
        "Untitled".to_owned()
    } else {
        cut
    }
}

/// `<title>-<YYYY-MM-DD-HHMMSS>[.ext]`.
pub(crate) fn untitled_file_name(title: &str, local: [u32; 6], extension: &str) -> String {
    let [y, mo, d, h, mi, s] = local;
    let stem = safe_file_stem(title);
    let base = format!("{stem}-{y:04}-{mo:02}-{d:02}-{h:02}{mi:02}{s:02}");
    if extension.is_empty() {
        base
    } else {
        format!("{base}.{extension}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_extensions_to_doc_kinds() {
        assert_eq!(doc_kind("/Users/a/Számla 2026.PDF"), DocKind::Pdf);
        assert_eq!(doc_kind("/a/scan.heic"), DocKind::Image);
        assert_eq!(doc_kind("/a/b.docx"), DocKind::Word);
        assert_eq!(doc_kind("/a/b.xlsx"), DocKind::Excel);
        assert_eq!(doc_kind("/a/b.csv"), DocKind::Excel);
        assert_eq!(doc_kind("/a/b.pptx"), DocKind::PowerPoint);
        assert_eq!(doc_kind("/a/Report.pages/"), DocKind::Pages);
        assert_eq!(doc_kind("/a/b.key"), DocKind::Keynote);
        assert_eq!(doc_kind("/a/b.numbers"), DocKind::Numbers);
        assert_eq!(doc_kind("/a/notes.md"), DocKind::Text);
        assert_eq!(doc_kind("/a/archive.zip"), DocKind::Other);
        assert_eq!(doc_kind("/a/Makefile"), DocKind::Other);
        assert_eq!(doc_kind("/a.b/noext"), DocKind::Other);
    }

    #[test]
    fn decodes_file_urls() {
        assert_eq!(
            file_url_to_path("file:///Users/a/My%20Docs/Sz%C3%A1mla.pdf").as_deref(),
            Some("/Users/a/My Docs/Számla.pdf")
        );
        assert_eq!(
            file_url_to_path("file://localhost/tmp/x.txt").as_deref(),
            Some("/tmp/x.txt")
        );
        assert_eq!(
            file_url_to_path("file:///Users/a/Deck.key/").as_deref(),
            Some("/Users/a/Deck.key")
        );
        assert_eq!(file_url_to_path("file:///").as_deref(), Some("/"));
        assert_eq!(
            file_url_to_path("file:///a/100%").as_deref(),
            Some("/a/100%")
        );
        assert_eq!(file_url_to_path("https://example.com/x"), None);
    }

    #[test]
    fn compares_posix_and_hfs_paths() {
        assert!(same_file("/Users/a/x.docx", "/Users/a/x.docx"));
        assert!(same_file("/Users/a/x.docx", "Macintosh HD:Users:a:x.docx"));
        assert!(same_file("/Volumes/USB/x.docx", "USB:x.docx"));
        assert!(!same_file("/Users/a/x.docx", "Macintosh HD:Users:b:x.docx"));
        assert!(!same_file("/Users/a/x.docx", "Document1"));
    }

    #[test]
    fn converts_hfs_paths() {
        let only_usb = |v: &str| v == "USB";
        assert_eq!(
            posix_path("Macintosh HD:Users:a:x.docx", only_usb).as_deref(),
            Some("/Users/a/x.docx")
        );
        assert_eq!(
            posix_path("USB:x.docx", only_usb).as_deref(),
            Some("/Volumes/USB/x.docx")
        );
        assert_eq!(posix_path("/tmp/x", only_usb).as_deref(), Some("/tmp/x"));
        assert_eq!(posix_path("Document1", only_usb), None);
    }

    #[test]
    fn matches_decorated_titles() {
        assert!(title_matches("Report.docx", "Report.docx"));
        assert!(title_matches("Report.docx — Edited", "Report.docx"));
        assert!(title_matches("Report - Compatibility Mode", "Report.docx"));
        assert!(title_matches("Report", "Report.docx"));
        assert!(!title_matches("Reports 2026", "Report.docx"));
        assert!(!title_matches("Anything", ""));
    }

    #[test]
    fn trusts_ax_only_when_it_says_edited() {
        assert_eq!(dirty_state(None, Some(true), false), Dirty::Dirty);
        assert_eq!(dirty_state(Some(true), Some(false), true), Dirty::Dirty);
        assert_eq!(dirty_state(Some(false), Some(false), true), Dirty::Clean);
        assert_eq!(dirty_state(None, Some(false), true), Dirty::Unknown);
        assert_eq!(dirty_state(None, None, true), Dirty::Unknown);
        assert_eq!(dirty_state(None, Some(false), false), Dirty::Clean);
    }

    fn facts(dirty: Dirty, has_path: bool, scriptable: bool) -> CloseFacts {
        CloseFacts {
            dirty,
            has_path,
            scriptable,
            autosaves: false,
            saves_untitled: false,
        }
    }

    #[test]
    fn clean_documents_close_through_the_button() {
        for policy in [
            UnsavedPolicy::Hide,
            UnsavedPolicy::SkipAndNotify,
            UnsavedPolicy::SaveThenClose,
            UnsavedPolicy::Discard,
        ] {
            assert_eq!(
                close_route(policy, facts(Dirty::Clean, true, true)),
                CloseRoute::Button
            );
        }
    }

    #[test]
    fn unknown_is_handled_like_dirty() {
        let unknown = facts(Dirty::Unknown, true, false);
        assert_eq!(
            close_route(UnsavedPolicy::Hide, unknown),
            CloseRoute::Conceal(keys::UNSAVED_HIDDEN)
        );
        assert_eq!(
            close_route(UnsavedPolicy::SkipAndNotify, unknown),
            CloseRoute::Skip(keys::UNSAVED_SKIPPED)
        );
    }

    #[test]
    fn save_then_close_prefers_the_script_and_never_drives_a_save_panel() {
        assert_eq!(
            close_route(
                UnsavedPolicy::SaveThenClose,
                facts(Dirty::Dirty, true, true)
            ),
            CloseRoute::ScriptSave
        );
        assert_eq!(
            close_route(
                UnsavedPolicy::SaveThenClose,
                facts(Dirty::Dirty, true, false)
            ),
            CloseRoute::Button
        );
        assert_eq!(
            close_route(
                UnsavedPolicy::SaveThenClose,
                facts(Dirty::Dirty, false, true)
            ),
            CloseRoute::Conceal(keys::UNTITLED_HIDDEN)
        );
        let untitled_ok = CloseFacts {
            saves_untitled: true,
            ..facts(Dirty::Dirty, false, true)
        };
        assert_eq!(
            close_route(UnsavedPolicy::SaveThenClose, untitled_ok),
            CloseRoute::ScriptSaveUntitled
        );
    }

    #[test]
    fn discard_needs_a_file_and_a_non_autosaving_app() {
        assert_eq!(
            close_route(UnsavedPolicy::Discard, facts(Dirty::Dirty, false, true)),
            CloseRoute::Conceal(keys::DISCARD_UNTITLED_HIDDEN)
        );
        let autosaving = CloseFacts {
            autosaves: true,
            ..facts(Dirty::Dirty, true, true)
        };
        assert_eq!(
            close_route(UnsavedPolicy::Discard, autosaving),
            CloseRoute::Conceal(keys::AUTOSAVE_DISCARD_HIDDEN)
        );
        assert_eq!(
            close_route(UnsavedPolicy::Discard, facts(Dirty::Dirty, true, true)),
            CloseRoute::ScriptDiscard
        );
        assert_eq!(
            close_route(UnsavedPolicy::Discard, facts(Dirty::Unknown, true, false)),
            CloseRoute::Button
        );
    }

    #[test]
    fn builds_untitled_file_names() {
        let at = [2026, 11, 16, 9, 5, 7];
        assert_eq!(
            untitled_file_name("Névtelen", at, ""),
            "Névtelen-2026-11-16-090507"
        );
        assert_eq!(
            untitled_file_name("a/b: c", at, "pages"),
            "a-b- c-2026-11-16-090507.pages"
        );
        assert_eq!(
            untitled_file_name("  ", at, "key"),
            "Untitled-2026-11-16-090507.key"
        );
        assert_eq!(safe_file_stem("..hidden"), "hidden");
        assert_eq!(safe_file_stem(&"x".repeat(200)).chars().count(), 80);
    }

    #[test]
    fn doc_app_table_is_consistent() {
        let ppt = doc_app("com.microsoft.powerpoint").unwrap();
        assert!(ppt.close_discards);
        assert_eq!(ppt.dict.element, "presentation");
        assert!(!ppt.autosaves);
        let text_edit = doc_app("com.apple.TextEdit").unwrap();
        assert!(text_edit.autosaves && text_edit.match_by_path);
        for app in DOC_APPS {
            assert!(
                !(app.match_by_path && app.dict.path_property != "path"),
                "{} compares a non-text path property",
                app.bundle_id
            );
            assert!(
                app.untitled_extension.is_none() || !app.bundle_id.starts_with("com.microsoft"),
                "Office must not save untitled documents"
            );
        }
        assert!(doc_app("com.example.nope").is_none());
    }
}
