//! "macos.ax": windows and documents of regular apps, through Accessibility, with
//! AppleScript for the dirty flag and saving where an app has a dictionary.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use closecall_core::platform::{
    Discovered, EventSender, ExecContext, InventoryEvent, Permission, Provider, ProviderError,
    Reverify,
};
use closecall_core::{
    Action, AppWindow, Capability, DocumentWindow, Handle, ProviderId, Step, StepOutcome, Target,
    TargetKind, UnsavedPolicy,
};
use objc2_application_services::AXError;
use serde_json::Value;

use super::{done, failed, fell_back, hide_instead, same_instance, skipped, unhide_app};
use crate::apps::{self, OpenError, RunningApp};
use crate::ax::{self, Ax};
use crate::doc::{self, CloseFacts, CloseRoute, DocApp};
use crate::osa;
use crate::poll::{self, LastGood, Poller};
use crate::script::{self, DocOp, DocSelector, ProbeDoc};
use crate::sheet::{self, SheetAnswer};
use crate::{automation, caps, keys};

pub(crate) const ID: &str = "macos.ax";
const FINDER: &str = "com.apple.finder";
const SHEET_WAIT: Duration = Duration::from_millis(1500);
const POLL_STEP: Duration = Duration::from_millis(50);

pub(crate) struct WindowProvider {
    last: Arc<LastGood<i32>>,
    poller: Option<Poller>,
}

impl WindowProvider {
    pub(crate) fn new() -> Self {
        Self {
            last: Arc::new(LastGood::new()),
            poller: None,
        }
    }
}

/// `Index` is the fallback when `_AXUIElementGetWindow` is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WindowRef {
    Id(u32),
    Index(usize),
}

fn encode_handle(pid: i32, window: WindowRef) -> String {
    match window {
        WindowRef::Id(id) => format!("{pid}:{id}"),
        WindowRef::Index(index) => format!("{pid}:#{index}"),
    }
}

fn decode_handle(handle: &str) -> Option<(i32, WindowRef)> {
    let (pid, rest) = handle.split_once(':')?;
    let window = match rest.strip_prefix('#') {
        Some(index) => WindowRef::Index(index.parse().ok()?),
        None => WindowRef::Id(rest.parse().ok()?),
    };
    Some((pid.parse().ok()?, window))
}

fn target_window(target: &Target) -> Option<WindowRef> {
    let handle = match &target.kind {
        TargetKind::DocumentWindow(w) => &w.window,
        TargetKind::AppWindow(w) => &w.window,
        _ => return None,
    };
    decode_handle(&handle.0).map(|(_, window)| window)
}

struct WindowInfo {
    ax: Ax,
    window: WindowRef,
    title: String,
    path: Option<String>,
    minimized: bool,
    edited: Option<bool>,
}

fn read_windows(pid: i32) -> Result<Vec<WindowInfo>, AXError> {
    let windows = Ax::application(pid).elements("AXWindows")?;
    Ok(windows
        .into_iter()
        .enumerate()
        .filter_map(|(index, ax)| {
            if ax.string("AXRole").as_deref() != Some("AXWindow") {
                return None;
            }
            // Excludes palettes, the Finder desktop and other non-document surfaces.
            let subrole = ax.string("AXSubrole");
            if !matches!(subrole.as_deref(), Some("AXStandardWindow" | "AXDialog")) {
                return None;
            }
            Some(WindowInfo {
                window: ax
                    .window_id()
                    .map_or(WindowRef::Index(index), WindowRef::Id),
                title: ax.string("AXTitle").unwrap_or_default(),
                path: ax
                    .string("AXDocument")
                    .and_then(|url| doc::file_url_to_path(&url)),
                minimized: ax.boolean("AXMinimized").unwrap_or(false),
                edited: ax
                    .element("AXCloseButton")
                    .and_then(|button| button.boolean("AXEdited")),
                ax,
            })
        })
        .collect())
}

