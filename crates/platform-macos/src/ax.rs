//! Thin wrapper over `AXUIElement`. Elements are not `Send`, so they never leave the thread
//! that created them; windows are found again by pid and `CGWindowID`.

use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::sync::OnceLock;

use objc2_application_services::{
    AXError, AXIsProcessTrusted, AXIsProcessTrustedWithOptions, AXUIElement,
    kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFType};

use crate::sheet;

/// AX defaults to several seconds; a hung app must not stall a whole scan.
const MESSAGING_TIMEOUT: f32 = 0.5;

pub(crate) fn is_trusted() -> bool {
    // SAFETY: no preconditions.
    unsafe { AXIsProcessTrusted() }
}

/// Shows the system "would like to control this computer" prompt if not yet trusted.
pub(crate) fn prompt_for_trust() -> bool {
    // SAFETY: framework constant.
    let key: &CFString = unsafe { kAXTrustedCheckOptionPrompt };
    let options = CFDictionary::<CFString, CFBoolean>::from_slices(&[key], &[CFBoolean::new(true)]);
    // SAFETY: the dictionary has the documented key and a CFBoolean value.
    unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) }
}

pub(crate) struct Ax(CFRetained<AXUIElement>);

impl Ax {
    pub(crate) fn application(pid: i32) -> Self {
        // SAFETY: any pid is accepted; a dead one only makes later calls fail.
        let element = unsafe { AXUIElement::new_application(pid) };
        Self::adopt(element)
    }

    pub(crate) fn raw(&self) -> &AXUIElement {
        &self.0
    }

    fn copy(&self, attribute: &str) -> Result<CFRetained<CFType>, AXError> {
        let name = CFString::from_str(attribute);
        let mut value: *const CFType = ptr::null();
        // SAFETY: `value` is a valid out-pointer; on success it holds a +1 reference.
        let error = unsafe {
            self.0
                .copy_attribute_value(&name, NonNull::from(&mut value))
        };
        if error != AXError::Success {
            return Err(error);
        }
        // SAFETY: Copy rule: we own the returned reference.
        NonNull::new(value.cast_mut())
            .map(|value| unsafe { CFRetained::from_raw(value) })
            .ok_or(AXError::NoValue)
    }

    pub(crate) fn string(&self, attribute: &str) -> Option<String> {
        let value = self.copy(attribute).ok()?.downcast::<CFString>().ok()?;
        Some(value.to_string())
    }

    pub(crate) fn boolean(&self, attribute: &str) -> Option<bool> {
        let value = self.copy(attribute).ok()?.downcast::<CFBoolean>().ok()?;
        Some(value.as_bool())
    }

    /// The timeout belongs to each element object, so every child gets its own: a window or
    /// button of a hung app would otherwise wait the system default of about six seconds.
    fn adopt(element: CFRetained<AXUIElement>) -> Self {
        // SAFETY: plain setter on a valid element.
        unsafe { element.set_messaging_timeout(MESSAGING_TIMEOUT) };
        Self(element)
    }

    pub(crate) fn element(&self, attribute: &str) -> Option<Ax> {
        let value = self.copy(attribute).ok()?.downcast::<AXUIElement>().ok()?;
        Some(Self::adopt(value))
    }

    pub(crate) fn elements(&self, attribute: &str) -> Result<Vec<Ax>, AXError> {
        let value = self.copy(attribute)?;
        let array = value
            .downcast_ref::<CFArray>()
            .ok_or(AXError::IllegalArgument)?;
        // SAFETY: AX element arrays hold CF objects only.
        let array = unsafe { array.cast_unchecked::<CFType>() };
        Ok(array
            .to_vec()
            .into_iter()
            .filter_map(|item| item.downcast::<AXUIElement>().ok().map(Self::adopt))
            .collect())
    }

    /// False once the element's window or app is gone.
    pub(crate) fn is_alive(&self) -> bool {
        !matches!(self.copy("AXRole"), Err(AXError::InvalidUIElement))
    }

    pub(crate) fn set_bool(&self, attribute: &str, value: bool) -> bool {
        let name = CFString::from_str(attribute);
        // SAFETY: CFBoolean is a valid CFType value for boolean attributes.
        unsafe { self.0.set_attribute_value(&name, CFBoolean::new(value)) == AXError::Success }
    }

    pub(crate) fn press(&self) -> bool {
        // SAFETY: plain action on a valid element.
        unsafe { self.0.perform_action(&CFString::from_static_str("AXPress")) == AXError::Success }
    }

    pub(crate) fn window_id(&self) -> Option<u32> {
        let get_window = get_window_fn()?;
        let mut id = 0u32;
        // SAFETY: `get_window` has the signature declared below; `id` is a valid out-pointer.
        let error = unsafe { get_window(CFRetained::as_ptr(&self.0).as_ptr(), &mut id) };
        (error == AXError::Success.0 && id != 0).then_some(id)
    }
}

impl sheet::Node for Ax {
    fn role(&self) -> Option<String> {
        self.string("AXRole")
    }

    fn children(&self) -> Vec<Self> {
        self.elements("AXChildren").unwrap_or_default()
    }

    fn same(&self, other: &Self) -> bool {
        let (a, b): (&CFType, &CFType) = (&self.0, &other.0);
        a == b
    }
}

type GetWindow = unsafe extern "C-unwind" fn(*const AXUIElement, *mut u32) -> i32;

/// Private, but the only way to tie an AX window to a stable `CGWindowID`. Looked up at
/// runtime so a future macOS without it degrades to index-based handles instead of failing
/// to launch.
fn get_window_fn() -> Option<GetWindow> {
    static FN: OnceLock<Option<GetWindow>> = OnceLock::new();
    *FN.get_or_init(|| {
        // SAFETY: dlsym only looks the name up.
        let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"_AXUIElementGetWindow".as_ptr()) };
        // SAFETY: `_AXUIElementGetWindow(AXUIElementRef, CGWindowID *) -> AXError` has had this
        // signature since it appeared; yabai and AltTab depend on it too.
        (!symbol.is_null())
            .then(|| unsafe { std::mem::transmute::<*mut c_void, GetWindow>(symbol) })
    })
}
