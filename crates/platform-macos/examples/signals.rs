//! Prints every signal for 60 seconds, or for as many seconds as the first argument says.
//! Lock the screen, close the lid, mirror a display or start a screen share meanwhile.

#![allow(clippy::print_stdout)]

#[cfg(target_os = "macos")]
fn main() {
    use std::time::{Duration, Instant};

    use closecall_core::platform::{EventSender, Platform, Signal};
    use closecall_platform_macos::MacPlatform;
    use objc2_core_foundation::{CFRunLoop, CFRunLoopRunResult, kCFRunLoopDefaultMode};

    struct Print(Instant);

    impl EventSender<Signal> for Print {
        fn send(&self, signal: Signal) -> bool {
            println!(
                "{:>6.1}s {}",
                self.0.elapsed().as_secs_f32(),
                serde_json::to_string(&signal).expect("signals serialize")
            );
            true
        }
    }

    let seconds = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(60u64);
    let platform = MacPlatform::new();
    let started = Instant::now();
    let mut sources = platform.signal_sources();
    for source in &mut sources {
        let result = source.start(Box::new(Print(started)));
        println!(
            "{}: start {:?}, health {:?}",
            source.id(),
            result,
            source.health()
        );
    }

    // Notification observers and the display callback are delivered on the main run loop.
    // SAFETY: framework constant.
    let mode = unsafe { kCFRunLoopDefaultMode };
    let until = started + Duration::from_secs(seconds);
    while Instant::now() < until {
        if CFRunLoop::run_in_mode(mode, 0.5, false) == CFRunLoopRunResult::Finished {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    for source in &sources {
        println!("{}: health {:?}", source.id(), source.health());
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {}
