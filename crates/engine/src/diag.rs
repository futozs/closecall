//! `closecall diag`: what the engine can see and do, safe to paste into a public issue. Counts,
//! ids and states only; never titles, URLs, paths or rule contents.

use closecall_core::platform::{Permission, PermissionStatus, SourceHealth};
use closecall_core::{Support, TargetKindTag};

use crate::runtime::{Core, lock};
use crate::store::Problem;

impl Core {
    pub fn diag(&self) -> String {
        let permissions = lock(&self.permissions).clone();
        let sources = self.source_statuses();
        let now = self.now();
        let state = lock(&self.state);
        let mut out = String::new();
        let mut line = |text: String| {
            out.push_str(&text);
            out.push('\n');
        };

        line("Close Call diagnostics (no titles, URLs or paths)".to_owned());
        line(format!("app: {}", self.settings.app_version));
        line(format!(
            "os: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        line(format!("platform: {}", self.platform));
        line(format!(
            "config: {}",
            match &state.config_problem {
                None if state.needs_onboarding =>
                    "none saved yet, using the recommended preset".to_owned(),
                None => "loaded".to_owned(),
                Some(problem) => format!(
                    "{}; using the recommended preset, file left untouched",
                    describe(problem)
                ),
            }
        ));
        let enabled = state
            .config
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .count();
        line(format!(
            "rules: {} ({enabled} enabled), user categories: {}",
            state.config.rules.len(),
            state.config.categories.len()
        ));
        let share = state.fusion.share_state(&state.config);
        line(format!(
            "share: {}, mirrored: {}, external display: {}, call likely: {}, guard: {}, paused: {}",
            if share.active { "active" } else { "inactive" },
            yes_no(share.mirrored_display),
            yes_no(share.external_display),
            yes_no(share.call_likely),
            if state.guard_active(now) { "on" } else { "off" },
            yes_no(state.paused(now)),
        ));
        line(format!(
            "ignore: {} capture apps, {} displays; capture put down to an ignored app: {}",
            state.config.ignore.capture_apps.len(),
            state.config.ignore.displays.len(),
            yes_no(share.ignored_capture.is_some()),
        ));

        line("providers:".to_owned());
        if self.providers.is_empty() {
            line("  none".to_owned());
        }
        for (i, provider) in self.providers.iter().enumerate() {
            let counts = state.index.counts(i);
            let targets = if counts.is_empty() {
                "nothing open".to_owned()
            } else {
                counts
                    .iter()
                    .map(|(kind, n)| format!("{n} {}", kind_name(*kind, *n)))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let capabilities = state.capabilities.get(i).map_or(&[][..], Vec::as_slice);
            let count =
                |f: fn(&Support) -> bool| capabilities.iter().filter(|c| f(&c.support)).count();
            let mut text = format!(
                "  {}: {targets}; capabilities {} full, {} partial, {} unsupported",
                provider.id.0,
                count(|s| matches!(s, Support::Full)),
                count(|s| matches!(s, Support::Partial { .. })),
                count(|s| matches!(s, Support::Unsupported)),
            );
            if state.permission_missing.get(i).copied().unwrap_or(false) {
                text.push_str("; permission missing");
            }
            if provider.stalled() {
                text.push_str("; not responding");
            }
            line(text);
        }

        line("permissions:".to_owned());
        if permissions.is_empty() {
            line("  none needed".to_owned());
        }
        for row in &permissions {
            line(format!(
                "  {}: {}",
                permission_name(&row.permission),
                status_name(row.status)
            ));
        }

        line("signal sources:".to_owned());
        if sources.is_empty() {
            line("  none".to_owned());
        }
        for source in &sources {
            let health = match &source.health {
                SourceHealth::Ok => "ok".to_owned(),
                SourceHealth::Unavailable { reason_key } => format!("unavailable ({reason_key})"),
                SourceHealth::Noisy { reason_key } => format!("noisy ({reason_key})"),
            };
            line(format!("  {}: {health}", source.id));
        }

        line(format!(
            "activity: {} runs in memory, {} restorable steps",
            state.activity.len(),
            state.journal.len()
        ));
        drop(state);
        out
    }
}

fn describe(problem: &Problem) -> String {
    match problem {
        Problem::Unreadable => "could not be read".to_owned(),
        Problem::Unparsable => "could not be parsed".to_owned(),
        Problem::TooNew(version) => format!("was written by a newer version ({version})"),
        Problem::Invalid(count) => format!("has {count} problem(s)"),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn kind_name(kind: TargetKindTag, count: usize) -> &'static str {
    match (kind, count) {
        (TargetKindTag::Tab, 1) => "tab",
        (TargetKindTag::Tab, _) => "tabs",
        (TargetKindTag::Document, 1) => "document",
        (TargetKindTag::Document, _) => "documents",
        (TargetKindTag::AppWindow, 1) => "app window",
        (TargetKindTag::AppWindow, _) => "app windows",
        (TargetKindTag::Process, 1) => "process",
        (TargetKindTag::Process, _) => "processes",
    }
}

/// Bundle ids are not personal data; app names, titles and paths are left out.
fn permission_name(permission: &Permission) -> String {
    match permission {
        Permission::Accessibility => "accessibility".to_owned(),
        Permission::Automation { bundle_id, .. } => format!("automation ({bundle_id})"),
        Permission::Notifications => "notifications".to_owned(),
        Permission::ToolkitAccessibility => "toolkit accessibility".to_owned(),
    }
}

fn status_name(status: PermissionStatus) -> &'static str {
    match status {
        PermissionStatus::Granted => "granted",
        PermissionStatus::Denied => "denied",
        PermissionStatus::NotDetermined => "not determined",
        PermissionStatus::AppNotRunning => "app not running",
        PermissionStatus::NotApplicable => "not applicable",
    }
}
