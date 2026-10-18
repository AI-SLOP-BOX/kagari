use super::*;

#[derive(Clone, Copy)]
struct MaskRun {
    start: usize,
    end: usize,
    key: Option<u64>,
}

pub(super) struct EncodeRunsCtx<'a> {
    pub comp: &'a Composition,
    pub width: u32,
    pub height: u32,
    pub target_snapshot: bool,
    pub uniforms: &'a mut [LayerUniform],
    pub active_layers: &'a [&'a Layer],
    pub layer_mask_plans: &'a [Option<std::sync::Arc<MaskRaster>>],
    pub layer_textures: &'a [Option<std::sync::Arc<wgpu::BindGroup>>],
    pub matte_resources: &'a super::track_matte::TrackMatteResources,
}

impl WgpuRenderer {
    pub(super) fn upload_and_encode_render_runs(&mut self, ctx: EncodeRunsCtx<'_>) -> bool {
        let EncodeRunsCtx {
            comp,
            width,
            height,
            target_snapshot,
            uniforms,
            active_layers,
            layer_mask_plans,
            layer_textures,
            matte_resources,
        } = ctx;
        // layer gets mask_enabled=1; each distinct raster is uploaded in
        // its own submit during Step 3 (uploads execute just before their
        // own submit, so painter order across runs is preserved).
        for (u, plan) in uniforms.iter_mut().zip(layer_mask_plans.iter()) {
            let active = plan.is_some();
            u.mask_enabled = u32::from(active);
            u.mask_mode = u32::from(active); // 1 = alpha channel
            u.mask_inverted = 0; // inversion is baked per-mask into the raster
            u.mask_feather = 0.0; // feather is baked into the coverage ramp
        }

        if !uniforms.is_empty() {
            self.ensure_layer_buffer_capacity(uniforms.len());
            for (index, uniform) in uniforms.iter().enumerate() {
                self.queue.write_buffer(
                    &self.layer_buffer,
                    (index * LAYER_UNIFORM_STRIDE) as u64,
                    bytemuck::bytes_of(uniform),
                );
            }
        }

        let draw_count = active_layers.len();
        let mut runs: Vec<MaskRun> = Vec::new();
        for i in 0..draw_count {
            let k = layer_mask_plans
                .get(i)
                .and_then(|p| p.as_ref())
                .map(|r| r.key);
            match runs.last_mut() {
                Some(r) if r.key == k => r.end = i + 1,
                _ => runs.push(MaskRun {
                    start: i,
                    end: i + 1,
                    key: k,
                }),
            }
        }
        if runs.is_empty() {
            // No layers this frame — still clear to the background color.
            runs.push(MaskRun {
                start: 0,
                end: 0,
                key: None,
            });
        }

        for run in &runs {
            if let Some(_key) = run.key {
                if let Some(raster) = layer_mask_plans[run.start].as_ref() {
                    self.ensure_mask_texture(width, height);
                    let size = wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    };
                    self.queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: self
                                .mask_texture
                                .as_ref()
                                .expect("mask texture just ensured"),
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &rgba8_to_rgba16f_bytes(&raster.pixels),
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(width * 8),
                            rows_per_image: Some(height),
                        },
                        size,
                    );
                }
            }

            // RAM pre-pass draws straight into the ring slot's texture;
            // live/snapshot keep their dedicated targets.
            let view = if let Some(slot) = self
                .ram_ring
                .get(self.ram_render_idx)
                .and_then(|(_, t)| t.as_ref())
            {
                Some(&slot.1)
            } else if target_snapshot {
                self.snapshot_view.as_ref()
            } else {
                self.target_view.as_ref()
            };
            let Some(view) = view else { return false };

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(if target_snapshot {
                        "Snapshot Render Encoder"
                    } else {
                        "Render Encoder"
                    }),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(if target_snapshot {
                        "Snapshot Render Pass"
                    } else {
                        "Render Pass"
                    }),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if run.start == 0 {
                                wgpu::LoadOp::Clear(wgpu::Color {
                                    r: comp.background_color[0].clamp(0.0, 1.0) as f64,
                                    g: comp.background_color[1].clamp(0.0, 1.0) as f64,
                                    b: comp.background_color[2].clamp(0.0, 1.0) as f64,
                                    a: comp.background_color[3].clamp(0.0, 1.0) as f64,
                                })
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.set_bind_group(0, &self.globals_bind_group, &[]);

                #[allow(clippy::needless_range_loop)]
                for i in run.start..run.end {
                    let pipeline = match active_layers[i].blend_mode {
                        crate::core::timeline::BlendMode::Screen => &self.render_pipeline_screen,
                        _ => &self.render_pipeline,
                    };
                    render_pass.set_pipeline(pipeline);

                    // Bind resources using dynamic uniform offset
                    let dynamic_offset = (i * LAYER_UNIFORM_STRIDE) as u32;
                    render_pass.set_bind_group(1, &self.layer_bind_group, &[dynamic_offset]);

                    // Texture binding (use per-layer text texture when available, dummy for solid/SDF shapes)
                    let tex_bg: &wgpu::BindGroup = match layer_textures.get(i) {
                        Some(Some(bg)) => bg,
                        _ => &self.dummy_texture_bind_group,
                    };
                    render_pass.set_bind_group(2, tex_bg, &[]);
                    // Combined mask+shadow+matte bind group (group 3)
                    let combined_bg: &wgpu::BindGroup = match matte_resources.bind_groups.get(i) {
                        Some(Some(bg)) => bg,
                        _ => &self.combined_bind_group,
                    };
                    render_pass.set_bind_group(3, combined_bg, &[]);

                    // Draw!
                    render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
                }
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }
        true
    }
}
