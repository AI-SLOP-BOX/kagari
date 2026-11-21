//! 3D Directional/Spot Shadow Map Pass (`Depth32Float`) + 3x3 PCF Soft Shadow Sampling (`wgpu` / WGSL).
//!
//! Provides real hardware depth-only light-space shadow rendering and 3x3 Percentage-Closer
//! Filtering (PCF) for `Model3D` / Element 3D V2 / C4D-style 3D scene lighting.

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowPassUniform {
    light_vp: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LitPassUniform {
    camera_vp: [[f32; 4]; 4],
    light_vp: [[f32; 4]; 4],
    light_dir: [f32; 4],
}

const SHADOW_DEPTH_WGSL: &str = r#"
struct ShadowUni {
    light_vp: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> u_sh: ShadowUni;

@vertex
fn vs_shadow(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u_sh.light_vp * vec4<f32>(pos, 1.0);
}
"#;

const SHADOW_LIT_WGSL: &str = r#"
struct LitUni {
    camera_vp: mat4x4<f32>,
    light_vp: mat4x4<f32>,
    light_dir: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u_lit: LitUni;
@group(0) @binding(1) var t_shadow: texture_depth_2d;
@group(0) @binding(2) var s_shadow: sampler_comparison;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light_clip: vec4<f32>,
};

@vertex
fn vs_lit(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u_lit.camera_vp * vec4<f32>(in.pos, 1.0);
    out.normal = normalize(in.normal);
    out.color = in.color;
    out.light_clip = u_lit.light_vp * vec4<f32>(in.pos, 1.0);
    return out;
}

fn pcf_shadow(light_clip: vec4<f32>) -> f32 {
    let ndc = light_clip.xyz / max(light_clip.w, 1e-5);
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return 1.0;
    }
    let dims = vec2<f32>(textureDimensions(t_shadow));
    let texel = 1.0 / max(dims, vec2<f32>(1.0));
    let bias = 0.005;
    let cmp_z = ndc.z - bias;
    var vis = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            vis += textureSampleCompare(t_shadow, s_shadow, uv + vec2<f32>(f32(x), f32(y)) * texel, cmp_z);
        }
    }
    return vis / 9.0;
}

@fragment
fn fs_lit(in: VsOut) -> @location(0) vec4<f32> {
    let ldir = normalize(u_lit.light_dir.xyz);
    let diff = max(dot(normalize(in.normal), ldir), 0.0);
    let vis = pcf_shadow(in.light_clip);
    let shade = 0.15 + diff * vis * 0.85;
    return vec4<f32>(in.color.rgb * shade, in.color.a);
}
"#;

