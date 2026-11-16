mod commands;
mod native;
mod shell;
mod sink;
mod tray;

use std::error::Error;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

use closecall_engine::{Engine, EngineSettings};
use tauri::{App, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::native::Native;
use crate::shell::Shell;
use crate::sink::{SilentSink, TauriSink};
use crate::tray::{Mode, Tray};

/// Also the Tauri identifier: TCC grants and the config directory are keyed to it.
const APP_ID: &str = "hu.futozs.closecall";

pub fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "diag") {
        return diag();
    }
    init_tracing();
    let panic_at_launch = args.iter().any(|arg| arg == "panic");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            shell::second_instance(app, &argv);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(MacosLauncher::LaunchAgent)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::reset_config,
            commands::get_status,
            commands::preview,
            commands::run_panic,
            commands::simulate,
            commands::get_activity,
            commands::restore,
            commands::get_permissions,
            commands::request_permission,
            commands::get_capabilities,
            commands::list_categories,
            commands::list_displays,
            commands::list_running_apps,
            commands::pause,
            commands::resume,
        ])
        .setup(move |app| setup(app, panic_at_launch))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = window.hide() {
                    tracing::warn!("could not hide the settings window: {err}");
                }
            }
        })
        .build(tauri::generate_context!());

    match app {
        Ok(app) => {
            app.run(|_, event| {
                // Only Quit (an explicit exit code) ends the app; it lives in the menu bar.
                if let RunEvent::ExitRequested {
                    code: None, api, ..
                } = event
                {
                    api.prevent_exit();
                }
            });
            ExitCode::SUCCESS
        }
        Err(err) => {
            tracing::error!("Close Call could not start: {err}");
            ExitCode::FAILURE
        }
    }
}

fn setup(app: &mut App, panic_at_launch: bool) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let handle = app.handle().clone();
    let shell = Arc::new(Shell::new(Native::load()?));
    let settings = EngineSettings {
        config_dir: app.path().app_config_dir()?,
        home: app.path().home_dir()?,
        app_version: app.package_info().version.to_string(),
        own_app_id: APP_ID.to_owned(),
    };
    let sink = Arc::new(TauriSink::new(handle.clone(), Arc::clone(&shell)));
    let engine = Engine::new(closecall_engine::default_platform(), settings, sink)?;
    let config = engine.config();
    let status = engine.status();
    app.manage(engine.clone());
    app.manage(Arc::clone(&shell));

    shell.set_mode(&handle, Mode::of(&status));
    shell::apply_config(&handle, config.language, &config.panic_hotkey);
    let tray = Tray::build(&handle, &shell.strings().tray, &shell.view())?;
    shell.attach_tray(tray);

    engine.start_signals();

    if status.needs_onboarding && !shell::is_shared(&status) {
        shell::show_window(&handle);
    }
    if panic_at_launch {
        shell::clean_up_now(&handle);
    }
    Ok(())
}

/// `closecall diag`: a stand-alone report, so it works when the app is broken or not running.
fn diag() -> ExitCode {
    let (Some(config_root), Some(home)) = (dirs::config_dir(), dirs::home_dir()) else {
        eprintln!("closecall diag: no home directory");
        return ExitCode::FAILURE;
    };
    let settings = EngineSettings {
        config_dir: config_root.join(APP_ID),
        home,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        own_app_id: APP_ID.to_owned(),
    };
    let engine = match Engine::new(
        closecall_engine::default_platform(),
        settings,
        Arc::new(SilentSink),
    ) {
        Ok(engine) => engine,
        Err(err) => {
            eprintln!("closecall diag: {err}");
            return ExitCode::FAILURE;
        }
    };
    match writeln!(std::io::stdout().lock(), "{}", engine.diag()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
