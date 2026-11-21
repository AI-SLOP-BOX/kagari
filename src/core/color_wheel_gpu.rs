//! DaVinci-style 3-Way Color Wheels (Lift / Gamma / Gain / Offset) + Pivot Contrast
//! in linear `RGBA32F` with CPU reference and WGSL GPU compute execution.

/// Primary color grading parameters for 3-Way Color Wheels + Pivot Contrast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorWheelGradeParams {
    /// `[r, g, b, master]` shadow lift (default `[0, 0, 0, 0]`).
    pub lift: [f32; 4],
    /// `[r, g, b, master]` midtone gamma (default `[1, 1, 1, 1]`).
    pub gamma: [f32; 4],
    /// `[r, g, b, master]` highlight gain (default `[1, 1, 1, 1]`).
    pub gain: [f32; 4],
    /// `[r, g, b, master]` linear offset (default `[0, 0, 0, 0]`).
    pub offset: [f32; 4],
    /// Contrast multiplier around `pivot` (default `1.0`).
    pub contrast: f32,
    /// Contrast pivot in linear light (default `0.18` middle gray).
    pub pivot: f32,
    /// Saturation multiplier (default `1.0`).
    pub saturation: f32,
    /// Warm/cool temperature shift (`-1.0..=1.0`, default `0.0`).
    pub temperature: f32,
    /// Green/magenta tint shift (`-1.0..=1.0`, default `0.0`).
    pub tint: f32,
}

impl Default for ColorWheelGradeParams {
    fn default() -> Self {
        Self {
            lift: [0.0, 0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0, 1.0],
            gain: [1.0, 1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0, 0.0],
            contrast: 1.0,
            pivot: 0.18,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
        }
    }
}

#[inline]
fn grade_channel(v: f32, lift: f32, gamma: f32, gain: f32, offset: f32, contrast: f32, pivot: f32) -> f32 {
    // Lift / Gain / Offset (ASC-CDL / Resolve style)
    let lifted = v * (gain - lift) + lift + offset;
    // Safe power for midtone gamma
    let g = gamma.max(0.05);
    let inv_g = 1.0 / g;
    let curved = if lifted >= 0.0 {
        lifted.powf(inv_g)
    } else {
        -(-lifted).powf(inv_g)
    };
    // Pivot contrast
    (curved - pivot) * contrast + pivot
}

/// CPU reference implementation of 3-Way Color Wheel grading on linear `RGBA32F` pixels.
pub fn apply_color_wheel_grade_cpu(rgba: &mut [f32], params: &ColorWheelGradeParams) {
    let lift_r = params.lift[0] + params.lift[3];
    let lift_g = params.lift[1] + params.lift[3];
    let lift_b = params.lift[2] + params.lift[3];

    let gamma_r = (params.gamma[0] * params.gamma[3]).max(0.05);
    let gamma_g = (params.gamma[1] * params.gamma[3]).max(0.05);
    let gamma_b = (params.gamma[2] * params.gamma[3]).max(0.05);

    let gain_r = params.gain[0] * params.gain[3];
    let gain_g = params.gain[1] * params.gain[3];
    let gain_b = params.gain[2] * params.gain[3];

    let off_r = params.offset[0] + params.offset[3];
    let off_g = params.offset[1] + params.offset[3];
    let off_b = params.offset[2] + params.offset[3];

    let wb_r = 1.0 + params.temperature * 0.15;
    let wb_g = 1.0 - params.tint * 0.12;
    let wb_b = 1.0 - params.temperature * 0.15;

    for px in rgba.chunks_exact_mut(4) {
        let mut r = grade_channel(px[0] * wb_r, lift_r, gamma_r, gain_r, off_r, params.contrast, params.pivot);
        let mut g = grade_channel(px[1] * wb_g, lift_g, gamma_g, gain_g, off_g, params.contrast, params.pivot);
        let mut b = grade_channel(px[2] * wb_b, lift_b, gamma_b, gain_b, off_b, params.contrast, params.pivot);

        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r = luma + (r - luma) * params.saturation;
        g = luma + (g - luma) * params.saturation;
        b = luma + (b - luma) * params.saturation;

        px[0] = r;
        px[1] = g;
        px[2] = b;
    }
}

#[cfg(feature = "wgpu")]
pub mod gpu {
    use super::ColorWheelGradeParams;
    use wgpu::util::DeviceExt;

