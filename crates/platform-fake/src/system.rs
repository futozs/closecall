use closecall_core::{AppRef, DisplayInfo};

pub const DISPLAYLINK: &str = "com.displaylink.DisplayLinkUserAgent";

pub fn running_app(id: &str, name: &str, pid: u32) -> AppRef {
    AppRef {
        id: id.to_owned(),
        name: name.to_owned(),
        pid: Some(pid),
    }
}

/// Neither built in nor mirrored, and without a serial number.
pub fn external_display(id: u32, name: &str, vendor: u32, model: u32) -> DisplayInfo {
    DisplayInfo {
        id,
        name: name.to_owned(),
        vendor,
        model,
        serial: None,
        builtin: false,
        mirrored: false,
    }
}

/// The apps behind the demo inventory and a meeting app. No always-on capturer: with one
/// running, a share without a named app would count as its capture.
pub fn demo_running_apps() -> Vec<AppRef> {
    vec![
        running_app("com.google.Chrome", "Google Chrome", 501),
        running_app("com.apple.Safari", "Safari", 502),
        running_app("com.apple.Preview", "Preview", 503),
        running_app("com.microsoft.Word", "Microsoft Word", 504),
        running_app("com.1password.1password", "1Password", 505),
        running_app("org.whispersystems.signal-desktop", "Signal", 506),
        running_app("us.zoom.xos", "zoom.us", 507),
    ]
}

/// The built-in panel and two monitors.
pub fn demo_displays() -> Vec<DisplayInfo> {
    vec![
        DisplayInfo {
            builtin: true,
            ..external_display(1, "Built-in Retina Display", 1552, 41006)
        },
        DisplayInfo {
            serial: Some(16_843_009),
            ..external_display(4, "LG HDR 4K", 7789, 30471)
        },
        DisplayInfo {
            serial: Some(16_843_010),
            ..external_display(5, "LG HDR 4K", 7789, 30471)
        },
    ]
}
