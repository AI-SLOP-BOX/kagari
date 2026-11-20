use super::*;

impl WgpuRenderer {
    /// Uploads a video frame PNG as a GPU texture, cached by (layer, dir, frame).
    /// Pixels come from the shared CPU image cache, so playback reuses decoded data.
    pub(super) fn get_or_create_video_frame_texture(
        &self,
        layer_id: &str,
        frames_dir: &str,
        frame_idx: u32,
        next_frame_idx: Option<u32>,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let frame_path = crate::core::video_import::frame_path_in_dir(frames_dir, frame_idx);
        let source_revision = media_file_revision(&frame_path);
        let next_frame_path = next_frame_idx.map(|index| {
            crate::core::video_import::frame_path_in_dir(frames_dir, index)
                .to_string_lossy()
                .into_owned()
        });
        let next_revision = next_frame_path
            .as_deref()
            .map(|path| media_file_revision(std::path::Path::new(path)))
            .unwrap_or_default();
        let key: VideoFrameKey = (
            layer_id.to_string(),
            format!("{frames_dir}:{source_revision}:{next_revision}"),
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
        let first = crate::core::image_cache::load_image_shared(&frame_path)?;
        let (tw, th) = (first.width, first.height);
        let pixels = first.pixels.clone();
        let next_pixels = next_frame_path
            .as_deref()
            .and_then(crate::core::image_cache::load_image_shared)
            .filter(|image| image.width == tw && image.height == th)
            .map(|image| image.pixels.clone());

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
        let upload_bytes = rgba8_to_rgba16f_bytes(&pixels)?;
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &upload_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tw * 8),
                rows_per_image: Some(th),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (next_texture, next_view) = if let Some(next_pixels) = next_pixels {
            let next_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Video Frame Blend Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let next_upload_bytes = rgba8_to_rgba16f_bytes(&next_pixels)?;
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &next_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &next_upload_bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(tw * 8),
                    rows_per_image: Some(th),
                },
                size,
            );
            let next_view = next_texture.create_view(&wgpu::TextureViewDescriptor::default());
            (Some(std::sync::Arc::new(next_texture)), Some(next_view))
        } else {
            (None, None)
        };
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        next_view.as_ref().unwrap_or(&view),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
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
                next_texture,
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

        let image = crate::core::image_cache::load_image_shared(path)?;
        let (tw, th, pixels) = (image.width, image.height, image.pixels.clone());
        let upload_bytes = rgba8_to_rgba16f_bytes(&pixels)?;
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
            &upload_bytes,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some("image_layer_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.video_frame_cache.borrow_mut().insert(
            key,
            std::sync::Arc::new(texture),
            None,
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
        self.upload_and_cache_layer_pixels(
            key,
            "PreComp Frame Texture",
            "precomp_frame_bind_group",
            &pixels,
            width,
            height,
        )
    }

    pub(super) fn get_or_create_raster_layer_texture(
        &self,
        comp: &Composition,
        layer_idx: usize,
        frame_idx: u32,
        width: u32,
        height: u32,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let layer = comp.layers.get(layer_idx)?;
        let revision = composition_cache_key(comp);
        let key: VideoFrameKey = (
            format!("raster_layer:{}", layer.id),
            format!("{revision}:{width}x{height}"),
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

        let mut single_comp = comp.clone();
        single_comp.background_color = [0.0, 0.0, 0.0, 0.0];
        single_comp.blend_linear = false;
        single_comp.working_color_space =
            crate::core::color_science::WorkingColorSpace::Rec709;
        single_comp.dither_output = false;
        for (idx, item) in single_comp.layers.iter_mut().enumerate() {
            if idx == layer_idx {
                item.visible = true;
                item.masks.clear();
                item.track_matte = crate::core::timeline::TrackMatteMode::None;
                item.track_matte_layer_id = None;
                item.blend_mode = crate::core::timeline::BlendMode::Normal;
                item.transform.opacity = crate::core::property::Animatable::new_constant(100.0);
                item.transform.opacity_expression = None;
            } else {
                item.visible = false;
            }
        }
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            &single_comp,
            frame_idx,
            width,
            height,
            0.0,
            0,
        );
        self.upload_and_cache_layer_pixels(
            key,
            "Raster Layer Texture",
            "raster_layer_bind_group",
            &pixels,
            width,
            height,
        )
    }

    fn upload_and_cache_layer_pixels(
        &self,
        key: VideoFrameKey,
        texture_label: &str,
        bind_group_label: &str,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        if pixels.len() != crate::core::software_renderer::rgba_buffer_size(width, height)? {
            return None;
        }
        let upload_bytes = rgba8_to_rgba16f_bytes(pixels)?;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(texture_label),
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
            &upload_bytes,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some(bind_group_label),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.video_frame_cache.borrow_mut().insert(
            key,
            std::sync::Arc::new(texture),
            None,
            bind_group.clone(),
            width,
            height,
        );
        Some((width, height, bind_group))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn get_or_create_freeform_bezier_texture(
        &self,
        layer_id: &str,
        points: &[[f32; 2]],
        tangents: &[([f32; 2], [f32; 2])],
        closed: bool,
        color: [f32; 4],
        stroke_color: [f32; 4],
        stroke_width: f32,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        use std::hash::{Hash, Hasher};
        if points.len() < 2 {
            return None;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        closed.hash(&mut hasher);
        for p in points {
            p[0].to_bits().hash(&mut hasher);
            p[1].to_bits().hash(&mut hasher);
        }
        for (ti, to) in tangents {
            ti[0].to_bits().hash(&mut hasher);
            ti[1].to_bits().hash(&mut hasher);
            to[0].to_bits().hash(&mut hasher);
            to[1].to_bits().hash(&mut hasher);
        }
        for c in color.iter().chain(stroke_color.iter()) {
            c.to_bits().hash(&mut hasher);
        }
        stroke_width.to_bits().hash(&mut hasher);
        let hash = hasher.finish();

        // Flatten cubic Bezier segments
        let mut poly: Vec<[f32; 2]> = Vec::new();
        let seg_count = if closed { points.len() } else { points.len() - 1 };
        const STEPS: usize = 16;
        for i in 0..seg_count {
            let p0 = points[i];
            let p3 = points[(i + 1) % points.len()];
            let t_out = tangents.get(i).map_or([0.0, 0.0], |t| t.1);
            let t_in = tangents
                .get((i + 1) % points.len())
                .map_or([0.0, 0.0], |t| t.0);
            let p1 = [p0[0] + t_out[0], p0[1] + t_out[1]];
            let p2 = [p3[0] + t_in[0], p3[1] + t_in[1]];
            for s in 0..STEPS {
                let u = s as f32 / STEPS as f32;
                let v = 1.0 - u;
                let x = v * v * v * p0[0]
                    + 3.0 * v * v * u * p1[0]
                    + 3.0 * v * u * u * p2[0]
                    + u * u * u * p3[0];
                let y = v * v * v * p0[1]
                    + 3.0 * v * v * u * p1[1]
                    + 3.0 * v * u * u * p2[1]
                    + u * u * u * p3[1];
                poly.push([x, y]);
            }
        }
        if !closed {
            if let Some(&last) = points.last() {
                poly.push(last);
            }
        }

        let mut max_x = 1.0f32;
        let mut max_y = 1.0f32;
        for p in &poly {
            max_x = max_x.max(p[0].abs());
            max_y = max_y.max(p[1].abs());
        }
        let pad = stroke_width.max(0.0) * 0.5 + 2.0;
        let half_w = (max_x + pad).ceil().clamp(2.0, 2048.0);
        let half_h = (max_y + pad).ceil().clamp(2.0, 2048.0);
        let width = (half_w * 2.0) as u32;
        let height = (half_h * 2.0) as u32;

        let key: VideoFrameKey = (
            format!("freeform_bezier:{layer_id}"),
            format!("{hash}:{width}x{height}"),
            0,
        );
        if self.video_cache_version.get() != version {
            self.video_frame_cache.borrow_mut().clear();
            self.video_cache_version.set(version);
        }
        if let Some((bg, cw, ch)) = self.video_frame_cache.borrow_mut().get(&key) {
            return Some((cw, ch, bg));
        }

        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        let fill_rgba = [
            to_u8(color[0]),
            to_u8(color[1]),
            to_u8(color[2]),
            to_u8(color[3]),
        ];
        if closed && poly.len() >= 3 && color[3] > 0.001 {
            let n = poly.len();
            let mut xs = Vec::with_capacity(n);
            for y in 0..height {
                let py = y as f32 + 0.5 - half_h;
                xs.clear();
                let mut j = n - 1;
                for i in 0..n {
                    let yi = poly[i][1];
                    let yj = poly[j][1];
                    if (yi > py) != (yj > py) {
                        let x = poly[i][0] + (py - yi) * (poly[j][0] - poly[i][0]) / (yj - yi);
                        xs.push(x + half_w);
                    }
                    j = i;
                }
                xs.sort_by(|a, b| a.total_cmp(b));
                for pair in xs.chunks_exact(2) {
                    let x0 = pair[0].floor().max(0.0) as u32;
                    let x1 = pair[1].ceil().min(width as f32) as u32;
                    for x in x0..x1 {
                        let idx = ((y * width + x) * 4) as usize;
                        pixels[idx..idx + 4].copy_from_slice(&fill_rgba);
                    }
                }
            }
        }
        if stroke_width > 0.01 && stroke_color[3] > 0.001 && poly.len() >= 2 {
            let half_sw = stroke_width * 0.5;
            let segs = if closed { poly.len() } else { poly.len() - 1 };
            for y in 0..height {
                let py = y as f32 + 0.5 - half_h;
                for x in 0..width {
                    let px = x as f32 + 0.5 - half_w;
                    let mut min_d2 = f32::MAX;
                    for i in 0..segs {
                        let a = poly[i];
                        let b = poly[(i + 1) % poly.len()];
                        let ab = [b[0] - a[0], b[1] - a[1]];
                        let ap = [px - a[0], py - a[1]];
                        let len2 = ab[0] * ab[0] + ab[1] * ab[1];
                        let t = if len2 > 1e-6 {
                            ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let dx = px - (a[0] + ab[0] * t);
                        let dy = py - (a[1] + ab[1] * t);
                        min_d2 = min_d2.min(dx * dx + dy * dy);
                    }
                    let dist = min_d2.sqrt();
                    let cov = (half_sw + 0.5 - dist).clamp(0.0, 1.0) * stroke_color[3];
                    if cov > 0.001 {
                        let idx = ((y * width + x) * 4) as usize;
                        let dst_a = pixels[idx + 3] as f32 / 255.0;
                        let out_a = cov + dst_a * (1.0 - cov);
                        if out_a > 0.0001 {
                            for ch in 0..3 {
                                let d_c = pixels[idx + ch] as f32 / 255.0;
                                let s_c = stroke_color[ch];
                                pixels[idx + ch] =
                                    (((s_c * cov + d_c * dst_a * (1.0 - cov)) / out_a) * 255.0)
                                        .round() as u8;
                            }
                            pixels[idx + 3] = (out_a * 255.0).round() as u8;
                        }
                    }
                }
            }
        }
        self.upload_and_cache_layer_pixels(
            key,
            "FreeformBezier Texture",
            "freeform_bezier_bind_group",
            &pixels,
            width,
            height,
        )
    }
}
