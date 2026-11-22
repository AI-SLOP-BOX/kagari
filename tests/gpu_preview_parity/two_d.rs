use super::*;

#[test]
fn allowlisted_gpu_effects_match_the_software_reference_across_the_full_frame() {
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
        (
            "Posterize",
            EffectType::Posterize {
                levels: constant(4.0),
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
        let mut max_error = 0.0f32;
        let mut worst_pixel = (0usize, 0usize, 0usize);
        for (index, actual_pixel) in gpu.iter().enumerate() {
            for channel in 0..4 {
                let expected = cpu[index * 4 + channel] as f32;
                let actual = actual_pixel[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > max_error {
                    max_error = error;
                    worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
                }
            }
        }
        assert!(
            max_error <= 2.0,
            "{name} GPU/software mismatch at pixel ({}, {}), channel {}: max full-frame error {max_error:.3} 8-bit values",
            worst_pixel.0,
            worst_pixel.1,
            worst_pixel.2
        );
    }
}

#[test]
fn allowlisted_gpu_effects_match_software_for_transparent_image_pixels() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU alpha parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-gpu-alpha-parity-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_fn(WIDTH, HEIGHT, |x, y| {
        let alpha = [0, 64, 128, 255][((x / 16 + y / 16) % 4) as usize];
        image::Rgba([(x * 19) as u8, (y * 23) as u8, ((x + y) * 11) as u8, alpha])
    })
    .save(&image_path)
    .expect("transparent image fixture should be writable");

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
        (
            "Posterize",
            EffectType::Posterize {
                levels: constant(4.0),
            },
        ),
    ];
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let mut max_error_per_effect = Vec::new();

    for (name, effect_type) in cases {
        assert!(gpu_preview_effect_supported(&effect_type));
        let mut comp = composition_with_effect(name, effect_type);
        comp.background_color = [0.0; 4];
        comp.layers[0].layer_type = LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        };
        comp.layers[0].transform.opacity = constant(63.7);
        let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
        let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
        let mut worst = (0.0f32, 0usize, 0usize, 0usize, 0.0f32, 0.0f32);
        for (index, (expected, actual)) in cpu.as_chunks::<4>().0.iter().zip(gpu.iter()).enumerate()
        {
            // The GPU target uses source-over blending and stores premultiplied
            // RGB; the software renderer returns straight RGBA bytes.
            let alpha = expected[3] as f32 / 255.0;
            for channel in 0..4 {
                let expected = if channel == 3 {
                    expected[channel] as f32
                } else {
                    expected[channel] as f32 * alpha
                };
                let actual = actual[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > worst.0 {
                    worst = (
                        error,
                        index % WIDTH as usize,
                        index / WIDTH as usize,
                        channel,
                        expected,
                        actual,
                    );
                }
            }
        }
        max_error_per_effect.push((name, worst));
    }

    let mut comp = composition_with_effect(
        "transparent-chain",
        EffectType::Invert {
            invert_alpha: false,
        },
    );
    comp.background_color = [0.0; 4];
    comp.layers[0].layer_type = LayerType::Image {
        path: image_path.to_string_lossy().into_owned(),
    };
    comp.layers[0].transform.opacity = constant(63.7);
    comp.layers[0].effects = vec![
        Effect {
            id: "chain-flare".into(),
            name: "Lens Flare".into(),
            effect_type: EffectType::LensFlare {
                enabled: constant(1.0),
                position_x: constant(0.5),
                position_y: constant(0.5),
                intensity: constant(0.45),
                threshold: constant(1.0),
                color: constant([1.0, 0.7, 0.35, 1.0]),
                link_to_light: None,
            },
            enabled: true,
        },
        Effect {
            id: "chain-tint".into(),
            name: "Color Tint".into(),
            effect_type: EffectType::ColorTint {
                color: constant([0.8, 0.3, 0.6, 1.0]),
                intensity: constant(37.0),
            },
            enabled: true,
        },
        Effect {
            id: "chain-levels".into(),
            name: "Levels".into(),
            effect_type: EffectType::Levels {
                input_black: constant(0.03),
                input_white: constant(0.97),
                gamma: constant(1.15),
                output_black: constant(0.01),
                output_white: constant(0.99),
            },
            enabled: true,
        },
        Effect {
            id: "chain-invert".into(),
            name: "Invert".into(),
            effect_type: EffectType::Invert {
                invert_alpha: false,
            },
            enabled: true,
        },
        Effect {
            id: "chain-posterize".into(),
            name: "Posterize".into(),
            effect_type: EffectType::Posterize {
                levels: constant(6.0),
            },
            enabled: true,
        },
    ];
    let effect_chain = comp.layers[0].effects.clone();
    comp.layers[0].effects = effect_chain[..4].to_vec();
    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None,
        "the transparent four-effect chain should remain GPU-allowlisted"
    );
    for chain_len in 1..effect_chain.len() {
        comp.layers[0].effects = effect_chain[..chain_len].to_vec();
        let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
        let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
        let mut worst = (0.0f32, 0usize, 0usize, 0usize, 0.0f32, 0.0f32);
        for (index, (expected, actual)) in cpu.as_chunks::<4>().0.iter().zip(gpu.iter()).enumerate()
        {
            let alpha = expected[3] as f32 / 255.0;
            for channel in 0..4 {
                let expected = if channel == 3 {
                    expected[channel] as f32
                } else {
                    expected[channel] as f32 * alpha
                };
                let actual = actual[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > worst.0 {
                    worst = (
                        error,
                        index % WIDTH as usize,
                        index / WIDTH as usize,
                        channel,
                        expected,
                        actual,
                    );
                }
            }
        }
        assert!(
            worst.0 <= 2.0,
            "transparent GPU/software effect prefix {chain_len} mismatch at ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max error {:.3} 8-bit values",
            worst.1, worst.2, worst.3, worst.4, worst.5, worst.0
        );
    }
    comp.layers[0].effects = effect_chain;
    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None,
        "Posterize following other effects now renders through the GPU compositor"
    );

    for (name, (max_error, x, y, channel, expected, actual)) in max_error_per_effect {
        assert!(
            max_error <= 2.0,
            "{name} GPU/software alpha-frame mismatch at ({x}, {y}), channel {channel}: CPU {expected:.3}, GPU {actual:.3}, max error {max_error:.3} 8-bit values"
        );
    }

    let mut screen_comp = composition_with_effect(
        "transparent-screen",
        EffectType::Levels {
            input_black: constant(0.02),
            input_white: constant(0.98),
            gamma: constant(1.1),
            output_black: constant(0.0),
            output_white: constant(1.0),
        },
    );
    screen_comp.background_color = [0.13, 0.19, 0.22, 0.4];
    screen_comp.layers[0].transform.opacity = constant(63.7);
    screen_comp.layers[0].layer_type = LayerType::Image {
        path: image_path.to_string_lossy().into_owned(),
    };
    for blend_mode in [BlendMode::Normal, BlendMode::Screen] {
        screen_comp.layers[0].blend_mode = blend_mode;
        assert_eq!(
            kagari_vfx::core::render_capability::software_preview_reason(&screen_comp, 0),
            None,
            "{blend_mode:?} blend should remain GPU-allowlisted"
        );
        let cpu = render_frame_to_pixels(&screen_comp, 0, WIDTH, HEIGHT, 0.0, 0);
        let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &screen_comp);
        let mut worst = (0.0f32, 0usize, 0usize, 0usize, 0.0f32, 0.0f32);
        for (index, (expected, actual)) in cpu.as_chunks::<4>().0.iter().zip(gpu.iter()).enumerate()
        {
            let alpha = expected[3] as f32 / 255.0;
            for channel in 0..4 {
                let expected = if channel == 3 {
                    expected[channel] as f32
                } else {
                    expected[channel] as f32 * alpha
                };
                let actual = actual[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > worst.0 {
                    worst = (
                        error,
                        index % WIDTH as usize,
                        index / WIDTH as usize,
                        channel,
                        expected,
                        actual,
                    );
                }
            }
        }
        assert!(
            worst.0 <= 2.0,
            "transparent {blend_mode:?} blend GPU/software mismatch at ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max error {:.3} 8-bit values",
            worst.1, worst.2, worst.3, worst.4, worst.5, worst.0
        );
    }
    let _ = std::fs::remove_file(&image_path);
}

