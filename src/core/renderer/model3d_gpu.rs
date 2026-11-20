use super::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Model3dVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub pbr: [f32; 4], // x: roughness, y: metallic, z: emission_luma, w: unused
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Model3dUniform {
    mvp: [[f32; 4]; 4],
    normal_mat: [[f32; 4]; 4],
}

pub(super) const MODEL3D_WGSL: &str = r#"
struct ModelUniform {
    mvp: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> u_model: ModelUniform;
@group(0) @binding(1) var t_albedo: texture_2d<f32>;
@group(0) @binding(2) var s_albedo: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) pbr: vec4<f32>,
    @location(4) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) pbr: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

@vertex
fn vs_model3d(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u_model.mvp * vec4<f32>(in.pos, 1.0);
    out.normal = normalize((u_model.normal_mat * vec4<f32>(in.normal, 0.0)).xyz);
    out.color = in.color;
    out.pbr = in.pbr;
    out.uv = in.uv;
    return out;
}

fn studio_env_reflection(r: vec3<f32>, rough: f32, metal: f32) -> vec3<f32> {
    let sky_t = clamp(r.y * 0.5 + 0.5, 0.0, 1.0);
    let horizon = vec3<f32>(0.55, 0.65, 0.78);
    let zenith = vec3<f32>(0.18, 0.28, 0.45);
    let ground = vec3<f32>(0.08, 0.07, 0.06);
    var env = mix(ground, horizon, smoothstep(0.0, 0.52, sky_t));
    env = mix(env, zenith, smoothstep(0.52, 1.0, sky_t));
    // Studio softbox key strip + warm rim strip
    let softbox1 = pow(max(dot(r, normalize(vec3<f32>(0.55, 0.65, 0.52))), 0.0), mix(48.0, 4.0, rough));
    let softbox2 = pow(max(dot(r, normalize(vec3<f32>(-0.7, 0.3, -0.6))), 0.0), mix(32.0, 3.0, rough));
    env += vec3<f32>(1.0, 0.96, 0.9) * softbox1 * 1.1 + vec3<f32>(0.55, 0.75, 1.0) * softbox2 * 0.6;
    let env_weight = (1.0 - rough * 0.75) * mix(0.12, 0.85, metal);
    return env * env_weight;
}

@fragment
fn fs_model3d(in: VsOut) -> @location(0) vec4<f32> {
    let tex_col = textureSample(t_albedo, s_albedo, in.uv);
    let base_col = in.color * tex_col;
    let light_dir = normalize(vec3<f32>(0.45, -0.55, 0.7));
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    let n = normalize(in.normal);
    let diff = max(dot(n, light_dir), 0.0);
    let back = max(dot(-n, light_dir), 0.0) * 0.25;
    let rough = clamp(in.pbr.x, 0.05, 1.0);
    let metal = clamp(in.pbr.y, 0.0, 1.0);
    let half_v = normalize(light_dir + view_dir);
    let shininess = mix(128.0, 4.0, rough);
    let spec = pow(max(dot(n, half_v), 0.0), shininess) * mix(0.25, 0.9, 1.0 - rough);
    let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0) * (0.15 + metal * 0.35);
    let reflect_dir = reflect(-view_dir, n);
    let env_refl = studio_env_reflection(reflect_dir, rough, metal) * mix(vec3<f32>(1.0), base_col.rgb, metal);
    let shade = 0.26 + diff * (0.74 - metal * 0.25) + back;
    let spec_col = mix(vec3<f32>(1.0), base_col.rgb, metal) * (spec + fresnel);
    let lit = base_col.rgb * shade + spec_col + env_refl + base_col.rgb * in.pbr.z;
    return vec4<f32>(clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0)), base_col.a);
}
"#;

pub(super) fn create_model3d_pipeline(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("model3d_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Model3D Pipeline Layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Model3D Shader"),
        source: wgpu::ShaderSource::Wgsl(MODEL3D_WGSL.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model3D Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_model3d",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Model3dVertex>() as wgpu::BufferAddress,
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
                    wgpu::VertexAttribute {
                        offset: 40,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                    wgpu::VertexAttribute {
                        offset: 56,
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_model3d",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
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
    (bgl, pipeline)
}

fn mat4_mul_3d(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] = a[0][r] * b[c][0]
                + a[1][r] * b[c][1]
                + a[2][r] * b[c][2]
                + a[3][r] * b[c][3];
        }
    }
    out
}

