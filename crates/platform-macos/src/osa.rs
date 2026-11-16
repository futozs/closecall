//! In-process AppleScript through `NSAppleScript`. In-process rather than `osascript` so the
//! Automation prompt names Close Call and no process is spawned per call.

use std::sync::{Mutex, MutexGuard, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::{AllocAnyThread, msg_send};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleScript, NSAppleScriptErrorMessage, NSAppleScriptErrorNumber,
    NSDictionary, NSNumber, NSString,
};
use serde_json::Value;

use crate::script;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OsaError {
    NotPermitted,
    NeedsConsent,
    Timeout,
    NotRunning,
    /// Another script held the interpreter past the deadline.
    Busy,
    Script {
        number: i64,
        message: String,
    },
}

impl OsaError {
    /// -1728 "can't get", -1719 "invalid index": the object we addressed no longer exists.
    pub(crate) fn is_missing_object(&self) -> bool {
        matches!(
            self,
            Self::Script {
                number: -1728 | -1719,
                ..
            }
        )
    }
}

impl std::fmt::Display for OsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPermitted => f.write_str("Automation not permitted"),
            Self::NeedsConsent => f.write_str("Automation not yet granted"),
            Self::Timeout => f.write_str("Apple Event timed out"),
            Self::NotRunning => f.write_str("app is not running"),
            Self::Busy => f.write_str("AppleScript busy"),
            Self::Script { number, message } => write!(f, "AppleScript error {number}: {message}"),
        }
    }
}

// NSAppleScript must not run on two threads at once: concurrent scripts get each other's
// results (seen in this crate's tests). Every provider has its own thread.
static GATE: Mutex<()> = Mutex::new(());

fn acquire(deadline: Instant) -> Option<MutexGuard<'static, ()>> {
    loop {
        match GATE.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(TryLockError::WouldBlock) => return None,
        }
    }
}

/// Runs `body` inside `tell application id …` with a `with timeout` derived from `deadline`.
/// Returns `Value::Null` when the app is not running.
pub(crate) fn run_in_app(
    bundle_id: &str,
    body: &str,
    deadline: Instant,
) -> Result<Value, OsaError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(OsaError::Timeout);
    }
    run(
        &script::in_app(bundle_id, script::timeout_secs(remaining), body),
        deadline,
    )
}

fn run(source: &str, deadline: Instant) -> Result<Value, OsaError> {
    let _guard = acquire(deadline).ok_or(OsaError::Busy)?;
    autoreleasepool(|_| execute(source))
}

fn execute(source: &str) -> Result<Value, OsaError> {
    let script = NSAppleScript::initWithSource(NSAppleScript::alloc(), &NSString::from_str(source))
        .ok_or_else(|| OsaError::Script {
            number: 0,
            message: "script rejected".to_owned(),
        })?;
    let mut info: Option<Retained<NSDictionary<NSString, AnyObject>>> = None;
    // SAFETY: `executeAndReturnError:` takes an `NSDictionary **` out-parameter. The generated
    // binding declares a non-null result, but the method returns nil on failure, so the
    // result is declared optional here.
    let result: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![&*script, executeAndReturnError: &mut info] };
    match result {
        Some(descriptor) => Ok(to_value(&descriptor)),
        None => Err(error_from(info.as_deref())),
    }
}

fn error_from(info: Option<&NSDictionary<NSString, AnyObject>>) -> OsaError {
    let Some(info) = info else {
        return OsaError::Script {
            number: 0,
            message: String::new(),
        };
    };
    // SAFETY: the keys are immutable framework constants.
    let (number_key, message_key) =
        unsafe { (NSAppleScriptErrorNumber, NSAppleScriptErrorMessage) };
    let number = info
        .objectForKey(number_key)
        .and_then(|n| n.downcast::<NSNumber>().ok())
        .map_or(0, |n| n.as_i64());
    let message = info
        .objectForKey(message_key)
        .and_then(|m| m.downcast::<NSString>().ok())
        .map(|m| m.to_string())
        .unwrap_or_default();
    match number {
        -1743 => OsaError::NotPermitted,
        -1744 => OsaError::NeedsConsent,
        -1712 => OsaError::Timeout,
        -600 | -609 => OsaError::NotRunning,
        _ => OsaError::Script { number, message },
    }
}

const fn code(c: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*c)
}

