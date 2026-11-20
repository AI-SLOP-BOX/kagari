//! GPU Compute Dense Optical Flow & Sub-Pixel Block Tracker (wgpu / WGSL).
//!
//! Computes per-pixel motion vectors `[dx, dy]` and confidence scores on GPU
//! using luminance-weighted RGB SAD block matching followed by 1D parabolic
//! sub-pixel refinement in X and Y.

use std::sync::OnceLock;
use wgpu::util::DeviceExt;

use crate::core::optical_flow_timewarp::{DenseFlowField, MAX_DENSE_FLOW_PIXELS};

struct CachedGpuFlow {
    device: wgpu::Device,
    queue: wgpu::Queue,
    bgl: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

static GLOBAL_GPU_FLOW: OnceLock<Option<CachedGpuFlow>> = OnceLock::new();

fn global_gpu_flow() -> Option<&'static CachedGpuFlow> {
    GLOBAL_GPU_FLOW
        .get_or_init(|| {
            let instance = wgpu::Instance::default();
            let adapter =
                block_on_sync(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
            let (device, queue) =
                block_on_sync(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                    .ok()?;
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("optical_flow_compute"),
                source: wgpu::ShaderSource::Wgsl(OPTICAL_FLOW_WGSL.into()),
            });
            let bgl = create_bgl(&device);
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("optical_flow_layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("optical_flow_pipeline"),
                layout: Some(&layout),
                module: &shader,
                entry_point: "main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            Some(CachedGpuFlow {
                device,
                queue,
                bgl,
                pipeline,
            })
        })
        .as_ref()
}

/// Runs dense optical flow using a lazily-cached global GPU device and compiled pipeline.
pub fn compute_dense_optical_flow_gpu(
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> Option<(DenseFlowField, Vec<f32>)> {
    let ctx = global_gpu_flow()?;
    run_with_pipeline(
        &ctx.device,
        &ctx.queue,
        &ctx.bgl,
        &ctx.pipeline,
        src_rgba,
        tgt_rgba,
        width,
        height,
        block_radius,
        search_radius,
    )
}

fn block_on_sync<F: std::future::Future>(fut: F) -> F::Output {
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

fn create_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("optical_flow_bgl"),
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
            storage_entry(1, true),
            storage_entry(2, true),
            storage_entry(3, false),
            storage_entry(4, false),
        ],
    })
}

const OPTICAL_FLOW_WGSL: &str = r#"
struct FlowParams {
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
};

@group(0) @binding(0) var<uniform> params: FlowParams;
@group(0) @binding(1) var<storage, read> src_pixels: array<u32>;
@group(0) @binding(2) var<storage, read> tgt_pixels: array<u32>;
@group(0) @binding(3) var<storage, read_write> out_vectors: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> out_confidence: array<f32>;

fn unpack_luma_rgb(packed: u32) -> vec3<f32> {
    let r = f32(packed & 0xFFu);
    let g = f32((packed >> 8u) & 0xFFu);
    let b = f32((packed >> 16u) & 0xFFu);
    return vec3<f32>(r, g, b);
}

fn load_src(x: i32, y: i32) -> vec3<f32> {
    let cx = clamp(x, 0, i32(params.width) - 1);
    let cy = clamp(y, 0, i32(params.height) - 1);
    return unpack_luma_rgb(src_pixels[u32(cy) * params.width + u32(cx)]);
}

fn load_tgt(x: i32, y: i32) -> vec3<f32> {
    let cx = clamp(x, 0, i32(params.width) - 1);
    let cy = clamp(y, 0, i32(params.height) - 1);
    return unpack_luma_rgb(tgt_pixels[u32(cy) * params.width + u32(cx)]);
}

fn block_sad(px: i32, py: i32, dx: i32, dy: i32, br: i32) -> f32 {
    var sad = 0.0;
    for (var oy = -br; oy <= br; oy++) {
        for (var ox = -br; ox <= br; ox++) {
            let s = load_src(px + ox, py + oy);
            let t = load_tgt(px + ox + dx, py + oy + dy);
            let d = abs(s - t);
            sad += d.r * 0.299 + d.g * 0.587 + d.b * 0.114;
        }
    }
    return sad;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }
    let idx = gid.y * params.width + gid.x;
    let px = i32(gid.x);
    let py = i32(gid.y);
    let br = params.block_radius;
    let sr = params.search_radius;

    var best_dx = 0;
    var best_dy = 0;
    var best_sad = 1e30;
    var sum_sad = 0.0;
    var cand_count = 0.0;

    for (var dy = -sr; dy <= sr; dy++) {
        for (var dx = -sr; dx <= sr; dx++) {
            let sad = block_sad(px, py, dx, dy, br);
            sum_sad += sad;
            cand_count += 1.0;
            if (sad < best_sad) {
                best_sad = sad;
                best_dx = dx;
                best_dy = dy;
            }
        }
    }

    // Parabolic sub-pixel refinement in X and Y
    var fx = f32(best_dx);
    var fy = f32(best_dy);
    if (best_dx > -sr && best_dx < sr) {
        let sad_l = block_sad(px, py, best_dx - 1, best_dy, br);
        let sad_r = block_sad(px, py, best_dx + 1, best_dy, br);
        let denom = sad_l - 2.0 * best_sad + sad_r;
        if (denom > 1e-3) {
            fx += clamp(0.5 * (sad_l - sad_r) / denom, -0.5, 0.5);
        }
    }
    if (best_dy > -sr && best_dy < sr) {
        let sad_u = block_sad(px, py, best_dx, best_dy - 1, br);
        let sad_d = block_sad(px, py, best_dx, best_dy + 1, br);
        let denom = sad_u - 2.0 * best_sad + sad_d;
        if (denom > 1e-3) {
            fy += clamp(0.5 * (sad_u - sad_d) / denom, -0.5, 0.5);
        }
    }

    let mean_sad = sum_sad / max(cand_count, 1.0);
    let conf = clamp(1.0 - best_sad / max(mean_sad, 1e-3), 0.0, 1.0);
    out_vectors[idx] = vec2<f32>(fx, fy);
    out_confidence[idx] = conf;
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FlowParams {
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
}

