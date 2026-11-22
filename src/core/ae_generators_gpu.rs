//! After Effects Classic GPU Generators & Utility Assets (`wgpu` / WGSL Compute):
//! - **CC Sphere**: 3D ray-sphere projection with rotation, equirectangular UV lookup, and Blinn-Phong lighting
//! - **Cell Pattern**: Animated Worley/Voronoi cellular generator (Bubbles / Crystals / Plates)
//! - **4-Color Gradient**: Inverse-distance weighted 4-point color gradient
//! - **SMPTE Color Bars & Pro Grid**: Broadcast & motion-design reference assets

use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AeGeneratorKind {
    CcSphere = 0,
    CellPattern = 1,
    FourColorGradient = 2,
    SmpteBars = 3,
    ProGrid = 4,
}

#[derive(Debug, Clone, Copy)]
pub struct AeGeneratorParams {
    pub kind: AeGeneratorKind,
    /// Used by CcSphere (radius in px) / CellPattern (cell size in px) / ProGrid (grid step px).
    pub scale_px: f32,
    /// Animation phase / evolution (radians or cycles).
    pub evolution: f32,
    /// Rotation XYZ in degrees (for CcSphere) or blend sharpness (for FourColorGradient).
    pub rot_deg: [f32; 3],
    pub color0: [f32; 4],
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub color3: [f32; 4],
}

impl Default for AeGeneratorParams {
    fn default() -> Self {
        Self {
            kind: AeGeneratorKind::FourColorGradient,
            scale_px: 48.0,
            evolution: 0.0,
            rot_deg: [0.0, 0.0, 0.0],
            color0: [0.95, 0.25, 0.35, 1.0],
            color1: [0.20, 0.70, 0.98, 1.0],
            color2: [0.98, 0.78, 0.20, 1.0],
            color3: [0.45, 0.22, 0.90, 1.0],
        }
    }
}

const AE_GENERATORS_WGSL: &str = r#"
struct GenUniform {
    width: u32,
    height: u32,
    kind: u32,
    _pad: u32,
    scale_px: f32,
    evolution: f32,
    _pad0: f32,
    _pad1: f32,
    rot_rad: vec4<f32>,
    c0: vec4<f32>,
    c1: vec4<f32>,
    c2: vec4<f32>,
    c3: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: GenUniform;
@group(0) @binding(1) var<storage, read> src_rgba: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> dst_rgba: array<vec4<f32>>;

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.xx + p3.yz) * d);
}

fn sample_src_uv(uv: vec2<f32>) -> vec4<f32> {
    let fx = fract(uv.x) * f32(u.width - 1u);
    let fy = clamp(uv.y, 0.0, 1.0) * f32(u.height - 1u);
    let x0 = u32(floor(fx));
    let y0 = u32(floor(fy));
    let x1 = min(x0 + 1u, u.width - 1u);
    let y1 = min(y0 + 1u, u.height - 1u);
    let tx = fx - f32(x0);
    let ty = fy - f32(y0);
    let c00 = src_rgba[y0 * u.width + x0];
    let c10 = src_rgba[y0 * u.width + x1];
    let c01 = src_rgba[y1 * u.width + x0];
    let c11 = src_rgba[y1 * u.width + x1];
    return mix(mix(c00, c10, tx), mix(c01, c11, tx), ty);
}

fn rot_y(v: vec3<f32>, a: f32) -> vec3<f32> {
    let s = sin(a);
    let c = cos(a);
    return vec3<f32>(v.x * c + v.z * s, v.y, -v.x * s + v.z * c);
}