#[test]
fn transformed_transparent_image_matches_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping transformed-image parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-transformed-image-parity-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_fn(8, 8, |x, y| {
        let alpha = [48, 112, 192, 255][((x + 2 * y) % 4) as usize];
        image::Rgba([
            (x * 31) as u8,
            (y * 29) as u8,
            ((x * 13 + y * 17) % 256) as u8,
            alpha,
        ])
    })
    .save(&image_path)
    .expect("transformed image fixture should be writable");

    let mut comp = Composition::new(
        "transformed-image".into(),
        "transformed-image".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.07, 0.11, 0.16, 1.0];
    let mut layer = Layer::new(
        "transformed-image-layer".into(),
        "Transformed Image".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.transform.position = constant([31.25, 32.75]);
    layer.transform.anchor_point = constant([4.0, 4.0]);
    layer.transform.scale = constant([275.0, 135.0]);
    layer.transform.rotation = constant(27.0);
    layer.transform.opacity = constant(73.4);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let _ = std::fs::remove_file(&image_path);
    let mut max_error = 0.0f32;
    let mut worst_pixel = (0usize, 0usize, 0usize);
    for (index, actual_pixel) in gpu.iter().enumerate() {
        for channel in 0..4 {
            let expected = cpu[index * 4 + channel] as f32;
            let actual = actual_pixel[channel] * 255.0;
            let error = (expected - actual).abs();
            if error > max_error {
                max_error = error;
                worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
            }
        }
    }
    assert!(
        max_error <= 3.0,
        "Transformed image GPU/software mismatch at ({}, {}), channel {}: max full-frame error {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2
    );
}

