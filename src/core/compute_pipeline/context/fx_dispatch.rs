use super::*;

impl GpuComputeContext {
    pub(crate) fn dispatch_fx(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        params: ParamsUniform,
        label: &str,
    ) -> bool {
        self.dispatch_fx_inner(pixels, width, height, params, label)
            .is_some()
    }

    fn dispatch_fx_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        params: ParamsUniform,
        label: &str,
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
        inner
            .queue
            .write_buffer(&bufs.params, 0, bytemuck::bytes_of(&params));
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
            label: Some(label),
        });
        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);
        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fx_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fx_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(b) = inner.bufs.as_ref() {
            b.staging_out
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
            b.staging_out.mark_mapped();
        }
        inner.device.poll(wgpu::Maintain::Wait);
        let _ = rx.recv();
        if let Some(bufs) = inner.bufs.as_ref() {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() == pixels.len() {
                pixels.copy_from_slice(&data);
            }
            bufs.staging_out.unmap_tracking();
        }
        record_timing(t);
        Some(())
    }
}
