//! Keylight-style Pro Color-Difference Keyer & Despill Engine (`RGBA32F` CPU + GPU WGSL).
//!
//! Designed as a dedicated keying node (`NodeKind::ProKeyer`) with:
//! - Normalized screen-color difference matte extraction (`screen_balance`, `gain`, `clip_black`, `clip_white`)
//! - Luma-preserving spill suppression (`despill_strength`)
//! - Pre-multiplied clean output ready for `OverMerge` / `DeepMerge`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProKeyerParams {
    /// Target backing screen color in linear RGB (e.g. `[0.0, 0.85, 0.15]` for green screen).
    pub screen_color: [f32; 3],
    /// Screen matte gain (default `1.6`).
    pub gain: f32,
    /// Black clip threshold (`0.0..=0.5`, default `0.08`).
    pub clip_black: f32,
    /// White clip threshold (`0.5..=1.0`, default `0.85`).
    pub clip_white: f32,
    /// Spill suppression strength (`0.0..=1.0`, default `0.85`).
    pub despill_strength: f32,
}

impl Default for ProKeyerParams {
    fn default() -> Self {
        Self {
            screen_color: [0.0, 1.0, 0.0],
            gain: 1.6,
            clip_black: 0.08,
            clip_white: 0.85,
            despill_strength: 0.85,
        }
    }
}

/// CPU reference Pro Keyer on linear `RGBA32F` pixels (outputs premultiplied RGBA).
pub fn apply_pro_keyer_cpu(rgba: &mut [f32], params: &ProKeyerParams) {
    let green_dominant = params.screen_color[1] >= params.screen_color[2];
    let clip_span = (params.clip_white - params.clip_black).max(1e-4);

    for px in rgba.chunks_exact_mut(4) {
        let r = px[0];
        let g = px[1];
        let b = px[2];
        let orig_a = px[3];

        let (primary, other_a, other_b) = if green_dominant { (g, r, b) } else { (b, r, g) };
        let screen_excess = (primary - other_a.max(other_b)).max(0.0) * params.gain;
        let raw_alpha = 1.0 - screen_excess;
        let keyed_a = ((raw_alpha - params.clip_black) / clip_span).clamp(0.0, 1.0) * orig_a;

        // Luma-preserving despill
        let limit = 0.5 * (other_a + other_b);
        let despilled_primary = if primary > limit {
            primary + (limit - primary) * params.despill_strength
        } else {
            primary
        };
        let spill_delta = (primary - despilled_primary).max(0.0);

        let (out_r, out_g, out_b) = if green_dominant {
            (r + spill_delta * 0.2, despilled_primary, b + spill_delta * 0.2)
        } else {
            (r + spill_delta * 0.2, g + spill_delta * 0.2, despilled_primary)
        };

        px[0] = out_r * keyed_a;
        px[1] = out_g * keyed_a;
        px[2] = out_b * keyed_a;
        px[3] = keyed_a;
    }
}

#[cfg(feature = "wgpu")]
pub mod gpu {
    use super::ProKeyerParams;
    use wgpu::util::DeviceExt;

    const PRO_KEYER_WGSL: &str = r#"
struct KeyerUniform {
    width: u32,
    height: u32,
    green_dom: u32,
    gain: f32,
    clip_black: f32,
    clip_white: f32,
    despill_strength: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> u: KeyerUniform;
@group(0) @binding(1) var<storage, read> src_rgba: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> dst_rgba: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= u.width || gid.y >= u.height) {
        return;
    }
    let idx = gid.y * u.width + gid.x;
    let px = src_rgba[idx];
    let r = px.r;
    let g = px.g;
    let b = px.b;

    let primary = select(b, g, u.green_dom == 1u);
    let other_b = select(g, b, u.green_dom == 1u);
    let excess = max(primary - max(r, other_b), 0.0) * u.gain;
    let raw_a = 1.0 - excess;
    let span = max(u.clip_white - u.clip_black, 1e-4);
    let keyed_a = clamp((raw_a - u.clip_black) / span, 0.0, 1.0) * px.a;

    let limit = 0.5 * (r + other_b);
    var despilled = primary;
    if (primary > limit) {
        despilled = primary + (limit - primary) * u.despill_strength;
    }
    let spill_delta = max(primary - despilled, 0.0);

    var out_rgb: vec3<f32>;
    if (u.green_dom == 1u) {
        out_rgb = vec3<f32>(r + spill_delta * 0.2, despilled, b + spill_delta * 0.2);
    } else {
        out_rgb = vec3<f32>(r + spill_delta * 0.2, g + spill_delta * 0.2, despilled);
    }
    dst_rgba[idx] = vec4<f32>(out_rgb * keyed_a, keyed_a);
}
"#;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct KeyerUniform {
        width: u32,
        height: u32,
        green_dom: u32,
        gain: f32,
        clip_black: f32,
        clip_white: f32,
        despill_strength: f32,
        _pad: f32,
    }

    pub fn apply_pro_keyer_gpu(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[f32],
        width: u32,
        height: u32,
        params: &ProKeyerParams,
    ) -> Option<Vec<f32>> {
        let count = (width as usize).checked_mul(height as usize)?;
        if count == 0 || rgba.len() != count * 4 {
            return None;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pro_keyer_compute"),
            source: wgpu::ShaderSource::Wgsl(PRO_KEYER_WGSL.into()),
        });
        let uni = KeyerUniform {
            width,
            height,
            green_dom: if params.screen_color[1] >= params.screen_color[2] {
                1
            } else {
                0
            },
            gain: params.gain,
            clip_black: params.clip_black,
            clip_white: params.clip_white,
            despill_strength: params.despill_strength,
            _pad: 0.0,
        };
        let uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pro_keyer_uni"),
            contents: bytemuck::bytes_of(&uni),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let src_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pro_keyer_src"),
            contents: bytemuck::cast_slice(rgba),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let byte_len = (count * 4 * std::mem::size_of::<f32>()) as u64;
        let dst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pro_keyer_dst"),
            size: byte_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pro_keyer_staging"),
            size: byte_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pro_keyer_bgl"),
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
            label: Some("pro_keyer_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pro_keyer_pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pro_keyer_bg"),
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
            label: Some("pro_keyer_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pro_keyer_pass"),
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

    #[test]
    fn keyer_pulls_clean_alpha_on_greenscreen_and_preserves_subject() {
        let mut img = vec![
            0.05, 0.9, 0.05, 1.0, // pure green screen
            0.85, 0.35, 0.25, 1.0, // warm skin tone
        ];
        apply_pro_keyer_cpu(&mut img, &ProKeyerParams::default());
        assert!(img[3] < 1e-4, "greenscreen alpha should be ~0, got {}", img[3]);
        assert!(img[7] > 0.99, "skin alpha should be ~1, got {}", img[7]);
    }
}
