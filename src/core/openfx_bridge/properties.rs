//! Property sets and C ABI property suite implementations for OpenFX.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Property values storable in an OFX property set.
#[derive(Debug, Clone, PartialEq)]
pub enum OfxPropValue {
    Int(i32),
    Double(f64),
    Str(String),
}

/// Minimal property-set storage backing the host's OfxPropertySuiteV1.
/// Handles passed through the C ABI are raw pointers to this struct.
#[derive(Default, Debug)]
pub struct OfxPropertySet {
    props: HashMap<String, Vec<OfxPropValue>>,
}

impl OfxPropertySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_int(&mut self, prop: &str, index: usize, v: i32) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Int(0));
        }
        match &mut slot[index] {
            OfxPropValue::Int(old) => {
                *old = v;
                true
            }
            slot => {
                *slot = OfxPropValue::Int(v);
                true
            }
        }
    }

    pub fn set_double(&mut self, prop: &str, index: usize, v: f64) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Double(0.0));
        }
        match &mut slot[index] {
            OfxPropValue::Double(old) => {
                *old = v;
                true
            }
            slot => {
                *slot = OfxPropValue::Double(v);
                true
            }
        }
    }

    pub fn set_string(&mut self, prop: &str, index: usize, v: &str) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Str(String::new()));
        }
        slot[index] = OfxPropValue::Str(v.to_string());
        true
    }

    pub fn get_int(&self, prop: &str, index: usize) -> Option<i32> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_double(&self, prop: &str, index: usize) -> Option<f64> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Double(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, prop: &str, index: usize) -> Option<&str> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

// OFX status codes used by the handshake (ofxCore.h)
pub const K_OFX_STAT_OK: i32 = 0;
pub const K_OFX_STAT_FAILED: i32 = 1;
pub const K_OFX_STAT_ERR_FATAL: i32 = 2;
pub const K_OFX_STAT_ERR_UNKNOWN: i32 = 3;
pub const K_OFX_STAT_ERR_MISSING_HOST_FEATURE: i32 = 4;
pub const K_OFX_STAT_ERR_BAD_HANDLE: i32 = 9;
pub const K_OFX_STAT_ERR_BAD_INDEX: i32 = 10;
pub const K_OFX_STAT_ERR_VALUE: i32 = 11;
pub const K_OFX_STAT_REPLY_YES: i32 = 12;
pub const K_OFX_STAT_REPLY_NO: i32 = 13;
pub const K_OFX_STAT_REPLY_DEFAULT: i32 = 14;
const MAX_PROPERTY_INDEX: i32 = 4095;

fn property_index(index: i32) -> Option<usize> {
    (0..=MAX_PROPERTY_INDEX)
        .contains(&index)
        .then_some(index as usize)
}

fn property_set_handles() -> &'static Mutex<std::collections::HashSet<usize>> {
    static HANDLES: OnceLock<Mutex<std::collections::HashSet<usize>>> = OnceLock::new();
    HANDLES.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

pub(crate) fn register_property_set_handle(handle: *mut OfxPropertySet) {
    if !handle.is_null() {
        property_set_handles()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(handle as usize);
    }
}

pub(crate) fn unregister_property_set_handle(handle: *mut OfxPropertySet) {
    if !handle.is_null() {
        property_set_handles()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(handle as usize));
    }
}

fn with_property_set_handle<R>(
    handle: *mut std::os::raw::c_void,
    apply: impl FnOnce(&mut OfxPropertySet) -> R,
) -> Option<R> {
    if handle.is_null() {
        return None;
    }
    let handles = property_set_handles()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !handles.contains(&(handle as usize)) {
        return None;
    }
    // The registry lock serializes callbacks and unregister-on-drop, so a
    // registered boxed property set cannot be freed during this access.
    let set = unsafe { (handle as *mut OfxPropertySet).as_mut()? };
    Some(apply(set))
}

pub(crate) unsafe fn cstr<'a>(p: *const std::os::raw::c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(p).to_str().ok()
}

