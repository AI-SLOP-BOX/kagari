use crate::core::mask::MaskMode;
use crate::core::timeline::{Composition, Layer, LayerType, TrackMatteMode};

mod api;
mod audio_reactive;
#[cfg(feature = "wgpu")]
pub(crate) use audio_reactive::inject_audio_reactive_data;
mod buffer;
pub use api::*;
pub(crate) use buffer::{
    mark_render_allocation_failed, try_clone_buffer, try_vec_with_capacity, try_zeroed_buffer,
    RenderAllocationScope,
};
pub(super) use cancel::render_cancelled;
pub(crate) use cancel::RenderCancelFlagGuard;
pub use cancel::{get_render_cancel_flag, set_render_cancel_flag, with_render_cancel_flag};
mod cancel;
mod composite;
mod flatten;
mod frame;
pub use frame::render_frame_to_pixels_filtered;
mod layer_prepare;
mod mask;
mod matte;
mod model3d_raster;
mod output;
mod particle_layer;
mod postfx;
#[cfg(any(feature = "wgpu", test))]
pub(crate) use postfx::layer_3d_shading;
mod precomp;
mod precomp_layer;
mod projective;
mod raster;
mod render_order;
mod resolution;
mod sampling;
mod shadow;
mod shape_raster;
mod text_raster;

#[cfg(feature = "wgpu")]
pub(crate) use mask::{distance_to_polygon, polygon_bounds};
pub(crate) use mask::{offset_polygon_vertices, CpuMaskEntry};

pub use shadow::{build_shadow_map, light_attenuation, spot_cone_factor};

use composite::{apply_adjustment_layer, composite_layer_buffer, AdjustmentLayerCtx, CompositeCtx};
use matte::render_matte_buffer;
use particle_layer::{render_particle_layer, ParticleLayerCtx};
use precomp_layer::{render_precomp_layer, PreCompLayerCtx};

#[cfg(test)]
fn layer_depth_for_sort(_comp: &Composition, layer: &Layer, frame: u32) -> f32 {
    render_order::layer_depth_for_sort(layer, frame)
}

pub use flatten::flatten_collapsed;

pub(crate) use precomp::render_single_layer_pixels;
pub use precomp::{render_precomp_layers, MAX_PRECOMP_DEPTH};

/// Safe buffer size calculation: returns None on overflow or zero dimensions.
pub fn rgba_buffer_size(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    let w = width as usize;
    let h = height as usize;
    let size = w.checked_mul(h)?.checked_mul(4)?;
    (size <= MAX_RENDER_BYTES).then_some(size)
}

// ─── Shape SDF Rasterization (delegated to sdf.rs) ──────────────────────
// NOTE: shape raster calls live in shape_raster.rs / matte.rs now.

/// Professional CPU-based rasterizer to composite active composition layers
/// into a flat RGBA8 pixel buffer for preview rendering or FFmpeg export.
///
/// Each visible layer is first rasterized into its own straight-alpha RGBA
/// buffer, then its stack of effects (`EffectType`) is applied via the CPU
/// effect pipeline (`core::cpu_effects`), and finally the result is composited
/// over the frame using the layer's blend mode. This keeps spatial effects
/// (blur, twirl, bulge, wipe, ...) correct instead of flattening them away.
/// Maximum render dimension per axis (16384 x 16384). Prevents runaway
/// allocations from corrupted project files or hostile CLI arguments — a
/// 100000x100000 request would otherwise attempt a ~40 GB allocation and abort.
pub const MAX_RENDER_DIMENSION: u32 = 16384;
/// Maximum size of one RGBA8 render buffer (512 MiB).
pub const MAX_RENDER_BYTES: usize = 512 * 1024 * 1024;
/// Peak working memory allowed when converting an RGBA8 render to float RGBA.
pub const MAX_FLOAT_RENDER_WORKING_BYTES: usize = 512 * 1024 * 1024;

/// Resolve the transformed layer center from an AE-style anchor point.
/// Position marks the anchor in composition space; raster bounds are centered
/// around the layer, so shift that center by the rotated/scaled anchor offset.
pub(crate) fn layer_center_from_anchor(
    position: [f32; 2],
    scale: [f32; 2],
    rotation_degrees: f32,
    anchor: [f32; 2],
    layer_size: [f32; 2],
) -> [f32; 2] {
    let rad = rotation_degrees.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();
    let offset_x = (layer_size[0] * 0.5 - anchor[0]) * (scale[0] / 100.0);
    let offset_y = (layer_size[1] * 0.5 - anchor[1]) * (scale[1] / 100.0);
    [
        position[0] + cos_r * offset_x - sin_r * offset_y,
        position[1] + sin_r * offset_x + cos_r * offset_y,
    ]
}