    const COLOR_WHEEL_WGSL: &str = r#"
struct GradeUniform {
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    offset: vec4<f32>,
    contrast: f32,
    pivot: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    width: u32,
    height: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> u: GradeUniform;
@group(0) @binding(1) var<storage, read> src_rgba: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> dst_rgba: array<vec4<f32>>;

fn grade_ch(v: f32, lift: f32, gamma: f32, gain: f32, offset: f32) -> f32 {
    let lifted = v * (gain - lift) + lift + offset;
    let inv_g = 1.0 / max(gamma, 0.05);
    var curved: f32;
    if (lifted >= 0.0) {
        curved = pow(lifted, inv_g);
    } else {
        curved = -pow(-lifted, inv_g);
    }
    return (curved - u.pivot) * u.contrast + u.pivot;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= u.width || gid.y >= u.height) {
        return;
    }
    let idx = gid.y * u.width + gid.x;
    let px = src_rgba[idx];

    let lift_rgb = u.lift.xyz + vec3<f32>(u.lift.w);
    let gamma_rgb = max(u.gamma.xyz * u.gamma.w, vec3<f32>(0.05));
    let gain_rgb = u.gain.xyz * u.gain.w;
    let off_rgb = u.offset.xyz + vec3<f32>(u.offset.w);

    let wb = vec3<f32>(
        1.0 + u.temperature * 0.15,
        1.0 - u.tint * 0.12,
        1.0 - u.temperature * 0.15,
    );

    var r = grade_ch(px.r * wb.r, lift_rgb.r, gamma_rgb.r, gain_rgb.r, off_rgb.r);
    var g = grade_ch(px.g * wb.g, lift_rgb.g, gamma_rgb.g, gain_rgb.g, off_rgb.g);
    var b = grade_ch(px.b * wb.b, lift_rgb.b, gamma_rgb.b, gain_rgb.b, off_rgb.b);

    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    r = luma + (r - luma) * u.saturation;
    g = luma + (g - luma) * u.saturation;
    b = luma + (b - luma) * u.saturation;

    dst_rgba[idx] = vec4<f32>(r, g, b, px.a);
}
"#;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct GradeUniform {
        lift: [f32; 4],
        gamma: [f32; 4],
        gain: [f32; 4],
        offset: [f32; 4],
        contrast: f32,
        pivot: f32,
        saturation: f32,
        temperature: f32,
        tint: f32,
        width: u32,
        height: u32,
        _pad: u32,
    }

    /// Runs 3-Way Color Wheel grading on GPU using a WGSL compute shader.
    pub fn apply_color_wheel_grade_gpu(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[f32],
        width: u32,
        height: u32,
        params: &ColorWheelGradeParams,
    ) -> Option<Vec<f32>> {
        let count = (width as usize).checked_mul(height as usize)?;
        if count == 0 || rgba.len() != count * 4 {
            return None;
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("color_wheel_compute"),
            source: wgpu::ShaderSource::Wgsl(COLOR_WHEEL_WGSL.into()),
        });

        let uni = GradeUniform {
            lift: params.lift,
            gamma: params.gamma,
            gain: params.gain,
            offset: params.offset,
            contrast: params.contrast,
            pivot: params.pivot,
            saturation: params.saturation,
            temperature: params.temperature,
            tint: params.tint,
            width,
            height,
            _pad: 0,
        };
        let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("color_wheel_uniform"),
            contents: bytemuck::bytes_of(&uni),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let src_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("color_wheel_src"),
            contents: bytemuck::cast_slice(rgba),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let byte_len = (count * 4 * std::mem::size_of::<f32>()) as u64;
        let dst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("color_wheel_dst"),
            size: byte_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("color_wheel_staging"),
            size: byte_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("color_wheel_bgl"),
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
            label: Some("color_wheel_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("color_wheel_pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("color_wheel_bg"),
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
            label: Some("color_wheel_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("color_wheel_pass"),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "wgpu")]
    #[test]
    fn gpu_and_cpu_color_wheel_grade_match() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let src = vec![
            0.2, 0.5, 0.8, 1.0,
            0.9, 0.3, 0.1, 0.75,
            0.05, 0.18, 0.45, 1.0,
            0.6, 0.6, 0.6, 1.0,
        ];
        let params = ColorWheelGradeParams {
            lift: [0.02, -0.01, 0.03, 0.0],
            gamma: [1.1, 0.95, 1.05, 1.0],
            gain: [1.2, 1.0, 0.9, 1.05],
            offset: [0.01, 0.0, -0.01, 0.0],
            contrast: 1.15,
            pivot: 0.18,
            saturation: 1.1,
            temperature: 0.2,
            tint: -0.1,
        };

        let mut cpu = src.clone();
        apply_color_wheel_grade_cpu(&mut cpu, &params);
        let gpu = gpu::apply_color_wheel_grade_gpu(&device, &queue, &src, 2, 2, &params)
            .expect("gpu grade");

        for (i, (c, g)) in cpu.iter().zip(gpu.iter()).enumerate() {
            assert!(
                (c - g).abs() < 1e-4,
                "mismatch at {i}: cpu={c} vs gpu={g}"
            );
        }
    }

    #[cfg(feature = "wgpu")]
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
}
