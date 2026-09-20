//! Nested pre-composition rendering for the software renderer.
//!
//! Covers sub-comp entry points, nesting guards/caches, and the
//! recursive inner renderer (with its own opacity/matte handling).

use super::{render_frame_to_pixels_filtered, rgba_buffer_size};
use crate::core::sdf::rasterize_shape_sdf;
use crate::core::timeline::{Composition, LayerType};

/// Maximum pre-comp nesting depth before we bail out (cycle / pathological nesting guard).
pub const MAX_PRECOMP_DEPTH: u32 = 16;

thread_local! {
    static PRECOMP_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Stack of composition IDs currently being rendered (cycle detection).
    static PRECOMP_STACK: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Precomp render cache: avoids re-rendering the same sub-comp at the
    /// same frame when it's referenced by multiple layers.
    static PRECOMP_RENDER_CACHE: std::cell::RefCell<crate::core::precomp_cache::PrecompCache> = std::cell::RefCell::new(crate::core::precomp_cache::PrecompCache::new(64));
    /// Source-layer effects can reference another layer which references a
    /// third one, but a cycle must not recurse forever.
    static SOURCE_LAYER_RENDER_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

const MAX_SOURCE_LAYER_RENDER_DEPTH: u32 = 16;

/// Render a pre-comp by recursively rendering its layers into a pixel buffer.
/// This is the core of pre-comp nesting support. Uses a thread-local cache
/// to avoid redundant re-renders when the same pre-comp is referenced by
/// multiple layers at the same frame.
pub fn render_precomp_layers(
    _comp: &Composition,
    precomp_comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let content_revision = composition_content_revision(precomp_comp);
    // Cache check: skip full render if we already have this precomp's pixels
    let cached = PRECOMP_RENDER_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .get(&precomp_comp.id, content_revision, frame, width, height)
    });
    if let Some(pixels) = cached {
        return pixels;
    }

    // Cycle detection: a comp that (indirectly) contains itself returns empty
    // immediately instead of burning the whole depth budget on garbage.
    let cyclic = PRECOMP_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        if s.iter().any(|id| id == &precomp_comp.id) {
            true
        } else {
            s.push(precomp_comp.id.clone());
            false
        }
    });
    if cyclic {
        log::warn!(
            "[Renderer] Pre-comp cycle detected at '{}' ; skipping nested render",
            precomp_comp.name
        );
        return Vec::new();
    }
    // Guard against pathologically deep (but acyclic) pre-comp nesting
    let overflow = PRECOMP_DEPTH.with(|d| {
        let cur = d.get();
        if cur >= MAX_PRECOMP_DEPTH {
            true
        } else {
            d.set(cur + 1);
            false
        }
    });
    if overflow {
        log::warn!("[Renderer] Pre-comp nesting depth limit exceeded; skipping nested render");
        PRECOMP_STACK.with(|s| {
            s.borrow_mut().pop();
        });
        return Vec::new();
    }
    let result = render_precomp_layers_inner(_comp, precomp_comp, frame, width, height);
    PRECOMP_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    PRECOMP_STACK.with(|s| {
        s.borrow_mut().pop();
    });
    // Cache the result for future lookups at the same (comp, frame, resolution)
    if !result.is_empty() {
        PRECOMP_RENDER_CACHE.with(|cache| {
            cache.borrow_mut().insert(
                &precomp_comp.id,
                content_revision,
                frame,
                width,
                height,
                result.clone(),
            );
        });
    }
    result
}

