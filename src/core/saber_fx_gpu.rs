//! Video Copilot "Saber"-class GPU Energy Beam & Neon Path Shader (`wgpu` / WGSL Compute).
//!
//! Renders glowing energy beams, lightsabers, electric arcs, and plasma/fire
//! strokes along arbitrary 2D polylines (Mask / Text / Layer paths) in linear `RGBA32F`:
//! - `start_offset` / `end_offset` trim path animation
//! - FBM noise normal displacement (`distortion_amount`, `distortion_scale`, `wind_speed`, `time_sec`)
//! - Hot white core (`core_size`) + inverse-square exponential HDR glow (`glow_intensity`, `glow_spread`)
//! - Built-in presets (`Saber`, `Electric`, `Fire`, `Neon`, `Portal`, `Energy`)

use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaberPreset {
    Saber,
    Electric,
    Fire,
    Neon,
    Portal,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SaberParams {
    pub glow_color: [f32; 3],
    pub core_color: [f32; 3],
    pub core_size: f32,
    pub glow_intensity: f32,
    pub glow_spread: f32,
    pub glow_bias: f32,
    pub start_offset: f32,
    pub end_offset: f32,
    pub distortion_amount: f32,
    pub distortion_scale: f32,
    pub wind_speed: f32,
    pub time_sec: f32,
}

impl Default for SaberParams {
    fn default() -> Self {
        Self::from_preset(SaberPreset::Saber)
    }
}

impl SaberParams {
    pub fn from_preset(preset: SaberPreset) -> Self {
        match preset {
            SaberPreset::Saber => Self {
                glow_color: [0.15, 0.55, 1.0],
                core_color: [1.0, 1.0, 1.0],
                core_size: 2.5,
                glow_intensity: 1.8,
                glow_spread: 18.0,
                glow_bias: 0.75,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 0.5,
                distortion_scale: 0.04,
                wind_speed: 2.0,
                time_sec: 0.0,
            },
            SaberPreset::Electric => Self {
                glow_color: [0.35, 0.75, 1.0],
                core_color: [0.95, 0.98, 1.0],
                core_size: 1.8,
                glow_intensity: 2.1,
                glow_spread: 14.0,
                glow_bias: 0.7,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 14.0,
                distortion_scale: 0.06,
                wind_speed: 8.0,
                time_sec: 0.0,
            },
            SaberPreset::Fire => Self {
                glow_color: [1.0, 0.32, 0.04],
                core_color: [1.0, 0.92, 0.65],
                core_size: 3.0,
                glow_intensity: 2.2,
                glow_spread: 22.0,
                glow_bias: 0.85,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 10.0,
                distortion_scale: 0.05,
                wind_speed: 5.0,
                time_sec: 0.0,
            },
            SaberPreset::Neon => Self {
                glow_color: [1.0, 0.12, 0.65],
                core_color: [1.0, 0.94, 0.98],
                core_size: 3.2,
                glow_intensity: 1.6,
                glow_spread: 20.0,
                glow_bias: 0.8,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 0.0,
                distortion_scale: 0.03,
                wind_speed: 0.0,
                time_sec: 0.0,
            },
            SaberPreset::Portal => Self {
                glow_color: [1.0, 0.48, 0.08],
                core_color: [1.0, 0.95, 0.85],
                core_size: 2.2,
                glow_intensity: 2.0,
                glow_spread: 16.0,
                glow_bias: 0.78,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 7.5,
                distortion_scale: 0.05,
                wind_speed: 4.5,
                time_sec: 0.0,
            },
            SaberPreset::Energy => Self {
                glow_color: [0.25, 1.0, 0.55],
                core_color: [0.95, 1.0, 0.97],
                core_size: 2.4,
                glow_intensity: 1.9,
                glow_spread: 18.0,
                glow_bias: 0.8,
                start_offset: 0.0,
                end_offset: 1.0,
                distortion_amount: 5.0,
                distortion_scale: 0.045,
                wind_speed: 3.5,
                time_sec: 0.0,
            },
        }
    }
}

const SABER_WGSL: &str = r#"
struct SaberUniform {
    width: u32,
    height: u32,
    seg_count: u32,
    _pad0: u32,
    glow_color: vec4<f32>,
    core_color: vec4<f32>,
    core_size: f32,
    glow_intensity: f32,
    glow_spread: f32,
    glow_bias: f32,
    start_offset: f32,
    end_offset: f32,
    distortion_amount: f32,
    distortion_scale: f32,
    wind_speed: f32,
    time_sec: f32,
    _pad1: vec2<f32>,
};

struct Segment {
    p0: vec2<f32>,
    p1: vec2<f32>,
    t0: f32,
    t1: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: SaberUniform;
@group(0) @binding(1) var<storage, read> segments: array<Segment>;
@group(0) @binding(2) var<storage, read_write> out_rgba: array<vec4<f32>>;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * d);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u2 = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u2.x), mix(c, d, u2.x), u2.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var val = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 4; i++) {
        val += amp * (noise2(p) * 2.0 - 1.0);
        p *= 2.03;
        amp *= 0.5;
    }
    return val;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= u.width || gid.y >= u.height) {
        return;
    }
    let idx = gid.y * u.width + gid.x;
    let px = vec2<f32>(f32(gid.x), f32(gid.y));

    // Apply FBM distortion to sample coordinate for electric/fire turbulence
    let ncoord = px * u.distortion_scale + vec2<f32>(u.time_sec * u.wind_speed, -u.time_sec * u.wind_speed * 0.7);
    let disp = vec2<f32>(fbm(ncoord), fbm(ncoord + vec2<f32>(17.3, 9.1))) * u.distortion_amount;
    let q = px + disp;

    var min_dist = 1e9;
    let s_min = min(u.start_offset, u.end_offset);
    let s_max = max(u.start_offset, u.end_offset);

    for (var i = 0u; i < u.seg_count; i++) {
        let seg = segments[i];
        if (seg.t1 < s_min || seg.t0 > s_max) {
            continue;
        }
        let ab = seg.p1 - seg.p0;
        let len_sq = max(dot(ab, ab), 1e-6);
        var t = clamp(dot(q - seg.p0, ab) / len_sq, 0.0, 1.0);
        let seg_t = mix(seg.t0, seg.t1, t);
        if (seg_t < s_min || seg_t > s_max) {
            let clamped_t = clamp(seg_t, s_min, s_max);
            t = clamp((clamped_t - seg.t0) / max(seg.t1 - seg.t0, 1e-6), 0.0, 1.0);
        }
        let closest = seg.p0 + ab * t;
        min_dist = min(min_dist, length(q - closest));
    }

    let core_r = max(u.core_size, 0.25);
    let core = exp(-(min_dist * min_dist) / (core_r * core_r));
    let spread = max(u.glow_spread, 1.0);
    let glow_near = exp(-min_dist / (spread * 0.35));
    let glow_far = (spread * spread) / (min_dist * min_dist + spread * spread);
    let glow = mix(glow_near, glow_far, clamp(u.glow_bias, 0.0, 1.0)) * u.glow_intensity;

    let col = u.core_color.rgb * core * 1.4 + u.glow_color.rgb * glow;
    let alpha = clamp(core + glow * 0.65, 0.0, 1.0);
    out_rgba[idx] = vec4<f32>(col, alpha);
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SaberUniform {
    width: u32,
    height: u32,
    seg_count: u32,
    _pad0: u32,
    glow_color: [f32; 4],
    core_color: [f32; 4],
    core_size: f32,
    glow_intensity: f32,
    glow_spread: f32,
    glow_bias: f32,
    start_offset: f32,
    end_offset: f32,
    distortion_amount: f32,
    distortion_scale: f32,
    wind_speed: f32,
    time_sec: f32,
    _pad1: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSegment {
    p0: [f32; 2],
    p1: [f32; 2],
    t0: f32,
    t1: f32,
    _pad: [f32; 2],
}

/// Renders a Saber energy beam along `points` into linear `RGBA32F` on GPU.
pub fn render_saber_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    points: &[[f32; 2]],
    width: u32,
    height: u32,
    params: &SaberParams,
) -> Option<Vec<f32>> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || points.len() < 2 {
        return None;
    }

    let mut total_len = 0.0_f32;
    let mut seg_lens = Vec::with_capacity(points.len() - 1);
    for w in points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        let l = (dx * dx + dy * dy).sqrt();
        seg_lens.push(l);
        total_len += l;
    }
    let inv_total = if total_len > 1e-5 { 1.0 / total_len } else { 1.0 };
    let mut acc = 0.0_f32;
    let mut gpu_segs = Vec::with_capacity(seg_lens.len());
    for (w, &l) in points.windows(2).zip(seg_lens.iter()) {
        let t0 = acc * inv_total;
        acc += l;
        let t1 = acc * inv_total;
        gpu_segs.push(GpuSegment {
            p0: w[0],
            p1: w[1],
            t0,
            t1,
            _pad: [0.0; 2],
        });
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("saber_compute"),
        source: wgpu::ShaderSource::Wgsl(SABER_WGSL.into()),
    });

    let uni = SaberUniform {
        width,
        height,
        seg_count: gpu_segs.len() as u32,
        _pad0: 0,
        glow_color: [params.glow_color[0], params.glow_color[1], params.glow_color[2], 1.0],
        core_color: [params.core_color[0], params.core_color[1], params.core_color[2], 1.0],
        core_size: params.core_size,
        glow_intensity: params.glow_intensity,
        glow_spread: params.glow_spread,
        glow_bias: params.glow_bias,
        start_offset: params.start_offset.clamp(0.0, 1.0),
        end_offset: params.end_offset.clamp(0.0, 1.0),
        distortion_amount: params.distortion_amount,
        distortion_scale: params.distortion_scale,
        wind_speed: params.wind_speed,
        time_sec: params.time_sec,
        _pad1: [0.0; 2],
    };

    let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("saber_uni"),
        contents: bytemuck::bytes_of(&uni),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let seg_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("saber_segs"),
        contents: bytemuck::cast_slice(&gpu_segs),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let byte_len = (count * 4 * std::mem::size_of::<f32>()) as u64;
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("saber_out"),
        size: byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("saber_staging"),
        size: byte_len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("saber_bgl"),
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
        label: Some("saber_layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("saber_pipeline"),
        layout: Some(&layout),
        module: &shader,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("saber_bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uni_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: seg_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: out_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("saber_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("saber_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, byte_len);
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
    fn gpu_saber_renders_beam_core_and_trim_offset() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let mut params = SaberParams::from_preset(SaberPreset::Saber);
        params.distortion_amount = 0.0;
        params.end_offset = 0.5; // Only left half of beam should light up!

        let out = render_saber_gpu(&device, &queue, &[[4.0, 16.0], [28.0, 16.0]], 32, 32, &params)
            .expect("saber render");

        // Left core pixel (8, 16) is bright white+blue HDR (> 1.5)
        let left = (16 * 32 + 8) * 4;
        assert!(out[left] > 1.0 && out[left + 2] > 1.5);
        // Far right pixel (27, 16) is trimmed out and much dimmer
        let right = (16 * 32 + 27) * 4;
        assert!(out[right + 2] < out[left + 2] * 0.4);
    }
}
