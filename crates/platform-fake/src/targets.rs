use closecall_core::{
    AppRef, AppWindow, BrowserFamily, BrowserTab, Dirty, DocKind, DocumentWindow, Handle,
    ProcessInfo, ProviderId, Target, TargetId, TargetKind, Tri, UrlQuality,
};

use crate::{APPS, CHROMIUM, DOCUMENTS, SAFARI};

struct Browser {
    provider: &'static str,
    app: (&'static str, &'static str),
    family: BrowserFamily,
    window: &'static str,
    private: Tri,
}

const CHROME: Browser = Browser {
    provider: CHROMIUM,
    app: ("com.google.Chrome", "Google Chrome"),
    family: BrowserFamily::Chromium,
    window: "chrome-window-1",
    private: Tri::No,
};

const CHROME_INCOGNITO: Browser = Browser {
    window: "chrome-window-incognito",
    private: Tri::Yes,
    ..CHROME
};

/// Safari cannot tell private windows apart, so its tabs are `private: Unknown`.
const SAFARI_BROWSER: Browser = Browser {
    provider: SAFARI,
    app: ("com.apple.Safari", "Safari"),
    family: BrowserFamily::Safari,
    window: "safari-window-1",
    private: Tri::Unknown,
};

/// The key is also the target's handle, so the provider finds an item again from a `Target`.
pub fn chrome_tab(key: &str, url: &str, title: &str) -> Target {
    tab(&CHROME, key, url, title)
}

pub fn chrome_incognito_tab(key: &str, url: &str, title: &str) -> Target {
    tab(&CHROME_INCOGNITO, key, url, title)
}

pub fn safari_tab(key: &str, url: &str, title: &str) -> Target {
    tab(&SAFARI_BROWSER, key, url, title)
}

/// `path: None` is an untitled document; the kind comes from the extension.
pub fn document(
    key: &str,
    app: (&str, &str),
    path: Option<&str>,
    title: &str,
    dirty: Dirty,
) -> Target {
    target(
        DOCUMENTS,
        app,
        TargetKind::DocumentWindow(DocumentWindow {
            window: Handle(key.to_owned()),
            title: title.to_owned(),
            path: path.map(str::to_owned),
            doc_kind: path.map_or(DocKind::Other, doc_kind_of),
            dirty,
        }),
    )
}

pub fn app_window(key: &str, app: (&str, &str), title: &str) -> Target {
    target(
        APPS,
        app,
        TargetKind::AppWindow(AppWindow {
            window: Handle(key.to_owned()),
            title: title.to_owned(),
            minimized: false,
        }),
    )
}

pub fn process(app: (&str, &str), pid: u32, exe: &str) -> Target {
    let mut target = target(
        APPS,
        app,
        TargetKind::Process(ProcessInfo {
            pid,
            exe: exe.to_owned(),
        }),
    );
    target.app.pid = Some(pid);
    target
}

/// Chrome: an OTP netbank tab, an adult site, Gmail, an incognito tab and a Meet call.
/// Safari: an EESZT tab. Preview: an invoice with unsaved changes. Word: a payslip.
/// Apps: 1Password and Signal.
pub fn demo_inventory() -> Vec<Target> {
    vec![
        chrome_tab(
            "chrome-otp",
            "https://netbank.otpbank.hu/portal/hu/Szamlak",
            "OTP Bank – Internetbank",
        ),
        chrome_tab("chrome-adult", "https://www.pornhub.com/", "Pornhub"),
        chrome_tab(
            "chrome-gmail",
            "https://mail.google.com/mail/u/0/#inbox",
            "Beérkező levelek (3) – Gmail",
        ),
        chrome_incognito_tab(
            "chrome-incognito",
            "https://www.youtube.com/results?search_query=ajandek",
            "ajándék – YouTube",
        ),
        chrome_tab(
            "chrome-meet",
            "https://meet.google.com/abc-defg-hij",
            "Meet – abc-defg-hij",
        ),
        safari_tab(
            "safari-eeszt",
            "https://www.eeszt.gov.hu/hu/web/guest/lakossagi-portal",
            "EESZT – Lakossági portál",
        ),
        document(
            "preview-invoice",
            ("com.apple.Preview", "Preview"),
            Some("/Users/demo/Documents/Szamlak/szamla-2026-10.pdf"),
            "szamla-2026-10.pdf",
            Dirty::Dirty,
        ),
        document(
            "word-payslip",
            ("com.microsoft.Word", "Microsoft Word"),
            Some("/Users/demo/Documents/berjegyzek-2026-10.docx"),
            "Bérjegyzék 2026. október.docx",
            Dirty::Clean,
        ),
        app_window(
            "app-1password",
            ("com.1password.1password", "1Password"),
            "1Password",
        ),
        app_window(
            "app-signal",
            ("org.whispersystems.signal-desktop", "Signal"),
            "Signal",
        ),
    ]
}

pub(crate) fn key_of(target: &Target) -> String {
    match &target.kind {
        TargetKind::BrowserTab(tab) => tab.tab.0.clone(),
        TargetKind::DocumentWindow(doc) => doc.window.0.clone(),
        TargetKind::AppWindow(window) => window.window.0.clone(),
        TargetKind::Process(process) => format!("pid-{}", process.pid),
    }
}

fn tab(browser: &Browser, key: &str, url: &str, title: &str) -> Target {
    target(
        browser.provider,
        browser.app,
        TargetKind::BrowserTab(BrowserTab {
            family: browser.family,
            window: Handle(browser.window.to_owned()),
            tab: Handle(key.to_owned()),
            title: title.to_owned(),
            url: Some(url.to_owned()),
            url_quality: UrlQuality::Exact,
            private: browser.private,
            active: false,
            audible: Tri::No,
            pinned: Tri::No,
        }),
    )
}

/// Like the real adapters, the provider leaves `id` and `first_seen_ms` to the engine.
fn target(provider: &str, (app_id, app_name): (&str, &str), kind: TargetKind) -> Target {
    Target {
        id: TargetId(0),
        provider: ProviderId(provider.to_owned()),
        app: AppRef {
            id: app_id.to_owned(),
            name: app_name.to_owned(),
            pid: Some(500),
        },
        first_seen_ms: 0,
        kind,
    }
}

fn doc_kind_of(path: &str) -> DocKind {
    let extension = path.rsplit_once('.').map_or("", |(_, ext)| ext);
    match extension.to_ascii_lowercase().as_str() {
        "pdf" => DocKind::Pdf,
        "png" | "jpg" | "jpeg" | "heic" => DocKind::Image,
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
