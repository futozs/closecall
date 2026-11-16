//! AppleScript sources and parsers for their results. Pure, so it is unit-tested.
//!
//! AppleScript rather than JXA: only AppleScript has `with timeout`, and without it a hung
//! browser blocks for the two-minute Apple Event default.

use std::time::Duration;

use closecall_core::Tri;
use serde_json::Value;

/// A window or tab id as the app reported it. Chrome uses integers, Brave and Arc use text,
/// and the id must go back in the same type or the lookup fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScriptId {
    Int(i64),
    Text(String),
}

impl ScriptId {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(n) => n.as_i64().map(Self::Int),
            Value::String(s) => Some(Self::Text(s.clone())),
            _ => None,
        }
    }

    pub(crate) fn encode(&self) -> String {
        match self {
            Self::Int(n) => format!("i:{n}"),
            Self::Text(s) => format!("s:{s}"),
        }
    }

    pub(crate) fn decode(encoded: &str) -> Option<Self> {
        if let Some(n) = encoded.strip_prefix("i:") {
            n.parse().ok().map(Self::Int)
        } else {
            encoded.strip_prefix("s:").map(|s| Self::Text(s.to_owned()))
        }
    }

    fn literal(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Text(s) => quote(s),
        }
    }
}

pub(crate) fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\" & (character id {}) & \"", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn optional_text(text: Option<&str>) -> String {
    text.map_or_else(|| "missing value".to_owned(), quote)
}

pub(crate) fn timeout_secs(remaining: Duration) -> u64 {
    remaining.as_secs().clamp(1, 2)
}

/// Wraps `body` so it never launches the app: `tell` alone would start it if it quit
/// after our running check.
pub(crate) fn in_app(bundle_id: &str, timeout_secs: u64, body: &str) -> String {
    let app = quote(bundle_id);
    format!(
        "with timeout of {timeout_secs} seconds\n\
         if application id {app} is running then\n\
         tell application id {app}\n\
         {body}\n\
         end tell\n\
         end if\n\
         end timeout\n\
         return missing value"
    )
}

fn swallow_missing(body: &str) -> String {
    format!(
        "try\n{body}\non error number n\n\
         if n is -1728 or n is -1719 then return missing value\n\
         error number n\nend try"
    )
}

/// A window that closes mid-scan, or a browser without `mode`, is skipped; a timeout, a lost
/// connection or a revoked grant fails the scan, so its tabs are kept rather than dropped.
pub(crate) const CHROMIUM_LIST: &str = r#"set out to {}
repeat with wid in (get id of every window)
	set w to window id (contents of wid)
	set m to missing value
	try
		set m to mode of w
	on error number n
		if n is -1712 or n is -609 or n is -1743 then error number n
	end try
	set ai to 0
	try
		set ai to active tab index of w
	on error number n
		if n is -1712 or n is -609 or n is -1743 then error number n
	end try
	try
		set end of out to {contents of wid, m, ai, id of every tab of w, title of every tab of w, URL of every tab of w}
	on error number n
		if n is -1712 or n is -609 or n is -1743 then error number n
	end try
end repeat
return out"#;

pub(crate) fn chromium_tab_info(window: &ScriptId, tab: &ScriptId) -> String {
    swallow_missing(&format!(
        "set t to tab id {} of window id {}\nreturn {{title of t, URL of t}}",
        tab.literal(),
        window.literal()
    ))
}

/// A tab to close by id. It is closed only while its title and URL are what the plan was
/// made from: for the step's own tab that guards against a reverify that could not reach the
/// browser, for the other planned tabs of the window it stands in for their own reverify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChromiumPeer<'a> {
    pub id: ScriptId,
    pub title: &'a str,
    pub url: Option<&'a str>,
}

/// What [`chromium_close_tabs`] did with one tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabClose {
    Closed,
    Changed,
    Gone,
}

