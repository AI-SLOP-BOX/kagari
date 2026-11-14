use crate::core::timeline::Effect;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_EFFECT_PRESET_BYTES: usize = 1024 * 1024;
const MAX_PRESET_DISCOVERY_ENTRIES: usize = 4096;
const PRESET_DISCOVERY_TTL: Duration = Duration::from_secs(1);

type PresetDiscoverySnapshot = (PathBuf, Instant, Vec<(String, PathBuf)>);
static PRESET_DISCOVERY_CACHE: OnceLock<Mutex<Option<PresetDiscoverySnapshot>>> = OnceLock::new();

/// A saved effect preset that can be stored as JSON and applied to any layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectPreset {
    /// User-facing name for this preset
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: String,
    /// The effect configuration (name, type, enabled state)
    pub effect: Effect,
    /// ISO-8601 timestamp when this preset was created
    #[serde(default)]
    pub created_at: String,
    /// Optional category tag for organizing presets
    #[serde(default)]
    pub category: String,
}

impl EffectPreset {
    /// Create a preset from an existing effect
    pub fn from_effect(effect: &Effect, name: String) -> Self {
        Self {
            name,
            description: String::new(),
            effect: effect.clone(),
            created_at: chrono_free_timestamp(),
            category: "Custom".to_string(),
        }
    }

    /// Save this preset to a JSON file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        crate::core::atomic_file::write_atomic(path, json.as_bytes())
            .map_err(|e| format!("File write error: {}", e))?;
        invalidate_preset_discovery_cache();
        Ok(())
    }

    /// Load a preset from a JSON file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let json =
            crate::core::project_migration::read_bounded_text_file(path, MAX_EFFECT_PRESET_BYTES)
                .map_err(|e| format!("File read error: {}", e))?;
        serde_json::from_str(&json).map_err(|e| format!("Parse error: {}", e))
    }

    /// Apply this preset to a layer by creating a new Effect instance
    pub fn apply_to_layer(&self, layer: &mut crate::core::timeline::Layer) {
        let mut fx = self.effect.clone();
        // Give it a unique ID based on current effect count
        fx.id = format!("preset_{}", layer.effects.len());
        fx.enabled = true;
        layer.effects.push(fx);
    }
}

/// Simple timestamp without chrono dependency
fn chrono_free_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!(
        "{}-{:02}-{:02}T00:00:00Z",
        1970 + (secs / 31536000) as u32,
        ((secs % 31536000) / 2592000) % 12 + 1,
        ((secs % 2592000) / 86400) + 1,
    )
}

/// Discover all preset files in a directory
pub fn discover_presets_in_dir(dir: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    let mut presets = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .take(MAX_PRESET_DISCOVERY_ENTRIES)
        {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|e| e == "json" || e == "kagari-preset")
            {
                if let Ok(preset) = EffectPreset::load_from_file(&path) {
                    presets.push((preset.name.clone(), path));
                }
            }
        }
    }
    presets.sort_by(|a, b| a.0.cmp(&b.0));
    presets
}

/// Return a one-second cached preset listing for immediate-mode UI callers.
pub fn discover_presets_in_dir_cached(dir: &Path) -> Vec<(String, PathBuf)> {
    let key = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let cache = PRESET_DISCOVERY_CACHE.get_or_init(|| Mutex::new(None));
    {
        let snapshot = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((cached_dir, refreshed_at, presets)) = snapshot.as_ref() {
            if cached_dir == &key && refreshed_at.elapsed() < PRESET_DISCOVERY_TTL {
                return presets.clone();
            }
        }
    }

    let presets = discover_presets_in_dir(dir);
    let mut snapshot = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *snapshot = Some((key, Instant::now(), presets.clone()));
    presets
}

fn invalidate_preset_discovery_cache() {
    if let Some(cache) = PRESET_DISCOVERY_CACHE.get() {
        let mut snapshot = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *snapshot = None;
    }
}

/// Get the default presets directory (~/.kagari/presets/ or platform equivalent)
pub fn default_preset_dir() -> std::path::PathBuf {
    dirs_or_temp().join("kagari").join("presets")
}

fn dirs_or_temp() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}
