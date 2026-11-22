fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = layer.transform_matrix * vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    out.local_pos = model.position; // Ranges in [-0.5, 0.5]
    return out;
}

fn pick_shift_channel(c: vec4<f32>, luma: f32, code: f32, def: f32) -> f32 {
    let idx = i32(round(code));
    if (idx == 0) { return c.r; }
    if (idx == 1) { return c.g; }
    if (idx == 2) { return c.b; }
    if (idx == 3) { return c.a; }
    if (idx == 4) { return luma; }
    if (idx == 5) { return 1.0; }
    if (idx == 6) { return 0.0; }
    return def;
}

fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let max_c = max(c.r, max(c.g, c.b));
    let min_c = min(c.r, min(c.g, c.b));
    let l = (max_c + min_c) * 0.5;
    let d = max_c - min_c;
    if (d < 0.00001) {
        return vec3<f32>(0.0, 0.0, l);
    }
    let s = select(d / (2.0 - max_c - min_c), d / (max_c + min_c), l < 0.5);
    var h = 0.0;
    if (max_c == c.r) {
        h = (c.g - c.b) / d + select(0.0, 6.0, c.g < c.b);
    } else if (max_c == c.g) {
        h = (c.b - c.r) / d + 2.0;
    } else {
        h = (c.r - c.g) / d + 4.0;
    }
    return vec3<f32>(h / 6.0, s, l);
}

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = fract(t_in + 1.0);
    if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 0.5) { return q; }
    if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    return p;
}

fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    if (hsl.y < 0.00001) {
        return vec3<f32>(hsl.z);
    }
    let q = select(hsl.z + hsl.y - hsl.z * hsl.y, hsl.z * (1.0 + hsl.y), hsl.z < 0.5);
    let p = 2.0 * hsl.z - q;
    return vec3<f32>(
        hue_to_rgb(p, q, hsl.x + 1.0 / 3.0),
        hue_to_rgb(p, q, hsl.x),
        hue_to_rgb(p, q, hsl.x - 1.0 / 3.0)
    );
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

// Helper: signed distance in local [-0.5, 0.5] space for Shape layers (layer_type == 2u).
fn shape_sdf_distance(local_pos_in: vec2<f32>) -> f32 {
    if (layer.shape_type == 0u) {
        if (layer.shape_params_y > 0.001) {
            let half_sz = vec2<f32>(0.5, 0.5);
            let q = abs(local_pos_in) - (half_sz - layer.shape_params_y);
            return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - layer.shape_params_y;
        }
        let d_x = abs(local_pos_in.x) - 0.5;
        let d_y = abs(local_pos_in.y) - 0.5;
        return max(d_x, d_y);
    } else if (layer.shape_type == 1u) {
        return length(local_pos_in) - 0.5;
    } else if (layer.shape_type == 2u) {
        // Procedural N-Point Star SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params_x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        return r * cos(angle_mod - 0.314159) - 0.45;
    } else if (layer.shape_type == 3u) {
        // Procedural Regular N-Gon Polygon SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params_x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        return r * cos(angle_mod) - 0.45;
    }
    return -1.0;
}