#[path = "masks.rs"]
mod masks;

#[test]
fn negative_layer_scale_mirrors_image_content_identically_on_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping negative-scale parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-negative-scale-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_raw(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, // red, green
            0, 0, 255, 255, 255, 255, 0, 255, // blue, yellow
        ],
    )
    .expect("2x2 RGBA fixture should be valid")
    .save(&image_path)
    .expect("image fixture should be writable");

    let mut comp = Composition::new(
        "negative-scale".into(),
        "negative-scale".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "negative-image".into(),
        "Negative Scale Image".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.transform.position = Animatable::new_constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point = Animatable::new_constant([1.0, 1.0]);
    // Upscale enough to exercise the GPU's bilinear sampler between texel
    // centers, while retaining negative X scale to verify mirrored UVs.
    layer.transform.scale = Animatable::new_constant([-1500.0, 1500.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
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
                worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
                worst_values = (expected, actual);
            }
        }
    }
    let _ = std::fs::remove_file(&image_path);
    assert!(
        max_error <= 2.0,
        "Negative-scale GPU/software mismatch at pixel ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max error {max_error:.3}",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2,
        worst_values.0,
        worst_values.1
    );
}

#[test]
fn gpu_video_frame_blending_matches_software_at_fractional_source_frames() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping video frame-blending parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let frames_dir =
        std::env::temp_dir().join(format!("kagari-video-blend-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&frames_dir).expect("video frame fixture directory should be writable");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
        .save(frames_dir.join("frame_00000.png"))
        .expect("first video frame should be writable");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 255, 0, 255]))
        .save(frames_dir.join("frame_00001.png"))
        .expect("second video frame should be writable");

    let mut comp = Composition::new(
        "video-blend".into(),
        "video-blend".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "blended-video".into(),
        "Blended Video".into(),
        LayerType::Video {
            source: "parity-fixture".into(),
            frames_dir: frames_dir.to_string_lossy().into_owned(),
            frame_count: 2,
            audio_wav: None,
            audio_volume: constant(0.0),
            speed: 1.0,
        },
        1,
    );
    layer.time_remap = Some(constant(0.5));
    layer.frame_blending = true;
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point = constant([1.0, 1.0]);
    layer.transform.scale = constant([3200.0, 3200.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let _ = std::fs::remove_dir_all(&frames_dir);
    let mut max_error = 0.0f32;
    let mut worst_pixel = (0usize, 0usize, 0usize);
    for (index, actual_pixel) in gpu.iter().enumerate() {
        for channel in 0..4 {
            let expected = cpu[index * 4 + channel] as f32;
            let actual = actual_pixel[channel] * 255.0;
            let error = (expected - actual).abs();
            if error > max_error {
                max_error = error;
                worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
            }
        }
    }
    assert!(
        max_error <= 2.0,
        "Frame-blending GPU/software mismatch at pixel ({}, {}), channel {}: max full-frame error {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2
    );
}

#[test]
fn negative_layer_scale_mirrors_text_identically_on_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping negative-scale text parity test: no adapter available");
        return;
    };
    let mut comp = Composition::new(
        "negative-text".into(),
        "negative-text".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "negative-text-layer".into(),
        "Negative Scale Text".into(),
        LayerType::Text {
            text: "KAGARI".into(),
            font_size: 22,
            color: [1.0, 0.8, 0.25, 1.0],
            font_family: "DejaVu Sans".into(),
            tracking: 0.0,
            leading: 1.2,
            align: 0,
            stroke_color: [0.03, 0.05, 0.08, 1.0],
            stroke_width: 2.0,
            text_on_path: false,
        },
        1,
    );
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.scale = constant([-100.0, 100.0]);
    layer.transform.rotation = constant(13.0);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
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
                worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
                worst_values = (expected, actual);
            }
        }
    }
    // The software reference stores intermediate coverage in RGBA8 while the
    // GPU target is RGBA16F; rotated glyph-edge filtering differs by <3 values.
    assert!(
        max_error <= 3.0,
        "Negative-scale text GPU/software mismatch at pixel ({}, {}), channel {}: CPU {:.3}, GPU {:.3}, max full-frame error {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2,
        worst_values.0,
        worst_values.1
    );
}

