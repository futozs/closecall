//! Fixtures for tests in this and other crates. Enabled with the `testing` feature.

use std::collections::{HashMap, HashSet};

use crate::matcher::CategoryIndex;
use crate::{
    Action, AppRef, AppWindow, BrowserFamily, BrowserTab, Capability, CategoryDef, CategoryId,
    Dirty, DocKind, DocumentWindow, Handle, ProcessInfo, ProviderId, Support, Target, TargetId,
    TargetKind, TargetKindTag, Tri, UrlQuality,
};

pub const PROVIDER: &str = "fake";

#[derive(Debug, Default, Clone)]
pub struct StaticCategoryIndex {
    categories: HashMap<CategoryId, CategoryDef>,
    lists: HashMap<String, HashSet<String>>,
}

impl StaticCategoryIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_category(mut self, def: CategoryDef) -> Self {
        self.categories.insert(def.id.clone(), def);
        self
    }

    pub fn with_list(mut self, list_id: &str, domains: &[&str]) -> Self {
        self.lists.insert(
            list_id.to_owned(),
            domains.iter().map(|domain| (*domain).to_owned()).collect(),
        );
        self
    }
}

impl CategoryIndex for StaticCategoryIndex {
    fn category(&self, id: &CategoryId) -> Option<&CategoryDef> {
        self.categories.get(id)
    }

    fn list_contains(&self, list_id: &str, host: &str) -> bool {
        let Some(list) = self.lists.get(list_id) else {
            return false;
        };
        std::iter::once(host)
            .chain(host.match_indices('.').map(|(dot, _)| &host[dot + 1..]))
            .any(|candidate| list.contains(candidate))
    }
}

/// A Chrome tab, not private, audible or pinned, with id 0.
pub fn tab(url: &str, title: &str) -> Target {
    target(
        "com.google.Chrome",
        "Google Chrome",
        TargetKind::BrowserTab(BrowserTab {
            family: BrowserFamily::Chromium,
            window: Handle("window-1".to_owned()),
            tab: Handle(format!("tab-{url}")),
            title: title.to_owned(),
            url: Some(url.to_owned()),
            url_quality: UrlQuality::Exact,
            private: Tri::No,
            active: false,
            audible: Tri::No,
            pinned: Tri::No,
        }),
    )
}

/// A Preview document with id 0; the kind comes from the path's extension.
pub fn document(path: Option<&str>, title: &str, dirty: Dirty) -> Target {
    target(
        "com.apple.Preview",
        "Preview",
        TargetKind::DocumentWindow(DocumentWindow {
            window: Handle(format!("doc-{title}")),
            title: title.to_owned(),
            path: path.map(str::to_owned),
            doc_kind: path.map_or(DocKind::Other, doc_kind_of),
            dirty,
        }),
    )
}

pub fn app_window(app_id: &str, app_name: &str, title: &str) -> Target {
    target(
        app_id,
        app_name,
        TargetKind::AppWindow(AppWindow {
            window: Handle(format!("window-{app_id}-{title}")),
            title: title.to_owned(),
            minimized: false,
        }),
    )
}

pub fn process(app_id: &str, app_name: &str, exe: &str) -> Target {
    target(
        app_id,
        app_name,
        TargetKind::Process(ProcessInfo {
            pid: 100,
            exe: exe.to_owned(),
        }),
    )
}

fn target(app_id: &str, app_name: &str, kind: TargetKind) -> Target {
    Target {
        id: TargetId(0),
        provider: ProviderId(PROVIDER.to_owned()),
        app: AppRef {
            id: app_id.to_owned(),
            name: app_name.to_owned(),
            pid: Some(100),
        },
        first_seen_ms: 0,
        kind,
    }
}

fn doc_kind_of(path: &str) -> DocKind {
    let extension = path.rsplit_once('.').map_or("", |(_, ext)| ext);
    match extension.to_ascii_lowercase().as_str() {
        "pdf" => DocKind::Pdf,
        "png" | "jpg" | "jpeg" | "heic" | "gif" | "tiff" => DocKind::Image,
        "doc" | "docx" => DocKind::Word,
        "xls" | "xlsx" => DocKind::Excel,
        "ppt" | "pptx" => DocKind::PowerPoint,
        "pages" => DocKind::Pages,
        "key" => DocKind::Keynote,
        "numbers" => DocKind::Numbers,
        "txt" | "md" | "rtf" => DocKind::Text,
        _ => DocKind::Other,
    }
}

/// `private`, `audible` and `pinned` set the tab flag to `Yes` and panic on other targets.
pub trait TargetExt {
    fn with_id(self, id: u64) -> Self;
    fn with_app(self, app_id: &str, app_name: &str) -> Self;
    fn with_provider(self, provider: &str) -> Self;
    fn private(self) -> Self;
    fn audible(self) -> Self;
    fn pinned(self) -> Self;
}

impl TargetExt for Target {
    fn with_id(mut self, id: u64) -> Self {
        self.id = TargetId(id);
        self
    }

    fn with_app(mut self, app_id: &str, app_name: &str) -> Self {
        self.app.id = app_id.to_owned();
        self.app.name = app_name.to_owned();
        self
    }

    fn with_provider(mut self, provider: &str) -> Self {
        self.provider = ProviderId(provider.to_owned());
        self
    }

    fn private(self) -> Self {
        with_tab(self, |tab| tab.private = Tri::Yes)
    }

    fn audible(self) -> Self {
        with_tab(self, |tab| tab.audible = Tri::Yes)
    }

    fn pinned(self) -> Self {
        with_tab(self, |tab| tab.pinned = Tri::Yes)
    }
}

fn with_tab(mut target: Target, change: impl FnOnce(&mut BrowserTab)) -> Target {
    match &mut target.kind {
        TargetKind::BrowserTab(tab) => change(tab),
        other => panic!("expected a tab, got {other:?}"),
    }
    target
}

/// Every action on every target kind, fully supported by `provider`.
pub fn all_supported(provider: &str) -> Vec<Capability> {
    let kinds = [
        TargetKindTag::Tab,
        TargetKindTag::Document,
        TargetKindTag::AppWindow,
        TargetKindTag::Process,
    ];
    let actions = [
        Action::Quit,
        Action::Close,
        Action::Hide,
        Action::Minimize,
        Action::NotifyOnly,
    ];
    kinds
        .into_iter()
        .flat_map(|target_kind| {
            actions.into_iter().map(move |action| Capability {
                provider: ProviderId(provider.to_owned()),
                app: None,
                target_kind,
                action,
                support: Support::Full,
            })
        })
        .collect()
}
