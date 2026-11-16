//! Prints everything Close Call can see right now as JSON. Read-only: nothing is closed,
//! hidden or quit, and no permission prompt is shown.
//!
//! `cargo run -p closecall-platform-macos --example inventory -- --redact` replaces titles,
//! URLs, paths and keys with their length, for pasting into an issue.

#![allow(clippy::print_stdout)]

#[cfg(target_os = "macos")]
fn main() {
    use std::time::{Duration, Instant};

    use closecall_core::platform::Platform;
    use closecall_platform_macos::MacPlatform;
    use serde_json::{Value, json};

    fn redact(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, field) in map.iter_mut() {
                    match (key.as_str(), &*field) {
                        ("title" | "url" | "path" | "key", Value::String(text)) => {
                            *field = Value::String(format!("<{} chars>", text.chars().count()));
                        }
                        _ => redact(field),
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(redact),
            _ => {}
        }
    }

    let platform = MacPlatform::new();
    let mut providers = serde_json::Map::new();
    for mut provider in platform.providers() {
        let started = Instant::now();
        let result = provider.snapshot(Instant::now() + Duration::from_secs(10));
        let elapsed_ms = started.elapsed().as_millis();
        let value = match result {
            Ok(found) => json!({
                "elapsed_ms": elapsed_ms,
                "count": found.len(),
                "targets": found
                    .iter()
                    .map(|d| json!({ "key": d.key, "target": d.target }))
                    .collect::<Vec<_>>(),
            }),
            Err(error) => json!({ "elapsed_ms": elapsed_ms, "error": error.to_string() }),
        };
        providers.insert(provider.id().0, value);
    }
    let mut report = json!({
        "platform": platform.name(),
        "permissions": platform.permissions().status(),
        "providers": providers,
    });
    if std::env::args().any(|arg| arg == "--redact") {
        redact(&mut report);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report is plain JSON")
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {}
