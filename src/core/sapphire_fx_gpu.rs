//! Boris FX Sapphire-class GPU Optical Effects (`wgpu` / WGSL Compute).
//!
//! Implements signature high-end VFX optical shaders in linear `RGBA32F`:
//! - **Multi-Octave Exponential Optical Glow** (`S_Glow`-style inverse-square falloff,
//!   anamorphic X/Y stretch, threshold knee, and warm/cool tint).
//! - **Anamorphic Glint / Spectral Streaks** (`S_Glint`-style directional starburst
//!   with chromatic dispersion).
//! - **Film Halation** (red-orange emulsion scatter around specular highlights).

use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SapphireOpticalParams {
    /// Highlight extraction threshold in linear light (default `0.75`).
    pub threshold: f32,
    /// Soft knee width around `threshold` (default `0.25`).
    pub knee: f32,
    /// Multi-octave optical glow intensity (default `1.2`).
    pub glow_intensity: f32,
    /// Base glow radius in pixels (default `12.0`).
    pub glow_radius: f32,
    /// Anamorphic stretch factor (`>1.0` = wider horizontal streaks, default `1.6`).
    pub anamorphic_ratio: f32,
    /// Glow color tint (default warm white `[1.0, 0.96, 0.90]`).
    pub glow_tint: [f32; 3],
    /// Directional glint / streak intensity (default `0.6`).
    pub glint_intensity: f32,
    /// Primary glint angle in degrees (default `0.0` = horizontal).
    pub glint_angle_deg: f32,
    /// Film halation intensity (red/orange scatter, default `0.35`).
    pub halation_intensity: f32,
}

impl Default for SapphireOpticalParams {
    fn default() -> Self {
        Self {
            threshold: 0.75,
            knee: 0.25,
            glow_intensity: 1.2,
            glow_radius: 12.0,
            anamorphic_ratio: 1.6,
            glow_tint: [1.0, 0.96, 0.90],
            glint_intensity: 0.6,
            glint_angle_deg: 0.0,
            halation_intensity: 0.35,
        }
    }
}

const SAPPHIRE_OPTICAL_WGSL: &str = r#"
struct OpticalUniform {
    width: u32,
    height: u32,
    threshold: f32,
    knee: f32,
    glow_intensity: f32,
    glow_radius: f32,
    anamorphic_ratio: f32,
    glint_intensity: f32,
    glint_cos: f32,
    glint_sin: f32,
    halation_intensity: f32,
    _pad: f32,
    glow_tint: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: OpticalUniform;
@group(0) @binding(1) var<storage, read> src_rgba: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> dst_rgba: array<vec4<f32>>;

fn sample_clamped(x: f32, y: f32) -> vec4<f32> {
    let ix = clamp(i32(round(x)), 0, i32(u.width) - 1);
    let iy = clamp(i32(round(y)), 0, i32(u.height) - 1);
    return src_rgba[u32(iy) * u.width + u32(ix)];
}

fn highlight_extract(c: vec3<f32>) -> vec3<f32> {
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let soft = clamp((luma - (u.threshold - u.knee)) / max(2.0 * u.knee, 1e-4), 0.0, 1.0);
    let knee_weight = soft * soft * (3.0 - 2.0 * soft);
    let excess = max(luma - u.threshold, 0.0) + knee_weight * u.knee * 0.5;
    let scale = excess / max(luma, 1e-5);
    return max(c, vec3<f32>(0.0)) * scale;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= u.width || gid.y >= u.height) {
        return;
    }
    let idx = gid.y * u.width + gid.x;
    let base = src_rgba[idx];
    let fx = f32(gid.x);
    let fy = f32(gid.y);

    // 1. Multi-octave exponential optical glow (4 octaves, 8 taps per ring)
    var glow_acc = vec3<f32>(0.0);
    var glow_wsum = 0.0;
    let base_r = max(u.glow_radius, 1.0);
    let ax = max(u.anamorphic_ratio, 0.25);

    for (var oct = 0; oct < 4; oct++) {
        let oct_scale = pow(2.0, f32(oct) * 0.65);
        let r = base_r * oct_scale * 0.35;
        let oct_w = exp(-f32(oct) * 0.55);
        for (var k = 0; k < 8; k++) {
            let ang = f32(k) * 0.78539816339 + f32(oct) * 0.392699;
            let sx = fx + cos(ang) * r * ax;
            let sy = fy + sin(ang) * r;
            let hi = highlight_extract(sample_clamped(sx, sy).rgb);
            glow_acc += hi * oct_w;
            glow_wsum += oct_w;
        }
    }
    let glow = (glow_acc / max(glow_wsum, 1e-4)) * u.glow_intensity * u.glow_tint.rgb;

