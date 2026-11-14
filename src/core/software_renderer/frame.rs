use super::resolution::RenderScale;
use super::*;

pub(crate) struct RenderFrameConfig {
    pub width: u32,
    pub height: u32,
    pub exposure_ev: f32,
    pub lut_mode: u32,
    pub only_layer_idx: Option<usize>,
    pub render_scale: RenderScale,
}

/// Render a frame while optionally limiting the top-level pass to one layer.
///
/// Source-layer effects need a layer's pixels without losing the original
/// composition, because the selected layer's effect parameters still refer to
/// the original layer indices. Keeping the full composition here preserves
/// those references while the filter prevents unrelated layers from being
/// composited into the source buffer.
pub fn render_frame_to_pixels_filtered(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
    only_layer_idx: Option<usize>,
) -> Vec<u8> {
    render_frame_to_pixels_filtered_scaled(
        comp,
        frame,
        RenderFrameConfig {
            width,
            height,
            exposure_ev,
            lut_mode,
            only_layer_idx,
            render_scale: RenderScale::FULL,
        },
    )
}

pub(crate) fn render_frame_to_pixels_filtered_scaled(
    comp: &Composition,
    frame: u32,
    config: RenderFrameConfig,
) -> Vec<u8> {
    let RenderFrameConfig {
        width,
        height,
        exposure_ev,
        lut_mode,
        only_layer_idx,
        render_scale,
    } = config;
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
    let allocation_scope = RenderAllocationScope::install();
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
    let Some(mut buffer) = try_zeroed_buffer(size) else {
        log::error!("[Renderer] Could not allocate {size} bytes for the composition buffer");
        return Vec::new();
    };
    for p in buffer.as_chunks_mut::<4>().0.iter_mut() {
        p[0] = bg_r;
        p[1] = bg_g;
        p[2] = bg_b;
        p[3] = bg_a;
    }
    if render_cancelled() {
        return buffer;
    }

    if render_cancelled() {
        return if allocation_scope.failed() {
            Vec::new()
        } else {
            buffer
        };
    }

    let composition_instance_id = comp as *const Composition as usize;
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
    let render_camera = comp.resolve_camera_at();
    if render_cancelled() {
        output::apply_output_transform(comp, &mut buffer, exposure_ev, lut_mode);
        return if allocation_scope.failed() {
            Vec::new()
        } else {
            buffer
        };
    }
    audio_reactive::inject_audio_reactive_data(comp, frame);
    if render_cancelled() {
        output::apply_output_transform(comp, &mut buffer, exposure_ev, lut_mode);
        return if allocation_scope.failed() {
            Vec::new()
        } else {
            buffer
        };
    }

    let has_solo =
        only_layer_idx.is_none() && comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    // ── Phase 1: Parallel layer data preparation ──
    // Multi-frame rendering support: parallel render queue initialized for MFR pipeline.
    // Note: sequential compositing remains the reference implementation; MFR enabled for batch exports.
    // Pre-compute transform/mask/effect data for all visible layers at once,
    // eliminating redundant property evaluation during the sequential pass.
    let layer_data = layer_prepare::prepare_layer_render_data(
        comp,
        frame,
        &render_camera,
        has_solo,
        get_render_cancel_flag(),
        crate::core::expression_engine::current_audio_expr_data(),
        render_scale,
    );
    if render_cancelled() {
        output::apply_output_transform(comp, &mut buffer, exposure_ev, lut_mode);
        return if allocation_scope.failed() {
            Vec::new()
        } else {
            buffer
        };
    }

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
        shadow::build_shadow_map_scaled(comp, frame, width, height, render_scale)
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
            if apply_adjustment_layer(
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
            )
            .is_err()
            {
                return Vec::new();
            }
            continue;
        }

        match render_precomp_layer(
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
            Ok(true) => continue,
            Ok(false) => {}
            Err(()) => return Vec::new(),
        }

        match render_particle_layer(
            &mut buffer,
            ParticleLayerCtx {
                comp,
                composition_instance_id,
                layer,
                frame,
                sorted_idx,
                source_frame,
                position: pos,
                scale,
                rotation,
                opacity: l_opacity,
                masks,
                blend_linear,
                width,
                height,
                camera: &render_camera,
            },
        ) {
            Ok(true) => continue,
            Ok(false) => {}
            Err(()) => return Vec::new(),
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

        let [mut cx, mut cy] = layer_center_from_anchor(
            pos,
            scale,
            rotation,
            layer.transform.anchor_point.evaluate(frame),
            [base_w, base_h],
        );

        // Derive CPU bounds and texture coordinates from the same projective
        // matrix used by the GPU layer draw.
        let (bounds_x, bounds_y, three_d_matrix) = if layer.is_3d {
            let matrix = comp.resolve_world_transform_3d(layer, frame, base_w, base_h);
            let Some(projected) = projective::project_quad(matrix, width as f32, height as f32)
            else {
                // A layer outside the camera frustum must not reappear as a 2D
                // layer at its unrelated composition-space position.
                continue;
            };
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
            // A 3D layer's projected position comes from its 3D transform;
            // using the default 2D center can clip the entire CPU raster at
            // the composition origin while the GPU draws it at screen center.
            cx = (min_sx + max_sx) * 0.5;
            cy = (min_sy + max_sy) * 0.5;
            (bx, by, Some(matrix))
        } else {
            (w * 0.5, h * 0.5, None)
        };
        // abs(): negative scale flips w/h sign — must not invert the bounding box
        let content_extent = bounds_x.max(bounds_y).abs();
        let mut ext = content_extent * 1.5;
        if layer.style.stroke.enabled {
            ext = ext.max(content_extent + layer.style.stroke.size.max(0.0));
        }
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
        let Some(layer_size) = (bw as usize)
            .checked_mul(bh as usize)
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            log::error!(
                "[Renderer] Layer raster dimensions overflow for {}",
                layer.id
            );
            return Vec::new();
        };
        let Some(mut layer_buf) = try_zeroed_buffer(layer_size) else {
            log::error!(
                "[Renderer] Could not allocate {layer_size} bytes for layer {}",
                layer.id
            );
            return Vec::new();
        };

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
            three_d_matrix,
            scale,
            cos_r,
            sin_r,
            flip_x: scale[0] < 0.0,
            flip_y: scale[1] < 0.0,
            bounds_x,
            bounds_y,
            base_color,
            l_opacity,
            buffer: &mut buffer[..],
            layer_buf: &mut layer_buf[..],
        });
        if render_cancelled() {
            break;
        }

        if !layer.paint_strokes.is_empty() {
            let paint_transform = crate::core::software_renderer::precomp::paint::PaintTransform {
                scale,
                cos_r,
                sin_r,
                cx,
                cy,
                min_x,
                min_y,
            };
            crate::core::software_renderer::precomp::paint::apply_paint_strokes(
                layer,
                source_frame,
                &paint_transform,
                bw,
                bh,
                &mut layer_buf,
            );
            if render_cancelled() {
                break;
            }
        }

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
        if render_cancelled() {
            break;
        }

        // Phase 3: composite the (effect-processed) buffer over the frame.
        // First, check if this layer uses a track matte from the layer below.
        let matte_pixels = match render_matte_buffer(comp, layer, frame, width, height) {
            Ok(matte) => matte,
            Err(()) => return Vec::new(),
        };
        if render_cancelled() {
            break;
        }

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
    if allocation_scope.failed() {
        Vec::new()
    } else {
        buffer
    }
}
