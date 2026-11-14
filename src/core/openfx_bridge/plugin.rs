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

unsafe extern "C" fn kagari_fetch_suite(
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
    std::ptr::null()
}

type OfxMainEntryFn = unsafe extern "C" fn(
    action: *const std::ffi::c_char,
    handle: *const std::ffi::c_void,
    in_args: *mut OfxPropertySet,
    out_args: *mut OfxPropertySet,
) -> std::ffi::c_int;

/// A third-party plugin whose binary is held open for its lifetime.
pub struct LoadedOfxPlugin {
    pub name: String,
    #[allow(dead_code)]
    library: libloading::Library,
    /// Per-instance property sets the plugin may have requested during Load.
    #[allow(clippy::vec_box)]
    instance_props: Vec<Box<OfxPropertySet>>,
    #[allow(dead_code)]
    host: Box<OfxHost>,
    main_entry: Option<OfxMainEntryFn>,
    instance_created: bool,
}

impl LoadedOfxPlugin {
    /// Load an OpenFX `.ofx` dynamic library, locate its `OfxPlugin` entry,
    /// attach the Kagari `OfxHost`, and invoke `OfxActionLoad`.
    pub fn attach(path: &Path, name: &str) -> Result<Self, String> {
        let library = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("failed to load OpenFX library {}: {e}", path.display()))?;
        let get_count: libloading::Symbol<unsafe extern "C" fn() -> std::ffi::c_int> =
            unsafe { library.get(b"OfxGetNumberOfPlugins\0") }
                .map_err(|e| format!("missing OfxGetNumberOfPlugins: {e}"))?;
        let get_plugin: libloading::Symbol<
            unsafe extern "C" fn(nth: std::ffi::c_int) -> *const OfxPlugin,
        > = unsafe { library.get(b"OfxGetPlugin\0") }
            .map_err(|e| format!("missing OfxGetPlugin: {e}"))?;

        let count = unsafe { get_count() };
        if count <= 0 {
            return Err("OpenFX binary reported 0 plugins".into());
        }
        let plugin_ptr = unsafe { get_plugin(0) };
        if plugin_ptr.is_null() {
            return Err("OfxGetPlugin(0) returned null".into());
        }

        let mut host_props = Box::new(OfxPropertySet::new());
        let host_props_ptr = host_props.as_mut() as *mut OfxPropertySet;
        register_property_set_handle(host_props_ptr);
        let mut host = Box::new(OfxHost {
            host: host_props_ptr,
            fetch_suite: Some(kagari_fetch_suite),
        });
        let host_ptr = host.as_mut() as *mut OfxHost;

