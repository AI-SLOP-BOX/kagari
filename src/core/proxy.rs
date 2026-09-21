use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_PROXY_FRAMES: u32 = 2_000_000;

/// Proxy resolution level for layer previews.
/// When enabled, the layer renders at a fraction of full resolution
/// to speed up preview, then switches to full quality on final render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyResolution {
    #[default]
    Full,
    Half,
    Quarter,
    Eighth,
}

impl ProxyResolution {
    pub fn factor(self) -> f32 {
        match self {
            ProxyResolution::Full => 1.0,
            ProxyResolution::Half => 0.5,
            ProxyResolution::Quarter => 0.25,
            ProxyResolution::Eighth => 0.125,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProxyResolution::Full => "Full",
            ProxyResolution::Half => "Half",
            ProxyResolution::Quarter => "Quarter",
            ProxyResolution::Eighth => "Eighth",
        }
    }
}

/// Per-layer proxy state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerProxy {
    pub enabled: bool,
    pub resolution: ProxyResolution,
    /// Optional path to a pre-rendered proxy file (image sequence).
    #[serde(default)]
    pub proxy_path: Option<String>,
}

impl Default for LayerProxy {
    fn default() -> Self {
        Self {
            enabled: false,
            resolution: ProxyResolution::Full,
            proxy_path: None,
        }
    }
}

/// Comp-level proxy settings applied during preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompProxy {
    pub global_resolution: ProxyResolution,
    /// Whether proxy is active during playback (disables on final render).
    pub active_in_preview: bool,
}

impl Default for CompProxy {
    fn default() -> Self {
        Self {
            global_resolution: ProxyResolution::Half,
            active_in_preview: true,
        }
    }
}

/// Compute the effective render scale for a layer considering both
/// comp-level proxy and layer-level proxy. Layer proxy takes priority.
pub fn effective_proxy_scale(
    layer_proxy: Option<&LayerProxy>,
    comp_proxy_active: bool,
    comp_resolution: ProxyResolution,
    is_final_render: bool,
) -> f32 {
    if is_final_render {
        return 1.0;
    }
    if let Some(lp) = layer_proxy {
        if lp.enabled {
            return lp.resolution.factor();
        }
    }
    if comp_proxy_active {
        return comp_resolution.factor();
    }
    1.0
}

/// Return the smallest active preview scale required by a composition.
/// Final/export paths do not call this helper and therefore remain full
/// resolution.
pub fn composition_preview_scale(comp: &crate::core::timeline::Composition) -> f32 {
    let comp_scale = if comp.comp_proxy.active_in_preview {
        comp.comp_proxy.global_resolution.factor()
    } else {
        1.0
    };
    comp.layers
        .iter()
        .filter(|layer| layer.proxy.enabled)
        .map(|layer| layer.proxy.resolution.factor())
        .fold(comp_scale, f32::min)
        .clamp(0.125, 1.0)
}

/// Generate a real, low-resolution WebP proxy for a layer's preview media.
///
/// Proxies are intentionally stored separately from source media. The returned
/// path is either a single WebP image or a directory containing a contiguous
/// frame_%05d.webp sequence, which is understood by both renderers.
pub fn generate_preview_proxy_for_layer(
    layer: &crate::core::timeline::Layer,
    resolution: ProxyResolution,
    destination: &Path,
) -> Result<String, String> {
    let factor = resolution.factor();
    if !factor.is_finite() || factor >= 1.0 {
        return Err("Choose Half, Quarter, or Eighth to generate a preview proxy".into());
    }
    if !destination.as_os_str().is_empty() {
        std::fs::create_dir_all(destination)
            .map_err(|error| format!("could not create proxy directory: {error}"))?;
    }

    match &layer.layer_type {
        crate::core::timeline::LayerType::Image { path } => {
            let source = Path::new(path);
            let image = image::open(source)
                .map_err(|error| format!("could not decode source image: {error}"))?;
            let (width, height) = scaled_dimensions(image.width(), image.height(), factor);
            let proxy = image.resize(width, height, image::imageops::FilterType::Triangle);
            let output = unique_proxy_file(destination, &layer.id);
            proxy
                .save_with_format(&output, image::ImageFormat::WebP)
                .map_err(|error| format!("could not encode WebP proxy: {error}"))?;
            Ok(output.to_string_lossy().into_owned())
        }
        crate::core::timeline::LayerType::Video {
            frames_dir,
            frame_count,
            ..
        } => generate_video_proxy(frames_dir, *frame_count, factor, destination, &layer.id),
        _ => Err("Only image and video layers can generate preview proxies".into()),
    }
}

fn scaled_dimensions(width: u32, height: u32, factor: f32) -> (u32, u32) {
    (
        ((width as f32 * factor).round() as u32).max(1),
        ((height as f32 * factor).round() as u32).max(1),
    )
}

fn safe_proxy_stem(layer_id: &str) -> String {
    let stem: String = layer_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "layer".into()
    } else {
        stem
    }
}

fn unique_proxy_file(destination: &Path, layer_id: &str) -> PathBuf {
    let stem = safe_proxy_stem(layer_id);
    let mut candidate = destination.join(format!("{stem}.webp"));
    let mut suffix = 1u32;
    while candidate.exists() {
        candidate = destination.join(format!("{stem}_{suffix}.webp"));
        suffix = suffix.saturating_add(1);
    }
    candidate
}

