use super::super::*;
use super::PreparedLayerInputs;

impl WgpuRenderer {
    pub(super) fn expand_layer_modifiers<'a>(
        &self,
        comp: &'a Composition,
        frame: u32,
        prepared: &mut PreparedLayerInputs<'a>,
        layer_textures_arg: &mut Vec<Option<std::sync::Arc<wgpu::BindGroup>>>,
    ) {
        let mut layer_textures = std::mem::take(layer_textures_arg);
        let mut active_layers = std::mem::take(&mut prepared.active_layers);
        let mut comp_indices = std::mem::take(&mut prepared.comp_indices);
        let mut uniforms = std::mem::take(&mut prepared.uniforms);
        let mut layer_mask_plans = std::mem::take(&mut prepared.layer_mask_plans);
        let mut multipass_plans = std::mem::take(&mut prepared.multipass_plans);
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
        prepared.active_layers = active_layers;
        prepared.comp_indices = comp_indices;
        prepared.uniforms = uniforms;
        prepared.layer_mask_plans = layer_mask_plans;
        prepared.multipass_plans = multipass_plans;
        *layer_textures_arg = layer_textures;
    }
}
