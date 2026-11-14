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
        Some("Posterize effect stacks require CPU renderer to preserve 8-bit stage rounding"),
        "Posterize following other effects must use the software reference"
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
