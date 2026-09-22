use std::sync::Arc;

use half::f16;
use kagari_vfx::core::renderer::WgpuRenderer;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Effect, EffectType, Layer, LayerType};
use kagari_vfx::core::{effect_plugin::gpu_preview_effect_supported, property::Animatable};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

fn constant<T: Clone>(value: T) -> Animatable<T> {
    Animatable::new_constant(value)
}

fn request_gpu() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("gpu-preview-parity-test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 6,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .ok()?;
    Some((Arc::new(device), Arc::new(queue)))
}

fn composition_with_effect(id: &str, effect_type: EffectType) -> Composition {
    let mut comp = Composition::new(id.into(), id.into(), WIDTH, HEIGHT, 30, 1);
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "parity-layer".into(),
        "Parity Layer".into(),
        LayerType::Solid {
            color: [0.21, 0.41, 0.63, 1.0],
        },
        1,
    );
    layer.transform.position = Animatable::new_constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point =
        Animatable::new_constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.effects.push(Effect {
        id: format!("effect-{id}"),
        name: id.into(),
        effect_type,
        enabled: true,
    });
    comp.layers.push(layer);
    comp
}

fn gpu_rgba16f_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut WgpuRenderer,
    comp: &Composition,
) -> Vec<[f32; 4]> {
    renderer.render(comp, 0, 0.0, 0);
    let texture = renderer
        .target_texture
        .as_ref()
        .expect("render should allocate the offscreen target");
    let unpadded_bytes_per_row = WIDTH * 8;
    let bytes_per_row = unpadded_bytes_per_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpu-preview-parity-readback"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpu-preview-parity-copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .expect("readback mapping callback should run")
        .expect("GPU readback should map");
    let mapped = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT) as usize);
    for y in 0..HEIGHT as usize {
        for x in 0..WIDTH as usize {
            let offset = y * bytes_per_row as usize + x * 8;
            let channel = |index: usize| {
                let start = offset + index * 2;
                f16::from_le_bytes([mapped[start], mapped[start + 1]]).to_f32()
            };
            pixels.push([channel(0), channel(1), channel(2), channel(3)]);
        }
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

#[test]
fn allowlisted_gpu_effects_match_the_software_reference_at_representative_pixels() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU preview parity test: no adapter available");
        return;
    };
    let cases = [
        (
            "Color Tint",
            EffectType::ColorTint {
                color: constant([0.91, 0.34, 0.73, 1.0]),
                intensity: constant(63.7),
            },
        ),
        (
            "Levels",
            EffectType::Levels {
                input_black: constant(0.05),
                input_white: constant(0.95),
                gamma: constant(1.3),
                output_black: constant(0.02),
                output_white: constant(0.98),
            },
        ),
        (
            "Lens Flare",
            EffectType::LensFlare {
                enabled: constant(1.0),
                position_x: constant(0.5),
                position_y: constant(0.5),
                intensity: constant(0.7),
                threshold: constant(1.2),
                color: constant([1.0, 0.6, 0.2, 1.0]),
                link_to_light: None,
            },
        ),
        (
            "Invert",
            EffectType::Invert {
                invert_alpha: false,
            },
        ),
    ];
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());

    for (name, effect_type) in cases {
        assert!(
            gpu_preview_effect_supported(&effect_type),
            "{name} is included in the GPU preview allowlist"
        );
        let comp = composition_with_effect(name, effect_type);
        let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
        let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
        let sample_coords = [(10, 10), (20, 20), (30, 25)];
        let mut max_error = 0.0f32;
        for (x, y) in sample_coords {
            let index = (y * WIDTH + x) as usize;
            for channel in 0..4 {
                let expected = cpu[index * 4 + channel] as f32;
                let actual = gpu[index][channel] * 255.0;
                max_error = max_error.max((expected - actual).abs());
            }
        }
        assert!(
            max_error <= 2.0,
            "{name} GPU/software mismatch: max sampled error {max_error:.3} 8-bit values"
        );
    }
}
