//! Dynamic Custom WGSL Shader Hot-Reloading & Validation Engine.
//!
//! Allows users to write or load custom WGSL fragment shaders at runtime,
//! validate syntax via naga with precise line/column errors, and compile
//! pipelines for real-time GPU rendering.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Compilation result for a custom WGSL shader.
#[derive(Debug, Clone)]
pub struct ShaderCompileStatus {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub line_number: Option<usize>,
}

/// Global cache for compiled custom WGSL shaders.
pub struct CustomShaderRegistry {
    cache: Mutex<HashMap<String, ShaderCompileStatus>>,
}

static REGISTRY: OnceLock<CustomShaderRegistry> = OnceLock::new();
const MAX_SHADER_CACHE_ENTRIES: usize = 128;
const MAX_SHADER_CACHE_BYTES: usize = 16 * 1024 * 1024;

impl CustomShaderRegistry {
    pub fn global() -> &'static Self {
        REGISTRY.get_or_init(|| Self {
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Validates and compiles a WGSL snippet, returning syntax errors if any.
    pub fn validate_wgsl(&self, source: &str) -> ShaderCompileStatus {
        if source.trim().is_empty() {
            return ShaderCompileStatus {
                is_valid: true,
                error_message: None,
                line_number: None,
            };
        }

        // Cache lookup
        if let Ok(guard) = self.cache.lock() {
            if let Some(cached) = guard.get(source) {
                return cached.clone();
            }
        }

        // Wrap code in a standard fragment shader test harness if user just wrote a fragment body
        let full_source = if source.contains("@fragment") {
            source.to_string()
        } else {
            format!(
                r#"
struct CustomUniforms {{
    time: f32,
    width: f32,
    height: f32,
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    _pad: f32,
}};
@group(0) @binding(0) var<uniform> u: CustomUniforms;
@group(0) @binding(1) var s_tex: texture_2d<f32>;
@group(0) @binding(2) var s_samp: sampler;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {{
    let src = textureSample(s_tex, s_samp, uv);
    {}
}}
"#,
                source
            )
        };

        // Parse with naga WGSL frontend
        let validation_error = match naga::front::wgsl::parse_str(&full_source) {
            Ok(module) => {
                let mut validator = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                );
                validator
                    .validate(&module)
                    .err()
                    .map(|error| error.to_string())
            }
            Err(error) => Some(error.emit_to_string(&full_source)),
        };
        let status = if let Some(message) = validation_error {
            let line = message
                .lines()
                .find(|line| line.contains("-->") || line.contains(':'))
                .and_then(|line| {
                    line.split(':')
                        .nth(1)
                        .and_then(|number| number.trim().parse::<usize>().ok())
                });
            ShaderCompileStatus {
                is_valid: false,
                error_message: Some(message),
                line_number: line,
            }
        } else {
            ShaderCompileStatus {
                is_valid: true,
                error_message: None,
                line_number: None,
            }
        };
        self.cache_status(source, status.clone());
        status
    }

    fn cache_status(&self, source: &str, status: ShaderCompileStatus) {
        let entry_bytes = source.len() + status.error_message.as_ref().map_or(0, String::len);
        if entry_bytes > MAX_SHADER_CACHE_BYTES {
            return;
        }
        if let Ok(mut cache) = self.cache.lock() {
            let current_bytes: usize = cache
                .iter()
                .map(|(key, value)| key.len() + value.error_message.as_ref().map_or(0, String::len))
                .sum();
            if cache.len() >= MAX_SHADER_CACHE_ENTRIES
                || current_bytes.saturating_add(entry_bytes) > MAX_SHADER_CACHE_BYTES
            {
                cache.clear();
            }
            cache.insert(source.to_string(), status);
        }
    }

    /// Clear compiled shader cache (e.g. on hot reload or project reset).
    pub fn clear_cache(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.clear();
        }
    }
}

/// Helper to get default template for custom WGSL shaders.
pub fn default_wgsl_template() -> String {
    crate::core::custom_shader_cpu::DEFAULT_TEMPLATE_SOURCE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_wgsl_validation() {
        let registry = CustomShaderRegistry::global();
        let valid_code = "return vec4<f32>(src.r, src.g * u.p0, src.b, src.a);";
        let status = registry.validate_wgsl(valid_code);
        assert!(
            status.is_valid,
            "Valid WGSL failed: {:?}",
            status.error_message
        );
    }

    #[test]
    fn test_invalid_wgsl_validation() {
        let registry = CustomShaderRegistry::global();
        let invalid_code = "return vec4<f32>(src.r, undefined_variable + 10.0);";
        let status = registry.validate_wgsl(invalid_code);
        assert!(!status.is_valid);
        assert!(status.error_message.is_some());
    }

    #[test]
    fn shader_within_file_picker_limit_is_retained_in_validation_cache() {
        let registry = CustomShaderRegistry {
            cache: Mutex::new(HashMap::new()),
        };
        let source = format!(
            "//{}\nreturn vec4<f32>(src.rgb, src.a);",
            "x".repeat(3 * 1024 * 1024)
        );

        let status = registry.validate_wgsl(&source);

        assert!(status.is_valid, "{:?}", status.error_message);
        assert!(registry.cache.lock().unwrap().contains_key(&source));
    }
}