/// Runs dense block-matching optical flow with sub-pixel refinement on the provided GPU device.
///
/// Returns `(DenseFlowField, per_pixel_confidence)` on success, or `None` if dimensions
/// or buffer lengths are invalid.
pub fn compute_dense_optical_flow_gpu_on_device(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> Option<(DenseFlowField, Vec<f32>)> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("optical_flow_compute"),
        source: wgpu::ShaderSource::Wgsl(OPTICAL_FLOW_WGSL.into()),
    });
    let bgl = create_bgl(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("optical_flow_layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("optical_flow_pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    run_with_pipeline(
        device,
        queue,
        &bgl,
        &pipeline,
        src_rgba,
        tgt_rgba,
        width,
        height,
        block_radius,
        search_radius,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_with_pipeline(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bgl: &wgpu::BindGroupLayout,
    pipeline: &wgpu::ComputePipeline,
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> Option<(DenseFlowField, Vec<f32>)> {
    let pixel_count = (width as usize).checked_mul(height as usize)?;
    if pixel_count == 0 || pixel_count > MAX_DENSE_FLOW_PIXELS {
        return None;
    }
    let expected_bytes = pixel_count.checked_mul(4)?;
    if src_rgba.len() != expected_bytes || tgt_rgba.len() != expected_bytes {
        return None;
    }

    let br = block_radius.clamp(1, 4);
    let sr = search_radius.clamp(1, 16);

    let params = FlowParams {
        width,
        height,
        block_radius: br,
        search_radius: sr,
    };
    let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("optical_flow_params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let src_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("optical_flow_src"),
        contents: src_rgba,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let tgt_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("optical_flow_tgt"),
        contents: tgt_rgba,
        usage: wgpu::BufferUsages::STORAGE,
    });

    let vec_bytes = (pixel_count * 2 * std::mem::size_of::<f32>()) as u64;
    let conf_bytes = (pixel_count * std::mem::size_of::<f32>()) as u64;

    let out_vec_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("optical_flow_vectors"),
        size: vec_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let out_conf_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("optical_flow_conf"),
        size: conf_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_vec = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("optical_flow_staging_vec"),
        size: vec_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let staging_conf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("optical_flow_staging_conf"),
        size: conf_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("optical_flow_bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: src_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: tgt_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: out_vec_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: out_conf_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("optical_flow_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("optical_flow_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&out_vec_buf, 0, &staging_vec, 0, vec_bytes);
    encoder.copy_buffer_to_buffer(&out_conf_buf, 0, &staging_conf, 0, conf_bytes);
    queue.submit(Some(encoder.finish()));

    let raw_vecs = read_mapped_f32(device, &staging_vec, pixel_count * 2)?;
    let conf = read_mapped_f32(device, &staging_conf, pixel_count)?;

    let mut field = DenseFlowField::new(width, height);
    if !field.is_complete() {
        return None;
    }
    for (dst, chunk) in field.vectors.iter_mut().zip(raw_vecs.chunks_exact(2)) {
        *dst = [chunk[0], chunk[1]];
    }
    Some((field, conf))
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn read_mapped_f32(device: &wgpu::Device, buf: &wgpu::Buffer, len: usize) -> Option<Vec<f32>> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    let data = slice.get_mapped_range();
    let floats: &[f32] = bytemuck::cast_slice(&data);
    let out = floats[..len].to_vec();
    drop(data);
    buf.unmap();
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
    fn gpu_optical_flow_recovers_translation() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let w = 32u32;
        let h = 32u32;
        let mut src = vec![0u8; (w * h * 4) as usize];
        let mut tgt = vec![0u8; (w * h * 4) as usize];

        let put_blob = |buf: &mut [u8], cx: f32, cy: f32| {
            for y in 0..h {
                for x in 0..w {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let val = (255.0 * (-0.08 * (dx * dx + dy * dy)).exp()) as u8;
                    let idx = ((y * w + x) * 4) as usize;
                    buf[idx] = val;
                    buf[idx + 1] = val;
                    buf[idx + 2] = val;
                    buf[idx + 3] = 255;
                }
            }
        };

        put_blob(&mut src, 12.0, 14.0);
        put_blob(&mut tgt, 15.0, 12.0); // shifted (+3, -2)

        let (flow, conf) =
            compute_dense_optical_flow_gpu_on_device(&device, &queue, &src, &tgt, w, h, 2, 5)
                .expect("gpu flow");

        let v = flow.get(12, 14);
        assert!((v[0] - 3.0).abs() < 0.35, "expected dx ~3.0, got {}", v[0]);
        assert!((v[1] - (-2.0)).abs() < 0.35, "expected dy ~-2.0, got {}", v[1]);
        assert!(conf[(14 * w + 12) as usize] > 0.25);
    }
}
