use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_USER_PRESET_JSON_BYTES: u64 = 4 * 1024 * 1024;
const MAX_USER_PRESET_COUNT: usize = 4_096;
const USER_PRESET_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(1);

type CachedUserPresets = Result<Vec<ExportPreset>, String>;
type UserPresetCache = std::sync::Mutex<Option<(std::time::Instant, CachedUserPresets)>>;

fn user_preset_cache() -> &'static UserPresetCache {
    static CACHE: std::sync::OnceLock<UserPresetCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub format: ExportFormat,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: u32,
    pub codec: String,
    pub audio_enabled: bool,
    pub audio_bitrate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Mp4,
    Webm,
    Gif,
    Webp,
    PngSequence,
    ProRes,
    Exr,
}

impl ExportPreset {
    /// Codec index used by the interactive export dialog, if this preset is
    /// representable by its current export pipeline.
    pub fn export_dialog_codec_index(&self) -> Option<usize> {
        match self.format {
            ExportFormat::Mp4 if self.codec.eq_ignore_ascii_case("h264") => Some(0),
            ExportFormat::ProRes
                if self.codec.eq_ignore_ascii_case("h264")
                    || self.codec.eq_ignore_ascii_case("prores_422") =>
            {
                Some(1)
            }
            ExportFormat::ProRes if self.codec.eq_ignore_ascii_case("prores_4444") => Some(2),
            ExportFormat::PngSequence => Some(3),
            ExportFormat::Webp => Some(4),
            _ => None,
        }
    }

    fn has_valid_values(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.len() <= 256
            && (2..=crate::core::software_renderer::MAX_RENDER_DIMENSION).contains(&self.width)
            && (2..=crate::core::software_renderer::MAX_RENDER_DIMENSION).contains(&self.height)
            && (1..=120).contains(&self.fps)
            && self.quality <= 100
            && self.codec.len() <= 64
            && (32..=512).contains(&self.audio_bitrate)
    }
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            format: ExportFormat::Mp4,
            width: 1920,
            height: 1080,
            fps: 30,
            quality: 80,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 192,
        }
    }
}

pub fn builtin_presets() -> Vec<ExportPreset> {
    vec![
        ExportPreset {
            name: "YouTube 1080p".into(),
            format: ExportFormat::Mp4,
            width: 1920,
            height: 1080,
            fps: 30,
            quality: 85,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 192,
        },
        ExportPreset {
            name: "YouTube 4K".into(),
            format: ExportFormat::Mp4,
            width: 3840,
            height: 2160,
            fps: 30,
            quality: 90,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 320,
        },
        ExportPreset {
            name: "TikTok / Reels (9:16)".into(),
            format: ExportFormat::Mp4,
            width: 1080,
            height: 1920,
            fps: 30,
            quality: 80,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 128,
        },
        ExportPreset {
            name: "Instagram Square".into(),
            format: ExportFormat::Mp4,
            width: 1080,
            height: 1080,
            fps: 30,
            quality: 80,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 128,
        },
        ExportPreset {
            name: "ProRes 422 Master".into(),
            format: ExportFormat::ProRes,
            width: 1920,
            height: 1080,
            fps: 30,
            quality: 100,
            codec: "prores_422".into(),
            audio_enabled: true,
            audio_bitrate: 320,
        },
        ExportPreset {
            name: "GIF (Animated)".into(),
            format: ExportFormat::Gif,
            width: 640,
            height: 480,
            fps: 15,
            quality: 70,
            codec: "gif".into(),
            audio_enabled: false,
            audio_bitrate: 0,
        },
        ExportPreset {
            name: "PNG Sequence".into(),
            format: ExportFormat::PngSequence,
            width: 1920,
            height: 1080,
            fps: 30,
            quality: 100,
            codec: "png".into(),
            audio_enabled: false,
            audio_bitrate: 0,
        },
    ]
}

fn presets_path() -> PathBuf {
    let mut p = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push(".kagari");
    p.push("export_presets.json");
    p
}

pub fn load_user_presets() -> Vec<ExportPreset> {
    match try_load_user_presets() {
        Ok(presets) => presets,
        Err(error) => {
            log::warn!("Could not load export presets: {error}");
            Vec::new()
        }
    }
}

