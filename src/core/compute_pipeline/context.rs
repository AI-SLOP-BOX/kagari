use super::*;

mod directional;
mod fx_dispatch;
mod gaussian;

struct Inner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    /// Cached ping-pong buffers keyed by byte length (recreated on resize).
    bufs: Option<BufSet>,
}

/// Last-run timing stats for the HUD (nanoseconds, atomic for cross-thread read).
pub struct GpuTimings {
    /// Total GPU wall time of the most recent blur call
    pub last_nanos: AtomicU64,
    /// Exponential moving average of blur wall time
    pub avg_nanos: AtomicU64,
    pub calls: AtomicU64,
}

pub static TIMINGS: GpuTimings = GpuTimings {
    last_nanos: AtomicU64::new(0),
    avg_nanos: AtomicU64::new(0),
    calls: AtomicU64::new(0),
};

fn record_timing(start: std::time::Instant) {
    let nanos = start.elapsed().as_nanos() as u64;
    TIMINGS.last_nanos.store(nanos, Ordering::Relaxed);
    let prev_avg = TIMINGS.avg_nanos.load(Ordering::Relaxed);
    let n_calls = TIMINGS.calls.fetch_add(1, Ordering::Relaxed);
    // EMA with alpha ~0.25 (or exact mean for early samples)
    let new_avg = if n_calls < 4 {
        (prev_avg * n_calls + nanos) / (n_calls + 1)
    } else {
        (prev_avg * 3 + nanos) / 4
    };
    TIMINGS.avg_nanos.store(new_avg, Ordering::Relaxed);
}

/// Human-readable HUD line, e.g. "GPU blur 1.2ms (avg 1.4ms, 12 calls)".
pub fn timing_hud_line() -> String {
    let last_us = TIMINGS.last_nanos.load(Ordering::Relaxed) / 1000;
    let avg_us = TIMINGS.avg_nanos.load(Ordering::Relaxed) / 1000;
    let calls = TIMINGS.calls.load(Ordering::Relaxed);
    if calls == 0 {
        "GPU compute: idle".to_string()
    } else {
        format!("GPU compute: {last_us}µs last, {avg_us}µs avg, {calls} calls")
    }
}

struct StagingBuffer {
    buffer: wgpu::Buffer,
    mapped: std::cell::Cell<bool>,
}

impl std::ops::Deref for StagingBuffer {
    type Target = wgpu::Buffer;
    fn deref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

impl StagingBuffer {
    fn new(buffer: wgpu::Buffer) -> Self {
        Self {
            buffer,
            mapped: std::cell::Cell::new(false),
        }
    }

    fn mark_mapped(&self) {
        self.mapped.set(true);
    }

    fn unmap_tracking(&self) {
        if self.mapped.get() {
            self.buffer.unmap();
            self.mapped.set(false);
        }
    }
}

impl Drop for StagingBuffer {
    fn drop(&mut self) {
        self.unmap_tracking();
    }
}

struct BufSet {
    len: u64,
    src: wgpu::Buffer,
    mid: wgpu::Buffer,
    dst: wgpu::Buffer,
    kernel: wgpu::Buffer,
    staging_out: StagingBuffer,
    params: wgpu::Buffer,
}

/// A live wgpu compute context. Obtain via [`global()`].
pub struct GpuComputeContext {
    inner: Mutex<Inner>,
    label: String,
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub(crate) struct ParamsUniform {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) radius: u32,
    pub(crate) mode: u32,
    pub(crate) angle: f32,
    pub(crate) brightness: f32,
    pub(crate) contrast: f32,
    pub(crate) saturation: f32,
    pub(crate) hue_shift: f32,
    pub(crate) param_f3: f32,
    pub(crate) param_f4: f32,
    pub(crate) param_f5: f32,
    pub(crate) param_f6: f32,
    pub(crate) param_f7: f32,
    pub(crate) param_f8: f32,
    pub(crate) _pad: f32,
}

impl GpuComputeContext {
    /// Request an adapter + device. Returns None when no suitable adapter
    /// exists (headless CI, missing drivers) — never panics.
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kagari-compute"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .ok()?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_blur.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gaussian_blur"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });
        let info = adapter.get_info();
        let label = format!("{} ({})", info.name.trim(), info.backend.to_str());
        // Leak the instance so its Metal backend destructor does not run at
        // process exit.  The Device + Queue hold their own Arc references to
        // the wgpu-core context, so the Instance is no longer needed.
        std::mem::forget(instance);
        Some(Self {
            inner: Mutex::new(Inner {
                device,
                queue,
                pipeline,
                bufs: None,
            }),
            label,
        })
    }

    pub fn backend_label(&self) -> &str {
        &self.label
    }

    fn alloc_buf_set(&self, dev: &wgpu::Device, buf_len: u64) -> BufSet {
        let any = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        BufSet {
            len: buf_len,
            src: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_src"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            mid: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_mid"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            dst: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_dst"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            kernel: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_kernel"),
                size: ((2 * MAX_BLUR_RADIUS + 1) * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            staging_out: StagingBuffer::new(dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_staging"),
                size: buf_len,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })),
            params: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_params"),
                size: std::mem::size_of::<ParamsUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }
}
