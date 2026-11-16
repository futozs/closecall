use super::*;
use crate::testing::{StaticCategoryIndex, TargetExt, app_window, document, process, tab};
use crate::{Dirty, DocKind};

const HOME: &str = "/Users/anna";

fn index() -> StaticCategoryIndex {
    StaticCategoryIndex::new()
        .with_category(CategoryDef {
            id: CategoryId("banking".to_owned()),
            domains: vec!["otpbank.hu".to_owned()],
            lists: vec!["ut1-bank".to_owned()],
            title_keywords: vec!["Internetbank".to_owned()],
            ..CategoryDef::default()
        })
        .with_category(CategoryDef {
            id: CategoryId("password_managers".to_owned()),
            apps: vec![AppSelector::BundleId("com.1password.1password".to_owned())],
            ..CategoryDef::default()
        })
        .with_category(CategoryDef {
            id: CategoryId("invoices".to_owned()),
            title_keywords: vec!["számla".to_owned()],
            file_globs: vec!["**/*bérjegyzék*".to_owned()],
            ..CategoryDef::default()
        })
        .with_list("ut1-bank", &["kh.hu", "secure.example-bank.com"])
}

fn eval_with(index: &StaticCategoryIndex, matcher: &Matcher, target: &Target) -> Option<Evidence> {
    let ctx = MatchContext {
        categories: index,
        home: HOME,
    };
    evaluate(matcher, target, &ctx)
}

fn eval(matcher: &Matcher, target: &Target) -> Option<Evidence> {
    eval_with(&index(), matcher, target)
}

fn hits(matcher: &Matcher, target: &Target) -> bool {
    eval(matcher, target).is_some()
}

fn domain(d: &str) -> Matcher {
    Matcher::Domain(d.to_owned())
}

fn title(keyword: &str) -> Matcher {
    Matcher::TitleContains(keyword.to_owned())
}

fn category(id: &str) -> Matcher {
    Matcher::Category(CategoryId(id.to_owned()))
}

#[test]
fn domain_matches_itself_and_subdomains_on_label_boundaries() {
    let bank = domain("bank.hu");
    assert!(hits(&bank, &tab("https://bank.hu/", "")));
    assert!(hits(&bank, &tab("https://net.bank.hu/login", "")));
    assert!(hits(&bank, &tab("https://a.b.bank.hu", "")));
    assert!(!hits(&bank, &tab("https://notbank.hu/", "")));
    assert!(!hits(&bank, &tab("https://bank.hu.evil.com/", "")));
    assert!(!hits(&bank, &tab("https://bank.hungary.com/", "")));
}

#[test]
fn domain_ignores_case_trailing_dots_and_wildcard_prefix() {
    assert!(hits(&domain("Bank.HU"), &tab("https://NET.Bank.hu/", "")));
    assert!(hits(&domain("bank.hu."), &tab("https://bank.hu./", "")));
    assert!(hits(
        &domain("*.zoom.us"),
        &tab("https://us02web.zoom.us/j/1", "")
    ));
    assert!(hits(&domain("*.zoom.us"), &tab("https://zoom.us/", "")));
}

#[test]
fn domain_compares_internationalized_names_in_punycode() {
    let idn = domain("bücher.de");
    assert!(hits(&idn, &tab("https://www.bücher.de/", "")));
    assert!(hits(&idn, &tab("https://xn--bcher-kva.de/", "")));
    assert!(hits(
        &domain("xn--bcher-kva.de"),
        &tab("https://BÜCHER.de/", "")
    ));
    assert!(!hits(&idn, &tab("https://bucher.de/", "")));
}

#[test]
fn domain_on_ip_hosts_is_exact() {
    assert!(hits(
        &domain("192.168.1.1"),
        &tab("http://192.168.1.1/admin", "")
    ));
    assert!(!hits(
        &domain("168.1.1"),
        &tab("http://192.168.1.1/admin", "")
    ));
}

#[test]
fn domain_needs_a_url() {
    let mut no_url = tab("https://bank.hu/", "bank.hu");
    if let TargetKind::BrowserTab(t) = &mut no_url.kind {
        t.url = None;
    }
    assert!(!hits(&domain("bank.hu"), &no_url));
    assert!(!hits(&domain("bank.hu"), &tab("not a url", "")));
    assert!(!hits(
        &domain("bank.hu"),
        &document(Some("/tmp/bank.hu.pdf"), "bank.hu", Dirty::Clean)
    ));
}

