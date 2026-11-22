//! Deep Compositing & Z-Depth Merge Engine (Nuke-style multi-sample & GPU Z-Merge).
//!
//! Supports:
//! - Multi-sample per-pixel [`DeepImage`] with volumetric `[z_front, z_back]` interval
//!   splitting and exponential optical absorption.
//! - [`DeepImage::deep_merge`] for depth-sorted front-to-back compositing across
//!   overlapping translucent / volumetric layers.
//! - [`DeepImage::deep_holdout`] for cutting out occluded regions behind holdout mattes.
//! - [`gpu::deep_merge_gpu`] (when `wgpu` is enabled) for real-time WGSL compute
//!   Z-depth compositing of linear HDR `RGBA32F` + `Z` buffers on GPU.

/// A single premultiplied linear HDR deep sample covering `[z_front, z_back]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeepSample {
    pub z_front: f32,
    pub z_back: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl DeepSample {
    /// Creates a point (hard-surface) deep sample at `z`.
    #[inline]
    pub fn point(z: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            z_front: z,
            z_back: z,
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Creates a volumetric deep sample spanning `[z_front, z_back]`.
    #[inline]
    pub fn volume(z_front: f32, z_back: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        let (zf, zb) = if z_front <= z_back {
            (z_front, z_back)
        } else {
            (z_back, z_front)
        };
        Self {
            z_front: zf,
            z_back: zb,
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    #[inline]
    fn is_point(&self) -> bool {
        (self.z_back - self.z_front).abs() <= 1e-6
    }

    /// Splits a volumetric sample to a sub-interval `[sub_front, sub_back]` using
    /// Beer-Lambert optical absorption (`alpha_sub = 1 - (1 - a)^fraction`).
    fn sub_interval(&self, sub_front: f32, sub_back: f32) -> Self {
        let total = self.z_back - self.z_front;
        if total <= 1e-6 {
            return *self;
        }
        let frac = ((sub_back - sub_front) / total).clamp(0.0, 1.0);
        if frac >= 1.0 - 1e-6 {
            return Self {
                z_front: sub_front,
                z_back: sub_back,
                ..*self
            };
        }
        let a = self.a.clamp(0.0, 1.0);
        let sub_a = if a >= 1.0 - 1e-6 {
            1.0
        } else if a <= 0.0 {
            0.0
        } else {
            1.0 - (1.0 - a).powf(frac)
        };
        let scale = if a > 1e-6 { sub_a / a } else { frac };
        Self {
            z_front: sub_front,
            z_back: sub_back,
            r: self.r * scale,
            g: self.g * scale,
            b: self.b * scale,
            a: sub_a,
        }
    }
}

/// Multi-sample deep image storing sorted [`DeepSample`] lists per pixel.
#[derive(Debug, Clone)]
pub struct DeepImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Vec<DeepSample>>,
}

impl DeepImage {
    /// Creates an empty deep image with `width * height` pixels.
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            pixels: vec![Vec::new(); len],
        }
    }

    /// Builds a [`DeepImage`] from premultiplied linear `rgba` (`len == w*h*4`)
    /// and scalar `depth` (`len == w*h`). Pixels with `alpha <= 0` or non-finite
    /// depth are omitted.
    pub fn from_rgba_depth(rgba: &[f32], depth: &[f32], width: u32, height: u32) -> Result<Self, String> {
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "DeepImage dimensions overflow".to_string())?;
        if rgba.len() != count * 4 {
            return Err(format!(
                "rgba buffer length {} does not match {}x{}x4",
                rgba.len(),
                width,
                height
            ));
        }
        if depth.len() != count {
            return Err(format!(
                "depth buffer length {} does not match {}x{}",
                depth.len(),
                width,
                height
            ));
        }
        let mut img = Self::new(width, height);
        for i in 0..count {
            let a = rgba[i * 4 + 3];
            let z = depth[i];
            if a > 1e-6 && z.is_finite() {
                img.pixels[i].push(DeepSample::point(
                    z,
                    rgba[i * 4],
                    rgba[i * 4 + 1],
                    rgba[i * 4 + 2],
                    a,
                ));
            }
        }
        Ok(img)
    }

    /// Deep-merges `self` and `other` front-to-back, splitting overlapping volumetric
    /// intervals `[z_front, z_back]` so intersecting semi-transparent volumes blend accurately.
    pub fn deep_merge(&self, other: &DeepImage) -> Result<DeepImage, String> {
        if self.width != other.width || self.height != other.height {
            return Err(format!(
                "DeepImage size mismatch: {}x{} vs {}x{}",
                self.width, self.height, other.width, other.height
            ));
        }
        let mut out = DeepImage::new(self.width, self.height);
        for (idx, dst) in out.pixels.iter_mut().enumerate() {
            *dst = merge_pixel_samples(&self.pixels[idx], &other.pixels[idx]);
        }
        Ok(out)
    }

    /// Applies `holdout` as a deep holdout matte: samples in `self` behind
    /// holdout samples are attenuated by `(1 - holdout_alpha_before_z)`.
    pub fn deep_holdout(&self, holdout: &DeepImage) -> Result<DeepImage, String> {
        if self.width != holdout.width || self.height != holdout.height {
            return Err(format!(
                "DeepImage size mismatch: {}x{} vs {}x{}",
                self.width, self.height, holdout.width, holdout.height
            ));
        }
        let mut out = DeepImage::new(self.width, self.height);
        for (idx, dst) in out.pixels.iter_mut().enumerate() {
            let mut hold = holdout.pixels[idx].clone();
            hold.sort_by(|a, b| a.z_front.total_cmp(&b.z_front));

            let mut fg = self.pixels[idx].clone();
            fg.sort_by(|a, b| a.z_front.total_cmp(&b.z_front));

            let mut result = Vec::with_capacity(fg.len());
            for s in fg {
                let mut transmittance = 1.0_f32;
                for h in &hold {
                    if h.z_back <= s.z_front {
                        transmittance *= (1.0 - h.a).max(0.0);
                    } else if h.z_front < s.z_back && !h.is_point() {
                        // Partial overlap: integrate holdout up to s midpoint
                        let mid = 0.5 * (s.z_front + s.z_back);
                        if h.z_front < mid {
                            let end = mid.min(h.z_back);
                            let sub = h.sub_interval(h.z_front, end);
                            transmittance *= (1.0 - sub.a).max(0.0);
                        }
                    }
                    if transmittance <= 1e-6 {
                        break;
                    }
                }
                if transmittance > 1e-6 {
                    result.push(DeepSample {
                        z_front: s.z_front,
                        z_back: s.z_back,
                        r: s.r * transmittance,
                        g: s.g * transmittance,
                        b: s.b * transmittance,
                        a: s.a * transmittance,
                    });
                }
            }
            *dst = result;
        }
        Ok(out)
    }

    /// Flattens every pixel front-to-back into premultiplied linear `RGBA32F`.
    pub fn flatten_rgba(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            let mut sorted = pixel.clone();
            sorted.sort_by(|a, b| {
                a.z_front
                    .total_cmp(&b.z_front)
                    .then_with(|| a.z_back.total_cmp(&b.z_back))
            });
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut a = 0.0_f32;
            for s in sorted {
                let trans = (1.0 - a).max(0.0);
                if trans <= 1e-6 {
                    break;
                }
                r += trans * s.r;
                g += trans * s.g;
                b += trans * s.b;
                a += trans * s.a;
            }
            out.extend_from_slice(&[r, g, b, a.clamp(0.0, 1.0)]);
        }
        out
    }

    /// Returns the frontmost finite `z_front` per pixel (or `f32::INFINITY` if empty).
    pub fn flatten_depth(&self) -> Vec<f32> {
        self.pixels
            .iter()
            .map(|px| {
                px.iter()
                    .filter(|s| s.a > 1e-6 && s.z_front.is_finite())
                    .map(|s| s.z_front)
                    .fold(f32::INFINITY, f32::min)
            })
            .collect()
    }
}

