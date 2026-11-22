//! OpenFX host/plugin loading, probing, and execution interface.

use super::properties::{
    cstr, register_property_set_handle, unregister_property_set_handle, OfxPropertySet,
};
use std::path::Path;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct OfxImageEffectHandle {
    pub effect_id: String,
    pub plugin_name: String,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfxStatus {
    OK = 0,
    Failed = 1,
    ErrMemory = 2,
}

/// Bridge interface representing an external OpenFX visual effect plugin.
pub struct OpenFxPluginBridge {
    pub plugin_name: String,
    pub handle: OfxImageEffectHandle,
}

impl OpenFxPluginBridge {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            handle: OfxImageEffectHandle {
                effect_id: format!("ofx.plugin.{}", plugin_name.to_lowercase()),
                plugin_name: plugin_name.to_string(),
            },
        }
    }

    /// Render one RGBA8 frame through an attached OpenFX plugin if registered,
    /// or return `OfxStatus::Failed` when no host-attached plugin is available.
    pub fn render_frame(
        &self,
        in_pixels: &[u8],
        out_pixels: &mut [u8],
        width: u32,
        height: u32,
        frame: f64,
    ) -> OfxStatus {
        if in_pixels.len() != out_pixels.len() {
            return OfxStatus::ErrMemory;
        }
        let mut req = OfxRenderRequest {
            width,
            height,
            frame,
            input: in_pixels,
            output: out_pixels,
        };
        let mut guard = match plugin_registry().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let Some(plugin) = guard.iter_mut().find(|p| p.name == self.plugin_name) else {
            return OfxStatus::Failed;
        };
        match plugin.render(&mut req) {
            Ok(()) => OfxStatus::OK,
            Err(_) => OfxStatus::Failed,
        }
    }
}

#[repr(C)]
pub struct OfxHost {
    pub host: *mut OfxPropertySet,
    pub fetch_suite: Option<
        unsafe extern "C" fn(
            host: *mut OfxPropertySet,
            suite_name: *const std::ffi::c_char,
            suite_version: std::ffi::c_int,
        ) -> *const std::ffi::c_void,
    >,
}

unsafe impl Send for OfxHost {}

#[repr(C)]
pub struct OfxPlugin {
    pub plugin_api: *const std::ffi::c_char,
    pub api_version: std::ffi::c_int,
    pub plugin_identifier: *const std::ffi::c_char,
    pub plugin_version_major: std::ffi::c_uint,
    pub plugin_version_minor: std::ffi::c_uint,
    pub set_host: Option<unsafe extern "C" fn(host: *mut OfxHost)>,
    pub main_entry: Option<
        unsafe extern "C" fn(
            action: *const std::ffi::c_char,
            handle: *const std::ffi::c_void,
            in_args: *mut OfxPropertySet,
            out_args: *mut OfxPropertySet,
        ) -> std::ffi::c_int,
    >,
}

