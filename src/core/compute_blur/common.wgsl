// Compute-shader separable Gaussian blur (ping-pong storage buffers).
//
// Buffer-based instead of texture-based so it stays inside core WebGPU
// features (write-only storage is guaranteed; no adapter-specific format
// features needed) and works identically on Metal / Vulkan / DX12.
//
// Pass model:
//   dispatch 1: horizontal pass, src -> mid
//   dispatch 2: vertical pass,   mid -> dst
// The Rust host alternates which buffers are bound for repeated passes.
//
// Colors are premultiplied before weighting and unpremultiplied after, so
// transparent pixels never bleed their color into the blurred result.

struct Params {
    width: u32,
    height: u32,
    radius: u32,
    mode: u32,
    angle: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    hue_shift: f32,
    param_f3: f32,
    param_f4: f32,
    param_f5: f32,
    param_f6: f32,
    param_f7: f32,
    param_f8: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> src: array<u32>;
@group(0) @binding(2) var<storage, read_write> dst: array<u32>;
@group(0) @binding(3) var<storage, read> kernel: array<f32>;

fn unpack(px: u32) -> vec4<f32> {
    let r = f32((px >> 24u) & 0xFFu) / 255.0;
    let g = f32((px >> 16u) & 0xFFu) / 255.0;
    let b = f32((px >> 8u) & 0xFFu) / 255.0;
    let a = f32(px & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn pack(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r, 0.0, 1.0) * 255.0 + 0.5);
    let g = u32(clamp(c.g, 0.0, 1.0) * 255.0 + 0.5);
    let b = u32(clamp(c.b, 0.0, 1.0) * 255.0 + 0.5);
    let a = u32(clamp(c.a, 0.0, 1.0) * 255.0 + 0.5);
    return (r << 24u) | (g << 16u) | (b << 8u) | a;
}

// ── Color correction kernel ──
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let d = mx - mn;
    var h: f32 = 0.0;
    let s = select(0.0, d / max(mx, 0.0001), mx > 0.0001);
    let v = mx;
    if d > 0.0001 {
        if mx == c.r { h = (c.g - c.b) / d; }
        else if mx == c.g { h = 2.0 + (c.b - c.r) / d; }
        else { h = 4.0 + (c.r - c.g) / d; }
        h = h / 6.0;
        if h < 0.0 { h = h + 1.0; }
    }
    return vec3<f32>(h, s, v);
}

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;
    var rgb: vec3<f32>;
    if h < 1.0 { rgb = vec3<f32>(c, x, 0.0); }
    else if h < 2.0 { rgb = vec3<f32>(x, c, 0.0); }
    else if h < 3.0 { rgb = vec3<f32>(0.0, c, x); }
    else if h < 4.0 { rgb = vec3<f32>(0.0, x, c); }
    else if h < 5.0 { rgb = vec3<f32>(x, 0.0, c); }
    else { rgb = vec3<f32>(c, 0.0, x); }
    return rgb + vec3<f32>(m);
}

fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

