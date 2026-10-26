use crate::core::{renderer::WgpuRenderer, timeline::Composition};
use half::f16;
use std::sync::Arc;

pub struct HeadlessGpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    renderer: WgpuRenderer,
}

impl HeadlessGpuRenderer {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no compatible GPU adapter is available".to_owned())?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Kagari headless renderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|error| format!("GPU device creation failed: {error}"))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let renderer = WgpuRenderer::new(device.clone(), queue.clone());
        Ok(Self {
            device,
            queue,
            renderer,
        })
    }

    pub fn render_rgba8(
        &mut self,
        composition: &Composition,
        frame: u32,
        exposure_ev: f32,
        lut_mode: u32,
    ) -> Result<Vec<u8>, String> {
        let width = composition.width.max(1);
        let height = composition.height.max(1);
        let max_dimension = self
            .device
            .limits()
            .max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        if width > max_dimension || height > max_dimension {
            return Err(format!(
                "composition dimensions {width}x{height} exceed this GPU's render limit of {max_dimension}px"
            ));
        }
        let output_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "RGBA readback size overflows addressable memory".to_owned())?;
        self.renderer
            .render(composition, frame, exposure_ev, lut_mode);
        let texture = self
            .renderer
            .target_texture
            .as_ref()
            .ok_or_else(|| "GPU render target was not created".to_owned())?;
        let target_size = self.renderer.target_size;
        if target_size != (width, height) {
            return Err(format!(
                "GPU render target dimensions {}x{} do not match requested output {width}x{height}",
                target_size.0, target_size.1
            ));
        }
        let bytes_per_pixel = 8u32;
        let unpadded_bytes_per_row = width
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| "render row is too wide".to_owned())?;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(alignment)
            .checked_mul(alignment)
            .ok_or_else(|| "aligned GPU readback row is too wide".to_owned())?;
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Kagari GPU frame readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Kagari GPU frame readback encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = output.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|error| format!("GPU readback callback failed: {error}"))?
            .map_err(|error| format!("GPU frame readback failed: {error}"))?;

        let mapped = slice.get_mapped_range();
        let mut rgba = vec![0; output_len];
        for y in 0..height as usize {
            let source_row = &mapped[y * padded_bytes_per_row as usize
                ..y * padded_bytes_per_row as usize + unpadded_bytes_per_row as usize];
            for x in 0..width as usize {
                let source = x * 8;
                let target = (y * width as usize + x) * 4;
                for channel in 0..4 {
                    let offset = source + channel * 2;
                    let value =
                        f16::from_le_bytes([source_row[offset], source_row[offset + 1]]).to_f32();
                    rgba[target + channel] = if value.is_finite() {
                        (value.clamp(0.0, 1.0) * 255.0).round() as u8
                    } else {
                        0
                    };
                }
            }
        }
        drop(mapped);
        output.unmap();
        Ok(rgba)
    }
}
