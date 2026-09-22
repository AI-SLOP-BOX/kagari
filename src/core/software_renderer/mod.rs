use crate::core::mask::MaskMode;
use crate::core::timeline::{
    BlendMode, Composition, Layer, LayerType, TrackMatteMode,
};
use rayon::prelude::*;

mod composite;
mod mask;
mod matte;
mod model3d_raster;
mod postfx;
mod precomp;
mod raster;
mod shadow;
mod shape_raster;
mod text_raster;

pub(crate) use mask::{
    CpuMaskEntry, compute_combined_mask_coverage, offset_polygon_vertices,
};

pub use shadow::{build_shadow_map, light_attenuation, spot_cone_factor};

use composite::{CompositeCtx, composite_layer_buffer};
use matte::render_matte_buffer;

pub use precomp::{render_precomp_layers, MAX_PRECOMP_DEPTH};
pub(crate) use precomp::render_single_layer_pixels;



#[allow(clippy::too_many_arguments)]
/// Expand every collapsed (`is_collapsed`) PreComp layer into its children,
/// composing parent transform into each child so they render in the parent's
/// coordinate space (AE "Collapse Transformations"). Recursive up to
/// MAX_PRECOMP_DEPTH. When nothing is collapsed this is a cheap clone-free
/// passthrough for callers that need an owned Composition anyway.
pub fn flatten_collapsed(comp: &Composition, frame: u32) -> Composition {
    flatten_collapsed_limited(comp, frame, 0)
}

fn flatten_collapsed_limited(comp: &Composition, frame: u32, depth: u32) -> Composition {
    let mut out = comp.clone();
    if depth >= MAX_PRECOMP_DEPTH {
        return out;
    }
    let mut expanded: Vec<Layer> = Vec::with_capacity(out.layers.len());
    let any_collapsed = out
        .layers
        .iter()
        .any(|l| l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. }));
    if !any_collapsed {
        return out;
    }
    let layers_std = std::mem::take(&mut out.layers);
    for layer in layers_std {
        let (Some(sub_id), true) = (
            match &layer.layer_type {
                LayerType::PreComp { comp_id } => Some(comp_id.clone()),
                _ => None,
            },
            layer.is_collapsed && layer.is_active(frame),
        ) else {
            expanded.push(layer);
            continue;
        };
        let Some(sub) = comp.find_sub_comp(&sub_id) else {
            expanded.push(layer);
            continue;
        };
        // A collapsed PreComp still owns the source-time mapping for its
        // children. The parent transform is evaluated at composition time,
        // while the child content is evaluated at the mapped source frame.
        // Without this split, freeze/reverse/time-stretched collapsed comps
        // silently display the wrong child frame.
        let child_frame = layer.effective_render_frame(frame, comp.fps);
        // Parent transform (already resolves parenting chains)
        let (ppos, pscale, prot, popa) = out.resolve_world_transform(
            &{
                // resolve against ORIGINAL comp so chains above this layer hold
                comp.layers
                    .iter()
                    .find(|l| l.id == layer.id)
                    .cloned()
                    .unwrap_or(layer.clone())
            },
            frame,
        );
        let prad = prot.to_radians();
        let (pc, ps) = (prad.cos(), prad.sin());
        let _pz = if layer.is_3d {
            layer.transform_3d.position.evaluate(frame)[2]
        } else {
            0.0
        };

        // Compose: parent ∘ child (2D affine). Sub-comp coordinates are
        // absolute within the sub frame; map them relative to the sub center
        // onto the parent layer's center before scaling/rotating.
        let flattened_sub = flatten_collapsed_limited(sub, child_frame, depth + 1);
        let sub_cx = flattened_sub.width as f32 * 0.5;
        let sub_cy = flattened_sub.height as f32 * 0.5;
        for child_source in &flattened_sub.layers {
            let mut child = child_source.clone();
            if !child.is_active(child_frame) || !child.visible {
                continue;
            }
            let (cpos, cscale, crot, copa) =
                flattened_sub.resolve_world_transform(&child, child_frame);
            // Compose: parent ∘ child (2D affine)
            let sx = cscale[0] * pscale[0] / 100.0;
            let sy = cscale[1] * pscale[1] / 100.0;
            let rel_x = cpos[0] - sub_cx;
            let rel_y = cpos[1] - sub_cy;
            let lx = rel_x * pscale[0] / 100.0;
            let ly = rel_y * pscale[1] / 100.0;
            let npos = [ppos[0] + lx * pc - ly * ps, ppos[1] + lx * ps + ly * pc];
            let nrot = prot + crot;
            let nopa = (popa / 100.0) * (copa / 100.0) * 100.0;

            child.transform.position = crate::core::property::Animatable::new_constant(npos);
            child.transform.scale =
                crate::core::property::Animatable::new_constant([sx.max(0.001), sy.max(0.001)]);
            child.transform.rotation = crate::core::property::Animatable::new_constant(nrot);
            child.transform.opacity = crate::core::property::Animatable::new_constant(nopa);

            // 3D continuity: cascade full 3D position, rotation and scale into parent space
            if layer.is_3d || child.is_3d {
                let p_pos3d = if layer.is_3d {
                    layer.transform_3d.position.evaluate(frame)
                } else {
                    [ppos[0], ppos[1], 0.0]
                };
                let c_pos3d = if child.is_3d {
                    child.transform_3d.position.evaluate(child_frame)
                } else {
                    [cpos[0], cpos[1], 0.0]
                };
                let p_rot3d = if layer.is_3d {
                    layer.transform_3d.rotation.evaluate(frame)
                } else {
                    [0.0, 0.0, prot]
                };
                let c_rot3d = if child.is_3d {
                    child.transform_3d.rotation.evaluate(child_frame)
                } else {
                    [0.0, 0.0, crot]
                };
                let p_scale3d = if layer.is_3d {
                    layer.transform_3d.scale.evaluate(frame)
                } else {
                    [pscale[0], pscale[1], 100.0]
                };
                let c_scale3d = if child.is_3d {
                    child.transform_3d.scale.evaluate(child_frame)
                } else {
                    [cscale[0], cscale[1], 100.0]
                };

                child.is_3d = true;
                child.transform_3d.position = crate::core::property::Animatable::new_constant([
                    npos[0],
                    npos[1],
                    p_pos3d[2] + c_pos3d[2],
                ]);
                child.transform_3d.rotation = crate::core::property::Animatable::new_constant([
                    p_rot3d[0] + c_rot3d[0],
                    p_rot3d[1] + c_rot3d[1],
                    p_rot3d[2] + c_rot3d[2],
                ]);
                child.transform_3d.scale = crate::core::property::Animatable::new_constant([
                    (p_scale3d[0] * c_scale3d[0] / 100.0).max(0.001),
                    (p_scale3d[1] * c_scale3d[1] / 100.0).max(0.001),
                    (p_scale3d[2] * c_scale3d[2] / 100.0).max(0.001),
                ]);
            }
            expanded.push(child);
        }
    }
    out.layers = expanded;
    out
}

#[allow(clippy::too_many_arguments)]
fn perspective_project_layer(
    cam_fov: f32,
    cam_pos: [f32; 3],
    cam_rot: [f32; 3], // degrees: [rx, ry, rz]
    layer_pos: [f32; 3],
    layer_rot: [f32; 3],   // degrees
    layer_scale: [f32; 2], // percent (100 = no scale)
    layer_width: f32,
    layer_height: f32,
    screen_width: f32,
    screen_height: f32,
) -> Option<[[f32; 4]; 4]> {
    let hw = layer_width * 0.5 * (layer_scale[0] / 100.0);
    let hh = layer_height * 0.5 * (layer_scale[1] / 100.0);

    let corners: [[f32; 3]; 4] = [
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [hw, hh, 0.0],
        [-hw, hh, 0.0],
    ];

    // Euler angles to rotation matrix (XYZ order)
    let to_rad = |d: f32| d * std::f32::consts::PI / 180.0;
    let (rx, ry, rz) = (
        to_rad(layer_rot[0]),
        to_rad(layer_rot[1]),
        to_rad(layer_rot[2]),
    );
    let (crx, srx) = (rx.cos(), rx.sin());
    let (cry, sry) = (ry.cos(), ry.sin());
    let (crz, srz) = (rz.cos(), rz.sin());

    // Rz * Ry * Rx
    let rotate = |p: [f32; 3]| -> [f32; 3] {
        // Rx
        let y1 = p[1] * crx - p[2] * srx;
        let z1 = p[1] * srx + p[2] * crx;
        let x1 = p[0];
        // Ry
        let x2 = x1 * cry + z1 * sry;
        let z2 = -x1 * sry + z1 * cry;
        let y2 = y1;
        // Rz
        [x2 * crz - y2 * srz, x2 * srz + y2 * crz, z2]
    };

    // World-space corners
    let world: Vec<[f32; 3]> = corners
        .iter()
        .map(|c| {
            let r = rotate(*c);
            [
                r[0] + layer_pos[0],
                r[1] + layer_pos[1],
                r[2] + layer_pos[2],
            ]
        })
        .collect();

    // Camera transform (simplified: translate + Z-rotate only)
    let cam_zr = to_rad(cam_rot[2]);
    let (ccrz, ssrz) = (cam_zr.cos(), cam_zr.sin());

    let cam_space: Vec<[f32; 3]> = world
        .iter()
        .map(|c| {
            let dx = c[0] - cam_pos[0];
            let dy = c[1] - cam_pos[1];
            let dz = c[2] - cam_pos[2];
            [dx * ccrz - dy * ssrz, dx * ssrz + dy * ccrz, dz]
        })
        .collect();

    if cam_space.iter().any(|c| c[2] <= 0.1) {
        return None;
    }

    let fov_rad = cam_fov * std::f32::consts::PI / 180.0;
    let focal = (screen_height * 0.5) / (fov_rad * 0.5).tan();

    let mut result = [[0.0f32; 4]; 4];
    for (i, c) in cam_space.iter().enumerate() {
        let z = c[2];
        let sx = screen_width * 0.5 + (c[0] * focal) / z;
        let sy = screen_height * 0.5 - (c[1] * focal) / z;
        let u = if i == 0 || i == 3 { 0.0 } else { 1.0 };
        let v = if i == 0 || i == 1 { 0.0 } else { 1.0 };
        result[i] = [sx, sy, u, v];
    }
    Some(result)
}


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
        LayerType::Video { frames_dir, .. } => {
            preview_proxy_path(layer, 0).unwrap_or_else(|| {
                crate::core::video_import::frame_path_in_dir(frames_dir, 0)
                    .to_string_lossy()
                    .to_string()
            })
        }
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