// Helper: SDF-based alpha coverage for Shape layers (layer_type == 2u).
// Shared between the layer fill and the drop-shadow mask so all four
// shape types (rect / ellipse / star / polygon) cast correct shadows.
fn shape_sdf_alpha(local_pos_in: vec2<f32>, blur_extend: f32) -> f32 {
    var alpha = 0.0;
    let d = shape_sdf_distance(local_pos_in);
    if (layer.shape_type == 0u) {
        alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.02 + blur_extend, d);
    } else if (layer.shape_type == 1u) {
        alpha = 1.0 - smoothstep(-0.02 - blur_extend, blur_extend, d);
    } else if (layer.shape_type == 2u || layer.shape_type == 3u) {
        alpha = 1.0 - smoothstep(-0.04 - blur_extend, 0.04 + blur_extend, d);
    } else if (layer.shape_type == 4u) {
        // Particular-style soft volumetric smoke/fire/glow sprite
        let r = length(local_pos_in) * 2.0;
        let radial = exp(-r * r * 3.2) * (1.0 - smoothstep(0.65, 1.0, r));
        let seed = layer.shape_params_x;
        let wisp = 0.65 + 0.35 * sin(local_pos_in.x * 11.0 + seed) * cos(local_pos_in.y * 11.0 - seed * 0.7);
        alpha = clamp(radial * wisp, 0.0, 1.0);
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

fn fn_sample_backdrop_uv(uv: vec2<f32>) -> vec4<f32> {
    let premul = textureSampleLevel(t_backdrop, s_matte, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    if (premul.a <= 0.000001) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(premul.rgb / premul.a, premul.a);
}

fn sample_layer_color(local_pos_in: vec2<f32>, tc_in: vec2<f32>, blur_extend: f32, clip_pos: vec2<f32>) -> vec4<f32> {
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

        // GPU Shape Stroke (over fill along SDF boundary)
        if (layer.shape_stroke_width > 0.0 && layer.shape_stroke_a > 0.001) {
            let d = shape_sdf_distance(local_pos);
            let min_dim = max(min(layer.shape_size_x, layer.shape_size_y), 1.0);
            let sw_local = (layer.shape_stroke_width * 0.5) / min_dim;
            let aa = max(1.0 / min_dim, 0.006) + blur_extend;
            let angle = atan2(local_pos.y, local_pos.x);
            let stroke_cov = (1.0 - smoothstep(max(sw_local - aa, 0.0), sw_local + aa, abs(d)))
                * trim_shape_alpha(angle, layer.trim_start, layer.trim_end, layer.trim_offset);
            let sa = layer.shape_stroke_a * stroke_cov;
            let out_a = sa + c.a * (1.0 - sa);
            if (out_a > 0.0001) {
                let srgb = vec3<f32>(layer.shape_stroke_r, layer.shape_stroke_g, layer.shape_stroke_b);
                let out_rgb = (srgb * sa + c.rgb * c.a * (1.0 - sa)) / out_a;
                c = vec4<f32>(out_rgb, out_a);
            }
        }

        // 3D Extrusion: pseudo-3D depth shading based on distance from shape edge
        if ((layer.extrusion_depth > 0.01 || layer.bevel_depth > 0.01) && alpha > 0.01) {
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
    } else if (layer.layer_type == 7u) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let screen_uv = clamp(clip_pos / vp, vec2<f32>(0.0), vec2<f32>(1.0));
        let sample_backdrop = fn_sample_backdrop_uv(screen_uv);
        if (layer.effect_blur_enabled == 1u) {
            let texel = 1.0 / vp;
            let offset = layer.effect_blur_radius * texel;
            var sum = fn_sample_backdrop_uv(screen_uv) * 0.1111;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x, 0.0)) * 0.1053;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x, 0.0)) * 0.1053;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y)) * 0.1053;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y)) * 0.1053;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            sum += fn_sample_backdrop_uv(screen_uv + vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            sum += fn_sample_backdrop_uv(screen_uv - vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            c = sum;
        } else {
            c = sample_backdrop;
        }
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
        if (layer.blend_mode == 255u) {
            let samples = max(layer.motionblur_samples, 2u);
            let phase = clamp(layer.motionblur_shutter, -1.0, 1.0);
            let texel = vec2<f32>(1.0) / globals.viewport_size;
            var blur_color = vec4<f32>(0.0);
            for (var s = 0u; s < samples; s = s + 1u) {
                let t = (f32(s) / f32(samples - 1u)) - 0.5 + phase;
                let uv = clamp(tc + vel * texel * t, vec2<f32>(0.0), vec2<f32>(1.0));
                blur_color += sample_diffuse(uv);
            }
            c = blur_color / f32(samples);
        } else {
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
    }

    // ── GPU Layer Styles (sim_enabled == 2u) ──
    if (layer.sim_enabled == 2u) {
        let texel = vec2<f32>(1.0) / max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let shadow_rgb = vec3<f32>(
            layer.effect_shadow_color_r,
            layer.effect_shadow_color_g,
            layer.effect_shadow_color_b
        );
        if (layer.sim_type == 0u || layer.sim_type == 1u) {
            // 0 = Drop Shadow, 1 = Outer Glow
            let shadow_offset = select(vec2<f32>(0.0), vec2<f32>(layer.sim_p1, layer.sim_p2) * texel, layer.sim_type == 0u);
            let radius = max(layer.sim_p3, 0.5);
            let step_uv = (radius / 3.0) * texel;
            var alpha_sum = 0.0;
            var weight_sum = 0.0;
            for (var iy = -3; iy <= 3; iy = iy + 1) {
                for (var ix = -3; ix <= 3; ix = ix + 1) {
                    let d2 = f32(ix * ix + iy * iy);
                    let w = exp(-d2 / 4.5);
                    let suv = clamp(
                        tc - shadow_offset + vec2<f32>(f32(ix), f32(iy)) * step_uv,
                        vec2<f32>(0.0),
                        vec2<f32>(1.0)
                    );
                    alpha_sum += sample_diffuse(suv).a * w;
                    weight_sum += w;
                }
            }
            let shadow_a = clamp(
                (alpha_sum / max(weight_sum, 0.0001)) * layer.effect_shadow_opacity,
                0.0,
                1.0
            );
            let out_a = c.a + shadow_a * (1.0 - c.a);
            let out_rgb = select(
                vec3<f32>(0.0),
                (c.rgb * c.a + shadow_rgb * shadow_a * (1.0 - c.a)) / out_a,
                out_a > 0.0001
            );
            c = vec4<f32>(out_rgb, out_a);
        } else if (layer.sim_type == 2u || layer.sim_type == 3u) {
            // 2 = Inner Glow, 3 = Inner Shadow
            let shadow_offset = select(vec2<f32>(0.0), vec2<f32>(layer.sim_p1, layer.sim_p2) * texel, layer.sim_type == 3u);
            let radius = max(layer.sim_p3, 0.5);
            let step_uv = (radius / 3.0) * texel;
            var inv_sum = 0.0;
            var weight_sum = 0.0;
            for (var iy = -3; iy <= 3; iy = iy + 1) {
                for (var ix = -3; ix <= 3; ix = ix + 1) {
                    let d2 = f32(ix * ix + iy * iy);
                    let w = exp(-d2 / 4.5);
                    let suv = clamp(
                        tc - shadow_offset + vec2<f32>(f32(ix), f32(iy)) * step_uv,
                        vec2<f32>(0.0),
                        vec2<f32>(1.0)
                    );
                    inv_sum += (1.0 - sample_diffuse(suv).a) * w;
                    weight_sum += w;
                }
            }
            let inner_factor = clamp(
                (inv_sum / max(weight_sum, 0.0001)) * layer.effect_shadow_opacity,
                0.0,
                1.0
            ) * step(0.01, c.a);
            c = vec4<f32>(mix(c.rgb, shadow_rgb, inner_factor), c.a);
        } else if (layer.sim_type == 4u) {
            // 4 = Layer Style Stroke
            let radius = clamp(layer.sim_p3, 0.5, 32.0);
            let step_uv = (radius / 3.0) * texel;
            var max_a = c.a;
            for (var iy = -3; iy <= 3; iy = iy + 1) {
                for (var ix = -3; ix <= 3; ix = ix + 1) {
                    if (f32(ix * ix + iy * iy) <= 10.0) {
                        let suv = clamp(
                            tc + vec2<f32>(f32(ix), f32(iy)) * step_uv,
                            vec2<f32>(0.0),
                            vec2<f32>(1.0)
                        );
                        max_a = max(max_a, sample_diffuse(suv).a);
                    }
                }
            }
            let stroke_a = clamp((max_a - c.a) * layer.effect_shadow_opacity, 0.0, 1.0);
            let out_a = c.a + stroke_a * (1.0 - c.a);
            let out_rgb = select(
                vec3<f32>(0.0),
                (c.rgb * c.a + shadow_rgb * stroke_a * (1.0 - c.a)) / out_a,
                out_a > 0.0001
            );
            c = vec4<f32>(out_rgb, out_a);
        } else if (layer.sim_type == 5u) {
            // 5 = Satin
            let off = vec2<f32>(layer.sim_p1, layer.sim_p2) * texel;
            let a1 = sample_diffuse(clamp(tc + off, vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let a2 = sample_diffuse(clamp(tc - off, vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let satin_factor = clamp(abs(a1 - a2) * layer.effect_shadow_opacity, 0.0, 1.0) * step(0.01, c.a);
            c = vec4<f32>(mix(c.rgb, shadow_rgb, satin_factor), c.a);
        }
    }

    // ── Extended GPU Effects (sim_enabled == 3u) ──
    if (layer.sim_enabled == 3u) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let texel = vec2<f32>(1.0) / vp;
        if (layer.sim_type == 0u) {
            // Mosaic
            let blocks = max(vec2<f32>(layer.sim_p1, layer.sim_p2), vec2<f32>(1.0, 1.0));
            let cell = blocks * texel;
            let q_uv = clamp((floor(tc / cell) + vec2<f32>(0.5)) * cell, vec2<f32>(0.0), vec2<f32>(1.0));
            c = sample_diffuse(q_uv);
        } else if (layer.sim_type == 1u) {
            // Sharpen
            let n = sample_diffuse(clamp(tc - vec2<f32>(0.0, texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let s = sample_diffuse(clamp(tc + vec2<f32>(0.0, texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let w = sample_diffuse(clamp(tc - vec2<f32>(texel.x, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let e = sample_diffuse(clamp(tc + vec2<f32>(texel.x, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let lap = 4.0 * c.rgb - (n + s + w + e);
            c = vec4<f32>(clamp(c.rgb + lap * layer.sim_p1, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 2u) {
            // Solarize
            let thr = layer.sim_p1;
            let sol = select(c.rgb, vec3<f32>(1.0) - c.rgb, c.rgb > vec3<f32>(thr));
            c = vec4<f32>(clamp(sol, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 3u) {
            // LinearWipe
            let dir = vec2<f32>(cos(layer.sim_p2), sin(layer.sim_p2));
            let proj = dot(tc - vec2<f32>(0.5), dir) + 0.5;
            let mask = step(layer.sim_p1, proj);
            c = vec4<f32>(c.rgb, c.a * mask);
        } else if (layer.sim_type == 4u) {
            // ChromaKey
            let key = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
            let dist = length(c.rgb - key) * layer.sim_p1;
            let lo = min(layer.sim_p2, layer.sim_p3 - 0.001);
            let hi = max(layer.sim_p3, lo + 0.001);
            let matte = smoothstep(lo, hi, dist);
            c = vec4<f32>(c.rgb, c.a * matte);
        } else if (layer.sim_type == 5u) {
            // Vibrance
            let max_c = max(c.r, max(c.g, c.b));
            let min_c = min(c.r, min(c.g, c.b));
            let sat = max_c - min_c;
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let amt = layer.sim_p1 * (1.0 - sat);
            c = vec4<f32>(clamp(mix(vec3<f32>(luma), c.rgb, 1.0 + amt), vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 6u) {
            // WhiteBalance
            let temp = layer.sim_p1 * 0.25;
            let tint = layer.sim_p2 * 0.25;
            let wb = vec3<f32>(c.r + temp, c.g - tint, c.b - temp);
            c = vec4<f32>(clamp(wb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 7u) {
            // SobelEdges / FindEdges
            let luma = vec3<f32>(0.2126, 0.7152, 0.0722);
            let tl = dot(sample_diffuse(clamp(tc + vec2<f32>(-texel.x, -texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb, luma);
            let tr = dot(sample_diffuse(clamp(tc + vec2<f32>(texel.x, -texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb, luma);
            let bl = dot(sample_diffuse(clamp(tc + vec2<f32>(-texel.x, texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb, luma);
            let br = dot(sample_diffuse(clamp(tc + vec2<f32>(texel.x, texel.y), vec2<f32>(0.0), vec2<f32>(1.0))).rgb, luma);
            let gx = (tr + br) - (tl + bl);
            let gy = (bl + br) - (tl + tr);
            var edge = clamp(sqrt(gx * gx + gy * gy) * 2.0, 0.0, 1.0);
            if (layer.sim_p1 > 0.5) {
                edge = 1.0 - edge;
            }
            c = vec4<f32>(vec3<f32>(edge), c.a);
        } else if (layer.sim_type == 8u) {
            // Letterbox
            let bar = layer.sim_p1;
            if (tc.y < bar || tc.y > 1.0 - bar) {
                c = vec4<f32>(0.0, 0.0, 0.0, c.a);
            }
        } else if (layer.sim_type == 9u) {
            // Checkerboard
            let px = floor((tc * vp) / max(layer.sim_p1, 1.0));
            let parity = (i32(px.x) + i32(px.y)) & 1;
            let c1 = vec4<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b, 1.0);
            let c2 = vec4<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b, 1.0);
            c = select(c1, c2, parity != 0);
        } else if (layer.sim_type == 10u) {
            // GradientRamp
            let t_grad = select(tc.y, clamp(length(tc - vec2<f32>(0.5)) * 2.0, 0.0, 1.0), layer.sim_p1 > 0.5);
            let c1 = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b);
            let c2 = vec3<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b);
            c = vec4<f32>(mix(c1, c2, t_grad), c.a);
        } else if (layer.sim_type == 11u) {
            // RadialBlur / RadialBlurZoom / RadialFastBlur (24-sample smooth zoom)
            let dir = tc - vec2<f32>(0.5);
            let amt = layer.sim_p1 * 0.25;
            var acc = vec4<f32>(0.0);
            for (var s = 0u; s < 24u; s = s + 1u) {
                let scale = 1.0 - amt * (f32(s) / 23.0);
                let uv = clamp(vec2<f32>(0.5) + dir * scale, vec2<f32>(0.0), vec2<f32>(1.0));
                acc += sample_diffuse(uv);
            }
            c = acc / 24.0;
        } else if (layer.sim_type == 12u) {
            // VenetianBlinds
            let stripe = fract((tc.x * vp.x) / max(layer.sim_p2, 1.0));
            let mask = step(layer.sim_p1, stripe);
            c = vec4<f32>(c.rgb, c.a * mask);
        } else if (layer.sim_type == 13u) {
            // IrisWipe
            let dist = length(tc - vec2<f32>(0.5)) / 0.70710678;
            let mask = step(layer.sim_p1, dist);
            c = vec4<f32>(c.rgb, c.a * mask);
        } else if (layer.sim_type == 14u) {
            // RadialWipe
            let p = tc - vec2<f32>(0.5);
            let ang = fract((atan2(p.y, p.x) + 1.5707963) / 6.2831853 + 1.0);
            let mask = step(layer.sim_p1, ang);
            c = vec4<f32>(c.rgb, c.a * mask);
        } else if (layer.sim_type == 15u) {
            // RGBSplit
            let r_uv = clamp(tc + vec2<f32>(layer.corner_top_left_x, layer.corner_top_left_y) * texel, vec2<f32>(0.0), vec2<f32>(1.0));
            let g_uv = clamp(tc + vec2<f32>(layer.corner_top_right_x, layer.corner_top_right_y) * texel, vec2<f32>(0.0), vec2<f32>(1.0));
            let b_uv = clamp(tc + vec2<f32>(layer.corner_bottom_left_x, layer.corner_bottom_left_y) * texel, vec2<f32>(0.0), vec2<f32>(1.0));
            let cr = sample_diffuse(r_uv).r;
            let cg = sample_diffuse(g_uv);
            let cb = sample_diffuse(b_uv).b;
            c = vec4<f32>(cr, cg.g, cb, cg.a);
        } else if (layer.sim_type == 16u) {
            // CCKaleida
            let p = tc - vec2<f32>(0.5);
            let r = length(p);
            let sides = max(layer.sim_p1, 2.0);
            let slice = 6.2831853 / sides;
            var a = atan2(p.y, p.x);
            a = abs(fract(a / slice) * slice - slice * 0.5);
            let kuv = clamp(vec2<f32>(0.5) + vec2<f32>(cos(a), sin(a)) * r, vec2<f32>(0.0), vec2<f32>(1.0));
            c = sample_diffuse(kuv);
        } else if (layer.sim_type == 17u) {
            // Halftone
            let cell = max(layer.sim_p1, 2.0);
            let pos_px = tc * vp;
            let center_px = (floor(pos_px / cell) + vec2<f32>(0.5)) * cell;
            let sample_c = sample_diffuse(clamp(center_px * texel, vec2<f32>(0.0), vec2<f32>(1.0)));
            let luma = dot(sample_c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let d = length(pos_px - center_px) / (cell * 0.5);
            let dot_on = step(d, sqrt(luma));
            c = vec4<f32>(sample_c.rgb * dot_on, sample_c.a);
        } else if (layer.sim_type == 18u) {
            // Flicker
            c = vec4<f32>(clamp(c.rgb * layer.sim_p1, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 19u) {
            // LumaKeyRange
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let in_range = step(layer.sim_p1, luma) * (1.0 - step(layer.sim_p2, luma));
            let matte = select(1.0 - in_range, in_range, layer.sim_p3 > 0.5);
            c = vec4<f32>(c.rgb, c.a * matte);
        } else if (layer.sim_type == 20u) {
            // AlphaFromLuminance
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let a_out = select(luma, 1.0 - luma, layer.sim_p1 > 0.5);
            c = vec4<f32>(c.rgb, c.a * a_out);
        } else if (layer.sim_type == 21u) {
            // NightVision
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let noise = (fract(sin(dot(tc * vp + vec2<f32>(layer.sim_p2), vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5) * 0.15;
            let g = clamp(luma * layer.sim_p1 + noise, 0.0, 1.0);
            c = vec4<f32>(g * 0.15, g, g * 0.25, c.a);
        } else if (layer.sim_type == 22u) {
            // ColorBalance (shadows/midtones/highlights)
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let w_sh = clamp(1.0 - luma * 2.0, 0.0, 1.0);
            let w_hi = clamp((luma - 0.5) * 2.0, 0.0, 1.0);
            let w_mid = clamp(1.0 - w_sh - w_hi, 0.0, 1.0);
            let sh = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b);
            let mid = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
            let hi = vec3<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b);
            c = vec4<f32>(clamp(c.rgb + sh * w_sh + mid * w_mid + hi * w_hi, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 23u) {
            // SimpleChoker
            let r_px = clamp(abs(layer.sim_p1), 0.5, 8.0);
            var ext_a = c.a;
            for (var iy = -2; iy <= 2; iy = iy + 1) {
                for (var ix = -2; ix <= 2; ix = ix + 1) {
                    let suv = clamp(tc + vec2<f32>(f32(ix), f32(iy)) * (r_px * 0.5) * texel, vec2<f32>(0.0), vec2<f32>(1.0));
                    let sa = sample_diffuse(suv).a;
                    ext_a = select(max(ext_a, sa), min(ext_a, sa), layer.sim_p1 > 0.0);
                }
            }
            c = vec4<f32>(c.rgb, ext_a);
        } else if (layer.sim_type == 24u) {
            // Offset
            let uv = fract(tc - vec2<f32>(layer.sim_p1, layer.sim_p2) * texel + vec2<f32>(1.0));
            c = sample_diffuse(uv);
        } else if (layer.sim_type == 25u) {
            // Tiler
            let scaled = tc * layer.sim_p1;
            var uv = fract(scaled);
            if (layer.sim_p2 > 0.5) {
                let tile = floor(scaled);
                if ((i32(tile.x) & 1) != 0) { uv.x = 1.0 - uv.x; }
                if ((i32(tile.y) & 1) != 0) { uv.y = 1.0 - uv.y; }
            }
            c = sample_diffuse(uv);
        } else if (layer.sim_type == 26u) {
            // Fisheye / CcLens
            let p = (tc - vec2<f32>(0.5)) * 2.0;
            let r2 = dot(p, p);
            let zoom = select(1.0, layer.sim_p2, layer.sim_p2 > 0.05);
            let fuv = clamp(vec2<f32>(0.5) + (p * (1.0 + layer.sim_p1 * r2) / zoom) * 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
            c = sample_diffuse(fuv);
        } else if (layer.sim_type == 27u) {
            // PolarCoordinates
            let p = tc - vec2<f32>(0.5);
            var target_uv = tc;
            if (layer.sim_p2 > 0.5) {
                let r = length(p) * 2.0;
                let a = fract((atan2(p.y, p.x) + 3.14159265) / 6.2831853);
                target_uv = vec2<f32>(a, clamp(r, 0.0, 1.0));
            } else {
                let a = tc.x * 6.2831853 - 3.14159265;
                let r = tc.y * 0.5;
                target_uv = vec2<f32>(0.5) + vec2<f32>(cos(a), sin(a)) * r;
            }
            let puv = clamp(mix(tc, target_uv, layer.sim_p1), vec2<f32>(0.0), vec2<f32>(1.0));
            c = sample_diffuse(puv);
        } else if (layer.sim_type == 28u) {
            // Vortex
            let rel = tc - vec2<f32>(0.5);
            let dist = length(rel);
            let rad = max(layer.sim_p1, 0.01);
            if (dist < rad) {
                let f = 1.0 - dist / rad;
                let ang = layer.sim_p2 * f * f;
                let ca = cos(ang);
                let sa = sin(ang);
                let vuv = clamp(vec2<f32>(0.5) + vec2<f32>(rel.x * ca - rel.y * sa, rel.x * sa + rel.y * ca), vec2<f32>(0.0), vec2<f32>(1.0));
                c = sample_diffuse(vuv);
            }
        } else if (layer.sim_type == 29u) {
            // FilmEmulation (ASC CDL lift/gamma/gain)
            let graded = pow(max(c.rgb * layer.sim_p3 + vec3<f32>(layer.sim_p1), vec3<f32>(0.0)), vec3<f32>(1.0 / max(layer.sim_p2, 0.05)));
            c = vec4<f32>(clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 30u) {
            // PinchPunch
            let rel = tc - vec2<f32>(0.5);
            let dist = length(rel);
            let rad = max(layer.sim_p1, 0.01);
            if (dist < rad && dist > 0.0001) {
                let f = dist / rad;
                let p_factor = pow(f, max(1.0 + layer.sim_p2, 0.1));
                let puv = clamp(vec2<f32>(0.5) + normalize(rel) * (p_factor * rad), vec2<f32>(0.0), vec2<f32>(1.0));
                c = sample_diffuse(puv);
            }
        } else if (layer.sim_type == 31u) {
            // ScanlineGlitch
            let row = floor(tc.y * vp.y * 0.25);
            let rnd = fract(sin(row * 12.9898 + layer.sim_p2 * 78.233) * 43758.5453) - 0.5;
            let shift = rnd * layer.sim_p1 * texel.x;
            c = sample_diffuse(clamp(vec2<f32>(tc.x + shift, tc.y), vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 32u) {
            // LightSweep
            let dir = vec2<f32>(cos(layer.sim_p1), sin(layer.sim_p1));
            let pos = dot(tc - vec2<f32>(0.5), dir) + 0.5;
            let dist = abs(pos - layer.sim_p2);
            let sweep = exp(- (dist * dist) / max(layer.sim_p3 * layer.sim_p3, 0.0001)) * layer.tint_amount;
            c = vec4<f32>(clamp(c.rgb + vec3<f32>(sweep) * c.a, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 33u) {
            // ShiftChannels
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            c = vec4<f32>(
                pick_shift_channel(c, luma, layer.sim_p1, c.r),
                pick_shift_channel(c, luma, layer.sim_p2, c.g),
                pick_shift_channel(c, luma, layer.sim_p3, c.b),
                pick_shift_channel(c, luma, layer.sim_p4, c.a)
            );
        } else if (layer.sim_type == 34u || layer.sim_type == 35u) {
            // 34 = OpticsCompensation, 35 = LensCorrection
            let p = (tc - vec2<f32>(0.5)) * 2.0;
            let r2 = dot(p, p);
            let factor = select(
                1.0 + layer.sim_p1 * r2 * 0.5,
                1.0 + layer.sim_p1 * r2 + layer.sim_p2 * r2 * r2,
                layer.sim_type == 35u
            );
            let zoom = select(1.0, max(layer.sim_p2, 0.1), layer.sim_type == 34u);
            let ouv = clamp(vec2<f32>(0.5) + (p * factor / zoom) * 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
            c = sample_diffuse(ouv);
        } else if (layer.sim_type == 36u) {
            // TiltShift
            let dist = max(abs(tc.y - layer.sim_p1) - layer.sim_p2 * 0.5, 0.0);
            let rad = clamp(dist * layer.sim_p3 * 2.0, 0.0, 12.0);
            if (rad > 0.25) {
                let step_uv = rad * texel;
                var sum = vec4<f32>(0.0);
                for (var iy = -1; iy <= 1; iy = iy + 1) {
                    for (var ix = -1; ix <= 1; ix = ix + 1) {
                        sum += sample_diffuse(clamp(tc + vec2<f32>(f32(ix), f32(iy)) * step_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
                    }
                }
                c = sum / 9.0;
            }
        } else if (layer.sim_type == 37u) {
            // Emboss
            let off = vec2<f32>(cos(layer.sim_p1), sin(layer.sim_p1)) * texel;
            let c1 = dot(sample_diffuse(clamp(tc + off, vec2<f32>(0.0), vec2<f32>(1.0))).rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let c2 = dot(sample_diffuse(clamp(tc - off, vec2<f32>(0.0), vec2<f32>(1.0))).rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let v = clamp(0.5 + (c1 - c2) * layer.sim_p2, 0.0, 1.0);
            c = vec4<f32>(vec3<f32>(v), c.a);
        } else if (layer.sim_type == 38u) {
            // BlockGlitch
            let blk = floor((tc * vp) / max(layer.sim_p1, 4.0));
            let rnd = fract(sin(dot(blk, vec2<f32>(12.9898, 78.233)) + layer.sim_p3) * 43758.5453);
            if (rnd > 0.75) {
                let shift = (rnd - 0.875) * 8.0 * layer.sim_p2 * texel.x;
                c = sample_diffuse(clamp(vec2<f32>(tc.x + shift, tc.y), vec2<f32>(0.0), vec2<f32>(1.0)));
            }
        } else if (layer.sim_type == 39u) {
            // SliceTear
            let coord = select(tc.y, tc.x, layer.sim_p4 > 0.5);
            let slice_id = floor(coord * layer.sim_p1);
            let rnd = fract(sin(slice_id * 19.13 + layer.sim_p3 * 41.7) * 43758.5453) - 0.5;
            let off = rnd * layer.sim_p2 * texel;
            let suv = select(vec2<f32>(tc.x + off.x, tc.y), vec2<f32>(tc.x, tc.y + off.y), layer.sim_p4 > 0.5);
            c = sample_diffuse(clamp(suv, vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 40u) {
            // LightLeak
            let d = length(tc - vec2<f32>(layer.sim_p1, layer.sim_p2));
            let leak = exp(-d * d * 4.5) * layer.sim_p3;
            let warm = vec3<f32>(1.0, 0.58, 0.22) * leak;
            c = vec4<f32>(clamp(c.rgb + warm * c.a, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 41u) {
            // StrobeLight
            if (layer.sim_p1 > 0.5) {
                let sc = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
                c = vec4<f32>(mix(c.rgb, sc, layer.effect_shadow_color_a), c.a);
            }
        } else if (layer.sim_type == 42u) {
            // ChannelMixer
            let r_row = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b);
            let g_row = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
            let b_row = vec3<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b);
            var out_rgb = clamp(vec3<f32>(dot(c.rgb, r_row), dot(c.rgb, g_row), dot(c.rgb, b_row)), vec3<f32>(0.0), vec3<f32>(1.0));
            if (layer.sim_p1 > 0.5) {
                let m = dot(out_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
                out_rgb = vec3<f32>(m);
            }
            c = vec4<f32>(out_rgb, c.a);
        } else if (layer.sim_type == 43u) {
            // HslAdjust
            var hsl = rgb_to_hsl(c.rgb);
            hsl.x = fract(hsl.x + layer.sim_p1 + 1.0);
            hsl.y = clamp(hsl.y + layer.sim_p2, 0.0, 1.0);
            hsl.z = clamp(hsl.z + layer.sim_p3, 0.0, 1.0);
            c = vec4<f32>(hsl_to_rgb(hsl), c.a);
        } else if (layer.sim_type == 44u) {
            // DirectionalSharpen
            let dir = vec2<f32>(cos(layer.sim_p1), sin(layer.sim_p1)) * texel;
            let c1 = sample_diffuse(clamp(tc + dir, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let c2 = sample_diffuse(clamp(tc - dir, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
            let sharp = c.rgb + (c.rgb - (c1 + c2) * 0.5) * layer.sim_p2;
            c = vec4<f32>(clamp(sharp, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 45u) {
            // BevelAlpha
            let d = max(layer.sim_p1, 1.0) * texel;
            let ldir = vec2<f32>(cos(layer.sim_p2), sin(layer.sim_p2));
            let a1 = sample_diffuse(clamp(tc + ldir * d, vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let a2 = sample_diffuse(clamp(tc - ldir * d, vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let diff = clamp((a2 - a1) * 0.8, -1.0, 1.0);
            c = vec4<f32>(clamp(c.rgb + vec3<f32>(diff * 0.6), vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 46u) {
            // GlassEdgeBevel
            let d = layer.sim_p1 * texel;
            let gx = sample_diffuse(clamp(tc + vec2<f32>(d.x, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).a
                   - sample_diffuse(clamp(tc - vec2<f32>(d.x, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let gy = sample_diffuse(clamp(tc + vec2<f32>(0.0, d.y), vec2<f32>(0.0), vec2<f32>(1.0))).a
                   - sample_diffuse(clamp(tc - vec2<f32>(0.0, d.y), vec2<f32>(0.0), vec2<f32>(1.0))).a;
            let refr = clamp(tc + vec2<f32>(gx, gy) * layer.sim_p2 * 0.05, vec2<f32>(0.0), vec2<f32>(1.0));
            let rc = sample_diffuse(refr);
            let spec = clamp(length(vec2<f32>(gx, gy)) * 0.4, 0.0, 0.6);
            c = vec4<f32>(clamp(rc.rgb + vec3<f32>(spec), vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 47u) {
            // MatteChoker / MatteChokeSpread
            let rad = abs(layer.sim_p1);
            if (rad > 0.01) {
                let d = rad * texel;
                var a_min = c.a;
                var a_max = c.a;
                for (var i = 0; i < 8; i = i + 1) {
                    let ang = f32(i) * 0.785398;
                    let sa = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * d, vec2<f32>(0.0), vec2<f32>(1.0))).a;
                    a_min = min(a_min, sa);
                    a_max = max(a_max, sa);
                }
                let target_a = select(a_min, a_max, layer.sim_p1 < 0.0);
                c = vec4<f32>(c.rgb, mix(c.a, target_a, 0.85));
            }
        } else if (layer.sim_type == 48u) {
            // AlphaFeather (16-tap 2-ring smooth alpha feather)
            if (layer.sim_p1 > 0.01) {
                let d = layer.sim_p1 * texel;
                var a_sum = c.a;
                var wsum = 1.0;
                for (var ring = 1; ring <= 2; ring = ring + 1) {
                    let rf = f32(ring) * 0.5;
                    let rw = select(0.6, 1.0, ring == 1);
                    let phase = f32(ring) * 0.392699;
                    for (var i = 0; i < 8; i = i + 1) {
                        let ang = f32(i) * 0.785398 + phase;
                        a_sum += sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * d * rf, vec2<f32>(0.0), vec2<f32>(1.0))).a * rw;
                        wsum += rw;
                    }
                }
                c = vec4<f32>(c.rgb, a_sum / wsum);
            }
        } else if (layer.sim_type == 49u) {
            // Minimax
            let d = max(layer.sim_p2, 0.5) * texel;
            var best = c;
            let use_max = layer.sim_p1 < 0.5;
            for (var i = 0; i < 8; i = i + 1) {
                let ang = f32(i) * 0.785398;
                let s = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * d, vec2<f32>(0.0), vec2<f32>(1.0)));
                if (use_max) {
                    best = max(best, s);
                } else {
                    best = min(best, s);
                }
            }
            c = best;
        } else if (layer.sim_type == 50u) {
            // MedianFilter / BilateralFilter (16-tap 2-ring edge-preserving filter)
            let d = layer.sim_p1 * texel;
            var acc = c;
            var wsum = 1.0;
            let is_bilat = layer.sim_p3 > 0.5;
            let sigma2 = max(layer.sim_p2 * layer.sim_p2, 0.001);
            for (var ring = 1; ring <= 2; ring = ring + 1) {
                let rf = f32(ring) * 0.5;
                let phase = f32(ring) * 0.392699;
                for (var i = 0; i < 8; i = i + 1) {
                    let ang = f32(i) * 0.785398 + phase;
                    let s = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * d * rf, vec2<f32>(0.0), vec2<f32>(1.0)));
                    var w = select(0.6, 1.0, ring == 1);
                    if (is_bilat) {
                        let dc = s.rgb - c.rgb;
                        w *= exp(-dot(dc, dc) / sigma2);
                    }
                    acc += s * w;
                    wsum += w;
                }
            }
            c = acc / wsum;
        } else if (layer.sim_type == 51u) {
            // MotionTile
            let center = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let out_scale = vec2<f32>(layer.sim_p3, layer.sim_p4);
            let tile_scale = vec2<f32>(layer.tint_black_r, layer.tint_black_g);
            let rel = (tc - 0.5) * out_scale / tile_scale + center + vec2<f32>(layer.tint_black_a, 0.0);
            var tuv = fract(rel);
            if (layer.tint_black_b > 0.5) {
                let fl = floor(rel);
                if (i32(abs(fl.x)) % 2 == 1) { tuv.x = 1.0 - tuv.x; }
                if (i32(abs(fl.y)) % 2 == 1) { tuv.y = 1.0 - tuv.y; }
            }
            c = sample_diffuse(clamp(tuv, vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 52u) {
            // Transform
            let anc = vec2<f32>(layer.tint_black_r, layer.tint_black_g);
            let pos = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let scl = vec2<f32>(layer.sim_p3, layer.sim_p4);
            let ang = -layer.tint_black_b;
            let p = tc - pos;
            let ca = cos(ang);
            let sa = sin(ang);
            var rot = vec2<f32>(p.x * ca - p.y * sa, p.x * sa + p.y * ca);
            rot.x -= tan(layer.tint_white_r) * rot.y;
            let uv = rot / scl + anc;
            if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
                c = vec4<f32>(0.0);
            } else {
                c = sample_diffuse(uv);
                c.a *= layer.tint_black_a;
            }
        } else if (layer.sim_type == 53u) {
            // PageTurn
            let fp = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let dir = vec2<f32>(cos(layer.sim_p4), sin(layer.sim_p4));
            let d = dot(tc - fp, dir);
            if (d > 0.0) {
                if (d < layer.sim_p3) {
                    let shade = 0.7 + 0.3 * sin(d / layer.sim_p3 * 3.14159);
                    c = vec4<f32>(vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b) * shade, layer.effect_shadow_color_a);
                } else {
                    c = vec4<f32>(0.0);
                }
            }
        } else if (layer.sim_type == 54u) {
            // BendIt
            let off = mix(layer.sim_p1, layer.sim_p2, tc.y) * 0.002;
            let buv = vec2<f32>(tc.x - off * sin(tc.y * 3.14159), tc.y);
            if (buv.x < 0.0 || buv.x > 1.0) {
                c = vec4<f32>(0.0);
            } else {
                c = sample_diffuse(buv);
            }
        } else if (layer.sim_type == 55u) {
            // GlitchDisplacement
            let band = floor(tc.y * 32.0);
            let rnd = fract(sin(band * 91.7 + layer.sim_p2 * 13.3) * 43758.5453) - 0.5;
            let disp = rnd * layer.sim_p1 * 0.002;
            let r = sample_diffuse(clamp(vec2<f32>(tc.x + disp * 1.4, tc.y), vec2<f32>(0.0), vec2<f32>(1.0))).r;
            let g = sample_diffuse(clamp(vec2<f32>(tc.x + disp, tc.y), vec2<f32>(0.0), vec2<f32>(1.0))).g;
            let b = sample_diffuse(clamp(vec2<f32>(tc.x - disp * 1.4, tc.y), vec2<f32>(0.0), vec2<f32>(1.0))).b;
            c = vec4<f32>(r, g, b, c.a);
        } else if (layer.sim_type == 56u) {
            // PixelSort
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            if (luma > layer.sim_p1) {
                var acc = c;
                for (var i = 1; i <= 4; i = i + 1) {
                    let suv = clamp(tc + vec2<f32>(f32(i) * 2.0 * texel.x, 0.0), vec2<f32>(0.0), vec2<f32>(1.0));
                    acc = max(acc, sample_diffuse(suv));
                }
                c = vec4<f32>(acc.rgb, c.a);
            }
        } else if (layer.sim_type == 57u) {
            // CrossHatch
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let px = tc * vp;
            let gap = layer.sim_p1;
            let d1 = abs(((px.x + px.y) % gap) - gap * 0.5);
            let d2 = abs(((px.x - px.y + 4096.0) % gap) - gap * 0.5);
            var ink = 1.0;
            if (luma < layer.sim_p2 && d1 < 1.0) { ink = 0.15; }
            if (luma < layer.sim_p2 * 0.5 && d2 < 1.0) { ink = 0.05; }
            c = vec4<f32>(c.rgb * ink, c.a);
        } else if (layer.sim_type == 58u) {
            // GodRays (24-sample volumetric shafts)
            let sun = vec2<f32>(layer.sim_p1, layer.sim_p2);
            var delta = (tc - sun) / 24.0;
            var suv = tc;
            var illum = 1.0;
            var ray = vec3<f32>(0.0);
            for (var i = 0; i < 24; i = i + 1) {
                suv -= delta;
                let s = sample_diffuse(clamp(suv, vec2<f32>(0.0), vec2<f32>(1.0)));
                ray += s.rgb * illum * layer.sim_p4 * 0.04;
                illum *= layer.sim_p3;
            }
            c = vec4<f32>(clamp(c.rgb + ray, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 59u) {
            // OpticalFlares
            let fc = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let d = length((tc - fc) * vec2<f32>(vp.x / vp.y, 1.0)) / layer.sim_p4;
            let glow = exp(-d * d * 24.0) * layer.sim_p3;
            let streak = exp(-abs(tc.y - fc.y) * 90.0) * exp(-abs(tc.x - fc.x) * 4.0) * layer.sim_p3 * 0.5;
            let fl = vec3<f32>(1.0, 0.85, 0.65) * (glow + streak);
            c = vec4<f32>(clamp(c.rgb + fl, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 60u) {
            // StarField
            let grid = floor(tc * (20.0 + sqrt(layer.sim_p1)));
            let rnd = fract(sin(dot(grid, vec2<f32>(127.1, 311.7)) + layer.sim_p2) * 43758.5453);
            if (rnd > 0.96) {
                let star = (rnd - 0.96) * 25.0;
                c = vec4<f32>(clamp(c.rgb + vec3<f32>(star), vec3<f32>(0.0), vec3<f32>(1.0)), max(c.a, star));
            }
        } else if (layer.sim_type == 61u) {
            // LightningArc
            let a = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let b = vec2<f32>(layer.sim_p3, layer.sim_p4);
            let ab = b - a;
            let t = clamp(dot(tc - a, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
            let jitter = (fract(sin(floor(t * 24.0) * 47.3 + layer.tint_black_r) * 43758.5453) - 0.5) * 0.04 * sin(t * 3.14159);
            let perp = normalize(vec2<f32>(-ab.y, ab.x) + vec2<f32>(0.0001));
            let p_arc = a + ab * t + perp * jitter;
            let d = length(tc - p_arc);
            let arc = exp(-d * 180.0) + exp(-d * 40.0) * layer.tint_black_g * 0.4;
            let col = vec3<f32>(0.65, 0.85, 1.0) * arc;
            c = vec4<f32>(clamp(c.rgb + col, vec3<f32>(0.0), vec3<f32>(1.0)), max(c.a, clamp(arc, 0.0, 1.0)));
        } else if (layer.sim_type == 62u) {
            // LaserBeam
            let a = vec2<f32>(layer.sim_p1, layer.sim_p2);
            let b = mix(a, vec2<f32>(layer.sim_p3, layer.sim_p4), layer.tint_white_r);
            let ab = b - a;
            let t = clamp(dot(tc - a, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
            let d = length(tc - (a + ab * t)) * min(vp.x, vp.y);
            let thick = mix(layer.tint_white_g, layer.tint_white_b, t);
            let core_m = smoothstep(thick, 0.0, d);
            let glow_m = exp(-d / max(thick * 2.0, 1.0));
            let beam = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b) * core_m
                     + vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b) * glow_m;
            c = vec4<f32>(clamp(c.rgb + beam, vec3<f32>(0.0), vec3<f32>(1.0)), max(c.a, clamp(core_m + glow_m * 0.5, 0.0, 1.0)));
        } else if (layer.sim_type == 63u) {
            // PerlinFlow
            let p = tc * (layer.sim_p1 * 0.1 + 2.0) + vec2<f32>(layer.sim_p2, layer.sim_p2 * 0.7);
            let nx = sin(p.x * 3.1 + cos(p.y * 2.7)) * 0.015;
            let ny = cos(p.y * 3.3 + sin(p.x * 2.3)) * 0.015;
            c = sample_diffuse(clamp(tc + vec2<f32>(nx, ny), vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 64u) {
            // FbmTurbulence
            var n = 0.0;
            var freq = 4.0;
            var amp = 0.5;
            let octs = i32(layer.sim_p1);
            for (var i = 0; i < 6; i = i + 1) {
                if (i >= octs) { break; }
                let q = tc * freq + vec2<f32>(layer.sim_p3 * f32(i + 1));
                n += (sin(q.x) * cos(q.y) * 0.5 + 0.5) * amp;
                freq *= 2.0;
                amp *= 0.5;
            }
            let off = (n - 0.5) * layer.sim_p2 * 0.002;
            c = sample_diffuse(clamp(tc + vec2<f32>(off, off), vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 65u) {
            // VRHorizon
            let d = tc - 0.5;
            let ca = cos(layer.sim_p3);
            let sa = sin(layer.sim_p3);
            let rot = vec2<f32>(d.x * ca - d.y * sa, d.x * sa + d.y * ca);
            let uv = fract(rot + 0.5 + vec2<f32>(layer.sim_p2 / 6.2831853, layer.sim_p1 / 3.1415926));
            c = sample_diffuse(clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 66u) {
            // CCPixelPolly
            let cell = floor(tc * 12.0);
            let rnd = vec2<f32>(
                fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5,
                fract(sin(dot(cell, vec2<f32>(39.346, 11.135))) * 43758.5453) - 0.5
            );
            let suv = tc - rnd * layer.sim_p1 * 0.35;
            if (suv.x < 0.0 || suv.x > 1.0 || suv.y < 0.0 || suv.y > 1.0) {
                c = vec4<f32>(0.0);
            } else {
                c = sample_diffuse(suv);
            }
        } else if (layer.sim_type == 67u) {
            // Colorama
            let luma = fract(dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) + layer.sim_p2 + layer.sim_p1 * 0.1);
            let pal = 0.5 + 0.5 * cos(6.2831853 * (luma + vec3<f32>(0.0, 0.33, 0.67)));
            c = vec4<f32>(pal, c.a);
        } else if (layer.sim_type == 68u) {
            // GradientMap
            let luma = clamp(dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0, 1.0);
            let low = vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b);
            let mid = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
            let hi = vec3<f32>(layer.tint_white_r, layer.tint_white_g, layer.tint_white_b);
            let mapped = select(mix(mid, hi, (luma - 0.5) * 2.0), mix(low, mid, luma * 2.0), luma < 0.5);
            c = vec4<f32>(mapped, c.a);
        } else if (layer.sim_type == 69u) {
            // RefractionLens
            let d = tc - 0.5;
            let dist = length(d);
            if (dist < layer.sim_p1 && dist > 0.0001) {
                let k = (1.0 - dist / layer.sim_p1) * (layer.sim_p2 - 1.0);
                c = sample_diffuse(clamp(tc - d * k, vec2<f32>(0.0), vec2<f32>(1.0)));
            }
        } else if (layer.sim_type == 70u) {
            // ChannelCombiner
            let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let hsl = rgb_to_hsl(c.rgb);
            var val = luma;
            let f_idx = i32(round(layer.sim_p1));
            if (f_idx == 0) { val = c.r; }
            else if (f_idx == 1) { val = c.g; }
            else if (f_idx == 2) { val = c.b; }
            else if (f_idx == 3) { val = c.a; }
            else if (f_idx == 5) { val = hsl.x; }
            else if (f_idx == 6) { val = hsl.z; }
            else if (f_idx == 7) { val = hsl.y; }
            else if (f_idx == 8) { val = min(c.r, min(c.g, c.b)); }
            else if (f_idx == 9) { val = max(c.r, max(c.g, c.b)); }
            if (layer.sim_p3 > 0.5) { val = 1.0 - val; }
            let t_idx = i32(round(layer.sim_p2));
            if (t_idx == 0) { c.r = val; }
            else if (t_idx == 1) { c.g = val; }
            else if (t_idx == 2) { c.b = val; }
            else if (t_idx == 3) { c.a = val; }
            else if (t_idx == 4) { c = vec4<f32>(val, val, val, c.a); }
            else if (t_idx == 5) { c = vec4<f32>(val, val, val, val); }
            else if (t_idx == 6) { c = vec4<f32>(hsl_to_rgb(vec3<f32>(hsl.x, hsl.y, val)), c.a); }
        } else if (layer.sim_type == 71u) {
            // CameraLensBlur (24-tap 3-ring bokeh disk + highlight bloom)
            if (layer.sim_p1 > 0.1) {
                let r = layer.sim_p1 * texel;
                var acc = c;
                var wsum = 1.0;
                for (var ring = 1; ring <= 3; ring = ring + 1) {
                    let rf = f32(ring) / 3.0;
                    let phase = f32(ring) * 0.392699;
                    for (var i = 0; i < 8; i = i + 1) {
                        let ang = f32(i) * 0.785398 + phase;
                        var s = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * r * rf, vec2<f32>(0.0), vec2<f32>(1.0)));
                        let lum = dot(s.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
                        if (lum > layer.sim_p3) {
                            s = vec4<f32>(s.rgb * (1.0 + layer.sim_p2), s.a);
                        }
                        acc += s;
                        wsum += 1.0;
                    }
                }
                c = clamp(acc / wsum, vec4<f32>(0.0), vec4<f32>(1.0));
            }
        } else if (layer.sim_type == 72u) {
            // VRSphericalBlur
            if (layer.sim_p1 > 0.1) {
                let lat_scale = 1.0 / max(cos((tc.y - 0.5) * 3.14159), 0.2);
                let r = vec2<f32>(layer.sim_p1 * lat_scale, layer.sim_p1) * texel;
                var acc = c;
                for (var i = 0; i < 8; i = i + 1) {
                    let ang = f32(i) * 0.785398;
                    let suv = vec2<f32>(fract(tc.x + cos(ang) * r.x + 1.0), clamp(tc.y + sin(ang) * r.y, 0.0, 1.0));
                    acc += sample_diffuse(suv);
                }
                c = acc / 9.0;
            }
        } else if (layer.sim_type == 73u) {
            // AudioSpectrum (samples real FFT bands from t_matte)
            let rx = (tc.x - layer.sim_p3) / layer.tint_white_r;
            let ry = (tc.y - layer.sim_p4) / layer.tint_white_g;
            if (rx >= 0.0 && rx <= 1.0 && ry >= 0.0 && ry <= 1.0) {
                let band_idx = floor(rx * layer.sim_p1);
                let band_u = fract(rx * layer.sim_p1);
                let fft_val = textureSample(t_matte, s_matte, vec2<f32>((band_idx + 0.5) / max(layer.sim_p1, 1.0), 0.5)).r;
                let proc_val = 0.2 + 0.75 * fract(sin(band_idx * 17.3 + layer.tint_white_b) * 43758.5453);
                let amp = max(fft_val, proc_val * 0.15);
                if (band_u > 0.15 && band_u < 0.85 && (1.0 - ry) <= amp) {
                    let bar_col = mix(
                        vec3<f32>(layer.tint_black_r, layer.tint_black_g, layer.tint_black_b),
                        vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b),
                        1.0 - ry
                    );
                    c = vec4<f32>(mix(c.rgb, bar_col, layer.sim_p2), max(c.a, layer.sim_p2));
                }
            }
        } else if (layer.sim_type == 74u) {
            // FireAutomaton
            let fy = 1.0 - tc.y;
            let n1 = sin(tc.x * 18.0 + layer.sim_p2 * 3.1) * cos(tc.y * 12.0 + layer.sim_p2 * 4.7);
            let n2 = sin(tc.x * 37.0 - layer.sim_p2 * 5.3) * 0.5;
            let heat = clamp((1.0 - fy * 1.35) + (n1 + n2) * 0.25, 0.0, 1.0) * layer.sim_p1;
            if (heat > 0.05) {
                let flame = vec3<f32>(clamp(heat * 1.8, 0.0, 1.0), clamp(heat * 0.85, 0.0, 1.0), clamp(heat * 0.2, 0.0, 1.0));
                c = vec4<f32>(clamp(c.rgb + flame * 0.75, vec3<f32>(0.0), vec3<f32>(1.0)), max(c.a, clamp(heat, 0.0, 1.0)));
            }
        } else if (layer.sim_type == 75u) {
            // Curves (smooth S-curve contrast by channel)
            let ch = i32(round(layer.sim_p1));
            let s = c.rgb * c.rgb * (3.0 - 2.0 * c.rgb);
            if (ch == 0) { c = vec4<f32>(s, c.a); }
            else if (ch == 1) { c.r = s.r; }
            else if (ch == 2) { c.g = s.g; }
            else if (ch == 3) { c.b = s.b; }
        } else if (layer.sim_type == 76u) {
            // ColorGradeLUT (analytic filmic grade)
            let graded = clamp(pow(max(c.rgb, vec3<f32>(0.0)), vec3<f32>(0.95)) * 1.04 - vec3<f32>(0.01, 0.005, 0.0), vec3<f32>(0.0), vec3<f32>(1.0));
            c = vec4<f32>(mix(c.rgb, graded, layer.sim_p1), c.a);
        } else if (layer.sim_type == 77u) {
            // ZDepthFog
            let depth = 1.0 - dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let f = clamp((depth - layer.sim_p1) / max(layer.sim_p2 - layer.sim_p1, 0.001), 0.0, 1.0);
            let fc = vec3<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g, layer.effect_shadow_color_b);
            c = vec4<f32>(mix(c.rgb, fc, f * layer.effect_shadow_color_a), c.a);
        } else if (layer.sim_type == 78u) {
            // SSAO
            let r = layer.sim_p1 * texel;
            let l0 = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            var occ = 0.0;
            for (var i = 0; i < 8; i = i + 1) {
                let ang = f32(i) * 0.785398;
                let sc = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * r, vec2<f32>(0.0), vec2<f32>(1.0)));
                let ls = dot(sc.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
                occ += max(ls - l0, 0.0);
            }
            let ao = clamp(1.0 - (occ / 8.0) * layer.sim_p2 * 2.0, 0.0, 1.0);
            c = vec4<f32>(c.rgb * ao, c.a);
        } else if (layer.sim_type == 79u) {
            // ReflectionMap
            if (tc.y > layer.sim_p1) {
                let dy = tc.y - layer.sim_p1;
                let ry = clamp(layer.sim_p1 - dy, 0.0, 1.0);
                let refl = sample_diffuse(vec2<f32>(tc.x, ry));
                let fade = clamp(1.0 - dy / layer.sim_p2, 0.0, 1.0) * layer.sim_p3;
                c = vec4<f32>(mix(c.rgb, refl.rgb, fade * refl.a), max(c.a, fade * refl.a));
            }
        } else if (layer.sim_type == 80u) {
            // CompoundBlur (samples source_layer via t_matte when available)
            let map_c = textureSample(t_matte, s_matte, clamp(tc, vec2<f32>(0.0), vec2<f32>(1.0)));
            let luma = dot(map_c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * map_c.a;
            let r = layer.sim_p1 * luma * texel;
            if (layer.sim_p1 * luma > 0.1) {
                var acc = c;
                for (var i = 0; i < 8; i = i + 1) {
                    let ang = f32(i) * 0.785398;
                    acc += sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * r, vec2<f32>(0.0), vec2<f32>(1.0)));
                }
                c = acc / 9.0;
            }
        } else if (layer.sim_type == 81u) {
            // DisplacementMap (samples source_layer via t_matte)
            let map_c = textureSample(t_matte, s_matte, clamp(tc, vec2<f32>(0.0), vec2<f32>(1.0)));
            let off = vec2<f32>((map_c.r - 0.5) * layer.sim_p1, (map_c.g - 0.5) * layer.sim_p2) * texel;
            c = sample_diffuse(clamp(tc + off, vec2<f32>(0.0), vec2<f32>(1.0)));
        } else if (layer.sim_type == 82u) {
            // TemporalDenoise (edge-preserving spatial denoise)
            let r = (1.0 + layer.sim_p1 * 2.0) * texel;
            let thresh2 = max(layer.sim_p2 * layer.sim_p2, 0.0005);
            var acc = c;
            var wsum = 1.0;
            for (var i = 0; i < 8; i = i + 1) {
                let ang = f32(i) * 0.785398;
                let s = sample_diffuse(clamp(tc + vec2<f32>(cos(ang), sin(ang)) * r, vec2<f32>(0.0), vec2<f32>(1.0)));
                let dc = s.rgb - c.rgb;
                let w = exp(-dot(dc, dc) / thresh2) * layer.sim_p1;
                acc += s * w;
                wsum += w;
            }
            c = acc / wsum;
        } else if (layer.sim_type == 83u) {
            // PuppetPin IDW mesh warp (up to 4 pins in UV space)
            let pin_count = i32(round(layer.tint_white_b));
            var disp = vec2<f32>(0.0);
            var wsum = 0.0;
            if (pin_count >= 1) {
                let s0 = vec2<f32>(layer.sim_p1, layer.sim_p2);
                let d0 = vec2<f32>(layer.sim_p3, layer.sim_p4);
                let dist = distance(tc, d0);
                let w = 1.0 / (dist * dist + 0.002);
                disp += (s0 - d0) * w;
                wsum += w;
            }
            if (pin_count >= 2) {
                let s1 = vec2<f32>(layer.sim_p5, layer.sim_p6);
                let d1 = vec2<f32>(layer.tint_black_r, layer.tint_black_g);
                let dist = distance(tc, d1);
                let w = 1.0 / (dist * dist + 0.002);
                disp += (s1 - d1) * w;
                wsum += w;
            }
            if (pin_count >= 3) {
                let s2 = vec2<f32>(layer.tint_black_b, layer.tint_black_a);
                let d2 = vec2<f32>(layer.effect_shadow_color_r, layer.effect_shadow_color_g);
                let dist = distance(tc, d2);
                let w = 1.0 / (dist * dist + 0.002);
                disp += (s2 - d2) * w;
                wsum += w;
            }
            if (pin_count >= 4) {
                let s3 = vec2<f32>(layer.effect_shadow_color_b, layer.effect_shadow_color_a);
                let d3 = vec2<f32>(layer.tint_white_r, layer.tint_white_g);
                let dist = distance(tc, d3);
                let w = 1.0 / (dist * dist + 0.002);
                disp += (s3 - d3) * w;
                wsum += w;
            }
            if (wsum > 0.0) {
                let wuv = tc + disp / wsum;
                if (wuv.x < 0.0 || wuv.x > 1.0 || wuv.y < 0.0 || wuv.y > 1.0) {
                    c = vec4<f32>(0.0);
                } else {
                    c = sample_diffuse(wuv);
                }
            }
        } else if (layer.sim_type == 84u) {
            // CloneStamp circular brush dab sampling offset layer texture
            let d = length(local_pos);
            let cov = 1.0 - smoothstep(0.45, 0.5, d);
            let suv = clamp(tc + vec2<f32>(layer.sim_p1, layer.sim_p2), vec2<f32>(0.0), vec2<f32>(1.0));
            let sc = sample_diffuse(suv);
            c = vec4<f32>(sc.rgb, sc.a * cov);
        } else if (layer.sim_type == 85u) {
            // ColorSpaceConvert: 1=LogCToLinear, 2=LinearToLogC, 3=SLog3ToLinear, 4=LinearToSLog3
            let m = i32(round(layer.sim_p1));
            var out_rgb = c.rgb;
            if (m == 1) {
                // ARRI LogC3 to Linear
                let cut = 0.1496582;
                let lo = (c.rgb - 0.092809) / 5.367655;
                let hi = (pow(vec3<f32>(10.0), (c.rgb - 0.385537) / 0.247190) - 0.052272) / 5.555556;
                out_rgb = select(hi, lo, c.rgb <= vec3<f32>(cut));
            } else if (m == 2) {
                // Linear to ARRI LogC3
                let cut = 0.010591;
                let lo = 5.367655 * c.rgb + 0.092809;
                let hi = 0.247190 * log2(max(c.rgb * 5.555556 + 0.052272, vec3<f32>(0.0001))) / log2(10.0) + 0.385537;
                out_rgb = select(hi, lo, c.rgb <= vec3<f32>(cut));
            } else if (m == 3) {
                // Sony S-Log3 to Linear
                let cut = 171.2102946929 / 1023.0;
                let lo = (c.rgb * 1023.0 - 95.0) * 0.01125 / (171.2102946929 - 95.0);
                let hi = pow(vec3<f32>(10.0), (c.rgb * 1023.0 - 420.0) / 261.5) * (0.18 + 0.01) - 0.01;
                out_rgb = select(hi, lo, c.rgb <= vec3<f32>(cut));
            } else if (m == 4) {
                // Linear to Sony S-Log3
                let cut = 0.01125;
                let lo = (c.rgb * (171.2102946929 - 95.0) / 0.01125 + 95.0) / 1023.0;
                let hi = (420.0 + (log2(max((c.rgb + 0.01) / (0.18 + 0.01), vec3<f32>(0.0001))) / log2(10.0)) * 261.5) / 1023.0;
                out_rgb = select(hi, lo, c.rgb < vec3<f32>(cut));
            }
            c = vec4<f32>(clamp(out_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
        } else if (layer.sim_type == 86u || layer.sim_type == 87u) {
            // 2-Pass Separable Gaussian Blur (86u = Horizontal, 87u = Vertical)
            let radius = max(layer.sim_p1, 0.0);
            if (radius > 0.05) {
                let step_px = max(radius / 6.0, 1.0);
                let axis = select(vec2<f32>(0.0, step_px), vec2<f32>(step_px, 0.0), layer.sim_type == 86u) * texel;
                var sum = c * 0.148;
                sum += sample_diffuse(clamp(tc + axis * 1.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.138;
                sum += sample_diffuse(clamp(tc - axis * 1.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.138;
                sum += sample_diffuse(clamp(tc + axis * 2.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.114;
                sum += sample_diffuse(clamp(tc - axis * 2.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.114;
                sum += sample_diffuse(clamp(tc + axis * 3.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.083;
                sum += sample_diffuse(clamp(tc - axis * 3.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.083;
                sum += sample_diffuse(clamp(tc + axis * 4.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.053;
                sum += sample_diffuse(clamp(tc - axis * 4.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.053;
                sum += sample_diffuse(clamp(tc + axis * 5.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.028;
                sum += sample_diffuse(clamp(tc - axis * 5.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.028;
                sum += sample_diffuse(clamp(tc + axis * 6.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.010;
                sum += sample_diffuse(clamp(tc - axis * 6.0, vec2<f32>(0.0), vec2<f32>(1.0))) * 0.010;
                c = sum;
            }
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

