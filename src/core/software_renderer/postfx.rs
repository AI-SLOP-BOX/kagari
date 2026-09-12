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

/// Run all post-raster processing on `ctx.layer_buf` in place.
pub(crate) fn apply_post_fx(ctx: PostFxCtx<'_>) {
    let PostFxCtx {
        comp,
        layer,
        effective_frame,
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
    // Resolve lens-flare light links (project the named light through the camera).
    let flare_light_screen: Option<[f32; 2]> = layer
        .effects
        .iter()
        .find_map(|e| match &e.effect_type {
            crate::core::timeline::EffectType::LensFlare {
                link_to_light: Some(n),
                ..
            } if e.enabled => Some(n.clone()),
            _ => None,
        })
        .and_then(|light_name| {
            comp.lights
                .iter()
                .find(|l| l.name == light_name)
                .and_then(|light| {
                    crate::core::timeline::project_point_to_screen_at_frame(
                        comp.resolve_camera(),
                        light.position.evaluate(effective_frame),
                        bw as f32,
                        bh as f32,
                        effective_frame,
                    )
                })
        });
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

    // Phase 2.5: velocity-based motion blur (AE-style shutter angle and phase).
    // Computes the layer's positional velocity across neighboring frames and
    // smears the layer buffer along the motion vector.
    if layer.motion_blur && comp.motion_blur_shutter_angle > 0.0 {
        let fps = comp.fps.max(1);
        let f_prev = effective_frame.saturating_sub(1);
        let f_next = effective_frame.saturating_add(1);
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
        let mat = &layer.material;
        let layer_z = layer.transform_3d.position.evaluate(effective_frame)[2];
        let mut shade_r = mat.ambient;
        let mut shade_g = mat.ambient;
        let mut shade_b = mat.ambient;
        for light in &comp.lights {
            let lpos = light.position.evaluate(effective_frame);
            let lx = cx - lpos[0];
            let ly = cy - lpos[1];
            let lz = layer_z - lpos[2];
            let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);

            // Light direction (normalized)
            let _ldx = lx / dist;
            let _ldy = ly / dist;
            let ldz = lz / dist;

            // N·L with flat normal facing the camera (+z)
            let ndotl = (ldz).max(0.0);

            // Inverse-square attenuation with configurable intensity
            let atten_base = light.intensity / 100.0;
            let atten_dist = 1.0 / (1.0 + (dist / 500.0).powi(2));
            let mut attenuation = atten_base * atten_dist;

            // Spot light cone falloff
            if let LightType::Spot {
                cone_angle_deg,
                cone_feather_pct,
            } = light.light_type
            {
                let cone_rad = cone_angle_deg.to_radians() * 0.5;
                let cos_angle = ldz.max(0.0); // angle from light's forward direction (+z)
                let cone_edge = cone_rad.cos();
                let feather = (cone_feather_pct / 100.0).clamp(0.01, 1.0);
                let spot_falloff = ((cos_angle - cone_edge)
                    / (feather * (1.0 - cone_edge).max(0.01)))
                .clamp(0.0, 1.0);
                attenuation *= spot_falloff;
            }

            // Diffuse component with light color
            let lc = light.color;
            shade_r += ndotl * attenuation * mat.diffuse * lc[0];
            shade_g += ndotl * attenuation * mat.diffuse * lc[1];
            shade_b += ndotl * attenuation * mat.diffuse * lc[2];

            // Specular component (Blinn-Phong) with light color
            if mat.specular > 0.01 {
                let hx = 0.0;
                let hy = 0.0;
                let hz = 1.0 + ldz;
                let h_len = (hx * hx + hy * hy + hz * hz).sqrt().max(0.001);
                let ndoth = (hz / h_len).max(0.0);
                let spec = ndoth.powf(mat.specular_exponent) * attenuation * mat.specular;
                shade_r += spec * lc[0];
                shade_g += spec * lc[1];
                shade_b += spec * lc[2];
            }
        }
        // Emission adds constant self-illumination
        shade_r += mat.emission;
        shade_g += mat.emission;
        shade_b += mat.emission;
        let shade_avg = ((shade_r + shade_g + shade_b) / 3.0).clamp(0.0, 3.0);
        if (shade_avg - 1.0).abs() > 0.01 {
            for px_chunk in layer_buf.chunks_exact_mut(4) {
                px_chunk[0] = ((px_chunk[0] as f32 * shade_r.clamp(0.0, 3.0)).min(255.0)) as u8;
                px_chunk[1] = ((px_chunk[1] as f32 * shade_g.clamp(0.0, 3.0)).min(255.0)) as u8;
                px_chunk[2] = ((px_chunk[2] as f32 * shade_b.clamp(0.0, 3.0)).min(255.0)) as u8;
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
            crate::core::ae_effects_pack::apply_drop_shadow(
                &mut *layer_buf,
                bw,
                bh,
                st.drop_shadow.distance,
                st.drop_shadow.angle,
                (st.drop_shadow.size.round() as u32).max(1),
                to_rgba8(st.drop_shadow.color),
            );
        }
        if st.inner_shadow.enabled {
            let s = &st.inner_shadow;
            crate::core::ae_effects_pack::apply_inner_shadow(
                &mut *layer_buf,
                bw,
                bh,
                s.distance,
                s.angle,
                (s.size.round() as u32).min(64),
                to_rgba8(s.color),
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
            crate::core::ae_effects_pack_v2::apply_stroke_effect(
                &mut *layer_buf,
                bw,
                bh,
                to_rgba8(st.stroke.color),
                (st.stroke.size.round() as u32).max(1),
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
    if !shadow_map.is_empty() && bw > 0 && bh > 0 {
        let sw = width.max(1) as usize;
        let sh = height.max(1) as usize;
        for ly in 0..bh {
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
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                if lidx + 3 >= layer_buf.len() || layer_buf[lidx + 3] == 0 {
                    continue;
                }
                let cov = compute_combined_mask_coverage(
                    (min_x + lx) as f32,
                    (min_y + ly) as f32,
                    masks,
                );
                if cov < 0.999 {
                    layer_buf[lidx + 3] = (layer_buf[lidx + 3] as f32 * cov) as u8;
                }
            }
        }
    }
}
