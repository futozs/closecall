//! Pure domain logic: no I/O, no threads, no OS calls. Time is passed in, never read,
//! so planning is deterministic.

pub mod config;
pub mod duration;
pub mod ipc;
pub mod matcher;
pub mod model;
pub mod plan;
pub mod platform;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod text;

pub use duration::Dur;
pub use model::*;