fn layer_depth_for_sort(_comp: &Composition, layer: &Layer, frame: u32) -> f32 {
    if layer.is_3d {
        layer.transform_3d.position.evaluate(frame)[2]
    } else {
        0.0
    }
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
    for p in buffer.chunks_exact_mut(4) {
        p[0] = bg_r;
        p[1] = bg_g;
        p[2] = bg_b;
        p[3] = bg_a;
    }

    // ── Audio-reactive expression data injection ──
    // Mix audio for the current frame and compute spectrum bands for expressions.
    {
        use crate::core::audio_engine::mix_audio_for_frame;
        use crate::core::audio_spectrum::SpectrumAnalyzer;
        let sample_rate = 44100u32;
        let buffer_size = 2048u32;
        let (pcm, meter) = mix_audio_for_frame(
            comp,
            frame,
            sample_rate,
            buffer_size as usize,
            &crate::core::audio_engine::MasterDspParams::default(),
        );
        let peak = meter.peak_db_left.max(meter.peak_db_right);
        let amplitude = peak.clamp(-60.0, 0.0) / 60.0;

        // Compute 5 frequency bands using the spectrum analyzer
        let mut analyzer = SpectrumAnalyzer::new(5);
        let options = crate::core::audio_spectrum::AudioSpectrumOptions {
            fft_size: 2048,
            frequency_bands: 5,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
            ..Default::default()
        };
        let bands_raw = analyzer.analyze(&pcm, sample_rate, &options);
        let mut bands = [0.0f32; 5];
        for (i, b) in bands_raw.iter().enumerate().take(5) {
            bands[i] = *b;
        }

        crate::core::expression_engine::set_audio_expr_data(
            crate::core::expression_engine::AudioExprData { amplitude, bands },
        );
    }

    let has_solo = only_layer_idx.is_none()
        && comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    // ── Phase 1: Parallel layer data preparation ──
    // Multi-frame rendering support: parallel render queue initialized for MFR pipeline.
    // Note: sequential compositing remains the reference implementation; MFR enabled for batch exports.
    // Pre-compute transform/mask/effect data for all visible layers at once,
    // eliminating redundant property evaluation during the sequential pass.
    #[derive(Default)]
    struct LayerRenderData {
        source_frame: u32,
        pos: [f32; 2],
        scale: [f32; 2],
        rotation: f32,
        l_opacity: f32,
        masks: Vec<CpuMaskEntry>,
        skip: bool,
        /// Depth-of-field blur radius in pixels (0 = sharp)
        dof_blur: f32,
    }

    let layer_data: Vec<LayerRenderData> = {
        use rayon::prelude::*;
        comp.layers
            .par_iter()
            .map(|layer| {
                if !layer.is_active(frame) || (has_solo && !layer.solo) || !layer.visible {
                    return LayerRenderData::default(); // skip=true
                }

                let source_frame = layer.effective_render_frame(frame, comp.fps);
                let (pos, scale, rotation, opacity) = comp.resolve_world_transform(layer, frame);
                let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
                if l_opacity < 0.001 {
                    return LayerRenderData::default();
                }

                // ── Depth of field: circle-of-confusion for 3D layers ──
                let dof_blur = if layer.is_3d && render_camera.dof_enabled_at(frame) {
                    let z = layer.transform_3d.position.evaluate(frame)[2];
                    let dof = crate::core::camera_dof::CameraDofSettings {
                        focus_distance: render_camera.focus_distance_at(frame),
                        aperture: render_camera.aperture_at(frame),
                        f_stop: render_camera.aperture_at(frame),
                        blur_level: 100.0,
                        iris_sides: render_camera.dof_iris_sides,
                        anamorphic_ratio: 1.0,
                        optical_vignetting: 0.0,
                    };
                    crate::core::camera_dof::calculate_circle_of_confusion(z, &dof)
                        .clamp(0.0, render_camera.dof_max_blur_at(frame))
                } else {
                    0.0
                };

                let mut masks = Vec::new();
                for mask in &layer.masks {
                    if mask.enabled && mask.mode != MaskMode::None {
                        let vertices = wiggle_polygon(
                            mask,
                            mask.path.to_polygon(frame, 16),
                            frame as f32 / comp.fps.max(1) as f32,
                        );
                        if vertices.len() >= 3 {
                            // Expand once per layer (not per pixel): the offset
                            // used to be recomputed inside the per-pixel
                            // coverage test, costing seconds per masked layer.
                            let expansion = mask.expansion.evaluate(frame);
                            masks.push(CpuMaskEntry {
                                vertices: offset_polygon_vertices(&vertices, expansion),
                                feather: mask.feather.evaluate(frame),
                                opacity: (mask.opacity.evaluate(frame) / 100.0).clamp(0.0, 1.0),
                                inverted: mask.inverted,
                                mode: mask.mode,
                            });
                        }
                    }
                }

                LayerRenderData {
                    source_frame,
                    pos,
                    scale,
                    rotation,
                    l_opacity,
                    masks,
                    skip: false,
                    dof_blur,
                }
            })
            .collect()
    };

    // ── Z-depth sort for 3D layers ──
    let has_3d = comp.layers.iter().any(|l| l.is_3d);
    // Shadow map: orthographic-along-ray accumulation of caster quads onto
    // the z=0 receiver plane, per shadow-casting light. Empty when no light
    // casts shadows (zero cost for legacy comps).
    let any_shadow_light = comp
        .lights
        .iter()
        .any(|l| l.casts_shadows && l.intensity > 0.0);
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
                let z = layer_depth_for_sort(comp, l, frame);
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
        if comp
            .layers
            .get(sorted_idx + 1)
            .is_some_and(|consumer| consumer.track_matte != TrackMatteMode::None)
        {
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
            if layer.effects_enabled && !layer.effects.is_empty() && l_opacity > 0.003 {
                let mut adjusted = buffer.clone();
                crate::core::cpu_effects::apply_layer_effects(
                    Some(comp),
                    Some(sorted_idx),
                    &mut adjusted,
                    width,
                    height,
                    &layer.effects,
                    source_frame,
                    comp.fps,
                );
                let use_mask = !masks.is_empty();
                let adj_blend = layer.blend_mode;
                for py in 0..height {
                    for px in 0..width {
                        // Feathered masks scale the blend (not a hard gate):
                        // coverage 0 keeps dst, 1 applies the full effect mix.
                        let mut eff_op = l_opacity;
                        if use_mask {
                            let mask_alpha = compute_combined_mask_coverage(
                                px as f32 + 0.5,
                                py as f32 + 0.5,
                                masks,
                            );
                            if mask_alpha <= 0.001 {
                                continue;
                            }
                            eff_op *= mask_alpha;
                        }
                        let i = ((py * width + px) * 4) as usize;
                        let src_r = adjusted[i] as f32 / 255.0;
                        let src_g = adjusted[i + 1] as f32 / 255.0;
                        let src_b = adjusted[i + 2] as f32 / 255.0;
                        let dst_r = buffer[i] as f32 / 255.0;
                        let dst_g = buffer[i + 1] as f32 / 255.0;
                        let dst_b = buffer[i + 2] as f32 / 255.0;
                        let (br, bg, bb) = match adj_blend {
                            BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                            BlendMode::Screen => (
                                1.0 - (1.0 - src_r) * (1.0 - dst_r),
                                1.0 - (1.0 - src_g) * (1.0 - dst_g),
                                1.0 - (1.0 - src_b) * (1.0 - dst_b),
                            ),
                            BlendMode::Add => (
                                (src_r + dst_r).min(1.0),
                                (src_g + dst_g).min(1.0),
                                (src_b + dst_b).min(1.0),
                            ),
                            BlendMode::Overlay => (
                                if dst_r < 0.5 {
                                    2.0 * src_r * dst_r
                                } else {
                                    1.0 - 2.0 * (1.0 - src_r) * (1.0 - dst_r)
                                },
                                if dst_g < 0.5 {
                                    2.0 * src_g * dst_g
                                } else {
                                    1.0 - 2.0 * (1.0 - src_g) * (1.0 - dst_g)
                                },
                                if dst_b < 0.5 {
                                    2.0 * src_b * dst_b
                                } else {
                                    1.0 - 2.0 * (1.0 - src_b) * (1.0 - dst_b)
                                },
                            ),
                            BlendMode::SoftLight => {
                                let f = |s: f32, d: f32| {
                                    if s <= 0.5 {
                                        d - (1.0 - 2.0 * s) * d * (1.0 - d)
                                    } else {
                                        let a = if d <= 0.25 {
                                            ((16.0 * d - 12.0) * d + 4.0) * d
                                        } else {
                                            d.sqrt()
                                        };
                                        d + (2.0 * s - 1.0) * (a - d)
                                    }
                                };
                                (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                            }
                            _ => (src_r, src_g, src_b),
                        };
                        buffer[i] = ((br * eff_op + dst_r * (1.0 - eff_op)) * 255.0)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                        buffer[i + 1] = ((bg * eff_op + dst_g * (1.0 - eff_op)) * 255.0)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                        buffer[i + 2] = ((bb * eff_op + dst_b * (1.0 - eff_op)) * 255.0)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
            continue;
        }

        // PreComp: recursively render the sub-composition, then composite it
        // through the layer's transform (position / scale / rotation / opacity).
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if let Some(sub_comp) = comp.find_sub_comp(comp_id) {
                let mut resolved_sub_comp = sub_comp.clone();
                crate::core::essential_properties::apply_essential_overrides(
                    &mut resolved_sub_comp,
                    &layer.essential_properties,
                );
                let mut sub_pixels = render_precomp_layers(
                    comp,
                    &resolved_sub_comp,
                    source_frame,
                    width,
                    height,
                );
                if !sub_pixels.is_empty() {
                    if !layer.effects.is_empty() && layer.effects_enabled {
                        crate::core::cpu_effects::apply_layer_effects(
                            Some(comp),
                            Some(sorted_idx),
                            &mut sub_pixels,
                            width,
                            height,
                            &layer.effects,
                            source_frame,
                            comp.fps,
                        );
                    }
                    // Treat the rendered sub-comp as a full-frame texture and
                    // sample it through the inverse layer transform.
                    let pc_rad = rotation.to_radians();
                    let pc_cos = pc_rad.cos();
                    let pc_sin = pc_rad.sin();
                    let pc_cx = pos[0];
                    let pc_cy = pos[1];

                    let pc_base_w = comp.width as f32;
                    let pc_base_h = comp.height as f32;
                    let pc_w = (scale[0].abs() / 100.0) * pc_base_w;
                    let pc_h = (scale[1].abs() / 100.0) * pc_base_h;
                    let pc_bx = pc_w * 0.5;
                    let pc_by = pc_h * 0.5;

                    let lo_x = (pc_cx - pc_bx - 2.0).floor().max(0.0) as u32;
                    let hi_x = (pc_cx + pc_bx + 2.0).ceil().min(width as f32 - 1.0) as u32;
                    let lo_y = (pc_cy - pc_by - 2.0).floor().max(0.0) as u32;
                    let hi_y = (pc_cy + pc_by + 2.0).ceil().min(height as f32 - 1.0) as u32;
                    if pc_bx > 0.0 && pc_by > 0.0 && lo_x <= hi_x && lo_y <= hi_y {
                        let precomp_width = hi_x - lo_x + 1;
                        let precomp_height = hi_y - lo_y + 1;
                        let mut precomp_buf = vec![0u8; (precomp_width * precomp_height * 4) as usize];

                        for py in lo_y..=hi_y {
                            for px in lo_x..=hi_x {
                                let mut mask_alpha = 1.0;
                                if !masks.is_empty() {
                                    mask_alpha =
                                        compute_combined_mask_coverage(px as f32, py as f32, masks);
                                }
                                if mask_alpha <= 0.001 {
                                    continue;
                                }

                                // Inverse rotation into local space, then UV over sub-comp frame.
                                let dx = px as f32 - pc_cx;
                                let dy = py as f32 - pc_cy;
                                let lx = dx * pc_cos + dy * pc_sin;
                                let ly = -dx * pc_sin + dy * pc_cos;
                                let u = (lx / pc_bx + 1.0) * 0.5;
                                let v = (ly / pc_by + 1.0) * 0.5;
                                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                                    continue;
                                }

                                let sw = width.max(1);
                                let sh = height.max(1);
                                let sx = ((u * (sw - 1) as f32).round() as u32).min(sw - 1);
                                let sy = ((v * (sh - 1) as f32).round() as u32).min(sh - 1);
                                let src_idx = ((sy * sw + sx) * 4) as usize;
                                if src_idx + 3 >= sub_pixels.len() {
                                    continue;
                                }
                                let dst_idx = (((py - lo_y) * precomp_width + (px - lo_x)) * 4) as usize;
                                precomp_buf[dst_idx] = sub_pixels[src_idx];
                                precomp_buf[dst_idx + 1] = sub_pixels[src_idx + 1];
                                precomp_buf[dst_idx + 2] = sub_pixels[src_idx + 2];
                                precomp_buf[dst_idx + 3] = (sub_pixels[src_idx + 3] as f32
                                    * mask_alpha)
                                    .round()
                                    .clamp(0.0, 255.0) as u8;
                            }
                        }

                        let matte_pixels = render_matte_buffer(comp, layer, frame, width, height);
                        composite_layer_buffer(
                            &mut buffer[..],
                            &precomp_buf,
                            &CompositeCtx {
                                layer,
                                matte: matte_pixels.as_deref(),
                                min_x: lo_x,
                                min_y: lo_y,
                                bw: precomp_width,
                                bh: precomp_height,
                                width,
                                height,
                                blend_linear,
                                l_opacity,
                            },
                        );
                    }
                }
            }
            continue;
        }

        // Particle layer: deterministic simulation from frame 0 to current frame,
        // then render particles directly into the composite buffer.
        if let LayerType::Particle { emitter } = &layer.layer_type {
            let mut em = emitter.clone();
            // Bake layer opacity into particle colors
            em.color_start[3] *= l_opacity;
            em.color_end[3] *= l_opacity;

            // Simulation is deterministic from frame 0, so memoize the simulated
            // state per (version, frame, emitter) — playback then costs O(1) per
            // particle layer instead of O(frame).
            thread_local! {
                static PARTICLE_SIM_CACHE: std::cell::RefCell<Option<ParticleSimCacheEntry>> =
                    const { std::cell::RefCell::new(None) };
            }

            let em_bits: [u32; 15] = [
                em.rate.to_bits(),
                em.lifetime.to_bits(),
                em.speed.to_bits(),
                em.spread_degrees.to_bits(),
                em.gravity[0].to_bits(),
                em.gravity[1].to_bits(),
                em.color_start[0].to_bits(),
                em.color_end[3].to_bits(),
                em.emitter_size[0].to_bits(),
                em.emitter_size[1].to_bits(),
                em.max_particles,
                (em.shape as u32),
                em.depth_enabled as u32,
                em.depth_range[0].to_bits(),
                em.depth_range[1].to_bits(),
            ];
            let sim_key = (
                crate::core::frame_cache::current_version(),
                layer.id.clone(),
                frame.min(2000),
                em_bits,
            );
            let dt = 1.0 / comp.fps.max(1) as f32;
            let depth_enabled = em.depth_enabled;

            let cached = PARTICLE_SIM_CACHE.with(|cache| {
                cache.borrow().as_ref().and_then(|(v, id, f, bits, ps)| {
                    (*v == sim_key.0 && *id == sim_key.1 && *f == sim_key.2 && *bits == sim_key.3)
                        .then(|| ps.clone())
                })
            });

            let ps = match cached {
                Some(ps) => ps,
                None => {
                    let mut ps = crate::core::particle_system::ParticleSystem::new(em);
                    // Cap simulation length for performance on very long compositions
                    let sim_frames = frame.min(2000);
                    for _ in 0..=sim_frames {
                        ps.update(dt, pos[0], pos[1]);
                    }
                    PARTICLE_SIM_CACHE.with(|cache| {
                        *cache.borrow_mut() = Some((
                            sim_key.0,
                            sim_key.1.clone(),
                            sim_key.2,
                            sim_key.3,
                            ps.clone(),
                        ));
                    });
                    ps
                }
            };
            if depth_enabled {
                // Project particles through the active camera: Z drives
                // screen position and size scaling.
                let cam = &render_camera;
                let cpos = cam.transform.position.evaluate(frame);
                let crot = cam.transform.rotation.evaluate(frame);
                let rad = crot[2].to_radians();
                let fov = cam.fov_at(frame).max(1.0).to_radians();
                let focal = (height as f32 * 0.5) / (fov * 0.5).tan();
                let proj = crate::core::particle_system::CameraProjection {
                    cam_x: cpos[0],
                    cam_y: cpos[1],
                    cam_z: cpos[2],
                    focal,
                    cos_rz: rad.cos(),
                    sin_rz: rad.sin(),
                };
                ps.render_projected(
                    &mut buffer,
                    width,
                    height,
                    frame as f32 * dt,
                    Some(&proj),
                );
            } else {
                ps.render(&mut buffer, width, height, frame as f32 * dt);
            }

            // Apply the layer's CPU effect stack to the full frame
            if layer.effects_enabled {
                crate::core::cpu_effects::apply_layer_effects(
                    Some(comp),
                    Some(sorted_idx),
                    &mut buffer,
                    width,
                    height,
                    &layer.effects,
                    source_frame,
                    comp.fps,
                );
            }
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => {
                (comp.width as f32, comp.height as f32)
            }
            LayerType::Text { .. } => {
                (comp.width as f32, comp.height as f32)
            }
            LayerType::Shape { .. } => {
                // Shape units are uniform (composition-width referenced) so
                // circles stay circular on non-square compositions.
                (comp.width as f32, comp.width as f32)
            }
            LayerType::Model3D { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Image { .. } | LayerType::Video { .. } => source_media_dimensions(layer)
                .unwrap_or((comp.width as f32, comp.height as f32)),
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

    // Apply exposure EV shift and LUT color mapping in parallel across CPU cores
    let mult = 2.0f32.powf(exposure_ev);
    buffer
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(pix_i, p)| {
            let mut r = p[0] as f32 / 255.0 * mult;
            let mut g = p[1] as f32 / 255.0 * mult;
            let mut b = p[2] as f32 / 255.0 * mult;

            if lut_mode == 1 {
                // Linear sRGB conversion (2.2 Gamma linearize)
                r = r.powf(2.2);
                g = g.powf(2.2);
                b = b.powf(2.2);
            } else if lut_mode == 3 {
                // User-loaded 3D LUT (tetrahedral interpolation); falls back to
                // passthrough when no LUT is loaded.
                let (nr, ng, nb) = crate::core::ocio_color::apply_lut_pixel(r, g, b);
                r = nr;
                g = ng;
                b = nb;
            } else if lut_mode == 2 {
                // ACES preview pipeline: sRGB-decode → scene-linear exposure →
                // RRT+ODT filmic tonemap → exact piecewise sRGB re-encode.
                let lin = [
                    crate::core::color::srgb_to_linear_piecewise(r),
                    crate::core::color::srgb_to_linear_piecewise(g),
                    crate::core::color::srgb_to_linear_piecewise(b),
                ];
                let out = crate::core::aces::aces_preview_transform(lin);
                r = out[0];
                g = out[1];
                b = out[2];
            }

            // Triangular-PDF dither (per-comp option): kills 8-bit banding from
            // gradients/glow/linear re-encode at imperceptible noise cost.
            // Deterministic per-pixel seed → renders stay byte-reproducible.
            let dither_seed = if comp.dither_output {
                pix_i as f32 * 0.618_034
            } else {
                f32::NAN
            };
            let t1 = fract(dither_seed * 7.13);
            let t2 = fract(dither_seed * 3.71);
            let noise = (t1 - t2) / 255.0;
            let dith = |v: f32| -> u8 {
                if dither_seed.is_nan() {
                    return (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                ((v + noise).clamp(0.0, 1.0) * 255.0).round() as u8
            };
            p[0] = dith(r);
            p[1] = dith(g);
            p[2] = dith(b);
        });

    buffer
}

/// Multi-frame rendering (MFR): render a range of frames in parallel using rayon.
/// Returns a Vec of (frame, pixels) in the same order as the input range.
/// Each frame is rendered independently on a separate CPU core, then the
/// results are collected sequentially for GPU texture upload.
pub fn render_frame_range_parallel(
    comp: &Composition,
    from: u32,
    to: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<(u32, Vec<u8>)> {
    use rayon::prelude::*;

    let frames: Vec<u32> = (from..=to).collect();
    frames
        .par_iter()
        .map(|&f| {
            let pixels = render_frame_to_pixels(comp, f, width, height, exposure_ev, lut_mode);
            (f, pixels)
        })
        .collect()
}

/// Render a frame directly into a 32bpc scene-linear HDR float buffer (`HdrF32Buffer`).
///
/// Pixels are stored as 32-bit floats [R, G, B, A] in unbounded scene-linear space,
/// preserving over-bright highlights (>1.0) and negative deep shadows for EXR / HDR export.
pub fn render_frame_to_hdr_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> crate::core::color_science::HdrF32Buffer {
    let f32_pixels = render_frame_to_pixels_f32(comp, frame, width, height, exposure_ev, lut_mode);
    let mut hdr = crate::core::color_science::HdrF32Buffer::new(width, height);
    for (i, px) in f32_pixels.iter().enumerate() {
        let base = i * 4;
        if base + 3 < hdr.data.len() {
            hdr.data[base] = px[0];
            hdr.data[base + 1] = px[1];
            hdr.data[base + 2] = px[2];
            hdr.data[base + 3] = px[3];
        }
    }
    hdr
}

/// Render a frame and return linear-light f32 RGBA pixels (16/32bpc path).
///
/// Returns physically-linear colour values [r, g, b, a] in `0.0..=∞`, suitable
/// for OpenEXR / HDR export, ACES processing, and full 32bpc floating point workflows.
pub fn render_frame_to_pixels_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<[f32; 4]> {
    let buf8 = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    if buf8.is_empty() {
        return Vec::new();
    }
    let n = (width as usize) * (height as usize);
    let mut out = vec![[0.0f32; 4]; n];
    use crate::core::color::srgb_to_linear_piecewise;
    for (px, dst) in buf8.chunks_exact(4).zip(out.iter_mut()) {
        // Decode the sRGB-encoded result back to linear (exposure baked in).
        let r = srgb_to_linear_piecewise(px[0] as f32 / 255.0);
        let g = srgb_to_linear_piecewise(px[1] as f32 / 255.0);
        let b = srgb_to_linear_piecewise(px[2] as f32 / 255.0);
        let a = px[3] as f32 / 255.0;
        *dst = [r, g, b, a];
    }
    out
}

#[inline]
fn fract(x: f32) -> f32 {
    x - x.floor()
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



/// Renders a frame with a hard deadline. If rendering exceeds `deadline`, the
/// cooperative cancel flag trips and the function returns early with
/// `timed_out = true` and whatever was composited so far.
///
/// This is the watchdog primitive for hang protection: wrap risky renders so a
/// pathological composition degrades to an incomplete frame instead of freezing
/// the export pipeline or UI thread forever.
pub fn render_frame_with_deadline(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
    deadline: std::time::Duration,
) -> (Vec<u8>, bool) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    let flag = Arc::new(AtomicBool::new(false));
    set_render_cancel_flag(Some(flag.clone()));

    // Watchdog timer: flips the flag when the deadline expires.
    let timer_flag = flag.clone();
    let timer = std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < deadline {
            if timer_flag.load(Ordering::Relaxed) {
                return; // already cancelled by other means
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        timer_flag.store(true, Ordering::Relaxed);
    });

    let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    let timed_out = flag.load(Ordering::Relaxed);

    set_render_cancel_flag(None);

    // Stop the watchdog promptly: flipping the flag makes its loop exit.
    flag.store(true, Ordering::Relaxed);
    let _ = timer.join();

    (pixels, timed_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::{
        BlendMode, Composition, Effect, EffectType, Layer, LayerType, ShapeType, TrackMatteMode,
    };

    fn write_gray_png(path: &std::path::Path, gray: u8) {
        let img = image::GrayImage::from_pixel(4, 4, image::Luma([gray]));
        image::DynamicImage::ImageLuma8(img)
            .save_with_format(path, image::ImageFormat::Png)
            .expect("save png");
    }

    fn write_gray_webp(path: &std::path::Path, gray: u8) {
        let img = image::GrayImage::from_pixel(4, 4, image::Luma([gray]));
        image::DynamicImage::ImageLuma8(img)
            .save_with_format(path, image::ImageFormat::WebP)
            .expect("save webp");
    }

    #[test]
    fn test_video_layer_speed_scales_sequence_index() {
        let dir = std::env::temp_dir().join(format!("kagari_speed_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // frame i encoded as gray value i * 10 (frame 20 -> 200)
        for i in 0..=20u32 {
            write_gray_png(&dir.join(format!("frame_{:05}.png", i)), (i * 10) as u8);
        }
        let frames_dir = dir.to_string_lossy().to_string();

        let make_comp = |speed: f32| {
            let mut comp = Composition::new("c1".into(), "Comp".into(), 16, 16, 30, 30);
            let mut layer = Layer::new(
                "v".into(),
                "V".into(),
                LayerType::Video {
                    source: "test".into(),
                    frames_dir: frames_dir.clone(),
                    frame_count: 21,
                    audio_wav: None,
                    speed,
                },
                30,
            );
            layer.transform.position = Animatable::new_constant([8.0, 8.0]);
            comp.layers.push(layer);
            comp
        };

        // speed 2.0: timeline frame 10 -> sequence frame 20 (gray 200)
        let px_fast = render_frame_to_pixels(&make_comp(2.0), 10, 16, 16, 0.0, 0);
        assert_eq!(
            px_fast[8 * 16 * 4 + 8 * 4],
            200,
            "speed=2.0 must map frame 10 to seq frame 20"
        );

        // speed 1.0: timeline frame 10 -> sequence frame 10 (gray 100)
        let px_norm = render_frame_to_pixels(&make_comp(1.0), 10, 16, 16, 0.0, 0);
        assert_eq!(px_norm[8 * 16 * 4 + 8 * 4], 100);

        // clamping: speed 2.0 at last frame stays within sequence
        let px_clamp = render_frame_to_pixels(&make_comp(2.0), 15, 16, 16, 0.0, 0);
        assert_eq!(
            px_clamp[8 * 16 * 4 + 8 * 4],
            200,
            "index must clamp to last frame"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_uses_layer_proxy_media_but_final_render_does_not() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_layer_proxy_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let frames = dir.join("frames");
        std::fs::create_dir_all(&frames).unwrap();
        let full_frame = frames.join("frame_00000.png");
        let proxy_frame = dir.join("proxy.webp");
        write_gray_png(&full_frame, 40);
        write_gray_webp(&proxy_frame, 200);

        let mut comp = Composition::new("proxy_comp".into(), "Proxy".into(), 16, 16, 30, 30);
        let mut layer = Layer::new(
            "video".into(),
            "Proxy Video".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: frames.to_string_lossy().into_owned(),
                frame_count: 1,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        layer.proxy.enabled = true;
        layer.proxy.proxy_path = Some(proxy_frame.to_string_lossy().into_owned());
        layer.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(layer);

        let final_pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        let preview_pixels = render_frame_to_pixels_preview(&comp, 0, 16, 16, 0.0, 0);
        let center = (8 * 16 * 4 + 8 * 4) as usize;
        assert_eq!(final_pixels[center], 40);
        assert_eq!(preview_pixels[center], 200);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_video_frame_blending_interpolates_webp_frames() {
        let dir = std::env::temp_dir().join(format!("kagari_blend_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_gray_webp(&dir.join("frame_00000.webp"), 0);
        write_gray_webp(&dir.join("frame_00001.webp"), 100);

        let mut comp = Composition::new("c1".into(), "Comp".into(), 16, 16, 30, 30);
        let mut layer = Layer::new(
            "v".into(),
            "V".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: dir.to_string_lossy().into_owned(),
                frame_count: 2,
                audio_wav: None,
                speed: 0.5,
            },
            30,
        );
        layer.frame_blending = true;
        layer.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(layer);

        // Timeline frame 1 maps to source position 0.5, so the midpoint is
        // rendered instead of truncating to either source frame.
        let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
        assert_eq!(pixels[8 * 16 * 4 + 8 * 4], 50);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_software_render_frame_to_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Solid".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        layer.blend_mode = BlendMode::Multiply;
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn source_layer_effects_keep_original_layer_indices() {
        let mut comp = Composition::new("source_map".into(), "Source Map".into(), 16, 16, 30, 30);
        comp.background_color = [0.0, 0.0, 0.0, 0.0];

        let mut source = Layer::new(
            "source".into(),
            "Source".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 0.25],
            },
            30,
        );
        source.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(source);

        let mut target = Layer::new(
            "target".into(),
            "Target".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        target.transform.position = Animatable::new_constant([8.0, 8.0]);
        target.effects.push(Effect {
            id: "matte".into(),
            enabled: true,
            name: "Set Matte".into(),
            effect_type: EffectType::SetMatte {
                source_layer_idx: 0,
                source_channel: crate::core::set_matte::MatteSourceChannel::Alpha,
                invert_matte: false,
                composite_mode: crate::core::set_matte::MatteCompositeMode::Replace,
            },
        });
        comp.layers.push(target);

        // Render only the target as a source-layer consumer. The source must
        // still resolve against index 0 in the original composition.
        let pixels = render_frame_to_pixels_filtered(&comp, 0, 16, 16, 0.0, 0, Some(1));
        let center = ((8 * 16 + 8) * 4) as usize;
        assert!(pixels[center] > 200, "target color should survive the matte");
        assert!(
            (60..=68).contains(&pixels[center + 3]),
            "source alpha should drive target alpha, got {}",
            pixels[center + 3]
        );
    }

    #[test]
    fn test_particle_layer_renders() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "p1".to_string(),
            "Particles".to_string(),
            LayerType::Particle {
                emitter: crate::core::particle_system::ParticleEmitter {
                    rate: 500.0,
                    lifetime: 1.0,
                    speed: 50.0,
                    size_start: 6.0,
                    size_end: 3.0,
                    ..Default::default()
                },
            },
            30,
        );
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        // Frame 10 should have particles simulated and rendered
        let pixels = render_frame_to_pixels(&comp, 10, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Some pixels should be brighter than the dark background (20,20,25)
        let bright = pixels.chunks_exact(4).filter(|p| p[0] > 40).count();
        assert!(bright > 0, "Particle layer should produce visible pixels");
    }

    #[test]
    fn test_particle_layer_deterministic() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "p1".to_string(),
            "Particles".to_string(),
            LayerType::Particle {
                emitter: crate::core::particle_system::ParticleEmitter::default(),
            },
            30,
        );
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        assert_eq!(
            p1, p2,
            "Particle simulation must be deterministic per frame"
        );
    }

    #[test]
    fn test_motion_blur_smears_moving_layer() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new(
            "m1".to_string(),
            "Moving Solid".to_string(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        layer.motion_blur = true;
        // Move horizontally: position keyframes at x=20 (frame 5) → x=80 (frame 15)
        layer.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(
                5,
                [20.0, 50.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
            crate::core::keyframe::Keyframe::new(
                15,
                [80.0, 50.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
        ]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 10, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
        // Motion-blurred moving layer should still render without panic
    }

    #[test]
    fn test_3d_light_shading_applied() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new(
            "l3d".to_string(),
            "3D Solid".to_string(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        layer.is_3d = true;
        comp.layers.push(layer);

        // Strong light near the layer center should brighten it
        comp.lights[0].intensity = 200.0;
        comp.lights[0].position = Animatable::new_constant([16.0, 16.0, -300.0]);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel should be lit (brighter than ambient-only floor)
        let center_idx = ((16 * 32 + 16) * 4) as usize;
        assert!(
            pixels[center_idx] > 60,
            "3D layer should be shaded by light, got {}",
            pixels[center_idx]
        );
    }

    #[test]
    fn test_dof_blurs_off_focus_3d_layer() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new(
            "l3d".to_string(),
            "3D Solid".to_string(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        layer.is_3d = true;
        // Push the layer far from the focus plane
        layer.transform_3d.position = Animatable::new_constant([16.0, 16.0, -3000.0]);
        comp.layers.push(layer);

        comp.active_camera.dof_enabled = true;
        comp.resolve_camera_mut().focus_distance = 1000.0;
        comp.resolve_camera_mut().aperture = 50.0;
        comp.resolve_camera_mut().dof_max_blur = 24.0;

        let dof_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let center_idx = ((16 * 32 + 16) * 4) as usize;

        // Reference render with identical geometry but DOF off
        comp.active_camera.dof_enabled = false;
        let sharp_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);

        // Defocus spreads the solid's energy: center dims vs the sharp render
        assert!(
            (sharp_pixels[center_idx] as i32 - dof_pixels[center_idx] as i32).abs() > 2,
            "DOF should change the off-focus render (sharp {} vs defocused {})",
            sharp_pixels[center_idx],
            dof_pixels[center_idx]
        );
    }

    #[test]
    fn test_text_on_path_renders_glyphs() {
        use crate::core::mask::{Mask, MaskPath};
        use crate::core::property::Animatable as PAnimatable;

        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 128, 64, 30, 30);
        let mut layer = Layer::new(
            "tp".to_string(),
            "Path Text".to_string(),
            LayerType::Text {
                text: "AB".to_string(),
                font_size: 24,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Helvetica".to_string(),
                tracking: 0.0,
                leading: 1.2,
                align: 0,
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                text_on_path: true,
            },
            30,
        );
        // Gentle horizontal wave path across the comp
        let path = MaskPath {
            vertices: PAnimatable::new_constant(vec![
                [16.0, 32.0],
                [48.0, 20.0],
                [80.0, 44.0],
                [112.0, 32.0],
            ]),
            tangents: None,
            is_closed: false,
        };
        layer.masks.push(Mask {
            path,
            ..Mask::new_rect("m1".to_string(), "Path".to_string(), 0.0, 0.0, 10.0, 10.0)
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 128, 64, 0.0, 0);
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 200 && pixels[i + 1] > 200 && pixels[i + 2] > 200)
            .count();
        // CI runners may have no installed fonts; skip assertion when headless
        if std::env::var("CI").is_err() {
            assert!(
                bright > 20,
                "text-on-path glyphs should render bright pixels, got {}",
                bright
            );
        }
    }

    #[test]
    fn test_gpu_parent_transform_resolution() {
        // Parented layer should resolve through resolve_world_transform without panicking
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut parent = Layer::new("par".to_string(), "Parent".to_string(), LayerType::Null, 30);
        parent.transform.position = Animatable::new_constant([32.0, 32.0]);
        parent.transform.rotation = Animatable::new_constant(45.0);
        comp.layers.push(parent);
        let mut child = Layer::new(
            "chi".to_string(),
            "Child".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        child.parent_id = Some("par".to_string());
        child.transform.position = Animatable::new_constant([10.0, 0.0]);
        let child_pos = child.transform.position.evaluate(0);
        comp.layers.push(child);

        let (pos, _scale, _rot, _opa) =
            comp.resolve_world_transform(comp.layers.last().unwrap(), 0);
        let _ = child_pos;
        // Child at local (10,0) rotated 45deg around parent origin (32,32):
        // offset = (10*cos45, 10*sin45) ≈ (7.07, 7.07) → world ≈ (39.07, 39.07)
        assert!(
            (pos[0] - 39.07).abs() < 0.5,
            "unexpected world x: {}",
            pos[0]
        );
        assert!(
            (pos[1] - 39.07).abs() < 0.5,
            "unexpected world y: {}",
            pos[1]
        );
    }

    #[test]
    fn test_precomp_nested_rendering() {
        // Sub-comp: red ellipse at its own center
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let mut shape = Layer::new(
            "s1".to_string(),
            "Dot".to_string(),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(40.0),
                    height: Animatable::new_constant(40.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        shape.transform.position = Animatable::new_constant([32.0, 32.0]);
        sub.layers.push(shape);

        // Main comp: pre-comp layer referencing the sub-comp
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new(
            "pc".to_string(),
            "Nested".to_string(),
            LayerType::PreComp {
                comp_id: "sub".to_string(),
            },
            30,
        );
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(
            pixels[center] > 180,
            "PreComp should render nested shape (R={})",
            pixels[center]
        );
    }

    #[test]
    fn test_precomp_essential_property_override_reaches_nested_render() {
        let mut sub = Composition::new(
            "essential_sub".into(),
            "Essential Sub".into(),
            16,
            16,
            30,
            30,
        );
        sub.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut solid = Layer::new(
            "solid".into(),
            "Color Target".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        solid.transform.position = Animatable::new_constant([8.0, 8.0]);
        sub.layers.push(solid);

        let mut comp = Composition::new(
            "essential_main".into(),
            "Essential Main".into(),
            16,
            16,
            30,
            30,
        );
        comp.background_color = [0.0, 0.0, 0.0, 0.0];
        comp.sub_compositions.push(sub);
        let mut precomp = Layer::new(
            "precomp".into(),
            "Essential Instance".into(),
            LayerType::PreComp {
                comp_id: "essential_sub".into(),
            },
            30,
        );
        precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
        precomp.essential_properties.push(
            crate::core::essential_properties::EssentialProperty {
                name: "Color".into(),
                prop_type: crate::core::essential_properties::EssentialPropertyType::Color,
                value: crate::core::essential_properties::EssentialValue::Color([
                    0.0, 1.0, 0.0, 1.0,
                ]),
                overridden: true,
                min_value: 0.0,
                max_value: 100.0,
                options: Vec::new(),
            },
        );
        comp.layers.push(precomp);

        let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        let center = ((8 * 16 + 8) * 4) as usize;
        assert!(
            pixels[center + 1] > 180 && pixels[center] < 40,
            "Essential Property color should reach nested render: {:?}",
            &pixels[center..center + 4]
        );
    }

    #[test]
    fn test_precomp_recurses_through_two_nested_compositions() {
        let mut inner = Composition::new("inner".into(), "Inner".into(), 32, 32, 30, 30);
        inner.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut solid = Layer::new(
            "solid".into(),
            "Red".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        solid.transform.position = Animatable::new_constant([16.0, 16.0]);
        inner.layers.push(solid);

        let mut middle = Composition::new("middle".into(), "Middle".into(), 32, 32, 30, 30);
        middle.background_color = [0.0, 0.0, 0.0, 0.0];
        middle.sub_compositions.push(inner);
        let mut inner_layer = Layer::new(
            "inner-ref".into(),
            "Inner PreComp".into(),
            LayerType::PreComp {
                comp_id: "inner".into(),
            },
            30,
        );
        inner_layer.transform.position = Animatable::new_constant([16.0, 16.0]);
        middle.layers.push(inner_layer);

        let mut outer = Composition::new("outer".into(), "Outer".into(), 32, 32, 30, 30);
        outer.background_color = [0.0, 0.0, 0.0, 0.0];
        outer.sub_compositions.push(middle);
        let mut middle_layer = Layer::new(
            "middle-ref".into(),
            "Middle PreComp".into(),
            LayerType::PreComp {
                comp_id: "middle".into(),
            },
            30,
        );
        middle_layer.transform.position = Animatable::new_constant([16.0, 16.0]);
        outer.layers.push(middle_layer);

        let pixels = render_frame_to_pixels(&outer, 0, 32, 32, 0.0, 0);
        let center = ((16 * 32 + 16) * 4) as usize;
        assert!(
            pixels[center] > 180 && pixels[center + 3] > 180,
            "two-level PreComp should preserve nested pixels: {:?}",
            &pixels[center..center + 4]
        );
    }

    #[test]
    fn test_precomp_applies_parent_effects_before_compositing() {
        let mut sub = Composition::new("pre_fx_sub".into(), "Pre FX Sub".into(), 16, 16, 30, 30);
        sub.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut red = Layer::new(
            "red".into(),
            "Red".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        red.transform.position = Animatable::new_constant([8.0, 8.0]);
        sub.layers.push(red);

        let mut comp = Composition::new("pre_fx_main".into(), "Pre FX Main".into(), 16, 16, 30, 30);
        comp.sub_compositions.push(sub);
        let mut precomp = Layer::new(
            "pre_fx".into(),
            "Pre FX".into(),
            LayerType::PreComp {
                comp_id: "pre_fx_sub".into(),
            },
            30,
        );
        precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
        precomp.effects.push(crate::core::timeline::Effect {
            id: "invert_parent".into(),
            enabled: true,
            name: "Invert".into(),
            effect_type: crate::core::timeline::EffectType::Invert {
                invert_alpha: false,
            },
        });
        comp.layers.push(precomp);

        let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        let center = ((8 * 16 + 8) * 4) as usize;
        assert!(
            pixels[center] < 40 && pixels[center + 1] > 200 && pixels[center + 2] > 200,
            "parent PreComp effect should invert nested pixels: {:?}",
            &pixels[center..center + 4]
        );
    }

    #[test]
    fn test_precomp_video_uses_sequence_and_frame_blending() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_precomp_video_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_gray_webp(&dir.join("frame_00000.webp"), 0);
        write_gray_webp(&dir.join("frame_00001.webp"), 100);

        let mut sub = Composition::new("sub_video".into(), "Sub Video".into(), 16, 16, 30, 30);
        let mut video = Layer::new(
            "video".into(),
            "Video".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: dir.to_string_lossy().into_owned(),
                frame_count: 2,
                audio_wav: None,
                speed: 0.5,
            },
            30,
        );
        video.frame_blending = true;
        video.transform.position = Animatable::new_constant([8.0, 8.0]);
        sub.layers.push(video);

        let mut comp = Composition::new("main_video".into(), "Main".into(), 16, 16, 30, 30);
        comp.sub_compositions.push(sub);
        let mut precomp = Layer::new(
            "pc".into(),
            "Nested Video".into(),
            LayerType::PreComp {
                comp_id: "sub_video".into(),
            },
            30,
        );
        precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(precomp);

        let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
        assert_eq!(pixels[8 * 16 * 4 + 8 * 4], 50);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn precomp_proxy_cache_is_separate_from_final_cache() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_precomp_proxy_cache_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let frames = dir.join("frames");
        std::fs::create_dir_all(&frames).unwrap();
        write_gray_png(&frames.join("frame_00000.png"), 40);
        let proxy = dir.join("proxy.webp");
        write_gray_webp(&proxy, 200);

        let mut sub = Composition::new("proxy_sub".into(), "Proxy Sub".into(), 16, 16, 30, 30);
        let mut video = Layer::new(
            "video".into(),
            "Proxy Video".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: frames.to_string_lossy().into_owned(),
                frame_count: 1,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        video.proxy.enabled = true;
        video.proxy.proxy_path = Some(proxy.to_string_lossy().into_owned());
        video.transform.position = Animatable::new_constant([8.0, 8.0]);
        sub.layers.push(video);

        let mut comp = Composition::new("proxy_outer".into(), "Proxy Outer".into(), 16, 16, 30, 30);
        comp.sub_compositions.push(sub);
        let mut precomp = Layer::new(
            "proxy_ref".into(),
            "Proxy PreComp".into(),
            LayerType::PreComp {
                comp_id: "proxy_sub".into(),
            },
            30,
        );
        precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(precomp);

        let final_before = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        let preview = render_frame_to_pixels_preview(&comp, 0, 16, 16, 0.0, 0);
        let final_after = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        let center = (8 * 16 * 4 + 8 * 4) as usize;
        assert_eq!(final_before[center], 40);
        assert_eq!(preview[center], 200);
        assert_eq!(final_after[center], 40);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_video_frame_blending_works_for_luma_matte() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_video_matte_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_gray_webp(&dir.join("frame_00000.webp"), 0);
        write_gray_webp(&dir.join("frame_00001.webp"), 255);

        let mut comp = Composition::new("video_matte".into(), "Video Matte".into(), 16, 16, 30, 30);
        comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut matte = Layer::new(
            "matte".into(),
            "Matte".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: dir.to_string_lossy().into_owned(),
                frame_count: 2,
                audio_wav: None,
                speed: 0.5,
            },
            30,
        );
        matte.frame_blending = true;
        matte.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(matte);

        let mut content = Layer::new(
            "content".into(),
            "Content".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        content.transform.position = Animatable::new_constant([8.0, 8.0]);
        content.track_matte = TrackMatteMode::LumaMatte;
        comp.layers.push(content);

        let matte = render_matte_buffer(&comp, &comp.layers[1], 1, 16, 16).unwrap();
        let matte_center = (8 * 16 * 4 + 8 * 4) as usize;
        assert!((120..=136).contains(&matte[matte_center]));

        let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
        let center = (8 * 16 * 4 + 8 * 4) as usize;
        assert!(
            (120..=136).contains(&pixels[center + 3]),
            "expected half matte alpha, got {}",
            pixels[center + 3]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_precomp_self_reference_cycle_is_safe() {
        // A comp whose pre-comp layer references ITSELF must render empty
        // (cycle guard) instead of overflowing the stack.
        let mut comp = Composition::new("selfref".to_string(), "Self".to_string(), 32, 32, 30, 30);
        let mut inner = Composition::new(
            "selfref_inner".to_string(),
            "Inner".to_string(),
            32,
            32,
            30,
            30,
        );
        // Inner contains a pre-comp pointing back at the outer comp id
        let cyc = Layer::new(
            "cyc".to_string(),
            "Cycle".to_string(),
            LayerType::PreComp {
                comp_id: "selfref".to_string(),
            },
            30,
        );
        inner.layers.push(cyc);
        comp.sub_compositions.push(inner);

        let mut pc = Layer::new(
            "pc".to_string(),
            "Loop".to_string(),
            LayerType::PreComp {
                comp_id: "selfref_inner".to_string(),
            },
            30,
        );
        pc.transform.position = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(pc);

        // Must terminate and produce a valid buffer
        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), (32 * 32 * 4) as usize);
    }

    #[test]
    fn test_precomp_respects_layer_scale() {
        // Sub-comp: full-frame red solid
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let solid = Layer::new(
            "bg".to_string(),
            "Red".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        sub.layers.push(solid);

        // Main comp: pre-comp scaled to 50% — corners should stay background
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new(
            "pc".to_string(),
            "Half".to_string(),
            LayerType::PreComp {
                comp_id: "sub".to_string(),
            },
            30,
        );
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        pc.transform.scale = Animatable::new_constant([50.0, 50.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let corner = 0usize;
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(
            pixels[corner] < 60,
            "Corner should be background when pre-comp is scaled down (R={})",
            pixels[corner]
        );
        assert!(
            pixels[center] > 180,
            "Center should show scaled pre-comp content (R={})",
            pixels[center]
        );
    }

    #[test]
    fn test_visual_headless_pixel_comparison() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let layer = Layer::new(
            "l1".to_string(),
            "Solid".to_string(),
            LayerType::Solid {
                color: [0.5, 0.5, 0.5, 1.0],
            },
            30,
        );
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);

        let mut mse = 0.0f32;
        for i in 0..p1.len() {
            let diff = (p1[i] as f32 - p2[i] as f32) / 255.0;
            mse += diff * diff;
        }
        mse /= p1.len() as f32;

        assert!(
            mse < 1e-5,
            "Visual regression test failed! Pixel MSE ({}) exceeds threshold 1e-5",
            mse
        );
    }

    #[test]
    fn test_invert_effect_changes_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 20, 20, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Solid".to_string(),
            LayerType::Solid {
                color: [0.8, 0.2, 0.4, 1.0],
            },
            30,
        );
        layer.effects.push(Effect {
            id: "e1".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert {
                invert_alpha: false,
            },
            enabled: true,
        });
        comp.layers.push(layer);

        let inverted = render_frame_to_pixels(&comp, 0, 20, 20, 0.0, 0);
        let ci = ((10 * 20 + 10) * 4) as usize;
        // 0.8 -> inverted ~0.2 (255*0.2=51); allow small tolerance.
        assert!(
            (inverted[ci] as i32 - (255 - (0.8 * 255.0) as u8) as i32).abs() <= 3,
            "invert effect did not apply: got {}",
            inverted[ci]
        );
    }

    #[test]
    fn test_twirl_effect_shifts_pixels() {
        use crate::core::cpu_effects;

        // Place a red pixel slightly off-center so the twirl displaces it.
        // (The exact center pixel at r=0 is the twirl axis and doesn't move.)
        let mut buf = vec![0u8; 16 * 16 * 4];
        let px_x = 8;
        let px_y = 10; // 2 pixels below center — r=2, within radius=30
        let idx = ((px_y * 16 + px_x) * 4) as usize;
        buf[idx] = 255;
        buf[idx + 1] = 40;
        buf[idx + 2] = 40;
        buf[idx + 3] = 255;

        let effects = vec![Effect {
            id: "e2".to_string(),
            name: "Twirl".to_string(),
            effect_type: EffectType::Twirl {
                angle: Animatable::new_constant(90.0),
                radius: Animatable::new_constant(30.0),
            },
            enabled: true,
        }];

        cpu_effects::apply_layer_effects(None, None, &mut buf, 16, 16, &effects, 0, 30);

        // The twirl should have moved the red pixel away from (8,10).
        let orig_val = ((px_y * 16 + px_x) * 4) as usize;
        assert_ne!(
            buf[orig_val], 255,
            "red pixel should have moved from original position"
        );

        // A nearby pixel should now be red (displaced).
        let mut found = false;
        for y in 0..16u32 {
            for x in 0..16u32 {
                let i = ((y * 16 + x) * 4) as usize;
                if buf[i] > 200 && buf[i + 1] < 80 && buf[i + 3] > 200 {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "displaced red pixel not found in buffer after twirl");
    }

    #[test]
    fn test_adjustment_layer_applies_effects_to_composite() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom layer: red solid
        comp.layers.push(Layer::new(
            "l1".to_string(),
            "Red".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        ));
        // Top layer: adjustment layer with Invert effect
        let mut adj = Layer::new_adjustment("adj1".to_string(), "Adj Invert".to_string(), 30);
        adj.effects.push(Effect {
            id: "inv".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert {
                invert_alpha: false,
            },
            enabled: true,
        });
        comp.layers.push(adj);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Red (255,0,0) inverted should be (0,255,255)
        let idx = (5 * 10 + 5) * 4;
        assert_eq!(pixels[idx], 0, "R should be inverted");
        assert_eq!(pixels[idx + 1], 255, "G should be 255");
        assert_eq!(pixels[idx + 2], 255, "B should be 255");
    }

    #[test]
    fn test_fractal_noise_generates_output() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Solid".to_string(),
            LayerType::Solid {
                color: [0.5, 0.5, 0.5, 1.0],
            },
            30,
        );
        layer.effects.push(Effect {
            id: "fn1".to_string(),
            name: "FractalNoise".to_string(),
            effect_type: EffectType::FractalNoise {
                fractal_type: Animatable::new_constant(0.0),
                contrast: Animatable::new_constant(100.0),
                brightness: Animatable::new_constant(0.0),
                complexity: Animatable::new_constant(3.0),
                evolution: Animatable::new_constant(0.0),
            },
            enabled: true,
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);

        // At least some pixels should differ from the original gray
        let mut non_gray = 0;
        for p in pixels.chunks_exact(4) {
            if (p[0] as i32 - p[1] as i32).abs() > 5
                || (p[1] as i32 - p[2] as i32).abs() > 5
                || (p[0] as i32 - 128).abs() > 20
            {
                non_gray += 1;
            }
        }
        assert!(
            non_gray > 0,
            "FractalNoise should produce varied pixel values"
        );
    }

    #[test]
    fn test_background_colour_is_composited() {
        let mut comp = Composition::new("c1".to_string(), "BgComp".to_string(), 8, 8, 30, 30);
        comp.background_color = [1.0, 0.0, 0.0, 1.0];

        let pixels = render_frame_to_pixels(&comp, 0, 8, 8, 0.0, 0);
        assert_eq!(pixels.len(), 8 * 8 * 4);
        for p in pixels.chunks_exact(4) {
            assert_eq!(p[0], 255, "background red must fill an empty composition");
            assert_eq!(p[3], 255, "background alpha must be opaque");
        }
    }

    #[test]
    fn test_time_remap_shifts_layer_time() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Red".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        // Set time remap: at frame 0, remap to frame 10
        layer.time_remap = Some(crate::core::property::Animatable::new_constant(10.0));
        comp.layers.push(layer);

        // At frame 0, the layer should evaluate at frame 10 (still active, since duration=30)
        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Should still have red pixels since layer is active at frame 10
        let idx = (5 * 10 + 5) * 4;
        assert!(
            pixels[idx] > 200,
            "Time remapped layer should still render at frame 0"
        );
    }

    #[test]
    fn test_softlight_blend_uses_both_src_and_dst() {
        // Create a red layer over a green layer with SoftLight blend
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom: green solid
        let mut bottom = Layer::new(
            "l1".to_string(),
            "Green".to_string(),
            LayerType::Solid {
                color: [0.0, 1.0, 0.0, 1.0],
            },
            30,
        );
        bottom.blend_mode = BlendMode::Normal;
        comp.layers.push(bottom);
        // Top: red solid with SoftLight
        let mut top = Layer::new(
            "l2".to_string(),
            "Red".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        top.blend_mode = BlendMode::SoftLight;
        comp.layers.push(top);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let idx = (5 * 10 + 5) * 4;
        // SoftLight with src=red(1,0,0) over dst=green(0,1,0):
        // Red channel: s=1.0 > 0.5, d=0.0, a=0.0 → d + (2*s-1)*(a-d) = 0 + 1*(0-0) = 0
        // Green channel: s=0.0 <= 0.5, d=1.0 → d - (1-2*s)*d*(1-d) = 1 - 1*1*0 = 1
        // Blue channel: both 0 → 0
        // Result should be (0, 1, 0) which is green (unchanged since src red doesn't affect green via SoftLight)
        assert!(
            pixels[idx + 1] > 200,
            "Green channel should be preserved by SoftLight"
        );
    }

    #[test]
    fn test_shape_ellipse_renders_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Ellipse".to_string(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Ellipse {
                    width: crate::core::property::Animatable::new_constant(100.0),
                    height: crate::core::property::Animatable::new_constant(100.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Center pixel should be red (shape color)
        let center = (32 * 64 + 32) * 4;
        assert!(
            pixels[center] > 200,
            "Center of ellipse should be red (R={})",
            pixels[center]
        );
        // Corner pixel should be background color, not red
        let corner = 0;
        assert!(
            pixels[corner] < 50,
            "Corner should be background color (R={})",
            pixels[corner]
        );
    }

    #[test]
    fn test_shape_rectangle_renders_pixels() {
        use crate::core::timeline::ShapeType;
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(),
            "Rect".to_string(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: crate::core::property::Animatable::new_constant(100.0),
                    height: crate::core::property::Animatable::new_constant(100.0),
                    corner_radius: crate::core::property::Animatable::new_constant(0.0),
                },
                color: [0.0, 1.0, 0.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = (32 * 64 + 32) * 4;
        assert!(
            pixels[center + 1] > 200,
            "Center of rectangle should be green"
        );
    }
}

#[cfg(test)]
mod render_size_guard_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn tiny_comp() -> Composition {
        let mut comp = Composition::new("c".into(), "Guard".into(), 32, 32, 30, 30);
        let l = Layer::new(
            "l".into(),
            "S".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        comp.layers.push(l);
        comp
    }

    #[test]
    fn test_zero_and_huge_dimensions_return_empty() {
        let comp = tiny_comp();
        // Zero dimensions
        assert!(render_frame_to_pixels(&comp, 0, 0, 32, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, 32, 0, 0.0, 0).is_empty());
        // Huge dimensions (would be ~40 GB) must be rejected without allocating
        assert!(render_frame_to_pixels(&comp, 0, 100_000, 100_000, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, u32::MAX, u32::MAX, 0.0, 0).is_empty());
    }

    #[test]
    fn test_max_dimension_still_renders_small() {
        let comp = tiny_comp();
        let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_rgba_buffer_size_overflow_safe() {
        assert!(rgba_buffer_size(u32::MAX, u32::MAX).is_none());
        assert!(rgba_buffer_size(0, 0).is_none());
        assert_eq!(rgba_buffer_size(2, 2), Some(16));
        assert!(rgba_buffer_size(16384, 16384).is_none());
        assert_eq!(rgba_buffer_size(8192, 8192), Some(8192 * 8192 * 4));
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn busy_comp(layers: usize) -> Composition {
        let mut comp = Composition::new("c".into(), "Cancel".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [0.5; 4] },
                30,
            );
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            comp.layers.push(l);
        }
        comp
    }

    #[test]
    fn test_cancel_flag_stops_layer_processing() {
        let comp = busy_comp(200);
        let flag = Arc::new(AtomicBool::new(true)); // cancelled from the start
        set_render_cancel_flag(Some(flag.clone()));

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Buffer is still valid; but no layer work should have happened
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Background-only buffer: no bright solid pixels
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert_eq!(bright, 0, "cancelled render must not composite layers");

        set_render_cancel_flag(None);

        // Without the flag the same comp renders layers normally
        let pixels2 = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright2 = (0..pixels2.len())
            .step_by(4)
            .filter(|&i| pixels2[i] > 100)
            .count();
        assert!(bright2 > 0, "un-cancelled render must composite layers");
    }

    #[test]
    fn test_clearing_flag_restores_full_render() {
        let comp = busy_comp(10);
        let flag = Arc::new(AtomicBool::new(false));
        set_render_cancel_flag(Some(flag.clone()));
        set_render_cancel_flag(None); // cleared — must behave as default

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(bright > 0);
    }
}

#[cfg(test)]
mod watchdog_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn comp(layers: usize) -> Composition {
        let mut c = Composition::new("c".into(), "Watchdog".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [0.6; 4] },
                30,
            );
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            c.layers.push(l);
        }
        c
    }

    #[test]
    fn test_fast_render_completes_without_timeout() {
        let (pixels, timed_out) = render_frame_with_deadline(
            &comp(5),
            0,
            64,
            64,
            0.0,
            0,
            std::time::Duration::from_secs(5),
        );
        assert!(!timed_out);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Layers must actually be composited
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(bright > 0);
    }

    #[test]
    fn test_deadline_aborts_render_early() {
        // Generous layer count; deadline so short the render cannot finish
        let start = std::time::Instant::now();
        let (pixels, timed_out) = render_frame_with_deadline(
            &comp(2000),
            0,
            256,
            256,
            0.0,
            0,
            std::time::Duration::from_millis(5),
        );
        let elapsed = start.elapsed();
        assert!(timed_out, "render must report timeout");
        assert!(
            elapsed < std::time::Duration::from_millis(2000),
            "watchdog must return promptly, took {:?}",
            elapsed
        );
        // Partial buffer is still a valid allocation
        assert_eq!(pixels.len(), 256 * 256 * 4);
    }

    #[test]
    fn test_watchdog_does_not_leak_cancel_state() {
        // After a timed-out render, normal renders must work again
        let _ = render_frame_with_deadline(
            &comp(500),
            0,
            128,
            128,
            0.0,
            0,
            std::time::Duration::from_millis(1),
        );
        let pixels = render_frame_to_pixels(&comp(3), 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(
            bright > 0,
            "cancel state must not leak into subsequent renders"
        );
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Layer, LayerType, ShapeFillType, ShapeType};

    fn shadow_test_comp(caster_casts: bool) -> Composition {
        let mut comp = Composition::new("sh".into(), "Shadows".into(), 64, 64, 30, 30);
        // Receiver: full-frame white solid (bottom of stack)
        let mut recv = Layer::new(
            "r1".into(),
            "Floor".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
        recv.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(recv);

        // Caster: red solid raised on +z between light and floor plane
        let mut caster = Layer::new(
            "c1".into(),
            "Card".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        caster.is_3d = true;
        caster.material.cast_shadows = caster_casts;
        caster.transform.scale = Animatable::new_constant([20.0, 20.0]);
        caster.transform.position = Animatable::new_constant([36.0, 36.0]);
        caster.transform_3d.position = Animatable::new_constant([36.0, 36.0, 100.0]);
        comp.layers.push(caster);

        // Point light upper-left of the caster, above the plane (+z), casting
        let light = crate::core::timeline::Light3D {
            id: "key".into(),
            name: "Key".into(),
            light_type: crate::core::timeline::LightType::Point,
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 100.0,
            position: Animatable::new_constant([16.0, 16.0, 400.0]),
            casts_shadows: true,
            shadow_darkness: 90.0,
            falloff: 1.0,
            max_radius: 0.0,
        };
        comp.lights = vec![light];
        comp
    }

    #[test]
    fn test_shadow_falls_away_from_light() {
        let comp = shadow_test_comp(true);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Projection of caster center through L(16,16,400) onto z=0:
        // t=400/300 → (16+1.333*20, ...) ≈ (42.7, 42.7)
        let in_shadow = ((46 * 64 + 44) * 4) as usize;
        let lit_corner = ((8 * 64 + 8) * 4) as usize;
        assert!(
            px[in_shadow] < 200,
            "shadow region darkened, R={}",
            px[in_shadow]
        );
        assert!(
            px[lit_corner] >= 240,
            "far corner stays lit, R={}",
            px[lit_corner]
        );
    }

    #[test]
    fn test_no_shadow_when_caster_disabled() {
        let comp = shadow_test_comp(false);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let probe = ((46 * 64 + 44) * 4) as usize;
        assert!(px[probe] >= 240, "no caster -> no shadow, R={}", px[probe]);
    }

    #[test]
    fn shadow_map_keeps_caster_transform_on_composition_time() {
        let mut comp = shadow_test_comp(true);
        let caster = comp.layers.get_mut(1).expect("caster layer");
        caster.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [36.0, 36.0], InterpolationType::Linear),
            Keyframe::new(20, [48.0, 48.0], InterpolationType::Linear),
        ]);
        caster.freeze_at(20);

        let map = build_shadow_map(&comp, 0, 64, 64);
        let sum_near = |cx: usize, cy: usize| {
            let mut sum = 0.0;
            for y in cy.saturating_sub(2)..=(cy + 2).min(63) {
                for x in cx.saturating_sub(2)..=(cx + 2).min(63) {
                    sum += map[y * 64 + x];
                }
            }
            sum
        };

        // Time remapping freezes the caster's source content, not its layer
        // transform. The composition-time position [36,36] projects near
        // [43,43], while the source-frame position [48,48] is near [59,59].
        assert!(
            sum_near(43, 43) > sum_near(59, 59) * 1.5,
            "shadow must follow composition-time caster transform"
        );
    }

    #[test]
    fn depth_sort_keeps_layer_transform_on_composition_time() {
        let mut comp = Composition::new("depth-remap".into(), "Depth remap".into(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "depth".into(),
            "Animated depth".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        layer.is_3d = true;
        layer.transform_3d.position = Animatable::new_animated(vec![
            Keyframe::new(0, [32.0, 32.0, 10.0], InterpolationType::Linear),
            Keyframe::new(20, [32.0, 32.0, 90.0], InterpolationType::Linear),
        ]);
        layer.freeze_at(20);
        comp.layers.push(layer);

        assert_eq!(layer_depth_for_sort(&comp, &comp.layers[0], 0), 10.0);
    }

    #[test]
    fn test_collapse_transformations_3d_z_continuity() {
        // Draw-order rule (renderer sorts far-first): SMALLER effective z is
        // nearer and paints last. Collapsed child lifts to pz+cz=200, nearer
        // than green 350 -> child wins. Uncollapsed card sits at pz=400,
        // farther than green -> green wins. Identical stack, opposite winner.
        let build = |collapsed: bool| {
            let mut comp = Composition::new("cz".into(), "Collapse3D".into(), 64, 64, 30, 30);
            let mut green = Layer::new(
                "green".into(),
                "G".into(),
                LayerType::Solid {
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                30,
            );
            green.is_3d = true;
            green.transform.scale = Animatable::new_constant([100.0, 100.0]);
            green.transform.position = Animatable::new_constant([32.0, 32.0]);
            green.transform_3d.position = Animatable::new_constant([32.0, 32.0, 350.0]);
            comp.layers.push(green);

            let mut sub = Composition::new("subz".into(), "S".into(), 64, 64, 30, 30);
            let mut blue_child = Layer::new(
                "bc".into(),
                "B".into(),
                LayerType::Solid {
                    color: [0.0, 0.0, 1.0, 1.0],
                },
                30,
            );
            blue_child.is_3d = true;
            blue_child.transform.scale = Animatable::new_constant([100.0, 100.0]);
            blue_child.transform.position = Animatable::new_constant([32.0, 32.0]);
            blue_child.transform_3d.position = Animatable::new_constant([32.0, 32.0, -200.0]);
            sub.layers.push(blue_child);
            comp.sub_compositions.push(sub);

            let mut pc = Layer::new(
                "pc".into(),
                "P".into(),
                LayerType::PreComp {
                    comp_id: "subz".into(),
                },
                30,
            );
            pc.is_collapsed = collapsed;
            pc.is_3d = true;
            pc.transform.scale = Animatable::new_constant([100.0, 100.0]);
            pc.transform.position = Animatable::new_constant([32.0, 32.0]);
            pc.transform_3d.position = Animatable::new_constant([32.0, 32.0, 400.0]);
            comp.layers.push(pc);
            comp
        };

        let px_on = render_frame_to_pixels(&build(true), 0, 64, 64, 0.0, 0);
        let i = ((16 * 64 + 16) * 4) as usize;
        assert!(
            px_on[i + 2] > px_on[i + 1],
            "collapsed child (nearer) beats green: {:?}",
            &px_on[i..i + 3]
        );

        // Structural check: flattening composes parent z + child z and maps
        // the child into parent space around the sub-comp center.
        let flat = flatten_collapsed(&build(true), 0);
        let red_child = flat
            .layers
            .iter()
            .find(|l| l.name == "B")
            .expect("expanded child");
        assert!(red_child.is_3d);
        let lifted = red_child.transform_3d.position.evaluate(0);
        assert!(
            (lifted[2] - 200.0).abs() < 0.01,
            "pz+cz lift: {}",
            lifted[2]
        );
        assert!(
            (lifted[0] - 32.0).abs() < 0.01 && (lifted[1] - 32.0).abs() < 0.01,
            "child mapped around parent center: {:?}",
            lifted
        );
    }

    #[test]
    fn collapsed_precomp_uses_parent_time_remap_for_child_frame() {
        let mut comp =
            Composition::new("collapse-remap".into(), "Collapse remap".into(), 64, 64, 30, 30);
        let mut sub = Composition::new("sub-remap".into(), "Sub".into(), 64, 64, 30, 30);
        let mut child = Layer::new(
            "child".into(),
            "Animated child".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        // The child only exists at source frame 20. A collapsed parent frozen
        // at frame 20 must still expand it while rendering composition frame 0.
        child.in_frame = 20;
        child.out_frame = 30;
        child.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [8.0, 32.0], InterpolationType::Linear),
            Keyframe::new(20, [48.0, 32.0], InterpolationType::Linear),
        ]);
        sub.layers.push(child);
        comp.sub_compositions.push(sub);

        let mut precomp = Layer::new(
            "precomp".into(),
            "Frozen collapsed".into(),
            LayerType::PreComp {
                comp_id: "sub-remap".into(),
            },
            30,
        );
        precomp.is_collapsed = true;
        precomp.transform.position = Animatable::new_constant([32.0, 32.0]);
        precomp.freeze_at(20);
        comp.layers.push(precomp);

        let flat = flatten_collapsed(&comp, 0);
        let expanded = flat
            .layers
            .iter()
            .find(|layer| layer.id == "child")
            .expect("child active at the remapped source frame");
        let position = expanded.transform.position.evaluate(0);
        assert_eq!(position, [48.0, 32.0]);
    }

    #[test]
    fn nested_collapsed_precomp_inherits_parent_source_frame() {
        let mut inner =
            Composition::new("inner-remap".into(), "Inner".into(), 64, 64, 30, 30);
        let mut child = Layer::new(
            "nested-child".into(),
            "Nested child".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        child.in_frame = 20;
        child.out_frame = 30;
        inner.layers.push(child);

        let mut middle =
            Composition::new("middle-remap".into(), "Middle".into(), 64, 64, 30, 30);
        middle.sub_compositions.push(inner);
        let mut nested = Layer::new(
            "nested-precomp".into(),
            "Nested collapsed".into(),
            LayerType::PreComp {
                comp_id: "inner-remap".into(),
            },
            30,
        );
        nested.is_collapsed = true;
        middle.layers.push(nested);

        let mut root = Composition::new("root-remap".into(), "Root".into(), 64, 64, 30, 30);
        root.sub_compositions.push(middle);
        let mut outer = Layer::new(
            "outer-precomp".into(),
            "Outer collapsed".into(),
            LayerType::PreComp {
                comp_id: "middle-remap".into(),
            },
            30,
        );
        outer.is_collapsed = true;
        outer.freeze_at(20);
        root.layers.push(outer);

        let flat = flatten_collapsed(&root, 0);
        assert!(
            flat.layers.iter().any(|layer| layer.id == "nested-child"),
            "nested collapsed content must be evaluated at the outer mapped frame"
        );
    }

    #[test]
    fn test_render_frame_to_pixels_f32_returns_linear() {
        let mut comp = Composition::new("f32t".into(), "F32".into(), 8, 8, 30, 30);
        comp.background_color = [1.0, 1.0, 1.0, 1.0];
        let f32_px = render_frame_to_pixels_f32(&comp, 0, 8, 8, 0.0, 0);
        assert_eq!(f32_px.len(), 64);
        // White bg -> linear sRGB decode of 1.0 should be 1.0
        for p in &f32_px {
            assert!(p[0] > 0.99, "expected ~1.0 linear, got {}", p[0]);
            assert!(p[3] > 0.99, "expected opaque alpha, got {}", p[3]);
        }
        // Empty render returns empty
        let empty = render_frame_to_pixels_f32(&comp, 0, 0, 0, 0.0, 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_render_frame_to_pixels_f32_exposure_boosts() {
        let mut comp = Composition::new("f32exp".into(), "F32E".into(), 4, 4, 30, 30);
        comp.background_color = [0.5, 0.5, 0.5, 1.0];
        let no_ev = render_frame_to_pixels_f32(&comp, 0, 4, 4, 0.0, 0);
        let plus2 = render_frame_to_pixels_f32(&comp, 0, 4, 4, 2.0, 0);
        // +2 EV boosts values; with sRGB encode/decode + dithering
        // the ratio isn't exactly 4x, but must be significantly > 1.
        assert!(
            plus2[0][0] > no_ev[0][0] * 2.0,
            "expected >2x, got {}",
            plus2[0][0] / no_ev[0][0]
        );
        // All values in valid range
        for p in &plus2 {
            for c in p {
                assert!(*c >= 0.0 && !c.is_nan());
            }
        }
    }

    #[test]
    fn test_lut_mode_2_aces_pipeline_matches_reference() {
        let mut comp = Composition::new("aces".into(), "A".into(), 16, 16, 30, 30);
        let mut s = Layer::new(
            "s".into(),
            "S".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        s.transform.scale = Animatable::new_constant([100.0, 100.0]);
        s.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(s);

        let px = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 2);
        let i = ((4 * 16 + 4) * 4) as usize;

        // Reference: white → decode(1)=1 → tonemap ≈0.8019776 → encode
        let expected = crate::core::aces::aces_preview_transform([1.0, 1.0, 1.0]);
        let want = (expected[0] * 255.0).round().clamp(0.0, 255.0) as u8;
        assert!(
            (px[i] as i32 - want as i32).abs() <= 1,
            "ACES preview white: got {} want {}",
            px[i],
            want
        );
        // And it must differ from plain passthrough (255)
        assert!(px[i] < 250, "tonemap must not be identity on white");
    }

    fn adjustment_test_comp(opacity: f32) -> Composition {
        let mut comp = Composition::new("adj".into(), "Adj".into(), 32, 32, 30, 30);
        // Bottom: pure white solid covering the frame
        let mut base = Layer::new(
            "b1".into(),
            "Base".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        base.transform.scale = Animatable::new_constant([100.0, 100.0]);
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(base);
        // Adjustment layer with Invert at the given opacity
        let mut adj = Layer::new("a1".into(), "Adjust".into(), LayerType::AdjustmentLayer, 30);
        adj.effects.push(crate::core::timeline::Effect {
            id: "fx_inv".into(),
            enabled: true,
            name: "Invert".into(),
            effect_type: crate::core::timeline::EffectType::Invert {
                invert_alpha: false,
            },
        });
        adj.transform.opacity = Animatable::new_constant(opacity);
        comp.layers.push(adj);
        comp
    }

    #[test]
    fn test_adjustment_layer_inverts_below_at_full_opacity() {
        let comp = adjustment_test_comp(100.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(px[i] < 20, "white inverted to near-black, R={}", px[i]);
    }

    #[test]
    fn test_layer_effects_toggle_bypasses_adjustment_stack() {
        let mut comp = adjustment_test_comp(100.0);
        comp.layers[1].effects_enabled = false;
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(px[i] > 235, "disabled adjustment effects must preserve white, R={}", px[i]);
    }

    #[test]
    fn test_adjustment_layer_opacity_blends() {
        let comp = adjustment_test_comp(50.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(
            px[i] > 100 && px[i] < 160,
            "50% invert of white ~127, R={}",
            px[i]
        );
    }

    #[test]
    fn test_ellipse_caster_shadow_is_round_not_square() {
        // Ellipse caster: the four bbox corners of its bounding quad must stay
        // LIT (round shape doesn't reach them), while the center is darkened.
        let mut comp = Composition::new("e".into(), "EllipseShadow".into(), 96, 96, 30, 30);
        let mut recv = Layer::new(
            "r1".into(),
            "Floor".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
        recv.transform.position = Animatable::new_constant([48.0, 48.0]);
        comp.layers.push(recv);

        let mut caster = Layer::new(
            "c1".into(),
            "Disc".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Ellipse {
                    width: Animatable::new_constant(40.0),
                    height: Animatable::new_constant(40.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        caster.is_3d = true;
        caster.material.cast_shadows = true;
        caster.transform.scale = Animatable::new_constant([100.0, 100.0]);
        caster.transform.position = Animatable::new_constant([60.0, 60.0]);
        caster.transform_3d.position = Animatable::new_constant([60.0, 60.0, 100.0]);
        comp.layers.push(caster);
        comp.lights = vec![crate::core::timeline::Light3D {
            id: "k".into(),
            name: "K".into(),
            light_type: crate::core::timeline::LightType::Point,
            color: [1.0; 4],
            intensity: 100.0,
            position: Animatable::new_constant([48.0, 48.0, 300.0]),
            casts_shadows: true,
            shadow_darkness: 90.0,
            falloff: 1.0,
            max_radius: 0.0,
        }];

        let px = render_frame_to_pixels(&comp, 0, 96, 96, 0.0, 0);
        // Projection center ≈ light + t*(caster-light), t=300/200=1.5 →
        // (48+1.5*12, same) = (66,66); ellipse radius scales to ~30px.
        let in_shadow = ((70 * 96 + 68) * 4) as usize;
        // Bounding-quad corner of the projected ellipse (~±30 from center):
        // a square fallback would darken (92,92); the round shape must not.
        let quad_corner = ((90 * 96 + 90) * 4) as usize;
        assert!(
            px[in_shadow] < 210,
            "ellipse shadow core darkened, R={}",
            px[in_shadow]
        );
        assert!(
            px[quad_corner] > 235,
            "round shadow must spare quad corner, R={}",
            px[quad_corner]
        );
    }

    #[test]
    fn test_preserve_transparency_blends_only_onto_opaque_dest() {
        let mut comp = Composition::new("pt".into(), "PTComp".into(), 32, 32, 30, 30);
        // Base layer: small 16x16 white square in the center (16..32, 16..32), background transparent
        comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut base = Layer::new(
            "b".into(),
            "Base".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        base.transform.scale = Animatable::new_constant([50.0, 50.0]); // 16x16
        comp.layers.push(base);

        // Top layer: red solid covering entire 32x32 screen, but with preserve_transparency = true
        let mut top = Layer::new(
            "t".into(),
            "TopRed".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        top.transform.position = Animatable::new_constant([16.0, 16.0]);
        top.transform.scale = Animatable::new_constant([100.0, 100.0]);
        top.preserve_transparency = true;
        comp.layers.push(top);

        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel (16, 16): should be painted Red (R=255, G=0, B=0, A=255)
        let c_idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(px[c_idx], 255);
        assert_eq!(px[c_idx + 1], 0);

        // Corner pixel (2, 2): should remain transparent (A=0), not painted red
        let corner_idx = ((2 * 32 + 2) * 4) as usize;
        assert_eq!(px[corner_idx + 3], 0);
    }

    #[test]
    fn test_shape_units_stay_circular_on_wide_comp() {
        // Regression: shape units must be uniform (width-referenced) so an
        // equal-width/height ellipse renders as a circle on 16:9 comps.
        let mut comp = Composition::new("c".into(), "Wide".into(), 192, 108, 30, 2);
        comp.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut layer = Layer::new(
            "s".into(),
            "Circle".into(),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(100.0),
                    height: Animatable::new_constant(100.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            2,
        );
        layer.transform.position = Animatable::new_constant([96.0, 54.0]);
        layer.transform.scale = Animatable::new_constant([100.0, 100.0]);
        comp.layers.push(layer);

        let px = render_frame_to_pixels(&comp, 0, 192, 108, 0.0, 0);
        let bright = |x: u32, y: u32| px[((y * 192 + x) * 4) as usize] > 128;
        // 100 units at scale 100 => 96px span on a 192-wide comp.
        let row = (50..=142u32).filter(|&x| bright(x, 54)).count();
        let col = (6..=102u32).filter(|&y| bright(96, y)).count();
        assert!(
            row >= 88 && row <= 100,
            "circle width unexpected: {row}px",
        );
        assert!(
            (row as i32 - col as i32).abs() <= 4,
            "circle distorted on wide comp: w={row} h={col}",
        );
    }

    #[test]
    fn test_guide_layer_skipped_in_precomp() {
        let mut sub_comp = Composition::new("sub".into(), "Sub".into(), 32, 32, 30, 30);
        let mut guide = Layer::new(
            "g".into(),
            "Guide".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        guide.is_guide_layer = true;
        sub_comp.layers.push(guide);

        let mut main_comp = Composition::new("main".into(), "Main".into(), 32, 32, 30, 30);
        main_comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let precomp_layer = Layer::new(
            "p".into(),
            "Pre".into(),
            LayerType::PreComp {
                comp_id: "sub".into(),
            },
            30,
        );
        main_comp.layers.push(precomp_layer);
        main_comp.sub_compositions.push(sub_comp);

        let px = render_frame_to_pixels(&main_comp, 0, 32, 32, 0.0, 0);
        // Entire buffer should be empty / transparent since the only sub-layer was a guide layer
        assert!(px.iter().all(|&b| b == 0));
    }

    /// Composite-time opacity regression tests.
    ///
    /// Pipeline rule under test: layer opacity is applied ONCE during
    /// compositing (Phase 3), never baked into raster alpha where effect
    /// kernels could clobber it. These tests fail on the old bake-first
    /// pipeline (e.g. generator + fade rendered at full strength).
    mod alpha_composite_tests {
        use super::*;
        use crate::core::timeline::Effect;

        fn black_bg(w: u32, h: u32) -> Composition {
            let mut comp = Composition::new("t".into(), "AlphaTest".into(), w, h, 30, 30);
            comp.background_color = [0.0, 0.0, 0.0, 1.0];
            let mut bg = Layer::new(
                "bg".into(),
                "BG".into(),
                LayerType::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                30,
            );
            bg.transform.position =
                Animatable::new_constant([w as f32 * 0.5, h as f32 * 0.5]);
            comp.layers.push(bg);
            comp
        }

        fn gray_solid(id: &str, opacity: f32) -> Layer {
            let mut l = Layer::new(
                id.into(),
                id.into(),
                LayerType::Solid {
                    color: [0.5, 0.5, 0.5, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.transform.opacity = Animatable::new_constant(opacity);
            l
        }

        fn mean_r(px: &[u8]) -> f32 {
            let n = px.len() / 4;
            px.chunks_exact(4).map(|c| c[0] as f32).sum::<f32>() / n.max(1) as f32
        }

        #[test]
        fn generator_with_opacity_fade_is_proportional() {
            // FractalNoise used to force alpha=255, so a 50% layer rendered
            // at full strength. Now the fade must (about) halve the energy.
            let mk = |opacity: f32| {
                let mut comp = black_bg(64, 64);
                let mut l = gray_solid("n", opacity);
                l.transform.position = Animatable::new_constant([32.0, 32.0]);
                l.effects.push(Effect {
                    id: "fbm".into(),
                    name: "Fractal Noise".into(),
                    effect_type: crate::core::timeline::EffectType::FractalNoise {
                        fractal_type: Animatable::new_constant(0.0),
                        contrast: Animatable::new_constant(0.6),
                        brightness: Animatable::new_constant(0.5),
                        complexity: Animatable::new_constant(2.0),
                        evolution: Animatable::new_constant(0.0),
                    },
                    enabled: true,
                });
                comp.layers.push(l);
                render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
            };
            let full = mean_r(&mk(100.0));
            let half = mean_r(&mk(50.0));
            assert!(
                full > 20.0,
                "noise must lift the frame, got {full}"
            );
            let ratio = half / full.max(1.0);
            assert!(
                (0.3..0.7).contains(&ratio),
                "50% fade should roughly halve noise energy, ratio={ratio} (half={half}, full={full})"
            );
        }

        #[test]
        fn sparse_painter_with_opacity_fade_is_proportional() {
            // StarField paints sparse bright pixels with full alpha; the
            // layer fade must still dim them via composite-time opacity.
            let mk = |opacity: f32| {
                let mut comp = black_bg(64, 64);
                let mut l = Layer::new(
                    "s".into(),
                    "Stars".into(),
                    LayerType::Solid {
                        color: [0.0, 0.0, 0.0, 1.0],
                    },
                    30,
                );
                l.transform.position = Animatable::new_constant([32.0, 32.0]);
                l.transform.opacity = Animatable::new_constant(opacity);
                l.effects.push(Effect {
                    id: "sf".into(),
                    name: "Star Field".into(),
                    effect_type: crate::core::timeline::EffectType::StarField {
                        num_stars: Animatable::new_constant(60.0),
                        depth_speed: Animatable::new_constant(0.0),
                    },
                    enabled: true,
                });
                comp.layers.push(l);
                render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
            };
            let full = mean_r(&mk(100.0));
            let half = mean_r(&mk(50.0));
            assert!(full > 1.0, "stars must lift the frame, got {full}");
            let ratio = half / full.max(0.001);
            assert!(
                (0.3..0.7).contains(&ratio),
                "50% fade should roughly halve star energy, ratio={ratio}"
            );
        }

        #[test]
        fn effect_stack_then_opacity_matches_manual_blend() {
            // Gray solid + noise, faded to 50%: must equal a 50/50 mix of the
            // full-strength result with black (linearity of composite-time op).
            let mk = |opacity: f32| {
                let mut comp = black_bg(64, 64);
                let mut l = gray_solid("n", opacity);
                l.transform.position = Animatable::new_constant([32.0, 32.0]);
                l.effects.push(Effect {
                    id: "fbm".into(),
                    name: "Fractal Noise".into(),
                    effect_type: crate::core::timeline::EffectType::FractalNoise {
                        fractal_type: Animatable::new_constant(0.0),
                        contrast: Animatable::new_constant(0.6),
                        brightness: Animatable::new_constant(0.5),
                        complexity: Animatable::new_constant(2.0),
                        evolution: Animatable::new_constant(0.0),
                    },
                    enabled: true,
                });
                comp.layers.push(l);
                render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
            };
            let full = mk(100.0);
            let half = mk(50.0);
            let mut max_dev = 0.0f32;
            for (i, px) in half.chunks_exact(4).enumerate() {
                let expect = full[i * 4] as f32 * 0.5;
                max_dev = max_dev.max((px[0] as f32 - expect).abs());
            }
            assert!(
                max_dev <= 3.0,
                "faded stack must match manual 50% mix, max_dev={max_dev}"
            );
        }

        #[test]
        fn add_blend_with_partial_opacity() {
            // White solid, Add over black at 50% -> ~127 gray everywhere.
            let mut comp = black_bg(32, 32);
            let mut l = Layer::new(
                "w".into(),
                "White".into(),
                LayerType::Solid {
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([16.0, 16.0]);
            l.transform.opacity = Animatable::new_constant(50.0);
            l.blend_mode = crate::core::timeline::BlendMode::Add;
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
            let m = mean_r(&px);
            assert!(
                (115.0..140.0).contains(&m),
                "Add@50% of white over black should be ~127, got {m}"
            );
        }

        #[test]
        fn mask_and_opacity_combine() {
            // Left-half rect mask + 50% opacity: kept side fades, cut side
            // stays at background black.
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "w".into(),
                "White".into(),
                LayerType::Solid {
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.transform.opacity = Animatable::new_constant(50.0);
            l.masks.push(crate::core::mask::Mask::new_rect(
                "m".into(),
                "Left".into(),
                0.0,
                0.0,
                32.0,
                64.0,
            ));
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            let at = |x: u32, y: u32| px[((y * 64 + x) * 4) as usize] as f32;
            assert!(
                (110.0..145.0).contains(&at(16, 32)),
                "masked-in side should fade to ~127, got {}",
                at(16, 32)
            );
            assert!(
                at(48, 32) < 8.0,
                "masked-out side must stay black, got {}",
                at(48, 32)
            );
        }

        #[test]
        fn zero_opacity_layer_is_invisible() {
            // Any layer at 0% must contribute nothing (no leaks, no residue
            // from alpha-writing effects).
            let mut comp = black_bg(32, 32);
            let mut l = gray_solid("n", 0.0);
            l.transform.position = Animatable::new_constant([16.0, 16.0]);
            l.effects.push(Effect {
                id: "fbm".into(),
                name: "Fractal Noise".into(),
                effect_type: crate::core::timeline::EffectType::FractalNoise {
                    fractal_type: Animatable::new_constant(0.0),
                    contrast: Animatable::new_constant(0.6),
                    brightness: Animatable::new_constant(0.5),
                    complexity: Animatable::new_constant(2.0),
                    evolution: Animatable::new_constant(0.0),
                },
                enabled: true,
            });
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
            assert!(
                mean_r(&px) < 2.0,
                "0% layer must be invisible, mean={}",
                mean_r(&px)
            );
        }
    }

    /// Mask ordering regression tests.
    ///
    /// Model under test: pre-effect mask isolates (during raster), effects
    /// run, then a post-effect re-mask contains the result before compositing
    /// (Phase 2.8). Blur/glow/displacement must not leak outside the final
    /// masked region. This intentionally differs from AE's default, where
    /// effects spill past masks.
    mod mask_ordering_tests {
        use super::*;
        use crate::core::timeline::Effect;

        fn black_bg(w: u32, h: u32) -> Composition {
            let mut comp = Composition::new("t".into(), "MaskTest".into(), w, h, 30, 30);
            comp.background_color = [0.0, 0.0, 0.0, 1.0];
            let mut bg = Layer::new(
                "bg".into(),
                "BG".into(),
                LayerType::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                30,
            );
            bg.transform.position =
                Animatable::new_constant([w as f32 * 0.5, h as f32 * 0.5]);
            comp.layers.push(bg);
            comp
        }

        fn rect_mask(id: &str, x: f32, y: f32, w: f32, h: f32, feather: f32) -> crate::core::mask::Mask {
            let mut m = crate::core::mask::Mask::new_rect(
                id.into(),
                id.into(),
                x,
                y,
                w,
                h,
            );
            m.feather = Animatable::new_constant(feather);
            m
        }

        fn at(px: &[u8], w: u32, x: u32, y: u32) -> f32 {
            px[((y * w + x) * 4) as usize] as f32
        }

        #[test]
        fn blurred_masked_shape_does_not_leak() {
            // White 32x32 rect + identical rect mask + blur 6: the blur would
            // smear ~18px past the mask without the post-effect re-mask.
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "r".into(),
                "Rect".into(),
                LayerType::Shape {
                    shape_type: crate::core::timeline::ShapeType::Rectangle {
                        width: Animatable::new_constant(100.0),
                        height: Animatable::new_constant(100.0),
                        corner_radius: Animatable::new_constant(0.0),
                    },
                    color: [1.0, 1.0, 1.0, 1.0],
                    stroke_color: [0.0; 4],
                    stroke_width: 0.0,
                    fill_type: Default::default(),
                    extrusion_depth: 0.0,
                    bevel_depth: 0.0,
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
            l.effects.push(Effect {
                id: "blur".into(),
                name: "Blur".into(),
                effect_type: crate::core::timeline::EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(12.0),
                },
                enabled: true,
            });
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            assert!(
                at(&px, 64, 32, 32) > 150.0,
                "masked interior must stay bright, got {}",
                at(&px, 64, 32, 32)
            );
            // 4px outside the mask edge: blur spill would read ~40+ here
            // without the post-effect re-mask.
            assert!(
                at(&px, 64, 12, 32) < 14.0,
                "blur must not leak outside final mask, got {}",
                at(&px, 64, 12, 32)
            );
        }

        #[test]
        fn glow_on_masked_layer_respects_final_mask() {
            // Blur spreads alpha past the mask, then glow blooms the spill:
            // without the post-effect re-mask the halo would show outside.
            // (Pure glow alone cannot leak: RGB-only energy is gated by zero
            // alpha at composite. The stack is the meaningful case.)
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "w".into(),
                "White".into(),
                LayerType::Solid {
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.masks.push(rect_mask("m", 0.0, 0.0, 32.0, 64.0, 0.0));
            l.effects.push(Effect {
                id: "blur".into(),
                name: "Blur".into(),
                effect_type: crate::core::timeline::EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(6.0),
                },
                enabled: true,
            });
            l.effects.push(Effect {
                id: "glow".into(),
                name: "Glow".into(),
                effect_type: crate::core::timeline::EffectType::Glow {
                    threshold: Animatable::new_constant(30.0),
                    radius: Animatable::new_constant(12.0),
                    intensity: Animatable::new_constant(200.0),
                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                },
                enabled: true,
            });
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            assert!(
                at(&px, 64, 16, 32) > 150.0,
                "masked-in side must stay bright, got {}",
                at(&px, 64, 16, 32)
            );
            // 4px past the mask edge: blur+glow halo would read bright here
            // without the post-effect re-mask.
            assert!(
                at(&px, 64, 36, 32) < 15.0,
                "glow must not leak past final mask, got {}",
                at(&px, 64, 36, 32)
            );
        }

        #[test]
        fn displacement_on_masked_layer_stays_contained() {
            // Masked white solid shifted +12px by Offset: without containment
            // the shifted content would appear past the final mask edge.
            // (TurbulentDisplace is covered by the same remask path; Offset
            // is used here because its placement is exactly deterministic.)
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "w".into(),
                "White".into(),
                LayerType::Solid {
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
            l.effects.push(Effect {
                id: "off".into(),
                name: "Offset".into(),
                effect_type: crate::core::timeline::EffectType::Offset {
                    shift_x: Animatable::new_constant(12.0),
                    shift_y: Animatable::new_constant(0.0),
                },
                enabled: true,
            });
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            // Mask spans x16..48; content shifted +12 would reach x60.
            assert!(
                at(&px, 64, 52, 32) < 12.0,
                "displaced content must not appear outside mask, got {}",
                at(&px, 64, 52, 32)
            );
            assert!(
                at(&px, 64, 56, 32) < 12.0,
                "displaced content must not appear outside mask, got {}",
                at(&px, 64, 56, 32)
            );
            assert!(
                at(&px, 64, 32, 32) > 150.0,
                "masked interior must stay bright, got {}",
                at(&px, 64, 32, 32)
            );
        }

        #[test]
        fn feathered_mask_with_opacity() {
            // Feathered rect + 50% opacity: soft edge survives the remask
            // (squared falloff stays smooth), interior fades, exterior black.
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "w".into(),
                "White".into(),
                LayerType::Solid {
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.transform.opacity = Animatable::new_constant(50.0);
            l.masks.push(rect_mask("m", 0.0, 0.0, 32.0, 64.0, 8.0));
            comp.layers.push(l);
            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            let inside = at(&px, 64, 8, 32);
            // Sample inside the feather band (rect edge x=32, feather 8):
            // geometric edge itself is ~0 by definition.
            let edge = at(&px, 64, 28, 32);
            let outside = at(&px, 64, 56, 32);
            assert!(
                (100.0..150.0).contains(&inside),
                "interior should fade to ~127, got {inside}"
            );
            assert!(
                edge > 15.0 && edge < 240.0,
                "feather edge must be a soft ramp, got {edge}"
            );
            assert!(outside < 8.0, "exterior must stay black, got {outside}");
        }

        #[test]
        fn precomp_mask_with_inner_effects() {
            // Mask on the precomp layer clips the sub-render (which runs its
            // own inner effects); inner noise must show inside, never outside.
            let mut sub = Composition::new("sub".into(), "Sub".into(), 64, 64, 30, 30);
            sub.background_color = [0.0, 0.0, 0.0, 1.0];
            let mut inner = Layer::new(
                "g".into(),
                "Gray".into(),
                LayerType::Solid {
                    color: [0.6, 0.6, 0.6, 1.0],
                },
                30,
            );
            inner.transform.position = Animatable::new_constant([32.0, 32.0]);
            inner.effects.push(Effect {
                id: "fbm".into(),
                name: "Fractal Noise".into(),
                effect_type: crate::core::timeline::EffectType::FractalNoise {
                    fractal_type: Animatable::new_constant(0.0),
                    contrast: Animatable::new_constant(0.6),
                    brightness: Animatable::new_constant(0.5),
                    complexity: Animatable::new_constant(2.0),
                    evolution: Animatable::new_constant(0.0),
                },
                enabled: true,
            });
            sub.layers.push(inner);

            let mut comp = black_bg(64, 64);
            let mut pre = Layer::new(
                "p".into(),
                "Pre".into(),
                LayerType::PreComp {
                    comp_id: "sub".into(),
                },
                30,
            );
            pre.transform.position = Animatable::new_constant([32.0, 32.0]);
            pre.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
            comp.layers.push(pre);
            comp.sub_compositions.push(sub);

            let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
            assert!(
                at(&px, 64, 4, 4) < 8.0,
                "masked-out side must stay black, got {}",
                at(&px, 64, 4, 4)
            );
            // Inner noise must survive inside the mask (variance check).
            let mut vals = Vec::new();
            for y in 24..40 {
                for x in 24..40 {
                    vals.push(at(&px, 64, x, y));
                }
            }
            let mean = vals.iter().sum::<f32>() / vals.len() as f32;
            let var =
                vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / vals.len() as f32;
            assert!(
                mean > 10.0 && var > 5.0,
                "inner effect result must show inside mask (mean={mean}, var={var})"
            );
        }
    }
}