fn rot_x(v: vec3<f32>, a: f32) -> vec3<f32> {
    let s = sin(a);
    let c = cos(a);
    return vec3<f32>(v.x, v.y * c - v.z * s, v.y * s + v.z * c);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= u.width || gid.y >= u.height) {
        return;
    }
    let idx = gid.y * u.width + gid.x;
    let uv = vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5) / vec2<f32>(f32(u.width), f32(u.height));

    if (u.kind == 0u) {
        // CC Sphere
        let center = vec2<f32>(f32(u.width), f32(u.height)) * 0.5;
        let r = max(u.scale_px, 4.0);
        let p = (vec2<f32>(f32(gid.x), f32(gid.y)) - center) / r;
        let r2 = dot(p, p);
        if (r2 > 1.0) {
            dst_rgba[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            return;
        }
        let z = sqrt(max(1.0 - r2, 0.0));
        let n_view = normalize(vec3<f32>(p.x, -p.y, z));
        let n_obj = normalize(rot_x(rot_y(n_view, u.rot_rad.y), u.rot_rad.x));
        let lon = atan2(n_obj.x, n_obj.z) / 6.2831853 + 0.5;
        let lat = acos(clamp(n_obj.y, -1.0, 1.0)) / 3.14159265;
        let tex = sample_src_uv(vec2<f32>(lon, lat));

        let light_dir = normalize(vec3<f32>(0.5, 0.6, 0.65));
        let view_dir = vec3<f32>(0.0, 0.0, 1.0);
        let diff = max(dot(n_view, light_dir), 0.0);
        let half_v = normalize(light_dir + view_dir);
        let spec = pow(max(dot(n_view, half_v), 0.0), 48.0) * 0.45;
        let edge_aa = 1.0 - smoothstep(0.96, 1.0, sqrt(r2));
        let col = tex.rgb * (0.22 + diff * 0.85) + vec3<f32>(spec);
        dst_rgba[idx] = vec4<f32>(col * edge_aa, tex.a * edge_aa);
    } else if (u.kind == 1u) {
        // Cell Pattern (Worley / Voronoi)
        let cell_sz = max(u.scale_px, 4.0);
        let st = vec2<f32>(f32(gid.x), f32(gid.y)) / cell_sz;
        let i_st = floor(st);
        let f_st = fract(st);
        var min_d1 = 8.0;
        var min_d2 = 8.0;
        for (var j = -1; j <= 1; j++) {
            for (var i = -1; i <= 1; i++) {
                let nb = vec2<f32>(f32(i), f32(j));
                let rnd = hash22(i_st + nb);
                let pt = 0.5 + 0.45 * sin(u.evolution + 6.2831853 * rnd);
                let diff = nb + pt - f_st;
                let d = length(diff);
                if (d < min_d1) {
                    min_d2 = min_d1;
                    min_d1 = d;
                } else if (d < min_d2) {
                    min_d2 = d;
                }
            }
        }
        let val = clamp(1.0 - min_d1 * 1.15, 0.0, 1.0);
        let edge = smoothstep(0.0, 0.15, min_d2 - min_d1);
        let c = mix(u.c0.rgb, u.c1.rgb, val) * (0.35 + 0.65 * edge);
        dst_rgba[idx] = vec4<f32>(c, 1.0);
    } else if (u.kind == 2u) {
        // 4-Color Gradient (IDW)
        let p0 = vec2<f32>(0.15, 0.15);
        let p1 = vec2<f32>(0.85, 0.15);
        let p2 = vec2<f32>(0.15, 0.85);
        let p3 = vec2<f32>(0.85, 0.85);
        let sharp = max(u.rot_rad.x, 1.5);
        let w0 = 1.0 / (pow(distance(uv, p0), sharp) + 0.04);
        let w1 = 1.0 / (pow(distance(uv, p1), sharp) + 0.04);
        let w2 = 1.0 / (pow(distance(uv, p2), sharp) + 0.04);
        let w3 = 1.0 / (pow(distance(uv, p3), sharp) + 0.04);
        let wsum = w0 + w1 + w2 + w3;
        let col = (u.c0 * w0 + u.c1 * w1 + u.c2 * w2 + u.c3 * w3) / wsum;
        dst_rgba[idx] = vec4<f32>(col.rgb, 1.0);
    } else if (u.kind == 3u) {
        // SMPTE Color Bars
        let bar = u32(clamp(floor(uv.x * 7.0), 0.0, 6.0));
        var col = vec3<f32>(0.75);
        if (uv.y < 0.67) {
            if (bar == 0u) { col = vec3<f32>(0.75, 0.75, 0.75); }
            else if (bar == 1u) { col = vec3<f32>(0.75, 0.75, 0.0); }
            else if (bar == 2u) { col = vec3<f32>(0.0, 0.75, 0.75); }
            else if (bar == 3u) { col = vec3<f32>(0.0, 0.75, 0.0); }
            else if (bar == 4u) { col = vec3<f32>(0.75, 0.0, 0.75); }
            else if (bar == 5u) { col = vec3<f32>(0.75, 0.0, 0.0); }
            else { col = vec3<f32>(0.0, 0.0, 0.75); }
        } else {
            col = select(vec3<f32>(0.05), vec3<f32>(1.0), (bar % 2u) == 0u);
        }
        dst_rgba[idx] = vec4<f32>(col, 1.0);
    } else {
        // Pro Grid
        let step_px = max(u.scale_px, 4.0);
        let gx = f32(gid.x) % step_px;
        let gy = f32(gid.y) % step_px;
        let on_line = (gx < 1.5) || (gy < 1.5);
        dst_rgba[idx] = select(u.c1, u.c0, on_line);
    }
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GenUniform {
    width: u32,
    height: u32,
    kind: u32,
    _pad: u32,
    scale_px: f32,
    evolution: f32,
    _pad0: [f32; 2],
    rot_rad: [f32; 4],
    c0: [f32; 4],
    c1: [f32; 4],
    c2: [f32; 4],
    c3: [f32; 4],
}

/// Renders any [`AeGeneratorKind`] on GPU into linear `RGBA32F`.
pub fn render_ae_generator_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    src_rgba: Option<&[f32]>,
    width: u32,
    height: u32,
    params: &AeGeneratorParams,
) -> Option<Vec<f32>> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 {
        return None;
    }
    let fallback_src;
    let src = match src_rgba {
        Some(s) if s.len() == count * 4 => s,
        Some(_) => return None,
        None => {
            fallback_src = vec![0.0_f32; count * 4];
            &fallback_src
        }
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ae_generators_compute"),
        source: wgpu::ShaderSource::Wgsl(AE_GENERATORS_WGSL.into()),
    });

    let uni = GenUniform {
        width,
        height,
        kind: params.kind as u32,
        _pad: 0,
        scale_px: params.scale_px,
        evolution: params.evolution,
        _pad0: [0.0; 2],
        rot_rad: [
            params.rot_deg[0].to_radians(),
            params.rot_deg[1].to_radians(),
            params.rot_deg[2].to_radians(),
            0.0,
        ],
        c0: params.color0,
        c1: params.color1,
        c2: params.color2,
        c3: params.color3,
    };

    let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ae_gen_uni"),
        contents: bytemuck::bytes_of(&uni),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let src_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ae_gen_src"),
        contents: bytemuck::cast_slice(src),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let byte_len = (count * 4 * std::mem::size_of::<f32>()) as u64;
    let dst_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ae_gen_dst"),
        size: byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ae_gen_staging"),
        size: byte_len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ae_gen_bgl"),
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
        label: Some("ae_gen_layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ae_gen_pipeline"),
        layout: Some(&layout),
        module: &shader,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ae_gen_bg"),
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
        label: Some("ae_gen_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ae_gen_pass"),
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
    fn gpu_cc_sphere_and_cell_pattern_render_cleanly() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let w = 32u32;
        let h = 32u32;
        let src = [0.2_f32, 0.6, 0.95, 1.0].repeat((w * h) as usize);

        let sphere = render_ae_generator_gpu(
            &device,
            &queue,
            Some(&src),
            w,
            h,
            &AeGeneratorParams {
                kind: AeGeneratorKind::CcSphere,
                scale_px: 12.0,
                rot_deg: [15.0, 30.0, 0.0],
                ..Default::default()
            },
        )
        .expect("cc sphere");

        // Corner (0,0) outside radius=12 is transparent; center (16,16) is opaque shaded blue
        assert!(sphere[3] < 1e-4);
        let center = (16 * w + 16) as usize * 4;
        assert!(sphere[center + 3] > 0.99 && sphere[center + 2] > sphere[center]);

        let cells = render_ae_generator_gpu(
            &device,
            &queue,
            None,
            w,
            h,
            &AeGeneratorParams {
                kind: AeGeneratorKind::CellPattern,
                scale_px: 8.0,
                evolution: 1.2,
                ..Default::default()
            },
        )
        .expect("cell pattern");
        assert!(cells.iter().any(|&v| v > 0.1));
    }
}
