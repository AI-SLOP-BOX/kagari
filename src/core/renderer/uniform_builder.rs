use super::*;

pub(super) fn build_layer_uniform(
    comp: &Composition,
    layer: &Layer,
    frame: u32,
    effective_frame: u32,
    transform_matrix: [[f32; 4]; 4],
    opacity: f32,
    textured_source: bool,
) -> LayerUniform {
    // Prepare Layer Uniform details
    let (
        mut layer_type,
        shape_type,
        mut color,
        fill_type_u32,
        grad_start,
        grad_end,
        grad_colors,
        grad_center,
        grad_radius,
    ) = match &layer.layer_type {
        LayerType::Solid { color } => (
            0u32,
            0u32,
            *color,
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Image { .. } | LayerType::Video { .. } => (
            1u32,
            0u32,
            [1.0, 1.0, 1.0, 1.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Model3D { .. } => (
            1u32,
            0u32,
            [1.0, 1.0, 1.0, 1.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Shape {
            shape_type,
            color,
            fill_type,
            ..
        } => {
            let st = match shape_type {
                ShapeType::Rectangle { .. } => 0u32,
                ShapeType::Ellipse { .. } => 1u32,
                ShapeType::Star { .. } => 2u32,
                ShapeType::Polygon { .. } => 3u32,
                ShapeType::FreeformBezier { .. } => 4u32,
            };
            let (ft, ge, gs, gc, gcen, gr) = match fill_type {
                ShapeFillType::Solid => (
                    0u32,
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
                    [0.5, 0.5],
                    0.5,
                ),
                ShapeFillType::LinearGradient {
                    start, end, colors, ..
                } => {
                    let c1 = colors.first().copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let c2 = colors.get(1).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
                    (1u32, *start, *end, [c1, c2], [0.5, 0.5], 0.5)
                }
                ShapeFillType::RadialGradient {
                    center,
                    radius,
                    colors,
                    ..
                } => {
                    let c1 = colors.first().copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let c2 = colors.get(1).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
                    (2u32, [0.0, 0.0], [1.0, 1.0], [c1, c2], *center, *radius)
                }
            };
            (2u32, st, *color, ft, gs, ge, gc, gcen, gr)
        }
        LayerType::Text { color, .. } => (
            3u32,
            0u32,
            *color,
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Null => (
            4u32,
            0u32,
            [0.0, 0.0, 0.0, 0.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::PreComp { .. } => (
            5u32,
            0u32,
            [1.0, 1.0, 1.0, 1.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::AdjustmentLayer => (
            7u32,
            0u32,
            [1.0, 1.0, 1.0, 1.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Audio { .. } => (
            6u32,
            0u32,
            [0.0, 0.0, 0.0, 0.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
        LayerType::Particle { .. } => (
            8u32,
            0u32,
            [1.0, 1.0, 1.0, 1.0],
            0u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
            [0.5, 0.5],
            0.5,
        ),
    };

    // Textured text uses the image sampling path with unmodified texture colors
    if textured_source {
        layer_type = 1u32;
        color = [1.0, 1.0, 1.0, 1.0];
    }

    let mut ep = evaluate_effects(&layer.effects, frame);

    // Lens flare light-link: track a comp light's projected position
    if ep.flare_enabled == 1 {
        if let Some(light_name) = layer.effects.iter().find_map(|e| match &e.effect_type {
            crate::core::timeline::EffectType::LensFlare {
                link_to_light: Some(n),
                ..
            } if e.enabled => Some(n.clone()),
            _ => None,
        }) {
            if let Some(light) = comp.lights.iter().find(|l| l.name == light_name) {
                let lp = comp.light_position_at(light, frame);
                let render_camera = comp.resolve_camera_at();
                if let Some(sp) = crate::core::timeline::project_point_to_screen_at_frame(
                    &render_camera,
                    lp,
                    comp.width as f32,
                    comp.height as f32,
                    frame,
                ) {
                    ep.flare_pos_x = sp[0];
                    ep.flare_pos_y = sp[1];
                }
            }
        }
    }

    if ep.motionblur_enabled == 1 {
        let velocity = layer_motion_velocity(comp, layer, frame);
        ep.motionblur_velocity_x = velocity[0];
        ep.motionblur_velocity_y = velocity[1];
    }

    // Shape parameters for GPU SDFs: polygon/star point count, rectangle corner radius
    let shape_params_eval: [f32; 4] = match &layer.layer_type {
        LayerType::Shape { shape_type, .. } => match shape_type {
            ShapeType::Polygon { sides, .. } => [sides.evaluate(effective_frame), 0.0, 0.0, 0.0],
            ShapeType::Star { points, .. } => [points.evaluate(effective_frame), 0.0, 0.0, 0.0],
            ShapeType::Rectangle {
                corner_radius,
                width,
                height,
                ..
            } => {
                let cr = corner_radius.evaluate(effective_frame);
                let w = width.evaluate(effective_frame).max(1.0);
                let h = height.evaluate(effective_frame).max(1.0);
                // Normalize corner radius to 0..0.5 of the smaller half-size
                [0.0, (cr / w.min(h)).clamp(0.0, 0.5), 0.0, 0.0]
            }
            _ => [0.0; 4],
        },
        _ => [0.0; 4],
    };

    let layer_uniform = LayerUniform {
        transform_matrix,
        color,
        opacity,
        layer_type,
        shape_type,
        effect_tint_enabled: ep.tint_enabled,
        effect_tint_color: ep.tint_color,
        effect_tint_intensity: ep.tint_intensity,
        effect_blur_enabled: ep.blur_enabled,
        effect_blur_radius: ep.blur_radius,
        effect_shadow_enabled: ep.shadow_enabled,
        effect_shadow_color: ep.shadow_color,
        effect_shadow_opacity: ep.shadow_opacity,
        effect_shadow_direction: ep.shadow_direction,
        effect_shadow_distance: ep.shadow_distance,
        effect_shadow_softness: ep.shadow_softness,
        effect_ca_enabled: ep.chromatic_enabled,
        effect_ca_shift_r: ep.chromatic_shift_r,
        effect_ca_shift_b: ep.chromatic_shift_b,
        effect_ca_edge_falloff: ep.chromatic_edge_falloff,
        effect_vignette_enabled: ep.vignette_enabled,
        effect_vignette_intensity: ep.vignette_intensity,
        effect_vignette_roundness: ep.vignette_roundness,
        effect_vignette_feather: ep.vignette_feather,
        effect_vignette_color: ep.vignette_color,
        blend_mode: match layer.blend_mode {
            crate::core::timeline::BlendMode::Normal => 0,
            crate::core::timeline::BlendMode::Multiply => 1,
            crate::core::timeline::BlendMode::Screen => 2,
            crate::core::timeline::BlendMode::Overlay => 3,
            crate::core::timeline::BlendMode::Add => 4,
            crate::core::timeline::BlendMode::Darken => 5,
            crate::core::timeline::BlendMode::Lighten => 6,
            crate::core::timeline::BlendMode::SoftLight => 7,
            crate::core::timeline::BlendMode::HardLight => 8,
            crate::core::timeline::BlendMode::Difference => 9,
            crate::core::timeline::BlendMode::Exclusion => 10,
            crate::core::timeline::BlendMode::Divide => 11,
            crate::core::timeline::BlendMode::Subtract => 12,
            crate::core::timeline::BlendMode::ColorBurn => 13,
            crate::core::timeline::BlendMode::LinearBurn => 14,
            crate::core::timeline::BlendMode::VividLight => 15,
            crate::core::timeline::BlendMode::ColorDodge => 16,
            crate::core::timeline::BlendMode::LinearDodge => 17,
            crate::core::timeline::BlendMode::Color => 18,
            crate::core::timeline::BlendMode::Hue => 19,
            crate::core::timeline::BlendMode::Saturation => 20,
            crate::core::timeline::BlendMode::Luminosity => 21,
            crate::core::timeline::BlendMode::StencilAlpha => 22,
            crate::core::timeline::BlendMode::StencilLuma => 23,
            crate::core::timeline::BlendMode::SilhouetteAlpha => 24,
            crate::core::timeline::BlendMode::SilhouetteLuma => 25,
            crate::core::timeline::BlendMode::Behind => 26,
            crate::core::timeline::BlendMode::AlphaAdd => 27,
            crate::core::timeline::BlendMode::LinearLight => 28,
        },
        levels_enabled: ep.levels_enabled,
        levels_in_black: ep.levels_in_black,
        levels_in_white: ep.levels_in_white,
        levels_gamma: ep.levels_gamma,
        levels_out_black: ep.levels_out_black,
        levels_out_white: ep.levels_out_white,
        huesat_enabled: ep.huesat_enabled,
        huesat_hue: ep.huesat_hue,
        huesat_sat: ep.huesat_sat,
        huesat_light: ep.huesat_light,
        glow_enabled: ep.glow_enabled,
        glow_threshold: ep.glow_threshold,
        glow_radius: ep.glow_radius,
        glow_intensity: ep.glow_intensity,
        glow_color: ep.glow_color,
        grain_enabled: ep.grain_enabled,
        grain_intensity: ep.grain_intensity,
        grain_size: ep.grain_size,
        shape_params: shape_params_eval,
        track_matte_mode: match layer.track_matte {
            crate::core::timeline::TrackMatteMode::None => 0,
            crate::core::timeline::TrackMatteMode::AlphaMatte => 1,
            crate::core::timeline::TrackMatteMode::AlphaMatteInverted => 2,
            crate::core::timeline::TrackMatteMode::LumaMatte => 3,
            crate::core::timeline::TrackMatteMode::LumaMatteInverted => 4,
        },
        meshwarp_enabled: ep.meshwarp_enabled,
        corner_top_left: ep.corner_top_left,
        corner_top_right: ep.corner_top_right,
        corner_bottom_left: ep.corner_bottom_left,
        corner_bottom_right: ep.corner_bottom_right,
        motionblur_enabled: ep.motionblur_enabled,
        motionblur_shutter: ep.motionblur_shutter,
        motionblur_velocity_x: ep.motionblur_velocity_x,
        motionblur_velocity_y: ep.motionblur_velocity_y,
        motionblur_samples: ep.motionblur_samples,
        flare_enabled: ep.flare_enabled,
        flare_pos_x: ep.flare_pos_x,
        flare_pos_y: ep.flare_pos_y,
        flare_intensity: ep.flare_intensity,
        flare_threshold: ep.flare_threshold,
        flare_color: ep.flare_color,
        mask_enabled: 0,
        mask_mode: 0,
        mask_inverted: 0,
        mask_feather: 0.0,
        trim_start: layer
            .trim_paths
            .as_ref()
            .map(|t| t.start.evaluate(effective_frame) / 100.0)
            .unwrap_or(0.0),
        trim_end: layer
            .trim_paths
            .as_ref()
            .map(|t| t.end.evaluate(effective_frame) / 100.0)
            .unwrap_or(1.0),
        trim_offset: layer
            .trim_paths
            .as_ref()
            .map(|t| t.offset.evaluate(effective_frame).to_radians() / std::f32::consts::TAU)
            .unwrap_or(0.0),
        _pad_trim: 0.0,

        fill_type: fill_type_u32,
        grad_start_x: grad_start[0],
        grad_start_y: grad_start[1],
        grad_end_x: grad_end[0],
        grad_end_y: grad_end[1],
        grad_color1_r: grad_colors[0][0],
        grad_color1_g: grad_colors[0][1],
        grad_color1_b: grad_colors[0][2],
        grad_color1_a: grad_colors[0][3],
        grad_color2_r: grad_colors[1][0],
        grad_color2_g: grad_colors[1][1],
        grad_color2_b: grad_colors[1][2],
        grad_color2_a: grad_colors[1][3],
        grad_center_x: grad_center[0],
        grad_center_y: grad_center[1],
        grad_radius,
        _grad_pad: 0.0,

        ls_stroke_width: layer.style.stroke.size,
        ls_stroke_r: layer.style.stroke.color[0],
        ls_stroke_g: layer.style.stroke.color[1],
        ls_stroke_b: layer.style.stroke.color[2],
        ls_color_overlay_r: layer.style.color_overlay.color[0],
        ls_color_overlay_g: layer.style.color_overlay.color[1],
        ls_color_overlay_b: layer.style.color_overlay.color[2],
        ls_color_overlay_a: layer.style.color_overlay.opacity / 100.0,
        ls_gradient_start_x: 0.0,
        ls_gradient_start_y: 0.0,
        ls_gradient_end_x: 1.0,
        ls_gradient_end_y: 1.0,
        ls_gradient_color1_r: layer.style.gradient_overlay.color_start[0],
        ls_gradient_color1_g: layer.style.gradient_overlay.color_start[1],
        ls_gradient_color1_b: layer.style.gradient_overlay.color_start[2],
        ls_gradient_color1_a: layer.style.gradient_overlay.color_start[3]
            * (layer.style.gradient_overlay.opacity / 100.0),
        ls_gradient_color2_r: layer.style.gradient_overlay.color_end[0],
        ls_gradient_color2_g: layer.style.gradient_overlay.color_end[1],
        ls_gradient_color2_b: layer.style.gradient_overlay.color_end[2],
        ls_gradient_color2_a: layer.style.gradient_overlay.color_end[3]
            * (layer.style.gradient_overlay.opacity / 100.0),
        ls_inner_shadow_offset_x: {
            let rad = layer.style.inner_shadow.angle.to_radians();
            rad.cos() * layer.style.inner_shadow.distance
        },
        ls_inner_shadow_offset_y: {
            let rad = layer.style.inner_shadow.angle.to_radians();
            rad.sin() * layer.style.inner_shadow.distance
        },
        ls_inner_shadow_size: layer.style.inner_shadow.size,
        ls_inner_shadow_opacity: layer.style.inner_shadow.opacity / 100.0,
        ls_inner_shadow_r: layer.style.inner_shadow.color[0],
        ls_inner_shadow_g: layer.style.inner_shadow.color[1],
        ls_inner_shadow_b: layer.style.inner_shadow.color[2],
        ls_bevel_size: layer.style.bevel_emboss.size,
        ls_bevel_angle: layer.style.bevel_emboss.angle,
        ls_bevel_strength: layer.style.bevel_emboss.highlight,
        ls_bevel_light_r: layer.style.bevel_emboss.color_light[0],
        ls_bevel_light_g: layer.style.bevel_emboss.color_light[1],
        ls_bevel_light_b: layer.style.bevel_emboss.color_light[2],
        ls_style_flags: {
            let mut flags = 0u32;
            if layer.style.stroke.enabled {
                flags |= 1;
            }
            if layer.style.color_overlay.enabled {
                flags |= 2;
            }
            if layer.style.gradient_overlay.enabled {
                flags |= 4;
            }
            if layer.style.inner_shadow.enabled {
                flags |= 8;
            }
            if layer.style.bevel_emboss.enabled {
                flags |= 16;
            }
            flags
        },
        _ls_pad1: 0.0,
        _ls_pad2: 0.0,
        _ls_pad3: 0.0,

        extrusion_depth: match &layer.layer_type {
            crate::core::timeline::LayerType::Shape {
                extrusion_depth, ..
            } => *extrusion_depth,
            _ => 0.0,
        },
        bevel_depth: match &layer.layer_type {
            crate::core::timeline::LayerType::Shape { bevel_depth, .. } => *bevel_depth,
            _ => 0.0,
        },

        // ── GPU Real-time VFX Shader Extensions ──
        chromatic_enabled: ep.chromatic_enabled,
        chromatic_amount: ep.chromatic_amount,
        chromatic_angle: ep.chromatic_angle,
        _pad_chromatic: 0.0,

        vignette_enabled: ep.vignette_enabled,
        vignette_amount: ep.vignette_amount,
        vignette_midpoint: ep.vignette_midpoint,
        vignette_feather: ep.vignette_feather,

        invert_enabled: ep.invert_enabled,
        posterize_enabled: ep.posterize_enabled,
        posterize_levels: ep.posterize_levels,
        threshold_level: ep.threshold_level,

        tint_enabled: ep.tint_enabled,
        tint_amount: ep.tint_amount,
        _pad_tint1: 0.0,
        _pad_tint2: 0.0,
        tint_black: ep.tint_black,
        tint_white: ep.tint_white,

        crt_enabled: ep.crt_enabled,
        crt_scanline_count: ep.crt_scanline_count,
        crt_scanline_intensity: ep.crt_scanline_intensity,
        crt_curvature: ep.crt_curvature,

        sim_enabled: ep.sim_enabled,
        sim_type: ep.sim_type,
        sim_p1: ep.sim_p1,
        sim_p2: ep.sim_p2,
        sim_p3: ep.sim_p3,
        sim_p4: ep.sim_p4,
        sim_p5: ep.sim_p5,
        sim_p6: ep.sim_p6,

        _padding_align: [0.0; 4],
        _ls_pad4: 0.0,
        _ls_pad5: 0.0,
    };

    layer_uniform
}