fn composition_content_revision(comp: &Composition) -> u64 {
    let Ok(bytes) = serde_json::to_vec(comp) else {
        return crate::core::frame_cache::current_version();
    };
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

/// Rasterize an image sequence into a transformed layer buffer.
///
/// The main composition renderer and the nested pre-comp renderer must sample
/// the same source pixels. Keeping the sequence sampling here also makes the
/// WebP-first/PNG-fallback rule explicit for nested video layers.
fn rasterize_texture_layer(
    layer_buf: &mut [u8],
    path: &str,
    next_path: Option<&str>,
    blend_t: f32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    bw: u32,
    cx: f32,
    cy: f32,
    cos_r: f32,
    sin_r: f32,
    bounds_x: f32,
    bounds_y: f32,
) {
    use crate::core::image_cache::with_image_cache;

    with_image_cache(|cache| {
        let Some(img) = cache.load_image(path).cloned() else {
            return;
        };
        let next_img = next_path
            .filter(|candidate| *candidate != path && blend_t > 0.0)
            .and_then(|candidate| cache.load_image(candidate).cloned())
            .filter(|next| next.width == img.width && next.height == img.height);
        let img_w = img.width as f32;
        let img_h = img.height as f32;

        for py in min_y..max_y {
            for px in min_x..max_x {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                let lx = dx * cos_r + dy * sin_r;
                let ly = -dx * sin_r + dy * cos_r;
                let u = (lx / bounds_x + 1.0) * 0.5;
                let v = (ly / bounds_y + 1.0) * 0.5;
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    continue;
                }

                let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img.width - 1);
                let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img.height - 1);
                let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                if tidx + 3 >= img.pixels.len() {
                    continue;
                }
                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                if lidx + 3 >= layer_buf.len() {
                    continue;
                }

                let sample = |channel: usize| -> u8 {
                    let a = img.pixels[tidx + channel] as f32;
                    let b = next_img
                        .as_ref()
                        .and_then(|next| {
                            let next_idx = ((tex_y * next.width + tex_x) * 4) as usize;
                            next.pixels.get(next_idx + channel).copied()
                        })
                        .map(f32::from)
                        .unwrap_or(a);
                    (a + (b - a) * blend_t).round().clamp(0.0, 255.0) as u8
                };
                layer_buf[lidx] = sample(0);
                layer_buf[lidx + 1] = sample(1);
                layer_buf[lidx + 2] = sample(2);
                layer_buf[lidx + 3] = sample(3);
            }
        }
    });
}

/// Rasterize an already-rendered nested composition into a transformed layer
/// buffer. Keeping this sampling path separate from filesystem-backed footage
/// lets nested PreComps recurse without manufacturing temporary image files.
fn rasterize_pixel_layer(
    layer_buf: &mut [u8],
    pixels: &[u8],
    source_width: u32,
    source_height: u32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    bw: u32,
    cx: f32,
    cy: f32,
    cos_r: f32,
    sin_r: f32,
    bounds_x: f32,
    bounds_y: f32,
) {
    if source_width == 0 || source_height == 0 {
        return;
    }
    let source_size = (source_width as usize)
        .checked_mul(source_height as usize)
        .and_then(|size| size.checked_mul(4));
    if source_size != Some(pixels.len()) {
        return;
    }

    let source_w = source_width as f32;
    let source_h = source_height as f32;
    for py in min_y..max_y {
        for px in min_x..max_x {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let lx = dx * cos_r + dy * sin_r;
            let ly = -dx * sin_r + dy * cos_r;
            let u = (lx / bounds_x + 1.0) * 0.5;
            let v = (ly / bounds_y + 1.0) * 0.5;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }

            let sx = ((u * (source_w - 1.0)).round() as u32).min(source_width - 1);
            let sy = ((v * (source_h - 1.0)).round() as u32).min(source_height - 1);
            let source_idx = ((sy * source_width + sx) * 4) as usize;
            let layer_idx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
            if source_idx + 3 < pixels.len() && layer_idx + 3 < layer_buf.len() {
                layer_buf[layer_idx..layer_idx + 4]
                    .copy_from_slice(&pixels[source_idx..source_idx + 4]);
            }
        }
    }
}

/// Render a single layer from `comp` at the given frame, returning an RGBA8 buffer
/// of size `width * height * 4`. Used by effects like SetMatte that need another
/// layer's pixel data.
pub(crate) fn render_single_layer_pixels(
    comp: &Composition,
    layer_idx: usize,
    frame: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 || layer_idx >= comp.layers.len() {
        return vec![0u8; size as usize];
    }

    SOURCE_LAYER_RENDER_DEPTH.with(|depth| {
        let current = depth.get();
        if current >= MAX_SOURCE_LAYER_RENDER_DEPTH {
            log::warn!(
                "[Renderer] source-layer effect recursion exceeded {} levels",
                MAX_SOURCE_LAYER_RENDER_DEPTH
            );
            return vec![0u8; size as usize];
        }

        depth.set(current + 1);
        let pixels = render_frame_to_pixels_filtered(
            comp,
            frame,
            width,
            height,
            0.0,
            0,
            Some(layer_idx),
        );
        depth.set(current);
        pixels
    })
}

