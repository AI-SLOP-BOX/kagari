//! Post-raster pixel processing for the software renderer.
//!
//! Phase 2 (effect stack), motion blur, 3D light shading, layer
//! styles, depth-of-field, shadow receiving, and the post-effect
//! re-mask containment pass. Operates in place on the layer buffer.

use super::mask::{compute_combined_mask_coverage, CpuMaskEntry};
use crate::core::timeline::{Composition, Layer, LightType};

/// Everything [`apply_post_fx`] needs from the render loop.
pub(crate) struct PostFxCtx<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub effective_frame: u32,
    pub frame: u32,
    pub sorted_idx: usize,
    pub layer_buf: &'a mut [u8],
    pub min_x: u32,
    pub min_y: u32,
    pub bw: u32,
    pub bh: u32,
    pub width: u32,
    pub height: u32,
    pub masks: &'a [CpuMaskEntry],
    pub cx: f32,
    pub cy: f32,
    pub shadow_map: &'a [f32],
    pub dof_blur: f32,
}

/// Match the software renderer's flat-plane material lighting in the GPU
/// uniform builder so CPU and GPU image layers use the same evaluated shade.
pub(crate) fn layer_3d_shading(
    comp: &Composition,
    layer: &Layer,
    frame: u32,
    cx: f32,
    cy: f32,
) -> [f32; 3] {
    let mat = &layer.material;
    let layer_z = layer.transform_3d.position.evaluate(frame)[2];
    let metalness = mat.metalness.clamp(0.0, 1.0);
    let mut shade = [mat.ambient * (1.0 - metalness); 3];
    if mat.accepts_lights {
        for light in &comp.lights {
            if matches!(&light.light_type, LightType::Ambient) {
                let contribution =
                    mat.ambient * (1.0 - metalness) * light.intensity_at(frame) / 100.0;
                for (channel, value) in shade.iter_mut().enumerate() {
                    *value += contribution * light.color[channel];
                }
                continue;
            }
            let lpos = comp.light_position_at(light, frame);
            let (ldz, atten_dist) = if matches!(&light.light_type, LightType::Parallel) {
                let direction = super::shadow::PARALLEL_LIGHT_DIRECTION;
                let length = (direction[0] * direction[0]
                    + direction[1] * direction[1]
                    + direction[2] * direction[2])
                    .sqrt();
                (direction[2] / length, 1.0)
            } else {
                let lx = cx - lpos[0];
                let ly = cy - lpos[1];
                let lz = layer_z - lpos[2];
                let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);
                (lz / dist, 1.0 / (1.0 + (dist / 500.0).powi(2)))
            };
            let ndotl = ldz.max(0.0);
            let atten_base = light.intensity_at(frame) / 100.0;
            let mut attenuation = atten_base * atten_dist;

            if let LightType::Spot {
                cone_angle_deg,
                cone_feather_pct,
            } = light.light_type
            {
                let cone_rad = cone_angle_deg.to_radians() * 0.5;
                let cone_edge = cone_rad.cos();
                let feather = (cone_feather_pct / 100.0).clamp(0.01, 1.0);
                let spot_falloff =
                    ((ndotl - cone_edge) / (feather * (1.0 - cone_edge).max(0.01))).clamp(0.0, 1.0);
                attenuation *= spot_falloff;
            }

            let lc = light.color;
            for channel in 0..3 {
                shade[channel] +=
                    ndotl * attenuation * mat.diffuse * (1.0 - metalness) * lc[channel];
            }

            if mat.specular > 0.01 {
                let hz = 1.0 + ldz;
                let h_len = hz.abs().max(0.001);
                let ndoth = (hz / h_len).max(0.0);
                let spec = ndoth.powf(mat.specular_exponent) * attenuation * mat.specular;
                for channel in 0..3 {
                    shade[channel] += spec * lc[channel];
                }
            }
        }
    }
    for value in &mut shade {
        *value = (*value + mat.emission).clamp(0.0, 3.0);
    }
    shade
}