pub(crate) unsafe extern "C" fn prop_set_int(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: i32,
) -> i32 {
    let Some(index) = property_index(index) else {
        return K_OFX_STAT_ERR_BAD_INDEX;
    };
    let Some(name) = (unsafe { cstr(prop) }) else {
        return K_OFX_STAT_ERR_VALUE;
    };
    let Some(result) = with_property_set_handle(handle, |set| set.set_int(name, index, value))
    else {
        return K_OFX_STAT_ERR_BAD_HANDLE;
    };
    if result {
        K_OFX_STAT_OK
    } else {
        K_OFX_STAT_FAILED
    }
}

pub(crate) unsafe extern "C" fn prop_set_double(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: f64,
) -> i32 {
    let Some(index) = property_index(index) else {
        return K_OFX_STAT_ERR_BAD_INDEX;
    };
    let Some(name) = (unsafe { cstr(prop) }) else {
        return K_OFX_STAT_ERR_VALUE;
    };
    let Some(result) = with_property_set_handle(handle, |set| set.set_double(name, index, value))
    else {
        return K_OFX_STAT_ERR_BAD_HANDLE;
    };
    if result {
        K_OFX_STAT_OK
    } else {
        K_OFX_STAT_FAILED
    }
}

pub(crate) unsafe extern "C" fn prop_set_string(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: *const std::os::raw::c_char,
) -> i32 {
    let Some(index) = property_index(index) else {
        return K_OFX_STAT_ERR_BAD_INDEX;
    };
    let (Some(name), Some(val)) = (unsafe { cstr(prop) }, unsafe { cstr(value) }) else {
        return K_OFX_STAT_ERR_VALUE;
    };
    let Some(result) = with_property_set_handle(handle, |set| set.set_string(name, index, val))
    else {
        return K_OFX_STAT_ERR_BAD_HANDLE;
    };
    if result {
        K_OFX_STAT_OK
    } else {
        K_OFX_STAT_FAILED
    }
}

/// `OfxPropertySuiteV1` field order follows the OpenFX C ABI. Unimplemented
/// entries remain null so plugins can detect unsupported suite operations.
#[repr(C)]
pub struct OfxPropertySuiteV1 {
    pub prop_set_pointer: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        *mut std::os::raw::c_void,
    ) -> i32,
    pub prop_set_string: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        *const std::os::raw::c_char,
    ) -> i32,
    pub prop_set_double: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        f64,
    ) -> i32,
    pub prop_set_int: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        i32,
    ) -> i32,
    pub prop_set_pointer_n: usize,
    pub prop_set_string_n: usize,
    pub prop_set_double_n: usize,
    pub prop_set_int_n: usize,
    pub prop_get_pointer: usize,
    pub prop_get_string: usize,
    pub prop_get_double: usize,
    pub prop_get_int: usize,
    pub prop_get_pointer_n: usize,
    pub prop_get_string_n: usize,
    pub prop_get_double_n: usize,
    pub prop_get_int_n: usize,
    pub prop_reset: usize,
    pub prop_get_dimension: usize,
}

pub fn property_suite_raw() -> &'static OfxPropertySuiteV1 {
    use std::sync::OnceLock;
    static SUITE: OnceLock<OfxPropertySuiteV1> = OnceLock::new();
    SUITE.get_or_init(|| OfxPropertySuiteV1 {
        prop_set_pointer: noop_set_pointer,
        prop_set_string,
        prop_set_double,
        prop_set_int,
        prop_set_pointer_n: 0,
        prop_set_string_n: 0,
        prop_set_double_n: 0,
        prop_set_int_n: 0,
        prop_get_pointer: 0,
        prop_get_string: 0,
        prop_get_double: 0,
        prop_get_int: 0,
        prop_get_pointer_n: 0,
        prop_get_string_n: 0,
        prop_get_double_n: 0,
        prop_get_int_n: 0,
        prop_reset: 0,
        prop_get_dimension: 0,
    })
}

unsafe extern "C" fn noop_set_pointer(
    _h: *mut std::os::raw::c_void,
    _p: *const std::os::raw::c_char,
    _i: i32,
    _v: *mut std::os::raw::c_void,
) -> i32 {
    3 // kOfxStatErrUnsupported
}
