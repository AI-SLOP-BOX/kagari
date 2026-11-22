use super::*;
mod expand_modifiers;
mod prepare;

#[derive(Clone)]
pub(super) struct LayerMultipassPlan {
    pub(super) steps: Vec<(
        LayerUniform,
        Option<usize>,
        bool,
        Option<(u64, [f32; 4])>,
        Option<(u64, f32)>,
    )>,
    pub(super) final_src_scratch: usize,
}

pub(super) struct PreparedLayerInputs<'a> {
    pub(super) active_layers: Vec<&'a crate::core::timeline::Layer>,
    pub(super) comp_indices: Vec<usize>,
    pub(super) uniforms: Vec<LayerUniform>,
    pub(super) layer_mask_plans: Vec<Option<std::sync::Arc<MaskRaster>>>,
    pub(super) multipass_plans: Vec<Option<LayerMultipassPlan>>,
}

impl WgpuRenderer {
    fn ensure_custom_shader_pipeline(&self, hash: u64, wgsl_source: &str) {
        if self.custom_shader_pipelines.borrow().contains_key(&hash) {
            return;
        }
        let full_wgsl = if wgsl_source.contains("@vertex") {
            wgsl_source.to_string()
        } else {
            format!(
                "struct CustomVsOut {{ @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, }};\n\
                 @vertex fn vs_custom(@builtin(vertex_index) vid: u32) -> CustomVsOut {{\n\
                     var out: CustomVsOut;\n\
                     let x = f32((vid << 1u) & 2u);\n\
                     let y = f32(vid & 2u);\n\
                     out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);\n\
                     out.uv = vec2<f32>(x, y);\n\
                     return out;\n\
                 }}\n{wgsl_source}"
            )
        };
        let vs_entry = if wgsl_source.contains("@vertex") {
            "vs_main"
        } else {
            "vs_custom"
        };
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom WGSL Effect Shader"),
            source: wgpu::ShaderSource::Wgsl(full_wgsl.into()),
        });
        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom WGSL Effect Pipeline"),
            layout: Some(&self.custom_shader_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: vs_entry,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
        let err = pollster::block_on(self.device.pop_error_scope());
        self.custom_shader_pipelines
            .borrow_mut()
            .insert(hash, if err.is_none() { Some(pipeline) } else { None });
    }

    fn clear_target_after_mask_failure(&self, target_snapshot: bool, background: [f32; 4]) {
        let view = if target_snapshot {
            self.snapshot_view.as_ref()
        } else {
            self.target_view.as_ref()
        };
        let Some(view) = view else {
            return;
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Clear target after mask raster failure"),
            });
        let background_alpha = background[3].clamp(0.0, 1.0) as f64;
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear target after mask raster failure"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0].clamp(0.0, 1.0) as f64 * background_alpha,
                            g: background[1].clamp(0.0, 1.0) as f64 * background_alpha,
                            b: background[2].clamp(0.0, 1.0) as f64 * background_alpha,
                            a: background_alpha,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(Some(encoder.finish()));
    }

    /// Internal core rendering implementation for both primary preview and snapshot target views.
    pub(super) fn render_internal(
        &mut self,
        comp: &Composition,
        frame: u32,
        exposure_ev: f32,
        lut_mode: u32,
        target_snapshot: bool,
    ) -> bool {
        // Dirty-checking: the viewport calls render() at display refresh rate even
        // when nothing changed. Skip the full encode/upload/draw pass when the
        // project version, frame, exposure, LUT, and target size are unchanged.
        // Effective preview resolution: downscale large comps to the viewport cap.
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (cap.max(1), ((comp.height as f32 * s) as u32).max(1))
            }
            _ => (comp.width, comp.height),
        };

        let composition_key = composition_cache_key(comp);
        let media_revision = composition_media_revision(comp, frame);
        let render_key: RenderKey = (
            crate::core::frame_cache::current_version(),
            composition_key,
            media_revision,
            frame,
            exposure_ev.to_bits(),
            lut_mode,
            (eff_w, eff_h),
        );
        let last_key = if target_snapshot {
            self.last_snapshot_key
        } else {
            self.last_main_key
        };
        let ram_mode = self.ram_render_idx != usize::MAX;
        if !ram_mode && last_key == Some(render_key) {
            return false; // nothing changed — reuse the existing target texture
        }

        // Clamp to both our sanity limit and the device's texture limit —
        // oversized textures would trip wgpu validation and abort the process.
        let max_dim = self
            .device
            .limits()
            .max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);
        let recreated = self.ensure_target_size(width, height);

        // Update Globals Uniform
        let globals = GlobalsUniform {
            viewport_size: [width as f32, height as f32],
            exposure_ev,
            lut_mode,
            shadow_enabled: self.shadow_active as u32,
            shadow_strength: 1.0,
            channel_mode: 0,
            wipe_split_x: 0.0,
            dither_output: comp.dither_output as u32,
            blend_linear: comp.blend_linear as u32,
            working_color_space: comp.working_color_space as u32,
            _pad2: 0,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Per-layer text texture bind groups (declared before the render pass so they outlive it)
        let mut layer_textures: Vec<Option<std::sync::Arc<wgpu::BindGroup>>> = Vec::new();

        // Target must exist; per-run views are borrowed fresh inside the Step-3
        // loop so mask texture uploads (&mut self) fit between submits.
        if (target_snapshot && self.snapshot_view.is_none())
            || (!target_snapshot && self.target_view.is_none())
        {
            return false;
        }

        // Expression and Audio Spectrum values are thread-local. Install the
        // same frame context used by the software renderer before evaluating
        // any GPU-layer transform or effect parameters.
        crate::core::software_renderer::inject_audio_reactive_data(comp, frame);

        {
            // Viewport projection matrix:
            // Maps [0, width] to [-1, 1] on X, and [0, height] to [1, -1] on Y.
            let m_proj = [
                [2.0 / width as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / height as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0], // Column-major translation: -1 on x, 1 on y
            ];

            // Step 1: Pre-evaluate active layer transform matrices and effect properties
            let Some(mut prepared) = self.prepare_active_layers(
                comp,
                frame,
                width,
                height,
                m_proj,
                target_snapshot,
                render_key,
                &mut layer_textures,
            ) else {
                return true;
            };
            self.expand_layer_modifiers(comp, frame, &mut prepared, &mut layer_textures);
            let PreparedLayerInputs {
                active_layers,
                comp_indices,
                mut uniforms,
                layer_mask_plans,
                multipass_plans,
            } = prepared;

            let final_uniform_indices: Vec<usize> = (0..active_layers.len()).collect();
            let mut scratch_steps_per_layer: Vec<Vec<super::render_encoder::ScratchDrawStep>> =
                vec![Vec::new(); active_layers.len()];
            let mut final_src_scratch: Vec<Option<usize>> = vec![None; active_layers.len()];
            for (layer_idx, plan_opt) in multipass_plans.into_iter().enumerate() {
                if let Some(plan) = plan_opt {
                    final_src_scratch[layer_idx] = Some(plan.final_src_scratch);
                    for (u, src_is_scratch, dst_is_a, custom_shader, lut_pass) in plan.steps {
                        let u_idx = uniforms.len();
                        uniforms.push(u);
                        scratch_steps_per_layer[layer_idx].push(
                            super::render_encoder::ScratchDrawStep {
                                uniform_index: u_idx,
                                src_is_scratch,
                                dst_is_a,
                                custom_shader,
                                lut_pass,
                            },
                        );
                    }
                }
            }

            // Step 1b: Ensure backdrop texture exists if needed, then prepare track matte textures.
            if active_layers
                .iter()
                .any(|l| super::render_encoder::needs_backdrop_copy(l))
            {
                self.ensure_backdrop_texture(width, height);
            }
            let Some(matte_resources) = self.prepare_track_mattes(
                comp,
                frame,
                width,
                height,
                &active_layers,
                &comp_indices,
            ) else {
                return false;
            };

            if !self.upload_and_encode_render_runs(super::render_encoder::EncodeRunsCtx {
                comp,
                width,
                height,
                target_snapshot,
                uniforms: &mut uniforms,
                active_layers: &active_layers,
                layer_mask_plans: &layer_mask_plans,
                layer_textures: &layer_textures,
                matte_resources: &matte_resources,
                final_uniform_indices: &final_uniform_indices,
                scratch_steps_per_layer: &scratch_steps_per_layer,
                final_src_scratch: &final_src_scratch,
            }) {
                return false;
            }
        }

        // Remember inputs so redundant renders can be skipped.
        // RAM pre-pass frames never touch the live-view keys.
        if !ram_mode {
            if target_snapshot {
                self.last_snapshot_key = Some(render_key);
            } else {
                self.last_main_key = Some(render_key);
            }
        }
        recreated
    }
}
