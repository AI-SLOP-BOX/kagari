use super::super::*;
use super::{LayerMultipassPlan, PreparedLayerInputs};

impl WgpuRenderer {
    pub(super) fn prepare_active_layers<'a>(
        &mut self,
        comp: &'a Composition,
        frame: u32,
        width: u32,
        height: u32,
        m_proj: [[f32; 4]; 4],
        target_snapshot: bool,
        render_key: RenderKey,
        layer_textures: &mut Vec<Option<std::sync::Arc<wgpu::BindGroup>>>,
    ) -> Option<PreparedLayerInputs<'a>> {
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
                            return None;
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

        Some(PreparedLayerInputs {
            active_layers,
            comp_indices,
            uniforms,
            layer_mask_plans,
            multipass_plans,
        })
    }
}
