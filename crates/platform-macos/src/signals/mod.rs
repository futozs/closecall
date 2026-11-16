mod call;
mod display;
mod session;
mod share_ui;
mod skylight;

pub(crate) use call::CallSource;
pub(crate) use display::{DisplaySource, displays};
pub(crate) use session::SessionSource;
pub(crate) use share_ui::{ShareUiSource, report as share_ui_report};
pub(crate) use skylight::ScreenWatcher;

use std::sync::{Arc, Mutex, PoisonError};

use closecall_core::Confidence;
use closecall_core::platform::{EventSender, Signal, SignalKind};

type Shared = Arc<Mutex<Box<dyn EventSender<Signal>>>>;

fn signal(kind: SignalKind, confidence: Confidence, source: &str) -> Signal {
    Signal {
        kind,
        confidence,
        source: source.to_owned(),
        capturing_app: None,
        display: None,
    }
}

fn send(shared: &Shared, signal: Signal) -> bool {
    shared
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .send(signal)
}
