//! Deterministic CPU fallbacks for the built-in custom shader templates.
//!
//! Arbitrary WGSL still requires a GPU pipeline. The built-in templates have
//! equivalent CPU kernels so previews and exports do not silently diverge.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinShader {
    CrtScanlines,
    ChromaticTwist,
    NeonEdge,
    DefaultTemplate,
}

fn builtin_shader(source: &str) -> Option<BuiltinShader> {
    if source.contains("CRT Scanlines Shader") {
        Some(BuiltinShader::CrtScanlines)
    } else if source.contains("Chromatic Twist Shader") {
        Some(BuiltinShader::ChromaticTwist)
    } else if source.contains("Neon Edge Glow") {
        Some(BuiltinShader::NeonEdge)
    } else if source.contains("Custom WGSL VFX Shader Template") {
        Some(BuiltinShader::DefaultTemplate)
    } else {
        None
    }
}

pub fn cpu_supported(source: &str) -> bool {
    builtin_shader(source).is_some()
}

pub fn apply(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    source: &str,
    uniform_values: &[f32],
    frame: u32,
    fps: u32,
) -> bool {
    let Some(shader) = builtin_shader(source) else {
        return false;
    };
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|size| size.checked_mul(4))
        .unwrap_or(0);
    if expected == 0 || pixels.len() != expected {
        return true;
    }

    let original = pixels.to_vec();
    match shader {
        BuiltinShader::CrtScanlines => apply_crt(pixels, width, height, &original, uniform_values),
        BuiltinShader::ChromaticTwist => {
            apply_chromatic_twist(pixels, width, height, &original, uniform_values)
        }
        BuiltinShader::NeonEdge => {
            apply_neon_edge(pixels, width, height, &original, uniform_values)
        }
        BuiltinShader::DefaultTemplate => {
            apply_default_template(pixels, width, height, &original, uniform_values, frame, fps)
        }
    }
    true
}