struct Documents {
    app: Option<&'static DocApp>,
    /// `None` when the app has no dictionary, no Automation grant, or didn't answer.
    probe: Option<Vec<ProbeDoc>>,
}

fn documents(bundle_id: Option<&str>, deadline: Instant) -> Documents {
    let app = bundle_id.and_then(doc::doc_app);
    let probe = app
        .filter(|app| automation::granted(app.bundle_id))
        .and_then(|app| {
            let value =
                osa::run_in_app(app.bundle_id, &script::doc_probe(&app.dict), deadline).ok()?;
            Some(script::parse_doc_probe(&value, app.dict.dirty_when))
        });
    Documents { app, probe }
}

fn unique<'a>(mut matches: impl Iterator<Item = &'a ProbeDoc>) -> Option<&'a ProbeDoc> {
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn matching_doc<'a>(docs: &'a Documents, window: &WindowInfo) -> Option<&'a ProbeDoc> {
    let probe = docs.probe.as_deref()?;
    match &window.path {
        Some(path) => unique(
            probe
                .iter()
                .filter(|d| d.path.as_deref().is_some_and(|r| doc::same_file(path, r))),
        ),
        None => unique(
            probe
                .iter()
                .filter(|d| doc::title_matches(&window.title, &d.name)),
        ),
    }
}

/// Only document apps can hold unsaved work in a window without a file; for anything else
/// a clear `AXEdited` means there is nothing to lose.
fn is_document_app(app: &RunningApp, docs: &Documents, windows: &[WindowInfo]) -> bool {
    app.bundle_id.as_deref() != Some(FINDER)
        && (docs.app.is_some() || windows.iter().any(|w| w.path.is_some()))
}

/// `/Volumes/Macintosh HD` is a symlink to `/`, so only real mount points count as volumes.
fn mounted_volume(name: &str) -> bool {
    Path::new("/Volumes")
        .join(name)
        .symlink_metadata()
        .is_ok_and(|meta| !meta.file_type().is_symlink())
}

fn describe(
    app: &RunningApp,
    window: &WindowInfo,
    docs: &Documents,
    document_app: bool,
) -> Discovered {
    let matched = matching_doc(docs, window);
    let path = window.path.clone().or_else(|| {
        matched
            .and_then(|d| d.path.as_deref())
            .and_then(|p| doc::posix_path(p, mounted_volume))
    });
    let handle = Handle(encode_handle(app.pid, window.window));
    let title = window.title.clone();
    let kind = match path {
        Some(path) => TargetKind::DocumentWindow(DocumentWindow {
            window: handle.clone(),
            title,
            doc_kind: doc::doc_kind(&path),
            dirty: doc::dirty_state(matched.and_then(|d| d.dirty), window.edited, document_app),
            path: Some(path),
        }),
        None => TargetKind::AppWindow(AppWindow {
            window: handle.clone(),
            title,
            minimized: window.minimized,
        }),
    };
    Discovered {
        key: handle.0,
        target: poll::target(ID, app.app_ref(), kind),
    }
}

fn scan_app(app: &RunningApp, deadline: Instant) -> Result<Vec<Discovered>, AXError> {
    let windows = match read_windows(app.pid) {
        Ok(windows) => windows,
        Err(AXError::CannotComplete) => return Err(AXError::CannotComplete),
        Err(_) => Vec::new(),
    };
    if windows.is_empty() {
        return Ok(Vec::new());
    }
    let docs = documents(app.bundle_id.as_deref(), deadline);
    let document_app = is_document_app(app, &docs, &windows);
    Ok(windows
        .iter()
        .map(|window| describe(app, window, &docs, document_app))
        .collect())
}

fn scan(last: &LastGood<i32>, deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
    if !ax::is_trusted() {
        return Err(ProviderError::PermissionMissing(Permission::Accessibility));
    }
    let mut found = Vec::new();
    let mut alive = HashSet::new();
    for app in apps::regular() {
        if app.is_own() || app.bundle_id.as_deref().is_some_and(caps::has_tab_provider) {
            continue;
        }
        let result = if Instant::now() < deadline {
            scan_app(&app, deadline)
        } else {
            Err(AXError::CannotComplete)
        };
        found.extend(last.resolve(&app.pid, result));
        alive.insert(app.pid);
    }
    last.retain(&alive);
    Ok(found)
}