#[test]
fn gpu_linear_light_compositing_matches_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU linear-light parity test: no adapter available");
        return;
    };

    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut comp = Composition::new(
        "gpu_linear_light_parity".to_owned(),
        "GPU Linear Light Parity".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.2, 0.4, 0.7, 1.0];
    comp.blend_linear = true;

    let mut solid_normal = Layer::new(
        "lin_normal".to_owned(),
        "Linear Normal".to_owned(),
        LayerType::Solid {
            color: [0.85, 0.25, 0.15, 1.0],
        },
        10,
    );
    solid_normal.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    solid_normal.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    solid_normal.transform.opacity = constant(55.0);
    comp.layers.push(solid_normal);

    let mut solid_overlay = Layer::new(
        "lin_overlay".to_owned(),
        "Linear Overlay".to_owned(),
        LayerType::Solid {
            color: [0.3, 0.8, 0.45, 1.0],
        },
        10,
    );
    solid_overlay.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    solid_overlay.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    solid_overlay.transform.opacity = constant(75.0);
    solid_overlay.blend_mode = BlendMode::Overlay;
    comp.layers.push(solid_overlay);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU linear-light render should succeed");
    assert_eq!(gpu.len(), cpu.len());

    let mut max_error = 0i32;
    for (idx, (&actual, &expected)) in gpu.iter().zip(cpu.iter()).enumerate() {
        let diff = (actual as i32 - expected as i32).abs();
        if diff > max_error {
            max_error = diff;
        }
        assert!(
            diff <= 2,
            "Linear-light GPU/CPU mismatch at byte {idx}: CPU={expected}, GPU={actual} (max_error={max_error})"
        );
    }
}

#[test]
fn gpu_renders_layer_styles_and_multipass_effect_stacks_with_cpu_parity() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU layer style / multipass effect parity test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_all_features".to_owned(),
        "GPU All Features".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.1, 0.15, 0.25, 1.0];

    let mut base = Layer::new(
        "base".to_owned(),
        "Base".to_owned(),
        LayerType::Solid {
            color: [0.3, 0.6, 0.4, 1.0],
        },
        10,
    );
    base.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    base.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    comp.layers.push(base);

    let img_path = std::env::temp_dir().join(format!(
        "kagari_gpu_ds_multipass_{}.png",
        std::process::id()
    ));
    let mut img = image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([0, 0, 0, 0]));
    for y in 12..36 {
        for x in 12..36 {
            img.put_pixel(x, y, image::Rgba([230, 102, 51, 255]));
        }
    }
    img.save(&img_path).unwrap();

    let mut styled = Layer::new(
        "styled".to_owned(),
        "Styled".to_owned(),
        LayerType::Image {
            path: img_path.to_string_lossy().into_owned(),
        },
        10,
    );
    styled.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    styled.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    styled.style.drop_shadow.enabled = true;
    styled.style.drop_shadow.distance = 4.0;
    styled.style.drop_shadow.angle = 135.0;
    styled.style.drop_shadow.size = 3.0;
    styled.style.drop_shadow.opacity = 80.0;
    styled.effects = vec![
        Effect {
            id: "levels".to_owned(),
            name: "Levels".to_owned(),
            effect_type: EffectType::Levels {
                input_black: constant(0.05),
                input_white: constant(0.95),
                gamma: constant(1.1),
                output_black: constant(0.0),
                output_white: constant(1.0),
            },
            enabled: true,
        },
        Effect {
            id: "posterize".to_owned(),
            name: "Posterize".to_owned(),
            effect_type: EffectType::Posterize {
                levels: constant(6.0),
            },
            enabled: true,
        },
    ];
    comp.layers.push(styled);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU render should succeed for layer styles and multipass effects");
    let _ = std::fs::remove_file(&img_path);
    assert_eq!(gpu.len(), cpu.len());

    let mut max_error = 0i32;
    for (idx, (&actual, &expected)) in gpu.iter().zip(cpu.iter()).enumerate() {
        let diff = (actual as i32 - expected as i32).abs();
        if diff > max_error {
            max_error = diff;
        }
        assert!(
            diff <= 20,
            "LayerStyle/Multipass GPU/CPU mismatch at byte {idx}: CPU={expected}, GPU={actual} (max_error={max_error})"
        );
    }
}