/// Estimated peak bytes for a float render, including its RGBA8 source buffer.
pub fn float_render_working_set_size(width: u32, height: u32) -> Option<usize> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let bytes = pixels.checked_mul(std::mem::size_of::<[f32; 4]>() + 4)?;
    (width > 0 && height > 0 && bytes <= MAX_FLOAT_RENDER_WORKING_BYTES).then_some(bytes)
}

/// Estimated peak bytes while converting a float render into four EXR channels.
pub fn exr_rgba_working_set_size(width: u32, height: u32) -> Option<usize> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let bytes = pixels.checked_mul(std::mem::size_of::<[f32; 4]>() * 2)?;
    (width > 0 && height > 0 && bytes <= MAX_FLOAT_RENDER_WORKING_BYTES).then_some(bytes)
}

#[inline]
fn is_sane_render_size(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= MAX_RENDER_DIMENSION
        && height <= MAX_RENDER_DIMENSION
        && rgba_buffer_size(width, height).is_some()
}

/// Memoized particle simulation with bounded rewind checkpoints.
pub(crate) struct ParticleSimCacheEntry {
    pub(crate) version: u64,
    pub(crate) composition_id: String,
    pub(crate) composition_instance_id: usize,
    pub(crate) layer_id: String,
    pub(crate) fps: u32,
    pub(crate) emitter_fingerprint: u64,
    pub(crate) frame: u32,
    pub(crate) system: crate::core::particle_system::ParticleSystem,
    pub(crate) checkpoints: Vec<(u32, crate::core::particle_system::ParticleSystem)>,
}

