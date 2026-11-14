fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = layer.transform_matrix * vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    out.local_pos = model.position; // Ranges in [-0.5, 0.5]
    return out;
}

fn trim_shape_alpha(angle: f32, trim_start: f32, trim_end: f32, trim_offset: f32) -> f32 {
    if (trim_start == 0.0 && trim_end == 1.0) {
        return 1.0;
    }
    let normalized = fract(angle / 6.2831853 + 1.0);
    let start = fract(trim_start + trim_offset);
    let end = fract(trim_end + trim_offset);
    // AE treats a zero-length trim as an empty path, not a wrapped full path.
    if (abs(start - end) < 0.000001) {
        return 0.0;
    }
    if (start < end) {
        return select(0.0, 1.0, normalized >= start && normalized <= end);
    } else {
        return select(0.0, 1.0, normalized >= start || normalized <= end);
    }
}

// Helper: SDF-based alpha coverage for Shape layers (layer_type == 2u).
// Shared between the layer fill and the drop-shadow mask so all four
// shape types (rect / ellipse / star / polygon) cast correct shadows.
fn shape_sdf_alpha(local_pos_in: vec2<f32>, blur_extend: f32) -> f32 {
    var alpha = 0.0;
    if (layer.shape_type == 0u) {
        let d_x = abs(local_pos_in.x) - 0.5;
        let d_y = abs(local_pos_in.y) - 0.5;
        let d = max(d_x, d_y);
        alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.02 + blur_extend, d);
    } else if (layer.shape_type == 1u) {
        let dist = length(local_pos_in);
        alpha = 1.0 - smoothstep(0.48 - blur_extend, 0.5 + blur_extend, dist);
    } else if (layer.shape_type == 2u) {
        // Procedural N-Point Star SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params_x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        let d_star = r * cos(angle_mod - 0.314159) - 0.45;
        alpha = 1.0 - smoothstep(-0.04 - blur_extend, 0.04 + blur_extend, d_star);
    } else if (layer.shape_type == 3u) {
        // Procedural Regular N-Gon Polygon SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params_x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        let d_poly = r * cos(angle_mod) - 0.45;
        alpha = 1.0 - smoothstep(-0.04 - blur_extend, 0.04 + blur_extend, d_poly);
    } else {
        alpha = 1.0;
    }
    let angle = atan2(local_pos_in.y, local_pos_in.x);
    return alpha * trim_shape_alpha(angle, layer.trim_start, layer.trim_end, layer.trim_offset);
}

// ─── GPU Procedural Noise & fBM Helpers ───

fn gpu_hash21(p: vec2<f32>) -> f32 {
    let q = fract(sin(vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)))) * 43758.5453);
    return fract(q.x + q.y);
}

fn gpu_noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = gpu_hash21(i + vec2<f32>(0.0, 0.0));
    let b = gpu_hash21(i + vec2<f32>(1.0, 0.0));
    let c = gpu_hash21(i + vec2<f32>(0.0, 1.0));
    let d = gpu_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn gpu_fbm2d(p: vec2<f32>, octaves: i32) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var pt = p;
    for (var i = 0; i < 6; i = i + 1) {
        if (i >= octaves) { break; }
        v += gpu_noise2d(pt * freq) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    return v;
}

// Helper: sample layer color at a given local_pos and tex_coords
fn sample_diffuse(tc: vec2<f32>) -> vec4<f32> {
    // Diffuse textures are stored premultiplied so hardware bilinear filtering
    // cannot pull hidden RGB from transparent texels into visible edges.
    var premultiplied = textureSample(t_diffuse, s_diffuse, tc);
    if (layer.video_frame_blend > 0.0) {
        let second = textureSample(t_diffuse_next, s_diffuse_next, tc);
        premultiplied = mix(premultiplied, second, clamp(layer.video_frame_blend, 0.0, 1.0));
    }
    if (premultiplied.a <= 0.000001) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(premultiplied.rgb / premultiplied.a, premultiplied.a);
}

