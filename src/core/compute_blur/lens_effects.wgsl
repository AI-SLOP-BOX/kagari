fn apply_lens_effects(gid: vec3<u32>) -> bool {
    let x = gid.x;
    let y = gid.y;
    let w = params.width;
    let h = params.height;
    // ── Mode 40: CcLens ──
    if (params.mode == 40u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let convergence = params.brightness;
        let zoom = params.contrast;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = d / max_d;
        let new_d = pow(t, max(convergence / 100.0, 0.01)) * max_d * zoom;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 41: PolarCoordinates ──
    if (params.mode == 41u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let to_polar = params.brightness > 0.5;
        if (to_polar) {
            let dx = (f32(x) - cx) / cx;
            let dy = (f32(y) - cy) / cy;
            let r = sqrt(dx * dx + dy * dy);
            let theta = (atan2(dy, dx) + 3.14159265) / 6.2831853;
            let sx = u32(clamp(i32(theta * f32(w)), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(r * f32(h)), 0, i32(h - 1u)));
            dst[y * w + x] = src[sy * w + sx];
        } else {
            let theta = (f32(x) / f32(w) - 0.5) * 6.2831853;
            let r = f32(y) / f32(h);
            let sx = u32(clamp(i32(cx + cos(theta) * r * cx), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(cy + sin(theta) * r * cy), 0, i32(h - 1u)));
            dst[y * w + x] = src[sy * w + sx];
        }
        return true;
    }

    // ── Mode 42: OpticsCompensation ──
    if (params.mode == 42u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let fov = params.brightness;
        let reverse = params.contrast > 0.5;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d2 = dx * dx + dy * dy;
        let k = fov / 100.0;
        let distortion = 1.0 + k * d2;
        let dist = select(distortion, 1.0 / max(distortion, 0.001), reverse);
        let sx = u32(clamp(i32(cx + dx * dist * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * dist * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 43: Fisheye ──
    if (params.mode == 43u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let strength = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = d / max_d;
        let new_d = sin(t * 3.14159265 * 0.5 * strength) * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 44: LensCorrection ──
    if (params.mode == 44u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let k1 = params.brightness;
        let k2 = params.contrast;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let r2 = dx * dx + dy * dy;
        let r4 = r2 * r2;
        let distortion = 1.0 + k1 * r2 + k2 * r4;
        let sx = u32(clamp(i32(cx + dx * distortion * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * distortion * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 45: Vortex ──
    if (params.mode == 45u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let angle = params.brightness;
        let radius_param = max(params.contrast, 1.0);
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_r = f32(min(w, h)) * 0.5;
        let t = clamp(d / (max_r * radius_param / 100.0), 0.0, 1.0);
        let swirl = angle * t * t;
        let cos_a = cos(swirl);
        let sin_a = sin(swirl);
        let rx = dx * cos_a - dy * sin_a + cx;
        let ry = dx * sin_a + dy * cos_a + cy;
        let sx = u32(clamp(i32(rx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ry), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 46: PinchPunch ──
    if (params.mode == 46u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let strength = params.brightness;
        let dx = (f32(x) - cx) / (f32(w) * 0.5);
        let dy = (f32(y) - cy) / (f32(h) * 0.5);
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = clamp(d / max_d, 0.0, 1.0);
        let new_d = pow(t, 1.0 + strength) * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * f32(w) * 0.5), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * f32(h) * 0.5), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 47: RefractionLens ──
    if (params.mode == 47u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let ior = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = clamp(d / max_d, 0.0, 1.0);
        let refracted = 1.0 / max(ior, 0.001);
        let new_d = t * refracted * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 48: BendIt ──
    if (params.mode == 48u) {
        let top_off = params.brightness;
        let bot_off = params.contrast;
        let py = f32(y) / f32(h);
        let offset = mix(top_off, bot_off, py);
        let sx = u32(clamp(i32(f32(x) + offset), 0, i32(w - 1u)));
        dst[y * w + x] = src[y * w + sx];
        return true;
    }

    // ── Mode 49: Tiler ──
    if (params.mode == 49u) {
        let scale = max(params.brightness, 0.01);
        let tx = (f32(x) / f32(w)) * scale;
        let ty = (f32(y) / f32(h)) * scale;
        let sx = u32(clamp(i32(tx * f32(w)) % i32(w), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ty * f32(h)) % i32(h), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 50: DirectionalSharpen ──
    if (params.mode == 50u) {
        let ix = i32(x);
        let iy = i32(y);
        let angle = params.brightness;
        let dx = cos(angle);
        let dy = sin(angle);
        let c0 = unpack(src[y * w + x]);
        let s1 = unpack(src[u32(clamp(iy + i32(dy), 0, i32(h-1u))) * w + u32(clamp(ix + i32(dx), 0, i32(w-1u)))]);
        let s2 = unpack(src[u32(clamp(iy - i32(dy), 0, i32(h-1u))) * w + u32(clamp(ix - i32(dx), 0, i32(w-1u)))]);
        let sharpen = max(params.contrast, 0.0);
        let rgb = c0.rgb + (c0.rgb * 2.0 - s1.rgb - s2.rgb) * sharpen;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return true;
    }

    // ── Mode 51: Halftone ──
    if (params.mode == 51u) {
        let c = unpack(src[y * w + x]);
        let cell = max(params.brightness, 2.0);
        let cx = (floor(f32(x) / cell) + 0.5) * cell;
        let cy = (floor(f32(y) / cell) + 0.5) * cell;
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let d = sqrt(dx * dx + dy * dy);
        let r = cell * 0.5 * sqrt(lum);
        let v = select(1.0, 0.0, d < r);
        dst[y * w + x] = pack(vec4<f32>(v, v, v, c.a));
        return true;
    }

    // ── Mode 52: Mosaic ──
    if (params.mode == 52u) {
        let bw = max(params.brightness, 1.0);
        let bh = max(params.contrast, 1.0);
        let cx = (floor(f32(x) / bw) + 0.5) * bw;
        let cy = (floor(f32(y) / bh) + 0.5) * bh;
        let sx = u32(clamp(i32(cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return true;
    }

    // ── Mode 53: CrossHatch ──
    if (params.mode == 53u) {
        let c = unpack(src[y * w + x]);
        let gap = max(params.brightness, 2.0);
        let threshold = params.contrast;
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        var v: f32 = 1.0;
        if (lum < threshold) {
            if ((f32(x) + f32(y)) % gap < 1.0) { v = 0.0; }
            if ((f32(x) - f32(y) + f32(h)) % gap < 1.0) { v = 0.0; }
        }
        dst[y * w + x] = pack(vec4<f32>(c.rgb * v, c.a));
        return true;
    }

    // ── Mode 54: Colorama (luminance to cyclic palette) ──
    if (params.mode == 54u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let h = lum + params.brightness;
        let rgb = hsv_to_rgb(vec3<f32>(fract(h), params.contrast, params.saturation));
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return true;
    }

    // ── Mode 55: BevelAlpha ──
    if (params.mode == 55u) {
        let ix = i32(x);
        let iy = i32(y);
        let depth = params.brightness;
        let angle = params.contrast;
        let a_l = unpack(src[y * w + u32(clamp(ix - 1, 0, i32(w-1u)))]).a;
        let a_r = unpack(src[y * w + u32(clamp(ix + 1, 0, i32(w-1u)))]).a;
        let a_t = unpack(src[u32(clamp(iy - 1, 0, i32(h-1u))) * w + x]).a;
        let a_b = unpack(src[u32(clamp(iy + 1, 0, i32(h-1u))) * w + x]).a;
        let dx = a_r - a_l;
        let dy = a_b - a_t;
        let light = cos(angle) * dx + sin(angle) * dy;
        let bevel = clamp(light * depth + 0.5, 0.0, 1.0);
        let c0 = unpack(src[y * w + x]);
        let rgb = mix(c0.rgb, vec3<f32>(bevel), abs(light) * depth);
        dst[y * w + x] = pack(vec4<f32>(rgb, c0.a));
        return true;
    }

    // ── Mode 56: ChromaKey ──
    if (params.mode == 56u) {
        let c = unpack(src[y * w + x]);
        let key = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let gain = params.hue_shift;
        let d = distance(c.rgb, key);
        let a = smoothstep(gain, gain + params.param_f3, d);
        dst[y * w + x] = pack(vec4<f32>(c.rgb, c.a * a));
        return true;
    }

    // ── Mode 57: ColorSpaceConvert (RGB<->HSV) ──
    if (params.mode == 57u) {
        let c = unpack(src[y * w + x]);
        let mode = u32(params.brightness);
        if (mode == 0u) {
            let hsv = rgb_to_hsv(c.rgb);
            dst[y * w + x] = pack(vec4<f32>(hsv, c.a));
        } else {
            let rgb = hsv_to_rgb(c.rgb);
            dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        }
        return true;
    }

    // ── Mode 58: FilmGrain ──
    if (params.mode == 58u) {
        let c = unpack(src[y * w + x]);
        let intensity = params.brightness;
        let seed = params.param_f5;
        let noise = hash2(vec2<f32>(f32(x) + seed, f32(y) + seed * 1.7)) * 2.0 - 1.0;
        let rgb = c.rgb + noise * intensity;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return true;
    }

    // ── Mode 59: FractalNoise ──
    if (params.mode == 59u) {
        let scale = max(params.brightness, 0.001);
        let evolution = params.contrast;
        let nx = f32(x) * scale / f32(w) + evolution;
        let ny = f32(y) * scale / f32(h) + evolution * 0.7;
        var v = 0.0;
        var amp = 0.5;
        var freq = 1.0;
        for (var i: u32 = 0u; i < 6u; i++) {
            v += amp * hash2(vec2<f32>(nx * freq, ny * freq));
            freq *= 2.0;
            amp *= 0.5;
        }
        let c0 = unpack(src[y * w + x]);
        let rgb = mix(c0.rgb, vec3<f32>(v), params.saturation);
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return true;
    }

    // ── Mode 60: GlitchDisplacement ──
    if (params.mode == 60u) {
        let seed = params.param_f5;
        let amount = params.brightness;
        let block_h = 8.0;
        let row = floor(f32(y) / block_h);
        let h_val = hash2(vec2<f32>(row, seed));
        if (h_val > (1.0 - amount)) {
            let offset = i32((hash2(vec2<f32>(row + 1.0, seed)) - 0.5) * amount * f32(w) * 0.3);
            let sx = u32(clamp(i32(x) + offset, 0, i32(w - 1u)));
            dst[y * w + x] = src[y * w + sx];
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return true;
    }

    // ── Mode 61: ScanlineGlitch ──
    if (params.mode == 61u) {
        let seed = params.param_f5;
        let amount = params.brightness;
        let h_val = hash2(vec2<f32>(f32(y), seed));
        if (h_val > (1.0 - amount * 0.3)) {
            let offset = i32((hash2(vec2<f32>(f32(y) + 3.0, seed)) - 0.5) * amount * f32(w) * 0.1);
            let sx = u32(clamp(i32(x) + offset, 0, i32(w - 1u)));
            dst[y * w + x] = src[y * w + sx];
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return true;
    }

    // ── Mode 62: ReflectionMap ──
    if (params.mode == 62u) {
        let reflect_y = params.brightness > 0.5;
        let fade = params.contrast;
        var sy: u32;
        var fade_factor: f32;
        if (reflect_y) {
            sy = u32(clamp(i32(f32(h) - f32(y)), 0, i32(h - 1u)));
            fade_factor = 1.0 - fade * (f32(y) / f32(h));
        } else {
            sy = y;
            fade_factor = 1.0 - fade * (f32(x) / f32(w));
        }
        let c = unpack(src[sy * w + x]);
        dst[y * w + x] = pack(vec4<f32>(c.rgb * clamp(fade_factor, 0.0, 1.0), c.a));
        return true;
    }

    return false;
}