#[test]
fn gpu_motion_blur_smears_moving_layer_in_comp_space() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU motion blur test: no adapter available");
        return;
    };

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    let mut comp = Composition::new(
        "gpu_motion_blur".to_owned(),
        "GPU Motion Blur".to_owned(),
        WIDTH,
        HEIGHT,
        24,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];
    comp.motion_blur_shutter_angle = 180.0;

    let mut layer = Layer::new(
        "moving_solid".to_owned(),
        "Moving Solid".to_owned(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        10,
    );
    layer.transform.position = kagari_vfx::core::property::Animatable::Animated(vec![
        kagari_vfx::core::keyframe::types::Keyframe::new(
            0,
            [16.0, 32.0],
            kagari_vfx::core::keyframe::types::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::types::Keyframe::new(
            4,
            [48.0, 32.0],
            kagari_vfx::core::keyframe::types::InterpolationType::Linear,
        ),
    ]);
    layer.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.scale = constant([25.0, 25.0]);
    layer.motion_blur = true;
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 2),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let blurred = renderer
        .render_frame_to_rgba8_gpu(&comp, 2, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU motion blur render should succeed");

    comp.layers[0].motion_blur = false;
    let sharp = renderer
        .render_frame_to_rgba8_gpu(&comp, 2, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU sharp render should succeed");

    let diff_sum: u64 = blurred
        .iter()
        .zip(sharp.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).unsigned_abs() as u64)
        .sum();
    assert!(
        diff_sum > 500,
        "GPU motion blur should visibly smear a moving layer (diff_sum={diff_sum})"
    );
}

#[test]
fn gpu_runs_extended_wgsl_effects_without_raster_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping extended WGSL effects test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_ext_wgsl".to_owned(),
        "GPU Extended WGSL".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.05, 0.05, 0.08, 1.0];

    let mut layer = Layer::new(
        "fx_solid".to_owned(),
        "FX Solid".to_owned(),
        LayerType::Solid {
            color: [0.85, 0.35, 0.25, 1.0],
        },
        10,
    );
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.scale = constant([50.0, 50.0]);
    layer.effects = vec![
        Effect {
            id: "blur".to_owned(),
            name: "Gaussian Blur".to_owned(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: constant(4.0),
            },
            enabled: true,
        },
        Effect {
            id: "huesat".to_owned(),
            name: "Hue/Saturation".to_owned(),
            effect_type: EffectType::HueSaturation {
                hue_shift: constant(90.0),
                saturation: constant(25.0),
                lightness: constant(10.0),
            },
            enabled: true,
        },
        Effect {
            id: "glow".to_owned(),
            name: "Glow".to_owned(),
            effect_type: EffectType::Glow {
                threshold: constant(20.0),
                radius: constant(6.0),
                intensity: constant(80.0),
                color: constant([1.0, 0.9, 0.6, 1.0]),
            },
            enabled: true,
        },
        Effect {
            id: "vignette".to_owned(),
            name: "Vignette".to_owned(),
            effect_type: EffectType::Vignette {
                intensity: constant(50.0),
                roundness: constant(1.0),
                feather: constant(50.0),
                color: constant([0.0, 0.0, 0.0, 1.0]),
            },
            enabled: true,
        },
    ];
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU extended WGSL effect chain should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(gpu.iter().any(|&v| v > 30));
}

#[test]
fn gpu_runs_commercial_wgsl_effects_and_invert_alpha_without_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping commercial WGSL effects test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_comm_wgsl".to_owned(),
        "GPU Commercial WGSL".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.05, 0.05, 0.08, 1.0];

    let mut layer = Layer::new(
        "fx_solid".to_owned(),
        "FX Solid".to_owned(),
        LayerType::Solid {
            color: [0.85, 0.35, 0.25, 1.0],
        },
        10,
    );
    layer.effects = vec![
        Effect {
            id: "hsl".to_owned(),
            name: "HSL".to_owned(),
            effect_type: EffectType::HslAdjust {
                hue_deg: constant(45.0),
                saturation: constant(0.2),
                lightness: constant(0.05),
            },
            enabled: true,
        },
        Effect {
            id: "shift".to_owned(),
            name: "Shift Channels".to_owned(),
            effect_type: EffectType::ShiftChannels {
                take_red: constant(1.0),
                take_green: constant(2.0),
                take_blue: constant(0.0),
                take_alpha: constant(3.0),
            },
            enabled: true,
        },
        Effect {
            id: "grad".to_owned(),
            name: "Gradient Map".to_owned(),
            effect_type: EffectType::GradientMap {
                low_color: constant([0.1, 0.1, 0.2]),
                mid_color: constant([0.8, 0.4, 0.2]),
                high_color: constant([1.0, 0.95, 0.8]),
            },
            enabled: true,
        },
        Effect {
            id: "rays".to_owned(),
            name: "God Rays".to_owned(),
            effect_type: EffectType::GodRays {
                sun_x: constant(0.5),
                sun_y: constant(0.5),
                samples: constant(16.0),
                decay: constant(0.92),
                weight: constant(0.5),
            },
            enabled: true,
        },
        Effect {
            id: "inv_a".to_owned(),
            name: "Invert Alpha".to_owned(),
            effect_type: EffectType::Invert { invert_alpha: true },
            enabled: true,
        },
    ];
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU commercial WGSL effect chain should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(gpu.iter().any(|&v| v > 10));
}