#[test]
fn domain_evidence_names_the_rule_domain() {
    assert_eq!(
        eval(
            &domain("otpbank.hu"),
            &tab("https://netbank.otpbank.hu/", "")
        ),
        Some(Evidence::Domain {
            domain: "otpbank.hu".to_owned()
        })
    );
}

#[test]
fn url_glob_matches_the_whole_url() {
    let gmail = Matcher::UrlGlob("https://mail.google.com/mail/u/1/*".to_owned());
    assert!(hits(
        &gmail,
        &tab("https://mail.google.com/mail/u/1/#inbox", "")
    ));
    assert!(!hits(
        &gmail,
        &tab("https://mail.google.com/mail/u/0/#inbox", "")
    ));
    let suffix = Matcher::UrlGlob("*google.com".to_owned());
    assert!(!hits(&suffix, &tab("https://google.com/x", "")));
    assert_eq!(
        eval(&gmail, &tab("https://MAIL.google.com/mail/u/1/", "")),
        Some(Evidence::UrlGlob {
            glob: "https://mail.google.com/mail/u/1/*".to_owned()
        })
    );
}

#[test]
fn url_glob_also_sees_the_normalized_url() {
    let glob = Matcher::UrlGlob("https://example.com/".to_owned());
    assert!(hits(&glob, &tab("https://EXAMPLE.com", "")));
    let idn = Matcher::UrlGlob("https://bücher.de/*".to_owned());
    assert!(hits(&idn, &tab("https://bücher.de/konto", "")));
}

#[test]
fn url_glob_ignores_targets_without_url() {
    let glob = Matcher::UrlGlob("*".to_owned());
    assert!(hits(&glob, &tab("https://a.hu/", "")));
    assert!(!hits(&glob, &app_window("com.apple.Notes", "Notes", "x")));
}

#[test]
fn wildcard_semantics() {
    assert!(wildcard_match("a*b*c", "aXXbYYc"));
    assert!(wildcard_match("a*b*c", "abc"));
    assert!(wildcard_match("*", ""));
    assert!(wildcard_match("**", "x"));
    assert!(wildcard_match("a?b", "a?b"));
    assert!(!wildcard_match("a?b", "axb"));
    assert!(!wildcard_match("a*b", "ab c"));
    assert!(wildcard_match("*ő*", "árvíztűrő tükör"));
    assert!(!wildcard_match("", "x"));
}

#[test]
fn title_contains_folds_case_and_diacritics() {
    let preview = document(None, "Számla_2026_10.pdf", Dirty::Clean);
    assert!(hits(&title("szamla"), &preview));
    assert!(hits(&title("SZÁMLA"), &preview));
    assert!(hits(
        &title("BÉRJEGYZÉK"),
        &tab("https://x.hu/", "berjegyzek oktober")
    ));
    assert!(hits(
        &title("netbank"),
        &app_window("com.example", "Example", "OTP NetBank")
    ));
    assert_eq!(
        eval(&title("Szamla"), &preview),
        Some(Evidence::TitleContains {
            keyword: "Szamla".to_owned()
        })
    );
}

#[test]
fn title_contains_never_matches_processes_or_empty_keywords() {
    assert!(!hits(
        &title("slack"),
        &process(
            "com.tinyspeck.slackmacgap",
            "Slack",
            "/Applications/Slack.app"
        )
    ));
    assert!(!hits(&title(""), &tab("https://a.hu/", "anything")));
    assert!(!hits(&title("\u{301}"), &tab("https://a.hu/", "anything")));
}

#[test]
fn app_bundle_and_desktop_ids_compare_exactly() {
    let window = app_window("com.1password.1password", "1Password", "Vault");
    assert!(app_matches(
        &AppSelector::BundleId("com.1password.1password".to_owned()),
        &window
    ));
    assert!(!app_matches(
        &AppSelector::BundleId("com.1password".to_owned()),
        &window
    ));
    let signal = app_window("org.signal.Signal.desktop", "Signal", "");
    assert!(app_matches(
        &AppSelector::DesktopId("org.signal.Signal.desktop".to_owned()),
        &signal
    ));
}