fn linked_lens_flare_position(
    comp: &Composition,
    layer: &Layer,
    frame: u32,
    min_x: u32,
    min_y: u32,
    width: u32,
    height: u32,
) -> Option<[f32; 2]> {
    let light_name = layer
        .effects
        .iter()
        .find_map(|effect| match &effect.effect_type {
            crate::core::timeline::EffectType::LensFlare {
                link_to_light: Some(name),
                ..
            } if effect.enabled => Some(name),
            _ => None,
        })?;
    let light = comp.lights.iter().find(|light| light.name == *light_name)?;
    let screen = crate::core::timeline::project_point_to_screen_at_frame(
        &comp.resolve_camera_at(),
        comp.light_position_at(light, frame),
        comp.width as f32,
        comp.height as f32,
        frame,
    )?;
    if width == 0 || height == 0 {
        return None;
    }
    Some([
        (screen[0] * comp.width as f32 - min_x as f32) / width as f32,
        (screen[1] * comp.height as f32 - min_y as f32) / height as f32,
    ])
}

/// Run all post-raster processing on `ctx.layer_buf` in place.
pub(crate) fn apply_post_fx(ctx: PostFxCtx<'_>) {
    let PostFxCtx {
        comp,
        layer,
        effective_frame,
        frame,
        sorted_idx,
        layer_buf,
        min_x,
        min_y,
        bw,
        bh,
        width,
        height,
        masks,
        cx,
        cy,
        shadow_map,
        dof_blur,
        ..
    } = ctx;
    // Phase 2: apply the layer's CPU effect stack.
    // Resolve the projected composition-space light into this cropped layer buffer.
    let flare_light_screen = if layer.effects_enabled {
        linked_lens_flare_position(comp, layer, frame, min_x, min_y, bw, bh)
    } else {
        None
    };
    if layer.effects_enabled {
        crate::core::cpu_effects::apply_layer_effects_ctx(
            Some(comp),
            Some(sorted_idx),
            &mut *layer_buf,
            bw,
            bh,
            &layer.effects,
            effective_frame,
            comp.fps,
            flare_light_screen,
        );
    }

    // Phase 2.5: velocity-based motion blur (AE-style shutter angle and phase).
    // Computes the layer's positional velocity across neighboring frames and
    // smears the layer buffer along the motion vector.
    if layer.motion_blur && comp.motion_blur_shutter_angle > 0.0 {
        let fps = comp.fps.max(1);
        let f_prev = frame.saturating_sub(1);
        let f_next = frame.saturating_add(1);
        let (p_prev, _, _, _) = comp.resolve_world_transform(layer, f_prev);
        let (p_next, _, _, _) = comp.resolve_world_transform(layer, f_next);
        let vel_x = (p_next[0] - p_prev[0]) * 0.5;
        let vel_y = (p_next[1] - p_prev[1]) * 0.5;
        let speed = (vel_x * vel_x + vel_y * vel_y).sqrt();
        if speed > 0.05 {
            let shutter = (comp.motion_blur_shutter_angle / 360.0).clamp(0.0, 2.0);
            let samples = ((speed * shutter).ceil() as u32).clamp(2, 32);
            // Shutter phase shifts the sampling window along the smear.
            // Default -90° maps to a centered window, preserving legacy output.
            let phase_offset = (comp.motion_blur_shutter_phase + 90.0) / 360.0;
            crate::core::ae_effects_pack_v17::apply_motion_blur_vector(
                &mut *layer_buf,
                bw,
                bh,
                vel_x * shutter * fps as f32 / 24.0,
                vel_y * shutter * fps as f32 / 24.0,
                samples,
                phase_offset,
            );
        }
    }

    // Phase 2.6: 3D light shading for 3D layers.
    // Phong shading with material properties (ambient, diffuse, specular, emission).
    // Enhanced: spot light cone falloff, inverse-square attenuation, light color tinting.
    if layer.is_3d {
        // Match the GPU uniform builder: evaluate lighting at the projected
        // layer origin, not the center of its clipped raster bounds. Those
        // centers diverge sharply when a plane crosses the near plane.
        let camera = comp.resolve_camera_at();
        let shade_center = crate::core::timeline::project_point_to_screen_at_frame(
            &camera,
            layer.transform_3d.position.evaluate(frame),
            comp.width as f32,
            comp.height as f32,
            frame,
        )
        .map(|[x, y]| [x * width as f32, y * height as f32])
        .unwrap_or([cx, cy]);
        let shade = layer_3d_shading(comp, layer, frame, shade_center[0], shade_center[1]);
        let shade_avg = ((shade[0] + shade[1] + shade[2]) / 3.0).clamp(0.0, 3.0);
        if (shade_avg - 1.0).abs() > 0.01 {
            for px_chunk in layer_buf.as_chunks_mut::<4>().0 {
                px_chunk[0] = ((px_chunk[0] as f32 * shade[0]).min(255.0)) as u8;
                px_chunk[1] = ((px_chunk[1] as f32 * shade[1]).min(255.0)) as u8;
                px_chunk[2] = ((px_chunk[2] as f32 * shade[2]).min(255.0)) as u8;
            }
        }
    }

    // Phase 2.65: AE Layer Styles (Drop Shadow / Outer Glow / Stroke)
    {
        let st = &layer.style;
        let to_rgba8 = |c: [f32; 4]| {
            [
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                (c[3].clamp(0.0, 1.0) * 255.0) as u8,
            ]
        };
        if st.drop_shadow.enabled {
            let mut color = st.drop_shadow.color;
            color[3] *= (st.drop_shadow.opacity / 100.0).clamp(0.0, 1.0);
            crate::core::ae_effects_pack::apply_drop_shadow(
                &mut *layer_buf,
                bw,
                bh,
                st.drop_shadow.distance,
                st.drop_shadow.angle,
                (st.drop_shadow.size.round() as u32).max(1),
                to_rgba8(color),
            );
        }
        if st.inner_shadow.enabled {
            let s = &st.inner_shadow;
            let mut color = s.color;
            color[3] *= (s.opacity / 100.0).clamp(0.0, 1.0);
            crate::core::ae_effects_pack::apply_inner_shadow(
                &mut *layer_buf,
                bw,
                bh,
                s.distance,
                s.angle,
                (s.size.round() as u32).min(64),
                to_rgba8(color),
            );
        }
        if st.outer_glow.enabled {
            crate::core::ae_effects_pack::apply_glow(
                &mut *layer_buf,
                bw,
                bh,
                1.0,
                (st.outer_glow.size.round() as u32).max(1),
                (st.outer_glow.opacity / 50.0).clamp(0.0, 2.0),
            );
        }
        if st.inner_glow.enabled {
            crate::core::ae_effects_pack::apply_inner_glow(
                &mut *layer_buf,
                bw,
                bh,
                (st.inner_glow.size.round() as u32).min(64),
                to_rgba8(st.inner_glow.color),
                st.inner_glow.opacity,
            );
        }
        if st.satin.enabled {
            let s = &st.satin;
            crate::core::ae_effects_pack::apply_satin(
                &mut *layer_buf,
                bw,
                bh,
                &crate::core::ae_effects_pack::SatinParams {
                    distance: s.distance,
                    angle_deg: s.angle,
                    size: (s.size.round() as u32).min(64),
                    color: to_rgba8(s.color),
                    opacity: s.opacity,
                },
            );
        }
        if st.bevel_emboss.enabled {
            let s = &st.bevel_emboss;
            crate::core::ae_effects_pack::apply_bevel_emboss(
                &mut *layer_buf,
                bw,
                bh,
                &crate::core::ae_effects_pack::BevelEmbossParams {
                    angle_deg: s.angle,
                    depth_px: s.depth.max(1.0),
                    size_px: (s.size.round() as u32).min(64),
                    color_light: to_rgba8(s.color_light),
                    color_dark: to_rgba8(s.color_dark),
                    highlight_strength: s.highlight,
                    shadow_strength: s.shadow,
                },
            );
        }
        if st.stroke.enabled {
            crate::core::ae_effects_pack_v2::apply_stroke_effect_position(
                &mut *layer_buf,
                bw,
                bh,
                to_rgba8(st.stroke.color),
                (st.stroke.size.round() as u32).max(1),
                st.stroke.position,
            );
        }
        if st.gradient_overlay.enabled {
            let s = &st.gradient_overlay;
            crate::core::ae_effects_pack::apply_gradient_overlay(
                &mut *layer_buf,
                bw,
                bh,
                &crate::core::ae_effects_pack::GradientOverlayParams {
                    angle_deg: s.angle,
                    scale_pct: s.scale,
                    start: to_rgba8(s.color_start),
                    end: to_rgba8(s.color_end),
                    opacity: s.opacity,
                },
            );
        }
        if st.color_overlay.enabled {
            crate::core::ae_effects_pack::apply_color_overlay(
                &mut *layer_buf,
                bw,
                bh,
                to_rgba8(st.color_overlay.color),
                st.color_overlay.opacity,
            );
        }
    }

    // Phase 2.7: depth-of-field defocus for 3D layers.
    if dof_blur >= 1.0 {
        crate::core::ae_effects_pack::apply_gaussian_blur(
            &mut *layer_buf,
            bw,
            bh,
            dof_blur.round() as u32,
        );
    }

    // Phase 2.72: receive shadows — multiply layer pixels by the sampled
    // shadow density at their world position (all receiver types).
    if layer.material.accepts_shadows && !shadow_map.is_empty() && bw > 0 && bh > 0 {
        let sw = width.max(1) as usize;
        let sh = height.max(1) as usize;
        for ly in 0..bh {
            if super::render_cancelled() {
                return;
            }
            let wy = (min_y + ly).min(sh as u32 - 1) as usize;
            for lx in 0..bw {
                let wx = (min_x + lx).min(sw as u32 - 1) as usize;
                let occ = shadow_map[wy * sw + wx];
                if occ <= 0.003 {
                    continue;
                }
                let lidx = ((ly * bw + lx) * 4) as usize;
                if lidx + 3 >= layer_buf.len() || layer_buf[lidx + 3] == 0 {
                    continue;
                }
                let f = 1.0 - occ;
                layer_buf[lidx] = (layer_buf[lidx] as f32 * f) as u8;
                layer_buf[lidx + 1] = (layer_buf[lidx + 1] as f32 * f) as u8;
                layer_buf[lidx + 2] = (layer_buf[lidx + 2] as f32 * f) as u8;
            }
        }
    }

    // Phase 2.8: post-effect re-mask (contain). Effects (blur, glow,
    // displacement) can spill pixels outside the masked region; re-cut
    // here so the final composite respects the masks. This intentionally
    // differs from AE's default (effects spill past masks): Kagari
    // contains by default for predictable compositing. Pre-effect
    // isolation happens during raster (baked) or just above (shapes).
    if !masks.is_empty() {
        for ly in 0..bh {
            if super::render_cancelled() {
                return;
            }
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                if lidx + 3 >= layer_buf.len() || layer_buf[lidx + 3] == 0 {
                    continue;
                }
                let cov = compute_combined_mask_coverage(
                    (min_x + lx) as f32 + 0.5,
                    (min_y + ly) as f32 + 0.5,
                    masks,
                );
                if cov < 0.999 {
                    layer_buf[lidx + 3] = (layer_buf[lidx + 3] as f32 * cov) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod lens_flare_link_tests {
    use super::linked_lens_flare_position;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Effect, EffectType, Layer, LayerType, Light3D};

    #[test]
    fn linked_flare_maps_comp_light_into_the_cropped_layer_buffer() {
        let mut comp = Composition::new("flare-link".into(), "Flare Link".into(), 100, 80, 30, 1);
        comp.lights.push(Light3D {
            name: "Key".into(),
            position: Animatable::new_constant([0.0, 0.0, 1000.0]),
            ..Default::default()
        });
        let mut layer = Layer::new(
            "source".into(),
            "Source".into(),
            LayerType::Solid {
                color: [0.2, 0.3, 0.4, 1.0],
            },
            1,
        );
        layer.effects.push(Effect {
            id: "flare".into(),
            name: "Lens Flare".into(),
            effect_type: EffectType::LensFlare {
                enabled: Animatable::new_constant(1.0),
                position_x: Animatable::new_constant(0.1),
                position_y: Animatable::new_constant(0.2),
                intensity: Animatable::new_constant(1.0),
                threshold: Animatable::new_constant(1.0),
                color: Animatable::new_constant([1.0; 4]),
                link_to_light: Some("Key".into()),
            },
            enabled: true,
        });

        let position = linked_lens_flare_position(&comp, &layer, 0, 25, 10, 50, 40)
            .expect("the linked light projects in front of the camera");
        assert!((position[0] - 0.5).abs() < 1e-5);
        assert!((position[1] - 0.75).abs() < 1e-5);
    }
}
