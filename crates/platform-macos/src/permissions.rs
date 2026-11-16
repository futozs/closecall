//! Accessibility and per-app Automation. Screen Recording and Input Monitoring are never
//! requested. Nothing here prompts except `request`.

use closecall_core::platform::{
    Permission, PermissionBroker, PermissionState, PermissionStatus, RequestOutcome,
};

use crate::doc::DOC_APPS;
use crate::{apps, automation, ax, caps};

const ACCESSIBILITY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
const AUTOMATION_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation";

fn automation_targets() -> impl Iterator<Item = (&'static str, &'static str)> {
    caps::CHROMIUM
        .iter()
        .copied()
        .chain([(caps::SAFARI, "Safari")])
        .chain(DOC_APPS.iter().map(|app| (app.bundle_id, app.name)))
}

pub(crate) struct MacPermissions;

impl PermissionBroker for MacPermissions {
    fn status(&self) -> Vec<PermissionState> {
        let mut out = vec![PermissionState {
            permission: Permission::Accessibility,
            status: if ax::is_trusted() {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            },
        }];
        for (bundle_id, fallback_name) in automation_targets() {
            if !apps::is_installed(bundle_id) {
                continue;
            }
            let running = apps::by_bundle(bundle_id);
            out.push(PermissionState {
                permission: Permission::Automation {
                    bundle_id: bundle_id.to_owned(),
                    app_name: running
                        .as_ref()
                        .map_or_else(|| fallback_name.to_owned(), |app| app.name.clone()),
                },
                status: if running.is_some() {
                    automation::status(bundle_id, false)
                } else {
                    PermissionStatus::AppNotRunning
                },
            });
        }
        out.push(PermissionState {
            permission: Permission::Notifications,
            status: PermissionStatus::NotApplicable,
        });
        out
    }

    /// Automation blocks until the user answers the system prompt; call it off the main thread.
    fn request(&self, permission: &Permission) -> RequestOutcome {
        match permission {
            Permission::Accessibility => {
                if ax::prompt_for_trust() {
                    return RequestOutcome::Granted;
                }
                apps::open_url(ACCESSIBILITY_SETTINGS);
                RequestOutcome::OpenedSettings
            }
            Permission::Automation { bundle_id, .. } => {
                automation::forget(bundle_id);
                match automation::status(bundle_id, false) {
                    PermissionStatus::Granted => RequestOutcome::Granted,
                    PermissionStatus::AppNotRunning => RequestOutcome::NotApplicable,
                    // A denied app is never asked again; only System Settings can change it.
                    PermissionStatus::Denied => {
                        apps::open_url(AUTOMATION_SETTINGS);
                        RequestOutcome::OpenedSettings
                    }
                    _ => match automation::status(bundle_id, true) {
                        PermissionStatus::Granted => RequestOutcome::Granted,
                        _ => RequestOutcome::Denied,
                    },
                }
            }
            Permission::Notifications | Permission::ToolkitAccessibility => {
                RequestOutcome::NotApplicable
            }
        }
    }
}
