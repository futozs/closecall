//! The command table in `closecall_core::ipc`, one to one. Engine calls block, so each runs
//! on the blocking pool.

use closecall_core::config::{Config, ValidationIssue};
use closecall_core::ipc::{CapabilityReport, CategoryInfo, Preset, RunReport, Status};
use closecall_core::platform::{Permission, PermissionState, RequestOutcome};
use closecall_core::{AppRef, DisplayInfo, FiredTrigger};
use closecall_engine::{Engine, EngineError};
use tauri::{AppHandle, State};

use crate::shell;

type Reply<T> = Result<T, String>;

async fn call<T, F>(engine: &Engine, f: F) -> Reply<T>
where
    T: Send + 'static,
    F: FnOnce(&Engine) -> Result<T, EngineError> + Send + 'static,
{
    let engine = engine.clone();
    tauri::async_runtime::spawn_blocking(move || f(&engine))
        .await
        .map_err(|err| err.to_string())?
        .map_err(|err| err.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_config(engine: State<'_, Engine>) -> Reply<Config> {
    call(&engine, |engine| Ok(engine.config())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_config(
    app: AppHandle,
    engine: State<'_, Engine>,
    config: Config,
) -> Reply<Vec<ValidationIssue>> {
    let language = config.language;
    let hotkey = config.panic_hotkey.clone();
    let issues = call(&engine, move |engine| engine.save_config(config)).await?;
    if issues.is_empty() {
        shell::apply_config(&app, language, &hotkey);
    }
    Ok(issues)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn reset_config(
    app: AppHandle,
    engine: State<'_, Engine>,
    preset: Preset,
) -> Reply<Config> {
    let config = call(&engine, move |engine| engine.reset_config(preset)).await?;
    shell::apply_config(&app, config.language, &config.panic_hotkey);
    Ok(config)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_status(engine: State<'_, Engine>) -> Reply<Status> {
    call(&engine, |engine| Ok(engine.status())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn preview(engine: State<'_, Engine>, trigger: FiredTrigger) -> Reply<RunReport> {
    call(&engine, move |engine| Ok(engine.preview(trigger))).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_panic(engine: State<'_, Engine>) -> Reply<RunReport> {
    call(&engine, |engine| Ok(engine.panic())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn simulate(engine: State<'_, Engine>, trigger: FiredTrigger) -> Reply<RunReport> {
    call(&engine, move |engine| Ok(engine.simulate(trigger))).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_activity(engine: State<'_, Engine>) -> Reply<Vec<RunReport>> {
    call(&engine, |engine| Ok(engine.activity())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn restore(engine: State<'_, Engine>, run_id: u64) -> Reply<RunReport> {
    call(&engine, move |engine| engine.restore(run_id)).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_permissions(engine: State<'_, Engine>) -> Reply<Vec<PermissionState>> {
    call(&engine, |engine| Ok(engine.permissions())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn request_permission(
    engine: State<'_, Engine>,
    permission: Permission,
) -> Reply<RequestOutcome> {
    call(&engine, move |engine| {
        Ok(engine.request_permission(&permission))
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_capabilities(engine: State<'_, Engine>) -> Reply<CapabilityReport> {
    call(&engine, |engine| Ok(engine.capabilities())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_categories(engine: State<'_, Engine>) -> Reply<Vec<CategoryInfo>> {
    call(&engine, |engine| Ok(engine.categories())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_displays(engine: State<'_, Engine>) -> Reply<Vec<DisplayInfo>> {
    call(&engine, |engine| Ok(engine.displays())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_running_apps(engine: State<'_, Engine>) -> Reply<Vec<AppRef>> {
    call(&engine, |engine| Ok(engine.running_apps())).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn pause(engine: State<'_, Engine>, minutes: u32) -> Reply<Status> {
    call(&engine, move |engine| Ok(engine.pause(minutes))).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn resume(engine: State<'_, Engine>) -> Reply<Status> {
    call(&engine, |engine| Ok(engine.resume())).await
}
