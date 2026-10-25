use super::adapter::RenderEffectPlugin;
use super::params::EffectParams;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe central registry for dynamic effect plugins.
pub struct EffectPluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn RenderEffectPlugin>>>,
}

impl Default for EffectPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, plugin: Box<dyn RenderEffectPlugin>) {
        let key = plugin.type_id().to_string();
        if let Ok(mut guard) = self.plugins.write() {
            guard.insert(key, plugin);
        }
    }

    pub fn apply_all(&self, plugin_ids: &[String], frame: u32, params: &mut EffectParams) {
        if let Ok(guard) = self.plugins.read() {
            for id in plugin_ids {
                if let Some(plugin) = guard.get(id) {
                    plugin.apply_to_params(frame, params);
                }
            }
        }
    }
}

/// Extensibility interface for custom 3rd-party image processing plugins and shaders.
pub trait CustomPixelEffectPlugin: Send + Sync {
    /// Unique identifier of the plugin.
    fn id(&self) -> &str;
    /// User-visible name displayed in UI.
    fn name(&self) -> &str;
    /// Category / submenu in the effects palette.
    fn category(&self) -> &str {
        "Custom Plugins"
    }
    /// Process RGBA8 pixel buffer in-place on CPU.
    fn process_pixels(&self, pixels: &mut [u8], width: u32, height: u32, frame: u32, time_sec: f32);
}

/// Global registry for dynamic third-party VFX plugins and custom filters.
pub struct CustomPluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn CustomPixelEffectPlugin>>>,
}

static GLOBAL_CUSTOM_PLUGINS: std::sync::OnceLock<CustomPluginRegistry> =
    std::sync::OnceLock::new();

pub fn global_custom_plugins() -> &'static CustomPluginRegistry {
    GLOBAL_CUSTOM_PLUGINS.get_or_init(CustomPluginRegistry::new)
}

impl Default for CustomPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a custom 3rd-party plugin.
    pub fn register(&self, plugin: Box<dyn CustomPixelEffectPlugin>) {
        let key = plugin.id().to_string();
        if let Ok(mut guard) = self.plugins.write() {
            guard.insert(key, plugin);
        }
    }

    /// Unregister a plugin by ID.
    pub fn unregister(&self, plugin_id: &str) -> bool {
        if let Ok(mut guard) = self.plugins.write() {
            guard.remove(plugin_id).is_some()
        } else {
            false
        }
    }

    /// Execute a custom plugin by ID on an RGBA pixel buffer.
    pub fn process_layer(
        &self,
        plugin_id: &str,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        frame: u32,
        time_sec: f32,
    ) -> bool {
        if let Ok(guard) = self.plugins.read() {
            if let Some(plugin) = guard.get(plugin_id) {
                plugin.process_pixels(pixels, width, height, frame, time_sec);
                return true;
            }
        }
        false
    }

    /// List all registered plugin IDs and display names.
    pub fn list_plugins(&self) -> Vec<(String, String, String)> {
        if let Ok(guard) = self.plugins.read() {
            guard
                .values()
                .map(|p| {
                    (
                        p.id().to_string(),
                        p.name().to_string(),
                        p.category().to_string(),
                    )
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// Register an AE C++ native plugin context into the custom plugin registry.
    pub fn register_ae_plugin(
        &self,
        id: &str,
        context: Arc<crate::core::aegp_sdk_bridge::AePluginHostContext>,
    ) {
        struct AePluginAdapter {
            id: String,
            context: Arc<crate::core::aegp_sdk_bridge::AePluginHostContext>,
        }
        impl CustomPixelEffectPlugin for AePluginAdapter {
            fn id(&self) -> &str {
                &self.id
            }
            fn name(&self) -> &str {
                &self.context.name
            }
            fn category(&self) -> &str {
                "After Effects Plugins"
            }
            fn process_pixels(
                &self,
                pixels: &mut [u8],
                width: u32,
                height: u32,
                frame: u32,
                _time_sec: f32,
            ) {
                let _ = self.context.render_frame(
                    pixels,
                    width as i32,
                    height as i32,
                    frame as i32,
                    30,
                );
            }
        }

        self.register(Box::new(AePluginAdapter {
            id: id.to_string(),
            context,
        }));
    }
}