#[test]
fn app_exe_is_case_insensitive_and_checks_file_names() {
    let windows = process(
        "1Password.exe",
        "1Password",
        r"C:\Program Files\1Password\app\1Password.EXE",
    );
    assert!(app_matches(
        &AppSelector::Exe("1password.exe".to_owned()),
        &windows
    ));
    let mac = process(
        "com.tinyspeck.slackmacgap",
        "Slack",
        "/Applications/Slack.app/Contents/MacOS/Slack",
    );
    assert!(app_matches(&AppSelector::Exe("slack".to_owned()), &mac));
    assert!(app_matches(
        &AppSelector::Exe("/Applications/Slack.app/Contents/MacOS/Slack".to_owned()),
        &mac
    ));
    assert!(!app_matches(&AppSelector::Exe("lack".to_owned()), &mac));
}

#[test]
fn app_name_compares_folded_names() {
    let messages = app_window("com.apple.MobileSMS", "Üzenetek", "");
    assert!(app_matches(
        &AppSelector::Name("uzenetek".to_owned()),
        &messages
    ));
    assert!(!app_matches(
        &AppSelector::Name("uzenet".to_owned()),
        &messages
    ));
    assert_eq!(
        eval(
            &Matcher::App(AppSelector::Name("ÜZENETEK".to_owned())),
            &messages
        ),
        Some(Evidence::App {
            app: "Üzenetek".to_owned()
        })
    );
}

#[test]
fn file_glob_expands_home_and_ignores_case() {
    let bank = Matcher::FileGlob("~/Documents/Bank/**".to_owned());
    let statement = document(
        Some("/Users/anna/Documents/Bank/2026/Kivonat.PDF"),
        "Kivonat.PDF",
        Dirty::Clean,
    );
    assert!(hits(&bank, &statement));
    assert!(!hits(
        &bank,
        &document(
            Some("/Users/anna/Documents/Other/x.pdf"),
            "x.pdf",
            Dirty::Clean
        )
    ));
    assert!(!hits(
        &Matcher::FileGlob("~anna/Documents/**".to_owned()),
        &statement
    ));
}

#[test]
fn file_glob_folds_diacritics_and_normalization() {
    let payslip = Matcher::FileGlob("**/*bérjegyzék*.pdf".to_owned());
    let nfc = "/Users/anna/Letöltések/Bérjegyzék_2026_10.PDF";
    let nfd = "/Users/anna/Leto\u{308}lte\u{301}sek/Be\u{301}rjegyze\u{301}k_2026_10.pdf";
    assert!(hits(&payslip, &document(Some(nfc), "", Dirty::Clean)));
    assert!(hits(&payslip, &document(Some(nfd), "", Dirty::Clean)));
    assert!(hits(
        &Matcher::FileGlob("**/*berjegyzek*".to_owned()),
        &document(Some(nfc), "", Dirty::Clean)
    ));
}

#[test]
fn file_glob_needs_a_document_path() {
    let any = Matcher::FileGlob("**".to_owned());
    assert!(!hits(&any, &document(None, "Untitled", Dirty::Dirty)));
    assert!(!hits(&any, &tab("file:///Users/anna/a.pdf", "a.pdf")));
}

#[test]
fn doc_kind_matches_documents_only() {
    let pdf = Matcher::DocKind(DocKind::Pdf);
    assert_eq!(
        eval(&pdf, &document(Some("/a/b.pdf"), "b.pdf", Dirty::Clean)),
        Some(Evidence::DocKind {
            doc_kind: DocKind::Pdf
        })
    );
    assert!(!hits(
        &pdf,
        &document(Some("/a/b.docx"), "b.docx", Dirty::Clean)
    ));
    assert!(!hits(&pdf, &tab("https://a.hu/b.pdf", "b.pdf")));
}

#[test]
fn private_needs_a_definitely_private_tab() {
    let private = tab("https://a.hu/", "").private();
    assert_eq!(eval(&Matcher::Private, &private), Some(Evidence::Private));
    assert!(!hits(&Matcher::Private, &tab("https://a.hu/", "")));
    let mut unknown = tab("https://a.hu/", "");
    if let TargetKind::BrowserTab(t) = &mut unknown.kind {
        t.private = Tri::Unknown;
    }
    assert!(!hits(&Matcher::Private, &unknown));
    assert!(!hits(
        &Matcher::Private,
        &app_window("com.apple.Safari", "Safari", "Private Browsing")
    ));
}

fn via(id: &str, inner: Evidence) -> Option<Evidence> {
    Some(Evidence::Category {
        id: CategoryId(id.to_owned()),
        via: Box::new(inner),
    })
}

#[test]
fn category_checks_inline_domains_first() {
    assert_eq!(
        eval(
            &category("banking"),
            &tab("https://netbank.otpbank.hu/", "")
        ),
        via(
            "banking",
            Evidence::Domain {
                domain: "otpbank.hu".to_owned()
            }
        )
    );
}

