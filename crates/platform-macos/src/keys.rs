//! Translation keys this adapter puts into `StepOutcome`, `Support` and `SourceHealth`. Each
//! is a full key in `locales/*/ui.json`; `scripts/ci/check-message-keys.ts` checks that.

pub const UNSAVED_HIDDEN: &str = "reason.unsaved_hidden";
pub const UNSAVED_SKIPPED: &str = "reason.unsaved_skipped";
pub const UNTITLED_HIDDEN: &str = "reason.untitled_hidden";
pub const DISCARD_UNTITLED_HIDDEN: &str = "reason.discard_untitled_hidden";
pub const AUTOSAVE_DISCARD_HIDDEN: &str = "reason.autosave_discard_hidden";
pub const SAVE_FAILED_HIDDEN: &str = "reason.save_failed_hidden";
pub const SHEET_UNRECOGNIZED_HIDDEN: &str = "reason.sheet_unrecognized_hidden";
pub const SHEET_OPEN_HIDDEN: &str = "reason.sheet_open_hidden";
pub const QUIT_WITH_DOCUMENTS_HIDDEN: &str = "reason.quit_with_documents_hidden";
pub const OTHER_WINDOWS_VISIBLE: &str = "reason.other_windows_visible";
pub const PROTECTED_IN_APP: &str = "reason.protected_in_app";
pub const AUTOMATION_MISSING: &str = "reason.automation_missing";
pub const ACCESSIBILITY_MISSING: &str = "reason.permission_missing";
pub const CLOSE_FAILED: &str = "reason.close_failed";
pub const WINDOW_UNREACHABLE: &str = "reason.window_unreachable";
pub const CLOSE_UNSUPPORTED: &str = "reason.firefox_no_tab_close";
pub const UNSUPPORTED: &str = "reason.unsupported";
pub const NOT_RUNNING: &str = "reason.app_not_running";
pub const TARGET_GONE: &str = "reason.gone";
pub const CHANGED: &str = "reason.changed_before_action";
pub const HANDLE_INVALID: &str = "reason.handle_invalid";
pub const SCRIPT_FAILED: &str = "reason.script_failed";
pub const HIDE_FAILED: &str = "reason.hide_failed";
pub const CLOSE_AND_HIDE_FAILED: &str = "reason.close_and_hide_failed";
pub const CONCEAL_FAILED: &str = "reason.conceal_failed";
pub const MINIMIZE_FAILED: &str = "reason.minimize_failed";
pub const QUIT_FAILED: &str = "reason.quit_failed";
pub const RESTORE_UNSUPPORTED: &str = "reason.restore_unsupported";
pub const RESTORE_PRIVATE: &str = "reason.private_not_restorable";
pub const RESTORE_QUIT: &str = "reason.restore_quit";
pub const RESTORE_AUTOMATION_MISSING: &str = "reason.restore_automation_missing";
pub const RESTORE_NO_FILE: &str = "reason.restore_no_file";
pub const RESTORE_FILE_MISSING: &str = "reason.restore_file_missing";
pub const UNHIDE_FAILED: &str = "reason.unhide_failed";
pub const UNMINIMIZE_FAILED: &str = "reason.unminimize_failed";
pub const OPEN_TIMEOUT: &str = "reason.open_timeout";
pub const OPEN_FAILED: &str = "reason.open_failed";

pub const NOTE_HIDES_WHOLE_APP: &str = "note.hides_whole_app";
pub const NOTE_HIDE_OR_MINIMIZE: &str = "note.hide_or_minimize";
pub const NOTE_AX_SAVE_SHEET: &str = "note.sheet_by_role";
pub const NOTE_SAFARI_PRIVATE_UNKNOWN: &str = "note.safari_private_unknown";
pub const NOTE_FIREFOX_SESSION_FILE: &str = "note.firefox_session_file";
pub const NOTE_ARC_UNVERIFIED: &str = "note.arc_unverified";

pub const HEALTH_WATCHER_MISSING: &str = "reason.macos.health.screen_watcher_missing";
pub const HEALTH_WATCHER_NOISY: &str = "reason.macos.health.screen_watcher_noisy";
pub const HEALTH_DISPLAY_CALLBACK_FAILED: &str = "reason.macos.health.display_callback_failed";
pub const HEALTH_NO_INPUT_DEVICE: &str = "reason.macos.health.no_input_device";
pub const HEALTH_WINDOW_LIST_UNAVAILABLE: &str = "reason.macos.health.window_list_unavailable";
