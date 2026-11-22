use super::*;

impl GpuComputeContext {
    pub fn gaussian_blur(&self, pixels: &mut [u8], width: u32, height: u32, radius: u32) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 {
            return false;
        }
        self.gaussian_blur_inner(pixels, width, height, radius)
            .is_some()
    }

    fn gaussian_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        radius_in: u32,
    ) -> Option<()> {
        let _t = std::time::Instant::now();
        let radius = radius_in
            .min(MAX_BLUR_RADIUS)
            .min(width.max(1) / 2)
            .min(height.max(1) / 2);
        if radius == 0 {
            return Some(()); // nothing to do; caller's result already correct
        }
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }

        // (Re)allocate cached buffers when size changes
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        // Gaussian kernel (sigma = radius / 2, matching CPU reference feel)
        let sigma = (radius as f32 / 2.0).max(0.5);
        let mut weights = Vec::with_capacity((2 * radius + 1) as usize);
        for i in 0..=(2 * radius) {
            let x = i as f32 - radius as f32;
            weights.push((-0.5 * (x / sigma) * (x / sigma)).exp());
        }
        let ksum: f32 = weights.iter().sum();
        for wk in weights.iter_mut() {
            *wk /= ksum;
        }

        // Upload input + kernel + params
        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner
            .queue
            .write_buffer(&bufs.kernel, 0, bytemuck::cast_slice(&weights));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius,
                mode: 0,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.5,
                param_f4: 8.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            }),
        );

        let bg_h = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.mid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_h"),
        });
        let bg_v = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.mid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_v"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blur_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            // Horizontal: src -> mid
            pass.set_bind_group(0, &bg_h, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
            // Vertical: mid -> dst (flip direction flag)
            inner.queue.write_buffer(
                &bufs.params,
                0,
                bytemuck::bytes_of(&ParamsUniform {
                    width,
                    height,
                    radius,
                    mode: 2,
                    angle: 0.0,
                    brightness: 0.0,
                    contrast: 1.0,
                    saturation: 1.0,
                    hue_shift: 0.0,
                    param_f3: 0.5,
                    param_f4: 8.0,
                    param_f5: 0.0,
                    param_f6: 0.0,
                    param_f7: 0.0,
                    param_f8: 0.0,
                    _pad: 0.0,
                }),
            );
            pass.set_bind_group(0, &bg_v, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

        // Block until done, then read back
        let (tx, rx) = std::sync::mpsc::channel();
        bufs.staging_out
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        bufs.staging_out.mark_mapped();
        inner.device.poll(wgpu::Maintain::Wait);
        let map_ok = rx.recv().ok().and_then(|r| r.ok()).is_some();
        if !map_ok {
            bufs.staging_out.unmap_tracking();
            return None;
        }
        {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() != pixels.len() {
                drop(data);
                bufs.staging_out.unmap_tracking();
                return None;
            }
            pixels.copy_from_slice(&data);
        }
        bufs.staging_out.unmap_tracking();
        record_timing(_t);
        Some(())
    }
}
