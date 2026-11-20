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

        let col_r = sample_layer_color(r_local, r_tc, blur_extend, in.clip_position.xy);
        let col_g = sample_layer_color(in.local_pos, in.tex_coords, blur_extend, in.clip_position.xy);
        let col_b = sample_layer_color(b_local, b_tc, blur_extend, in.clip_position.xy);

        final_color = vec4<f32>(col_r.r, col_g.g, col_b.b, col_g.a);
    } else {
        final_color = sample_layer_color(in.local_pos, in.tex_coords, blur_extend, in.clip_position.xy);
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
    } else if (layer.invert_enabled == 2u) {
        final_color = vec4<f32>(
            1.0 - final_color.r,
            1.0 - final_color.g,
            1.0 - final_color.b,
            1.0 - final_color.a
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

    if (layer.blend_mode == 255u) {
        if (final_color.a <= 0.001) {
            discard;
        }
        return vec4<f32>(
            round(clamp(final_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0) / 255.0,
            final_color.a
        );
    }

    // --- Viewport Exposure Adjustment ---
    let pre_grade_color = final_color;
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
    } else if (globals.lut_mode == 3u) {
        let linear = max(final_color.rgb, vec3<f32>(0.0));
        let lo = linear * 12.92;
        let hi = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
        let srgb = select(hi, lo, linear <= vec3<f32>(0.0031308));
        final_color = vec4<f32>(clamp(srgb, vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
    }

    if (globals.blend_linear == 1u && layer.layer_type != 7u) {
        final_color = vec4<f32>(srgb_to_linear_piecewise_vec3(final_color.rgb), final_color.a);
    }

    if (globals.working_color_space > 0u && layer.blend_mode != 255u && layer.layer_type != 7u) {
        let lin = srgb_to_linear_piecewise_vec3(final_color.rgb);
        if (globals.working_color_space == 1u) {
            // Display P3 primaries
            let p3 = vec3<f32>(
                dot(lin, vec3<f32>(0.822462, 0.177538, 0.0)),
                dot(lin, vec3<f32>(0.033194, 0.966806, 0.0)),
                dot(lin, vec3<f32>(0.017083, 0.072397, 0.910520))
            );
            final_color = vec4<f32>(clamp(p3, vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
        } else if (globals.working_color_space == 2u) {
            // Rec.2020 primaries
            let rec2020 = vec3<f32>(
                dot(lin, vec3<f32>(0.627404, 0.329283, 0.043313)),
                dot(lin, vec3<f32>(0.069097, 0.919540, 0.011362)),
                dot(lin, vec3<f32>(0.016391, 0.088013, 0.895595))
            );
            final_color = vec4<f32>(clamp(rec2020, vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
        } else if (globals.working_color_space == 3u) {
            // Linear sRGB
            final_color = vec4<f32>(lin, final_color.a);
        } else if (globals.working_color_space == 4u) {
            // ACEScg (AP1) primaries
            let ap1 = vec3<f32>(
                dot(lin, vec3<f32>(0.613097, 0.339523, 0.047379)),
                dot(lin, vec3<f32>(0.070194, 0.916354, 0.013452)),
                dot(lin, vec3<f32>(0.020616, 0.109570, 0.869815))
            );
            final_color = vec4<f32>(clamp(ap1, vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
        }
    }

    if (layer.blend_mode == 1u || layer.blend_mode == 2u) {
        final_color = vec4<f32>(final_color.rgb * final_color.a, final_color.a);
    }

    if (final_color.a <= 0.001) {
        discard;
    }

    if (globals.channel_mode == 1u) {
        final_color = vec4<f32>(vec3<f32>(final_color.r), 1.0);
    } else if (globals.channel_mode == 2u) {
        final_color = vec4<f32>(vec3<f32>(final_color.g), 1.0);
    } else if (globals.channel_mode == 3u) {
        final_color = vec4<f32>(vec3<f32>(final_color.b), 1.0);
    } else if (globals.channel_mode == 4u) {
        final_color = vec4<f32>(vec3<f32>(final_color.a), 1.0);
    } else if (globals.channel_mode == 5u) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        final_color = vec4<f32>(vec3<f32>(luma), 1.0);
    } else if (globals.channel_mode == 6u) {
        let stripe = step(0.5, fract((in.clip_position.x + in.clip_position.y) * 0.08));
        if (any(final_color.rgb > vec3<f32>(1.0))) {
            final_color = vec4<f32>(mix(final_color.rgb, vec3<f32>(1.0, 0.15, 0.15), stripe * 0.75), 1.0);
        } else if (any(final_color.rgb < vec3<f32>(0.0))) {
            final_color = vec4<f32>(mix(final_color.rgb, vec3<f32>(0.15, 0.45, 1.0), stripe * 0.75), 1.0);
        }
    }

    if (globals.wipe_split_x > 0.0 && (in.clip_position.x / max(globals.viewport_size.x, 1.0)) < globals.wipe_split_x) {
        final_color = pre_grade_color;
    }

    if (globals.dither_output == 1u) {
        let px = floor(in.clip_position.x);
        let py = floor(in.clip_position.y);
        let pixel_index = py * globals.viewport_size.x + px;
        let dither_seed = pixel_index * 0.618034;
        let noise = (fract(dither_seed * 7.13) - fract(dither_seed * 3.71)) / 255.0;
        final_color = vec4<f32>(
            clamp(final_color.rgb + vec3<f32>(noise), vec3<f32>(0.0), vec3<f32>(1.0)),
            final_color.a
        );
    }

    if (layer.blend_mode >= 22u && layer.blend_mode <= 25u) || layer.blend_mode == 27u {
        let dst = textureLoad(t_backdrop, vec2<i32>(in.clip_position.xy), 0);
        let dst_a = dst.a;
        let src_rgb = final_color.rgb;
        let src_a = final_color.a;
        if (layer.blend_mode == 22u) {
            // StencilAlpha
            return vec4<f32>(dst.rgb * src_a, dst_a * src_a);
        } else if (layer.blend_mode == 23u) {
            // StencilLuma
            let luma = clamp(dot(src_rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * src_a, 0.0, 1.0);
            return vec4<f32>(dst.rgb * luma, dst_a * luma);
        } else if (layer.blend_mode == 24u) {
            // SilhouetteAlpha
            let inv_a = 1.0 - src_a;
            return vec4<f32>(dst.rgb * inv_a, dst_a * inv_a);
        } else if (layer.blend_mode == 25u) {
            // SilhouetteLuma
            let luma = clamp(dot(src_rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * src_a, 0.0, 1.0);
            let inv_l = 1.0 - luma;
            return vec4<f32>(dst.rgb * inv_l, dst_a * inv_l);
        } else {
            // AlphaAdd (27u)
            let out_a = clamp(src_a + dst_a, 0.0, 1.0);
            let out_premul_rgb = clamp(src_rgb * src_a + dst.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
            return vec4<f32>(out_premul_rgb, out_a);
        }
    }

    if (layer.blend_mode == 3u
        || layer.blend_mode == 4u
        || (layer.blend_mode >= 5u && layer.blend_mode <= 21u)
        || layer.blend_mode == 28u)
    {
        let dst = textureLoad(t_backdrop, vec2<i32>(in.clip_position.xy), 0);
        let dst_a = dst.a;
        let dst_rgb = select(vec3<f32>(0.0), dst.rgb / dst_a, dst_a > 0.0001);
        let src_rgb = final_color.rgb;
        let src_a = final_color.a;
        let overlap_rgb = eval_programmable_blend(layer.blend_mode, src_rgb, dst_rgb);
        let blended_rgb = src_rgb * (1.0 - dst_a) + overlap_rgb * dst_a;
        let out_a = src_a + dst_a * (1.0 - src_a);
        let out_premul_rgb = blended_rgb * src_a + dst.rgb * (1.0 - src_a);
        return vec4<f32>(out_premul_rgb, out_a);
    }

    return final_color;
}

fn blend_overlay_ch(s: f32, d: f32) -> f32 {
    if (d < 0.5) { return 2.0 * s * d; }
    return 1.0 - 2.0 * (1.0 - s) * (1.0 - d);
}

fn blend_soft_light_ch(s: f32, d: f32) -> f32 {
    if (s <= 0.5) {
        return d - (1.0 - 2.0 * s) * d * (1.0 - d);
    }
    let curve = select(sqrt(max(d, 0.0)), ((16.0 * d - 12.0) * d + 4.0) * d, d <= 0.25);
    return d + (2.0 * s - 1.0) * (curve - d);
}

fn blend_hard_light_ch(s: f32, d: f32) -> f32 {
    if (s <= 0.5) { return 2.0 * s * d; }
    return 1.0 - 2.0 * (1.0 - s) * (1.0 - d);
}

fn blend_color_burn_ch(s: f32, d: f32) -> f32 {
    if (s <= 0.0) { return 0.0; }
    return clamp(1.0 - ((1.0 - d) / s), 0.0, 1.0);
}

fn blend_color_dodge_ch(s: f32, d: f32) -> f32 {
    if (s >= 1.0) { return 1.0; }
    return clamp(d / (1.0 - s), 0.0, 1.0);
}

fn blend_vivid_light_ch(s: f32, d: f32) -> f32 {
    if (s <= 0.5) {
        if (s <= 0.0) { return 0.0; }
        return clamp(1.0 - ((1.0 - d) / (2.0 * s)), 0.0, 1.0);
    }
    if (s >= 1.0) { return 1.0; }
    return clamp(d / (2.0 * (1.0 - s)), 0.0, 1.0);
}

fn blend_rgb_to_hsb(c: vec3<f32>) -> vec3<f32> {
    let max_c = max(c.r, max(c.g, c.b));
    let min_c = min(c.r, min(c.g, c.b));
    let delta = max_c - min_c;
    var h = 0.0;
    if (delta >= 0.001) {
        if (max_c == c.r) {
            let m = (c.g - c.b) / delta;
            h = 60.0 * (m - 6.0 * trunc(m / 6.0));
        } else if (max_c == c.g) {
            h = 60.0 * ((c.b - c.r) / delta + 2.0);
        } else {
            h = 60.0 * ((c.r - c.g) / delta + 4.0);
        }
    }
    if (h < 0.0) { h += 360.0; }
    let s = select(0.0, delta / max_c, max_c >= 0.001);
    return vec3<f32>(h, s, max_c);
}

fn blend_hsb_to_rgb(h_in: f32, s: f32, b: f32) -> vec3<f32> {
    let h = h_in - 360.0 * floor(h_in / 360.0);
    let c = b * s;
    let hd = h / 60.0;
    let h_mod2 = hd - 2.0 * trunc(hd / 2.0);
    let x = c * (1.0 - abs(h_mod2 - 1.0));
    let m = b - c;
    var rgb = vec3<f32>(c, 0.0, x);
    if (h < 60.0) {
        rgb = vec3<f32>(c, x, 0.0);
    } else if (h < 120.0) {
        rgb = vec3<f32>(x, c, 0.0);
    } else if (h < 180.0) {
        rgb = vec3<f32>(0.0, c, x);
    } else if (h < 240.0) {
        rgb = vec3<f32>(0.0, x, c);
    } else if (h < 300.0) {
        rgb = vec3<f32>(x, 0.0, c);
    }
    return rgb + vec3<f32>(m);
}

fn srgb_to_linear_piecewise_vec3(c: vec3<f32>) -> vec3<f32> {
    let v = max(c, vec3<f32>(0.0));
    let lo = v / 12.92;
    let hi = pow((v + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, v <= vec3<f32>(0.04045));
}

fn eval_programmable_blend(mode: u32, s: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    if (mode == 3u) {
        return vec3<f32>(blend_overlay_ch(s.r, d.r), blend_overlay_ch(s.g, d.g), blend_overlay_ch(s.b, d.b));
    } else if (mode == 4u) {
        return s * d;
    } else if (mode == 5u) {
        return min(s, d);
    } else if (mode == 6u) {
        return max(s, d);
    } else if (mode == 7u) {
        return vec3<f32>(blend_soft_light_ch(s.r, d.r), blend_soft_light_ch(s.g, d.g), blend_soft_light_ch(s.b, d.b));
    } else if (mode == 8u) {
        return vec3<f32>(blend_hard_light_ch(s.r, d.r), blend_hard_light_ch(s.g, d.g), blend_hard_light_ch(s.b, d.b));
    } else if (mode == 9u) {
        return abs(d - s);
    } else if (mode == 10u) {
        return s + d - 2.0 * s * d;
    } else if (mode == 11u) {
        return clamp(s / max(d, vec3<f32>(0.000001)), vec3<f32>(0.0), vec3<f32>(1.0));
    } else if (mode == 12u) {
        return max(s - d, vec3<f32>(0.0));
    } else if (mode == 13u) {
        return vec3<f32>(blend_color_burn_ch(s.r, d.r), blend_color_burn_ch(s.g, d.g), blend_color_burn_ch(s.b, d.b));
    } else if (mode == 14u) {
        return clamp(s + d - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0));
    } else if (mode == 15u) {
        return vec3<f32>(blend_color_dodge_ch(s.r, d.r), blend_color_dodge_ch(s.g, d.g), blend_color_dodge_ch(s.b, d.b));
    } else if (mode == 16u) {
        return vec3<f32>(blend_vivid_light_ch(s.r, d.r), blend_vivid_light_ch(s.g, d.g), blend_vivid_light_ch(s.b, d.b));
    } else if (mode == 28u) {
        return clamp(d + 2.0 * s - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0));
    } else if (mode == 18u) {
        let sh = blend_rgb_to_hsb(s);
        let dh = blend_rgb_to_hsb(d);
        return blend_hsb_to_rgb(sh.x, sh.y, dh.z);
    } else if (mode == 19u) {
        let sh = blend_rgb_to_hsb(s);
        let dh = blend_rgb_to_hsb(d);
        return blend_hsb_to_rgb(sh.x, dh.y, dh.z);
    } else if (mode == 20u) {
        let sh = blend_rgb_to_hsb(s);
        let dh = blend_rgb_to_hsb(d);
        return blend_hsb_to_rgb(dh.x, sh.y, dh.z);
    } else if (mode == 21u) {
        let sh = blend_rgb_to_hsb(s);
        let dh = blend_rgb_to_hsb(d);
        return blend_hsb_to_rgb(dh.x, dh.y, sh.z);
    }
    return s;
}