thread_local! {
    static RENDER_PREVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct RenderPreviewGuard {
    previous: bool,
}

impl RenderPreviewGuard {
    fn install() -> Self {
        let previous = RENDER_PREVIEW.with(|preview| preview.replace(true));
        Self { previous }
    }
}

impl Drop for RenderPreviewGuard {
    fn drop(&mut self) {
        RENDER_PREVIEW.with(|preview| preview.set(self.previous));
    }
}

/// Render the full composition through the software path, then resize the
/// finished frame to the requested preview dimensions. Layer proxy media is
/// scoped to this entry point so exports and CLI renders stay full quality.
/// Rendering in composition coordinates before resizing avoids cropping layers
/// when the preview dimensions are smaller than the composition.
pub fn render_frame_to_pixels_preview(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<u8> {
    let _preview_guard = RenderPreviewGuard::install();
    if comp.width == width && comp.height == height {
        render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode)
    } else if preview_can_render_at_target_resolution(comp) {
        let Some(render_scale) =
            resolution::RenderScale::from_dimensions([comp.width, comp.height], [width, height])
        else {
            return Vec::new();
        };
        frame::render_frame_to_pixels_filtered_scaled(
            comp,
            frame,
            frame::RenderFrameConfig {
                width,
                height,
                exposure_ev,
                lut_mode,
                only_layer_idx: None,
                render_scale,
            },
        )
    } else if width <= MAX_RENDER_DIMENSION
        && height <= MAX_RENDER_DIMENSION
        && rgba_buffer_size(width, height).is_some()
        && comp.width <= MAX_RENDER_DIMENSION
        && comp.height <= MAX_RENDER_DIMENSION
        && rgba_buffer_size(comp.width, comp.height).is_some()
    {
        let full =
            render_frame_to_pixels(comp, frame, comp.width, comp.height, exposure_ev, lut_mode);
        if rgba_buffer_size(comp.width, comp.height) == Some(full.len()) {
            resize_preview_rgba(full, comp.width, comp.height, width, height).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

fn preview_can_render_at_target_resolution(comp: &Composition) -> bool {
    !comp.layers.iter().any(|layer| {
        !layer.paint_strokes.is_empty()
            || !layer.roto_brush_strokes.is_empty()
            || layer.is_3d
            || layer.motion_blur
            || layer.shape_repeater.is_some()
            || layer.track_matte != TrackMatteMode::None
            || layer.style.drop_shadow.enabled
            || layer.style.inner_shadow.enabled
            || layer.style.outer_glow.enabled
            || layer.style.inner_glow.enabled
            || layer.style.satin.enabled
            || layer.style.bevel_emboss.enabled
            || layer.style.stroke.enabled
            || layer.style.gradient_overlay.enabled
            || layer.style.color_overlay.enabled
            || matches!(
                &layer.layer_type,
                LayerType::Shape { stroke_width, .. } if *stroke_width > 0.0
            )
            || (layer.effects_enabled && layer.effects.iter().any(|effect| effect.enabled))
            || matches!(
                layer.layer_type,
                LayerType::Particle { .. }
                    | LayerType::PreComp { .. }
                    | LayerType::Model3D { .. }
                    | LayerType::AdjustmentLayer
            )
            || (layer.is_collapsed && matches!(layer.layer_type, LayerType::PreComp { .. }))
    }) && !comp.lights.iter().any(|light| light.casts_shadows)
}

fn resize_preview_rgba(
    mut pixels: Vec<u8>,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> Option<Vec<u8>> {
    if rgba_buffer_size(source_width, source_height) != Some(pixels.len())
        || rgba_buffer_size(target_width, target_height).is_none()
    {
        return None;
    }

    // Filter in premultiplied-alpha space so RGB in transparent pixels cannot
    // bleed into visible edge pixels during downsampling.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let image = image::RgbaImage::from_raw(source_width, source_height, pixels)?;
    let mut resized = image::imageops::resize(
        &image,
        target_width,
        target_height,
        image::imageops::FilterType::Triangle,
    )
    .into_raw();
    for pixel in resized.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 {
            pixel[..3].fill(0);
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
    Some(resized)
}

pub(crate) fn preview_proxy_path(layer: &Layer, sequence_frame: u32) -> Option<String> {
    let enabled = RENDER_PREVIEW.with(std::cell::Cell::get);
    if !enabled || !layer.proxy.enabled {
        return None;
    }
    let raw_path = layer.proxy.proxy_path.as_deref()?;
    let path = std::path::Path::new(raw_path);
    if path.is_file() {
        return Some(raw_path.to_string());
    }
    if path.is_dir() {
        let frame = if matches!(layer.layer_type, LayerType::Video { .. }) {
            crate::core::video_import::frame_path_in_dir(raw_path, sequence_frame)
        } else {
            crate::core::video_import::frame_path_in_dir(raw_path, 0)
        };
        if frame.is_file() {
            return Some(frame.to_string_lossy().into_owned());
        }
    }
    None
}

pub(crate) fn preview_proxy_active() -> bool {
    RENDER_PREVIEW.with(std::cell::Cell::get)
}

fn source_media_dimensions(layer: &Layer) -> Option<(f32, f32)> {
    let path = match &layer.layer_type {
        LayerType::Image { path } => path.clone(),
        LayerType::Video { frames_dir, .. } => {
            crate::core::video_import::frame_path_in_dir(frames_dir, 0)
                .to_string_lossy()
                .to_string()
        }
        _ => return None,
    };

    let source_dimensions = crate::core::image_cache::load_image_shared(&path)
        .map(|image| (image.width as f32, image.height as f32));
    if source_dimensions.is_some() {
        return source_dimensions;
    }

    let proxy_path = preview_proxy_path(layer, 0)?;
    let proxy_image = crate::core::image_cache::load_image_shared(&proxy_path)?;
    let proxy_dimensions = (proxy_image.width as f32, proxy_image.height as f32);
    let proxy_scale = layer
        .proxy
        .path_resolution
        .unwrap_or(layer.proxy.resolution)
        .factor();
    if !proxy_scale.is_finite() || proxy_scale <= 0.0 {
        return Some(proxy_dimensions);
    }
    Some((
        (proxy_dimensions.0 / proxy_scale).round().max(1.0),
        (proxy_dimensions.1 / proxy_scale).round().max(1.0),
    ))
}

pub fn render_frame_to_pixels(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<u8> {
    render_frame_to_pixels_filtered(comp, frame, width, height, exposure_ev, lut_mode, None)
}

/// Calculate the shortest distance from point (px, py) to the polygon boundary.
/// Apply optional Wiggle Paths deformation to a sampled mask polygon.
fn wiggle_polygon(
    mask: &crate::core::mask::Mask,
    pts: Vec<[f32; 2]>,
    time_sec: f32,
) -> Vec<[f32; 2]> {
    match &mask.wiggle {
        Some(w) if w.size > 0.001 && pts.len() >= 3 => {
            let verts: Vec<crate::core::mask::MaskVertex> = pts
                .iter()
                .map(|p| crate::core::mask::MaskVertex::new(p[0], p[1]))
                .collect();
            crate::core::wiggle_paths::apply_wiggle_paths(&verts, time_sec, w)
                .into_iter()
                .map(|v| v.position)
                .collect()
        }
        _ => pts,
    }
}

#[cfg(test)]
mod tests;
