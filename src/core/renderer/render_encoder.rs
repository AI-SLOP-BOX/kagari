use super::*;

#[derive(Clone, Copy)]
pub(super) struct ScratchDrawStep {
    pub uniform_index: usize,
    pub src_is_scratch: Option<usize>,
    pub dst_is_a: bool,
    pub custom_shader: Option<(u64, [f32; 4])>,
    pub lut_pass: Option<(u64, f32)>,
}

#[derive(Clone, Copy)]
struct MaskRun {
    start: usize,
    end: usize,
    key: Option<u64>,
    isolated_blend: bool,
    has_scratch_passes: bool,
}

pub(super) fn needs_backdrop_copy(layer: &Layer) -> bool {
    is_programmable_blend(layer.blend_mode)
        || matches!(
            layer.layer_type,
            crate::core::timeline::LayerType::AdjustmentLayer
        )
        || (layer.effects_enabled
            && layer.effects.iter().any(|e| {
                e.enabled
                    && matches!(
                        e.effect_type,
                        crate::core::timeline::EffectType::Echo {
                            operator: crate::core::echo_effect::EchoOperator::Maximum
                                | crate::core::echo_effect::EchoOperator::Minimum,
                            ..
                        }
                    )
            }))
}

fn blend_mode_from_u32(code: u32, fallback: crate::core::timeline::BlendMode) -> crate::core::timeline::BlendMode {
    use crate::core::timeline::BlendMode;
    match code {
        0 => BlendMode::Normal,
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Add,
        5 => BlendMode::Darken,
        6 => BlendMode::Lighten,
        7 => BlendMode::SoftLight,
        8 => BlendMode::HardLight,
        9 => BlendMode::Difference,
        10 => BlendMode::Exclusion,
        11 => BlendMode::Divide,
        12 => BlendMode::Subtract,
        13 => BlendMode::ColorBurn,
        14 => BlendMode::LinearBurn,
        15 => BlendMode::VividLight,
        16 => BlendMode::ColorDodge,
        17 => BlendMode::LinearDodge,
        18 => BlendMode::Color,
        19 => BlendMode::Hue,
        20 => BlendMode::Saturation,
        21 => BlendMode::Luminosity,
        22 => BlendMode::StencilAlpha,
        23 => BlendMode::StencilLuma,
        24 => BlendMode::SilhouetteAlpha,
        25 => BlendMode::SilhouetteLuma,
        26 => BlendMode::Behind,
        27 => BlendMode::AlphaAdd,
        28 => BlendMode::LinearLight,
        _ => fallback,
    }
}