fn unique_proxy_directory(destination: &Path, layer_id: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("could not create proxy directory: {error}"))?;
    let stem = safe_proxy_stem(layer_id);
    let mut candidate = destination.join(format!("{stem}_proxy"));
    let mut suffix = 1u32;
    loop {
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                candidate = destination.join(format!("{stem}_proxy_{suffix}"));
                suffix = suffix.saturating_add(1);
            }
            Err(error) => return Err(format!("could not create proxy cache: {error}")),
        }
    }
}

fn generate_video_proxy(
    frames_dir: &str,
    frame_count: u32,
    factor: f32,
    destination: &Path,
    layer_id: &str,
) -> Result<String, String> {
    if frame_count == 0 || frame_count > MAX_PROXY_FRAMES {
        return Err(format!(
            "video proxy frame count must be between 1 and {MAX_PROXY_FRAMES}"
        ));
    }
    let proxy_dir = unique_proxy_directory(destination, layer_id)?;
    for frame in 0..frame_count {
        let source = crate::core::video_import::frame_path_in_dir(frames_dir, frame);
        let image = match image::open(&source) {
            Ok(image) => image,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&proxy_dir);
                return Err(format!("could not decode video frame {}: {error}", frame));
            }
        };
        let (width, height) = scaled_dimensions(image.width(), image.height(), factor);
        let proxy = image.resize(width, height, image::imageops::FilterType::Triangle);
        let output = proxy_dir.join(format!("frame_{frame:05}.webp"));
        if let Err(error) = proxy.save_with_format(&output, image::ImageFormat::WebP) {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(format!("could not encode video proxy frame {}: {error}", frame));
        }
    }
    Ok(proxy_dir.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_image(path: &Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([40, 80, 120, 255]));
        image.save(path).expect("test image should be writable");
    }

    #[test]
    fn test_proxy_resolution_factors() {
        assert_eq!(ProxyResolution::Full.factor(), 1.0);
        assert_eq!(ProxyResolution::Half.factor(), 0.5);
        assert_eq!(ProxyResolution::Quarter.factor(), 0.25);
        assert_eq!(ProxyResolution::Eighth.factor(), 0.125);
    }

    #[test]
    fn test_effective_proxy_scale_final_render_ignores_proxy() {
        let lp = Some(LayerProxy {
            enabled: true,
            resolution: ProxyResolution::Quarter,
            proxy_path: None,
        });
        assert_eq!(
            effective_proxy_scale(lp.as_ref(), true, ProxyResolution::Half, true),
            1.0
        );
    }

    #[test]
    fn test_effective_proxy_scale_layer_priority() {
        let lp = Some(LayerProxy {
            enabled: true,
            resolution: ProxyResolution::Eighth,
            proxy_path: None,
        });
        assert_eq!(
            effective_proxy_scale(lp.as_ref(), true, ProxyResolution::Half, false),
            0.125
        );
    }

    #[test]
    fn test_effective_proxy_scale_comp_fallback() {
        assert_eq!(
            effective_proxy_scale(None, true, ProxyResolution::Quarter, false),
            0.25
        );
    }

    #[test]
    fn test_effective_proxy_scale_no_proxy() {
        assert_eq!(
            effective_proxy_scale(None, false, ProxyResolution::Half, false),
            1.0
        );
    }

    #[test]
    fn test_composition_preview_scale_includes_enabled_layer_proxy() {
        let mut comp = crate::core::timeline::Composition::new(
            "c".into(),
            "Comp".into(),
            1920,
            1080,
            30,
            30,
        );
        comp.comp_proxy.active_in_preview = false;
        let mut layer = crate::core::timeline::Layer::new_null("n".into(), "Null".into(), 30);
        layer.proxy.enabled = true;
        layer.proxy.resolution = ProxyResolution::Quarter;
        comp.layers.push(layer);
        assert_eq!(composition_preview_scale(&comp), 0.25);
    }

    #[test]
    fn image_proxy_generation_writes_scaled_webp() {
        let root = std::env::temp_dir().join(format!(
            "kagari_image_proxy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be valid")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("proxy test directory should be writable");
        let source = root.join("source.png");
        write_test_image(&source, 80, 40);
        let layer = crate::core::timeline::Layer::new(
            "proxy_layer".into(),
            "Proxy Layer".into(),
            crate::core::timeline::LayerType::Image {
                path: source.to_string_lossy().into_owned(),
            },
            30,
        );

        let proxy_path = generate_preview_proxy_for_layer(
            &layer,
            ProxyResolution::Quarter,
            &root.join("proxies"),
        )
        .expect("image proxy should be generated");
        let proxy = image::open(&proxy_path).expect("generated proxy should decode");
        assert_eq!((proxy.width(), proxy.height()), (20, 10));
        assert!(proxy_path.ends_with(".webp"));
        std::fs::remove_dir_all(root).expect("proxy test directory should be removable");
    }

    #[test]
    fn video_proxy_generation_writes_a_webp_sequence() {
        let root = std::env::temp_dir().join(format!(
            "kagari_video_proxy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be valid")
                .as_nanos()
        ));
        let frames = root.join("frames");
        std::fs::create_dir_all(&frames).expect("proxy frame directory should be writable");
        write_test_image(&frames.join("frame_00000.png"), 80, 40);
        write_test_image(&frames.join("frame_00001.png"), 80, 40);
        let result = generate_video_proxy(
            &frames.to_string_lossy(),
            2,
            ProxyResolution::Half.factor(),
            &root.join("proxies"),
            "video_layer",
        )
        .expect("video proxy should be generated");
        assert_eq!(
            image::image_dimensions(Path::new(&result).join("frame_00001.webp"))
                .expect("proxy frame should decode"),
            (40, 20)
        );
        std::fs::remove_dir_all(root).expect("proxy test directory should be removable");
    }
}