impl WgpuRenderer {
    pub(super) fn render_model3d_to_gpu_texture(
        &self,
        comp: &Composition,
        layer: &Layer,
        frame: u32,
        width: u32,
        height: u32,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let LayerType::Model3D { path } = &layer.layer_type else {
            return None;
        };
        let mesh = crate::core::software_renderer::model3d_raster::load_mesh(path)?;
        if mesh.vertices.is_empty() || mesh.indices.len() < 3 {
            return None;
        }
        let w = width.max(1);
        let h = height.max(1);

        let center = [
            (mesh.bbox_min[0] + mesh.bbox_max[0]) * 0.5,
            (mesh.bbox_min[1] + mesh.bbox_max[1]) * 0.5,
            (mesh.bbox_min[2] + mesh.bbox_max[2]) * 0.5,
        ];
        let extent = (mesh.bbox_max[0] - mesh.bbox_min[0])
            .max(mesh.bbox_max[1] - mesh.bbox_min[1])
            .max(mesh.bbox_max[2] - mesh.bbox_min[2])
            .max(1.0e-4);
        let base_scale = (w.min(h) as f32 * 0.72 / extent).max(0.001);
        let position = layer.transform_3d.position.evaluate(frame);
        let rotation = layer.transform_3d.rotation.evaluate(frame);
        let model_scale = layer
            .transform_3d
            .scale
            .evaluate(frame)
            .map(|v| v / 100.0);

        let [rx, ry, rz] = rotation.map(f32::to_radians);
        let (sx, cx) = rx.sin_cos();
        let (sy, cy) = ry.sin_cos();
        let (sz, cz) = rz.sin_cos();
        let rot_x = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, cx, sx, 0.0],
            [0.0, -sx, cx, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let rot_y = [
            [cy, 0.0, -sy, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [sy, 0.0, cy, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let rot_z = [
            [cz, sz, 0.0, 0.0],
            [-sz, cz, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let rot_mat = mat4_mul_3d(rot_z, mat4_mul_3d(rot_y, rot_x));
        let scale_mat = [
            [base_scale * model_scale[0], 0.0, 0.0, 0.0],
            [0.0, base_scale * model_scale[1], 0.0, 0.0],
            [0.0, 0.0, base_scale * model_scale[2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let center_mat = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-center[0], -center[1], -center[2], 1.0],
        ];
        let pos_mat = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [position[0], position[1], position[2], 1.0],
        ];
        let model_mat = mat4_mul_3d(
            pos_mat,
            mat4_mul_3d(rot_mat, mat4_mul_3d(scale_mat, center_mat)),
        );

        let camera = comp.resolve_camera_at();
        let fov = camera.fov_at(frame).max(1.0).to_radians();
        let focal = (h as f32 * 0.5) / (fov * 0.5).tan();
        let cam_pos = camera.transform.position.evaluate(frame);
        let view_t = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-cam_pos[0], -cam_pos[1], -cam_pos[2], 1.0],
        ];
        let near = 10.0f32;
        let far = 20000.0f32;
        let proj = [
            [2.0 * focal / w as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 * focal / h as f32, 0.0, 0.0],
            [0.0, 0.0, far / (far - near), 1.0],
            [0.0, 0.0, -(far * near) / (far - near), 0.0],
        ];
        let mvp = mat4_mul_3d(proj, mat4_mul_3d(view_t, model_mat));

        let mut verts = Vec::with_capacity(mesh.indices.len());
        let mut tri_indices = Vec::with_capacity(mesh.indices.len());
        for (tri_idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
            let mat = mesh
                .material_indices
                .get(tri_idx)
                .and_then(|&mi| mesh.materials.get(mi));
            let (mat_col, roughness, metallic, em_luma) = if let Some(m) = mat {
                (
                    m.albedo_color,
                    m.roughness,
                    m.metallic,
                    (m.emission[0] + m.emission[1] + m.emission[2]) / 3.0,
                )
            } else {
                ([0.78, 0.82, 0.9, 1.0], 0.45, 0.1, 0.0)
            };
            for &vi in tri {
                let idx = vi as usize;
                let pos = mesh.vertices.get(idx).copied().unwrap_or([0.0; 3]);
                let normal = mesh.normals.get(idx).copied().unwrap_or([0.0, 0.0, 1.0]);
                let vcol = mesh.colors.get(idx).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                let uv = mesh.uvs.get(idx).copied().unwrap_or([0.0, 0.0]);
                let color = [
                    vcol[0] * mat_col[0],
                    vcol[1] * mat_col[1],
                    vcol[2] * mat_col[2],
                    vcol[3] * mat_col[3],
                ];
                tri_indices.push(verts.len() as u32);
                verts.push(Model3dVertex {
                    pos,
                    normal,
                    color,
                    pbr: [roughness, metallic, em_luma, 1.0],
                    uv,
                });
            }
        }

        let vb = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model3D VB"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ib = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model3D IB"),
                contents: bytemuck::cast_slice(&tri_indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let u_data = Model3dUniform {
            mvp,
            normal_mat: rot_mat,
        };
        let ub = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model3D UB"),
                contents: bytemuck::bytes_of(&u_data),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let albedo_img = mesh
            .materials
            .iter()
            .find_map(|m| m.albedo_texture_index.and_then(|i| mesh.embedded_textures.get(i)));
        let (albedo_w, albedo_h, albedo_pixels): (u32, u32, &[u8]) = if let Some(img) = albedo_img {
            (img.width.max(1), img.height.max(1), &img.pixels)
        } else {
            (1, 1, &[255, 255, 255, 255])
        };
        let albedo_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Model3D Albedo Texture"),
            size: wgpu::Extent3d {
                width: albedo_w,
                height: albedo_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &albedo_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            albedo_pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(albedo_w * 4),
                rows_per_image: Some(albedo_h),
            },
            wgpu::Extent3d {
                width: albedo_w,
                height: albedo_h,
                depth_or_array_layers: 1,
            },
        );
        let albedo_view = albedo_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model3d_ub_bg"),
            layout: &self.model3d_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ub.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let color_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Model3D Color Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Model3D Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Model3D GPU Pass"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Model3D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.model3d_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..(tri_indices.len() as u32), 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        let tex_bg = std::sync::Arc::new(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model3d_tex_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        Some((w, h, tex_bg))
    }
}

/// Evaluates `mograph_cloner` instances into `(mvp, model_matrix)` pairs ready for
/// Element 3D V2-style instanced `Model3D` rendering.
#[allow(dead_code)]
pub(crate) fn eval_cloner_model3d_mvps(
    mode: &crate::core::mograph_cloner::ClonerMode,
    effectors: &[crate::core::mograph_cloner::Effector],
    view_proj: [[f32; 4]; 4],
) -> Vec<([[f32; 4]; 4], [[f32; 4]; 4])> {
    crate::core::mograph_cloner::evaluate_cloner(mode, effectors)
        .into_iter()
        .map(|inst| {
            let mvp = super::helpers::mat4_mul(view_proj, inst.model_matrix);
            (mvp, inst.model_matrix)
        })
        .collect()
}

#[cfg(test)]
mod cloner_bridge_tests {
    use super::eval_cloner_model3d_mvps;
    use crate::core::mograph_cloner::{ClonerMode, Effector, FalloffField};

    #[test]
    fn cloner_produces_instanced_model3d_mvps() {
        let mode = ClonerMode::Linear {
            count: 3,
            offset: [100.0, 0.0, 0.0],
            rotate_step_deg: [0.0, 15.0, 0.0],
            scale_step: [0.0, 0.0, 0.0],
        };
        let eff = [Effector::Plain {
            strength: 1.0,
            position: [0.0, 20.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            falloff: FalloffField::Infinite,
        }];
        let id = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mvps = eval_cloner_model3d_mvps(&mode, &eff, id);
        assert_eq!(mvps.len(), 3);
        // Instance 2: x = 200.0, y = 20.0
        assert!((mvps[2].0[3][0] - 200.0).abs() < 1e-4);
        assert!((mvps[2].0[3][1] - 20.0).abs() < 1e-4);
    }
}
