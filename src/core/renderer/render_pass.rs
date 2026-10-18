use super::*;

impl WgpuRenderer {
    /// Internal core rendering implementation for both primary preview and snapshot target views.
    pub(super) fn render_internal(
        &mut self,
        comp: &Composition,
        frame: u32,
        exposure_ev: f32,
        lut_mode: u32,
        target_snapshot: bool,
    ) -> bool {
        // Dirty-checking: the viewport calls render() at display refresh rate even
        // when nothing changed. Skip the full encode/upload/draw pass when the
        // project version, frame, exposure, LUT, and target size are unchanged.
        // Effective preview resolution: downscale large comps to the viewport cap.
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (cap.max(1), ((comp.height as f32 * s) as u32).max(1))
            }
            _ => (comp.width, comp.height),
        };

        let composition_key = composition_cache_key(comp);
        let render_key: RenderKey = (
            crate::core::frame_cache::current_version(),
            composition_key,
            frame,
            exposure_ev.to_bits(),
            lut_mode,
            (eff_w, eff_h),
        );
        let last_key = if target_snapshot {
            self.last_snapshot_key
        } else {
            self.last_main_key
        };
        let ram_mode = self.ram_render_idx != usize::MAX;
        if !ram_mode && last_key == Some(render_key) {
            return false; // nothing changed — reuse the existing target texture
        }

        // Clamp to both our sanity limit and the device's texture limit —
        // oversized textures would trip wgpu validation and abort the process.
        let max_dim = self
            .device
            .limits()
            .max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);
        let recreated = self.ensure_target_size(width, height);

        // Update Globals Uniform
        let globals = GlobalsUniform {
            viewport_size: [width as f32, height as f32],
            exposure_ev,
            lut_mode,
            shadow_enabled: self.shadow_active as u32,
            shadow_strength: 1.0,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Per-layer text texture bind groups (declared before the render pass so they outlive it)
        let mut layer_textures: Vec<Option<std::sync::Arc<wgpu::BindGroup>>> = Vec::new();

        // Target must exist; per-run views are borrowed fresh inside the Step-3
        // loop so mask texture uploads (&mut self) fit between submits.
        if (target_snapshot && self.snapshot_view.is_none())
            || (!target_snapshot && self.target_view.is_none())
        {
            return false;
        }

        {
            // Viewport projection matrix:
            // Maps [0, width] to [-1, 1] on X, and [0, height] to [1, -1] on Y.
            let m_proj = [
                [2.0 / width as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / height as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0], // Column-major translation: -1 on x, 1 on y
            ];

            // Step 1: Pre-evaluate active layer transform matrices and effect properties
            let mut active_layers = Vec::new();
            let mut comp_indices: Vec<usize> = Vec::new();
            let mut uniforms = Vec::new();
            let mut layer_mask_plans: Vec<Option<std::sync::Arc<MaskRaster>>> = Vec::new();

            for (comp_idx, layer) in comp.layers.iter().enumerate() {
                if !layer.is_active(frame) {
                    continue;
                }
                comp_indices.push(comp_idx);
                let effective_frame = effective_render_frame(layer, frame, comp.fps);

                // Retrieve transform values at the current frame.
                // Parented layers and expression-driven layers resolve through the full
                // composition-aware path (position/scale/rotation/opacity inherit).
                let layer_has_exprs = layer.transform.position_expression.is_some()
                    || layer.transform.rotation_expression.is_some()
                    || layer.transform.scale_expression.is_some()
                    || layer.transform.opacity_expression.is_some();
                let (pos, scale, rotation, opacity) =
                    if layer.parent_id.is_some() || layer_has_exprs {
                        comp.resolve_world_transform(layer, frame)
                    } else {
                        (
                            layer.transform.position.evaluate(frame),
                            layer.transform.scale.evaluate(frame),
                            layer.transform.rotation.evaluate(frame),
                            layer.transform.opacity.evaluate(frame),
                        )
                    };

                // Default layer dimensions (solid size or fallback)
                let (mut layer_w, mut layer_h) = match &layer.layer_type {
                    LayerType::Solid { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::Image { .. } => (1.0, 1.0),
                    LayerType::Model3D { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::Video { .. } => (1.0, 1.0),
                    LayerType::Text { font_size, .. } => (1.0, *font_size as f32 * 10.0), // Overridden below if text texture rasterization succeeds
                    LayerType::Shape { .. } => (1.0, 1.0),
                    LayerType::Null => (0.0, 0.0),
                    LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::AdjustmentLayer => (comp.width as f32, comp.height as f32),
                    LayerType::Audio { .. } => (0.0, 0.0),
                    LayerType::Particle { .. } => (comp.width as f32, comp.height as f32),
                };

                // GPU text rendering: rasterize text to a cached texture; if successful,
                // size the quad to the text bitmap and render via the image sampling path.
                let mut text_bind_group: Option<std::sync::Arc<wgpu::BindGroup>> = None;
                let mut is_textured_text = false;
                // Video layers sample their frame sequence via the image path too,
                // but keep composition-sized geometry (unlike text, which fits the bitmap).
                let mut is_video_frame = false;
                let mut is_precomp_frame = false;
                let mut is_image_frame = false;
                if let LayerType::Image { path } = &layer.layer_type {
                    let image_path = self
                        .preview_proxy_path(layer, 0)
                        .unwrap_or_else(|| path.clone());
                    if let Some((tw, th, bg)) = self.get_or_create_image_texture(
                        &layer.id,
                        &image_path,
                        crate::core::frame_cache::current_version(),
                    ) {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_image_frame = true;
                    }
                }
                if let LayerType::Video {
                    frames_dir,
                    frame_count,
                    speed,
                    ..
                } = &layer.layer_type
                {
                    let source_frame = match &layer.posterize_time {
                        Some(pt) if pt.enabled => effective_frame as f32,
                        _ => layer.remap_frame_f32(frame),
                    };
                    let seq_frame = (source_frame * speed.max(0.0)).floor().max(0.0) as u32;
                    let seq_frame = seq_frame.min(frame_count.saturating_sub(1));
                    let version = crate::core::frame_cache::current_version();
                    let texture =
                        if let Some(proxy_path) = self.preview_proxy_path(layer, seq_frame) {
                            self.get_or_create_image_texture(
                                &format!("proxy:{}", layer.id),
                                &proxy_path,
                                version,
                            )
                        } else {
                            self.get_or_create_video_frame_texture(
                                &layer.id, frames_dir, seq_frame, version,
                            )
                        };
                    if let Some((tw, th, bg)) = texture {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_video_frame = true;
                    }
                }
                if let LayerType::PreComp { comp_id } = &layer.layer_type {
                    if let Some((_, _, bg)) = self.get_or_create_precomp_frame_texture(
                        comp,
                        comp_id,
                        effective_frame,
                        width,
                        height,
                        crate::core::frame_cache::current_version(),
                    ) {
                        text_bind_group = Some(bg);
                        is_precomp_frame = true;
                    }
                }
                if let LayerType::Text {
                    text,
                    font_size,
                    color,
                    font_family,
                    tracking,
                    leading,
                    align,
                    stroke_color,
                    stroke_width,
                    ..
                } = &layer.layer_type
                {
                    let formatting = layer.text_formatting.as_ref();
                    let effective_family = formatting
                        .map(|tf| tf.font_family.clone())
                        .unwrap_or_else(|| font_family.clone());
                    let effective_tracking = formatting.map(|tf| tf.tracking).unwrap_or(*tracking);
                    let effective_leading = formatting.map(|tf| tf.leading).unwrap_or(*leading);
                    let effective_align =
                        formatting.map(|tf| tf.alignment as usize).unwrap_or(*align);
                    let effective_box_width = formatting.map(|tf| tf.box_width).unwrap_or(0.0);
                    let effective_stroke_color = formatting
                        .map(|tf| tf.stroke_color.unwrap_or([0.0, 0.0, 0.0, 0.0]))
                        .unwrap_or(*stroke_color);
                    let effective_stroke_width = formatting
                        .map(|tf| {
                            if tf.stroke_color.is_some() {
                                tf.stroke_width
                            } else {
                                0.0
                            }
                        })
                        .unwrap_or(*stroke_width);
                    let faux_style = formatting
                        .map(|tf| crate::core::font_rasterizer::FauxTextStyle {
                            bold: tf.faux_bold,
                            italic: tf.faux_italic,
                            all_caps: tf.all_caps,
                            small_caps: tf.small_caps,
                        })
                        .unwrap_or_default();
                    let params = TextRasterParams {
                        text: text.clone(),
                        font_size: *font_size,
                        color: *color,
                        font_family: effective_family,
                        tracking: effective_tracking,
                        leading: effective_leading,
                        align: effective_align,
                        box_width: effective_box_width,
                        stroke_color: effective_stroke_color,
                        stroke_width: effective_stroke_width,
                        faux_style,
                    };
                    if let Some((tw, th, bg)) = self.get_or_create_text_texture(&layer.id, &params)
                    {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_textured_text = true;
                    }
                }
                layer_textures.push(text_bind_group);

                let anc = layer.transform.anchor_point.evaluate(frame);

                // Compute layer-to-world transformation matrix
                let m_size = [
                    [layer_w, 0.0, 0.0, 0.0],
                    [0.0, layer_h, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [layer_w * 0.5, layer_h * 0.5, 0.0, 1.0],
                ];

                let m_anc = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-anc[0], -anc[1], 0.0, 1.0],
                ];

                let m_scale = [
                    [scale[0] / 100.0, 0.0, 0.0, 0.0],
                    [0.0, scale[1] / 100.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let rad = rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let m_rot = [
                    [cos_r, sin_r, 0.0, 0.0],
                    [-sin_r, cos_r, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let m_pos = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [pos[0], pos[1], 0.0, 1.0],
                ];

                let m_model = mat4_mul(
                    m_pos,
                    mat4_mul(m_rot, mat4_mul(m_scale, mat4_mul(m_anc, m_size))),
                );

                // Total projection * model matrix
                let transform_matrix = if layer.is_3d {
                    comp.resolve_world_transform_3d(layer, frame)
                } else {
                    mat4_mul(m_proj, m_model)
                };

                let layer_uniform = super::uniform_builder::build_layer_uniform(
                    comp,
                    layer,
                    frame,
                    effective_frame,
                    transform_matrix,
                    opacity,
                    is_textured_text || is_video_frame || is_precomp_frame || is_image_frame,
                );

                layer_mask_plans.push(
                    collect_mask_shapes(layer, frame, width, height, comp.width, comp.height).map(
                        |(key, shapes)| {
                            if let Some(hit) = self.mask_raster_cache.borrow_mut().get(key) {
                                return hit;
                            }
                            // Drop the short-lived borrow before the (possibly slow)
                            // raster so we never hold the cache across heavy work.
                            let raster = std::sync::Arc::new(
                                rasterize_from_shapes(&shapes, width, height)
                                    .expect("non-empty shapes always yield coverage"),
                            );
                            self.mask_raster_cache.borrow_mut().insert(raster.clone());
                            raster
                        },
                    ),
                );
                uniforms.push(layer_uniform);
                active_layers.push(layer);
            }

            // Expand Shape Repeaters into ordinary GPU draws. Keeping each copy
            // in the existing layer pipeline preserves masks, blend modes and
            // effects without introducing a second, subtly different shader path.
            if active_layers
                .iter()
                .any(|layer| layer.shape_repeater.is_some())
            {
                let old_layers = std::mem::take(&mut active_layers);
                let old_comp_indices = std::mem::take(&mut comp_indices);
                let old_uniforms = std::mem::take(&mut uniforms);
                let old_masks = std::mem::take(&mut layer_mask_plans);
                let old_textures = std::mem::take(&mut layer_textures);
                for ((((layer, comp_idx), uniform), mask), texture) in old_layers
                    .into_iter()
                    .zip(old_comp_indices)
                    .zip(old_uniforms)
                    .zip(old_masks)
                    .zip(old_textures)
                {
                    let Some(repeater) = &layer.shape_repeater else {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask);
                        layer_textures.push(texture);
                        continue;
                    };
                    for instance in crate::core::shape_repeater::evaluate_shape_repeater_at_frame(
                        repeater,
                        effective_render_frame(layer, frame, comp.fps),
                    ) {
                        let matrix = instance.transform_matrix;
                        let copy_transform = [
                            [matrix[0][0], matrix[0][1], 0.0, 0.0],
                            [matrix[1][0], matrix[1][1], 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [matrix[0][2], matrix[1][2], 0.0, 1.0],
                        ];
                        let mut copy_uniform = uniform;
                        copy_uniform.transform_matrix =
                            mat4_mul(uniform.transform_matrix, copy_transform);
                        copy_uniform.opacity *= instance.opacity;
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(copy_uniform);
                        layer_mask_plans.push(mask.clone());
                        layer_textures.push(texture.clone());
                    }
                }
            }

            // Keep GPU painter order consistent with the software compositor.
            // A 3D composition must be sorted by the evaluated layer depth;
            // iterating the project array directly makes remapped layers draw
            // in the wrong front-to-back order.
            let order = gpu_layer_draw_order(comp, &active_layers, frame);
            if order
                .iter()
                .enumerate()
                .any(|(index, &value)| index != value)
            {
                let old_active_layers = active_layers;
                let old_comp_indices = comp_indices;
                let old_uniforms = uniforms;
                let old_layer_mask_plans = layer_mask_plans;
                let old_layer_textures = layer_textures;
                active_layers = order
                    .iter()
                    .map(|&index| old_active_layers[index])
                    .collect();
                comp_indices = order.iter().map(|&index| old_comp_indices[index]).collect();
                uniforms = order.iter().map(|&index| old_uniforms[index]).collect();
                layer_mask_plans = order
                    .iter()
                    .map(|&index| old_layer_mask_plans[index].clone())
                    .collect();
                layer_textures = order
                    .iter()
                    .map(|&index| old_layer_textures[index].clone())
                    .collect();
            }

            // Step 1b: Prepare track matte textures before the batched upload pass.
            let matte_resources = self.prepare_track_mattes(
                comp,
                frame,
                width,
                height,
                &active_layers,
                &comp_indices,
            );

            if !self.upload_and_encode_render_runs(super::render_encoder::EncodeRunsCtx {
                comp,
                width,
                height,
                target_snapshot,
                uniforms: &mut uniforms,
                active_layers: &active_layers,
                layer_mask_plans: &layer_mask_plans,
                layer_textures: &layer_textures,
                matte_resources: &matte_resources,
            }) {
                return false;
            }
        }

        // Remember inputs so redundant renders can be skipped.
        // RAM pre-pass frames never touch the live-view keys.
        if !ram_mode {
            if target_snapshot {
                self.last_snapshot_key = Some(render_key);
            } else {
                self.last_main_key = Some(render_key);
            }
        }
        recreated
    }
}
