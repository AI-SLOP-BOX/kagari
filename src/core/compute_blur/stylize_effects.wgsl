fn apply_stylize_effects(gid: vec3<u32>) -> bool {
    let x = gid.x;
    let y = gid.y;
    let w = params.width;
    let h = params.height;
    // ── Mode 23: Chromatic Aberration ──
    if (params.mode == 23u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let amount = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let dist2 = dx * dx + dy * dy;
        let offset = dist2 * amount;
        let r_x = u32(clamp(i32(f32(x) + dx * offset * 2.0), 0, i32(w - 1u)));
        let r_y = u32(clamp(i32(f32(y) + dy * offset * 2.0), 0, i32(h - 1u)));
        let g_x = x;
        let g_y = y;
        let b_x = u32(clamp(i32(f32(x) - dx * offset * 2.0), 0, i32(w - 1u)));
        let b_y = u32(clamp(i32(f32(y) - dy * offset * 2.0), 0, i32(h - 1u)));
        let r = unpack(src[r_y * w + r_x]).r;
        let g = unpack(src[g_y * w + g_x]).g;
        let b = unpack(src[b_y * w + b_x]).b;
        let a = unpack(src[y * w + x]).a;
        dst[y * w + x] = pack(vec4<f32>(r, g, b, a));
        return true;
    }

    // ── Mode 24: Vignette ──
    if (params.mode == 24u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let radius = params.brightness;
        let softness = max(params.contrast, 0.01);
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let v = smoothstep(radius, radius + softness, d);
        let c = unpack(src[y * w + x]);
        let rgb = c.rgb * (1.0 - v);
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return true;
    }

    // ── Mode 25: Minimax (dilate/erode) ──
    if (params.mode == 25u) {
        let radius = max(params.radius, 1u);
        let mode_flag = params.param_f5;
        var extreme: f32 = select(0.0, 1.0, mode_flag > 0.5);
        let c0 = unpack(src[y * w + x]);
        var result = c0;
        for (var dy: i32 = -i32(radius); dy <= i32(radius); dy++) {
            for (var dx: i32 = -i32(radius); dx <= i32(radius); dx++) {
                let sx = u32(clamp(i32(x) + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(i32(y) + dy, 0, i32(h - 1u)));
                let c = unpack(src[sy * w + sx]);
                if (mode_flag > 0.5) {
                    result.r = max(result.r, c.r);
                    result.g = max(result.g, c.g);
                    result.b = max(result.b, c.b);
                } else {
                    result.r = min(result.r, c.r);
                    result.g = min(result.g, c.g);
                    result.b = min(result.b, c.b);
                }
            }
        }
        dst[y * w + x] = pack(result);
        return true;
    }

    // ── Mode 26: LinearWipe ──
    if (params.mode == 26u) {
        let c0 = unpack(src[y * w + x]);
        let completion = clamp(params.brightness, 0.0, 1.0);
        let angle_rad = params.angle;
        let nx = cos(angle_rad);
        let ny = sin(angle_rad);
        let px = f32(x) / max(f32(w) - 1.0, 1.0);
        let py = f32(y) / max(f32(h) - 1.0, 1.0);
        let p0 = 0.0;
        let p1 = nx;
        let p2 = ny;
        let p3 = nx + ny;
        let min_proj = min(min(p0, p1), min(p2, p3));
        let max_proj = max(max(p0, p1), max(p2, p3));
        let d = (px * nx + py * ny - min_proj) / max(max_proj - min_proj, 0.0001);
        let edge = completion;
        let feather = max(params.contrast, 0.001);
        let a = clamp((edge - d) / feather, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, c0.a * a));
        return true;
    }

    // ── Mode 27: SimpleChoker (alpha expand/contract) ──
    if (params.mode == 27u) {
        let c0 = unpack(src[y * w + x]);
        let amount = params.brightness;
        var max_a: f32 = 0.0;
        let r = u32(abs(amount)) + 1u;
        for (var dy: i32 = -i32(r); dy <= i32(r); dy++) {
            for (var dx: i32 = -i32(r); dx <= i32(r); dx++) {
                let sx = u32(clamp(i32(x) + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(i32(y) + dy, 0, i32(h - 1u)));
                let a = unpack(src[sy * w + sx]).a;
                max_a = max(max_a, a);
            }
        }
        let new_a = select(max_a, 1.0 - max_a, amount < 0.0);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, clamp(new_a, 0.0, 1.0)));
        return true;
    }

    // ── Mode 28: ShiftChannels ──
    if (params.mode == 28u) {
        let c = unpack(src[y * w + x]);
        let take_r = u32(params.brightness);
        let take_g = u32(params.contrast);
        let take_b = u32(params.saturation);
        let take_a = u32(params.hue_shift);
        var channels = array<f32, 4>(c.r, c.g, c.b, c.a);
        let r = channels[clamp(take_r, 0u, 3u)];
        let g = channels[clamp(take_g, 0u, 3u)];
        let b = channels[clamp(take_b, 0u, 3u)];
        let a = channels[clamp(take_a, 0u, 3u)];
        dst[y * w + x] = pack(vec4<f32>(r, g, b, a));
        return true;
    }

    // ── Mode 29: ColorBalance ──
    if (params.mode == 29u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let shadows_w = clamp(1.0 - lum * 2.0, 0.0, 1.0);
        let highlights_w = clamp(lum * 2.0 - 1.0, 0.0, 1.0);
        let midtones_w = 1.0 - shadows_w - highlights_w;
        let shift = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let rgb = c.rgb + shift * shadows_w + shift * midtones_w * 0.5 + shift * highlights_w;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return true;
    }

    // ── Mode 30: Vibrance ──
    if (params.mode == 30u) {
        let c = unpack(src[y * w + x]);
        let amount = params.brightness;
        let hsv = rgb_to_hsv(c.rgb);
        let boost = amount * (1.0 - hsv.y);
        let s = clamp(hsv.y + boost, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(hsv_to_rgb(vec3<f32>(hsv.x, s, hsv.z)), c.a));
        return true;
    }

    // ── Mode 31: WhiteBalance ──
    if (params.mode == 31u) {
        let c = unpack(src[y * w + x]);
        let temperature = params.brightness;
        let tint = params.contrast;
        var rgb = c.rgb;
        rgb.r = rgb.r + temperature * 0.1;
        rgb.b = rgb.b - temperature * 0.1;
        rgb.g = rgb.g + tint * 0.05;
        rgb.r = rgb.r + tint * 0.05;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return true;
    }

    // ── Mode 32: HslAdjust ──
    if (params.mode == 32u) {
        let c = unpack(src[y * w + x]);
        let hsv = rgb_to_hsv(c.rgb);
        var h = hsv.x + params.brightness / 360.0;
        if (h < 0.0) { h = h + 1.0; }
        if (h > 1.0) { h = h - 1.0; }
        let s = clamp(hsv.y * (1.0 + params.contrast), 0.0, 1.0);
        let v = clamp(hsv.z + params.saturation, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(hsv_to_rgb(vec3<f32>(h, s, v)), c.a));
        return true;
    }

    // ── Mode 33: CrtScanlines ──
    if (params.mode == 33u) {
        let c = unpack(src[y * w + x]);
        let spacing = max(params.brightness, 1.0);
        let intensity = params.contrast;
        let mask = select(1.0, 1.0 - intensity, (f32(y) % spacing) < 1.0);
        dst[y * w + x] = pack(vec4<f32>(c.rgb * mask, c.a));
        return true;
    }

    // ── Mode 34: AlphaFromLuminance ──
    if (params.mode == 34u) {
        let c = unpack(src[y * w + x]);
        var lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (params.brightness > 0.5) { lum = 1.0 - lum; }
        dst[y * w + x] = pack(vec4<f32>(c.rgb, lum));
        return true;
    }

    // ── Mode 35: LumaKeyRange ──
    if (params.mode == 35u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let low = params.brightness;
        let high = params.contrast;
        var key: f32;
        if (params.saturation > 0.5) {
            key = select(0.0, 1.0, lum < low || lum > high);
        } else {
            key = select(0.0, 1.0, lum >= low && lum <= high);
        }
        dst[y * w + x] = pack(vec4<f32>(c.rgb, c.a * key));
        return true;
    }

    // ── Mode 36: VenetianBlinds ──
    if (params.mode == 36u) {
        let c0 = unpack(src[y * w + x]);
        let completion = clamp(params.brightness, 0.0, 1.0);
        let blind_w = max(params.contrast, 2.0);
        let pos = f32(y) % blind_w;
        let blind = step(blind_w * (1.0 - completion), pos);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, c0.a * blind));
        return true;
    }

    // ── Mode 37: Tritone ──
    if (params.mode == 37u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let shadow = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let mid = vec3<f32>(params.hue_shift, params.param_f3, params.param_f4);
        let highlight = vec3<f32>(params.param_f5, params.param_f6, params.param_f7);
        var rgb: vec3<f32>;
        if (lum < 0.5) {
            rgb = mix(shadow, mid, lum * 2.0);
        } else {
            rgb = mix(mid, highlight, (lum - 0.5) * 2.0);
        }
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return true;
    }

    // ── Mode 38: GradientMap ──
    if (params.mode == 38u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let low = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let mid = vec3<f32>(params.hue_shift, params.param_f3, params.param_f4);
        let high = vec3<f32>(params.param_f5, params.param_f6, params.param_f7);
        var rgb: vec3<f32>;
        if (lum < 0.5) {
            rgb = mix(low, mid, lum * 2.0);
        } else {
            rgb = mix(mid, high, (lum - 0.5) * 2.0);
        }
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return true;
    }

    // ── Mode 39: Letterbox ──
    if (params.mode == 39u) {
        let frac = params.brightness;
        let py = f32(y) / f32(h);
        if (py < frac || py > 1.0 - frac) {
            dst[y * w + x] = pack(vec4<f32>(0.0, 0.0, 0.0, 1.0));
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return true;
    }

    return false;
}