const LIST: u32 = code(b"list");
const RECORD: u32 = code(b"reco");
const TRUE: u32 = code(b"true");
const FALSE: u32 = code(b"fals");
const BOOLEAN: u32 = code(b"bool");
const SINT16: u32 = code(b"shor");
const SINT32: u32 = code(b"long");
const SINT64: u32 = code(b"comp");
const UINT32: u32 = code(b"magn");
const UINT64: u32 = code(b"ucom");
const FLOAT32: u32 = code(b"sing");
const FLOAT64: u32 = code(b"doub");
const TYPE: u32 = code(b"type");
const MISSING_VALUE: u32 = code(b"msng");
const NULL: u32 = code(b"null");
const FILE_URL: u32 = code(b"furl");
const ALIAS: u32 = code(b"alis");
const BOOKMARK: u32 = code(b"bmrk");

fn number(n: f64) -> Value {
    const EXACT: f64 = 9_007_199_254_740_992.0;
    if n.fract() == 0.0 && n.abs() < EXACT {
        #[allow(clippy::cast_possible_truncation)]
        Value::from(n as i64)
    } else {
        serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
    }
}

fn fourcc(code: u32) -> String {
    String::from_utf8_lossy(&code.to_be_bytes()).into_owned()
}

fn to_value(descriptor: &NSAppleEventDescriptor) -> Value {
    let items = || {
        (1..=descriptor.numberOfItems())
            .filter_map(|i| descriptor.descriptorAtIndex(i).map(|d| (i, d)))
    };
    match descriptor.descriptorType() {
        LIST => Value::Array(items().map(|(_, item)| to_value(&item)).collect()),
        RECORD => Value::Object(
            items()
                .map(|(i, item)| {
                    (
                        fourcc(descriptor.keywordForDescriptorAtIndex(i)),
                        to_value(&item),
                    )
                })
                .collect(),
        ),
        TRUE => Value::Bool(true),
        FALSE => Value::Bool(false),
        BOOLEAN => Value::Bool(descriptor.booleanValue() != 0),
        SINT16 | SINT32 => Value::from(descriptor.int32Value()),
        SINT64 | UINT32 | UINT64 | FLOAT32 | FLOAT64 => number(descriptor.doubleValue()),
        TYPE if descriptor.typeCodeValue() == MISSING_VALUE => Value::Null,
        NULL => Value::Null,
        FILE_URL | ALIAS | BOOKMARK => descriptor
            .fileURLValue()
            .and_then(|url| url.path())
            .map_or(Value::Null, |path| Value::String(path.to_string())),
        _ => descriptor
            .stringValue()
            .map_or(Value::Null, |s| Value::String(s.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // These run AppleScript in-process without addressing any other app, so they need no
    // permission and touch nothing.
    fn eval(source: &str) -> Result<Value, OsaError> {
        run(source, Instant::now() + Duration::from_secs(30))
    }

    #[test]
    fn converts_results_to_json() {
        let value = eval(r#"return {1, "Számla", true, false, missing value, 2.5, {3, {}}}"#);
        assert_eq!(
            value,
            Ok(json!([1, "Számla", true, false, null, 2.5, [3, []]]))
        );
    }

    #[test]
    fn failed_scripts_return_errors_instead_of_panicking() {
        assert!(
            eval(r#"error "gone" number -1728"#)
                .unwrap_err()
                .is_missing_object()
        );
        assert_eq!(eval("error number -1743"), Err(OsaError::NotPermitted));
        assert_eq!(eval("error number -1712"), Err(OsaError::Timeout));
        assert!(matches!(
            eval("this is not applescript ("),
            Err(OsaError::Script { .. })
        ));
    }

    #[test]
    fn concurrent_callers_get_their_own_results() {
        let threads: Vec<_> = (0..8)
            .map(|n| std::thread::spawn(move || (n, eval(&format!("return {n} * 10")))))
            .collect();
        for thread in threads {
            let (n, value) = thread.join().unwrap();
            assert_eq!(value, Ok(json!(n * 10)));
        }
    }

    #[test]
    fn quoted_strings_survive_the_real_interpreter() {
        for text in [
            r#"a"b\c"#,
            "line\nnext\ttab\rcr",
            "x\u{1}y\u{1f}z",
            "Számla – „x”",
            "\" & (do shell script \"echo pwned\") & \"",
        ] {
            let value = eval(&format!("return {}", script::quote(text)));
            assert_eq!(value, Ok(json!(text)), "{text:?}");
        }
    }
}