/// Closes each unchanged tab, in order. Returns 1 (closed), -1 (changed) or 0 (gone) per tab.
pub(crate) fn chromium_close_tabs(window: &ScriptId, tabs: &[ChromiumPeer<'_>]) -> String {
    let entries = tabs
        .iter()
        .map(|tab| {
            format!(
                "{{{}, {}, {}}}",
                tab.id.literal(),
                quote(tab.title),
                optional_text(tab.url)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "set w to window id {wid}\n\
         set out to {{}}\n\
         repeat with p in {{{entries}}}\n\
         set q to contents of p\n\
         set r to 0\n\
         try\n\
         set x to tab id (item 1 of q) of w\n\
         set r to -1\n\
         considering case\n\
         if (title of x) is (item 2 of q) and (URL of x) is (item 3 of q) then set r to 1\n\
         end considering\n\
         if r is 1 then close x\n\
         on error number n\n\
         if n is not -1728 and n is not -1719 then error number n\n\
         end try\n\
         set end of out to r\n\
         end repeat\n\
         return out",
        wid = window.literal(),
    )
}

/// Never into an incognito window: the tab came from the normal profile, with its cookies
/// and history. Prefers the tab's own window, then any normal one, then a new one.
pub(crate) fn chromium_reopen(window: Option<&ScriptId>, url: &str) -> String {
    let own_window = window.map_or_else(String::new, |window| {
        format!(
            "try\n\
             set c to window id {}\n\
             if mode of c is \"normal\" then set w to c\n\
             end try\n",
            window.literal()
        )
    });
    format!(
        "set u to {url}\n\
         set w to missing value\n\
         {own_window}\
         if w is missing value then\n\
         repeat with wid in (get id of every window)\n\
         set c to window id (contents of wid)\n\
         try\n\
         if mode of c is \"normal\" then\n\
         set w to c\n\
         exit repeat\n\
         end if\n\
         end try\n\
         end repeat\n\
         end if\n\
         if w is missing value then\n\
         set w to make new window with properties {{mode:\"normal\"}}\n\
         set URL of active tab of w to u\n\
         else\n\
         make new tab at end of tabs of w with properties {{URL:u}}\n\
         end if\n\
         return \"opened\"",
        url = quote(url),
    )
}

pub(crate) const SAFARI_LIST: &str = r#"set out to {}
repeat with wid in (get id of every window)
	set w to window id (contents of wid)
	try
		set end of out to {contents of wid, index of current tab of w, name of every tab of w, URL of every tab of w}
	end try
end repeat
return out"#;

/// Safari tabs have no id, so a tab is found again by window, last known index and URL,
/// immediately before acting, because indexes shift whenever a tab closes.
fn safari_find(window: &ScriptId, index: usize, url: Option<&str>, found: &str) -> String {
    let url = optional_text(url);
    swallow_missing(&format!(
        "set w to window id {wid}\n\
         set n to count of tabs of w\n\
         set i to 0\n\
         if n >= {index} then\n\
         if URL of tab {index} of w is {url} then set i to {index}\n\
         end if\n\
         if i is 0 then\n\
         repeat with k from 1 to n\n\
         if URL of tab k of w is {url} then\n\
         set i to k\n\
         exit repeat\n\
         end if\n\
         end repeat\n\
         end if\n\
         if i is 0 then return missing value\n\
         {found}",
        wid = window.literal(),
    ))
}

pub(crate) fn safari_tab_info(window: &ScriptId, index: usize, url: Option<&str>) -> String {
    safari_find(
        window,
        index,
        url,
        "return {i, name of tab i of w, URL of tab i of w}",
    )
}

/// A Safari tab to close, by its index at scan time and its URL. `title` is checked for the
/// other planned tabs of the window (see [`ChromiumPeer`]) and is `None` for the step's
/// own tab, which the engine has just reverified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SafariTab<'a> {
    pub index: usize,
    pub url: Option<&'a str>,
    pub title: Option<&'a str>,
}

/// Finds every tab first, each at most once, and closes from the right so the indexes of
/// tabs still to close don't shift. Returns the index each entry was found at, 0 if not.
pub(crate) fn safari_close_tabs(window: &ScriptId, tabs: &[SafariTab<'_>]) -> String {
    let entries = tabs
        .iter()
        .map(|tab| {
            format!(
                "{{{}, {}, {}}}",
                tab.index,
                optional_text(tab.url),
                optional_text(tab.title)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "set w to window id {wid}\n\
         set n to count of tabs of w\n\
         set picked to {{}}\n\
         set out to {{}}\n\
         considering case\n\
         repeat with p in {{{entries}}}\n\
         set q to contents of p\n\
         set k to item 1 of q\n\
         set u to item 2 of q\n\
         set t to item 3 of q\n\
         set i to 0\n\
         if k <= n and picked does not contain k then\n\
         if URL of tab k of w is u then set i to k\n\
         end if\n\
         if i is 0 then\n\
         repeat with j from 1 to n\n\
         if picked does not contain j then\n\
         if URL of tab j of w is u then\n\
         set i to j\n\
         exit repeat\n\
         end if\n\
         end if\n\
         end repeat\n\
         end if\n\
         if i > 0 and t is not missing value then\n\
         if name of tab i of w is not t then set i to 0\n\
         end if\n\
         if i > 0 then set end of picked to i\n\
         set end of out to i\n\
         end repeat\n\
         end considering\n\
         repeat with j from n to 1 by -1\n\
         if picked contains j then close tab j of w\n\
         end repeat\n\
         return out",
        wid = window.literal(),
    )
}

/// The index each tab of [`safari_close_tabs`] was closed at, `None` where it wasn't found.
pub(crate) fn parse_safari_closed(value: &Value, expected: usize) -> Option<Vec<Option<usize>>> {
    let items = value.as_array().filter(|items| items.len() == expected)?;
    Some(items.iter().map(index).collect())
}

/// What [`chromium_close_tabs`] did with each tab.
pub(crate) fn parse_chromium_closed(value: &Value, expected: usize) -> Option<Vec<TabClose>> {
    let items = value.as_array().filter(|items| items.len() == expected)?;
    Some(
        items
            .iter()
            .map(|item| match item.as_i64() {
                Some(1) => TabClose::Closed,
                Some(-1) => TabClose::Changed,
                _ => TabClose::Gone,
            })
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptTab {
    pub id: Option<ScriptId>,
    pub title: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptWindow {
    pub id: ScriptId,
    pub private: Tri,
    /// 1-based, as AppleScript counts.
    pub active_index: Option<usize>,
    pub tabs: Vec<ScriptTab>,
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn index(value: &Value) -> Option<usize> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .filter(|&n| n > 0)
}

fn private_mode(value: &Value) -> Tri {
    match value {
        Value::String(mode) if mode == "incognito" => Tri::Yes,
        Value::String(mode) if mode == "normal" => Tri::No,
        Value::Bool(true) => Tri::Yes,
        Value::Bool(false) => Tri::No,
        _ => Tri::Unknown,
    }
}

/// Windows whose columns disagree in length changed while we read them; they are dropped
/// and picked up by the next poll.
pub(crate) fn parse_chromium(value: &Value) -> Vec<ScriptWindow> {
    let Some(windows) = value.as_array() else {
        return Vec::new();
    };
    windows
        .iter()
        .filter_map(|row| {
            let [id, mode, active, ids, titles, urls] = row.as_array()?.as_slice() else {
                return None;
            };
            let (ids, titles, urls) = (ids.as_array()?, titles.as_array()?, urls.as_array()?);
            if ids.len() != titles.len() || ids.len() != urls.len() {
                return None;
            }
            let tabs = ids
                .iter()
                .zip(titles)
                .zip(urls)
                .map(|((id, title), url)| {
                    Some(ScriptTab {
                        id: Some(ScriptId::from_value(id)?),
                        title: text(title).unwrap_or_default(),
                        url: text(url),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(ScriptWindow {
                id: ScriptId::from_value(id)?,
                private: private_mode(mode),
                active_index: index(active),
                tabs,
            })
        })
        .collect()
}

pub(crate) fn parse_safari(value: &Value) -> Vec<ScriptWindow> {
    let Some(windows) = value.as_array() else {
        return Vec::new();
    };
    windows
        .iter()
        .filter_map(|row| {
            let [id, current, titles, urls] = row.as_array()?.as_slice() else {
                return None;
            };
            let (titles, urls) = (titles.as_array()?, urls.as_array()?);
            if titles.len() != urls.len() {
                return None;
            }
            Some(ScriptWindow {
                id: ScriptId::from_value(id)?,
                private: Tri::Unknown,
                active_index: index(current),
                tabs: titles
                    .iter()
                    .zip(urls)
                    .map(|(title, url)| ScriptTab {
                        id: None,
                        title: text(title).unwrap_or_default(),
                        url: text(url),
                    })
                    .collect(),
            })
        })
        .collect()
}

/// `(title, url)` of a single tab, or `None` when the script found no such tab.
pub(crate) fn parse_tab_info(value: &Value) -> Option<(String, Option<String>)> {
    match value.as_array()?.as_slice() {
        [title, url] => Some((text(title).unwrap_or_default(), text(url))),
        _ => None,
    }
}

pub(crate) fn parse_safari_tab_info(value: &Value) -> Option<(usize, String, Option<String>)> {
    match value.as_array()?.as_slice() {
        [i, title, url] => Some((index(i)?, text(title).unwrap_or_default(), text(url))),
        _ => None,
    }
}

/// How a document app names its documents in its dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocDictionary {
    pub element: &'static str,
    pub path_property: &'static str,
    pub dirty_property: &'static str,
    /// The `dirty_property` value that means unsaved changes (`modified` true, `saved` false).
    pub dirty_when: bool,
}

pub(crate) fn doc_probe(dict: &DocDictionary) -> String {
    format!(
        "return {{{path} of every {el}, {dirty} of every {el}, name of every {el}}}",
        path = dict.path_property,
        dirty = dict.dirty_property,
        el = dict.element,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbeDoc {
    pub path: Option<String>,
    pub name: String,
    pub dirty: Option<bool>,
}

pub(crate) fn parse_doc_probe(value: &Value, dirty_when: bool) -> Vec<ProbeDoc> {
    let Some([paths, dirty, names]) = value.as_array().map(Vec::as_slice) else {
        return Vec::new();
    };
    let (Some(paths), Some(dirty), Some(names)) =
        (paths.as_array(), dirty.as_array(), names.as_array())
    else {
        return Vec::new();
    };
    if paths.len() != names.len() || dirty.len() != names.len() {
        return Vec::new();
    }
    paths
        .iter()
        .zip(dirty)
        .zip(names)
        .map(|((path, dirty), name)| ProbeDoc {
            path: text(path).filter(|p| p.contains('/') || p.contains(':')),
            name: text(name).unwrap_or_default(),
            dirty: dirty.as_bool().map(|d| d == dirty_when),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocSelector {
    Path(String),
    Name(String),
    UntitledNamed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocOp {
    SaveAndClose,
    DiscardAndClose,
    /// For PowerPoint, whose `close` drops changes without asking.
    SaveCheckedThenClose,
    SaveInto(String),
}

pub(crate) fn doc_act(dict: &DocDictionary, selector: &DocSelector, op: &DocOp) -> String {
    let condition = match selector {
        DocSelector::Path(path) => format!("({} of d) is {}", dict.path_property, quote(path)),
        DocSelector::Name(name) => format!("(name of d) is {}", quote(name)),
        DocSelector::UntitledNamed(name) => format!(
            "(name of d) is {} and ({} of d) is missing value",
            quote(name),
            dict.path_property
        ),
    };
    let op = match op {
        DocOp::SaveAndClose => "close d saving yes".to_owned(),
        DocOp::DiscardAndClose => "close d saving no".to_owned(),
        DocOp::SaveCheckedThenClose => {
            "save d\nif saved of d is false then return \"unsaved\"\nclose d saving no".to_owned()
        }
        DocOp::SaveInto(path) => {
            format!("save d in (POSIX file {})\nclose d saving yes", quote(path))
        }
    };
    format!(
        "set found to {{}}\n\
         repeat with d in (every {el})\n\
         if {condition} then set end of found to contents of d\n\
         end repeat\n\
         if (count of found) is not 1 then return \"ambiguous\"\n\
         set d to item 1 of found\n\
         {op}\n\
         return \"closed\"",
        el = dict.element,
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn quote_escapes_quotes_backslashes_and_control_characters() {
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(quote("line\nnext\ttab"), r#""line\nnext\ttab""#);
        assert_eq!(quote("x\u{1}y"), r#""x" & (character id 1) & "y""#);
        assert_eq!(quote("Számla – „x”"), "\"Számla – „x”\"");
    }

    #[test]
    fn quoted_injection_stays_inside_the_literal() {
        let hostile = "\" & (do shell script \"rm -rf ~\") & \"";
        let quoted = quote(hostile);
        assert!(quoted.starts_with('"') && quoted.ends_with('"'));
        let inner = &quoted[1..quoted.len() - 1];
        let mut escaped = false;
        for c in inner.chars() {
            assert!(escaped || c != '"', "unescaped quote in {quoted}");
            escaped = !escaped && c == '\\';
        }
    }

    #[test]
    fn script_ids_round_trip_and_keep_their_type() {
        for id in [ScriptId::Int(42), ScriptId::Text("8F2A-11".into())] {
            assert_eq!(ScriptId::decode(&id.encode()), Some(id));
        }
        assert_eq!(ScriptId::Int(7).literal(), "7");
        assert_eq!(ScriptId::Text("7".into()).literal(), "\"7\"");
        assert_eq!(ScriptId::decode("x:1"), None);
    }

    #[test]
    fn timeout_is_between_one_and_two_seconds() {
        assert_eq!(timeout_secs(Duration::from_millis(300)), 1);
        assert_eq!(timeout_secs(Duration::from_secs(30)), 2);
    }

    #[test]
    fn in_app_checks_running_before_telling() {
        let script = in_app("com.google.Chrome", 2, "return 1");
        let running = script.find("is running").unwrap();
        let tell = script.find("tell application id").unwrap();
        assert!(running < tell);
        assert!(script.starts_with("with timeout of 2 seconds"));
    }

    #[test]
    fn parses_chromium_windows_with_int_and_text_ids() {
        let value = json!([
            [
                1,
                "normal",
                2,
                [10, 11],
                ["Inbox", "Bank"],
                ["https://mail.example/", "https://bank.example/"]
            ],
            ["w-2", "incognito", 1, ["t-1"], ["Private"], [null]],
            ["w-3", null, 0, [], [], []]
        ]);
        let windows = parse_chromium(&value);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].id, ScriptId::Int(1));
        assert_eq!(windows[0].private, Tri::No);
        assert_eq!(windows[0].active_index, Some(2));
        assert_eq!(windows[0].tabs[1].title, "Bank");
        assert_eq!(windows[0].tabs[1].id, Some(ScriptId::Int(11)));
        assert_eq!(windows[1].private, Tri::Yes);
        assert_eq!(windows[1].tabs[0].url, None);
        assert_eq!(windows[2].private, Tri::Unknown);
        assert_eq!(windows[2].active_index, None);
    }

    #[test]
    fn drops_chromium_windows_that_changed_mid_read() {
        let value = json!([[1, "normal", 1, [10, 11], ["only one"], ["u"]], "garbage"]);
        assert!(parse_chromium(&value).is_empty());
        assert!(parse_chromium(&json!(null)).is_empty());
    }

    #[test]
    fn parses_safari_windows() {
        let value = json!([[7, 2, ["A", "B"], ["https://a.example/", null]]]);
        let windows = parse_safari(&value);
        assert_eq!(windows[0].id, ScriptId::Int(7));
        assert_eq!(windows[0].private, Tri::Unknown);
        assert_eq!(windows[0].active_index, Some(2));
        assert_eq!(windows[0].tabs[1].url, None);
        assert_eq!(windows[0].tabs[0].id, None);
    }

    #[test]
    fn parses_single_tab_results() {
        assert_eq!(
            parse_tab_info(&json!(["T", "https://x/"])),
            Some(("T".into(), Some("https://x/".into())))
        );
        assert_eq!(parse_tab_info(&json!(null)), None);
        assert_eq!(
            parse_safari_tab_info(&json!([3, "T", null])),
            Some((3, "T".into(), None))
        );
    }

    #[test]
    fn safari_scripts_compare_against_the_quoted_url() {
        let script = safari_close_tabs(
            &ScriptId::Int(5),
            &[SafariTab {
                index: 3,
                url: Some("https://bank.example/?a=\"1\""),
                title: None,
            }],
        );
        assert!(script.contains("window id 5"));
        assert!(script.contains(r#"{3, "https://bank.example/?a=\"1\"", missing value}"#));
        let blank = safari_tab_info(&ScriptId::Int(5), 1, None);
        assert!(blank.contains("is missing value"));
    }

    #[test]
    fn safari_batch_finds_every_tab_before_closing_from_the_right() {
        let script = safari_close_tabs(
            &ScriptId::Int(5),
            &[
                SafariTab {
                    index: 2,
                    url: Some("https://a/"),
                    title: None,
                },
                SafariTab {
                    index: 4,
                    url: Some("https://b/"),
                    title: Some("Bank"),
                },
            ],
        );
        assert!(script.contains(
            r#"repeat with p in {{2, "https://a/", missing value}, {4, "https://b/", "Bank"}}"#
        ));
        assert!(script.contains("picked does not contain j"));
        let find = script.find("end considering").unwrap();
        let close = script.find("close tab j of w").unwrap();
        assert!(find < close, "nothing closes before every tab is found");
        assert!(script.contains("repeat with j from n to 1 by -1"));
        assert!(script.contains("name of tab i of w is not t"));
    }

    #[test]
    fn chromium_scripts_address_tabs_by_id() {
        let script = chromium_close_tabs(
            &ScriptId::Text("w1".into()),
            &[ChromiumPeer {
                id: ScriptId::Text("t1".into()),
                title: "Bank",
                url: Some("https://bank.example/"),
            }],
        );
        assert!(script.contains(r#"set w to window id "w1""#));
        assert!(script.contains(r#"repeat with p in {{"t1", "Bank", "https://bank.example/"}}"#));
        let info = chromium_tab_info(&ScriptId::Int(1), &ScriptId::Int(2));
        assert!(info.contains("tab id 2 of window id 1"));
        assert!(info.contains("-1728"));
    }

    #[test]
    fn chromium_batch_closes_every_tab_only_while_unchanged() {
        let script = chromium_close_tabs(
            &ScriptId::Int(1),
            &[
                ChromiumPeer {
                    id: ScriptId::Int(11),
                    title: "Bank \"x\"",
                    url: Some("https://bank.example/"),
                },
                ChromiumPeer {
                    id: ScriptId::Text("t-12".into()),
                    title: "Mail",
                    url: None,
                },
            ],
        );
        assert!(script.contains(
            r#"{{11, "Bank \"x\"", "https://bank.example/"}, {"t-12", "Mail", missing value}}"#
        ));
        assert!(
            !script.contains("close (tab id"),
            "no tab is closed unchecked"
        );
        assert!(script.contains("considering case"));
        assert!(script.contains("if r is 1 then close x"));
        assert!(
            script.contains("if n is not -1728 and n is not -1719 then error number n"),
            "a timeout fails the script instead of reading as gone"
        );
    }

    #[test]
    fn the_tab_list_fails_on_timeouts_instead_of_dropping_windows() {
        assert_eq!(
            CHROMIUM_LIST
                .matches("if n is -1712 or n is -609 or n is -1743 then error number n")
                .count(),
            3
        );
    }

    #[test]
    fn parses_batch_results_and_rejects_mismatched_lengths() {
        assert_eq!(
            parse_chromium_closed(&json!([1, -1, 0]), 3),
            Some(vec![TabClose::Closed, TabClose::Changed, TabClose::Gone])
        );
        assert_eq!(parse_chromium_closed(&json!([1]), 2), None);
        assert_eq!(parse_chromium_closed(&json!(null), 0), None);
        assert_eq!(
            parse_safari_closed(&json!([3, 0]), 2),
            Some(vec![Some(3), None])
        );
        assert_eq!(parse_safari_closed(&json!("closed"), 1), None);
    }

    #[test]
    fn chromium_reopen_prefers_a_normal_window_and_never_an_incognito_one() {
        let script = chromium_reopen(Some(&ScriptId::Int(3)), "https://a.example/?q=\"x\"");
        assert!(script.starts_with(r#"set u to "https://a.example/?q=\"x\"""#));
        let own = script.find("set c to window id 3").unwrap();
        let any = script.find("get id of every window").unwrap();
        let new = script
            .find(r#"make new window with properties {mode:"normal"}"#)
            .unwrap();
        assert!(own < any && any < new);
        assert_eq!(script.matches(r#"if mode of c is "normal""#).count(), 2);
        assert!(!script.contains("incognito"));
        assert!(script.contains("make new tab at end of tabs of w with properties {URL:u}"));
        let without_window = chromium_reopen(None, "https://a.example/");
        assert!(!without_window.contains("try\nset c to window id"));
    }

    const OFFICE: DocDictionary = DocDictionary {
        element: "presentation",
        path_property: "full name",
        dirty_property: "saved",
        dirty_when: false,
    };

    #[test]
    fn doc_probe_reads_three_columns() {
        assert_eq!(
            doc_probe(&OFFICE),
            "return {full name of every presentation, saved of every presentation, name of every presentation}"
        );
    }

    #[test]
    fn parses_doc_probe_with_inverted_saved_flag() {
        let value = json!([
            ["/Users/a/Deck.pptx", "Presentation1"],
            [true, false],
            ["Deck.pptx", "Presentation1"]
        ]);
        let docs = parse_doc_probe(&value, OFFICE.dirty_when);
        assert_eq!(docs[0].path.as_deref(), Some("/Users/a/Deck.pptx"));
        assert_eq!(docs[0].dirty, Some(false));
        assert_eq!(
            docs[1].path, None,
            "Office reports untitled documents by bare name"
        );
        assert_eq!(docs[1].dirty, Some(true));
        assert!(parse_doc_probe(&json!([[1], [], []]), true).is_empty());
    }

    #[test]
    fn doc_act_selects_exactly_one_document() {
        let script = doc_act(
            &OFFICE,
            &DocSelector::Name("Deck.pptx".into()),
            &DocOp::SaveCheckedThenClose,
        );
        assert!(script.contains("repeat with d in (every presentation)"));
        assert!(script.contains(r#"(name of d) is "Deck.pptx""#));
        assert!(script.contains("is not 1 then return \"ambiguous\""));
        let save = script.find("save d").unwrap();
        let close = script.find("close d saving no").unwrap();
        assert!(save < close);
    }

    #[test]
    fn doc_act_saves_untitled_documents_into_the_given_file() {
        let dict = DocDictionary {
            element: "document",
            path_property: "path",
            dirty_property: "modified",
            dirty_when: true,
        };
        let script = doc_act(
            &dict,
            &DocSelector::UntitledNamed("Untitled".into()),
            &DocOp::SaveInto("/Users/a/Documents/Close Call/Untitled-2026-11-16-1305".into()),
        );
        assert!(script.contains("(path of d) is missing value"));
        assert!(script.contains(
            r#"save d in (POSIX file "/Users/a/Documents/Close Call/Untitled-2026-11-16-1305")"#
        ));
    }
}
