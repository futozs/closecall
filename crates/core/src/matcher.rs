use globset::{GlobBuilder, GlobMatcher};
use url::{Host, Url};

use crate::text::fold;
use crate::{
    AppRef, AppSelector, CategoryDef, CategoryId, DisplayInfo, DisplaySelector, Evidence, Matcher,
    Target, TargetKind, Tri,
};

pub trait CategoryIndex {
    fn category(&self, id: &CategoryId) -> Option<&CategoryDef>;
    /// `host` is already lowercased and punycoded; parent domains count as matches.
    fn list_contains(&self, list_id: &str, host: &str) -> bool;
}

pub struct MatchContext<'a> {
    pub categories: &'a dyn CategoryIndex,
    pub home: &'a str,
}

/// `None` also when the matcher only matched through `Not`, which carries no evidence;
/// `Config::validate` rejects matchers where that can happen.
pub fn evaluate(matcher: &Matcher, target: &Target, ctx: &MatchContext<'_>) -> Option<Evidence> {
    match eval(matcher, &Subject::new(target), ctx) {
        Outcome::Hit(evidence) => evidence,
        Outcome::Miss => None,
    }
}

/// Like [`evaluate`], but also true for evidence-less matches. Used for `except` and protection.
pub(crate) fn matches(matcher: &Matcher, target: &Target, ctx: &MatchContext<'_>) -> bool {
    matches!(eval(matcher, &Subject::new(target), ctx), Outcome::Hit(_))
}

pub fn app_matches(selector: &AppSelector, target: &Target) -> bool {
    let process_exe = match &target.kind {
        TargetKind::Process(process) => Some(process.exe.as_str()),
        _ => None,
    };
    selects(selector, &target.app, process_exe)
}

/// For a running app that is not a target, such as a background agent capturing the screen.
pub fn app_ref_matches(selector: &AppSelector, app: &AppRef) -> bool {
    selects(selector, app, None)
}

fn selects(selector: &AppSelector, app: &AppRef, process_exe: Option<&str>) -> bool {
    match selector {
        AppSelector::BundleId(id) | AppSelector::DesktopId(id) => app.id == *id,
        AppSelector::Exe(exe) => exe_matches(exe, &app.id, process_exe),
        AppSelector::Name(name) => fold(&app.name) == fold(name),
    }
}

pub fn display_matches(selector: &DisplaySelector, display: &DisplayInfo) -> bool {
    match selector {
        DisplaySelector::Name(name) => {
            let needle = fold(name.trim());
            // An empty name would match every display.
            !needle.trim().is_empty() && fold(&display.name).contains(&needle)
        }
        DisplaySelector::Id(id) => {
            id.vendor == display.vendor
                && id.model == display.model
                && id
                    .serial
                    .is_none_or(|serial| display.serial == Some(serial))
        }
    }
}

enum Outcome {
    Miss,
    Hit(Option<Evidence>),
}

fn leaf(evidence: Option<Evidence>) -> Outcome {
    evidence.map_or(Outcome::Miss, |evidence| Outcome::Hit(Some(evidence)))
}

struct Subject<'a> {
    target: &'a Target,
    url: Option<&'a str>,
    site: Option<Site>,
    title: Option<&'a str>,
}

impl<'a> Subject<'a> {
    fn new(target: &'a Target) -> Self {
        let (url, title) = match &target.kind {
            TargetKind::BrowserTab(tab) => (tab.url.as_deref(), Some(tab.title.as_str())),
            TargetKind::DocumentWindow(doc) => (None, Some(doc.title.as_str())),
            TargetKind::AppWindow(window) => (None, Some(window.title.as_str())),
            TargetKind::Process(_) => (None, None),
        };
        Self {
            target,
            url,
            site: url.and_then(site_of_url),
            title,
        }
    }
}

