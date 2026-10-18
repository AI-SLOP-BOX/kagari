//! OpenFX host/plugin loading, probing, and execution interface.

use super::properties::{cstr, OfxPropertySet};
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

    /// External OpenFX rendering is not implemented by this placeholder bridge.
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

        let _ = (in_pixels, out_pixels, width, height, frame);
        OfxStatus::Failed
    }
}

/// A third-party plugin whose binary is held open for its lifetime.
pub struct LoadedOfxPlugin {
    pub name: String,
    library: libloading::Library,
    /// Per-instance property sets the plugin may have requested during Load.
    instance_props: Vec<OfxPropertySet>,
}

impl LoadedOfxPlugin {
    /// Attaching third-party plugins is disabled until the required OFX host
    /// and image-effect suites are implemented against the official C ABI.
    pub fn attach(_path: &Path, _name: &str) -> Result<Self, String> {
        Err("OpenFX execution is unavailable: the required host suites are not implemented".into())
    }

    /// Allocate an instance property set owned by this plugin session.
    /// The returned pointer stays valid for the plugin's lifetime (the set is
    /// owned by the LoadedOfxPlugin and freed on drop).
    pub fn new_property_set(&mut self) -> *mut OfxPropertySet {
        self.instance_props.push(OfxPropertySet::new());
        let last = self.instance_props.last_mut().unwrap() as *mut OfxPropertySet;
        last
    }

    /// Rendering stays unavailable until plugin attachment and image handles
    /// can be provided through a complete OFX Image Effect Suite.
    pub fn render(&mut self, _req: &mut OfxRenderRequest<'_>) -> Result<(), String> {
        Err("OpenFX rendering is unavailable until the Image Effect Suite is implemented".into())
    }
}

/// Registry keeping attached plugins alive for the process lifetime.
pub fn register_loaded(plugin: LoadedOfxPlugin) -> usize {
    use std::sync::Mutex;
    static REGISTRY: std::sync::OnceLock<Mutex<Vec<LoadedOfxPlugin>>> = std::sync::OnceLock::new();
    let reg = REGISTRY.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = match reg.lock() {
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