/// Windows of the target's app instance (`AppRef` includes the pid) that the run acts on.
fn planned_windows(target: &Target, planned: &[Target]) -> HashSet<WindowRef> {
    planned
        .iter()
        .filter(|other| other.provider == target.provider && other.app == target.app)
        .filter_map(target_window)
        .collect()
}

/// Hiding the app is right only when it hides nothing the run leaves alone.
fn hides_whole_app(
    own: WindowRef,
    visible: impl IntoIterator<Item = WindowRef>,
    planned: &HashSet<WindowRef>,
) -> bool {
    visible
        .into_iter()
        .all(|window| window == own || planned.contains(&window))
}

/// Hides the app when `whole_app` allows it, otherwise minimizes just this window.
fn conceal(pid: i32, window: &WindowInfo, whole_app: bool) -> Option<Action> {
    if whole_app && apps::hide(pid) {
        return Some(Action::Hide);
    }
    if window.ax.set_bool("AXMinimized", true) {
        return Some(Action::Minimize);
    }
    apps::hide(pid).then_some(Action::Hide)
}

fn conceal_instead(
    pid: i32,
    window: &WindowInfo,
    whole_app: bool,
    reason_key: &str,
) -> StepOutcome {
    conceal(pid, window, whole_app).map_or_else(
        || failed("could not conceal the window"),
        |action| fell_back(action, reason_key),
    )
}

fn still_open(pid: i32, window: &WindowInfo) -> bool {
    window.ax.is_alive()
        && read_windows(pid).is_ok_and(|all| all.iter().any(|w| w.window == window.window))
}

fn wait_closed(pid: i32, window: &WindowInfo, until: Instant) -> bool {
    loop {
        if !still_open(pid, window) {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        thread::sleep(POLL_STEP);
    }
}

fn press_cancel(sheet: &Ax, window: &WindowInfo) {
    if let Some(cancel) = sheet
        .element("AXCancelButton")
        .or_else(|| window.ax.element("AXCancelButton"))
    {
        cancel.press();
    }
}

#[derive(Clone, Copy)]
struct SheetPolicy {
    policy: UnsavedPolicy,
    has_path: bool,
    autosaves: bool,
}

fn answer_sheet(
    pid: i32,
    window: &WindowInfo,
    whole_app: bool,
    found: &Ax,
    policy: SheetPolicy,
    deadline: Instant,
) -> StepOutcome {
    let buttons = sheet::buttons(found);
    let default = found
        .element("AXDefaultButton")
        .or_else(|| window.ax.element("AXDefaultButton"));
    let cancel = found
        .element("AXCancelButton")
        .or_else(|| window.ax.element("AXCancelButton"));
    let roles = sheet::roles(&buttons, default.as_ref(), cancel.as_ref());
    match sheet::choose(policy.policy, policy.has_path, policy.autosaves, roles) {
        SheetAnswer::Press(i) => {
            if buttons[i].press() && wait_closed(pid, window, deadline) {
                return done();
            }
            // A Save panel or an error sheet is in the way; back out and conceal.
            if let Some(next) = sheet::find_sheet(&window.ax) {
                press_cancel(&next, window);
            }
            conceal_instead(pid, window, whole_app, keys::SAVE_FAILED_HIDDEN)
        }
        SheetAnswer::CancelAndConceal(reason) => {
            press_cancel(found, window);
            conceal_instead(pid, window, whole_app, reason)
        }
        SheetAnswer::CancelAndSkip(reason) => {
            press_cancel(found, window);
            skipped(reason)
        }
    }
}

fn press_close(
    pid: i32,
    window: &WindowInfo,
    whole_app: bool,
    policy: SheetPolicy,
    deadline: Instant,
) -> StepOutcome {
    let pressed = window
        .ax
        .element("AXCloseButton")
        .is_some_and(|button| button.press());
    if !pressed {
        return conceal_instead(pid, window, whole_app, keys::CLOSE_FAILED);
    }
    let sheet_deadline = deadline.min(Instant::now() + SHEET_WAIT);
    loop {
        if !still_open(pid, window) {
            return done();
        }
        if let Some(found) = sheet::find_sheet(&window.ax) {
            return answer_sheet(pid, window, whole_app, &found, policy, deadline);
        }
        if Instant::now() >= sheet_deadline {
            return conceal_instead(pid, window, whole_app, keys::CLOSE_FAILED);
        }
        thread::sleep(POLL_STEP);
    }
}

fn local_now() -> [u32; 6] {
    // SAFETY: `time(NULL)` only returns the clock; `localtime_r` writes into our own `tm`.
    let tm = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm
    };
    [
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
    ]
    .map(|part| u32::try_from(part).unwrap_or(0))
}

