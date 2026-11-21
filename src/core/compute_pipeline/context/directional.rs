use super::*;

impl GpuComputeContext {
    pub fn directional_blur(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        length_px: u32,
        angle_deg: f32,
    ) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 || length_px < 2 {
            return false;
        }
        self.directional_blur_inner(pixels, width, height, length_px, angle_deg)
            .is_some()
    }

    fn directional_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        taps: u32,
        angle_deg: f32,
    ) -> Option<()> {
        use std::time::Instant;
        let t = Instant::now();
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }

        // Reuse the same cached buffer set (kernel unused in this mode)
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius: taps.min(256),
                mode: 1,
                angle: angle_deg.to_radians(),
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

        let bg = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_dir"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("dir_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("dir_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

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
        record_timing(t);
        Some(())
    }

    /// Radial zoom blur: `taps` samples along the ray from each pixel toward
    /// the frame center (zoom-blur look). Single dispatch.
    pub fn radial_blur(&self, pixels: &mut [u8], width: u32, height: u32, taps: u32) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 || taps < 2 {
            return false;
        }
        self.radial_blur_inner(pixels, width, height, taps)
            .is_some()
    }

    fn radial_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        taps: u32,
    ) -> Option<()> {
        use std::time::Instant;
        let t = Instant::now();
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius: taps.min(256),
                mode: 3,
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

        let bg = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_radial"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("radial_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("radial_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

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
        record_timing(t);
        Some(())
    }
}