fn sample_layer_color(local_pos_in: vec2<f32>, tc_in: vec2<f32>, blur_extend: f32) -> vec4<f32> {
    // Mesh Warp / Corner Pin: bilinear corner-offset displacement field.
    // Corners are pixel offsets from each quad corner; normalize by viewport to get UV deltas.
    var tc = tc_in;
    var local_pos = local_pos_in;

    // ── GPU Spatial Simulation Distortion (Twirl, Bulge, Spherize, Wave Warp, Turbulent Displace) ──
    if (layer.sim_enabled == 1u) {
        let center = vec2<f32>(0.5, 0.5);
        let uv_rel = tc - center;
        let dist = length(uv_rel);

        if (layer.sim_type == 2u) { // Turbulent Displace
            let amount = layer.sim_p1 * 0.05;
            let size = max(layer.sim_p2, 1.0);
            let n_x = gpu_noise2d((tc + vec2<f32>(layer.sim_p3, 0.0)) * size) - 0.5;
            let n_y = gpu_noise2d((tc + vec2<f32>(0.0, layer.sim_p3 + 12.3)) * size) - 0.5;
            tc += vec2<f32>(n_x, n_y) * amount;
        } else if (layer.sim_type == 3u) { // Wave Warp
            let freq = layer.sim_p1;
            let amp = layer.sim_p2 * 0.01;
            let phase = layer.sim_p3;
            tc.x += sin(tc.y * freq + phase) * amp;
        } else if (layer.sim_type == 4u) { // Twirl
            let radius = max(layer.sim_p1, 0.001);
            let angle = layer.sim_p2 * 0.0174533; // deg to rad
            if (dist < radius) {
                let factor = (1.0 - dist / radius);
                let a = angle * factor * factor;
                let sin_a = sin(a);
                let cos_a = cos(a);
                let rotated = vec2<f32>(
                    uv_rel.x * cos_a - uv_rel.y * sin_a,
                    uv_rel.x * sin_a + uv_rel.y * cos_a
                );
                tc = center + rotated;
            }
        } else if (layer.sim_type == 5u) { // Bulge
            let radius = max(layer.sim_p1, 0.001);
            let amount = layer.sim_p2;
            if (dist < radius) {
                let factor = 1.0 - dist / radius;
                let displace = 1.0 - amount * factor * factor;
                tc = center + uv_rel * displace;
            }
        } else if (layer.sim_type == 6u) { // Spherize
            let radius = max(layer.sim_p1, 0.001);
            if (dist < radius) {
                let d_norm = dist / radius;
                let z = sqrt(max(1.0 - d_norm * d_norm, 0.0));
                let r = (1.0 - z) * 0.5 + d_norm * 0.5;
                tc = center + normalize(uv_rel) * (r * radius);
            }
        } else if (layer.sim_type == 7u) { // Heat Distortion
            let strength = layer.sim_p1 * 0.02;
            let speed = layer.sim_p2;
            let n = sin(tc.y * 40.0 + speed * 6.0) * cos(tc.x * 30.0 + speed * 4.0);
            tc.x += n * strength;
        }
    }

    if (layer.meshwarp_enabled == 1u) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let d_tl = vec2<f32>(layer.corner_top_left_x, layer.corner_top_left_y) / vp;
        let d_tr = vec2<f32>(layer.corner_top_right_x, layer.corner_top_right_y) / vp;
        let d_bl = vec2<f32>(layer.corner_bottom_left_x, layer.corner_bottom_left_y) / vp;
        let d_br = vec2<f32>(layer.corner_bottom_right_x, layer.corner_bottom_right_y) / vp;
        let u = tc.x;
        let v = tc.y;
        let top = mix(d_tl, d_tr, u);
        let bot = mix(d_bl, d_br, u);
        let disp = mix(top, bot, v);
        // One-step inverse approximation: sample opposite the displacement
        tc = vec2<f32>(tc.x - disp.x, tc.y + disp.y);
        local_pos = vec2<f32>(local_pos.x - disp.x * 2.0, local_pos.y + disp.y * 2.0);
    }
    var c = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    if (layer.layer_type == 0u) {
        let layer_color = vec4<f32>(layer.color_r, layer.color_g, layer.color_b, layer.color_a);
        if (layer.effect_blur_enabled == 1u) {
            let d_x = abs(local_pos.x) - 0.5;
            let d_y = abs(local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            let alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.0 + blur_extend, d);
            c = vec4<f32>(layer_color.rgb, layer_color.a * alpha);
        } else if (layer.shape_params_y > 0.001) {
            // Rounded-corner solid via SDF
            let half_sz = vec2<f32>(0.5, 0.5);
            let q = abs(local_pos) - (half_sz - layer.shape_params_y);
            let d = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - layer.shape_params_y;
            let alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.02 + blur_extend, d);
            c = vec4<f32>(layer_color.rgb, layer_color.a * alpha);
        } else {
            c = layer_color;
        }
    } else if (layer.layer_type == 1u) {
        if (layer.effect_blur_enabled == 1u) {
            // --- Gaussian Blur (13-tap single-pass separable approximation) ---
            let texel_size = 1.0 / globals.viewport_size;
            let offset = layer.effect_blur_radius * texel_size;

            var color_sum = sample_diffuse(tc) * 0.1111;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x, 0.0)) * 0.1053;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x, 0.0)) * 0.1053;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            color_sum += sample_diffuse(tc + vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            color_sum += sample_diffuse(tc - vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y)) * 0.1053;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y)) * 0.1053;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            color_sum += sample_diffuse(tc + vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            color_sum += sample_diffuse(tc - vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            c = color_sum;
        } else {
            c = sample_diffuse(tc);
        }
    } else if (layer.layer_type == 2u) {
        let alpha = shape_sdf_alpha(local_pos, blur_extend);
        let layer_color = vec4<f32>(layer.color_r, layer.color_g, layer.color_b, layer.color_a);
        var shape_color = layer_color;
        if (layer.fill_type == 1u) {
            let d = vec2<f32>(layer.grad_end_x - layer.grad_start_x, layer.grad_end_y - layer.grad_start_y);
            let len_sq = dot(d, d);
            if (len_sq > 0.001) {
                let t = clamp(dot(local_pos - vec2<f32>(layer.grad_start_x, layer.grad_start_y), d) / len_sq, 0.0, 1.0);
                shape_color = mix(vec4<f32>(layer.grad_color1_r, layer.grad_color1_g, layer.grad_color1_b, layer.grad_color1_a), vec4<f32>(layer.grad_color2_r, layer.grad_color2_g, layer.grad_color2_b, layer.grad_color2_a), t);
            }
        } else if (layer.fill_type == 2u) {
            let center = vec2<f32>(layer.grad_center_x, layer.grad_center_y);
            let dist = length(local_pos - center);
            let t = clamp(dist / max(layer.grad_radius, 0.001), 0.0, 1.0);
            shape_color = mix(vec4<f32>(layer.grad_color1_r, layer.grad_color1_g, layer.grad_color1_b, layer.grad_color1_a), vec4<f32>(layer.grad_color2_r, layer.grad_color2_g, layer.grad_color2_b, layer.grad_color2_a), t);
        }
        c = vec4<f32>(shape_color.rgb, shape_color.a * alpha);

        // 3D Extrusion: pseudo-3D depth shading based on distance from shape edge
        if (layer.extrusion_depth > 0.01 && alpha > 0.01) {
            var edge_dist = 0.0;
            if (layer.shape_type == 0u) {
                let d_x = abs(local_pos.x) - 0.5;
                let d_y = abs(local_pos.y) - 0.5;
                edge_dist = -max(d_x, d_y);
            } else if (layer.shape_type == 1u) {
                edge_dist = 0.5 - length(local_pos);
            } else {
                edge_dist = 0.1;
            }
            edge_dist = clamp(edge_dist * 2.0, 0.0, 1.0);

            let depth_factor = 1.0 - layer.extrusion_depth * 0.003;
            let extrusion_shade = mix(depth_factor, 1.0, edge_dist);

            let bevel_factor = 1.0 + layer.bevel_depth * 0.02 * (1.0 - smoothstep(0.0, 0.3, edge_dist));

            let final_shade = clamp(extrusion_shade * bevel_factor, 0.2, 1.0);
            c = vec4<f32>(c.rgb * final_shade, c.a);
        }
    } else if (layer.layer_type == 3u) {
        c = vec4<f32>(layer.color_r, layer.color_g, layer.color_b, layer.color_a);
    } else if (layer.layer_type == 8u) {
        c = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // ── GPU Procedural Fractal Noise Generator ──
    if (layer.sim_enabled == 1u && layer.sim_type == 1u) {
        let contrast = layer.sim_p1;
        let brightness = layer.sim_p2;
        let evolution = layer.sim_p3;
        let complexity = clamp(i32(layer.sim_p4), 1, 6);
        let n_val = gpu_fbm2d(tc * 6.0 + vec2<f32>(evolution * 0.5, evolution * 0.3), complexity);
        let f_col = clamp((n_val - 0.5) * contrast + 0.5 + brightness, 0.0, 1.0);
        c = vec4<f32>(f_col, f_col, f_col, 1.0);
    }

    /// ── Vignette: darkened edges ──
    if (layer.effect_vignette_enabled == 1u) {
        let center_uv = vec2<f32>(0.5, 0.5);
        let d = distance(local_pos, center_uv) * 2.0;
        let vig = 1.0 - smoothstep(
            1.0 - layer.effect_vignette_feather,
            1.0,
            d * (1.0 - layer.effect_vignette_roundness * 0.3)
        );
        let vig_amount = layer.effect_vignette_intensity;
        let vig_color = vec3<f32>(layer.effect_vignette_color_r, layer.effect_vignette_color_g, layer.effect_vignette_color_b);
        let vig_alpha = layer.effect_vignette_color_a;
        c = vec4<f32>(
            mix(c.rgb, vig_color * c.rgb, (1.0 - vig) * vig_amount),
            c.a
        );
    }

    // ── Chromatic Aberration: radial RGB separation ──
    if (layer.effect_ca_enabled == 1u) {
        let center = vec2<f32>(0.5, 0.5);
        let dir = normalize(tc - center);
        let shift_r = tc + dir * vec2<f32>(layer.effect_ca_shift_r / globals.viewport_size.x);
        let shift_b = tc + dir * vec2<f32>(layer.effect_ca_shift_b / globals.viewport_size.y);
        let r = sample_diffuse(shift_r).r;
        let b = sample_diffuse(shift_b).b;
        c = vec4<f32>(r, c.g, b, c.a);
    }

    // ── Motion Blur: directional blur based on per-pixel velocity ──
    if (layer.motionblur_enabled == 1u) {
        let vel = vec2<f32>(layer.motionblur_velocity_x, layer.motionblur_velocity_y);
        let speed = length(vel);
        if (speed > 0.01) {
            let texel = vec2<f32>(1.0) / globals.viewport_size;
            let dir = normalize(vel) * texel;
            let max_offset = speed * layer.motionblur_shutter;
            let samples = max(layer.motionblur_samples, 1u);
            var blur_color = c;
            var blur_weight = 1.0;
            for (var s = 1u; s <= samples; s = s + 1u) {
                let t = f32(s) / f32(samples);
                let offset = dir * max_offset * t;
                blur_color += sample_diffuse(tc + offset);
                blur_weight += 1.0;
            }
            for (var s = 1u; s <= samples; s = s + 1u) {
                let t = f32(s) / f32(samples);
                let offset = dir * max_offset * t;
                blur_color += sample_diffuse(tc - offset);
                blur_weight += 1.0;
            }
            c = blur_color / blur_weight;
        }
    }

    // ── Glow: screen-space bloom from bright areas ──
    if (layer.glow_enabled == 1u && c.a > 0.01) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let texel = vec2<f32>(1.0) / vp;
        let gr = layer.glow_radius * texel;
        let thresh = max(layer.glow_threshold, 0.001);

        var bloom = vec3<f32>(0.0);
        var bloom_weight = 0.0;

        for (var s = 0; s < 8; s = s + 1) {
            let angle = f32(s) * 0.785398;
            let offset = vec2<f32>(cos(angle), sin(angle)) * gr;
            let sc = sample_diffuse(tc + offset);
            let luma = dot(sc.rgb, vec3<f32>(0.299, 0.587, 0.114));
            if (luma > thresh) {
                let contribution = (luma - thresh) / max(luma, 0.001);
                bloom += sc.rgb * contribution;
                bloom_weight += contribution;
            }
        }

        for (var s = 0; s < 8; s = s + 1) {
            let angle = f32(s) * 0.785398 + 0.392699;
            let offset = vec2<f32>(cos(angle), sin(angle)) * gr * 2.0;
            let sc = sample_diffuse(tc + offset);
            let luma = dot(sc.rgb, vec3<f32>(0.299, 0.587, 0.114));
            if (luma > thresh) {
                let contribution = (luma - thresh) / max(luma, 0.001) * 0.5;
                bloom += sc.rgb * contribution;
                bloom_weight += contribution;
            }
        }

        if (bloom_weight > 0.01) {
            bloom = bloom / max(bloom_weight, 1.0) * layer.glow_intensity;
        }

        let gc = vec3<f32>(layer.glow_color_r, layer.glow_color_g, layer.glow_color_b);
        let gc_lum = max(dot(gc, vec3<f32>(0.333)), 0.001);
        c = vec4<f32>(c.r + bloom.r * gc.r / gc_lum, c.g + bloom.g * gc.g / gc_lum, c.b + bloom.b * gc.b / gc_lum, c.a);
    }

    // ── Lens Flare ──
    if (layer.flare_enabled == 1u && layer.flare_intensity > 0.0) {
        let flare_center = clamp(
            vec2<f32>(layer.flare_pos_x, layer.flare_pos_y),
            vec2<f32>(0.0),
            vec2<f32>(1.0)
        );
        let d_norm = distance(tc_in, flare_center) * 2.0;

        let core_radius = 0.02;
        let core = exp(-d_norm * d_norm / (core_radius * core_radius)) * layer.flare_intensity;
        let ring_phase = d_norm * 12.0;
        let ring = sin(ring_phase) * 0.3 + 0.5;
        let ring_mask = exp(-d_norm * 3.0);
        let rings = ring * ring_mask * layer.flare_intensity * 0.3;
        let to_center = tc_in - flare_center;
        let streak_h = exp(-abs(to_center.y) * 80.0) * exp(-abs(to_center.x) * 8.0);
        let streak_v = exp(-abs(to_center.x) * 80.0) * exp(-abs(to_center.y) * 8.0);
        let streaks = (streak_h + streak_v) * layer.flare_intensity * 0.4;
        let flare_total = min((core + rings + streaks) * layer.flare_threshold, 4.0);

        if (flare_total >= 0.002) {
            let flare_color = floor(
                clamp(
                    vec3<f32>(layer.flare_color_r, layer.flare_color_g, layer.flare_color_b),
                    vec3<f32>(0.0),
                    vec3<f32>(1.0)
                ) * 255.0
            );
            let flare_pixels = floor(clamp(c.rgb * 255.0 + flare_color * flare_total, vec3<f32>(0.0), vec3<f32>(255.0)));
            c = vec4<f32>(flare_pixels / 255.0, c.a);
        }
    }

    return c;
}

// ── Layer Style Helpers ────────────────────────────────────────────────────

