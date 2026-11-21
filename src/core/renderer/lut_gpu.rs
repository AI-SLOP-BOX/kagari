use super::*;

const LUT_WGSL: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var t_lut: texture_3d<f32>;
@group(0) @binding(3) var s_lut: sampler;
@group(0) @binding(4) var<uniform> u_lut: vec4<f32>; // x: intensity, y: size

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_lut(@builtin(vertex_index) vid: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_lut(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(t_diffuse, s_diffuse, clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let n = max(u_lut.y, 2.0);
    let scale = (n - 1.0) / n;
    let offset = 0.5 / n;
    let coord = clamp(base.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * scale + vec3<f32>(offset);
    let graded = textureSample(t_lut, s_lut, coord).rgb;
    return vec4<f32>(mix(base.rgb, graded, clamp(u_lut.x, 0.0, 1.0)), base.a);
}
"#;

pub(super) fn create_lut_pipeline(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lut3d_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D3,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("LUT 3D Pipeline Layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("LUT 3D Shader"),
        source: wgpu::ShaderSource::Wgsl(LUT_WGSL.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("LUT 3D Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_lut",
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_lut",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    (bgl, pipeline)
}

impl WgpuRenderer {
    pub(super) fn ensure_cube_lut_texture(&self, path: &str) -> Option<u64> {
        if path.is_empty() {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let key = hasher.finish();
        if let Some(entry) = self.lut_cache.borrow().get(&key) {
            return entry.as_ref().map(|_| key);
        }
        let Ok(lut) = crate::core::ocio_color::Lut3D::load_cube_file(path) else {
            self.lut_cache.borrow_mut().insert(key, None);
            return None;
        };
        let n = lut.size as u32;
        if n < 2 || lut.data.len() < (n as usize * n as usize * n as usize * 3) {
            self.lut_cache.borrow_mut().insert(key, None);
            return None;
        }
        let mut rgba16 = Vec::with_capacity((n as usize * n as usize * n as usize) * 4);
        for rgb in lut.data.chunks_exact(3) {
            rgba16.push(f16::from_f32(rgb[0]).to_bits());
            rgba16.push(f16::from_f32(rgb[1]).to_bits());
            rgba16.push(f16::from_f32(rgb[2]).to_bits());
            rgba16.push(f16::from_f32(1.0).to_bits());
        }
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3D LUT Texture"),
            size: wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: n,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&rgba16),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(n * 8),
                rows_per_image: Some(n),
            },
            wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: n,
            },
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.lut_cache.borrow_mut().insert(key, Some((view, n)));
        Some(key)
    }
}