fn untitled_destination(app: &DocApp, title: &str, ctx: &ExecContext) -> Option<String> {
    std::fs::create_dir_all(&ctx.untitled_save_dir).ok()?;
    let name = doc::untitled_file_name(title, local_now(), app.untitled_extension?);
    let path = ctx.untitled_save_dir.join(name);
    if path.exists() {
        return None;
    }
    path.to_str().map(str::to_owned)
}

struct CloseRequest<'a> {
    app: &'a RunningApp,
    window: &'a WindowInfo,
    all: &'a [WindowInfo],
    whole_app: bool,
    policy: UnsavedPolicy,
    ctx: &'a ExecContext,
    deadline: Instant,
}

fn close(request: &CloseRequest<'_>) -> StepOutcome {
    let &CloseRequest {
        app,
        window,
        all,
        whole_app,
        policy,
        ctx,
        deadline,
    } = request;
    let pid = app.pid;
    let docs = documents(app.bundle_id.as_deref(), deadline);
    let matched = matching_doc(&docs, window);
    let has_path = window.path.is_some() || matched.is_some_and(|d| d.path.is_some());
    let document_app = is_document_app(app, &docs, all);
    let facts = CloseFacts {
        dirty: doc::dirty_state(matched.and_then(|d| d.dirty), window.edited, document_app),
        has_path,
        scriptable: matched.is_some(),
        autosaves: docs.app.is_some_and(|a| a.autosaves),
        saves_untitled: docs.app.is_some_and(|a| a.untitled_extension.is_some()),
    };
    let sheet_policy = SheetPolicy {
        policy,
        has_path,
        autosaves: facts.autosaves,
    };
    let route = doc::close_route(policy, facts);
    let (dict_app, matched) = match (&route, docs.app, matched) {
        (CloseRoute::Conceal(reason), ..) => {
            return conceal_instead(pid, window, whole_app, reason);
        }
        (CloseRoute::Skip(reason), ..) => return skipped(reason),
        (CloseRoute::Button, ..) | (_, None, _) | (_, _, None) => {
            return press_close(pid, window, whole_app, sheet_policy, deadline);
        }
        (_, Some(dict_app), Some(matched)) => (dict_app, matched),
    };
    let by_path_or_name = || match (&matched.path, dict_app.match_by_path) {
        (Some(path), true) => DocSelector::Path(path.clone()),
        _ => DocSelector::Name(matched.name.clone()),
    };
    let (selector, op) = match route {
        CloseRoute::ScriptSave if dict_app.close_discards => {
            (by_path_or_name(), DocOp::SaveCheckedThenClose)
        }
        CloseRoute::ScriptSave => (by_path_or_name(), DocOp::SaveAndClose),
        CloseRoute::ScriptDiscard => (by_path_or_name(), DocOp::DiscardAndClose),
        _ => match untitled_destination(dict_app, &window.title, ctx) {
            Some(dest) => (
                DocSelector::UntitledNamed(matched.name.clone()),
                DocOp::SaveInto(dest),
            ),
            None => return conceal_instead(pid, window, whole_app, keys::SAVE_FAILED_HIDDEN),
        },
    };
    let untitled = matches!(op, DocOp::SaveInto(_));
    let script = script::doc_act(&dict_app.dict, &selector, &op);
    match osa::run_in_app(dict_app.bundle_id, &script, deadline) {
        Ok(Value::String(result)) if result == "closed" => done(),
        Ok(Value::String(result)) if result == "unsaved" => {
            conceal_instead(pid, window, whole_app, keys::SAVE_FAILED_HIDDEN)
        }
        _ if untitled => conceal_instead(pid, window, whole_app, keys::UNTITLED_HIDDEN),
        _ => press_close(pid, window, whole_app, sheet_policy, deadline),
    }
}