    // 2. Directional anamorphic glint with spectral dispersion
    var glint = vec3<f32>(0.0);
    if (u.glint_intensity > 1e-4) {
        let dir = vec2<f32>(u.glint_cos, u.glint_sin);
        var gsum = vec3<f32>(0.0);
        var gw = 0.0;
        for (var s = -6; s <= 6; s++) {
            if (s == 0) {
                continue;
            }
            let dist = f32(s) * (base_r * 0.22);
            let w = exp(-abs(f32(s)) * 0.35);
            // Chromatic shift along the streak
            let sr = highlight_extract(sample_clamped(fx + dir.x * dist * 1.08, fy + dir.y * dist * 1.08).rgb).r;
            let sg = highlight_extract(sample_clamped(fx + dir.x * dist, fy + dir.y * dist).rgb).g;
            let sb = highlight_extract(sample_clamped(fx + dir.x * dist * 0.92, fy + dir.y * dist * 0.92).rgb).b;
            gsum += vec3<f32>(sr, sg, sb) * w;
            gw += w;
        }
        glint = (gsum / max(gw, 1e-4)) * u.glint_intensity;
    }

    // 3. Film halation (warm red-orange emulsion scatter)
    var halation = vec3<f32>(0.0);
    if (u.halation_intensity > 1e-4) {
        var hsum = vec3<f32>(0.0);
        let hr = base_r * 0.5;
        for (var k = 0; k < 6; k++) {
            let ang = f32(k) * 1.04719755;
            let hi = highlight_extract(sample_clamped(fx + cos(ang) * hr, fy + sin(ang) * hr).rgb);
            hsum += hi;
        }
        halation = (hsum / 6.0) * vec3<f32>(1.0, 0.28, 0.06) * u.halation_intensity;
    }

    dst_rgba[idx] = vec4<f32>(base.rgb + glow + glint + halation, base.a);
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct OpticalUniform {
    width: u32,
    height: u32,
    threshold: f32,
    knee: f32,
    glow_intensity: f32,
    glow_radius: f32,
    anamorphic_ratio: f32,
    glint_intensity: f32,
    glint_cos: f32,
    glint_sin: f32,
    halation_intensity: f32,
    _pad: f32,
    glow_tint: [f32; 4],
}

/// Applies Sapphire-class multi-octave optical glow, anamorphic glint, and film halation on GPU.
pub fn apply_sapphire_optical_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[f32],
    width: u32,
    height: u32,
    params: &SapphireOpticalParams,
) -> Option<Vec<f32>> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || rgba.len() != count * 4 {
        return None;
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sapphire_optical_compute"),
        source: wgpu::ShaderSource::Wgsl(SAPPHIRE_OPTICAL_WGSL.into()),
    });

    let (glint_sin, glint_cos) = params.glint_angle_deg.to_radians().sin_cos();
    let uni = OpticalUniform {
        width,
        height,
        threshold: params.threshold,
        knee: params.knee,
        glow_intensity: params.glow_intensity,
        glow_radius: params.glow_radius,
        anamorphic_ratio: params.anamorphic_ratio,
        glint_intensity: params.glint_intensity,
        glint_cos,
        glint_sin,
        halation_intensity: params.halation_intensity,
        _pad: 0.0,
        glow_tint: [
            params.glow_tint[0],
            params.glow_tint[1],
            params.glow_tint[2],
            1.0,
        ],
    };

    let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sapphire_optical_uni"),
        contents: bytemuck::bytes_of(&uni),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let src_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sapphire_optical_src"),
        contents: bytemuck::cast_slice(rgba),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let byte_len = (count * 4 * std::mem::size_of::<f32>()) as u64;
    let dst_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sapphire_optical_dst"),
        size: byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sapphire_optical_staging"),
        size: byte_len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sapphire_optical_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sapphire_optical_layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("sapphire_optical_pipeline"),
        layout: Some(&layout),
        module: &shader,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sapphire_optical_bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uni_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: src_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: dst_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sapphire_optical_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sapphire_optical_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&dst_buf, 0, &staging, 0, byte_len);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    let data = slice.get_mapped_range();
    let floats: &[f32] = bytemuck::cast_slice(&data);
    let out = floats[..count * 4].to_vec();
    drop(data);
    staging.unmap();
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};
        struct W(std::thread::Thread);
        impl Wake for W {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
            fn wake_by_ref(self: &Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(W(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut fut = std::pin::pin!(fut);
        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => std::thread::park(),
            }
        }
    }

    #[test]
    fn gpu_sapphire_optical_blooms_hdr_highlight_anamorphically() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let w = 32u32;
        let h = 32u32;
        let mut img = vec![0.0_f32; (w * h * 4) as usize];
        for i in 0..(w * h) as usize {
            img[i * 4 + 3] = 1.0;
        }
        // Bright HDR specular impulse at center (16, 16)
        let center = (16 * w + 16) as usize * 4;
        img[center] = 8.0;
        img[center + 1] = 8.0;
        img[center + 2] = 8.0;

        let params = SapphireOpticalParams {
            glow_radius: 8.0,
            anamorphic_ratio: 2.0,
            glint_intensity: 1.0,
            halation_intensity: 0.5,
            ..Default::default()
        };

        let out = apply_sapphire_optical_gpu(&device, &queue, &img, w, h, &params)
            .expect("sapphire optical");

        // Horizontal neighbor (20, 16) should receive anamorphic streak + red halation
        let horiz = (16 * w + 20) as usize * 4;
        assert!(out[horiz] > 0.05, "expected horizontal glow/streak, got {}", out[horiz]);
        // Red channel should exceed blue due to film halation + warm tint
        assert!(out[horiz] > out[horiz + 2]);
    }
}