fn merge_pixel_samples(a: &[DeepSample], b: &[DeepSample]) -> Vec<DeepSample> {
    if a.is_empty() {
        let mut res = b.to_vec();
        res.sort_by(|x, y| x.z_front.total_cmp(&y.z_front));
        return res;
    }
    if b.is_empty() {
        let mut res = a.to_vec();
        res.sort_by(|x, y| x.z_front.total_cmp(&y.z_front));
        return res;
    }

    let mut all = Vec::with_capacity(a.len() + b.len());
    all.extend_from_slice(a);
    all.extend_from_slice(b);

    // Check if there are any volumetric samples; if all are point samples, sort directly.
    if all.iter().all(|s| s.is_point()) {
        all.sort_by(|x, y| x.z_front.total_cmp(&y.z_front));
        return all;
    }

    // Collect sorted unique depth breakpoints for volumetric interval splitting.
    let mut cuts: Vec<f32> = Vec::with_capacity(all.len() * 2);
    for s in &all {
        if s.z_front.is_finite() {
            cuts.push(s.z_front);
        }
        if s.z_back.is_finite() {
            cuts.push(s.z_back);
        }
    }
    cuts.sort_by(|x, y| x.total_cmp(y));
    cuts.dedup_by(|x, y| (*x - *y).abs() <= 1e-6);

    let mut merged = Vec::with_capacity(all.len());
    for (ci, &z) in cuts.iter().enumerate() {
        // Point samples exactly at `z`
        for s in &all {
            if s.is_point() && (s.z_front - z).abs() <= 1e-6 {
                merged.push(*s);
            }
        }
        // Volumetric interval [z, next_z]
        if ci + 1 < cuts.len() {
            let next_z = cuts[ci + 1];
            if next_z - z > 1e-6 {
                let mid = 0.5 * (z + next_z);
                let mut segs: Vec<DeepSample> = all
                    .iter()
                    .filter(|s| !s.is_point() && s.z_front <= mid && s.z_back >= mid)
                    .map(|s| s.sub_interval(z, next_z))
                    .collect();
                if segs.len() == 1 {
                    merged.push(segs[0]);
                } else if segs.len() > 1 {
                    // Combine simultaneous co-located volumetric segments cleanly.
                    let mut comb_a = 0.0_f32;
                    let mut sum_r = 0.0_f32;
                    let mut sum_g = 0.0_f32;
                    let mut sum_b = 0.0_f32;
                    let mut sum_raw_a = 0.0_f32;
                    for seg in segs.drain(..) {
                        comb_a = 1.0 - (1.0 - comb_a) * (1.0 - seg.a);
                        sum_r += seg.r;
                        sum_g += seg.g;
                        sum_b += seg.b;
                        sum_raw_a += seg.a;
                    }
                    let scale = if sum_raw_a > 1e-6 {
                        comb_a / sum_raw_a
                    } else {
                        1.0
                    };
                    merged.push(DeepSample::volume(
                        z,
                        next_z,
                        sum_r * scale,
                        sum_g * scale,
                        sum_b * scale,
                        comb_a,
                    ));
                }
            }
        }
    }
    merged
}

