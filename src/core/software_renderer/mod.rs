use crate::core::mask::MaskMode;
use crate::core::timeline::{Composition, Layer, LayerType, TrackMatteMode};

mod api;
mod audio_reactive;
pub use api::*;
mod composite;
mod flatten;
mod layer_prepare;
mod mask;
mod matte;
mod model3d_raster;
mod output;
mod particle_layer;
mod postfx;
mod precomp;
mod precomp_layer;
mod raster;
mod render_order;
mod shadow;
mod shape_raster;
mod text_raster;

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
use flatten::perspective_project_layer;

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

#[inline]
fn is_sane_render_size(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= MAX_RENDER_DIMENSION
        && height <= MAX_RENDER_DIMENSION
        && rgba_buffer_size(width, height).is_some()
}

/// Memoized particle simulation: (version, layer id, frame, emitter fingerprint, state).
type ParticleSimCacheEntry = (
    u64,
    String,
    u32,
    [u32; 15],
    crate::core::particle_system::ParticleSystem,
);

// ── Cooperative render cancellation ─────────────────────────────────────────
// A shared flag checked between layers and periodically inside pixel loops.
// Lets export pipelines and future watchdogs abort a long render without
// killing the process. The flag is thread-local so parallel renders don't
// cancel each other.

thread_local! {
    static RENDER_CANCEL: std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> =
        const { std::cell::RefCell::new(None) };
    static RENDER_PREVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Installs a cancellation flag for renders on the current thread.
/// Pass `None` to clear. Returns true if cancellation was requested during render.
pub fn set_render_cancel_flag(flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
    RENDER_CANCEL.with(|f| *f.borrow_mut() = flag);
}

/// Retrieves a clone of the currently installed cancellation flag for the current thread, if any.
pub fn get_render_cancel_flag() -> Option<std::sync::Arc<std::sync::atomic::AtomicBool>> {
    RENDER_CANCEL.with(|f| f.borrow().clone())
}

/// True if the installed flag has been set (or no flag — never cancelled).
#[inline]
fn render_cancelled() -> bool {
    RENDER_CANCEL.with(|f| {
        f.borrow()
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    })
}

/// Render through the software path in preview mode. Layer proxy media is
/// scoped to this entry point so exports and CLI renders stay full quality.
pub fn render_frame_to_pixels_preview(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<u8> {
    RENDER_PREVIEW.with(|preview| {
        let was_preview = preview.replace(true);
        let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
        preview.set(was_preview);
        pixels
    })
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
        LayerType::Image { path } => preview_proxy_path(layer, 0).unwrap_or_else(|| path.clone()),
        LayerType::Video { frames_dir, .. } => preview_proxy_path(layer, 0).unwrap_or_else(|| {
            crate::core::video_import::frame_path_in_dir(frames_dir, 0)
                .to_string_lossy()
                .to_string()
        }),
        _ => return None,
    };

    crate::core::image_cache::with_image_cache(|cache| {
        cache
            .load_image(&path)
            .map(|image| (image.width as f32, image.height as f32))
    })
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

/// Render a frame while optionally limiting the top-level pass to one layer.
///
/// Source-layer effects need a layer's pixels without losing the original
/// composition, because the selected layer's effect parameters still refer to
/// the original layer indices. Keeping the full composition here preserves
/// those references while the filter prevents unrelated layers from being
/// composited into the source buffer.
pub(crate) fn render_frame_to_pixels_filtered(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
    only_layer_idx: Option<usize>,
) -> Vec<u8> {
    // Collapse Transformations: expand collapsed precomps into parent space
    // so their 3D children join the parent camera / z-sort / shadow passes.
    let owned;
    let comp = if only_layer_idx.is_none()
        && comp
            .layers
            .iter()
            .any(|l| l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. }))
    {
        owned = flatten_collapsed(comp, frame);
        &owned
    } else {
        comp
    };
    if !is_sane_render_size(width, height) {
        log::warn!(
            "[Renderer] Rejecting render with dimensions {}x{} (max {})",
            width,
            height,
            MAX_RENDER_DIMENSION
        );
        return Vec::new();
    }
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }
    let render_camera = comp.resolve_camera_at();
    // Composition background colour (Comp Settings > Background Color).
    let (bg_r, bg_g, bg_b, bg_a) = {
        let bg = comp.background_color;
        (
            (bg[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    };
    let mut buffer = vec![0u8; size];
    for p in buffer.as_chunks_mut::<4>().0 {
        p[0] = bg_r;
        p[1] = bg_g;
        p[2] = bg_b;
        p[3] = bg_a;
    }

    audio_reactive::inject_audio_reactive_data(comp, frame);

    let has_solo =
        only_layer_idx.is_none() && comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    // ── Phase 1: Parallel layer data preparation ──
    // Multi-frame rendering support: parallel render queue initialized for MFR pipeline.
    // Note: sequential compositing remains the reference implementation; MFR enabled for batch exports.
    // Pre-compute transform/mask/effect data for all visible layers at once,
    // eliminating redundant property evaluation during the sequential pass.
    let layer_data =
        layer_prepare::prepare_layer_render_data(comp, frame, &render_camera, has_solo);

    // ── Z-depth sort for 3D layers ──
    let has_3d = comp.layers.iter().any(|l| l.is_3d);
    // Shadow map: orthographic-along-ray accumulation of caster quads onto
    // the z=0 receiver plane, per shadow-casting light. Empty when no light
    // casts shadows (zero cost for legacy comps).
    let any_shadow_light = comp
        .lights
        .iter()
        .any(|l| l.casts_shadows && l.intensity_at(frame) > 0.0);
    let shadow_map: Vec<f32> = if any_shadow_light {
        build_shadow_map(comp, frame, width, height)
    } else {
        Vec::new()
    };
    let sorted_layer_indices: Vec<usize> = if has_3d {
        let mut indexed: Vec<(usize, f32)> = comp
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let z = render_order::layer_depth_for_sort(l, frame);
                (i, z)
            })
            .collect();
        // Sort back-to-front: smaller z first (further from camera)
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.into_iter().map(|(i, _)| i).collect()
    } else {
        (0..comp.layers.len()).collect()
    };

    for &sorted_idx in &sorted_layer_indices {
        let layer = &comp.layers[sorted_idx];
        if only_layer_idx.is_some_and(|target_idx| target_idx != sorted_idx) {
            continue;
        }
        // Cooperative cancellation: checked once per layer
        if render_cancelled() {
            break;
        }
        // If this layer serves as a track matte source for another layer, it should not draw directly to the composition
        let is_matte_source = comp.layers.iter().any(|consumer| {
            if consumer.track_matte == TrackMatteMode::None {
                return false;
            }
            if let Some(target_id) = &consumer.track_matte_layer_id {
                target_id == &layer.id
            } else {
                comp.layers
                    .get(sorted_idx + 1)
                    .is_some_and(|next| next.id == consumer.id)
            }
        });
        if is_matte_source {
            continue;
        }
        if !layer.is_active(frame) || (has_solo && !layer.solo) || !layer.visible {
            // layer handled via sorted_idx;
            continue;
        }

        // Linear-light blending flag (hoisted once per frame)
        let blend_linear = comp.blend_linear;

        // Use precomputed data from the parallel phase
        let ld = &layer_data[sorted_idx];
        if ld.skip {
            // layer handled via sorted_idx;
            continue;
        }
        let source_frame = ld.source_frame;
        let pos = ld.pos;
        let scale = ld.scale;
        let rotation = ld.rotation;
        let l_opacity = ld.l_opacity;
        let masks = &ld.masks;

        // Adjustment Layer: apply effects to the composite below, blended by
        // this layer's opacity and clipped to its mask region when present.
        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            apply_adjustment_layer(
                &mut buffer,
                AdjustmentLayerCtx {
                    comp,
                    layer,
                    sorted_idx,
                    source_frame,
                    width,
                    height,
                    opacity: l_opacity,
                    masks,
                },
            );
            continue;
        }

        if render_precomp_layer(
            &mut buffer,
            PreCompLayerCtx {
                comp,
                layer,
                frame,
                source_frame,
                width,
                height,
                position: pos,
                scale,
                rotation,
                opacity: l_opacity,
                masks,
                blend_linear,
                sorted_idx,
            },
        ) {
            continue;
        }

        if render_particle_layer(
            &mut buffer,
            ParticleLayerCtx {
                comp,
                layer,
                frame,
                sorted_idx,
                source_frame,
                position: pos,
                opacity: l_opacity,
                width,
                height,
                camera: &render_camera,
            },
        ) {
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => {
                (comp.width as f32, comp.height as f32)
            }
            LayerType::Text { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Shape { .. } => {
                // Shape units are uniform (composition-width referenced) so
                // circles stay circular on non-square compositions.
                (comp.width as f32, comp.width as f32)
            }
            LayerType::Model3D { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Image { .. } | LayerType::Video { .. } => {
                source_media_dimensions(layer).unwrap_or((comp.width as f32, comp.height as f32))
            }
            _ => continue, // Null or audio layers don't output visual pixels
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Model3D { .. } => [0.72, 0.78, 0.9, 1.0],
            LayerType::Image { .. } | LayerType::Video { .. } => [0.2, 0.6, 0.9, 1.0], // fallback image color
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        // Extract layer transform matrix metrics for pixel boundaries
        let rad = rotation.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        let cx = pos[0];
        let cy = pos[1];

        // For 3D layers, use perspective projection from camera
        let (bounds_x, bounds_y, _perspective_uvs) = if layer.is_3d {
            let cam = &render_camera;
            let layer_rot_3d = layer.transform_3d.rotation.evaluate(frame);
            if let Some(projected) = perspective_project_layer(
                cam.fov_at(frame),
                cam.transform.position.evaluate(frame),
                cam.transform.rotation.evaluate(frame),
                layer.transform_3d.position.evaluate(frame),
                layer_rot_3d,
                scale,
                base_w,
                base_h,
                width as f32,
                height as f32,
            ) {
                // Compute bounding box from projected corners
                let min_sx = projected.iter().map(|c| c[0]).fold(f32::INFINITY, f32::min);
                let max_sx = projected
                    .iter()
                    .map(|c| c[0])
                    .fold(f32::NEG_INFINITY, f32::max);
                let min_sy = projected.iter().map(|c| c[1]).fold(f32::INFINITY, f32::min);
                let max_sy = projected
                    .iter()
                    .map(|c| c[1])
                    .fold(f32::NEG_INFINITY, f32::max);
                let bx = (max_sx - min_sx) * 0.5;
                let by = (max_sy - min_sy) * 0.5;
                (bx, by, Some(projected))
            } else {
                // Behind camera: fall back to flat 2D rendering
                (w * 0.5, h * 0.5, None)
            }
        } else {
            (w * 0.5, h * 0.5, None)
        };
        // abs(): negative scale flips w/h sign — must not invert the bounding box
        let mut ext = bounds_x.max(bounds_y).abs() * 1.5;
        if let Some(repeater) = &layer.shape_repeater {
            // Reserve a conservative raster bounds for every repeated copy. The
            // previous bounds only covered the source shape, so translated copies
            // were silently clipped at the local-buffer edge.
            let instances = crate::core::shape_repeater::evaluate_shape_repeater_at_frame(
                repeater,
                source_frame,
            );
            for instance in instances {
                let m = instance.transform_matrix;
                let tx = m[0][2].abs();
                let ty = m[1][2].abs();
                let half_x = bounds_x.abs() * m[0][0].hypot(m[1][0]);
                let half_y = bounds_y.abs() * m[0][1].hypot(m[1][1]);
                if tx.is_finite() && ty.is_finite() && half_x.is_finite() && half_y.is_finite() {
                    ext = ext.max(half_x.hypot(half_y) + tx.hypot(ty));
                }
            }
        }

        // Render loop over the target bounding box (NaN-safe: `as u32` saturates NaN/inf)
        let min_x = ((cx - ext).max(0.0) as u32).min(width);
        let max_x = ((cx + ext).max(0.0) as u32).min(width);
        let min_y = ((cy - ext).max(0.0) as u32).min(height);
        let max_y = ((cy + ext).max(0.0) as u32).min(height);

        let bw = max_x.saturating_sub(min_x).max(1);
        let bh = max_y.saturating_sub(min_y).max(1);

        // Phase 1: rasterize the layer into a local buffer.
        let mut layer_buf = vec![0u8; (bw * bh * 4) as usize];

        // Shape layers: use SDF rasterization instead of flat fill
        raster::rasterize_layer_content(raster::RasterCtx {
            comp,
            layer,
            frame,
            effective_frame: source_frame,
            source_frame: match &layer.posterize_time {
                Some(pt) if pt.enabled => layer.effective_render_frame(frame, comp.fps) as f32,
                _ => layer.remap_frame_f32(frame),
            },
            masks,
            min_x,
            min_y,
            max_x,
            max_y,
            bw,
            bh,
            width,
            height,
            cx,
            cy,
            cos_r,
            sin_r,
            bounds_x,
            bounds_y,
            base_color,
            l_opacity,
            buffer: &mut buffer[..],
            layer_buf: &mut layer_buf[..],
        });

        postfx::apply_post_fx(postfx::PostFxCtx {
            comp,
            layer,
            effective_frame: source_frame,
            frame,
            sorted_idx,
            layer_buf: &mut layer_buf[..],
            min_x,
            min_y,
            bw,
            bh,
            width,
            height,
            masks,
            cx,
            cy,
            shadow_map: &shadow_map[..],
            dof_blur: ld.dof_blur,
        });

        // Phase 3: composite the (effect-processed) buffer over the frame.
        // First, check if this layer uses a track matte from the layer below.
        let matte_pixels = render_matte_buffer(comp, layer, frame, width, height);

        composite_layer_buffer(
            &mut buffer[..],
            &layer_buf,
            &CompositeCtx {
                layer,
                matte: matte_pixels.as_deref(),
                min_x,
                min_y,
                bw,
                bh,
                width,
                height,
                blend_linear,
                l_opacity,
            },
        );
    }

    output::apply_output_transform(comp, &mut buffer, exposure_ev, lut_mode);

    buffer
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
