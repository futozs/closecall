//! "A call is probably starting": the default microphone is in use by some process.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use closecall_core::Confidence;
use closecall_core::platform::{
    EventSender, Signal, SignalError, SignalKind, SignalSource, SourceHealth,
};
use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectPropertySelector, kAudioDevicePropertyDeviceIsRunningSomewhere,
    kAudioHardwarePropertyDefaultInputDevice, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
};

use super::signal;
use crate::keys;

pub(crate) const ID: &str = "macos.call";
const INTERVAL: Duration = Duration::from_secs(1);

fn read_u32(object: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<u32> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: both properties are 32-bit values and every pointer is valid for the call.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&mut address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    (status == 0).then_some(value)
}

/// `None` when there is no input device.
fn microphone_in_use() -> Option<bool> {
    let device = read_u32(
        kAudioObjectSystemObject as AudioObjectID,
        kAudioHardwarePropertyDefaultInputDevice,
    )
    .filter(|&device| device != 0)?;
    read_u32(device, kAudioDevicePropertyDeviceIsRunningSomewhere).map(|running| running != 0)
}

pub(crate) struct CallSource {
    health: Arc<Mutex<SourceHealth>>,
    stop: Arc<AtomicBool>,
}

impl CallSource {
    pub(crate) fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(SourceHealth::Ok)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for CallSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn health_for(reading: Option<bool>) -> SourceHealth {
    match reading {
        Some(_) => SourceHealth::Ok,
        None => SourceHealth::Unavailable {
            reason_key: keys::HEALTH_NO_INPUT_DEVICE.to_owned(),
        },
    }
}

impl SignalSource for CallSource {
    fn id(&self) -> String {
        ID.to_owned()
    }

    fn health(&self) -> SourceHealth {
        self.health
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    // Polled rather than a HAL listener, because the default input device itself changes
    // (headset plugged in) and polling covers both without re-registering.
    fn start(&mut self, signals: Box<dyn EventSender<Signal>>) -> Result<(), SignalError> {
        let (health, stop) = (Arc::clone(&self.health), Arc::clone(&self.stop));
        thread::Builder::new()
            .name(ID.to_owned())
            .spawn(move || {
                let mut in_use = false;
                while !stop.load(Ordering::Relaxed) {
                    let reading = microphone_in_use();
                    *health.lock().unwrap_or_else(PoisonError::into_inner) = health_for(reading);
                    let now = reading.unwrap_or(false);
                    if now != in_use {
                        in_use = now;
                        let kind = if now {
                            SignalKind::CallLikely
                        } else {
                            SignalKind::CallEnded
                        };
                        if !signals.send(signal(kind, Confidence::Medium, ID)) {
                            return;
                        }
                    }
                    thread::sleep(INTERVAL);
                }
            })
            .map_err(|e| SignalError::Other(e.to_string()))?;
        Ok(())
    }
}
