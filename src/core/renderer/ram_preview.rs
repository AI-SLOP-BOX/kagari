use super::*;

impl WgpuRenderer {
    pub fn shadow_active(&self) -> bool {
        self.shadow_active
    }

    /// Upload a CPU-built shadow density map ([0..1] per pixel) for GPU-side
    /// sampling. Recreates the texture on size change; no-op when empty.
    pub fn update_shadow_map(&mut self, density: &[f32], width: u32, height: u32) {
        if width == 0 || height == 0 || density.len() != (width as usize) * (height as usize) {
            self.shadow_active = false;
            return;
        }
        let max_dim = self
            .device
            .limits()
            .max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let (w, h) = (width.min(max_dim), height.min(max_dim));

        // Pack f32 density into Rgba16Float (f16 per channel, 8 bytes/pixel).
        let mut bytes = Vec::with_capacity((w * h * 8) as usize);
        for y in 0..h {
            for x in 0..w {
                let d = density[(y as usize) * width as usize + x as usize];
                let v = d.clamp(0.0, 1.0);
                let f = f16::from_f32(v);
                bytes.extend_from_slice(&f.to_le_bytes());
                bytes.extend_from_slice(&f16::from_f32(0.0).to_le_bytes());
                bytes.extend_from_slice(&f16::from_f32(0.0).to_le_bytes());
                bytes.extend_from_slice(&f16::from_f32(1.0).to_le_bytes());
            }
        }

        if self.shadow_size != (w, h) || self.shadow_texture.is_none() {
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Shadow Density Map"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            self.shadow_texture = Some(tex);
            self.shadow_view = Some(view);
            self.shadow_size = (w, h);
            self.combined_bind_group = self.rebuild_combined_bind_group();
        }

        if let Some(tex) = &self.shadow_texture {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 8),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.shadow_active = true;
    }

    /// Disable shadow sampling (no casting lights in the comp).
    pub fn clear_shadow_map(&mut self) {
        self.shadow_active = false;
    }

    pub fn render_ram_preview_range(
        &mut self,
        comp: &Composition,
        from: u32,
        to: u32,
        exposure_ev: f32,
        lut_mode: u32,
        total_count: u32,
    ) {
        let ver = crate::core::frame_cache::current_version();
        if self.ram_ring_version != ver {
            self.ram_ring.clear();
            self.ram_ring_version = ver;
        }
        let count = total_count.max(1) as usize;
        if self.ram_ring.len() != count {
            self.ram_ring.clear();
            for _ in 0..count {
                self.ram_ring.push((u32::MAX, None));
            }
        }

        // Match the live-view resolution logic (preview cap + device limits)
        let max_dim = self
            .device
            .limits()
            .max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (cap.max(1), ((comp.height as f32 * s) as u32).max(1))
            }
            _ => (comp.width, comp.height),
        };
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);

        for frame in from..=to {
            // Find or allocate this frame's slot (frames map to slots by order of
            // first appearance; the cursor advances monotonically).
            let slot_idx = match self.ram_ring.iter().position(|(f, _)| *f == frame) {
                Some(i) => i,
                None => match self.ram_ring.iter().position(|(_, t)| t.is_none()) {
                    Some(i) => i,
                    None => continue, // ring full — skip
                },
            };

            let size_mismatch = match self.ram_ring[slot_idx].1.as_ref() {
                None => true,
                Some((tex, _)) => tex.size().width != width || tex.size().height != height,
            };
            if size_mismatch {
                let size = wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("RAM Preview Slot"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.ram_ring[slot_idx] = (frame, Some((texture, view)));
            } else {
                self.ram_ring[slot_idx].0 = frame;
            }
            self.ram_render_idx = slot_idx;
            self.render_internal(comp, frame, exposure_ev, lut_mode, false);
        }
        self.ram_render_idx = usize::MAX;
    }

    /// Texture view for a pre-rendered frame, if present and still valid.
    pub fn ram_frame_view(&self, frame: u32) -> Option<&wgpu::TextureView> {
        if self.ram_ring_version != crate::core::frame_cache::current_version() {
            return None;
        }
        self.ram_ring
            .iter()
            .find(|(f, t)| *f == frame && t.is_some())
            .and_then(|(_, t)| t.as_ref().map(|(_, v)| v))
    }

    /// Drops all cached RAM preview frames (e.g. when playback stops).
    pub fn clear_ram_preview(&mut self) {
        self.ram_ring.clear();
        self.ram_ring_version = 0;
    }
}