fn render_precomp_layers_inner(
    _comp: &Composition,
    precomp_comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; size];
    // Fill with transparent black
    for p in (0..size).step_by(4) {
        buffer[p] = 0;
        buffer[p + 1] = 0;
        buffer[p + 2] = 0;
        buffer[p + 3] = 0;
    }

    let has_solo = precomp_comp
        .layers
        .iter()
        .any(|l| l.is_active(frame) && l.solo);

    for (layer_idx, layer) in precomp_comp.layers.iter().enumerate() {
        if !layer.is_active(frame) || !layer.visible || layer.is_guide_layer {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }

        let effective_frame = {
            let f = layer.remap_frame(frame);
            match &layer.posterize_time {
                Some(pt) if pt.enabled => {
                    crate::core::posterize_time::quantize_frame_posterize(f, precomp_comp.fps, pt)
                }
                _ => f,
            }
        };
        let (pos, scale, rotation, opacity) =
            precomp_comp.resolve_world_transform(layer, effective_frame);
        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 {
            continue;
        }

        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            if !layer.effects.is_empty() && l_opacity > 0.003 {
                let mut adjusted = buffer.clone();
                crate::core::cpu_effects::apply_layer_effects(
                    Some(precomp_comp),
                    Some(layer_idx),
                    &mut adjusted,
                    width,
                    height,
                    &layer.effects,
                    effective_frame,
                    precomp_comp.fps,
                );
                for i in (0..buffer.len()).step_by(4) {
                    for c in 0..3 {
                        buffer[i + c] = (buffer[i + c] as f32 * (1.0 - l_opacity)
                            + adjusted[i + c] as f32 * l_opacity)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => {
                (precomp_comp.width as f32, precomp_comp.height as f32)
            }
            LayerType::Text { .. } => (precomp_comp.width as f32, precomp_comp.height as f32),
            LayerType::Shape { .. } => {
                // Shape units are uniform (composition-width referenced) so
                // circles stay circular on non-square compositions.
                (precomp_comp.width as f32, precomp_comp.width as f32)
            }
            LayerType::Image { .. } | LayerType::Video { .. } => {
                (precomp_comp.width as f32, precomp_comp.height as f32)
            }
            _ => continue,
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0],
            // Video content is filled by the sequence rasterizer below; the
            // fallback color only keeps this shared geometry setup alive.
            LayerType::Video { .. } => [1.0, 1.0, 1.0, 1.0],
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        let rad = rotation.to_radians();
        let (cos_r, sin_r) = rad.sin_cos();
        let cx = pos[0];
        let cy = pos[1];
        let bounds_x = w * 0.5;
        let bounds_y = h * 0.5;

        // NaN-safe, sign-safe bounding box (`as u32` saturates NaN/inf; abs() guards
        // against negative scale flipping the bounds)
        let ext_x = (bounds_x.abs() + 2.0) * 1.5;
        let ext_y = (bounds_y.abs() + 2.0) * 1.5;
        let min_x = ((cx - ext_x).max(0.0) as u32).min(width);
        let max_x = ((cx + ext_x).max(0.0) as u32).min(width);
        let min_y = ((cy - ext_y).max(0.0) as u32).min(height);
        let max_y = ((cy + ext_y).max(0.0) as u32).min(height);
        let bw = max_x.saturating_sub(min_x);
        let bh = max_y.saturating_sub(min_y);
        if bw == 0 || bh == 0 {
            continue;
        }

        let buf_size = (bw * bh * 4) as usize;
        let mut layer_buf = vec![0u8; buf_size];

        match &layer.layer_type {
            LayerType::Shape {
                shape_type,
                color,
                stroke_color,
                stroke_width,
                fill_type,
                ..
            } => {
                // SDF shape rendering (same path as the main renderer)
                rasterize_shape_sdf(
                    &mut layer_buf,
                    bw,
                    bh,
                    min_x,
                    min_y,
                    cx,
                    cy,
                    bounds_x,
                    bounds_y,
                    *color,
                    fill_type,
                    *stroke_color,
                    *stroke_width,
                    1.0,
                    shape_type,
                    effective_frame,
                    layer.trim_paths.as_ref(),
                );
            }
            LayerType::Image { path } => {
                rasterize_texture_layer(
                    &mut layer_buf,
                    path,
                    None,
                    0.0,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    bw,
                    cx,
                    cy,
                    cos_r,
                    sin_r,
                    bounds_x,
                    bounds_y,
                );
            }
            LayerType::Video {
                frames_dir,
                frame_count,
                speed,
                ..
            } => {
                let source_position = (layer.remap_frame_f32(frame) * speed.max(0.0)).max(0.0);
                let first = (source_position.floor() as u32).min(frame_count.saturating_sub(1));
                let second = (first + 1).min(frame_count.saturating_sub(1));
                let blend_t = if layer.frame_blending {
                    (source_position - first as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let first_path =
                    crate::core::video_import::frame_path_in_dir(frames_dir, first);
                let second_path =
                    crate::core::video_import::frame_path_in_dir(frames_dir, second);
                rasterize_texture_layer(
                    &mut layer_buf,
                    &first_path.to_string_lossy(),
                    Some(&second_path.to_string_lossy()),
                    blend_t,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    bw,
                    cx,
                    cy,
                    cos_r,
                    sin_r,
                    bounds_x,
                    bounds_y,
                );
            }
            LayerType::PreComp { comp_id } => {
                if let Some(nested_comp) = precomp_comp.find_sub_comp(comp_id) {
                    let nested_pixels = render_precomp_layers(
                        precomp_comp,
                        nested_comp,
                        effective_frame,
                        width,
                        height,
                    );
                    rasterize_pixel_layer(
                        &mut layer_buf,
                        &nested_pixels,
                        width,
                        height,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                        bw,
                        cx,
                        cy,
                        cos_r,
                        sin_r,
                        bounds_x,
                        bounds_y,
                    );
                }
            }
            LayerType::Text {
                text,
                font_size,
                color,
                font_family,
                tracking,
                ..
            } => {
                // Glyph rendering via font rasterizer
                use crate::core::font_rasterizer::with_font_rasterizer;
                let text_color = *color;
                let text_str = text.clone();
                let fs = *font_size as f32;
                let tk = *tracking;
                let family = font_family.clone();
                with_font_rasterizer(|rasterizer| {
                    let family_name = rasterizer.resolve_family(&family);
                    if let Some((tw, th, text_pixels)) =
                        rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk)
                    {
                        let origin_x = (cx - tw as f32 * 0.5) as i32;
                        let origin_y = (cy - th as f32 * 0.5) as i32;
                        // Tighten scan to bitmap bounds (same proof as above).
                        let text_w = tw as i32;
                        let text_h = th as i32;
                        let qx0 = origin_x.max(min_x as i32).clamp(0, width as i32) as u32;
                        let qy0 = origin_y.max(min_y as i32).clamp(0, height as i32) as u32;
                        let qx1 = (origin_x + text_w)
                            .clamp(min_x as i32, max_x as i32)
                            .max(qx0 as i32) as u32;
                        let qy1 = (origin_y + text_h)
                            .clamp(min_y as i32, max_y as i32)
                            .max(qy0 as i32) as u32;
                        for py in qy0..qy1 {
                            for px in qx0..qx1 {
                                let tx = px as i32 - origin_x;
                                let ty = py as i32 - origin_y;
                                if tx < 0 || ty < 0 || (tx as u32) >= tw || (ty as u32) >= th {
                                    continue;
                                }
                                let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                if tidx + 3 >= text_pixels.len() {
                                    continue;
                                }
                                let glyph_a = text_pixels[tidx + 3] as f32 / 255.0;
                                if glyph_a <= 0.001 {
                                    continue;
                                }
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = glyph_a;
                                    layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                    layer_buf[lidx + 1] = (text_color[1] * 255.0) as u8;
                                    layer_buf[lidx + 2] = (text_color[2] * 255.0) as u8;
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                });
            }
            _ => {
                // Flat fill for Solid / PreComp / others
                for py in min_y..max_y {
                    for px in min_x..max_x {
                        let dx = px as f32 - cx;
                        let dy = py as f32 - cy;
                        let lx = dx * cos_r + dy * sin_r;
                        let ly = -dx * sin_r + dy * cos_r;
                        if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                            let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                let src_a = base_color[3];
                                layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                                layer_buf[lidx + 1] = (base_color[1] * 255.0) as u8;
                                layer_buf[lidx + 2] = (base_color[2] * 255.0) as u8;
                                layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        }

        // ── Paint strokes: drawn in buffer space so they follow the layer ──
        // DirtyRect integration: compute the layer's bounding box in buffer
        // space and skip strokes that are entirely outside it.
        if !layer.paint_strokes.is_empty() {
            let inv_sx = if scale[0].abs() > f32::EPSILON {
                100.0 / scale[0]
            } else {
                0.0
            };
            let inv_sy = if scale[1].abs() > f32::EPSILON {
                100.0 / scale[1]
            } else {
                0.0
            };
            let to_buf_local = |lp: [f32; 2]| -> [f32; 2] {
                // local -> world (rotate + scale + pos), then into buffer px
                let wx = cx + (lp[0] * cos_r - lp[1] * sin_r) * scale[0] / 100.0;
                let wy = cy + (lp[0] * sin_r + lp[1] * cos_r) * scale[1] / 100.0;
                [wx - min_x as f32, wy - min_y as f32]
            };
            let _ = (inv_sx, inv_sy); // inverse reserved for future pick tools
                                      // DirtyRect: bounding box of all strokes in buffer-pixel space
            let mut dirty_min_x = bw as f32;
            let mut dirty_min_y = bh as f32;
            let mut dirty_max_x = 0.0f32;
            let mut dirty_max_y = 0.0f32;
            for stroke in &layer.paint_strokes {
                let end_f = if stroke.end_frame == 0 {
                    layer.out_frame
                } else {
                    stroke.end_frame
                };
                if effective_frame < stroke.start_frame || effective_frame > end_f {
                    continue;
                }
                let half = stroke.size * 0.5;
                for &p in &stroke.points {
                    let bp = to_buf_local(p);
                    dirty_min_x = dirty_min_x.min(bp[0] - half);
                    dirty_min_y = dirty_min_y.min(bp[1] - half);
                    dirty_max_x = dirty_max_x.max(bp[0] + half);
                    dirty_max_y = dirty_max_y.max(bp[1] + half);
                }
            }
            // Clamp dirty rect to buffer bounds
            let dr_x0 = dirty_min_x.floor().max(0.0) as u32;
            let dr_y0 = dirty_min_y.floor().max(0.0) as u32;
            let dr_x1 = dirty_max_x.ceil().min(bw as f32) as u32;
            let dr_y1 = dirty_max_y.ceil().min(bh as f32) as u32;
            for stroke in &layer.paint_strokes {
                let end_f = if stroke.end_frame == 0 {
                    layer.out_frame
                } else {
                    stroke.end_frame
                };
                if effective_frame < stroke.start_frame || effective_frame > end_f {
                    continue;
                }
                // Quick AABB test: skip strokes entirely outside the dirty rect
                let stroke_min = stroke.points.iter().fold([f32::MAX, f32::MAX], |acc, p| {
                    [acc[0].min(p[0]), acc[1].min(p[1])]
                });
                let stroke_max = stroke.points.iter().fold([f32::MIN, f32::MIN], |acc, p| {
                    [acc[0].max(p[0]), acc[1].max(p[1])]
                });
                let sb_min = to_buf_local(stroke_min);
                let sb_max = to_buf_local(stroke_max);
                let s_min_x = sb_min[0] - stroke.size * 0.5;
                let s_min_y = sb_min[1] - stroke.size * 0.5;
                let s_max_x = sb_max[0] + stroke.size * 0.5;
                let s_max_y = sb_max[1] + stroke.size * 0.5;
                if s_max_x < dr_x0 as f32
                    || s_min_x > dr_x1 as f32
                    || s_max_y < dr_y0 as f32
                    || s_min_y > dr_y1 as f32
                {
                    continue; // stroke entirely outside dirty rect
                }
                let buf_pts: Vec<[f32; 2]> =
                    stroke.points.iter().map(|&p| to_buf_local(p)).collect();
                let col = stroke.color;
                if stroke.mode == crate::core::timeline::PaintStrokeMode::CloneStamp {
                    let source_buf = layer_buf.clone();
                    let source_pts: Vec<[f32; 2]> = stroke
                        .points
                        .iter()
                        .map(|&p| to_buf_local([p[0] + stroke.clone_offset[0], p[1] + stroke.clone_offset[1]]))
                        .collect();
                    crate::core::paint::draw_clone_stroke_with_brush(
                        &mut layer_buf,
                        &source_buf,
                        bw,
                        bh,
                        &buf_pts,
                        &source_pts,
                        stroke.size.max(1.0),
                        stroke.hardness,
                        stroke.opacity,
                        stroke.flow,
                    );
                } else {
                    crate::core::paint::draw_stroke_with_brush(
                        &mut layer_buf,
                        bw,
                        bh,
                        &buf_pts,
                        col,
                        stroke.size.max(1.0),
                        stroke.hardness,
                        stroke.opacity,
                        stroke.flow,
                    );
                }
            }
        }

        // ── Puppet warp: IDW-displace the isolated layer buffer before effects ──
        if !layer.puppet_pins.is_empty() {
            // layer_buf rows/cols are world-aligned samples offset by
            // (min_x, min_y), so comp-space -> buffer-space is a translation.
            let to_buf = |p: [f32; 2]| -> [f32; 2] { [p[0] - min_x as f32, p[1] - min_y as f32] };
            let pins: Vec<([f32; 2], [f32; 2])> = layer
                .puppet_pins
                .iter()
                .map(|pin| {
                    let s = to_buf(pin.comp_source);
                    let d = to_buf(pin.position.evaluate(effective_frame));
                    (s, d)
                })
                .collect();
            crate::core::puppet_warp::warp_layer_buf_mesh(&mut layer_buf, bw, bh, &pins);
        }

        crate::core::cpu_effects::apply_layer_effects(
            Some(precomp_comp),
            Some(layer_idx),
            &mut layer_buf,
            bw,
            bh,
            &layer.effects,
            effective_frame,
            precomp_comp.fps,
        );

        // Composite onto buffer
        for ly in 0..bh {
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                // Composite-time opacity: the layer's opacity scales its
                // contribution here (once, for every blend path), instead of
                // being baked into the raster where effects could clobber it.
                let src_a = layer_buf[lidx + 3] as f32 / 255.0 * l_opacity;
                if src_a <= 0.001 {
                    continue;
                }
                let px = min_x + lx;
                let py = min_y + ly;
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 >= buffer.len() {
                    continue;
                }
                let src_linear = crate::core::color::Rgbaf::from_rgba8(
                    layer_buf[lidx],
                    layer_buf[lidx + 1],
                    layer_buf[lidx + 2],
                    255,
                );
                let src_lin =
                    crate::core::color::Rgbaf::new(src_linear.r, src_linear.g, src_linear.b, src_a);
                let dst_linear = crate::core::color::Rgbaf::from_rgba8(
                    buffer[idx],
                    buffer[idx + 1],
                    buffer[idx + 2],
                    buffer[idx + 3],
                );
                let out = src_lin.over(dst_linear);
                let out_rgba = out.to_rgba8();
                buffer[idx] = out_rgba[0];
                buffer[idx + 1] = out_rgba[1];
                buffer[idx + 2] = out_rgba[2];
                buffer[idx + 3] = out_rgba[3];
            }
        }
    }

    buffer
}
