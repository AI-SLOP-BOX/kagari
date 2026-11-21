use super::*;

#[test]
fn mask_stays_in_composition_space_on_transformed_layer() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping transformed-mask parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-transformed-mask-parity-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([54, 104, 160, 255]))
        .save(&image_path)
        .expect("transformed mask fixture should be writable");

    let mut comp = Composition::new(
        "transformed-mask".into(),
        "Parity".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "transformed-mask-layer".into(),
        "Transformed Mask".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.transform.position = constant([32.0, 32.0]);
    layer.transform.anchor_point = constant([32.0, 32.0]);
    layer.transform.scale = constant([250.0, 250.0]);
    layer.transform.rotation = constant(13.0);
    let mut mask = kagari_vfx::core::mask::Mask::new_rect(
        "parity-mask".into(),
        "Parity Mask".into(),
        40.0,
        12.0,
        12.0,
        40.0,
    );
    mask.feather = constant(0.0);
    mask.opacity = constant(73.0);
    layer.masks.push(mask);
    let mut curved_mask = kagari_vfx::core::mask::Mask::new_ellipse(
        "parity-intersection".into(),
        "Curved Multi-mask".into(),
        35.0,
        31.0,
        18.0,
        13.0,
    );
    curved_mask.mode = kagari_vfx::core::mask::MaskMode::Intersect;
    curved_mask.feather = constant(1.75);
    curved_mask.opacity = constant(86.0);
    layer.masks.push(curved_mask);
    comp.layers.push(layer);

    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let modes = [
        kagari_vfx::core::mask::MaskMode::Add,
        kagari_vfx::core::mask::MaskMode::Subtract,
        kagari_vfx::core::mask::MaskMode::Intersect,
        kagari_vfx::core::mask::MaskMode::Lighten,
        kagari_vfx::core::mask::MaskMode::Darken,
        kagari_vfx::core::mask::MaskMode::Difference,
    ];
    for first_mode in modes {
        comp.layers[0].masks[0].mode = first_mode;
        for mode in modes {
            comp.layers[0].masks[1].mode = mode;
            for feather in [2.5, 128.0, 5_000.0] {
                comp.layers[0].masks[0]
                    .feather
                    .set_value_at_frame(0, feather);
                // Match the edit revision the Studio advances after each property edit.
                kagari_vfx::core::frame_cache::bump_version();
                let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
                let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
                let mut max_error = 0.0f32;
                let mut worst_pixel = (0usize, 0usize, 0usize);
                let mut worst_values = (0.0f32, 0.0f32);
                for (index, actual_pixel) in gpu.iter().enumerate() {
                    for channel in 0..4 {
                        let expected = cpu[index * 4 + channel] as f32;
                        let actual = actual_pixel[channel] * 255.0;
                        let error = (expected - actual).abs();
                        if error > max_error {
                            max_error = error;
                            worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
                            worst_values = (expected, actual);
                        }
                    }
                }
                assert!(
                    max_error <= 3.0,
                    "Modes {first_mode:?}/{mode:?}, feather {feather}px transformed-mask GPU/software mismatch at ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max full-frame error {max_error:.3} 8-bit values",
                    worst_pixel.0,
                    worst_pixel.1,
                    worst_pixel.2,
                    worst_values.0,
                    worst_values.1
                );
            }
        }
    }
    let _ = std::fs::remove_file(&image_path);
}

#[test]
fn full_hd_curved_masks_match_software_rendering() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping Full HD mask parity test: no adapter available");
        return;
    };
    const FULL_HD_WIDTH: u32 = 1920;
    const FULL_HD_HEIGHT: u32 = 1080;
    let mut comp = Composition::new(
        "full-hd-mask-parity".into(),
        "Full HD Mask Parity".into(),
        FULL_HD_WIDTH,
        FULL_HD_HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];
    let mut layer = Layer::new(
        "full-hd-mask-layer".into(),
        "Full HD Mask Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        1,
    );
    layer.transform.position = constant([960.0, 540.0]);
    layer.transform.anchor_point = constant([960.0, 540.0]);
    layer.transform.rotation = constant(-7.0);
    let mut ellipse = kagari_vfx::core::mask::Mask::new_ellipse(
        "full-hd-ellipse".into(),
        "Full HD Ellipse".into(),
        960.0,
        540.0,
        470.0,
        310.0,
    );
    ellipse.feather = constant(24.0);
    let mut subtract = kagari_vfx::core::mask::Mask::new_rect(
        "full-hd-cutout".into(),
        "Full HD Cutout".into(),
        900.0,
        445.0,
        250.0,
        190.0,
    );
    subtract.mode = kagari_vfx::core::mask::MaskMode::Subtract;
    subtract.feather = constant(8.0);
    subtract.opacity = constant(91.0);
    layer.masks.extend([ellipse, subtract]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, FULL_HD_WIDTH, FULL_HD_HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let mut max_error = 0.0f32;
    let mut worst_pixel = (0usize, 0usize, 0usize);
    let mut worst_values = (0.0f32, 0.0f32);
    for (index, actual_pixel) in gpu.iter().enumerate() {
        for channel in 0..4 {
            let expected = cpu[index * 4 + channel] as f32;
            let actual = actual_pixel[channel] * 255.0;
            let error = (expected - actual).abs();
            if error > max_error {
                max_error = error;
                worst_pixel = (
                    index % FULL_HD_WIDTH as usize,
                    index / FULL_HD_WIDTH as usize,
                    channel,
                );
                worst_values = (expected, actual);
            }
        }
    }
    assert!(
        max_error <= 3.0,
        "Full HD curved-mask GPU/software mismatch at ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max full-frame error {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2,
        worst_values.0,
        worst_values.1
    );
}
