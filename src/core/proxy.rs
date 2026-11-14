use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_PROXY_FRAMES: u32 = 2_000_000;
const MIN_PROXY_FREE_BYTES: u64 = 512 * 1024 * 1024;
const PROXY_DISK_HEADROOM_NUMERATOR: u64 = 15;
const PROXY_DISK_HEADROOM_DENOMINATOR: u64 = 100;
const MAX_PROXY_SOURCE_RGBA_BYTES: u64 = 512 * 1024 * 1024;

/// Error value returned when proxy generation was cancelled via the shared
/// cancel flag. The UI treats this as a silent, user-requested stop.
pub const PROXY_CANCELLED: &str = "cancelled";

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
    /// Resolution represented by `proxy_path`; separate from the next
    /// generation setting because users may change that while keeping the
    /// current proxy active.
    #[serde(default)]
    pub path_resolution: Option<ProxyResolution>,
    pub resolution: ProxyResolution,
    /// Optional path to a pre-rendered proxy file (image sequence).
    #[serde(default)]
    pub proxy_path: Option<String>,
}

impl Default for LayerProxy {
    fn default() -> Self {
        Self {
            enabled: false,
            path_resolution: None,
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
///
/// `cancel` is polled between video frames so a running job stops promptly
/// (partial output is removed and [`PROXY_CANCELLED`] is returned).
/// `progress` receives `(done, total)` after each produced frame.
pub fn generate_preview_proxy_for_layer(
    layer: &crate::core::timeline::Layer,
    resolution: ProxyResolution,
    destination: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u32, u32),
) -> Result<String, String> {
    let factor = resolution.factor();
    if !factor.is_finite() || factor >= 1.0 {
        return Err("Choose Half, Quarter, or Eighth to generate a preview proxy".into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(PROXY_CANCELLED.to_string());
    }
    if !destination.as_os_str().is_empty() {
        std::fs::create_dir_all(destination)
            .map_err(|error| format!("could not create proxy directory: {error}"))?;
    }

    match &layer.layer_type {
        crate::core::timeline::LayerType::Image { path } => {
            let source = Path::new(path);
            let is_svg = source
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
            let expected_dimensions = if is_svg {
                None
            } else {
                let dimensions = image::image_dimensions(source)
                    .map_err(|error| format!("could not read source image dimensions: {error}"))?;
                ensure_proxy_source_dimensions(dimensions.0, dimensions.1)?;
                let (width, height) = scaled_dimensions(dimensions.0, dimensions.1, factor);
                ensure_proxy_disk_space(destination, width, height, 1)?;
                Some(dimensions)
            };
            let image = open_proxy_source_image(source)?;
            if cancel.load(Ordering::Relaxed) {
                return Err(PROXY_CANCELLED.to_string());
            }
            if expected_dimensions
                .is_some_and(|dimensions| dimensions != (image.width(), image.height()))
            {
                return Err("source image dimensions changed during proxy generation".to_owned());
            }
            ensure_proxy_source_dimensions(image.width(), image.height())?;
            let (width, height) = scaled_dimensions(image.width(), image.height(), factor);
            ensure_proxy_disk_space(destination, width, height, 1)?;
            let proxy = image.resize(width, height, image::imageops::FilterType::Triangle);
            let (output, identity) = reserve_unique_proxy_file(destination, &layer.id)?;
            if let Err(error) = proxy.save_with_format(&output, image::ImageFormat::WebP) {
                let _ = crate::core::atomic_file::remove_file_if_identity(&output, identity);
                return Err(format!("could not encode WebP proxy: {error}"));
            }
            progress(1, 1);
            Ok(output.to_string_lossy().into_owned())
        }
        crate::core::timeline::LayerType::Video {
            frames_dir,
            frame_count,
            ..
        } => generate_video_proxy(
            frames_dir,
            *frame_count,
            factor,
            destination,
            &layer.id,
            cancel,
            progress,
        ),
        _ => Err("Only image and video layers can generate preview proxies".into()),
    }
}

fn open_proxy_source_image(path: &Path) -> Result<image::DynamicImage, String> {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
    {
        let path_string = path.to_string_lossy();
        let decoded = crate::core::image_cache::decode_svg_file(&path_string, 256 * 1024 * 1024)
            .ok_or_else(|| "could not decode SVG source image within the size limits".to_owned())?;
        let rgba = image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.pixels)
            .ok_or_else(|| "decoded SVG image has invalid pixel data".to_owned())?;
        Ok(image::DynamicImage::ImageRgba8(rgba))
    } else {
        let reader = image::ImageReader::open(path)
            .map_err(|error| format!("could not open source image: {error}"))?;
        let (width, height) = reader
            .into_dimensions()
            .map_err(|error| format!("could not read source image dimensions: {error}"))?;
        ensure_proxy_source_dimensions(width, height)?;
        let mut reader = image::ImageReader::open(path)
            .map_err(|error| format!("could not reopen source image: {error}"))?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_image_height = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_alloc = Some(MAX_PROXY_SOURCE_RGBA_BYTES);
        reader.limits(limits);
        let image = reader.decode().map_err(|error| {
            format!("could not decode source image within memory limit: {error}")
        })?;
        if (image.width(), image.height()) != (width, height) {
            return Err("source image dimensions changed during decoding".to_owned());
        }
        Ok(image)
    }
}

fn scaled_dimensions(width: u32, height: u32, factor: f32) -> (u32, u32) {
    (
        ((width as f32 * factor).round() as u32).max(1),
        ((height as f32 * factor).round() as u32).max(1),
    )
}

fn ensure_proxy_source_dimensions(width: u32, height: u32) -> Result<(), String> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .filter(|pixels| {
            *pixels > 0 && *pixels <= crate::core::image_cache::ImageCache::MAX_IMAGE_PIXELS
        })
        .ok_or_else(|| "source image dimensions exceed the proxy limit".to_owned())?;
    if pixels
        .checked_mul(4)
        .is_none_or(|bytes| bytes > MAX_PROXY_SOURCE_RGBA_BYTES)
    {
        return Err("source image exceeds the proxy decode memory limit".to_owned());
    }
    Ok(())
}

fn ensure_proxy_disk_space(
    destination: &Path,
    width: u32,
    height: u32,
    frame_count: u32,
) -> Result<u64, String> {
    let estimated_bytes = crate::core::video_import::estimate_import_disk_bytes(
        width,
        height,
        u64::from(frame_count),
        0.0,
        false,
    )
    // The shared estimate assumes 0.5 bytes per output pixel; use 2 bytes
    // per pixel for WebP proxies to leave room for high-entropy frames.
    .and_then(|estimate| estimate.checked_mul(4))
    .ok_or_else(|| "could not estimate proxy disk usage".to_owned())?;
    let minimum_free_bytes = MIN_PROXY_FREE_BYTES.max(
        estimated_bytes.saturating_mul(PROXY_DISK_HEADROOM_NUMERATOR)
            / PROXY_DISK_HEADROOM_DENOMINATOR,
    );
    let available_bytes =
        crate::core::video_import::available_disk_space(destination).ok_or_else(|| {
            "could not determine available disk space for proxy generation".to_owned()
        })?;
    let required_bytes = estimated_bytes.saturating_add(minimum_free_bytes);
    if available_bytes < required_bytes {
        return Err(format!(
            "proxy generation is estimated to need {:.0} MiB and preserve {:.0} MiB free, but only {:.0} MiB is available",
            estimated_bytes as f64 / (1024.0 * 1024.0),
            minimum_free_bytes as f64 / (1024.0 * 1024.0),
            available_bytes as f64 / (1024.0 * 1024.0),
        ));
    }
    Ok(minimum_free_bytes)
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

fn reserve_unique_proxy_file(
    destination: &Path,
    layer_id: &str,
) -> Result<(PathBuf, crate::core::ffmpeg_export::FileIdentity), String> {
    let stem = safe_proxy_stem(layer_id);
    let mut candidate = destination.join(format!("{stem}.webp"));
    let mut suffix = 1u32;
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                let identity = match crate::core::ffmpeg_export::file_identity(&file) {
                    Ok(identity) => identity,
                    Err(error) => {
                        drop(file);
                        let _ = std::fs::remove_file(&candidate);
                        return Err(format!("could not identify reserved proxy output: {error}"));
                    }
                };
                drop(file);
                return Ok((candidate, identity));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                candidate = destination.join(format!("{stem}_{suffix}.webp"));
                suffix = suffix.saturating_add(1);
            }
            Err(error) => return Err(format!("could not reserve proxy output path: {error}")),
        }
    }
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
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u32, u32),
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err(PROXY_CANCELLED.to_string());
    }
    if frame_count == 0 || frame_count > MAX_PROXY_FRAMES {
        return Err(format!(
            "video proxy frame count must be between 1 and {MAX_PROXY_FRAMES}"
        ));
    }
    let first_source = crate::core::video_import::frame_path_in_dir(frames_dir, 0);
    let (source_width, source_height) = image::image_dimensions(&first_source)
        .map_err(|error| format!("could not read first proxy frame dimensions: {error}"))?;
    ensure_proxy_source_dimensions(source_width, source_height)?;
    let (proxy_width, proxy_height) = scaled_dimensions(source_width, source_height, factor);
    let minimum_free_bytes =
        ensure_proxy_disk_space(destination, proxy_width, proxy_height, frame_count)?;
    let proxy_dir = unique_proxy_directory(destination, layer_id)?;
    let mut last_space_check = std::time::Instant::now();
    for frame in 0..frame_count {
        // Cooperative cancel: checked before every frame so a running job
        // stops within one frame's encode time instead of only at startup.
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(PROXY_CANCELLED.to_string());
        }
        if last_space_check.elapsed() >= std::time::Duration::from_millis(500) {
            last_space_check = std::time::Instant::now();
            if crate::core::video_import::available_disk_space(destination)
                .is_none_or(|available| available < minimum_free_bytes)
            {
                let _ = std::fs::remove_dir_all(&proxy_dir);
                return Err(
                    "proxy generation stopped to preserve the minimum free disk space".to_owned(),
                );
            }
        }
        let source = crate::core::video_import::frame_path_in_dir(frames_dir, frame);
        let dimensions = match image::image_dimensions(&source) {
            Ok(dimensions) => dimensions,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&proxy_dir);
                return Err(format!(
                    "could not read video frame {} dimensions: {error}",
                    frame
                ));
            }
        };
        if dimensions != (source_width, source_height) {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(format!(
                "video frame {} is {}x{}, expected {}x{}",
                frame, dimensions.0, dimensions.1, source_width, source_height
            ));
        }
        if let Err(error) = ensure_proxy_source_dimensions(dimensions.0, dimensions.1) {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(format!("proxy frame {}: {error}", frame));
        }
        let image = match open_proxy_source_image(&source) {
            Ok(image) => image,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&proxy_dir);
                return Err(format!("could not decode video frame {}: {error}", frame));
            }
        };
        if (image.width(), image.height()) != dimensions {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(format!(
                "video frame {} dimensions changed during decoding",
                frame
            ));
        }
        let (width, height) = scaled_dimensions(image.width(), image.height(), factor);
        let proxy = image.resize(width, height, image::imageops::FilterType::Triangle);
        let output = proxy_dir.join(format!("frame_{frame:05}.webp"));
        if let Err(error) = proxy.save_with_format(&output, image::ImageFormat::WebP) {
            let _ = std::fs::remove_dir_all(&proxy_dir);
            return Err(format!(
                "could not encode video proxy frame {}: {error}",
                frame
            ));
        }
        progress(frame + 1, frame_count);
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
    fn legacy_layer_proxy_defaults_path_resolution_to_unknown() {
        let proxy: LayerProxy = serde_json::from_str(
            r#"{"enabled":true,"resolution":"Quarter","proxy_path":"proxy.webp"}"#,
        )
        .expect("legacy proxy settings should remain readable");

        assert_eq!(proxy.path_resolution, None);
        assert_eq!(proxy.resolution, ProxyResolution::Quarter);
    }

    #[test]
    fn test_effective_proxy_scale_final_render_ignores_proxy() {
        let lp = Some(LayerProxy {
            enabled: true,
            path_resolution: None,
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
            path_resolution: None,
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
        let mut comp =
            crate::core::timeline::Composition::new("c".into(), "Comp".into(), 1920, 1080, 30, 30);
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
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_, _| {},
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
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_, _| {},
        )
        .expect("video proxy should be generated");
        assert_eq!(
            image::image_dimensions(Path::new(&result).join("frame_00001.webp"))
                .expect("proxy frame should decode"),
            (40, 20)
        );
        std::fs::remove_dir_all(root).expect("proxy test directory should be removable");
    }

    /// A cancel raised mid-generation must stop within one frame, return the
    /// cancelled marker, and remove the partial output directory.
    #[test]
    fn video_proxy_cancel_stops_mid_generation_and_cleans_partial_output() {
        let root = std::env::temp_dir().join(format!(
            "kagari_video_proxy_cancel_{}_{}",
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
        write_test_image(&frames.join("frame_00002.png"), 80, 40);

        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut progress_calls = 0u32;
        let mut progress = |done: u32, _total: u32| {
            progress_calls += 1;
            if done == 1 {
                // Simulate the user pressing Cancel after the first frame.
                cancel.store(true, Ordering::Relaxed);
            }
        };
        let destination = root.join("proxies");
        let result = generate_video_proxy(
            &frames.to_string_lossy(),
            3,
            ProxyResolution::Half.factor(),
            &destination,
            "video_layer",
            &cancel,
            &mut progress,
        );

        assert_eq!(result.unwrap_err(), PROXY_CANCELLED);
        assert_eq!(progress_calls, 1, "cancel must be observed after frame 1");
        // The partially-written proxy directory must not survive the cancel.
        let leftovers: Vec<_> = std::fs::read_dir(&destination)
            .expect("proxies dir should exist")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        assert!(
            leftovers.is_empty(),
            "cancelled generation must clean up: {leftovers:?}"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn video_proxy_reports_progress_for_every_frame() {
        let root = std::env::temp_dir().join(format!(
            "kagari_video_proxy_progress_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be valid")
                .as_nanos()
        ));
        let frames = root.join("frames");
        std::fs::create_dir_all(&frames).expect("proxy frame directory should be writable");
        write_test_image(&frames.join("frame_00000.png"), 40, 20);
        write_test_image(&frames.join("frame_00001.png"), 40, 20);

        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut reports: Vec<(u32, u32)> = Vec::new();
        let mut progress = |done: u32, total: u32| reports.push((done, total));
        let result = generate_video_proxy(
            &frames.to_string_lossy(),
            2,
            ProxyResolution::Quarter.factor(),
            &root.join("proxies"),
            "video_layer",
            &cancel,
            &mut progress,
        );
        assert!(result.is_ok(), "generation should succeed: {result:?}");
        assert_eq!(reports, vec![(1, 2), (2, 2)]);
        std::fs::remove_dir_all(root).ok();
    }

    /// Pre-set cancel flag: the job must refuse to start at all.
    #[test]
    fn video_proxy_cancel_before_start_returns_cancelled() {
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let result = generate_video_proxy(
            "/nonexistent/frames",
            1,
            0.5,
            &std::env::temp_dir(),
            "never_started",
            &cancel,
            &mut |_, _| {},
        );
        assert_eq!(result.unwrap_err(), PROXY_CANCELLED);
    }
}