fn eval(matcher: &Matcher, subject: &Subject<'_>, ctx: &MatchContext<'_>) -> Outcome {
    match matcher {
        Matcher::Category(id) => leaf(
            ctx.categories
                .category(id)
                .and_then(|def| category_hit(def, subject, ctx)),
        ),
        Matcher::Domain(domain) => leaf(domain_hit(domain, subject)),
        Matcher::UrlGlob(glob) => leaf(url_glob_hit(glob, subject)),
        Matcher::TitleContains(keyword) => leaf(title_hit(keyword, subject)),
        Matcher::App(selector) => leaf(app_hit(selector, subject.target)),
        Matcher::FileGlob(glob) => leaf(file_glob_hit(glob, subject.target, ctx.home)),
        Matcher::DocKind(kind) => leaf(match &subject.target.kind {
            TargetKind::DocumentWindow(doc) if doc.doc_kind == *kind => {
                Some(Evidence::DocKind { doc_kind: *kind })
            }
            _ => None,
        }),
        Matcher::Private => leaf(match &subject.target.kind {
            TargetKind::BrowserTab(tab) if tab.private == Tri::Yes => Some(Evidence::Private),
            _ => None,
        }),
        Matcher::Any(children) => {
            let mut matched = false;
            for child in children {
                match eval(child, subject, ctx) {
                    Outcome::Hit(Some(evidence)) => return Outcome::Hit(Some(evidence)),
                    Outcome::Hit(None) => matched = true,
                    Outcome::Miss => {}
                }
            }
            if matched {
                Outcome::Hit(None)
            } else {
                Outcome::Miss
            }
        }
        Matcher::All(children) => {
            let mut evidence = None;
            for child in children {
                match eval(child, subject, ctx) {
                    Outcome::Miss => return Outcome::Miss,
                    Outcome::Hit(found) => evidence = evidence.or(found),
                }
            }
            Outcome::Hit(evidence)
        }
        Matcher::Not(inner) => match eval(inner, subject, ctx) {
            Outcome::Miss => Outcome::Hit(None),
            Outcome::Hit(_) => Outcome::Miss,
        },
    }
}

fn category_hit(
    def: &CategoryDef,
    subject: &Subject<'_>,
    ctx: &MatchContext<'_>,
) -> Option<Evidence> {
    let via = def
        .domains
        .iter()
        .find_map(|domain| domain_hit(domain, subject))
        .or_else(|| match &subject.site {
            Some(Site::Domain(host)) => def
                .lists
                .iter()
                .find_map(|list| listed_entry(ctx.categories, list, host))
                .map(|domain| Evidence::Domain {
                    domain: domain.to_owned(),
                }),
            _ => None,
        })
        .or_else(|| {
            def.apps
                .iter()
                .find_map(|selector| app_hit(selector, subject.target))
        })
        .or_else(|| {
            def.title_keywords
                .iter()
                .find_map(|keyword| title_hit(keyword, subject))
        })
        .or_else(|| {
            def.file_globs
                .iter()
                .find_map(|glob| file_glob_hit(glob, subject.target, ctx.home))
        })?;
    Some(Evidence::Category {
        id: def.id.clone(),
        via: Box::new(via),
    })
}

/// `list_contains` also accepts parent domains, so walk up from the shortest suffix to report
/// the entry that is actually listed (`otpbank.hu`, not `netbank.otpbank.hu`).
fn listed_entry<'h>(index: &dyn CategoryIndex, list: &str, host: &'h str) -> Option<&'h str> {
    if !index.list_contains(list, host) {
        return None;
    }
    let parents: Vec<&str> = host
        .match_indices('.')
        .map(|(dot, _)| &host[dot + 1..])
        .collect();
    parents
        .into_iter()
        .rev()
        .find(|parent| index.list_contains(list, parent))
        .or(Some(host))
}

fn domain_hit(domain: &str, subject: &Subject<'_>) -> Option<Evidence> {
    let site = subject.site.as_ref()?;
    parse_domain(domain)
        .is_some_and(|pattern| pattern.covers(site))
        .then(|| Evidence::Domain {
            domain: domain.to_owned(),
        })
}

fn url_glob_hit(glob: &str, subject: &Subject<'_>) -> Option<Evidence> {
    let raw = subject.url?;
    let pattern = glob.to_lowercase();
    let normalized = Url::parse(raw).map(String::from).ok();
    [Some(raw), normalized.as_deref()]
        .into_iter()
        .flatten()
        .any(|url| wildcard_match(&pattern, &url.to_lowercase()))
        .then(|| Evidence::UrlGlob {
            glob: glob.to_owned(),
        })
}