impl Provider for WindowProvider {
    fn id(&self) -> ProviderId {
        ProviderId(ID.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        caps::windows(ID)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        let last = Arc::clone(&self.last);
        self.poller = Some(poll::spawn(
            self.id(),
            apps::launches(),
            events,
            move |deadline| scan(&last, deadline),
        )?);
        Ok(())
    }

    fn snapshot(&mut self, deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
        scan(&self.last, deadline)
    }

    fn reverify(&mut self, target: &Target, deadline: Instant) -> Reverify {
        let (Some(app), Some(wanted)) = (same_instance(target), target_window(target)) else {
            return Reverify::Gone;
        };
        let windows = match read_windows(app.pid) {
            Ok(windows) => windows,
            // Without an answer we can still conceal the app, so don't drop the target.
            Err(AXError::CannotComplete | AXError::APIDisabled) => return Reverify::Same,
            Err(_) => return Reverify::Gone,
        };
        let Some(window) = windows.iter().find(|w| w.window == wanted) else {
            return Reverify::Gone;
        };
        let docs = documents(app.bundle_id.as_deref(), deadline);
        let document_app = is_document_app(&app, &docs, &windows);
        poll::compare(target, describe(&app, window, &docs, document_app).target)
    }

    fn execute(
        &mut self,
        step: &Step,
        target: &Target,
        ctx: &ExecContext,
        deadline: Instant,
    ) -> StepOutcome {
        let Some(app) = same_instance(target) else {
            return skipped(keys::NOT_RUNNING);
        };
        let Some(wanted) = target_window(target) else {
            return failed("malformed window handle");
        };
        match step.action {
            Action::NotifyOnly => return done(),
            Action::Quit => return skipped(keys::UNSUPPORTED),
            Action::Close | Action::Hide | Action::Minimize => {}
        }
        if !ax::is_trusted() {
            return hide_instead(app.pid, keys::ACCESSIBILITY_MISSING);
        }
        let Ok(all) = read_windows(app.pid) else {
            return hide_instead(app.pid, keys::WINDOW_UNREACHABLE);
        };
        let Some(window) = all.iter().find(|w| w.window == wanted) else {
            return skipped(keys::TARGET_GONE);
        };
        let whole_app = hides_whole_app(
            window.window,
            all.iter().filter(|w| !w.minimized).map(|w| w.window),
            &planned_windows(target, &ctx.planned),
        );
        match step.action {
            Action::Minimize if window.ax.set_bool("AXMinimized", true) => done(),
            Action::Minimize => hide_instead(app.pid, keys::WINDOW_UNREACHABLE),
            Action::Hide => match conceal(app.pid, window, whole_app) {
                Some(Action::Hide) => done(),
                Some(action) => fell_back(action, keys::OTHER_WINDOWS_VISIBLE),
                None => failed("could not conceal the window"),
            },
            _ => close(&CloseRequest {
                app: &app,
                window,
                all: &all,
                whole_app,
                policy: step.on_unsaved,
                ctx,
                deadline,
            }),
        }
    }

    fn restore(&mut self, target: &Target, action: Action, deadline: Instant) -> StepOutcome {
        match action {
            Action::NotifyOnly => done(),
            Action::Hide => unhide_app(target),
            Action::Minimize => unminimize(target),
            Action::Close => reopen_document(target, deadline),
            Action::Quit => skipped(keys::RESTORE_UNSUPPORTED),
        }
    }
}

fn unminimize(target: &Target) -> StepOutcome {
    let Some(app) = same_instance(target) else {
        return skipped(keys::NOT_RUNNING);
    };
    let Some(wanted) = target_window(target) else {
        return failed("malformed window handle");
    };
    if !ax::is_trusted() {
        return skipped(keys::ACCESSIBILITY_MISSING);
    }
    let Ok(windows) = read_windows(app.pid) else {
        return failed("could not read the app's windows");
    };
    match windows.iter().find(|w| w.window == wanted) {
        Some(window) if window.ax.set_bool("AXMinimized", false) => done(),
        Some(_) => failed("could not unminimize the window"),
        None => skipped(keys::TARGET_GONE),
    }
}

/// Launch Services hands the file to the app that had it open, launching the app if it has
/// quit since. A window that never had a file can't come back.
fn reopen_document(target: &Target, deadline: Instant) -> StepOutcome {
    let path = match &target.kind {
        TargetKind::DocumentWindow(document) => document.path.as_deref(),
        _ => None,
    };
    let Some(path) = path else {
        return skipped(keys::RESTORE_NO_FILE);
    };
    if !Path::new(path).exists() {
        return skipped(keys::RESTORE_FILE_MISSING);
    }
    match apps::open_with(path, &target.app.id, deadline) {
        Ok(()) => done(),
        Err(OpenError::NoSuchApp) => skipped(keys::RESTORE_UNSUPPORTED),
        Err(OpenError::Timeout) => failed("the app did not confirm opening the document in time"),
        Err(OpenError::Failed { domain, code }) => {
            failed(format!("opening the document failed: {domain} {code}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_handles_round_trip() {
        for window in [WindowRef::Id(4711), WindowRef::Index(2)] {
            assert_eq!(
                decode_handle(&encode_handle(321, window)),
                Some((321, window))
            );
        }
        assert_eq!(decode_handle("321"), None);
        assert_eq!(decode_handle("x:1"), None);
        assert_eq!(decode_handle("1:#x"), None);
    }

    fn window(pid: u32, window: WindowRef) -> Target {
        poll::target(
            ID,
            closecall_core::AppRef {
                id: "com.apple.Preview".into(),
                name: "Preview".into(),
                pid: Some(pid),
            },
            TargetKind::AppWindow(AppWindow {
                window: Handle(encode_handle(pid.cast_signed(), window)),
                title: String::new(),
                minimized: false,
            }),
        )
    }

    #[test]
    fn planned_windows_belong_to_the_same_app_instance() {
        let own = window(7, WindowRef::Id(1));
        let mut other_app = window(7, WindowRef::Id(5));
        other_app.app.id = "com.apple.TextEdit".into();
        let planned = [
            own.clone(),
            window(7, WindowRef::Id(2)),
            window(7, WindowRef::Index(3)),
            window(8, WindowRef::Id(4)),
            other_app,
        ];
        assert_eq!(
            planned_windows(&own, &planned),
            HashSet::from([WindowRef::Id(1), WindowRef::Id(2), WindowRef::Index(3)])
        );
    }

    #[test]
    fn hides_the_app_only_when_every_visible_window_is_planned() {
        let own = WindowRef::Id(1);
        let planned = HashSet::from([own, WindowRef::Id(2)]);
        assert!(hides_whole_app(own, [own, WindowRef::Id(2)], &planned));
        assert!(!hides_whole_app(
            own,
            [own, WindowRef::Id(2), WindowRef::Id(3)],
            &planned
        ));
        assert!(
            hides_whole_app(own, [own], &HashSet::new()),
            "the step's own window counts even when the engine plans nothing else"
        );
        assert!(
            hides_whole_app(own, [WindowRef::Id(2)], &planned),
            "an already minimized own window"
        );
    }
}
