use super::*;

impl WgpuRenderer {
    /// Uploads a video frame PNG as a GPU texture, cached by (layer, dir, frame).
    /// Pixels come from the shared CPU image cache, so playback reuses decoded data.
    pub(super) fn get_or_create_video_frame_texture(
        &self,
        layer_id: &str,
        frames_dir: &str,
        frame_idx: u32,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let frame_path = crate::core::video_import::frame_path_in_dir(frames_dir, frame_idx);
        let source_revision = media_file_revision(&frame_path);
        let key: VideoFrameKey = (
            layer_id.to_string(),
            format!("{frames_dir}:{source_revision}"),
            frame_idx,
        );
        // Re-imports write into the same frames_dir; the version check ensures
        // stale pixels from a previous project state are never displayed.
        if self.video_cache_version.get() != version {
            self.video_frame_cache.borrow_mut().clear();
            self.video_cache_version.set(version);
        }
        if let Some((bg, width, height)) = self.video_frame_cache.borrow_mut().get(&key) {
            return Some((width, height, bg));
        }
        let frame_path = frame_path.to_string_lossy().to_string();
        let (tw, th, pixels) = {
            use crate::core::image_cache::with_image_cache;
            with_image_cache(|cache| {
                cache
                    .load_image(&frame_path)
                    .map(|img| (img.width, img.height, img.pixels.clone()))
            })
        }?;

        let size = wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Video Frame Texture"),
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
            label: Some("video_frame_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        {
            self.video_frame_cache.borrow_mut().insert(
                key,
                std::sync::Arc::new(texture),
                bind_group.clone(),
                tw,
                th,
            );
        }
        Some((tw, th, bind_group))
    }

    /// Upload a still image through the same shared decode cache as video
    /// frames. The returned dimensions keep the GPU quad in sync with the
    /// source pixel aspect instead of rendering a 1×1 placeholder.
    pub(super) fn get_or_create_image_texture(
        &self,
        layer_id: &str,
        path: &str,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let source_revision = media_file_revision(std::path::Path::new(path));
        let key: VideoFrameKey = (
            format!("image:{layer_id}"),
            format!("{path}:{source_revision}"),
            0,
        );
        if self.video_cache_version.get() != version {
            self.video_frame_cache.borrow_mut().clear();
            self.video_cache_version.set(version);
        }
        if let Some((bg, width, height)) = self.video_frame_cache.borrow_mut().get(&key) {
            return Some((width, height, bg));
        }

        let (tw, th, pixels) = {
            use crate::core::image_cache::with_image_cache;
            with_image_cache(|cache| {
                cache
                    .load_image(path)
                    .map(|img| (img.width, img.height, img.pixels.clone()))
            })?
        };
        let size = wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Layer Texture"),
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
            label: Some("image_layer_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.video_frame_cache.borrow_mut().insert(
            key,
            std::sync::Arc::new(texture),
            bind_group.clone(),
            tw,
            th,
        );
        Some((tw, th, bind_group))
    }

    /// Render a nested composition through the same software reference path
    /// used by export, then cache its pixels as a GPU texture for the parent
    /// layer. The GPU shader can then apply the parent PreComp transform,
    /// effects, masks, mattes, and blend mode without treating the layer as a
    /// flat placeholder.
    pub(super) fn get_or_create_precomp_frame_texture(
        &self,
        comp: &Composition,
        comp_id: &str,
        frame_idx: u32,
        width: u32,
        height: u32,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let sub_comp = comp.find_sub_comp(comp_id)?;
        let revision = composition_cache_key(sub_comp);
        let media_revision = composition_media_revision(sub_comp, frame_idx);
        let key: VideoFrameKey = (
            format!("precomp:{comp_id}"),
            format!("{revision}:{media_revision}:{width}x{height}"),
            frame_idx,
        );

        if self.video_cache_version.get() != version {
            self.video_frame_cache.borrow_mut().clear();
            self.video_cache_version.set(version);
        }
        if let Some((bg, cached_width, cached_height)) =
            self.video_frame_cache.borrow_mut().get(&key)
        {
            return Some((cached_width, cached_height, bg));
        }

        let pixels = crate::core::software_renderer::render_precomp_layers(
            comp, sub_comp, frame_idx, width, height,
        );
        if pixels.len() != crate::core::software_renderer::rgba_buffer_size(width, height)? {
            return None;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("PreComp Frame Texture"),
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
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(height),
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
            label: Some("precomp_frame_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.video_frame_cache.borrow_mut().insert(
            key,
            std::sync::Arc::new(texture),
            bind_group.clone(),
            width,
            height,
        );
        Some((width, height, bind_group))
    }
}