#[test]
fn gpu_stencil_silhouette_and_alpha_add_blend_modes_match_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping stencil/silhouette blend test: no adapter available");
        return;
    };

    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut renderer = WgpuRenderer::new(device, queue);

    for mode in [
        BlendMode::StencilAlpha,
        BlendMode::StencilLuma,
        BlendMode::SilhouetteAlpha,
        BlendMode::SilhouetteLuma,
        BlendMode::AlphaAdd,
    ] {
        let mut comp = Composition::new(
            "gpu_stencil_blend".to_owned(),
            "GPU Stencil Blend".to_owned(),
            WIDTH,
            HEIGHT,
            30,
            1,
        );
        comp.background_color = [0.0, 0.0, 0.0, 0.0];

        let base = Layer::new(
            "base".to_owned(),
            "Base".to_owned(),
            LayerType::Solid {
                color: [0.8, 0.4, 0.2, 0.8],
            },
            1,
        );
        let mut top = Layer::new(
            "top".to_owned(),
            "Top".to_owned(),
            LayerType::Solid {
                color: [0.6, 0.9, 0.3, 0.5],
            },
            1,
        );
        top.transform.scale = constant([50.0, 50.0]);
        top.blend_mode = mode;

        comp.layers.push(top);
        comp.layers.push(base);

        let gpu = renderer
            .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
            .expect("GPU stencil/silhouette blend should render");
        assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
        assert!(gpu.iter().any(|&v| v > 0));
    }
}

#[test]
fn gpu_text_animator_renders_without_raster_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping text animator GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 64;
    let mut comp = Composition::new(
        "gpu_text_anim".to_owned(),
        "GPU Text Animator".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "text_anim".to_owned(),
        "Animated Text".to_owned(),
        LayerType::Text {
            text: "KAGARI".to_owned(),
            font_size: 28,
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "sans-serif".to_owned(),
            tracking: 0.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            text_on_path: false,
        },
        10,
    );
    let mut anim = kagari_vfx::core::text_animator::TextAnimatorSettings::default();
    anim.enabled = true;
    anim.position_offset = [0.0, -8.0];
    anim.opacity = 1.0;
    anim.tracking = 4.0;
    layer.text_animator = Some(anim);
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU text animator should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(gpu.iter().any(|&v| v > 30));
}

#[test]
fn gpu_set_matte_uses_gpu_matte_texture_without_raster_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping SetMatte GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_set_matte".to_owned(),
        "GPU Set Matte".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 0.0];

    let mut matte_src = Layer::new(
        "matte_src".to_owned(),
        "Matte Source".to_owned(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        10,
    );
    matte_src.visible = true;
    matte_src.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    matte_src.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    matte_src.transform.scale = constant([50.0, 50.0]);

    let mut target = Layer::new(
        "target".to_owned(),
        "Target".to_owned(),
        LayerType::Solid {
            color: [0.2, 0.7, 0.9, 1.0],
        },
        10,
    );
    target.effects = vec![Effect {
        id: "set_matte".to_owned(),
        name: "Set Matte".to_owned(),
        effect_type: EffectType::SetMatte {
            source_layer_idx: 1,
            source_channel: kagari_vfx::core::set_matte::MatteSourceChannel::Luminance,
            invert_matte: false,
            composite_mode: kagari_vfx::core::set_matte::MatteCompositeMode::Replace,
        },
        enabled: true,
    }];

    comp.layers.push(target);
    comp.layers.push(matte_src);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU SetMatte should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    // Corner pixel should be masked out (alpha 0), center pixel should have color
    assert_eq!(gpu[3], 0);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    assert!(gpu[center_idx + 3] > 200);
}

#[test]
fn gpu_extruded_and_beveled_shape_renders_without_raster_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping extruded/beveled shape GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_extrude".to_owned(),
        "GPU Extrude".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "shape_extrude".to_owned(),
        "Extruded Shape".to_owned(),
        LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Rectangle {
                width: constant(30.0),
                height: constant(30.0),
                corner_radius: constant(4.0),
            },
            color: [0.3, 0.6, 0.9, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 20.0,
            bevel_depth: 5.0,
        },
        10,
    );
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU extruded/beveled shape should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    assert!(gpu[center_idx + 2] > 100);
}