pub fn try_load_user_presets() -> Result<Vec<ExportPreset>, String> {
    let now = std::time::Instant::now();
    if let Ok(cache) = user_preset_cache().lock() {
        if let Some((loaded_at, result)) = cache.as_ref() {
            if now.saturating_duration_since(*loaded_at) < USER_PRESET_CACHE_TTL {
                return result.clone();
            }
        }
    }

    let result = read_user_presets();
    if let Ok(mut cache) = user_preset_cache().lock() {
        *cache = Some((now, result.clone()));
    }
    result
}

fn invalidate_user_preset_cache() {
    if let Ok(mut cache) = user_preset_cache().lock() {
        *cache = None;
    }
}

fn read_user_presets() -> Result<Vec<ExportPreset>, String> {
    use std::io::Read;

    let path = presets_path();
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read export presets: {error}")),
    };
    let file_size = file
        .metadata()
        .map_err(|error| format!("Could not inspect export presets: {error}"))?
        .len();
    if file_size > MAX_USER_PRESET_JSON_BYTES {
        return Err(format!(
            "Export presets exceed the {} MiB limit",
            MAX_USER_PRESET_JSON_BYTES / (1024 * 1024)
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(file_size as usize)
        .map_err(|error| format!("Could not reserve export preset memory: {error}"))?;
    file.take(MAX_USER_PRESET_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read export presets: {error}"))?;
    if bytes.len() as u64 > MAX_USER_PRESET_JSON_BYTES {
        return Err(format!(
            "Export presets exceed the {} MiB limit",
            MAX_USER_PRESET_JSON_BYTES / (1024 * 1024)
        ));
    }
    let presets: Vec<ExportPreset> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not parse export presets: {error}"))?;
    if presets.len() > MAX_USER_PRESET_COUNT {
        return Err(format!(
            "Export preset file contains more than {MAX_USER_PRESET_COUNT} presets"
        ));
    }
    if presets.iter().any(|preset| !preset.has_valid_values()) {
        return Err("Export preset file contains invalid settings".into());
    }
    Ok(presets)
}

pub fn save_user_presets(presets: &[ExportPreset]) -> Result<(), String> {
    if presets.len() > MAX_USER_PRESET_COUNT {
        return Err(format!(
            "Cannot save more than {MAX_USER_PRESET_COUNT} export presets"
        ));
    }
    if presets.iter().any(|preset| !preset.has_valid_values()) {
        return Err("Cannot save export presets with invalid settings".into());
    }
    let path = presets_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?;
    if json.len() as u64 > MAX_USER_PRESET_JSON_BYTES {
        return Err(format!(
            "Export presets exceed the {} MiB limit",
            MAX_USER_PRESET_JSON_BYTES / (1024 * 1024)
        ));
    }
    crate::core::atomic_file::write_atomic(&path, json.as_bytes()).map_err(|e| e.to_string())?;
    invalidate_user_preset_cache();
    Ok(())
}

pub fn add_user_preset(preset: ExportPreset) -> Result<(), String> {
    let mut presets = read_user_presets()?;
    presets.push(preset);
    save_user_presets(&presets)
}

pub fn delete_user_preset(name: &str) -> Result<(), String> {
    let mut presets = read_user_presets()?;
    presets.retain(|p| p.name != name);
    save_user_presets(&presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_presets_count() {
        let presets = builtin_presets();
        assert!(presets.len() >= 7);
    }

    #[test]
    fn test_default_preset() {
        let p = ExportPreset::default();
        assert_eq!(p.width, 1920);
        assert_eq!(p.height, 1080);
        assert_eq!(p.format, ExportFormat::Mp4);
    }

    #[test]
    fn test_preset_serialization() {
        let p = ExportPreset::default();
        let json = serde_json::to_string(&p).unwrap();
        let back: ExportPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, p.name);
        assert_eq!(back.width, p.width);
    }

    #[test]
    fn test_format_equality() {
        assert_eq!(ExportFormat::Mp4, ExportFormat::Mp4);
        assert_ne!(ExportFormat::Mp4, ExportFormat::Gif);
    }
}
