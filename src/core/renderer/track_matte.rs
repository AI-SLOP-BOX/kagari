use super::*;

pub(super) struct TrackMatteResources {
    pub(super) bind_groups: Vec<Option<wgpu::BindGroup>>,
    _owned_textures: Vec<Option<(wgpu::Texture, wgpu::TextureView)>>,
}

impl WgpuRenderer {
    pub(super) fn prepare_track_mattes(
        &self,
        comp: &Composition,
        frame: u32,
        width: u32,
        height: u32,
        active_layers: &[&Layer],
        comp_indices: &[usize],
    ) -> TrackMatteResources {
        let mut matte_bind_groups = Vec::with_capacity(active_layers.len());
        let mut matte_textures_owned = Vec::with_capacity(active_layers.len());
        for (layer, &comp_idx) in active_layers.iter().zip(comp_indices) {
            if matches!(layer.track_matte, TrackMatteMode::None) {
                matte_bind_groups.push(None);
                matte_textures_owned.push(None);
                continue;
            }
            let matte_source_idx = if let Some(target_id) = &layer.track_matte_layer_id {
                comp.layers.iter().position(|l| l.id == *target_id)
            } else if comp_idx > 0 {
                Some(comp_idx - 1)
            } else {
                None
            };
            let Some(matte_source_idx) = matte_source_idx else {
                matte_bind_groups.push(None);
                matte_textures_owned.push(None);
                continue;
            };
            let matte_source = &comp.layers[matte_source_idx];
            if !matte_source.is_active(frame) || !matte_source.visible {
                matte_bind_groups.push(None);
                matte_textures_owned.push(None);
                continue;
            }
            let matte_pixels = crate::core::software_renderer::render_single_layer_pixels(
                comp,
                matte_source_idx,
                frame,
                width,
                height,
            );
            let matte_rgba = match layer.track_matte {
                TrackMatteMode::AlphaMatte | TrackMatteMode::AlphaMatteInverted => matte_pixels,
                TrackMatteMode::LumaMatte | TrackMatteMode::LumaMatteInverted => {
                    let mut luminance = Vec::with_capacity(matte_pixels.len());
                    for chunk in matte_pixels.as_chunks::<4>().0 {
                        let value = (0.2126 * chunk[0] as f32
                            + 0.7152 * chunk[1] as f32
                            + 0.0722 * chunk[2] as f32) as u8;
                        luminance.extend_from_slice(&[value, value, value, chunk[3]]);
                    }
                    luminance
                }
                _ => unreachable!(),
            };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Track Matte Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
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
                &rgba8_to_rgba16f_bytes(&matte_rgba),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 8),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mask_view = self.mask_view.as_ref().unwrap_or(&self.dummy_mask_view);
            let shadow_view = self.shadow_view.as_ref().unwrap_or(&self.dummy_shadow_view);
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: Some("Combined Matte Bind Group"),
            });
            matte_textures_owned.push(Some((texture, view)));
            matte_bind_groups.push(Some(bind_group));
        }
        TrackMatteResources {
            bind_groups: matte_bind_groups,
            _owned_textures: matte_textures_owned,
        }
    }
}