#[cfg(feature = "wgpu")]
pub mod gpu {
    use wgpu::util::DeviceExt;

    const DEEP_MERGE_WGSL: &str = r#"
struct Params {
    width: u32,
    height: u32,
    z_epsilon: f32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> rgba_a: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> depth_a: array<f32>;
@group(0) @binding(3) var<storage, read> rgba_b: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> depth_b: array<f32>;
@group(0) @binding(5) var<storage, read_write> rgba_out: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> depth_out: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }
    let idx = gid.y * params.width + gid.x;
    let ca = rgba_a[idx];
    let cb = rgba_b[idx];
    let za = depth_a[idx];
    let zb = depth_b[idx];

    // Order front-to-back with smooth anti-aliased transition within z_epsilon.
    let a_over_b = ca + (1.0 - clamp(ca.a, 0.0, 1.0)) * cb;
    let b_over_a = cb + (1.0 - clamp(cb.a, 0.0, 1.0)) * ca;

    let eps = max(params.z_epsilon, 1e-6);
    let diff = za - zb;
    let t = smoothstep(-eps, eps, diff);
    rgba_out[idx] = mix(a_over_b, b_over_a, t);
    depth_out[idx] = min(za, zb);
}
"#;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct DeepParams {
        width: u32,
        height: u32,
        z_epsilon: f32,
        _pad: u32,
    }

    /// Executes a GPU WGSL compute `DeepMerge` pass over two linear `RGBA32F` + `Z` layers.
    ///
    /// Returns `(merged_rgba, merged_depth)` on success, or `None` if buffer sizes mismatch.
    pub fn deep_merge_gpu(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba_a: &[f32],
        depth_a: &[f32],
        rgba_b: &[f32],
        depth_b: &[f32],
        width: u32,
        height: u32,
        z_epsilon: f32,
    ) -> Option<(Vec<f32>, Vec<f32>)> {
        let count = (width as usize).checked_mul(height as usize)?;
        if count == 0
            || rgba_a.len() != count * 4
            || rgba_b.len() != count * 4
            || depth_a.len() != count
            || depth_b.len() != count
        {
            return None;
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("deep_merge_compute"),
            source: wgpu::ShaderSource::Wgsl(DEEP_MERGE_WGSL.into()),
        });

        let params = DeepParams {
            width,
            height,
            z_epsilon,
            _pad: 0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("deep_merge_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let make_ro = |label: &str, data: &[f32]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        let buf_rgba_a = make_ro("deep_rgba_a", rgba_a);
        let buf_depth_a = make_ro("deep_depth_a", depth_a);
        let buf_rgba_b = make_ro("deep_rgba_b", rgba_b);
        let buf_depth_b = make_ro("deep_depth_b", depth_b);

        let rgba_bytes = (count * 4 * std::mem::size_of::<f32>()) as u64;
        let depth_bytes = (count * std::mem::size_of::<f32>()) as u64;

        let buf_rgba_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deep_rgba_out"),
            size: rgba_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let buf_depth_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deep_depth_out"),
            size: depth_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_rgba = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deep_staging_rgba"),
            size: rgba_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_depth = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deep_staging_depth"),
            size: depth_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("deep_merge_bgl"),
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
                storage_entry(3, true),
                storage_entry(4, true),
                storage_entry(5, false),
                storage_entry(6, false),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deep_merge_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("deep_merge_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("deep_merge_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buf_rgba_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buf_depth_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buf_rgba_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buf_depth_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buf_rgba_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: buf_depth_out.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("deep_merge_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("deep_merge_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(&buf_rgba_out, 0, &staging_rgba, 0, rgba_bytes);
        encoder.copy_buffer_to_buffer(&buf_depth_out, 0, &staging_depth, 0, depth_bytes);
        queue.submit(Some(encoder.finish()));

        let out_rgba = read_mapped_f32(device, &staging_rgba, count * 4)?;
        let out_depth = read_mapped_f32(device, &staging_depth, count)?;
        Some((out_rgba, out_depth))
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

    fn read_mapped_f32(
        device: &wgpu::Device,
        buf: &wgpu::Buffer,
        len: usize,
    ) -> Option<Vec<f32>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_merge_overlapping_volumes_splits_and_flattens() {
        let mut a = DeepImage::new(1, 1);
        a.pixels[0].push(DeepSample::volume(1.0, 3.0, 0.6, 0.0, 0.0, 0.6));

        let mut b = DeepImage::new(1, 1);
        b.pixels[0].push(DeepSample::volume(2.0, 4.0, 0.0, 0.5, 0.0, 0.5));

        let merged = a.deep_merge(&b).expect("merge");
        // Expect 3 intervals: [1,2], [2,3], [3,4]
        assert_eq!(merged.pixels[0].len(), 3);
        let flat = merged.flatten_rgba();
        assert!(flat[0] > 0.3 && flat[1] > 0.15);
        assert!(flat[3] > 0.7 && flat[3] <= 1.0);
    }

    #[test]
    fn deep_holdout_attenuates_background_behind_matte() {
        let mut bg = DeepImage::new(1, 1);
        bg.pixels[0].push(DeepSample::point(10.0, 1.0, 0.5, 0.25, 1.0));

        let mut matte = DeepImage::new(1, 1);
        matte.pixels[0].push(DeepSample::point(5.0, 0.0, 0.0, 0.0, 0.75));

        let cut = bg.deep_holdout(&matte).expect("holdout");
        let flat = cut.flatten_rgba();
        assert!((flat[0] - 0.25).abs() < 1e-5);
        assert!((flat[3] - 0.25).abs() < 1e-5);
    }

    #[cfg(feature = "wgpu")]
    #[test]
    fn gpu_deep_merge_crosses_depth_ramps() {
        let instance = wgpu::Instance::default();
        let adapter = pollster_block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            pollster_block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        // 4x1 pixels: left half A is closer (z=1 < z=5), right half B is closer (z=9 > z=2)
        let rgba_a = [
            1.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0,
        ];
        let depth_a = [1.0, 1.0, 9.0, 9.0];
        let rgba_b = [
            0.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0,
        ];
        let depth_b = [5.0, 5.0, 2.0, 2.0];

        let (out_rgba, out_depth) =
            gpu::deep_merge_gpu(&device, &queue, &rgba_a, &depth_a, &rgba_b, &depth_b, 4, 1, 0.01)
                .expect("gpu deep merge");

        // Pixel 0: Red in front (depth 1.0)
        assert!(out_rgba[0] > 0.99 && out_rgba[2] < 0.01);
        assert!((out_depth[0] - 1.0).abs() < 1e-5);
        // Pixel 3: Blue in front (depth 2.0)
        assert!(out_rgba[12] < 0.01 && out_rgba[14] > 0.99);
        assert!((out_depth[3] - 2.0).abs() < 1e-5);
    }

    #[cfg(feature = "wgpu")]
    fn pollster_block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};
        struct W(std::thread::Thread);
        impl Wake for W {
            fn wake(self: Arc<Self>) { self.0.unpark(); }
            fn wake_by_ref(self: &Arc<Self>) { self.0.unpark(); }
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
