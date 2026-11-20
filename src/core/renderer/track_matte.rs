use super::*;

pub(super) struct TrackMatteResources {
    pub(super) bind_groups: Vec<Option<wgpu::BindGroup>>,
    _owned_textures: Vec<Option<(wgpu::Texture, wgpu::TextureView)>>,
}

impl WgpuRenderer {
    pub(super) fn prepare_track_mattes(
        &mut self,
        comp: &Composition,
        frame: u32,
        width: u32,
        height: u32,
        active_layers: &[&Layer],
        comp_indices: &[usize],
    ) -> Option<TrackMatteResources> {
        let mut matte_bind_groups = Vec::with_capacity(active_layers.len());
        let mut matte_textures_owned = Vec::with_capacity(active_layers.len());
        for (layer, &comp_idx) in active_layers.iter().zip(comp_indices) {
            let set_matte_info = if matches!(layer.track_matte, TrackMatteMode::None)
                && layer.effects_enabled
            {
                layer.effects.iter().rev().find_map(|eff| {
                    if !eff.enabled {
                        return None;
                    }
                    if let crate::core::timeline::EffectType::SetMatte {
                        source_layer_idx,
                        source_channel,
                        ..
                    } = eff.effect_type
                    {
                        Some((source_layer_idx, source_channel))
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            let map_source_idx = if matches!(layer.track_matte, TrackMatteMode::None)
                && set_matte_info.is_none()
                && layer.effects_enabled
            {
                layer.effects.iter().rev().find_map(|eff| {
                    if !eff.enabled {
                        return None;
                    }
                    match &eff.effect_type {
                        crate::core::timeline::EffectType::DisplacementMap {
                            source_layer, ..
                        }
                        | crate::core::timeline::EffectType::CompoundBlur {
                            source_layer, ..
                        } => {
                            let idx = source_layer.evaluate(frame).round() as isize;
                            if idx >= 0 && (idx as usize) < comp.layers.len() {
                                Some(idx as usize)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                })
            } else {
                None
            };

            let spectrum_info = if matches!(layer.track_matte, TrackMatteMode::None)
                && set_matte_info.is_none()
                && map_source_idx.is_none()
                && layer.effects_enabled
            {
                layer.effects.iter().rev().find_map(|eff| {
                    if !eff.enabled {
                        return None;
                    }
                    if let crate::core::timeline::EffectType::AudioSpectrum {
                        enabled,
                        bands,
                        start_frequency,
                        end_frequency,
                        ..
                    } = &eff.effect_type
                    {
                        if enabled.evaluate(frame) > 0.5 {
                            Some((
                                (bands.evaluate(frame).round() as usize).clamp(4, 64),
                                *start_frequency,
                                *end_frequency,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            if let Some((band_count, start_freq, end_freq)) = spectrum_info {
                let mut spec_bytes = Vec::with_capacity(band_count * 4);
                for b in 0..band_count {
                    let amp = crate::core::expression_engine::get_audio_spectrum_band_for_range(
                        b,
                        band_count,
                        start_freq,
                        end_freq,
                    )
                    .clamp(0.0, 1.0);
                    let v = (amp * 255.0).round() as u8;
                    spec_bytes.extend_from_slice(&[v, v, v, 255]);
                }
                let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("AudioSpectrum FFT Texture"),
                    size: wgpu::Extent3d {
                        width: band_count as u32,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                if let Some(upload_bytes) = rgba8_to_rgba16f_bytes(&spec_bytes) {
                    self.queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &upload_bytes,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(band_count as u32 * 8),
                            rows_per_image: Some(1),
                        },
                        wgpu::Extent3d {
                            width: band_count as u32,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                    );
                    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                    let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.combined_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&self.dummy_mask_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&self.dummy_shadow_view),
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
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                        ],
                        label: Some("audio_spectrum_fft_bg"),
                    });
                    matte_bind_groups.push(Some(bg));
                    matte_textures_owned.push(Some((tex, view)));
                    continue;
                }
            }

            if matches!(layer.track_matte, TrackMatteMode::None)
                && set_matte_info.is_none()
                && map_source_idx.is_none()
            {
                matte_bind_groups.push(None);
                matte_textures_owned.push(None);
                continue;
            }
            let matte_source_idx = if let Some((idx, _)) = set_matte_info {
                (idx < comp.layers.len()).then_some(idx)
            } else if let Some(idx) = map_source_idx {
                Some(idx)
            } else if let Some(target_id) = &layer.track_matte_layer_id {
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
            let mut matte_comp = Composition::new(
                format!("{}_matte_{}", comp.id, matte_source.id),
                "GPU Matte Pass".to_owned(),
                comp.width,
                comp.height,
                comp.fps,
                comp.duration_frames,
            );
            matte_comp.background_color = [0.0, 0.0, 0.0, 0.0];
            matte_comp.active_camera = comp.active_camera.clone();
            let mut matte_layer = matte_source.clone();
            matte_layer.visible = true;
            matte_layer.track_matte = TrackMatteMode::None;
            matte_layer.effects.retain(|e| {
                !matches!(
                    e.effect_type,
                    crate::core::timeline::EffectType::SetMatte { .. }
                        | crate::core::timeline::EffectType::DisplacementMap { .. }
                        | crate::core::timeline::EffectType::CompoundBlur { .. }
                )
            });
            matte_comp.layers.push(matte_layer);
            let matte_pixels = self.render_frame_to_rgba8_gpu(
                &matte_comp,
                frame,
                width,
                height,
                0.0,
                0,
            )?;
            let matte_rgba = if let Some((_, channel)) = set_matte_info {
                use crate::core::set_matte::MatteSourceChannel;
                let mut out = Vec::with_capacity(matte_pixels.len());
                for chunk in matte_pixels.as_chunks::<4>().0 {
                    let val = match channel {
                        MatteSourceChannel::Alpha => chunk[3],
                        MatteSourceChannel::Red => chunk[0],
                        MatteSourceChannel::Green => chunk[1],
                        MatteSourceChannel::Blue => chunk[2],
                        MatteSourceChannel::Luminance => (0.2126 * chunk[0] as f32
                            + 0.7152 * chunk[1] as f32
                            + 0.0722 * chunk[2] as f32)
                            as u8,
                        MatteSourceChannel::Lightness => {
                            let max_c = chunk[0].max(chunk[1]).max(chunk[2]) as u16;
                            let min_c = chunk[0].min(chunk[1]).min(chunk[2]) as u16;
                            ((max_c + min_c) / 2) as u8
                        }
                    };
                    out.extend_from_slice(&[val, val, val, val]);
                }
                out
            } else if map_source_idx.is_some() {
                matte_pixels
            } else {
                match layer.track_matte {
                    TrackMatteMode::AlphaMatte | TrackMatteMode::AlphaMatteInverted => matte_pixels,
                    TrackMatteMode::LumaMatte | TrackMatteMode::LumaMatteInverted => {
                        let mut luminance = Vec::with_capacity(matte_pixels.len());
                        for chunk in matte_pixels.as_chunks::<4>().0 {
                            let value = (0.2126 * chunk[0] as f32
                                + 0.7152 * chunk[1] as f32
                                + 0.0722 * chunk[2] as f32)
                                as u8;
                            luminance.extend_from_slice(&[value, value, value, chunk[3]]);
                        }
                        luminance
                    }
                    _ => unreachable!(),
                }
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
            let upload_bytes = rgba8_to_rgba16f_bytes(&matte_rgba)?;
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
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mask_view = self.mask_view.as_ref().unwrap_or(&self.dummy_mask_view);
            let shadow_view = self.shadow_view.as_ref().unwrap_or(&self.dummy_shadow_view);
            let backdrop_view = self.backdrop_view.as_ref().unwrap_or(&self.dummy_matte_view);
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
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(backdrop_view),
                    },
                ],
                label: Some("Combined Matte Bind Group"),
            });
            matte_textures_owned.push(Some((texture, view)));
            matte_bind_groups.push(Some(bind_group));
        }
        Some(TrackMatteResources {
            bind_groups: matte_bind_groups,
            _owned_textures: matte_textures_owned,
        })
    }
}