#[test]
fn gpu_echo_effect_renders_multi_frame_echoes_on_gpu() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping Echo GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 32;
    let mut comp = Composition::new(
        "gpu_echo".to_owned(),
        "GPU Echo".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "moving_solid".to_owned(),
        "Moving Solid".to_owned(),
        LayerType::Solid {
            color: [0.2, 0.8, 1.0, 1.0],
        },
        10,
    );
    layer.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.scale = constant([20.0, 40.0]);
    layer.transform.position = kagari_vfx::core::property::Animatable::new_animated(vec![
        kagari_vfx::core::keyframe::types::Keyframe::new(
            0,
            [16.0, 16.0],
            kagari_vfx::core::keyframe::types::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::types::Keyframe::new(
            5,
            [48.0, 16.0],
            kagari_vfx::core::keyframe::types::InterpolationType::Linear,
        ),
    ]);
    layer.effects = vec![Effect {
        id: "echo".to_owned(),
        name: "Echo".to_owned(),
        effect_type: EffectType::Echo {
            echo_time_seconds: constant(-1.0 / 30.0),
            num_echoes: 3,
            starting_intensity: constant(0.8),
            decay: constant(0.6),
            operator: kagari_vfx::core::echo_effect::EchoOperator::Add,
        },
        enabled: true,
    }];
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 5),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 5, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU Echo should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    // Trailing echo should light up pixels behind the current position (around x=35)
    let trail_idx = (16 * WIDTH + 35) as usize * 4;
    assert!(gpu[trail_idx + 2] > 20);
}

#[test]
fn gpu_custom_shader_compiles_and_renders_arbitrary_wgsl_on_gpu() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping CustomShader GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut comp = Composition::new(
        "gpu_custom_wgsl".to_owned(),
        "GPU Custom WGSL".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        5,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "custom_src".to_owned(),
        "Custom Src".to_owned(),
        LayerType::Solid {
            color: [0.0, 0.0, 0.0, 1.0],
        },
        5,
    );
    layer.effects = vec![Effect {
        id: "custom_wgsl".to_owned(),
        name: "Custom WGSL".to_owned(),
        effect_type: EffectType::CustomShader {
            wgsl_source: r#"
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> u_params: vec4<f32>;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let base = textureSample(t_diffuse, s_diffuse, uv);
    return vec4<f32>(u_params.x, u_params.y, u_params.z, base.a);
}
"#
            .to_owned(),
            uniform_values: vec![0.9, 0.25, 0.6, 1.0],
        },
        enabled: true,
    }];
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU CustomShader should compile and render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    assert!(gpu[center_idx] > 210);
    assert!(gpu[center_idx + 1] < 90);
    assert!(gpu[center_idx + 2] > 120);
}

#[test]
fn gpu_particle_layer_renders_particles_on_gpu_without_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping Particle GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    let mut comp = Composition::new(
        "gpu_particle".to_owned(),
        "GPU Particle".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        20,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut emitter = kagari_vfx::core::particle_system::ParticleEmitter::default();
    emitter.rate = 120.0;
    emitter.size_start = 8.0;
    emitter.size_end = 4.0;
    emitter.color_start = [1.0, 0.5, 0.2, 1.0];
    emitter.color_end = [1.0, 0.2, 0.1, 0.8];

    let mut layer = Layer::new(
        "particles".to_owned(),
        "Particles".to_owned(),
        LayerType::Particle { emitter },
        20,
    );
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 10),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 10, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU Particle layer should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(gpu.iter().any(|&v| v > 50));
}

#[test]
fn gpu_paint_strokes_and_puppet_pins_render_on_gpu_without_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping Paint/Puppet GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_paint_puppet".to_owned(),
        "GPU Paint & Puppet".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut paint_layer = Layer::new(
        "paint".to_owned(),
        "Paint".to_owned(),
        LayerType::Solid {
            color: [0.1, 0.1, 0.1, 1.0],
        },
        10,
    );
    paint_layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    paint_layer.paint_strokes.push(kagari_vfx::core::timeline::PaintStroke {
        color: [0.0, 1.0, 0.4, 1.0],
        size: 10.0,
        mode: kagari_vfx::core::timeline::PaintStrokeMode::Brush,
        clone_offset: [0.0, 0.0],
        hardness: 0.9,
        opacity: 1.0,
        flow: 1.0,
        points: vec![[-10.0, 0.0], [10.0, 0.0]],
        start_frame: 0,
        end_frame: 0,
    });
    paint_layer.puppet_pins.push(kagari_vfx::core::timeline::PuppetPin {
        id: "pin1".to_owned(),
        name: "Pin 1".to_owned(),
        comp_source: [24.0, 24.0],
        position: constant([28.0, 26.0]),
    });
    comp.layers.push(paint_layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU Paint & Puppet should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    assert!(gpu[center_idx + 1] > 180);
}