fn title_hit(keyword: &str, subject: &Subject<'_>) -> Option<Evidence> {
    let needle = fold(keyword);
    // An empty keyword would match every title.
    (!needle.is_empty() && fold(subject.title?).contains(&needle)).then(|| {
        Evidence::TitleContains {
            keyword: keyword.to_owned(),
        }
    })
}

fn app_hit(selector: &AppSelector, target: &Target) -> Option<Evidence> {
    app_matches(selector, target).then(|| Evidence::App {
        app: target.app.name.clone(),
    })
}

fn file_glob_hit(glob: &str, target: &Target, home: &str) -> Option<Evidence> {
    let TargetKind::DocumentWindow(doc) = &target.kind else {
        return None;
    };
    let path = doc.path.as_deref()?;
    let expanded = match glob.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
        _ => glob.to_owned(),
    };
    compile_glob(&expanded)
        .is_ok_and(|matcher| matcher.is_match(fold(path)))
        .then(|| Evidence::FileGlob {
            glob: glob.to_owned(),
        })
}

/// Pattern and path are both folded, so `*szamla*` finds `Számla.pdf`, and an NFD path from
/// the macOS file system matches an NFC pattern.
pub(crate) fn compile_glob(pattern: &str) -> Result<GlobMatcher, globset::Error> {
    GlobBuilder::new(&fold(pattern))
        .build()
        .map(|glob| glob.compile_matcher())
}

fn exe_matches(selector: &str, app_id: &str, process_exe: Option<&str>) -> bool {
    let selector = selector.to_lowercase();
    std::iter::once(app_id)
        .chain(process_exe)
        .map(str::to_lowercase)
        .any(|candidate| candidate == selector || file_name(&candidate) == selector)
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Site {
    Domain(String),
    Ip(String),
}

impl Site {
    /// Domains cover their subdomains on label boundaries; IP addresses only themselves.
    fn covers(&self, host: &Site) -> bool {
        match (self, host) {
            (Site::Domain(domain), Site::Domain(host)) => {
                host == domain
                    || host
                        .strip_suffix(domain.as_str())
                        .is_some_and(|rest| rest.ends_with('.'))
            }
            (Site::Ip(pattern), Site::Ip(host)) => pattern == host,
            _ => false,
        }
    }
}

fn site_of_url(url: &str) -> Option<Site> {
    let url = Url::parse(url).ok()?;
    Some(match url.host()? {
        Host::Domain(domain) => Site::Domain(domain.trim_end_matches('.').to_lowercase()),
        Host::Ipv4(ip) => Site::Ip(ip.to_string()),
        Host::Ipv6(ip) => Site::Ip(ip.to_string()),
    })
}

/// Accepts `bank.hu`, `*.bank.hu`, a trailing dot and Unicode names (punycoded here).
pub(crate) fn parse_domain(raw: &str) -> Option<Site> {
    if raw.is_empty() || raw.contains(['/', ':']) || raw.contains(char::is_whitespace) {
        return None;
    }
    let name = raw.strip_prefix("*.").unwrap_or(raw);
    let name = name.strip_suffix('.').unwrap_or(name);
    match Host::parse(name).ok()? {
        Host::Domain(domain) if !domain.split('.').any(str::is_empty) => Some(Site::Domain(domain)),
        Host::Domain(_) => None,
        Host::Ipv4(ip) => Some(Site::Ip(ip.to_string())),
        Host::Ipv6(ip) => Some(Site::Ip(ip.to_string())),
    }
}

/// `*` matches any run of characters, everything else is literal, and the whole text must match.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let (mut p, mut t) = (0, 0);
    let mut backtrack: Option<(usize, usize)> = None;
    while t < text.len() {
        if pattern.get(p) == Some(&b'*') {
            backtrack = Some((p, t));
            p += 1;
        } else if pattern.get(p) == Some(&text[t]) {
            p += 1;
            t += 1;
        } else if let Some((star, start)) = backtrack {
            backtrack = Some((star, start + 1));
            p = star + 1;
            t = start + 1;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&byte| byte == b'*')
}

#[cfg(test)]
mod tests;
