use super::*;

impl WgpuRenderer {
    fn ensure_custom_shader_pipeline(&self, hash: u64, wgsl_source: &str) {
        if self.custom_shader_pipelines.borrow().contains_key(&hash) {
            return;
        }
        let full_wgsl = if wgsl_source.contains("@vertex") {
            wgsl_source.to_string()
        } else {
            format!(
                "struct CustomVsOut {{ @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, }};\n\
                 @vertex fn vs_custom(@builtin(vertex_index) vid: u32) -> CustomVsOut {{\n\
                     var out: CustomVsOut;\n\
                     let x = f32((vid << 1u) & 2u);\n\
                     let y = f32(vid & 2u);\n\
                     out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);\n\
                     out.uv = vec2<f32>(x, y);\n\
                     return out;\n\
                 }}\n{wgsl_source}"
            )
        };
        let vs_entry = if wgsl_source.contains("@vertex") {
            "vs_main"
        } else {
            "vs_custom"
        };
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom WGSL Effect Shader"),
            source: wgpu::ShaderSource::Wgsl(full_wgsl.into()),
        });
        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom WGSL Effect Pipeline"),
            layout: Some(&self.custom_shader_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: vs_entry,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let err = pollster::block_on(self.device.pop_error_scope());
        self.custom_shader_pipelines
            .borrow_mut()
            .insert(hash, if err.is_none() { Some(pipeline) } else { None });
    }

    fn clear_target_after_mask_failure(&self, target_snapshot: bool, background: [f32; 4]) {
        let view = if target_snapshot {
            self.snapshot_view.as_ref()
        } else {
            self.target_view.as_ref()
        };
        let Some(view) = view else {
            return;
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Clear target after mask raster failure"),
            });
        let background_alpha = background[3].clamp(0.0, 1.0) as f64;
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear target after mask raster failure"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0].clamp(0.0, 1.0) as f64 * background_alpha,
                            g: background[1].clamp(0.0, 1.0) as f64 * background_alpha,
                            b: background[2].clamp(0.0, 1.0) as f64 * background_alpha,
                            a: background_alpha,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(Some(encoder.finish()));
    }

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
        let media_revision = composition_media_revision(comp, frame);
        let render_key: RenderKey = (
            crate::core::frame_cache::current_version(),
            composition_key,
            media_revision,
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
            channel_mode: 0,
            wipe_split_x: 0.0,
            dither_output: comp.dither_output as u32,
            blend_linear: comp.blend_linear as u32,
            working_color_space: comp.working_color_space as u32,
            _pad2: 0,
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

        // Expression and Audio Spectrum values are thread-local. Install the
        // same frame context used by the software renderer before evaluating
        // any GPU-layer transform or effect parameters.
        crate::core::software_renderer::inject_audio_reactive_data(comp, frame);

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
            #[derive(Clone)]
            struct LayerMultipassPlan {
                steps: Vec<(
                    LayerUniform,
                    Option<usize>,
                    bool,
                    Option<(u64, [f32; 4])>,
                    Option<(u64, f32)>,
                )>,
                final_src_scratch: usize,
            }
            let mut active_layers = Vec::new();
            let mut comp_indices: Vec<usize> = Vec::new();
            let mut uniforms = Vec::new();
            let mut layer_mask_plans: Vec<Option<std::sync::Arc<MaskRaster>>> = Vec::new();
            let mut multipass_plans: Vec<Option<LayerMultipassPlan>> = Vec::new();

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
                let mut is_model3d_frame = false;
                let mut is_puppet_mesh = false;
                let mut is_raster_layer = false;
                let mut video_frame_blend = 0.0;
                if crate::core::render_capability::layer_needs_raster_fallback(comp, layer, frame) {
                    if let Some((_, _, bg)) = self.get_or_create_raster_layer_texture(
                        comp,
                        comp_idx,
                        frame,
                        width,
                        height,
                        crate::core::frame_cache::current_version(),
                    ) {
                        layer_w = comp.width as f32;
                        layer_h = comp.height as f32;
                        text_bind_group = Some(bg);
                        is_raster_layer = true;
                    }
                }
                if !is_raster_layer {
                    if matches!(layer.layer_type, LayerType::Model3D { .. }) {
                        if let Some((mw, mh, bg)) = self.render_model3d_to_gpu_texture(
                            comp,
                            layer,
                            frame,
                            comp.width,
                            comp.height,
                        ) {
                            layer_w = mw as f32;
                            layer_h = mh as f32;
                            text_bind_group = Some(bg);
                            is_model3d_frame = true;
                        }
                    }
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
                    let source_position = crate::core::timeline::video_sequence_frame(
                        source_frame,
                        *speed,
                        frames_dir,
                        comp.fps,
                    );
                    let source_floor = source_position.floor();
                    let seq_frame = (source_floor as u32).min(frame_count.saturating_sub(1));
                    let blend = if layer.frame_blending {
                        (source_position - source_floor).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let next_frame = (blend > f32::EPSILON
                        && seq_frame < frame_count.saturating_sub(1)
                        && self.preview_proxy_path(layer, seq_frame).is_none())
                    .then_some(seq_frame + 1);
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
                                &layer.id, frames_dir, seq_frame, next_frame, version,
                            )
                        };
                    if let Some((tw, th, bg)) = texture {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_video_frame = true;
                        video_frame_blend = if next_frame.is_some() { blend } else { 0.0 };
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
                    let effective_box_height = formatting.map(|tf| tf.box_height).unwrap_or(0.0);
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
                    let resolved_anim = layer.text_animator.as_ref().filter(|a| a.enabled).map(|anim| {
                        if let Some(ref oa) = anim.selector.offset_anim {
                            let mut a2 = anim.clone();
                            a2.selector.offset = oa.evaluate(frame);
                            a2
                        } else {
                            anim.clone()
                        }
                    });
                    let resolved_stack = layer
                        .text_animator_stack
                        .as_ref()
                        .filter(|s| !s.animators.is_empty())
                        .cloned();
                    let animator_hash = if resolved_stack.is_some() || resolved_anim.is_some() {
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        frame.hash(&mut hasher);
                        if let Some(ref st) = resolved_stack {
                            if let Ok(js) = serde_json::to_string(st) {
                                js.hash(&mut hasher);
                            }
                        }
                        if let Some(ref an) = resolved_anim {
                            if let Ok(js) = serde_json::to_string(an) {
                                js.hash(&mut hasher);
                            }
                        }
                        hasher.finish()
                    } else {
                        0
                    };
                    let params = TextRasterParams {
                        text: text.clone(),
                        font_size: *font_size,
                        color: *color,
                        font_family: effective_family,
                        tracking: effective_tracking,
                        leading: effective_leading,
                        align: effective_align,
                        box_width: effective_box_width,
                        box_height: effective_box_height,
                        stroke_color: effective_stroke_color,
                        stroke_width: effective_stroke_width,
                        faux_style,
                        animator_hash,
                        time_sec: frame as f32 / comp.fps.max(1) as f32,
                        text_animator: resolved_anim,
                        text_animator_stack: resolved_stack,
                    };
                    if let Some((tw, th, bg)) = self.get_or_create_text_texture(&layer.id, &params)
                    {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_textured_text = true;
                    }
                }
                if let LayerType::Shape {
                    shape_type:
                        crate::core::timeline::ShapeType::FreeformBezier {
                            points,
                            tangents,
                            closed,
                        },
                    color,
                    stroke_color,
                    stroke_width,
                    ..
                } = &layer.layer_type
                {
                    if let Some((tw, th, bg)) = self.get_or_create_freeform_bezier_texture(
                        &layer.id,
                        points,
                        tangents,
                        *closed,
                        *color,
                        *stroke_color,
                        *stroke_width,
                        crate::core::frame_cache::current_version(),
                    ) {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_textured_text = true;
                    }
                }
                }
                if !is_raster_layer && layer.puppet_pins.len() > 4 {
                    let fallback_col = match &layer.layer_type {
                        LayerType::Solid { color } => *color,
                        LayerType::Shape { color, .. } => *color,
                        _ => [1.0, 1.0, 1.0, 1.0],
                    };
                    if let Some((pw, ph, bg)) = self.render_puppet_mesh_to_gpu_texture(
                        comp,
                        layer,
                        effective_frame,
                        comp.width,
                        comp.height,
                        text_bind_group.as_deref(),
                        fallback_col,
                    ) {
                        layer_w = pw as f32;
                        layer_h = ph as f32;
                        text_bind_group = Some(bg);
                        is_puppet_mesh = true;
                    }
                }
                layer_textures.push(text_bind_group);

                let (pos, scale, rotation, anc) = if is_raster_layer {
                    (
                        [comp.width as f32 * 0.5, comp.height as f32 * 0.5],
                        [100.0, 100.0],
                        0.0,
                        [comp.width as f32 * 0.5, comp.height as f32 * 0.5],
                    )
                } else {
                    (
                        pos,
                        scale,
                        rotation,
                        layer.transform.anchor_point.evaluate(frame),
                    )
                };

                // Compute layer-to-world transformation matrix
                let m_size = [
                    [layer_w, 0.0, 0.0, 0.0],
                    [0.0, layer_h, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    // Convert the centered unit quad to layer-local pixel
                    // coordinates; the anchor matrix then places its pivot.
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
                let transform_matrix = if layer.is_3d && !is_raster_layer {
                    comp.resolve_world_transform_3d(layer, frame, layer_w, layer_h)
                } else {
                    mat4_mul(m_proj, m_model)
                };

                let textured_source = is_textured_text
                    || is_video_frame
                    || is_precomp_frame
                    || is_image_frame
                    || is_model3d_frame
                    || is_puppet_mesh
                    || is_raster_layer;
                let needs_multipass_fx =
                    !is_raster_layer && crate::core::render_capability::layer_needs_gpu_multipass_effects(layer);
                let gpu_motion_blur = if !is_raster_layer {
                    crate::core::render_capability::layer_gpu_motion_blur_params(comp, layer, frame)
                } else {
                    None
                };
                let has_gpu_layer_styles = !is_raster_layer
                    && (layer.style.drop_shadow.enabled
                        || layer.style.inner_shadow.enabled
                        || layer.style.outer_glow.enabled
                        || layer.style.inner_glow.enabled
                        || layer.style.satin.enabled
                        || layer.style.stroke.enabled);
                let (layer_uniform, multipass_plan) =
                    if needs_multipass_fx || gpu_motion_blur.is_some() || has_gpu_layer_styles
                {
                    let fullscreen_model = [
                        [comp.width as f32, 0.0, 0.0, 0.0],
                        [0.0, comp.height as f32, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [comp.width as f32 * 0.5, comp.height as f32 * 0.5, 0.0, 1.0],
                    ];
                    let fullscreen_matrix = mat4_mul(m_proj, fullscreen_model);
                    let mut base_uniform = super::uniform_builder::build_layer_uniform(
                        comp,
                        layer,
                        frame,
                        effective_frame,
                        transform_matrix,
                        100.0,
                        textured_source,
                        video_frame_blend,
                        needs_multipass_fx,
                    );
                    base_uniform.blend_mode = 255;
                    base_uniform.track_matte_mode = 0;

                    let mut steps = Vec::new();
                    steps.push((base_uniform, None, true, None, None));

                    let mut pass_count = 0usize;
                    if needs_multipass_fx {
                        for effect in layer.effects.iter().filter(|e| e.enabled) {
                            if let crate::core::timeline::EffectType::GaussianBlur { blur_radius } =
                                &effect.effect_type
                            {
                                let r = blur_radius.evaluate(frame).max(0.0);
                                if r > 0.05 {
                                    let mut fx_layer = layer.clone();
                                    fx_layer.layer_type = LayerType::Image {
                                        path: String::new(),
                                    };
                                    fx_layer.is_3d = false;
                                    fx_layer.effects.clear();
                                    fx_layer.effects_enabled = false;
                                    fx_layer.track_matte = TrackMatteMode::None;
                                    let mut h_u = super::uniform_builder::build_layer_uniform(
                                        comp,
                                        &fx_layer,
                                        frame,
                                        effective_frame,
                                        fullscreen_matrix,
                                        100.0,
                                        true,
                                        0.0,
                                        true,
                                    );
                                    h_u.blend_mode = 255;
                                    h_u.sim_enabled = 3;
                                    h_u.sim_type = 86;
                                    h_u.sim_p1 = r;
                                    let src_a = pass_count % 2 == 0;
                                    steps.push((h_u, Some(if src_a { 0 } else { 1 }), !src_a, None, None));
                                    pass_count += 1;

                                    let mut v_u = h_u;
                                    v_u.sim_type = 87;
                                    let src_a2 = pass_count % 2 == 0;
                                    steps.push((v_u, Some(if src_a2 { 0 } else { 1 }), !src_a2, None, None));
                                    pass_count += 1;
                                    continue;
                                }
                            }
                            let mut fx_layer = layer.clone();
                            fx_layer.layer_type = LayerType::Image {
                                path: String::new(),
                            };
                            fx_layer.is_3d = false;
                            fx_layer.effects = vec![effect.clone()];
                            fx_layer.track_matte = TrackMatteMode::None;
                            let mut fx_u = super::uniform_builder::build_layer_uniform(
                                comp,
                                &fx_layer,
                                frame,
                                effective_frame,
                                fullscreen_matrix,
                                100.0,
                                true,
                                0.0,
                                false,
                            );
                            fx_u.blend_mode = 255;
                            let custom_step = if let crate::core::timeline::EffectType::CustomShader {
                                wgsl_source,
                                uniform_values,
                            } = &effect.effect_type
                            {
                                use std::hash::{Hash, Hasher};
                                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                                wgsl_source.hash(&mut hasher);
                                let hash = hasher.finish();
                                self.ensure_custom_shader_pipeline(hash, wgsl_source);
                                let params = [
                                    uniform_values.first().copied().unwrap_or(0.0),
                                    uniform_values.get(1).copied().unwrap_or(0.0),
                                    uniform_values.get(2).copied().unwrap_or(0.0),
                                    uniform_values.get(3).copied().unwrap_or(0.0),
                                ];
                                Some((hash, params))
                            } else {
                                None
                            };
                            let lut_step = if let crate::core::timeline::EffectType::ColorGradeLUT {
                                lut_path,
                                intensity,
                            } = &effect.effect_type
                            {
                                self.ensure_cube_lut_texture(lut_path)
                                    .map(|key| (key, (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0)))
                            } else {
                                None
                            };
                            let src_is_a = pass_count % 2 == 0;
                            steps.push((fx_u, Some(if src_is_a { 0 } else { 1 }), !src_is_a, custom_step, lut_step));
                            pass_count += 1;
                        }
                    }
                    if let Some((mb_vel, mb_samples, mb_phase)) = gpu_motion_blur {
                        let mut mb_layer = layer.clone();
                        mb_layer.layer_type = LayerType::Image {
                            path: String::new(),
                        };
                        mb_layer.is_3d = false;
                        mb_layer.effects.clear();
                        mb_layer.effects_enabled = false;
                        mb_layer.track_matte = TrackMatteMode::None;
                        let mut mb_u = super::uniform_builder::build_layer_uniform(
                            comp,
                            &mb_layer,
                            frame,
                            effective_frame,
                            fullscreen_matrix,
                            100.0,
                            true,
                            0.0,
                            true,
                        );
                        mb_u.blend_mode = 255;
                        mb_u.motionblur_enabled = 1;
                        mb_u.motionblur_velocity_x = mb_vel[0];
                        mb_u.motionblur_velocity_y = mb_vel[1];
                        mb_u.motionblur_samples = mb_samples;
                        mb_u.motionblur_shutter = mb_phase;
                        let src_is_a = pass_count % 2 == 0;
                        steps.push((mb_u, Some(if src_is_a { 0 } else { 1 }), !src_is_a, None, None));
                        pass_count += 1;
                    }
                    if has_gpu_layer_styles {
                        let mut push_style =
                            |sim_type: u32, dx: f32, dy: f32, size: f32, color: [f32; 4], opacity_pct: f32| {
                                let mut st_layer = layer.clone();
                                st_layer.layer_type = LayerType::Image {
                                    path: String::new(),
                                };
                                st_layer.is_3d = false;
                                st_layer.effects.clear();
                                st_layer.effects_enabled = false;
                                st_layer.track_matte = TrackMatteMode::None;
                                let mut st_u = super::uniform_builder::build_layer_uniform(
                                    comp,
                                    &st_layer,
                                    frame,
                                    effective_frame,
                                    fullscreen_matrix,
                                    100.0,
                                    true,
                                    0.0,
                                    true,
                                );
                                st_u.blend_mode = 255;
                                st_u.sim_enabled = 2;
                                st_u.sim_type = sim_type;
                                st_u.sim_p1 = dx;
                                st_u.sim_p2 = dy;
                                st_u.sim_p3 = size.max(1.0);
                                st_u.effect_shadow_color = color;
                                st_u.effect_shadow_opacity =
                                    color[3] * (opacity_pct / 100.0).clamp(0.0, 1.0);
                                let src_is_a = pass_count % 2 == 0;
                                steps.push((st_u, Some(if src_is_a { 0 } else { 1 }), !src_is_a, None, None));
                                pass_count += 1;
                            };
                        if layer.style.drop_shadow.enabled {
                            let ds = &layer.style.drop_shadow;
                            let rad = ds.angle.to_radians();
                            push_style(0, rad.sin() * ds.distance, -rad.cos() * ds.distance, ds.size, ds.color, ds.opacity);
                        }
                        if layer.style.outer_glow.enabled {
                            let og = &layer.style.outer_glow;
                            push_style(1, 0.0, 0.0, og.size, og.color, og.opacity);
                        }
                        if layer.style.inner_glow.enabled {
                            let ig = &layer.style.inner_glow;
                            push_style(2, 0.0, 0.0, ig.size, ig.color, ig.opacity);
                        }
                        if layer.style.inner_shadow.enabled {
                            let is = &layer.style.inner_shadow;
                            let rad = is.angle.to_radians();
                            push_style(3, rad.sin() * is.distance, -rad.cos() * is.distance, is.size, is.color, is.opacity);
                        }
                        if layer.style.stroke.enabled {
                            let sk = &layer.style.stroke;
                            push_style(4, 0.0, 0.0, sk.size, sk.color, 100.0);
                        }
                        if layer.style.satin.enabled {
                            let sa = &layer.style.satin;
                            let rad = sa.angle.to_radians();
                            push_style(5, rad.sin() * sa.distance, -rad.cos() * sa.distance, sa.size, sa.color, sa.opacity);
                        }
                    }
                    let final_src_scratch = if pass_count % 2 == 0 { 0 } else { 1 };
                    let final_uniform = super::uniform_builder::build_layer_uniform(
                        comp,
                        layer,
                        frame,
                        effective_frame,
                        fullscreen_matrix,
                        opacity,
                        true,
                        0.0,
                        true,
                    );
                    (
                        final_uniform,
                        Some(LayerMultipassPlan {
                            steps,
                            final_src_scratch,
                        }),
                    )
                } else {
                    (
                        super::uniform_builder::build_layer_uniform(
                            comp,
                            layer,
                            frame,
                            effective_frame,
                            transform_matrix,
                            opacity,
                            textured_source,
                            video_frame_blend,
                            is_raster_layer,
                        ),
                        None,
                    )
                };

                let mask_plan = if let Some((key, shapes)) =
                    collect_mask_shapes(layer, frame, comp.fps, width, height, comp.width, comp.height)
                {
                    let cached = self.mask_raster_cache.borrow_mut().get(key);
                    if let Some(hit) = cached {
                        Some(hit)
                    } else {
                        // Drop the short-lived borrow before the (possibly slow)
                        // raster so we never hold the cache across heavy work.
                        let Some(raster) = rasterize_from_shapes(&shapes, width, height) else {
                            log::error!(
                                "[WgpuRenderer] Could not rasterize layer mask within the allocation budget"
                            );
                            self.clear_target_after_mask_failure(
                                target_snapshot,
                                comp.background_color,
                            );
                            if target_snapshot {
                                self.last_snapshot_key = Some(render_key);
                            } else {
                                self.last_main_key = Some(render_key);
                            }
                            return true;
                        };
                        let raster = std::sync::Arc::new(raster);
                        self.mask_raster_cache.borrow_mut().insert(raster.clone());
                        Some(raster)
                    }
                } else {
                    None
                };
                layer_mask_plans.push(mask_plan);
                uniforms.push(layer_uniform);
                multipass_plans.push(multipass_plan);
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
                let old_multipass = std::mem::take(&mut multipass_plans);
                for (((((layer, comp_idx), uniform), mask), texture), mp) in old_layers
                    .into_iter()
                    .zip(old_comp_indices)
                    .zip(old_uniforms)
                    .zip(old_masks)
                    .zip(old_textures)
                    .zip(old_multipass)
                {
                    let Some(repeater) = &layer.shape_repeater else {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask);
                        layer_textures.push(texture);
                        multipass_plans.push(mp);
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
                        multipass_plans.push(mp.clone());
                    }
                }
            }

            if active_layers.iter().any(|layer| {
                layer.effects_enabled
                    && layer.effects.iter().any(|e| {
                        e.enabled
                            && matches!(e.effect_type, crate::core::timeline::EffectType::Echo { .. })
                    })
            }) {
                let old_layers = std::mem::take(&mut active_layers);
                let old_comp_indices = std::mem::take(&mut comp_indices);
                let old_uniforms = std::mem::take(&mut uniforms);
                let old_masks = std::mem::take(&mut layer_mask_plans);
                let old_textures = std::mem::take(&mut layer_textures);
                let old_multipass = std::mem::take(&mut multipass_plans);
                for (((((layer, comp_idx), uniform), mask), texture), mp) in old_layers
                    .into_iter()
                    .zip(old_comp_indices)
                    .zip(old_uniforms)
                    .zip(old_masks)
                    .zip(old_textures)
                    .zip(old_multipass)
                {
                    let echo_eff = if layer.effects_enabled {
                        layer.effects.iter().rev().find_map(|e| {
                            if !e.enabled {
                                return None;
                            }
                            if let crate::core::timeline::EffectType::Echo {
                                echo_time_seconds,
                                num_echoes,
                                starting_intensity,
                                decay,
                                operator,
                            } = &e.effect_type
                            {
                                Some((echo_time_seconds, *num_echoes, starting_intensity, decay, *operator))
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };
                    let Some((echo_time_seconds, num_echoes, starting_intensity, decay, operator)) =
                        echo_eff
                    else {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask);
                        layer_textures.push(texture);
                        multipass_plans.push(mp);
                        continue;
                    };

                    let start_w = starting_intensity.evaluate(frame);
                    let dec = decay.evaluate(frame);
                    let echo_dt = echo_time_seconds.evaluate(frame).abs();
                    let n = num_echoes.clamp(1, 16);
                    let fps_val = comp.fps.max(1) as f32;
                    let (p0, _, _, _) = comp.resolve_world_transform(layer, frame);

                    let mut echo_uniforms = Vec::new();
                    for i in 1..=n {
                        let offset_frames = (echo_dt * i as f32 * fps_val).round() as i32;
                        let echo_frame = (frame as i32 - offset_frames).max(0) as u32;
                        let weight = (start_w * dec.powi(i as i32)).clamp(0.0, 2.0);
                        if weight < 0.001 {
                            continue;
                        }
                        let (pe, _, _, _) = comp.resolve_world_transform(layer, echo_frame);
                        let dx_ndc = (pe[0] - p0[0]) * 2.0 / comp.width.max(1) as f32;
                        let dy_ndc = -(pe[1] - p0[1]) * 2.0 / comp.height.max(1) as f32;
                        let mut echo_u = uniform;
                        echo_u.transform_matrix[3][0] += dx_ndc;
                        echo_u.transform_matrix[3][1] += dy_ndc;
                        echo_u.opacity = (echo_u.opacity * weight).clamp(0.0, 100.0);
                        echo_u.blend_mode = match operator {
                            crate::core::echo_effect::EchoOperator::Add => 4,
                            crate::core::echo_effect::EchoOperator::Screen => 2,
                            crate::core::echo_effect::EchoOperator::Maximum => 6,
                            crate::core::echo_effect::EchoOperator::Minimum => 5,
                            _ => echo_u.blend_mode,
                        };
                        echo_uniforms.push(echo_u);
                    }

                    let in_back = matches!(
                        operator,
                        crate::core::echo_effect::EchoOperator::CompositeInBack
                    );
                    if !in_back {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask.clone());
                        layer_textures.push(texture.clone());
                        multipass_plans.push(mp.clone());
                    }
                    for echo_u in echo_uniforms {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(echo_u);
                        layer_mask_plans.push(mask.clone());
                        layer_textures.push(texture.clone());
                        multipass_plans.push(mp.clone());
                    }
                    if in_back {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask);
                        layer_textures.push(texture);
                        multipass_plans.push(mp);
                    }
                }
            }

            if active_layers.iter().any(|layer| {
                matches!(
                    layer.layer_type,
                    crate::core::timeline::LayerType::Particle { .. }
                )
            }) {
                let old_layers = std::mem::take(&mut active_layers);
                let old_comp_indices = std::mem::take(&mut comp_indices);
                let old_uniforms = std::mem::take(&mut uniforms);
                let old_masks = std::mem::take(&mut layer_mask_plans);
                let old_textures = std::mem::take(&mut layer_textures);
                let old_multipass = std::mem::take(&mut multipass_plans);
                let m_proj_p = [
                    [2.0 / comp.width.max(1) as f32, 0.0, 0.0, 0.0],
                    [0.0, -2.0 / comp.height.max(1) as f32, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-1.0, 1.0, 0.0, 1.0],
                ];
                let dt = 1.0 / comp.fps.max(1) as f32;
                for (((((layer, comp_idx), uniform), mask), texture), mp) in old_layers
                    .into_iter()
                    .zip(old_comp_indices)
                    .zip(old_uniforms)
                    .zip(old_masks)
                    .zip(old_textures)
                    .zip(old_multipass)
                {
                    let crate::core::timeline::LayerType::Particle { emitter } = &layer.layer_type
                    else {
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(uniform);
                        layer_mask_plans.push(mask);
                        layer_textures.push(texture);
                        multipass_plans.push(mp);
                        continue;
                    };
                    let target_f = effective_render_frame(layer, frame, comp.fps);
                    let mut ps = crate::core::particle_system::ParticleSystem::new(emitter.clone());
                    for f in 0..=target_f {
                        let (epos, _, _, _) = comp.resolve_world_transform(layer, f);
                        ps.update(dt, epos[0], epos[1]);
                    }
                    let proj = if layer.is_3d || comp.active_camera.active {
                        let cam = comp.resolve_camera_at();
                        let cpos = cam.transform.position.evaluate(frame);
                        let crot = cam.transform.rotation.evaluate(frame);
                        let rad = crot[2].to_radians();
                        let fov = cam.fov_at(frame).max(1.0).to_radians();
                        let focal = (comp.height.max(1) as f32 * 0.5) / (fov * 0.5).tan();
                        Some(crate::core::particle_system::CameraProjection {
                            cam_x: cpos[0],
                            cam_y: cpos[1],
                            cam_z: cpos[2],
                            focal,
                            cos_rz: rad.cos(),
                            sin_rz: rad.sin(),
                        })
                    } else {
                        None
                    };
                    for p in &ps.particles {
                        if p.life <= 0.0 {
                            continue;
                        }
                        let t = (1.0 - p.life / p.max_life.max(0.001)).clamp(0.0, 1.0);
                        let r = emitter.color_start[0]
                            + (emitter.color_end[0] - emitter.color_start[0]) * t;
                        let g = emitter.color_start[1]
                            + (emitter.color_end[1] - emitter.color_start[1]) * t;
                        let b = emitter.color_start[2]
                            + (emitter.color_end[2] - emitter.color_start[2]) * t;
                        let a = (emitter.color_start[3]
                            + (emitter.color_end[3] - emitter.color_start[3]) * t)
                            * emitter.fade_curve.apply(t);
                        if a <= 0.001 {
                            continue;
                        }
                        let Some((sx, sy, sscale)) = proj
                            .map_or(Some((p.x, p.y, 1.0)), |cp| cp.project(p.x, p.y, p.z))
                        else {
                            continue;
                        };
                        let sz = (p.size * sscale).max(1.0);
                        for ti in 0..(p.trail_len as usize).min(p.trail.len()) {
                            let (tx, ty) = p.trail[ti];
                            let decay = 1.0 - (ti + 1) as f32 / (p.trail_len as f32 + 1.0);
                            let tsz = (sz * decay).max(1.0);
                            let m_model_t = [
                                [tsz, 0.0, 0.0, 0.0],
                                [0.0, tsz, 0.0, 0.0],
                                [0.0, 0.0, 1.0, 0.0],
                                [tx, ty, 0.0, 1.0],
                            ];
                            let mut t_u = uniform;
                            t_u.layer_type = 2;
                            t_u.shape_type = 1;
                            t_u.color = [r, g, b, a * decay * 0.6];
                            t_u.shape_size_x = tsz;
                            t_u.shape_size_y = tsz;
                            t_u.shape_stroke_width = 0.0;
                            t_u.transform_matrix = mat4_mul(m_proj_p, m_model_t);
                            active_layers.push(layer);
                            comp_indices.push(comp_idx);
                            uniforms.push(t_u);
                            layer_mask_plans.push(mask.clone());
                            layer_textures.push(None);
                            multipass_plans.push(mp.clone());
                        }
                        let m_model_p = [
                            [sz, 0.0, 0.0, 0.0],
                            [0.0, sz, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [sx, sy, 0.0, 1.0],
                        ];
                        let mut p_u = uniform;
                        p_u.layer_type = 2;
                        p_u.shape_type = 1;
                        p_u.color = [r, g, b, a];
                        p_u.shape_size_x = sz;
                        p_u.shape_size_y = sz;
                        p_u.shape_stroke_width = 0.0;
                        p_u.transform_matrix = mat4_mul(m_proj_p, m_model_p);
                        active_layers.push(layer);
                        comp_indices.push(comp_idx);
                        uniforms.push(p_u);
                        layer_mask_plans.push(mask.clone());
                        layer_textures.push(None);
                        multipass_plans.push(mp.clone());
                    }
                }
            }

            if active_layers.iter().any(|layer| !layer.paint_strokes.is_empty()) {
                let old_layers = std::mem::take(&mut active_layers);
                let old_comp_indices = std::mem::take(&mut comp_indices);
                let old_uniforms = std::mem::take(&mut uniforms);
                let old_masks = std::mem::take(&mut layer_mask_plans);
                let old_textures = std::mem::take(&mut layer_textures);
                let old_multipass = std::mem::take(&mut multipass_plans);
                let m_proj_b = [
                    [2.0 / comp.width.max(1) as f32, 0.0, 0.0, 0.0],
                    [0.0, -2.0 / comp.height.max(1) as f32, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-1.0, 1.0, 0.0, 1.0],
                ];
                for (((((layer, comp_idx), uniform), mask), texture), mp) in old_layers
                    .into_iter()
                    .zip(old_comp_indices)
                    .zip(old_uniforms)
                    .zip(old_masks)
                    .zip(old_textures)
                    .zip(old_multipass)
                {
                    active_layers.push(layer);
                    comp_indices.push(comp_idx);
                    uniforms.push(uniform);
                    layer_mask_plans.push(mask.clone());
                    layer_textures.push(texture.clone());
                    multipass_plans.push(mp);

                    if layer.paint_strokes.is_empty() {
                        continue;
                    }
                    let sf = effective_render_frame(layer, frame, comp.fps);
                    let (wpos, wscale, wrot, _) = comp.resolve_world_transform(layer, frame);
                    let sx = wscale[0] / 100.0;
                    let sy = wscale[1] / 100.0;
                    let bscale = wscale[0].abs().max(wscale[1].abs()) / 100.0;
                    let rad = wrot.to_radians();
                    let cos_r = rad.cos();
                    let sin_r = rad.sin();
                    let to_world = |lp: [f32; 2]| -> [f32; 2] {
                        let x = lp[0] * sx;
                        let y = lp[1] * sy;
                        [wpos[0] + x * cos_r - y * sin_r, wpos[1] + x * sin_r + y * cos_r]
                    };
                    for stroke in &layer.paint_strokes {
                        let end_f = if stroke.end_frame == 0 {
                            layer.out_frame
                        } else {
                            stroke.end_frame
                        };
                        if sf < stroke.start_frame || sf > end_f || stroke.points.is_empty() {
                            continue;
                        }
                        let sz = (stroke.size * bscale).max(1.0);
                        let step_dist = (sz * 0.25).max(1.0);
                        let mut dabs = Vec::new();
                        let mut prev = to_world(stroke.points[0]);
                        dabs.push(prev);
                        for &pt in stroke.points.iter().skip(1) {
                            let cur = to_world(pt);
                            let dx = cur[0] - prev[0];
                            let dy = cur[1] - prev[1];
                            let dist = (dx * dx + dy * dy).sqrt();
                            let steps = ((dist / step_dist).ceil() as usize).clamp(1, 64);
                            for s in 1..=steps {
                                let t = s as f32 / steps as f32;
                                dabs.push([prev[0] + dx * t, prev[1] + dy * t]);
                            }
                            prev = cur;
                            if dabs.len() >= 256 {
                                break;
                            }
                        }
                        for wp in dabs {
                            let m_model_b = [
                                [sz, 0.0, 0.0, 0.0],
                                [0.0, sz, 0.0, 0.0],
                                [0.0, 0.0, 1.0, 0.0],
                                [wp[0], wp[1], 0.0, 1.0],
                            ];
                            let mut b_u = uniform;
                            if matches!(
                                stroke.mode,
                                crate::core::timeline::PaintStrokeMode::CloneStamp
                            ) {
                                b_u.layer_type = 1;
                                b_u.sim_enabled = 3;
                                b_u.sim_type = 84;
                                b_u.sim_p1 = stroke.clone_offset[0] / comp.width.max(1) as f32;
                                b_u.sim_p2 = stroke.clone_offset[1] / comp.height.max(1) as f32;
                                b_u.opacity = (uniform.opacity * stroke.opacity).clamp(0.0, 100.0);
                                b_u.blend_mode = 0;
                                b_u.transform_matrix = mat4_mul(m_proj_b, m_model_b);
                                active_layers.push(layer);
                                comp_indices.push(comp_idx);
                                uniforms.push(b_u);
                                layer_mask_plans.push(mask.clone());
                                layer_textures.push(texture.clone());
                                multipass_plans.push(None);
                            } else {
                                b_u.layer_type = 2;
                                b_u.shape_type = 1;
                                b_u.color = [
                                    stroke.color[0],
                                    stroke.color[1],
                                    stroke.color[2],
                                    stroke.color[3] * stroke.flow,
                                ];
                                b_u.opacity = (uniform.opacity * stroke.opacity).clamp(0.0, 100.0);
                                b_u.shape_size_x = sz;
                                b_u.shape_size_y = sz;
                                b_u.shape_stroke_width = 0.0;
                                b_u.blend_mode = 0;
                                b_u.transform_matrix = mat4_mul(m_proj_b, m_model_b);
                                active_layers.push(layer);
                                comp_indices.push(comp_idx);
                                uniforms.push(b_u);
                                layer_mask_plans.push(mask.clone());
                                layer_textures.push(None);
                                multipass_plans.push(None);
                            }
                        }
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
                let old_multipass_plans = multipass_plans;
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
                multipass_plans = order
                    .iter()
                    .map(|&index| old_multipass_plans[index].clone())
                    .collect();
            }

            let final_uniform_indices: Vec<usize> = (0..active_layers.len()).collect();
            let mut scratch_steps_per_layer: Vec<Vec<super::render_encoder::ScratchDrawStep>> =
                vec![Vec::new(); active_layers.len()];
            let mut final_src_scratch: Vec<Option<usize>> = vec![None; active_layers.len()];
            for (layer_idx, plan_opt) in multipass_plans.into_iter().enumerate() {
                if let Some(plan) = plan_opt {
                    final_src_scratch[layer_idx] = Some(plan.final_src_scratch);
                    for (u, src_is_scratch, dst_is_a, custom_shader, lut_pass) in plan.steps {
                        let u_idx = uniforms.len();
                        uniforms.push(u);
                        scratch_steps_per_layer[layer_idx].push(
                            super::render_encoder::ScratchDrawStep {
                                uniform_index: u_idx,
                                src_is_scratch,
                                dst_is_a,
                                custom_shader,
                                lut_pass,
                            },
                        );
                    }
                }
            }

            // Step 1b: Ensure backdrop texture exists if needed, then prepare track matte textures.
            if active_layers
                .iter()
                .any(|l| super::render_encoder::needs_backdrop_copy(l))
            {
                self.ensure_backdrop_texture(width, height);
            }
            let Some(matte_resources) = self.prepare_track_mattes(
                comp,
                frame,
                width,
                height,
                &active_layers,
                &comp_indices,
            ) else {
                return false;
            };

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
                final_uniform_indices: &final_uniform_indices,
                scratch_steps_per_layer: &scratch_steps_per_layer,
                final_src_scratch: &final_src_scratch,
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
