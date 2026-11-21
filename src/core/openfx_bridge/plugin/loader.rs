use super::*;

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

pub(super) fn plugin_registry() -> &'static std::sync::Mutex<Vec<LoadedOfxPlugin>> {
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
