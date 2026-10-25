use super::*;

impl WgpuRenderer {
    pub(super) fn preview_proxy_path(&self, layer: &Layer, sequence_frame: u32) -> Option<String> {
        if !self.preview_proxies || !layer.proxy.enabled {
            return None;
        }
        let raw_path = layer.proxy.proxy_path.as_deref()?;
        let path = std::path::Path::new(raw_path);
        if path.is_file() {
            return Some(raw_path.to_string());
        }
        if path.is_dir() {
            let frame = if matches!(&layer.layer_type, LayerType::Video { .. }) {
                crate::core::video_import::frame_path_in_dir(raw_path, sequence_frame)
            } else {
                crate::core::video_import::frame_path_in_dir(raw_path, 0)
            };
            if frame.is_file() {
                return Some(frame.to_string_lossy().into_owned());
            }
        }
        None
    }

    /// Rebuild the combined mask+shadow+matte bind group (group 3) from
    /// the current mask, shadow, and dummy matte views.
    pub(super) fn rebuild_combined_bind_group(&self) -> wgpu::BindGroup {
        let mask_view = self.mask_view.as_ref().unwrap_or(&self.dummy_mask_view);
        let shadow_view = self.shadow_view.as_ref().unwrap_or(&self.dummy_shadow_view);
        let matte_view = &self.dummy_matte_view;
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.combined_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(mask_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(matte_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some("combined_bind_group"),
        })
    }

    /// Creates/replaces the shared layer-mask texture when its size changed.
    pub(super) fn ensure_mask_texture(&mut self, width: u32, height: u32) {
        if self.mask_size == (width, height) && self.mask_texture.is_some() {
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Layer Mask Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.mask_texture = Some(texture);
        self.mask_view = Some(view);
        self.mask_size = (width, height);
        self.combined_bind_group = self.rebuild_combined_bind_group();
    }

    /// Prepares/resizes the offscreen target texture if needed.
    /// Returns true if the texture was recreated.
    /// Rasterizes text on CPU and uploads it as a GPU texture, cached by (layer_id, text, font_size).
    /// Returns (width, height, bind_group) for the text texture, or None if rasterization fails.
    pub(super) fn get_or_create_text_texture(
        &self,
        layer_id: &str,
        params: &TextRasterParams,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let (text, font_size, color, font_family, tracking, leading, align) = (
            params.text.as_str(),
            params.font_size,
            params.color,
            params.font_family.as_str(),
            params.tracking,
            params.leading,
            params.align,
        );
        let (stroke_color, stroke_width) = (params.stroke_color, params.stroke_width);
        // Floats hashed via bit patterns (f32 is not Hash). Include every
        // visual input so changing a text property cannot reuse stale pixels.
        let key = TextTextureKey {
            layer_id: layer_id.to_string(),
            text: text.to_string(),
            font_size,
            color: [
                color[0].to_bits(),
                color[1].to_bits(),
                color[2].to_bits(),
                color[3].to_bits(),
            ],
            font_family: font_family.to_string(),
            tracking: tracking.to_bits(),
            leading: leading.to_bits(),
            box_width: params.box_width.to_bits(),
            box_height: params.box_height.to_bits(),
            align,
            stroke_color: [
                stroke_color[0].to_bits(),
                stroke_color[1].to_bits(),
                stroke_color[2].to_bits(),
                stroke_color[3].to_bits(),
            ],
            stroke_width: stroke_width.to_bits(),
            faux_bold: params.faux_style.bold,
            faux_italic: params.faux_style.italic,
            all_caps: params.faux_style.all_caps,
            small_caps: params.faux_style.small_caps,
        };
        // Cached: return stored dimensions — no CPU rasterization on hits
        if let Some(bind_group) = self
            .text_texture_cache
            .borrow()
            .get(&key)
            .map(|(_, bg, w, h)| (bg.clone(), *w, *h))
        {
            let (bind_group, w, h) = bind_group;
            return Some((w, h, bind_group));
        }

        let alignment = match align {
            1 => crate::core::text_layout::TextAlign::Center,
            2 => crate::core::text_layout::TextAlign::Right,
            _ => crate::core::text_layout::TextAlign::Left,
        };
        let rasterized = crate::core::font_rasterizer::with_font_rasterizer(|r| {
            let family = r.resolve_family(font_family);
            r.rasterize_text_formatted_styled(
                &family,
                text,
                font_size as f32,
                color,
                tracking,
                leading,
                params.box_width,
                alignment,
                params.faux_style,
            )
        })?;
        let rasterized = crate::core::font_rasterizer::clip_text_bitmap_height(
            rasterized,
            params.box_height,
        );
        if rasterized.0 == 0 || rasterized.1 == 0 || rasterized.2.is_empty() {
            return None;
        }
        // Bake stroke (if any) into the bitmap with padding
        let (tw, th, pixels) = if stroke_width > 0.1 {
            bake_text_stroke(
                &rasterized.2,
                rasterized.0,
                rasterized.1,
                stroke_color,
                stroke_width,
            )
        } else {
            rasterized
        };

        let size = wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Layer Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba8_to_rgba16f_bytes(&pixels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tw * 8),
                rows_per_image: Some(th),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            ],
            label: Some("text_texture_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        {
            let mut cache = self.text_texture_cache.borrow_mut();
            cache.insert(key, (texture, bind_group.clone(), tw, th));
            // FIFO eviction: HashMap order is arbitrary but bounded memory
            // matters more than exact LRU here (entries re-rasterize cheaply).
            while cache.len() > MAX_TEXT_TEXTURES {
                if let Some(oldest) = cache.keys().next().cloned() {
                    cache.remove(&oldest);
                } else {
                    break;
                }
            }
        }
        Some((tw, th, bind_group))
    }

    pub fn ensure_target_size(&mut self, width: u32, height: u32) -> bool {
        if self.target_size == (width, height) && self.target_texture.is_some() {
            return false;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Offscreen Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let snap_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Snapshot Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let snap_view = snap_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ping_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Multi-Pass Ping Intermediate"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let ping_view = ping_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let pong_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Multi-Pass Pong Intermediate"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pong_view = pong_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.target_texture = Some(texture);
        self.target_view = Some(view);
        self.snapshot_texture = Some(snap_texture);
        self.snapshot_view = Some(snap_view);
        self.ping_texture = Some(ping_texture);
        self.ping_view = Some(ping_view);
        self.pong_texture = Some(pong_texture);
        self.pong_view = Some(pong_view);
        self.target_size = (width, height);
        true
    }

    pub(super) fn ensure_layer_buffer_capacity(&mut self, layer_count: usize) {
        if layer_count <= MAX_GPU_LAYERS {
            return;
        }
        let size = (LAYER_UNIFORM_STRIDE * layer_count) as u64;
        self.layer_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Layer Buffer (expanded)"),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.layer_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.layer_buffer,
                    offset: 0,
                    size: Some(LAYER_UNIFORM_SIZE),
                }),
            }],
            label: Some("layer_bind_group_expanded"),
        });
    }
}