        unsafe {
            if let Some(set_host) = (*plugin_ptr).set_host {
                set_host(host_ptr);
            }
            if let Some(main_entry) = (*plugin_ptr).main_entry {
                let action = std::ffi::CString::new("OfxActionLoad")
                    .map_err(|e| format!("invalid OfxActionLoad CString: {e}"))?;
                let status = main_entry(
                    action.as_ptr().cast(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if status != 0 {
                    unregister_property_set_handle(host_props_ptr);
                    return Err(format!("OfxActionLoad failed with status {status}"));
                }
            }
        }

        let main_entry = unsafe { (*plugin_ptr).main_entry };
        Ok(Self {
            name: name.to_string(),
            library,
            instance_props: vec![host_props],
            host,
            main_entry,
            instance_created: false,
        })
    }

    /// Allocate an instance property set owned by this plugin session.
    /// The returned pointer stays valid for the plugin's lifetime (the set is
    /// owned by the LoadedOfxPlugin and freed on drop).
    pub fn new_property_set(&mut self) -> *mut OfxPropertySet {
        let mut property_set = Box::new(OfxPropertySet::new());
        let pointer = property_set.as_mut() as *mut OfxPropertySet;
        self.instance_props.push(property_set);
        register_property_set_handle(pointer);
        pointer
    }

    /// Invoke `OfxActionCreateInstance` (once) and `OfxImageEffectActionRender`
    /// on the attached OpenFX plugin.
    pub fn render(&mut self, req: &mut OfxRenderRequest<'_>) -> Result<(), String> {
        if req.input.len() != req.output.len() {
            return Err("input/output buffer length mismatch".into());
        }
        let main_entry = self
            .main_entry
            .ok_or_else(|| "OpenFX plugin has no mainEntry".to_string())?;
        let effect_props = self.new_property_set();
        if !self.instance_created {
            let create_action = std::ffi::CString::new("OfxActionCreateInstance")
                .map_err(|e| format!("invalid OfxActionCreateInstance CString: {e}"))?;
            let status = unsafe {
                main_entry(
                    create_action.as_ptr().cast(),
                    effect_props.cast(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if status != 0 {
                return Err(format!("OfxActionCreateInstance failed with status {status}"));
            }
            self.instance_created = true;
        }
        let render_in_args = self.new_property_set();
        unsafe {
            (*render_in_args).set_double("OfxPropTime", 0, req.frame);
            (*render_in_args).set_int("OfxImageEffectPropRenderWindow", 0, 0);
            (*render_in_args).set_int("OfxImageEffectPropRenderWindow", 1, 0);
            (*render_in_args).set_int("OfxImageEffectPropRenderWindow", 2, req.width as i32);
            (*render_in_args).set_int("OfxImageEffectPropRenderWindow", 3, req.height as i32);
            (*render_in_args).set_double("OfxImageEffectPropRenderScale", 0, 1.0);
            (*render_in_args).set_double("OfxImageEffectPropRenderScale", 1, 1.0);
        }
        let in_image_props = self.new_property_set();
        let out_image_props = self.new_property_set();
        let row_bytes = (req.width as i32) * 4;
        for props in [in_image_props, out_image_props] {
            unsafe {
                (*props).set_int("OfxImagePropBounds", 0, 0);
                (*props).set_int("OfxImagePropBounds", 1, 0);
                (*props).set_int("OfxImagePropBounds", 2, req.width as i32);
                (*props).set_int("OfxImagePropBounds", 3, req.height as i32);
                (*props).set_int("OfxImagePropRowBytes", 0, row_bytes);
            }
        }
        ACTIVE_OFX_IMAGES.with(|c| c.set((in_image_props as usize, out_image_props as usize)));
        let render_action = std::ffi::CString::new("OfxImageEffectActionRender")
            .map_err(|e| format!("invalid OfxImageEffectActionRender CString: {e}"))?;
        let status = unsafe {
            main_entry(
                render_action.as_ptr().cast(),
                effect_props.cast(),
                render_in_args,
                std::ptr::null_mut(),
            )
        };
        ACTIVE_OFX_IMAGES.with(|c| c.set((0, 0)));
        if status != 0 {
            return Err(format!(
                "OfxImageEffectActionRender failed with status {status}"
            ));
        }
        Ok(())
    }
}

impl Drop for LoadedOfxPlugin {
    fn drop(&mut self) {
        for property_set in &mut self.instance_props {
            unregister_property_set_handle(property_set.as_mut());
        }
    }
}

fn plugin_registry() -> &'static std::sync::Mutex<Vec<LoadedOfxPlugin>> {
    static REGISTRY: std::sync::OnceLock<std::sync::Mutex<Vec<LoadedOfxPlugin>>> =
        std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Registry keeping attached plugins alive for the process lifetime.
pub fn register_loaded(plugin: LoadedOfxPlugin) -> usize {
    let mut guard = match plugin_registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.push(plugin);
    guard.len()
}

/// Attach-and-register convenience returning (total_registered, error).
pub fn attach_and_register(path: &Path, name: &str) -> Result<usize, String> {
    let plugin = LoadedOfxPlugin::attach(path, name)?;
    Ok(register_loaded(plugin))
}

/// Parameters for one OFX render invocation.
pub struct OfxRenderRequest<'a> {
    pub width: u32,
    pub height: u32,
    pub frame: f64,
    /// Source RGBA8 pixels (straight alpha)
    pub input: &'a [u8],
    /// Destination buffer (same length)
    pub output: &'a mut [u8],
}

/// `OfxPlugin` layout from ofxCore.h. Function pointer fields are represented
/// as raw pointers because probing reads them but never invokes them.
#[repr(C)]
pub struct OfxPluginRaw {
    pub plugin_api: *const std::os::raw::c_char,
    pub api_version: std::os::raw::c_int,
    pub plugin_identifier: *const std::os::raw::c_char,
    pub plugin_version_major: std::os::raw::c_uint,
    pub plugin_version_minor: std::os::raw::c_uint,
    pub set_host: *const std::os::raw::c_void,
    pub main_entry: *const std::os::raw::c_void,
}

/// Result of probing one .ofx binary.
#[derive(Debug, Clone, PartialEq)]
pub enum OfxProbeResult {
    /// Binary loaded and exposed at least one OfxImageEffectAPI plugin.
    ApiDetected {
        api: String,
        api_version: i32,
        plugin_version: (u32, u32),
        plugin_count: usize,
    },
    /// The binary loaded but is not an OFX plugin (no OfxGetPlugin export).
    NotOfx(String),
    /// The binary could not be loaded at all.
    LoadError(String),
}

/// dlopen the given `.ofx` binary and introspect its exported plugin list
/// via `OfxGetNumberOfPlugins` / `OfxGetPlugin`. Read-only: no host actions
/// are dispatched, so this is safe against arbitrary third-party binaries
/// beyond the dlopen constructor itself.
///
/// Returns `LoadError` when the file is missing/corrupt for this platform,
/// which callers surface without crashing the app.
pub fn probe_ofx_plugin(path: &Path) -> OfxProbeResult {
    let file = match unsafe { libloading::Library::new(path) } {
        Ok(lib) => lib,
        Err(e) => return OfxProbeResult::LoadError(format!("dlopen failed: {e}")),
    };
    let result = unsafe { probe_loaded(&file) };
    drop(file);
    result
}

unsafe fn probe_loaded(lib: &libloading::Library) -> OfxProbeResult {
    let get_num: libloading::Symbol<'_, unsafe extern "C" fn() -> i32> =
        match lib.get(b"OfxGetNumberOfPlugins") {
            Ok(s) => s,
            Err(e) => return OfxProbeResult::NotOfx(format!("missing OfxGetNumberOfPlugins: {e}")),
        };
    let get_plugin: libloading::Symbol<'_, unsafe extern "C" fn(i32) -> *mut OfxPluginRaw> =
        match lib.get(b"OfxGetPlugin") {
            Ok(s) => s,
            Err(e) => return OfxProbeResult::NotOfx(format!("missing OfxGetPlugin: {e}")),
        };

    let count = (get_num)().max(0) as usize;
    if count == 0 {
        return OfxProbeResult::NotOfx("exports zero plugins".into());
    }

    for i in 0..count {
        let raw = (get_plugin)(i as i32);
        if raw.is_null() {
            continue;
        }
        let p = &*raw;
        if let Some(api) = cstr(p.plugin_api) {
            if api == "OfxImageEffectPluginAPI"
                && cstr(p.plugin_identifier).is_some_and(|identifier| !identifier.is_empty())
                && !p.set_host.is_null()
                && !p.main_entry.is_null()
            {
                return OfxProbeResult::ApiDetected {
                    api: api.to_string(),
                    api_version: p.api_version,
                    plugin_version: (p.plugin_version_major, p.plugin_version_minor),
                    plugin_count: count,
                };
            }
        }
    }
    OfxProbeResult::NotOfx("no OfxImageEffect plugin in export list".into())
}