#[repr(C)]
pub struct OfxImageEffectSuiteV1 {
    pub get_property_set: unsafe extern "C" fn(
        image_effect: *const std::ffi::c_void,
        prop_handle: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub get_param_set: unsafe extern "C" fn(
        image_effect: *const std::ffi::c_void,
        param_set: *mut *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub clip_define: unsafe extern "C" fn(
        image_effect: *const std::ffi::c_void,
        name: *const std::ffi::c_char,
        property_set: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub clip_get_handle: unsafe extern "C" fn(
        image_effect: *const std::ffi::c_void,
        name: *const std::ffi::c_char,
        clip: *mut *mut std::ffi::c_void,
        property_set: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub clip_get_property_set: unsafe extern "C" fn(
        clip: *const std::ffi::c_void,
        prop_handle: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub clip_get_image: unsafe extern "C" fn(
        clip: *const std::ffi::c_void,
        time: f64,
        region: *const std::ffi::c_void,
        image_handle: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub clip_release_image:
        unsafe extern "C" fn(image_handle: *mut OfxPropertySet) -> std::ffi::c_int,
    pub clip_get_region_of_definition: unsafe extern "C" fn(
        clip: *const std::ffi::c_void,
        time: f64,
        bounds: *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub abort: unsafe extern "C" fn(image_effect: *const std::ffi::c_void) -> std::ffi::c_int,
    pub image_memory_alloc: unsafe extern "C" fn(
        instance_handle: *const std::ffi::c_void,
        n_bytes: usize,
        memory_handle: *mut *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub image_memory_free:
        unsafe extern "C" fn(memory_handle: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub image_memory_lock: unsafe extern "C" fn(
        memory_handle: *mut std::ffi::c_void,
        returned_ptr: *mut *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub image_memory_unlock:
        unsafe extern "C" fn(memory_handle: *mut std::ffi::c_void) -> std::ffi::c_int,
}

unsafe extern "C" fn ofx_get_property_set(
    image_effect: *const std::ffi::c_void,
    prop_handle: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if image_effect.is_null() || prop_handle.is_null() {
        return 3; // kOfxStatErrBadHandle
    }
    unsafe { *prop_handle = image_effect as *mut OfxPropertySet };
    0
}

unsafe extern "C" fn ofx_stub_ok_2(
    _a: *const std::ffi::c_void,
    _b: *mut *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_clip_define(
    _image_effect: *const std::ffi::c_void,
    _name: *const std::ffi::c_char,
    _property_set: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    0
}

std::thread_local! {
    static ACTIVE_OFX_IMAGES: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

unsafe extern "C" fn ofx_clip_get_handle(
    _image_effect: *const std::ffi::c_void,
    name: *const std::ffi::c_char,
    clip: *mut *mut std::ffi::c_void,
    _property_set: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if clip.is_null() {
        return 3;
    }
    let is_output = !name.is_null() && unsafe { cstr(name) } == Some("Output");
    unsafe { *clip = (if is_output { 2usize } else { 1usize }) as *mut std::ffi::c_void };
    0
}

unsafe extern "C" fn ofx_clip_get_property_set(
    _clip: *const std::ffi::c_void,
    _prop_handle: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_clip_get_image(
    clip: *const std::ffi::c_void,
    _time: f64,
    _region: *const std::ffi::c_void,
    image_handle: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if image_handle.is_null() {
        return 3;
    }
    let (in_ptr, out_ptr) = ACTIVE_OFX_IMAGES.with(|c| c.get());
    let chosen = if clip as usize == 2 { out_ptr } else { in_ptr };
    if chosen == 0 {
        return 1; // kOfxStatFailed
    }
    unsafe { *image_handle = chosen as *mut OfxPropertySet };
    0
}

unsafe extern "C" fn ofx_clip_release_image(
    _image_handle: *mut OfxPropertySet,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_clip_get_rod(
    _clip: *const std::ffi::c_void,
    _time: f64,
    _bounds: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_abort(_image_effect: *const std::ffi::c_void) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_image_memory_alloc(
    _instance_handle: *const std::ffi::c_void,
    _n_bytes: usize,
    _memory_handle: *mut *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_image_memory_free(
    _memory_handle: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_image_memory_lock(
    _memory_handle: *mut std::ffi::c_void,
    _returned_ptr: *mut *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_image_memory_unlock(
    _memory_handle: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    0
}

static IMAGE_EFFECT_SUITE_V1: OfxImageEffectSuiteV1 = OfxImageEffectSuiteV1 {
    get_property_set: ofx_get_property_set,
    get_param_set: ofx_stub_ok_2,
    clip_define: ofx_clip_define,
    clip_get_handle: ofx_clip_get_handle,
    clip_get_property_set: ofx_clip_get_property_set,
    clip_get_image: ofx_clip_get_image,
    clip_release_image: ofx_clip_release_image,
    clip_get_region_of_definition: ofx_clip_get_rod,
    abort: ofx_abort,
    image_memory_alloc: ofx_image_memory_alloc,
    image_memory_free: ofx_image_memory_free,
    image_memory_lock: ofx_image_memory_lock,
    image_memory_unlock: ofx_image_memory_unlock,
};

#[repr(C)]
pub struct OfxParamSuiteV1 {
    pub param_define: unsafe extern "C" fn(
        param_set: *const std::ffi::c_void,
        param_type: *const std::ffi::c_char,
        name: *const std::ffi::c_char,
        property_set: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub param_get_handle: unsafe extern "C" fn(
        param_set: *const std::ffi::c_void,
        name: *const std::ffi::c_char,
        param: *mut *mut std::ffi::c_void,
        property_set: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub param_set_get_property_set: unsafe extern "C" fn(
        param_set: *const std::ffi::c_void,
        prop_handle: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
    pub param_get_property_set: unsafe extern "C" fn(
        param: *const std::ffi::c_void,
        prop_handle: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfxParamDesc {
    pub name: String,
    pub param_type: String,
}

fn defined_ofx_params() -> &'static std::sync::Mutex<Vec<OfxParamDesc>> {
    static PARAMS: std::sync::OnceLock<std::sync::Mutex<Vec<OfxParamDesc>>> =
        std::sync::OnceLock::new();
    PARAMS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Return all OpenFX parameters registered via `OfxParamSuiteV1::paramDefine`.
pub fn list_defined_ofx_params() -> Vec<OfxParamDesc> {
    let guard = match defined_ofx_params().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.clone()
}

unsafe extern "C" fn ofx_param_define(
    _param_set: *const std::ffi::c_void,
    param_type: *const std::ffi::c_char,
    name: *const std::ffi::c_char,
    _property_set: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if let (Some(pt), Some(n)) = (unsafe { cstr(param_type) }, unsafe { cstr(name) }) {
        let mut guard = match defined_ofx_params().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if !guard.iter().any(|p| p.name == n) {
            guard.push(OfxParamDesc {
                name: n.to_string(),
                param_type: pt.to_string(),
            });
        }
    }
    0
}

unsafe extern "C" fn ofx_param_get_handle(
    _param_set: *const std::ffi::c_void,
    _name: *const std::ffi::c_char,
    param: *mut *mut std::ffi::c_void,
    _property_set: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if !param.is_null() {
        unsafe { *param = std::ptr::dangling_mut::<std::ffi::c_void>() };
    }
    0
}

unsafe extern "C" fn ofx_param_set_get_property_set(
    _param_set: *const std::ffi::c_void,
    _prop_handle: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    0
}

unsafe extern "C" fn ofx_param_get_property_set(
    _param: *const std::ffi::c_void,
    _prop_handle: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    0
}

static PARAM_SUITE_V1: OfxParamSuiteV1 = OfxParamSuiteV1 {
    param_define: ofx_param_define,
    param_get_handle: ofx_param_get_handle,
    param_set_get_property_set: ofx_param_set_get_property_set,
    param_get_property_set: ofx_param_get_property_set,
};

#[repr(C)]
pub struct OfxMemorySuiteV1 {
    pub memory_alloc: unsafe extern "C" fn(
        handle: *mut std::ffi::c_void,
        n_bytes: usize,
        allocated_data: *mut *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub memory_free: unsafe extern "C" fn(allocated_data: *mut std::ffi::c_void) -> std::ffi::c_int,
}

#[repr(C)]
pub struct OfxMultiThreadSuiteV1 {
    pub multi_thread: unsafe extern "C" fn(
        func: Option<
            unsafe extern "C" fn(
                thread_index: std::ffi::c_uint,
                thread_max: std::ffi::c_uint,
                custom_arg: *mut std::ffi::c_void,
            ),
        >,
        n_threads: std::ffi::c_uint,
        custom_arg: *mut std::ffi::c_void,
    ) -> std::ffi::c_int,
    pub multi_thread_num_cpus:
        unsafe extern "C" fn(n_cpus: *mut std::ffi::c_uint) -> std::ffi::c_int,
    pub multi_thread_index:
        unsafe extern "C" fn(thread_index: *mut std::ffi::c_uint) -> std::ffi::c_int,
    pub multi_thread_is_spawned_thread: unsafe extern "C" fn() -> std::ffi::c_int,
    pub mutex_create: unsafe extern "C" fn(
        mutex: *mut *mut std::ffi::c_void,
        lock_count: std::ffi::c_int,
    ) -> std::ffi::c_int,
    pub mutex_destroy: unsafe extern "C" fn(mutex: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub mutex_lock: unsafe extern "C" fn(mutex: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub mutex_unlock: unsafe extern "C" fn(mutex: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub mutex_try_lock: unsafe extern "C" fn(mutex: *mut std::ffi::c_void) -> std::ffi::c_int,
}

static OFX_ALLOCATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<usize, std::alloc::Layout>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

std::thread_local! {
    static OFX_THREAD_STATE: std::cell::Cell<(u32, bool)> = const { std::cell::Cell::new((0, false)) };
}

unsafe extern "C" fn ofx_memory_alloc(
    _handle: *mut std::ffi::c_void,
    n_bytes: usize,
    allocated_data: *mut *mut std::ffi::c_void,
) -> std::ffi::c_int {
    if allocated_data.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let size = n_bytes.max(1);
    let Ok(layout) = std::alloc::Layout::from_size_align(size, 16) else {
        return super::K_OFX_STAT_ERR_FATAL;
    };
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        return super::K_OFX_STAT_ERR_FATAL;
    }
    if let Ok(mut map) = OFX_ALLOCATIONS.lock() {
        map.insert(ptr as usize, layout);
    }
    unsafe {
        *allocated_data = ptr.cast();
    }
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_memory_free(allocated_data: *mut std::ffi::c_void) -> std::ffi::c_int {
    if allocated_data.is_null() {
        return super::K_OFX_STAT_OK;
    }
    let layout = OFX_ALLOCATIONS
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&(allocated_data as usize)));
    if let Some(layout) = layout {
        unsafe { std::alloc::dealloc(allocated_data.cast(), layout) };
        super::K_OFX_STAT_OK
    } else {
        super::K_OFX_STAT_ERR_BAD_HANDLE
    }
}

static MEMORY_SUITE_V1: OfxMemorySuiteV1 = OfxMemorySuiteV1 {
    memory_alloc: ofx_memory_alloc,
    memory_free: ofx_memory_free,
};

unsafe extern "C" fn ofx_multi_thread(
    func: Option<
        unsafe extern "C" fn(
            thread_index: std::ffi::c_uint,
            thread_max: std::ffi::c_uint,
            custom_arg: *mut std::ffi::c_void,
        ),
    >,
    n_threads: std::ffi::c_uint,
    custom_arg: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    let Some(func) = func else {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    };
    let count = n_threads.max(1);
    let arg_addr = custom_arg as usize;
    rayon::scope(|s| {
        for idx in 0..count {
            s.spawn(move |_| {
                OFX_THREAD_STATE.with(|cell| cell.set((idx, true)));
                unsafe { func(idx, count, arg_addr as *mut std::ffi::c_void) };
                OFX_THREAD_STATE.with(|cell| cell.set((0, false)));
            });
        }
    });
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_multi_thread_num_cpus(
    n_cpus: *mut std::ffi::c_uint,
) -> std::ffi::c_int {
    if n_cpus.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    unsafe {
        *n_cpus = count;
    }
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_multi_thread_index(
    thread_index: *mut std::ffi::c_uint,
) -> std::ffi::c_int {
    if thread_index.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let idx = OFX_THREAD_STATE.with(|cell| cell.get().0);
    unsafe {
        *thread_index = idx;
    }
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_multi_thread_is_spawned_thread() -> std::ffi::c_int {
    i32::from(OFX_THREAD_STATE.with(|cell| cell.get().1))
}

struct OfxMutex {
    locked: std::sync::Mutex<bool>,
    cv: std::sync::Condvar,
}

unsafe extern "C" fn ofx_mutex_create(
    mutex: *mut *mut std::ffi::c_void,
    _lock_count: std::ffi::c_int,
) -> std::ffi::c_int {
    if mutex.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let boxed = Box::new(OfxMutex {
        locked: std::sync::Mutex::new(false),
        cv: std::sync::Condvar::new(),
    });
    unsafe {
        *mutex = Box::into_raw(boxed).cast();
    }
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_mutex_destroy(mutex: *mut std::ffi::c_void) -> std::ffi::c_int {
    if mutex.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    unsafe { drop(Box::from_raw(mutex.cast::<OfxMutex>())) };
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_mutex_lock(mutex: *mut std::ffi::c_void) -> std::ffi::c_int {
    if mutex.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let m = unsafe { &*mutex.cast::<OfxMutex>() };
    let mut locked = m.locked.lock().unwrap_or_else(|e| e.into_inner());
    while *locked {
        locked = m.cv.wait(locked).unwrap_or_else(|e| e.into_inner());
    }
    *locked = true;
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_mutex_unlock(mutex: *mut std::ffi::c_void) -> std::ffi::c_int {
    if mutex.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let m = unsafe { &*mutex.cast::<OfxMutex>() };
    let mut locked = m.locked.lock().unwrap_or_else(|e| e.into_inner());
    *locked = false;
    m.cv.notify_one();
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_mutex_try_lock(mutex: *mut std::ffi::c_void) -> std::ffi::c_int {
    if mutex.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    let m = unsafe { &*mutex.cast::<OfxMutex>() };
    let mut locked = m.locked.lock().unwrap_or_else(|e| e.into_inner());
    if *locked {
        super::K_OFX_STAT_ERR_FATAL
    } else {
        *locked = true;
        super::K_OFX_STAT_OK
    }
}

static MULTI_THREAD_SUITE_V1: OfxMultiThreadSuiteV1 = OfxMultiThreadSuiteV1 {
    multi_thread: ofx_multi_thread,
    multi_thread_num_cpus: ofx_multi_thread_num_cpus,
    multi_thread_index: ofx_multi_thread_index,
    multi_thread_is_spawned_thread: ofx_multi_thread_is_spawned_thread,
    mutex_create: ofx_mutex_create,
    mutex_destroy: ofx_mutex_destroy,
    mutex_lock: ofx_mutex_lock,
    mutex_unlock: ofx_mutex_unlock,
    mutex_try_lock: ofx_mutex_try_lock,
};

#[repr(C)]
pub struct OfxMessageSuiteV2 {
    pub message: unsafe extern "C" fn(
        handle: *mut std::ffi::c_void,
        message_type: *const std::ffi::c_char,
        message_id: *const std::ffi::c_char,
        format: *const std::ffi::c_char,
    ) -> std::ffi::c_int,
    pub set_persistent_message: unsafe extern "C" fn(
        handle: *mut std::ffi::c_void,
        message_type: *const std::ffi::c_char,
        message_id: *const std::ffi::c_char,
        format: *const std::ffi::c_char,
    ) -> std::ffi::c_int,
    pub clear_persistent_message:
        unsafe extern "C" fn(handle: *mut std::ffi::c_void) -> std::ffi::c_int,
}

unsafe extern "C" fn ofx_message(
    _handle: *mut std::ffi::c_void,
    message_type: *const std::ffi::c_char,
    message_id: *const std::ffi::c_char,
    format: *const std::ffi::c_char,
) -> std::ffi::c_int {
    let kind = if message_type.is_null() {
        "OfxMessageLog".to_string()
    } else {
        unsafe { cstr(message_type) }.unwrap_or("OfxMessageLog").to_string()
    };
    let id = if message_id.is_null() {
        String::new()
    } else {
        unsafe { cstr(message_id) }.unwrap_or_default().to_string()
    };
    let msg = if format.is_null() {
        String::new()
    } else {
        unsafe { cstr(format) }.unwrap_or_default().to_string()
    };
    match kind.as_str() {
        "OfxMessageError" | "OfxMessageFatal" => log::error!("[OpenFX:{id}] {msg}"),
        "OfxMessageWarning" => log::warn!("[OpenFX:{id}] {msg}"),
        _ => log::info!("[OpenFX:{id}] {msg}"),
    }
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_set_persistent_message(
    handle: *mut std::ffi::c_void,
    message_type: *const std::ffi::c_char,
    message_id: *const std::ffi::c_char,
    format: *const std::ffi::c_char,
) -> std::ffi::c_int {
    unsafe { ofx_message(handle, message_type, message_id, format) }
}

unsafe extern "C" fn ofx_clear_persistent_message(
    _handle: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    super::K_OFX_STAT_OK
}

static MESSAGE_SUITE_V2: OfxMessageSuiteV2 = OfxMessageSuiteV2 {
    message: ofx_message,
    set_persistent_message: ofx_set_persistent_message,
    clear_persistent_message: ofx_clear_persistent_message,
};

#[repr(C)]
pub struct OfxInteractSuiteV1 {
    pub interact_swap_buffers:
        unsafe extern "C" fn(interact_instance: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub interact_redraw:
        unsafe extern "C" fn(interact_instance: *mut std::ffi::c_void) -> std::ffi::c_int,
    pub interact_get_property_set: unsafe extern "C" fn(
        interact_instance: *mut std::ffi::c_void,
        property: *mut *mut OfxPropertySet,
    ) -> std::ffi::c_int,
}

unsafe extern "C" fn ofx_interact_swap_buffers(
    _interact_instance: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_interact_redraw(
    _interact_instance: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    super::K_OFX_STAT_OK
}

unsafe extern "C" fn ofx_interact_get_property_set(
    interact_instance: *mut std::ffi::c_void,
    property: *mut *mut OfxPropertySet,
) -> std::ffi::c_int {
    if property.is_null() {
        return super::K_OFX_STAT_ERR_BAD_HANDLE;
    }
    unsafe {
        *property = interact_instance.cast::<OfxPropertySet>();
    }
    super::K_OFX_STAT_OK
}

static INTERACT_SUITE_V1: OfxInteractSuiteV1 = OfxInteractSuiteV1 {
    interact_swap_buffers: ofx_interact_swap_buffers,
    interact_redraw: ofx_interact_redraw,
    interact_get_property_set: ofx_interact_get_property_set,
};

pub(crate) unsafe extern "C" fn kagari_fetch_suite(
    _host: *mut OfxPropertySet,
    suite_name: *const std::ffi::c_char,
    suite_version: std::ffi::c_int,
) -> *const std::ffi::c_void {
    if suite_name.is_null() {
        return std::ptr::null();
    }
    let Some(name) = (unsafe { cstr(suite_name) }) else {
        return std::ptr::null();
    };
    if name == "OfxImageEffectSuite" && suite_version == 1 {
        return (&raw const IMAGE_EFFECT_SUITE_V1).cast();
    }
    if name == "OfxParamSuite" && suite_version == 1 {
        return (&raw const PARAM_SUITE_V1).cast();
    }
    if name == "OfxMemorySuite" && suite_version == 1 {
        return (&raw const MEMORY_SUITE_V1).cast();
    }
    if name == "OfxMultiThreadSuite" && suite_version == 1 {
        return (&raw const MULTI_THREAD_SUITE_V1).cast();
    }
    if name == "OfxMessageSuite" && (suite_version == 1 || suite_version == 2) {
        return (&raw const MESSAGE_SUITE_V2).cast();
    }
    if name == "OfxPropertySuite" && suite_version == 1 {
        return (super::properties::property_suite_raw() as *const super::properties::OfxPropertySuiteV1).cast();
    }
    if name == "OfxInteractSuite" && suite_version == 1 {
        return (&raw const INTERACT_SUITE_V1).cast();
    }
    std::ptr::null()
}

type OfxMainEntryFn = unsafe extern "C" fn(
    action: *const std::ffi::c_char,
    handle: *const std::ffi::c_void,
    in_args: *mut OfxPropertySet,
    out_args: *mut OfxPropertySet,
) -> std::ffi::c_int;

/// A third-party plugin whose binary is held open for its lifetime.

mod loader;
pub use loader::*;