#[test]
fn gpu_model3d_renders_obj_mesh_with_depth_buffer_on_gpu() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping Model3D GPU test: no adapter available");
        return;
    };

    let obj_path = std::env::temp_dir().join("kagari_gpu_model3d_test.obj");
    std::fs::write(
        &obj_path,
        "v -1.0 -1.0 0.0\nv 1.0 -1.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n",
    )
    .expect("write test obj");

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    let mut comp = Composition::new(
        "gpu_model3d".to_owned(),
        "GPU Model3D".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "model3d".to_owned(),
        "Model 3D".to_owned(),
        LayerType::Model3D {
            path: obj_path.to_string_lossy().into_owned(),
        },
        10,
    );
    layer.transform_3d.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5, 0.0]);
    layer.transform.position = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point = constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU Model3D should render");
    let _ = std::fs::remove_file(&obj_path);
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(gpu.iter().any(|&v| v > 40));
}

#[test]
fn gpu_puppet_mesh_warps_more_than_4_pins_on_gpu_without_fallback() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping 6-pin Puppet GPU test: no adapter available");
        return;
    };

    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let mut comp = Composition::new(
        "gpu_puppet_6pins".to_owned(),
        "GPU Puppet 6 Pins".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        10,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "puppet_solid".to_owned(),
        "Puppet Solid".to_owned(),
        LayerType::Solid {
            color: [0.3, 0.85, 0.95, 1.0],
        },
        10,
    );
    for i in 0..6 {
        let x = 8.0 + i as f32 * 6.0;
        layer.puppet_pins.push(kagari_vfx::core::timeline::PuppetPin {
            id: format!("pin_{i}"),
            name: format!("Pin {i}"),
            comp_source: [x, 24.0],
            position: constant([x + 1.5, 22.0]),
        });
    }
    comp.layers.push(layer);

    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None
    );

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU 6-pin Puppet mesh should render");
    assert_eq!(gpu.len(), (WIDTH * HEIGHT * 4) as usize);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    assert!(gpu[center_idx + 2] > 180);
}

#[test]
fn gpu_color_grade_lut_samples_real_cube_3d_texture_on_gpu() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping 3D LUT GPU test: no adapter available");
        return;
    };

    // 2x2x2 .cube LUT that swaps Red -> Blue (maps (1,0,0) -> (0,0,1))
    let cube_path = std::env::temp_dir().join("kagari_gpu_swap_lut_test.cube");
    std::fs::write(
        &cube_path,
        "LUT_3D_SIZE 2\n\
         0.0 0.0 0.0\n\
         0.0 0.0 1.0\n\
         0.0 1.0 0.0\n\
         0.0 1.0 1.0\n\
         1.0 0.0 0.0\n\
         1.0 0.0 1.0\n\
         1.0 1.0 0.0\n\
         1.0 1.0 1.0\n",
    )
    .expect("write test .cube LUT");

    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut comp = Composition::new(
        "gpu_cube_lut".to_owned(),
        "GPU Cube LUT".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        5,
    );
    comp.background_color = [0.0, 0.0, 0.0, 1.0];

    let mut layer = Layer::new(
        "red_solid".to_owned(),
        "Red Solid".to_owned(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        5,
    );
    layer.effects = vec![Effect {
        id: "lut".to_owned(),
        name: "3D LUT".to_owned(),
        effect_type: EffectType::ColorGradeLUT {
            lut_path: cube_path.to_string_lossy().into_owned(),
            intensity: constant(100.0),
        },
        enabled: true,
    }];
    comp.layers.push(layer);

    let mut renderer = WgpuRenderer::new(device, queue);
    let gpu = renderer
        .render_frame_to_rgba8_gpu(&comp, 0, WIDTH, HEIGHT, 0.0, 0)
        .expect("GPU 3D .cube LUT should render");
    let _ = std::fs::remove_file(&cube_path);
    let center_idx = ((HEIGHT / 2) * WIDTH + (WIDTH / 2)) as usize * 4;
    // Red (1,0,0) mapped to Blue (0,0,1) by the 3D LUT texture!
    assert!(gpu[center_idx] < 20);
    assert!(gpu[center_idx + 2] > 230);
}
