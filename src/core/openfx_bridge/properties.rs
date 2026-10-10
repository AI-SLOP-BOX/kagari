//! Property sets and C ABI property suite implementations for OpenFX.

use std::collections::HashMap;

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
pub const K_OFX_STAT_REPLY_YES: i32 = 1;
pub const K_OFX_STAT_REPLY_DEFAULT: i32 = 2;
pub const K_OFX_STAT_ERR_UNSET: i32 = 4; // what plugins return when ignoring an action

pub(crate) unsafe fn handle_as_set<'a>(
    handle: *mut std::os::raw::c_void,
) -> Option<&'a mut OfxPropertySet> {
    (handle as *mut OfxPropertySet).as_mut()
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
    let (Some(set), Some(name)) = (handle_as_set(handle), cstr(prop)) else {
        return 1; // kOfxStatErrBadHandle-ish; real code kept minimal
    };
    if set.set_int(name, index.max(0) as usize, value) {
        K_OFX_STAT_OK
    } else {
        1
    }
}

pub(crate) unsafe extern "C" fn prop_set_double(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: f64,
) -> i32 {
    let (Some(set), Some(name)) = (handle_as_set(handle), cstr(prop)) else {
        return 1;
    };
    if set.set_double(name, index.max(0) as usize, value) {
        K_OFX_STAT_OK
    } else {
        1
    }
}

pub(crate) unsafe extern "C" fn prop_set_string(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: *const std::os::raw::c_char,
) -> i32 {
    let (Some(set), Some(name), Some(val)) = (handle_as_set(handle), cstr(prop), cstr(value))
    else {
        return 1;
    };
    if set.set_string(name, index.max(0) as usize, val) {
        K_OFX_STAT_OK
    } else {
        1
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
