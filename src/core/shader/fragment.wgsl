fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var blur_extend = 0.0;
    if (layer.effect_blur_enabled == 1u) {
        blur_extend = layer.effect_blur_radius * 0.01;
    }

    var final_color: vec4<f32>;
    if (layer.effect_ca_enabled == 1u) {
        let edge_dist = length(in.local_pos) * 2.0;
        let falloff = pow(clamp(edge_dist, 0.0, 1.0), max(layer.effect_ca_edge_falloff * 4.0, 0.1));

        let texel_size = 1.0 / globals.viewport_size;

        let dir = normalize(in.local_pos + vec2<f32>(0.0001, 0.0001));
        let r_offset_local = dir * layer.effect_ca_shift_r * falloff * 0.01;
        let b_offset_local = dir * (-layer.effect_ca_shift_b) * falloff * 0.01;

        let r_local = in.local_pos - r_offset_local;
        let b_local = in.local_pos - b_offset_local;

        let r_tc = in.tex_coords - r_offset_local.x * dpdx(in.tex_coords) - r_offset_local.y * dpdy(in.tex_coords);
        let b_tc = in.tex_coords - b_offset_local.x * dpdx(in.tex_coords) - b_offset_local.y * dpdy(in.tex_coords);

        let col_r = sample_layer_color(r_local, r_tc, blur_extend);
        let col_g = sample_layer_color(in.local_pos, in.tex_coords, blur_extend);
        let col_b = sample_layer_color(b_local, b_tc, blur_extend);

        final_color = vec4<f32>(col_r.r, col_g.g, col_b.b, col_g.a);
    } else {
        final_color = sample_layer_color(in.local_pos, in.tex_coords, blur_extend);
    }

    // Software isolation applies the mask before layer effects, then reapplies
    // it after effects to contain generated pixels. Mirror the first pass here.
    if (layer.mask_enabled == 1u) {
        let mask_pixel = vec2<i32>(in.clip_position.xy);
        final_color.a *= textureLoad(t_mask, mask_pixel, 0).a;
    }

    // --- Color Tint ---
    if (layer.effect_tint_enabled == 1u) {
        let tint_color = round(
            clamp(
                vec3<f32>(layer.effect_tint_color_r, layer.effect_tint_color_g, layer.effect_tint_color_b),
                vec3<f32>(0.0),
                vec3<f32>(1.0)
            ) * 255.0
        ) / 255.0;
        let tint_amount = clamp(layer.effect_tint_intensity, 0.0, 1.0);
        let tinted_rgb = mix(final_color.rgb, tint_color, tint_amount);
        final_color = vec4<f32>(tinted_rgb, final_color.a);
    }

    // --- Drop Shadow ---
    if (layer.effect_shadow_enabled == 1u) {
        let rad = -layer.effect_shadow_direction * 0.01745329251;
        let offset_pixels = vec2<f32>(cos(rad), sin(rad)) * layer.effect_shadow_distance;
        
        let local_dx = dpdx(in.local_pos);
        let local_dy = dpdy(in.local_pos);
        let local_offset = offset_pixels.x * local_dx + offset_pixels.y * local_dy;
        let shadow_local_pos = in.local_pos - local_offset;
        
        let tex_dx = dpdx(in.tex_coords);
        let tex_dy = dpdy(in.tex_coords);
        let tex_offset = offset_pixels.x * tex_dx + offset_pixels.y * tex_dy;
        let shadow_tex_coords = in.tex_coords - tex_offset;
        
        let shadow_blur_extend = layer.effect_shadow_softness * 0.01;
        
        var shadow_alpha = 0.0;
        if (layer.layer_type == 0u) {
            let d_x = abs(shadow_local_pos.x) - 0.5;
            let d_y = abs(shadow_local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            shadow_alpha = 1.0 - smoothstep(-0.02 - shadow_blur_extend, 0.0 + shadow_blur_extend, d);
        } else if (layer.layer_type == 1u) {
            if (layer.effect_shadow_softness > 0.0) {
                let texel_size = 1.0 / globals.viewport_size;
                let offset = layer.effect_shadow_softness * texel_size * 0.5;
                var alpha_sum = sample_diffuse(shadow_tex_coords).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(-offset.x, -offset.y)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(offset.x, -offset.y)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(-offset.x, offset.y)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(offset.x, offset.y)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(-offset.x, 0.0)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(offset.x, 0.0)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(0.0, -offset.y)).a;
                alpha_sum += sample_diffuse(shadow_tex_coords + vec2<f32>(0.0, offset.y)).a;
                shadow_alpha = alpha_sum / 9.0;
            } else {
                shadow_alpha = sample_diffuse(shadow_tex_coords).a;
            }
        } else if (layer.layer_type == 2u) {
            shadow_alpha = shape_sdf_alpha(shadow_local_pos, shadow_blur_extend);
        } else if (layer.layer_type == 3u) {
            let d_x = abs(shadow_local_pos.x) - 0.5;
            let d_y = abs(shadow_local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            shadow_alpha = 1.0 - smoothstep(-0.02 - shadow_blur_extend, 0.0 + shadow_blur_extend, d);
        }
        
        let shadow_intensity = layer.effect_shadow_opacity / 100.0;
        let shadow_color = vec4<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b, shadow_alpha * shadow_intensity);
        
        let blended_rgb = final_color.rgb * final_color.a + shadow_color.rgb * shadow_color.a * (1.0 - final_color.a);
        let blended_alpha = final_color.a + shadow_color.a * (1.0 - final_color.a);
        final_color = vec4<f32>(blended_rgb / max(blended_alpha, 0.001), blended_alpha);
    }

    // --- Vignette ---
    if (layer.effect_vignette_enabled == 1u) {
        let uv = in.local_pos * 2.0;
        let r = layer.effect_vignette_roundness;
        let dist_sq = uv.x * uv.x * (1.0 - r * 0.5) + uv.y * uv.y * (1.0 - r * 0.5);
        let dist_circ = sqrt(uv.x * uv.x + uv.y * uv.y);
        let dist = mix(dist_sq, dist_circ, r);

        let feather = max(layer.effect_vignette_feather / 100.0, 0.001) * 1.5;
        let inner = 1.0 - feather;
        let vignette_factor = smoothstep(inner, inner + feather, dist);
        let vignette_strength = (layer.effect_vignette_intensity / 100.0) * vignette_factor;

        let vcolor = vec4<f32>(layer.effect_vignette_color_r, layer.effect_vignette_color_g, layer.effect_vignette_color_b, layer.effect_vignette_color_a);
        final_color = vec4<f32>(
            mix(final_color.rgb, vcolor.rgb, vignette_strength * vcolor.a),
            final_color.a
        );
    }

    // --- Levels Adjustment ---
    if (layer.levels_enabled == 1u) {
        let range = max(layer.levels_in_white - layer.levels_in_black, 0.001);
        let val_normalized = clamp((final_color.rgb - vec3<f32>(layer.levels_in_black)) / range, vec3<f32>(0.0), vec3<f32>(1.0));
        var gamma_power = 1.0;
        if (layer.levels_gamma > 0.0) {
            gamma_power = 1.0 / layer.levels_gamma;
        }
        let gamma_adjusted = pow(val_normalized, vec3<f32>(gamma_power));
        final_color = vec4<f32>(
            clamp(
                layer.levels_out_black + gamma_adjusted * (layer.levels_out_white - layer.levels_out_black),
                vec3<f32>(0.0),
                vec3<f32>(1.0)
            ),
            final_color.a
        );
    }

    // --- Hue / Saturation ---
    if (layer.huesat_enabled == 1u) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let desat = mix(vec3<f32>(luma), final_color.rgb, layer.huesat_sat);
        final_color = vec4<f32>(
            clamp(desat * layer.huesat_light, vec3<f32>(0.0), vec3<f32>(1.0)),
            final_color.a
        );
    }

    // --- Glow / Bloom ---
    if (layer.glow_enabled == 1u) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (luma >= layer.glow_threshold) {
            let highlight = (luma - layer.glow_threshold) / max(1.0 - layer.glow_threshold, 0.001);
            let bloom_rgb = vec3<f32>(layer.glow_color_r, layer.glow_color_g, layer.glow_color_b) * highlight * layer.glow_intensity;
            final_color = vec4<f32>(final_color.rgb + bloom_rgb, final_color.a);
        }
    }

    // --- Physical Film Grain Noise ---
    if (layer.grain_enabled == 1u) {
        let grain_uv = in.tex_coords * globals.viewport_size / max(layer.grain_size, 0.1);
        let grain_frame = fract(globals.exposure_ev * 0.1);
        let grain_uv_t = grain_uv + vec2<f32>(grain_frame * 43.758, grain_frame * 17.321);
        let n1 = fract(sin(dot(grain_uv_t, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let n2 = fract(sin(dot(grain_uv_t * 2.0, vec2<f32>(63.7264, 10.873))) * 23421.631);
        let n3 = fract(sin(dot(grain_uv_t * 0.5, vec2<f32>(45.164, 89.332))) * 65432.123);
        let grain_luma = (n1 * 0.60 + n2 * 0.25 + n3 * 0.15) - 0.5;
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let luma_weight = 1.0 - abs(luma - 0.5) * 1.5;
        let grain_r = grain_luma + (n2 - 0.5) * 0.15;
        let grain_g = grain_luma + (n3 - 0.5) * 0.15;
        let grain_b = grain_luma + (n1 - 0.5) * 0.15;
        let intensity = layer.grain_intensity * luma_weight;
        final_color = vec4<f32>(
            clamp(final_color.r + grain_r * intensity, 0.0, 1.0),
            clamp(final_color.g + grain_g * intensity, 0.0, 1.0),
            clamp(final_color.b + grain_b * intensity, 0.0, 1.0),
            final_color.a
        );
    }

    // --- Invert ---
    if (layer.invert_enabled == 1u) {
        final_color = vec4<f32>(
            1.0 - final_color.r,
            1.0 - final_color.g,
            1.0 - final_color.b,
            final_color.a
        );
    }

    // --- Posterize ---
    if (layer.posterize_enabled == 1u && layer.posterize_levels >= 2.0) {
        let l = floor(layer.posterize_levels);
        final_color = vec4<f32>(
            clamp(floor(final_color.rgb * l) / l, vec3<f32>(0.0), vec3<f32>(1.0)),
            final_color.a
        );
    }

    // --- Tint ---
    if (layer.tint_enabled == 1u && layer.tint_amount > 0.0) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let tb = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b);
        let tw = vec3<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b);
        let mapped = mix(tb, tw, luma);
        let tinted = mix(final_color.rgb, mapped, layer.tint_amount);
        final_color = vec4<f32>(tinted, final_color.a);
    }

    // --- Vignette / Lens Falloff ---
    if (layer.vignette_enabled == 1u && layer.vignette_amount > 0.0) {
        let center_dist = length(in.local_pos * 2.0);
        let vig_start = max(layer.vignette_midpoint, 0.0);
        let vig_feather = max(layer.vignette_feather, 0.01);
        let vig_factor = 1.0 - smoothstep(vig_start, vig_start + vig_feather, center_dist) * layer.vignette_amount;
        final_color = vec4<f32>(final_color.rgb * clamp(vig_factor, 0.0, 1.0), final_color.a);
    }

    // --- CRT Scanlines ---
    if (layer.crt_enabled == 1u) {
        let scan_count = max(layer.crt_scanline_count, 100.0);
        let scan_phase = in.tex_coords.y * scan_count * 3.14159265;
        let scanline = 0.5 + 0.5 * sin(scan_phase);
        let scan_mult = 1.0 - (1.0 - scanline) * layer.crt_scanline_intensity;
        final_color = vec4<f32>(final_color.rgb * clamp(scan_mult, 0.0, 1.0), final_color.a);
    }

    // Match the software renderer's evaluated flat-plane material lighting.
    if (layer.three_d_lighting_enabled == 1u) {
        final_color = vec4<f32>(
            final_color.rgb * vec3<f32>(layer.three_d_shade_r, layer.three_d_shade_g, layer.three_d_shade_b),
            final_color.a
        );
    }

    // ── Layer Styles ──
    if (layer.ls_style_flags != 0u && final_color.a > 0.001) {
        if (layer.ls_style_flags & 1u) != 0u {
            let stroke_col = vec3<f32>(layer.ls_stroke_r, layer.ls_stroke_g, layer.ls_stroke_b);
            final_color = ls_apply_stroke(final_color, final_color.a, stroke_col, layer.ls_stroke_width);
        }
        if (layer.ls_style_flags & 2u) != 0u {
            let overlay = vec4<f32>(layer.ls_color_overlay_r, layer.ls_color_overlay_g, layer.ls_color_overlay_b, layer.ls_color_overlay_a);
            final_color = ls_apply_color_overlay(final_color, overlay);
        }
        if (layer.ls_style_flags & 4u) != 0u {
            let start = vec2<f32>(layer.ls_gradient_start_x, layer.ls_gradient_start_y);
            let end = vec2<f32>(layer.ls_gradient_end_x, layer.ls_gradient_end_y);
            let c1 = vec4<f32>(layer.ls_gradient_color1_r, layer.ls_gradient_color1_g, layer.ls_gradient_color1_b, layer.ls_gradient_color1_a);
            let c2 = vec4<f32>(layer.ls_gradient_color2_r, layer.ls_gradient_color2_g, layer.ls_gradient_color2_b, layer.ls_gradient_color2_a);
            final_color = ls_apply_gradient_overlay(final_color, in.local_pos, start, end, c1, c2);
        }
        if (layer.ls_style_flags & 8u) != 0u {
            let is_offset = vec2<f32>(layer.ls_inner_shadow_offset_x, layer.ls_inner_shadow_offset_y);
            let is_color = vec3<f32>(layer.ls_inner_shadow_r, layer.ls_inner_shadow_g, layer.ls_inner_shadow_b);
            final_color = ls_apply_inner_shadow(final_color, final_color.a, is_offset, layer.ls_inner_shadow_size, is_color, layer.ls_inner_shadow_opacity);
        }
        if (layer.ls_style_flags & 16u) != 0u {
            let light_col = vec3<f32>(layer.ls_bevel_light_r, layer.ls_bevel_light_g, layer.ls_bevel_light_b);
            let dark_col = vec3<f32>(0.0, 0.0, 0.0);
            final_color = ls_apply_bevel(final_color, final_color.a, layer.ls_bevel_size, layer.ls_bevel_angle, layer.ls_bevel_strength, light_col, dark_col);
        }
    }


    // ── Shadow map ──
    if (globals.shadow_enabled == 1u && layer.accepts_shadows == 1u && final_color.a > 0.01) {
        // The shadow texture is a composition-space density map. Layer UVs
        // only identify points on the source layer and diverge after its
        // transform (especially for perspective 3D planes), so sample in the
        // fragment's output-pixel coordinate system instead.
        let shadow_uv = in.clip_position.xy / globals.viewport_size;
        let occ = textureSample(t_shadow, s_shadow, shadow_uv).r;
        let dark = 1.0 - min(occ, 1.0);
        final_color = vec4<f32>(final_color.rgb * select(1.0, dark, occ > 0.003), final_color.a);
    }

    // --- Layer Opacity ---
    final_color.a = final_color.a * (layer.opacity / 100.0);

    // --- GPU Layer Mask Compositing ---
    if (layer.mask_enabled == 1u) {
        var mask_alpha: f32 = 1.0;
        // The mask raster is in composition/output pixels, independent of
        // source-layer UVs and sampler filtering.
        let mask_pixel = vec2<i32>(in.clip_position.xy);
        if (layer.mask_mode == 1u) {
            mask_alpha = textureLoad(t_mask, mask_pixel, 0).a;
        } else if (layer.mask_mode == 2u) {
            mask_alpha = 1.0 - textureLoad(t_mask, mask_pixel, 0).a;
        } else if (layer.mask_mode == 3u) {
            let tex = textureLoad(t_mask, mask_pixel, 0);
            mask_alpha = dot(tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        } else if (layer.mask_mode == 4u) {
            let tex = textureLoad(t_mask, mask_pixel, 0);
            mask_alpha = 1.0 - dot(tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        }
        if (layer.mask_inverted == 1u) {
            mask_alpha = 1.0 - mask_alpha;
        }
        if (layer.mask_feather > 0.01) {
            mask_alpha = smoothstep(0.0, layer.mask_feather, mask_alpha * layer.mask_feather);
        }
        final_color.a = final_color.a * mask_alpha;
    }

    // --- Track Matte Masking ---
    if (layer.track_matte_mode > 0u) {
        let matte_tex = textureSample(t_matte, s_matte, in.tex_coords);
        var matte_alpha = 1.0;
        if (layer.track_matte_mode == 1u) {
            matte_alpha = matte_tex.a;
        } else if (layer.track_matte_mode == 2u) {
            matte_alpha = 1.0 - matte_tex.a;
        } else if (layer.track_matte_mode == 3u) {
            matte_alpha = dot(matte_tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        } else if (layer.track_matte_mode == 4u) {
            matte_alpha = 1.0 - dot(matte_tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        }
        final_color = vec4<f32>(final_color.rgb, final_color.a * matte_alpha);
    }

    // --- Viewport Exposure Adjustment ---
    final_color = vec4<f32>(final_color.rgb * pow(2.0, globals.exposure_ev), final_color.a);

    // --- Viewport LUT Color Management ---
    if (globals.lut_mode == 1u) {
        final_color = vec4<f32>(pow(final_color.rgb, vec3<f32>(2.2)), final_color.a);
    } else if (globals.lut_mode == 2u) {
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        let aces = clamp((final_color.rgb * (a * final_color.rgb + vec3<f32>(b))) / (final_color.rgb * (c * final_color.rgb + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
        final_color = vec4<f32>(aces, final_color.a);
    }

    if (layer.blend_mode == 2u) {
        final_color = vec4<f32>(final_color.rgb * final_color.a, final_color.a);
    }

    if (final_color.a <= 0.001) {
        discard;
    }

    return final_color;
}