/// Renders a 3D mesh list through a `Depth32Float` shadow-map pass and a 3x3 PCF lit pass,
/// returning `RGBA8` (`width * height * 4`).
pub fn render_shadow_mapped_scene_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    vertices: &[ShadowVertex],
    indices: &[u32],
    camera_vp: [[f32; 4]; 4],
    light_vp: [[f32; 4]; 4],
    light_dir: [f32; 3],
    width: u32,
    height: u32,
    shadow_res: u32,
) -> Option<Vec<u8>> {
    if vertices.is_empty() || indices.is_empty() || width == 0 || height == 0 || shadow_res == 0 {
        return None;
    }

    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("shadow_vb"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("shadow_ib"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow_depth_tex"),
        size: wgpu::Extent3d {
            width: shadow_res,
            height: shadow_res,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let cam_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cam_depth_tex"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let cam_depth_view = cam_depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let color_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow_lit_color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let vbuf_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ShadowVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    };

    // Pass 1: Light-space depth-only shadow pass
    let sh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadow_depth_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_DEPTH_WGSL.into()),
    });
    let sh_uni = ShadowPassUniform { light_vp };
    let sh_uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sh_uni_buf"),
        contents: bytemuck::bytes_of(&sh_uni),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let sh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sh_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let sh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sh_layout"),
        bind_group_layouts: &[&sh_bgl],
        push_constant_ranges: &[],
    });
    let sh_pipeline = device.create_compute_or_render_shadow_pipeline(
        device,
        &sh_layout,
        &sh_shader,
        &vbuf_layout,
    );
    let sh_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sh_bg"),
        layout: &sh_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: sh_uni_buf.as_entire_binding(),
        }],
    });

    // Pass 2: Camera lit pass with 3x3 PCF shadow comparison sampler
    let lit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadow_lit_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_LIT_WGSL.into()),
    });
    let lit_uni = LitPassUniform {
        camera_vp,
        light_vp,
        light_dir: [light_dir[0], light_dir[1], light_dir[2], 0.0],
    };
    let lit_uni_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lit_uni_buf"),
        contents: bytemuck::bytes_of(&lit_uni),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let cmp_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shadow_cmp_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });
    let lit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lit_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    });
    let lit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lit_layout"),
        bind_group_layouts: &[&lit_bgl],
        push_constant_ranges: &[],
    });
    let lit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("lit_pipeline"),
        layout: Some(&lit_layout),
        vertex: wgpu::VertexState {
            module: &lit_shader,
            entry_point: "vs_lit",
            buffers: &[vbuf_layout],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &lit_shader,
            entry_point: "fs_lit",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    let lit_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lit_bg"),
        layout: &lit_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: lit_uni_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&cmp_sampler),
            },
        ],
    });

    let padded_bpr = ((width * 4 + 255) / 256) * 256;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow_lit_staging"),
        size: (padded_bpr * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("shadow_lit_encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow_depth_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&sh_pipeline);
        pass.set_bind_group(0, &sh_bg, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..(indices.len() as u32), 0, 0..1);
    }
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow_lit_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &cam_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&lit_pipeline);
        pass.set_bind_group(0, &lit_bg, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..(indices.len() as u32), 0, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &color_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &staging,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height as usize {
        let start = row * padded_bpr as usize;
        out.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    drop(data);
    staging.unmap();
    Some(out)
}

trait ShadowPipelineHelper {
    fn create_compute_or_render_shadow_pipeline(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        vbuf: &wgpu::VertexBufferLayout<'_>,
    ) -> wgpu::RenderPipeline;
}

impl ShadowPipelineHelper for wgpu::Device {
    fn create_compute_or_render_shadow_pipeline(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        vbuf: &wgpu::VertexBufferLayout<'_>,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_depth_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_shadow",
                buffers: std::slice::from_ref(vbuf),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }
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
    fn gpu_shadow_map_casts_real_pcf_shadow_onto_receiver() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        // Receiver quad at z = 0.7 covering full NDC [-1..1]; blocker quad at z = 0.2 covering center [-0.3..0.3, -0.3..0.3]
        let n = [0.0, 0.0, 1.0];
        let white = [1.0, 1.0, 1.0, 1.0];
        let verts = [
            // Receiver (indices 0..4)
            ShadowVertex { pos: [-1.0, -1.0, 0.7], normal: n, color: white },
            ShadowVertex { pos: [ 1.0, -1.0, 0.7], normal: n, color: white },
            ShadowVertex { pos: [ 1.0,  1.0, 0.7], normal: n, color: white },
            ShadowVertex { pos: [-1.0,  1.0, 0.7], normal: n, color: white },
            // Blocker (indices 4..8) - slightly offset in X/Y to cast shadow onto receiver at (-0.25, 0)
            ShadowVertex { pos: [-0.35, -0.2, 0.2], normal: n, color: [0.2, 0.5, 1.0, 1.0] },
            ShadowVertex { pos: [-0.05, -0.2, 0.2], normal: n, color: [0.2, 0.5, 1.0, 1.0] },
            ShadowVertex { pos: [-0.05,  0.2, 0.2], normal: n, color: [0.2, 0.5, 1.0, 1.0] },
            ShadowVertex { pos: [-0.35,  0.2, 0.2], normal: n, color: [0.2, 0.5, 1.0, 1.0] },
        ];
        let indices = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
        let id = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        // Light looks slightly from the left so blocker at [-0.35..-0.05] casts shadow onto receiver at +0.25
        let mut light_vp = id;
        light_vp[2][0] = -0.8; // x_light = x - 0.8 * z

        let img = render_shadow_mapped_scene_rgba8(
            &device,
            &queue,
            &verts,
            &indices,
            id,
            light_vp,
            [0.0, 0.0, 1.0],
            32,
            32,
            64,
        )
        .expect("shadow render");

        // Unshadowed right side (x=28, y=16 -> NDC x=+0.75) should be bright white (~255)
        let lit_px = (16 * 32 + 28) * 4;
        // Shadowed receiver pixel around x=20 (NDC x=+0.25, where blocker at x=-0.2, z=0.2 projects to x_light=-0.36, matching receiver x=+0.2, z=0.7 -> x_light=-0.36)
        let sh_px = (16 * 32 + 19) * 4;
        assert!(img[lit_px] > 230, "expected lit pixel > 230, got {}", img[lit_px]);
        assert!(img[sh_px] < 100, "expected shadowed pixel < 100, got {}", img[sh_px]);
    }
}
