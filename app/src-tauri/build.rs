use tauri_build::{AppManifest, Attributes};

/// Declaring the commands makes Tauri deny any that a capability does not allow by name.
const COMMANDS: &[&str] = &[
    "get_config",
    "save_config",
    "reset_config",
    "get_status",
    "preview",
    "run_panic",
    "simulate",
    "get_activity",
    "restore",
    "get_permissions",
    "request_permission",
    "get_capabilities",
    "list_categories",
    "list_displays",
    "list_running_apps",
    "pause",
    "resume",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri_build::try_build(Attributes::new().app_manifest(AppManifest::new().commands(COMMANDS)))?;
    Ok(())
}
