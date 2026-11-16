//! Read-only probe for the `macos.share_ui` source and the ignore list: once a second, for as
//! many seconds as the first argument says (default 1), prints the sharing controls it can see
//! (bundle ids, window layers and sizes, never a title), then the displays and whether a known
//! always-on capturer runs. Start a share in Zoom, Teams or Meet meanwhile.

#![allow(clippy::print_stdout)]

#[cfg(target_os = "macos")]
fn main() {
    use std::time::Duration;

    use closecall_core::config::KNOWN_CAPTURE_APPS;
    use closecall_core::platform::Platform;
    use closecall_platform_macos::{MacPlatform, share_ui_report};

    let seconds = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(1u64);
    for second in 0..seconds {
        if second > 0 {
            std::thread::sleep(Duration::from_secs(1));
        }
        print!("{}", share_ui_report());
    }

    // Display names come from NSScreen, which this probe reads on its main thread.
    let platform = MacPlatform::new();
    println!("displays:");
    for display in (platform.displays())() {
        println!(
            "  {}: \"{}\" vendor {} model {} serial {} builtin {} mirrored {}",
            display.id,
            display.name,
            display.vendor,
            display.model,
            display
                .serial
                .map_or("none".to_owned(), |serial| serial.to_string()),
            display.builtin,
            display.mirrored,
        );
    }
    let running = (platform.running_apps())();
    let capturers: Vec<&str> = running
        .iter()
        .filter(|app| KNOWN_CAPTURE_APPS.contains(&app.id.as_str()))
        .map(|app| app.name.as_str())
        .collect();
    println!(
        "running apps listed: {}, known always-on capturers running: {:?}",
        running.len(),
        capturers
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {}