fn is_programmable_blend(mode: crate::core::timeline::BlendMode) -> bool {
    use crate::core::timeline::BlendMode;
    matches!(
        mode,
        BlendMode::Overlay
            | BlendMode::Multiply
            | BlendMode::Darken
            | BlendMode::Lighten
            | BlendMode::SoftLight
            | BlendMode::HardLight
            | BlendMode::Difference
            | BlendMode::Exclusion
            | BlendMode::Divide
            | BlendMode::Subtract
            | BlendMode::ColorBurn
            | BlendMode::LinearBurn
            | BlendMode::VividLight
            | BlendMode::ColorDodge
            | BlendMode::LinearLight
            | BlendMode::Color
            | BlendMode::Hue
            | BlendMode::Saturation
            | BlendMode::Luminosity
            | BlendMode::StencilAlpha
            | BlendMode::StencilLuma
            | BlendMode::SilhouetteAlpha
            | BlendMode::SilhouetteLuma
            | BlendMode::AlphaAdd
    )
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
    pub final_uniform_indices: &'a [usize],
    pub scratch_steps_per_layer: &'a [Vec<ScratchDrawStep>],
    pub final_src_scratch: &'a [Option<usize>],
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
            final_uniform_indices,
            scratch_steps_per_layer,
            final_src_scratch,
        } = ctx;
        // layer gets mask_enabled=1; each distinct raster is uploaded in
        // its own submit during Step 3 (uploads execute just before their
        // own submit, so painter order across runs is preserved).
        for (layer_idx, plan) in layer_mask_plans.iter().enumerate() {
            let active = plan.is_some();
            let u = &mut uniforms[final_uniform_indices[layer_idx]];
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
            let isolated_blend = needs_backdrop_copy(active_layers[i]);
            let has_scratch_passes = !scratch_steps_per_layer[i].is_empty();
            match runs.last_mut() {
                Some(r)
                    if r.key == k
                        && !r.isolated_blend
                        && !isolated_blend
                        && !r.has_scratch_passes
                        && !has_scratch_passes =>
                {
                    r.end = i + 1;
                }
                _ => runs.push(MaskRun {
                    start: i,
                    end: i + 1,
                    key: k,
                    isolated_blend,
                    has_scratch_passes,
                }),
            }
        }
        if runs.is_empty() {
            // No layers this frame — still clear to the background color.
            runs.push(MaskRun {
                start: 0,
                end: 0,
                key: None,
                isolated_blend: false,
                has_scratch_passes: false,
            });
        } else if runs[0].isolated_blend {
            // Ensure background clear happens before the first backdrop copy.
            runs.insert(
                0,
                MaskRun {
                    start: 0,
                    end: 0,
                    key: None,
                    isolated_blend: false,
                    has_scratch_passes: false,
                },
            );
        }

        for (run_index, run) in runs.iter().enumerate() {
            if let Some(_key) = run.key {
                if let Some(raster) = layer_mask_plans[run.start].as_ref() {
                    self.ensure_mask_texture(width, height);
                    let size = wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    };
                    let Some(upload_bytes) = rgba8_to_rgba16f_bytes(&raster.pixels) else {
                        return false;
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
                        &upload_bytes,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(width * 8),
                            rows_per_image: Some(height),
                        },
                        size,
                    );
                }
            }

            if run.isolated_blend {
                self.ensure_backdrop_texture(width, height);
            }
            if run.has_scratch_passes {
                self.ensure_scratch_textures(width, height);
            }

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(if target_snapshot {
                        "Snapshot Render Encoder"
                    } else {
                        "Render Encoder"
                    }),
                });

            if run.isolated_blend {
                let target_tex = if let Some(slot) = self
                    .ram_ring
                    .get(self.ram_render_idx)
                    .and_then(|(_, t)| t.as_ref())
                {
                    Some(&slot.0)
                } else if target_snapshot {
                    self.snapshot_texture.as_ref()
                } else {
                    self.target_texture.as_ref()
                };
                if let (Some(src_tex), Some(dst_tex)) = (target_tex, self.backdrop_texture.as_ref())
                {
                    encoder.copy_texture_to_texture(
                        src_tex.as_image_copy(),
                        dst_tex.as_image_copy(),
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }

            if run.has_scratch_passes {
                let layer_idx = run.start;
                for step in &scratch_steps_per_layer[layer_idx] {
                    let Some(dst_view) = (if step.dst_is_a {
                        self.scratch_a_view.as_ref()
                    } else {
                        self.scratch_b_view.as_ref()
                    }) else {
                        return false;
                    };
                    let mut scratch_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Layer Multipass Scratch Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: dst_view,
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
                    if let Some((cs_hash, cs_params)) = step.custom_shader {
                        if let Some(Some(cs_pipeline)) =
                            self.custom_shader_pipelines.borrow().get(&cs_hash)
                        {
                            let src_view = match step.src_is_scratch {
                                Some(0) => self.scratch_a_view.as_ref(),
                                Some(_) => self.scratch_b_view.as_ref(),
                                None => None,
                            };
                            if let Some(src_view) = src_view {
                                let u_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                    label: Some("Custom Shader Params Buffer"),
                                    size: 16,
                                    usage: wgpu::BufferUsages::UNIFORM,
                                    mapped_at_creation: true,
                                });
                                u_buf
                                    .slice(..)
                                    .get_mapped_range_mut()
                                    .copy_from_slice(bytemuck::cast_slice(&cs_params));
                                u_buf.unmap();
                                let cs_bg =
                                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: &self.custom_shader_bgl,
                                        entries: &[
                                            wgpu::BindGroupEntry {
                                                binding: 0,
                                                resource: wgpu::BindingResource::TextureView(
                                                    src_view,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 1,
                                                resource: wgpu::BindingResource::Sampler(
                                                    &self.sampler,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 2,
                                                resource: u_buf.as_entire_binding(),
                                            },
                                        ],
                                        label: Some("custom_shader_bg"),
                                    });
                                scratch_pass.set_pipeline(cs_pipeline);
                                scratch_pass.set_bind_group(0, &cs_bg, &[]);
                                scratch_pass.draw(0..3, 0..1);
                                continue;
                            }
                        }
                    }
                    if let Some((lut_key, lut_intensity)) = step.lut_pass {
                        if let Some(Some((lut_view, lut_size))) =
                            self.lut_cache.borrow().get(&lut_key)
                        {
                            let src_view = match step.src_is_scratch {
                                Some(0) => self.scratch_a_view.as_ref(),
                                Some(_) => self.scratch_b_view.as_ref(),
                                None => None,
                            };
                            if let Some(src_view) = src_view {
                                let lut_params = [lut_intensity, *lut_size as f32, 0.0, 0.0];
                                let u_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                    label: Some("LUT 3D Params Buffer"),
                                    size: 16,
                                    usage: wgpu::BufferUsages::UNIFORM,
                                    mapped_at_creation: true,
                                });
                                u_buf
                                    .slice(..)
                                    .get_mapped_range_mut()
                                    .copy_from_slice(bytemuck::cast_slice(&lut_params));
                                u_buf.unmap();
                                let lut_bg =
                                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: &self.lut_bgl,
                                        entries: &[
                                            wgpu::BindGroupEntry {
                                                binding: 0,
                                                resource: wgpu::BindingResource::TextureView(
                                                    src_view,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 1,
                                                resource: wgpu::BindingResource::Sampler(
                                                    &self.sampler,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 2,
                                                resource: wgpu::BindingResource::TextureView(
                                                    lut_view,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 3,
                                                resource: wgpu::BindingResource::Sampler(
                                                    &self.sampler,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 4,
                                                resource: u_buf.as_entire_binding(),
                                            },
                                        ],
                                        label: Some("lut3d_bg"),
                                    });
                                scratch_pass.set_pipeline(&self.lut_pipeline);
                                scratch_pass.set_bind_group(0, &lut_bg, &[]);
                                scratch_pass.draw(0..3, 0..1);
                                continue;
                            }
                        }
                    }
                    scratch_pass.set_pipeline(&self.render_pipeline_replace);
                    scratch_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    scratch_pass.set_index_buffer(
                        self.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    scratch_pass.set_bind_group(0, &self.globals_bind_group, &[]);
                    let dynamic_offset = (step.uniform_index * LAYER_UNIFORM_STRIDE) as u32;
                    scratch_pass.set_bind_group(1, &self.layer_bind_group, &[dynamic_offset]);
                    let tex_bg: &wgpu::BindGroup = match step.src_is_scratch {
                        Some(0) => self
                            .scratch_a_bind_group
                            .as_deref()
                            .unwrap_or(&self.dummy_texture_bind_group),
                        Some(_) => self
                            .scratch_b_bind_group
                            .as_deref()
                            .unwrap_or(&self.dummy_texture_bind_group),
                        None => match layer_textures.get(layer_idx) {
                            Some(Some(bg)) => bg,
                            _ => &self.dummy_texture_bind_group,
                        },
                    };
                    scratch_pass.set_bind_group(2, tex_bg, &[]);
                    let combined_bg: &wgpu::BindGroup = match matte_resources.bind_groups.get(layer_idx)
                    {
                        Some(Some(bg)) => bg,
                        _ => &self.combined_bind_group,
                    };
                    scratch_pass.set_bind_group(3, combined_bg, &[]);
                    scratch_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
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
                            load: if run_index == 0 {
                                let background_alpha =
                                    comp.background_color[3].clamp(0.0, 1.0) as f64;
                                let encode_bg = |c: f32| {
                                    let clamped = c.clamp(0.0, 1.0);
                                    let space = if comp.blend_linear {
                                        crate::core::color::srgb_to_linear_piecewise(clamped)
                                    } else {
                                        clamped
                                    };
                                    space as f64 * background_alpha
                                };
                                wgpu::LoadOp::Clear(wgpu::Color {
                                    r: encode_bg(comp.background_color[0]),
                                    g: encode_bg(comp.background_color[1]),
                                    b: encode_bg(comp.background_color[2]),
                                    a: background_alpha,
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
                    let effective_blend = blend_mode_from_u32(
                        uniforms[final_uniform_indices[i]].blend_mode,
                        active_layers[i].blend_mode,
                    );
                    let pipeline = if is_programmable_blend(effective_blend) {
                        &self.render_pipeline_replace
                    } else {
                        match effective_blend {
                            crate::core::timeline::BlendMode::Multiply => {
                                &self.render_pipeline_multiply
                            }
                            crate::core::timeline::BlendMode::Screen => {
                                &self.render_pipeline_screen
                            }
                            crate::core::timeline::BlendMode::Add
                            | crate::core::timeline::BlendMode::LinearDodge => {
                                &self.render_pipeline_add
                            }
                            crate::core::timeline::BlendMode::Behind => {
                                &self.render_pipeline_behind
                            }
                            _ => &self.render_pipeline,
                        }
                    };
                    render_pass.set_pipeline(pipeline);

                    // Bind resources using dynamic uniform offset
                    let dynamic_offset = (final_uniform_indices[i] * LAYER_UNIFORM_STRIDE) as u32;
                    render_pass.set_bind_group(1, &self.layer_bind_group, &[dynamic_offset]);

                    // Texture binding (use scratch result, per-layer texture, or dummy)
                    let tex_bg: &wgpu::BindGroup = match final_src_scratch[i] {
                        Some(0) => self
                            .scratch_a_bind_group
                            .as_deref()
                            .unwrap_or(&self.dummy_texture_bind_group),
                        Some(_) => self
                            .scratch_b_bind_group
                            .as_deref()
                            .unwrap_or(&self.dummy_texture_bind_group),
                        None => match layer_textures.get(i) {
                            Some(Some(bg)) => bg,
                            _ => &self.dummy_texture_bind_group,
                        },
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
