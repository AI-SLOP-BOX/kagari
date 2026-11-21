use super::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PuppetVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

const PUPPET_WGSL: &str = r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var t_next_diffuse: texture_2d<f32>;
@group(0) @binding(3) var s_next_diffuse: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_puppet(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_puppet(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}
"#;

pub(super) fn create_puppet_pipeline(
    device: &wgpu::Device,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Puppet Mesh Pipeline Layout"),
        bind_group_layouts: &[texture_bind_group_layout],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Puppet Mesh Shader"),
        source: wgpu::ShaderSource::Wgsl(PUPPET_WGSL.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Puppet Mesh Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_puppet",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PuppetVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    wgpu::VertexAttribute {
                        offset: 8,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_puppet",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

impl WgpuRenderer {
    pub(super) fn render_puppet_mesh_to_gpu_texture(
        &self,
        comp: &Composition,
        layer: &Layer,
        effective_frame: u32,
        width: u32,
        height: u32,
        src_bg: Option<&wgpu::BindGroup>,
        fallback_color: [f32; 4],
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        if layer.puppet_pins.is_empty() {
            return None;
        }
        let w = width.max(1);
        let h = height.max(1);
        let cw = comp.width.max(1) as f32;
        let ch = comp.height.max(1) as f32;

        let pins: Vec<([f32; 2], [f32; 2])> = layer
            .puppet_pins
            .iter()
            .map(|pin| {
                let d = pin.position.evaluate(effective_frame);
                (
                    [pin.comp_source[0] / cw, pin.comp_source[1] / ch],
                    [d[0] / cw, d[1] / ch],
                )
            })
            .collect();

        const GRID: usize = 24;
        let mut verts = Vec::with_capacity((GRID + 1) * (GRID + 1));
        for gy in 0..=GRID {
            let v = gy as f32 / GRID as f32;
            for gx in 0..=GRID {
                let u = gx as f32 / GRID as f32;
                let mut disp_x = 0.0f32;
                let mut disp_y = 0.0f32;
                let mut wsum = 0.0f32;
                for &(src, dst) in &pins {
                    let dx = u - src[0];
                    let dy = v - src[1];
                    let dist2 = dx * dx + dy * dy + 0.002;
                    let wt = 1.0 / dist2;
                    disp_x += (dst[0] - src[0]) * wt;
                    disp_y += (dst[1] - src[1]) * wt;
                    wsum += wt;
                }
                let wx = if wsum > 0.0 { u + disp_x / wsum } else { u };
                let wy = if wsum > 0.0 { v + disp_y / wsum } else { v };
                verts.push(PuppetVertex {
                    pos: [wx * 2.0 - 1.0, 1.0 - wy * 2.0],
                    uv: [u, v],
                });
            }
        }
        let mut indices = Vec::with_capacity(GRID * GRID * 6);
        let stride = (GRID + 1) as u32;
        for gy in 0..GRID as u32 {
            for gx in 0..GRID as u32 {
                let i0 = gy * stride + gx;
                let i1 = i0 + 1;
                let i2 = i0 + stride;
                let i3 = i2 + 1;
                indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }

        let solid_bg_owned;
        let active_src_bg = if let Some(bg) = src_bg {
            bg
        } else {
            let r = (fallback_color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (fallback_color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (fallback_color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            let a = (fallback_color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Puppet Solid Source"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let bytes = rgba8_color_to_rgba16f_bytes(r, g, b, a);
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            solid_bg_owned = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("puppet_solid_bg"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            &solid_bg_owned
        };

        let vb = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Puppet Mesh VB"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ib = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Puppet Mesh IB"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let dst_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Puppet Mesh Output"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dst_view = dst_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Puppet Mesh GPU Pass"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Puppet Mesh Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.puppet_pipeline);
            pass.set_bind_group(0, active_src_bg, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..(indices.len() as u32), 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        let out_bg = std::sync::Arc::new(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("puppet_out_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dst_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&dst_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        Some((w, h, out_bg))
    }
}