#[test]
fn category_reports_the_listed_entry_not_the_host() {
    assert_eq!(
        eval(&category("banking"), &tab("https://e-bank.kh.hu/", "")),
        via(
            "banking",
            Evidence::Domain {
                domain: "kh.hu".to_owned()
            }
        )
    );
    assert_eq!(
        eval(
            &category("banking"),
            &tab("https://login.secure.example-bank.com/", "")
        ),
        via(
            "banking",
            Evidence::Domain {
                domain: "secure.example-bank.com".to_owned()
            }
        )
    );
    assert!(!hits(
        &category("banking"),
        &tab("https://example-bank.com/", "")
    ));
}

#[test]
fn category_matches_apps_keywords_and_files() {
    assert_eq!(
        eval(
            &category("password_managers"),
            &app_window("com.1password.1password", "1Password", "Private vault")
        ),
        via(
            "password_managers",
            Evidence::App {
                app: "1Password".to_owned()
            }
        )
    );
    assert_eq!(
        eval(
            &category("banking"),
            &tab("https://example.com/", "K&H e-bank – INTERNETBANK")
        ),
        via(
            "banking",
            Evidence::TitleContains {
                keyword: "Internetbank".to_owned()
            }
        )
    );
    assert_eq!(
        eval(
            &category("invoices"),
            &document(
                Some("/Users/anna/Bérjegyzék.pdf"),
                "Bérjegyzék.pdf",
                Dirty::Clean
            )
        ),
        via(
            "invoices",
            Evidence::FileGlob {
                glob: "**/*bérjegyzék*".to_owned()
            }
        )
    );
}

#[test]
fn unknown_category_never_matches() {
    assert!(!hits(&category("nope"), &tab("https://otpbank.hu/", "")));
}

#[test]
fn any_returns_the_first_evidence() {
    let any = Matcher::Any(vec![title("nothing"), domain("a.hu"), title("Bank")]);
    assert_eq!(
        eval(&any, &tab("https://a.hu/", "Bank")),
        Some(Evidence::Domain {
            domain: "a.hu".to_owned()
        })
    );
    assert!(!hits(&any, &tab("https://b.hu/", "Shop")));
    assert!(!hits(&Matcher::Any(vec![]), &tab("https://b.hu/", "")));
}

#[test]
fn all_needs_every_child_and_reports_the_first_positive() {
    let all = Matcher::All(vec![
        Matcher::Not(Box::new(title("demo"))),
        domain("a.hu"),
        title("bank"),
    ]);
    assert_eq!(
        eval(&all, &tab("https://a.hu/", "Bank")),
        Some(Evidence::Domain {
            domain: "a.hu".to_owned()
        })
    );
    assert!(!hits(&all, &tab("https://a.hu/", "Bank demo")));
    assert!(!hits(&all, &tab("https://a.hu/", "Shop")));
}

#[test]
fn not_matches_without_evidence() {
    let target = tab("https://a.hu/", "");
    let ctx_index = index();
    let ctx = MatchContext {
        categories: &ctx_index,
        home: HOME,
    };
    let not = Matcher::Not(Box::new(domain("b.hu")));
    assert_eq!(evaluate(&not, &target, &ctx), None);
    assert!(matches(&not, &target, &ctx));
    assert!(!matches(
        &Matcher::Not(Box::new(domain("a.hu"))),
        &target,
        &ctx
    ));

    let only_nots = Matcher::Any(vec![not.clone(), domain("c.hu")]);
    assert_eq!(evaluate(&only_nots, &target, &ctx), None);
    assert!(matches(&only_nots, &target, &ctx));
}

#[test]
fn parse_domain_rejects_garbage() {
    for bad in [
        "",
        "https://bank.hu",
        "bank.hu/login",
        "bank.hu:443",
        "bank .hu",
        " bank.hu",
        ".bank.hu",
        "bank..hu",
        "*.",
        "[::1]",
    ] {
        assert_eq!(parse_domain(bad), None, "{bad:?}");
    }
    assert_eq!(
        parse_domain("Bücher.DE"),
        Some(Site::Domain("xn--bcher-kva.de".to_owned()))
    );
}

#[test]
fn exe_matching_uses_the_target_app_for_non_process_targets() {
    let window = app_window("Signal.exe", "Signal", "").with_id(3);
    assert!(app_matches(
        &AppSelector::Exe("SIGNAL.EXE".to_owned()),
        &window
    ));
}