fn uniform(values: &[f32], index: usize, default: f32) -> f32 {
    values
        .get(index)
        .copied()
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn uv(x: u32, y: u32, width: u32, height: u32) -> [f32; 2] {
    [
        if width > 1 {
            x as f32 / (width - 1) as f32
        } else {
            0.5
        },
        if height > 1 {
            y as f32 / (height - 1) as f32
        } else {
            0.5
        },
    ]
}

fn sample_linear(pixels: &[u8], width: u32, height: u32, uv: [f32; 2]) -> [f32; 4] {
    if width == 0 || height == 0 {
        return [0.0; 4];
    }
    let x = uv[0].clamp(0.0, 1.0) * (width.saturating_sub(1)) as f32;
    let y = uv[1].clamp(0.0, 1.0) * (height.saturating_sub(1)) as f32;
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let p00 = pixel(pixels, width, x0, y0);
    let p10 = pixel(pixels, width, x1, y0);
    let p01 = pixel(pixels, width, x0, y1);
    let p11 = pixel(pixels, width, x1, y1);
    let mut out = [0.0; 4];
    for channel in 0..4 {
        let top = p00[channel] * (1.0 - tx) + p10[channel] * tx;
        let bottom = p01[channel] * (1.0 - tx) + p11[channel] * tx;
        out[channel] = top * (1.0 - ty) + bottom * ty;
    }
    out
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [f32; 4] {
    let index = ((y * width + x) * 4) as usize;
    [
        pixels[index] as f32 / 255.0,
        pixels[index + 1] as f32 / 255.0,
        pixels[index + 2] as f32 / 255.0,
        pixels[index + 3] as f32 / 255.0,
    ]
}

fn write_pixel(pixels: &mut [u8], width: u32, x: u32, y: u32, color: [f32; 4]) {
    let index = ((y * width + x) * 4) as usize;
    for (channel, value) in color.into_iter().enumerate() {
        pixels[index + channel] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0).max(f32::EPSILON)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn apply_crt(pixels: &mut [u8], width: u32, height: u32, source: &[u8], uniforms: &[f32]) {
    let density = uniform(uniforms, 0, 1.0).max(0.0);
    let opacity = uniform(uniforms, 1, 0.4).clamp(0.0, 1.0);
    for y in 0..height {
        for x in 0..width {
            let color = pixel(source, width, x, y);
            let v = uv(x, y, width, height)[1];
            let scanline = (v * (density * 400.0 + 100.0)).sin() * 0.5 + 0.5;
            let factor = 1.0 - opacity + scanline * opacity;
            write_pixel(
                pixels,
                width,
                x,
                y,
                [
                    color[0] * factor,
                    color[1] * factor,
                    color[2] * factor,
                    color[3],
                ],
            );
        }
    }
}

fn apply_chromatic_twist(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    source: &[u8],
    uniforms: &[f32],
) {
    let strength = uniform(uniforms, 0, 1.5);
    let radius = uniform(uniforms, 1, 0.5).abs().max(0.0001);
    for y in 0..height {
        for x in 0..width {
            let input_uv = uv(x, y, width, height);
            let dx = input_uv[0] - 0.5;
            let dy = input_uv[1] - 0.5;
            let dist = (dx * dx + dy * dy).sqrt();
            let twist = (1.0 - smoothstep(0.0, radius, dist)) * strength;
            let angle = dy.atan2(dx) + twist;
            let twisted_uv = [0.5 + angle.cos() * dist, 0.5 + angle.sin() * dist];
            let color = sample_linear(source, width, height, twisted_uv);
            write_pixel(pixels, width, x, y, color);
        }
    }
}

fn apply_neon_edge(pixels: &mut [u8], width: u32, height: u32, source: &[u8], uniforms: &[f32]) {
    let threshold = uniform(uniforms, 0, 0.5);
    let intensity = uniform(uniforms, 1, 2.0).max(0.0);
    let dx = if width > 1 { 0.002 } else { 0.0 };
    let dy = if height > 1 { 0.002 } else { 0.0 };
    for y in 0..height {
        for x in 0..width {
            let input_uv = uv(x, y, width, height);
            let color = sample_linear(source, width, height, input_uv);
            let right = sample_linear(source, width, height, [input_uv[0] + dx, input_uv[1]]);
            let down = sample_linear(source, width, height, [input_uv[0], input_uv[1] + dy]);
            let diff = ((color[0] - right[0]).powi(2)
                + (color[1] - right[1]).powi(2)
                + (color[2] - right[2]).powi(2))
            .sqrt()
                + ((color[0] - down[0]).powi(2)
                    + (color[1] - down[1]).powi(2)
                    + (color[2] - down[2]).powi(2))
                .sqrt();
            let neon = smoothstep(threshold * 0.1, 0.5, diff) * intensity;
            write_pixel(
                pixels,
                width,
                x,
                y,
                [
                    color[0] + neon * 0.1,
                    color[1] + neon * 0.9,
                    color[2] + neon,
                    color[3],
                ],
            );
        }
    }
}

fn apply_default_template(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    source: &[u8],
    uniforms: &[f32],
    frame: u32,
    fps: u32,
) {
    let time = frame as f32 / fps.max(1) as f32;
    let p0 = uniform(uniforms, 0, 1.0);
    let p1 = uniform(uniforms, 1, 1.0);
    let p2 = uniform(uniforms, 2, 1.0);
    let mix_amount = uniform(uniforms, 3, 0.5).clamp(0.0, 1.0);
    for y in 0..height {
        for x in 0..width {
            let input_uv = uv(x, y, width, height);
            let dx = input_uv[0] - 0.5;
            let dy = input_uv[1] - 0.5;
            let dist = (dx * dx + dy * dy).sqrt();
            let wave = (dist * 20.0 - time * 4.0).sin() * 0.5 + 0.5;
            let color = pixel(source, width, x, y);
            let tint = [wave * p0, (1.0 - wave) * p1, wave * p2, 1.0];
            write_pixel(
                pixels,
                width,
                x,
                y,
                [
                    color[0] * (1.0 - mix_amount) + tint[0] * mix_amount,
                    color[1] * (1.0 - mix_amount) + tint[1] * mix_amount,
                    color[2] * (1.0 - mix_amount) + tint[2] * mix_amount,
                    color[3],
                ],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CRT: &str = "// CRT Scanlines Shader";
    const TWIST: &str = "// Chromatic Twist Shader";
    const NEON: &str = "// Neon Edge Glow";
    const DEFAULT: &str = "// Custom WGSL VFX Shader Template";

    #[test]
    fn built_in_templates_have_cpu_fallbacks() {
        for source in [CRT, TWIST, NEON, DEFAULT] {
            assert!(cpu_supported(source));
        }
        assert!(!cpu_supported(
            "@fragment fn fs_main() -> vec4<f32> { return vec4<f32>(1.0); }"
        ));
    }

    #[test]
    fn built_in_templates_modify_pixels_deterministically() {
        let mut source = vec![0u8; 16 * 16 * 4];
        for y in 0..16u32 {
            for x in 0..16u32 {
                let index = ((y * 16 + x) * 4) as usize;
                source[index] = (x * 17) as u8;
                source[index + 1] = (y * 17) as u8;
                source[index + 2] = ((x + y) * 8) as u8;
                source[index + 3] = 255;
            }
        }
        for shader in [CRT, TWIST, NEON, DEFAULT] {
            let input = if shader == NEON {
                let mut edge = vec![0u8; 16 * 16 * 4];
                for y in 0..16u32 {
                    for x in 8..16u32 {
                        let index = ((y * 16 + x) * 4) as usize;
                        edge[index..index + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
                edge
            } else {
                source.clone()
            };
            let mut first = input.clone();
            let mut second = input.clone();
            let uniforms = if shader == NEON {
                [0.0, 2.0, 0.8, 0.6]
            } else {
                [1.0, 0.5, 0.8, 0.6]
            };
            assert!(apply(&mut first, 16, 16, shader, &uniforms, 12, 24));
            assert!(apply(&mut second, 16, 16, shader, &uniforms, 12, 24));
            assert_eq!(first, second);
            assert_ne!(
                first, input,
                "template {shader} must have a visible CPU result"
            );
        }
    }

    #[test]
    fn unsupported_shader_is_not_claimed_as_cpu_supported() {
        let mut pixels = vec![128u8; 4 * 4 * 4];
        let original = pixels.clone();
        assert!(!apply(
            &mut pixels,
            4,
            4,
            "return vec4<f32>(src.b, src.g, src.r, src.a);",
            &[],
            0,
            30,
        ));
        assert_eq!(pixels, original);
    }
}
