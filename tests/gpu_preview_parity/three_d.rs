use super::*;

#[test]
fn three_dimensional_image_matches_gpu_and_software_at_camera_center() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping 3D image parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-projected-3d-image-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_raw(
        WIDTH,
        HEIGHT,
        (0..HEIGHT)
            .flat_map(|y| {
                (0..WIDTH).flat_map(move |x| match (x < WIDTH / 2, y < HEIGHT / 2) {
                    (true, true) => [255, 0, 0, 255],
                    (false, true) => [0, 255, 0, 255],
                    (true, false) => [0, 0, 255, 255],
                    (false, false) => [255, 255, 0, 255],
                })
            })
            .collect(),
    )
    .expect("RGBA fixture should be valid")
    .save(&image_path)
    .expect("3D image fixture should be writable");

    let mut comp = Composition::new(
        "centered-3d".into(),
        "Centered 3D".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "centered-3d-image".into(),
        "Centered 3D Image".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, 1000.0]);
    layer.transform_3d.rotation = constant([18.0, 27.0, 0.0]);
    layer.transform_3d.scale = constant([120.0, 80.0, 100.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let _ = std::fs::remove_file(&image_path);
    let drawn_pixels: Vec<_> = cpu
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, pixel)| pixel[0] != 8 || pixel[1] != 10 || pixel[2] != 13)
        .map(|(index, _)| {
            (
                (index % WIDTH as usize) as f32,
                (index / WIDTH as usize) as f32,
            )
        })
        .collect();
    assert!(
        !drawn_pixels.is_empty(),
        "CPU reference did not draw the 3D image"
    );
    let (sum_x, sum_y) = drawn_pixels
        .iter()
        .fold((0.0, 0.0), |(sum_x, sum_y), (x, y)| (sum_x + x, sum_y + y));
    let mean_x = sum_x / drawn_pixels.len() as f32;
    let mean_y = sum_y / drawn_pixels.len() as f32;
    assert!(
        (mean_x - WIDTH as f32 * 0.5).abs() <= 1.0 && (mean_y - HEIGHT as f32 * 0.5).abs() <= 1.0,
        "Projected 3D image should be centered, got ({mean_x:.1}, {mean_y:.1})"
    );
    let gpu_drawn_pixels: Vec<_> = gpu
        .iter()
        .enumerate()
        .filter(|(_, pixel)| {
            (pixel[0] * 255.0 - 8.0).abs() > 1.0
                || (pixel[1] * 255.0 - 10.0).abs() > 1.0
                || (pixel[2] * 255.0 - 13.0).abs() > 1.0
        })
        .map(|(index, _)| {
            (
                (index % WIDTH as usize) as f32,
                (index / WIDTH as usize) as f32,
            )
        })
        .collect();
    assert!(
        !gpu_drawn_pixels.is_empty(),
        "GPU did not draw the 3D image"
    );
    let (gpu_sum_x, gpu_sum_y) = gpu_drawn_pixels
        .iter()
        .fold((0.0, 0.0), |(sum_x, sum_y), (x, y)| (sum_x + x, sum_y + y));
    let gpu_mean_x = gpu_sum_x / gpu_drawn_pixels.len() as f32;
    let gpu_mean_y = gpu_sum_y / gpu_drawn_pixels.len() as f32;
    assert!(
        (gpu_mean_x - WIDTH as f32 * 0.5).abs() <= 1.0
            && (gpu_mean_y - HEIGHT as f32 * 0.5).abs() <= 1.0,
        "GPU 3D image should be centered, got ({gpu_mean_x:.1}, {gpu_mean_y:.1})"
    );
    let mut max_error = 0.0f32;
    let mut worst_pixel = (0usize, 0usize, 0usize);
    for (index, actual_pixel) in gpu.iter().enumerate() {
        for channel in 0..4 {
            let error = (cpu[index * 4 + channel] as f32 - actual_pixel[channel] * 255.0).abs();
            if error > max_error {
                max_error = error;
                worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
            }
        }
    }
    assert!(
        max_error <= 3.0,
        "3D image GPU/software mismatch at ({}, {}), channel {}: max full-frame error {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2
    );
}

#[test]
fn gpu_renders_a_3d_solid_at_camera_center() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping 3D GPU rendering test: no adapter available");
        return;
    };
    let mut comp = Composition::new("3d-solid".into(), "3D Solid".into(), WIDTH, HEIGHT, 30, 1);
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "3d-solid".into(),
        "3D Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, 1000.0]);
    comp.layers.push(layer);

    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let center = gpu[(HEIGHT as usize / 2) * WIDTH as usize + WIDTH as usize / 2];
    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let cpu_center = ((HEIGHT as usize / 2) * WIDTH as usize + WIDTH as usize / 2) * 4;
    assert!(
        (center[0] * 255.0 - cpu[cpu_center] as f32).abs() <= 2.0
            && (center[1] * 255.0 - cpu[cpu_center + 1] as f32).abs() <= 2.0
            && (center[2] * 255.0 - cpu[cpu_center + 2] as f32).abs() <= 2.0,
        "3D GPU/CPU solid center mismatch: GPU {center:?}, CPU {:?}",
        &cpu[cpu_center..cpu_center + 4]
    );
}

#[test]
fn parallel_light_3d_layer_matches_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping parallel-light GPU/software parity test: no adapter available");
        return;
    };
    let mut comp = Composition::new(
        "parallel-3d".into(),
        "Parallel 3D".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    comp.lights = vec![Light3D {
        id: "parallel".into(),
        name: "Parallel".into(),
        light_type: LightType::Parallel,
        color: [1.0; 4],
        intensity: 100.0,
        intensity_animation: None,
        position: constant([900.0, -700.0, -800.0]),
        casts_shadows: false,
        shadow_darkness: 60.0,
        falloff: 2.0,
        max_radius: 1.0,
    }];
    let mut layer = Layer::new(
        "lit-solid".into(),
        "Lit solid".into(),
        LayerType::Solid {
            color: [0.7, 0.5, 0.3, 1.0],
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, 1000.0]);
    layer.material.ambient = 0.1;
    layer.material.diffuse = 0.8;
    layer.material.specular = 0.0;
    layer.material.metalness = 0.25;
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let center = (HEIGHT as usize / 2) * WIDTH as usize + WIDTH as usize / 2;
    let cpu_center = center * 4;
    for channel in 0..3 {
        let gpu_value = gpu[center][channel] * 255.0;
        let cpu_value = cpu[cpu_center + channel] as f32;
        assert!(
            (gpu_value - cpu_value).abs() <= 2.0,
            "parallel-light GPU/software mismatch in channel {channel}: GPU={gpu_value:.2}, CPU={cpu_value:.2}"
        );
    }
    assert!(
        cpu[cpu_center] > 70,
        "parallel light should illuminate the 3D layer"
    );
}

#[test]
fn colored_ambient_light_matches_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping ambient-light GPU/software parity test: no adapter available");
        return;
    };
    let mut comp = Composition::new(
        "ambient-3d".into(),
        "Ambient 3D".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    comp.lights = vec![Light3D {
        id: "ambient".into(),
        name: "Ambient".into(),
        light_type: LightType::Ambient,
        color: [1.0, 0.0, 0.0, 1.0],
        intensity: 100.0,
        intensity_animation: None,
        position: constant([-900.0, 700.0, -800.0]),
        casts_shadows: false,
        shadow_darkness: 60.0,
        falloff: 2.0,
        max_radius: 1.0,
    }];
    let mut layer = Layer::new(
        "ambient-solid".into(),
        "Ambient solid".into(),
        LayerType::Solid {
            color: [0.8, 0.8, 0.8, 1.0],
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, 1000.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let center = (HEIGHT as usize / 2) * WIDTH as usize + WIDTH as usize / 2;
    let cpu_center = center * 4;
    assert!(
        cpu[cpu_center] > cpu[cpu_center + 1] + 20,
        "ambient light color should tint the 3D layer"
    );
    for channel in 0..3 {
        let gpu_value = gpu[center][channel] * 255.0;
        let cpu_value = cpu[cpu_center + channel] as f32;
        assert!(
            (gpu_value - cpu_value).abs() <= 2.0,
            "ambient-light GPU/software mismatch in channel {channel}: GPU={gpu_value:.2}, CPU={cpu_value:.2}"
        );
    }
}

#[test]
fn moving_the_3d_camera_moves_a_gpu_layer_in_view_space() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping 3D camera transform test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-camera-view-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([255, 0, 0, 255]))
        .save(&image_path)
        .expect("camera transform fixture should be writable");
    let mut comp = Composition::new(
        "3d-camera-view".into(),
        "3D Camera View".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "view-space-solid".into(),
        "View Space Solid".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, 1000.0]);
    comp.layers.push(layer);

    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let base = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let base_cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    comp.active_camera.transform.position = constant([100.0, 0.0, 0.0]);
    comp.active_camera.transform.rotation = constant([0.0, 10.0, 0.0]);
    kagari_vfx::core::frame_cache::bump_version();
    let moved = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let moved_cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let _ = std::fs::remove_file(&image_path);

    let assert_parity = |cpu: &[u8], gpu: &[[f32; 4]], state: &str| {
        let mut max_error = 0.0f32;
        let mut worst = (0usize, 0usize, 0usize, 0.0f32, 0.0f32);
        for (index, pixel) in gpu.iter().enumerate() {
            for channel in 0..4 {
                let expected = cpu[index * 4 + channel] as f32;
                let actual = pixel[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > max_error {
                    max_error = error;
                    worst = (
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
            max_error <= 3.0,
            "3D camera {state} GPU/software mismatch at ({}, {}) channel {}: CPU {:.1}, GPU {:.1}; max {max_error:.3}",
            worst.0,
            worst.1,
            worst.2,
            worst.3,
            worst.4
        );
    };
    assert_parity(&base_cpu, &base, "baseline");
    assert_parity(&moved_cpu, &moved, "moved");

    let centroid_x = |pixels: &[[f32; 4]]| {
        let visible: Vec<_> = pixels
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0] > 0.1 && p[1] < 0.1)
            .map(|(i, _)| (i % WIDTH as usize) as f32)
            .collect();
        assert!(
            !visible.is_empty(),
            "3D layer should remain visible after camera move"
        );
        visible.iter().sum::<f32>() / visible.len() as f32
    };
    let base_x = centroid_x(&base);
    let moved_x = centroid_x(&moved);
    assert!(
        moved_x < base_x - 2.0,
        "Moving and rotating the camera right should shift the stationary layer left: before {base_x:.2}, after {moved_x:.2}"
    );
}

#[test]
fn a_3d_image_crossing_the_near_plane_matches_gpu_and_software() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping near-plane 3D parity test: no adapter available");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let image_path = std::env::temp_dir().join(format!(
        "kagari-near-plane-{}-{nonce}.png",
        std::process::id()
    ));
    image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([240, 80, 30, 255]))
        .save(&image_path)
        .expect("near-plane fixture should be writable");

    let mut comp = Composition::new(
        "near-plane-3d".into(),
        "Near Plane 3D".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "near-plane-image".into(),
        "Near Plane Image".into(),
        LayerType::Image {
            path: image_path.to_string_lossy().into_owned(),
        },
        1,
    );
    layer.is_3d = true;
    // At this depth, the tilted plane has one half in front of the 0.1 world
    // unit near plane and one half behind it.
    layer.transform_3d.position = constant([0.0, 0.0, 6.4]);
    layer.transform_3d.rotation = constant([20.0, 0.0, 0.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let shadow_map = kagari_vfx::core::software_renderer::build_shadow_map(&comp, 0, WIDTH, HEIGHT);
    renderer.update_shadow_map(&shadow_map, WIDTH, HEIGHT);
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let _ = std::fs::remove_file(&image_path);

    let background = [8u8, 10u8, 13u8];
    assert!(
        cpu.as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != background),
        "software renderer should preserve the visible part of the clipped plane"
    );
    assert!(
        gpu.iter().any(|pixel| {
            (pixel[0] * 255.0 - background[0] as f32).abs() > 2.0
                || (pixel[1] * 255.0 - background[1] as f32).abs() > 2.0
                || (pixel[2] * 255.0 - background[2] as f32).abs() > 2.0
        }),
        "GPU should preserve the visible part of the clipped plane"
    );

    let mut max_error = 0.0f32;
    let mut worst_pixel = (0usize, 0usize, 0usize, 0.0f32, 0.0f32);
    for (index, (expected, actual)) in cpu.as_chunks::<4>().0.iter().zip(gpu.iter()).enumerate() {
        for channel in 0..4 {
            let cpu_value = expected[channel] as f32;
            let gpu_value = actual[channel] * 255.0;
            let error = (cpu_value - gpu_value).abs();
            if error > max_error {
                max_error = error;
                worst_pixel = (
                    index % WIDTH as usize,
                    index / WIDTH as usize,
                    channel,
                    cpu_value,
                    gpu_value,
                );
            }
        }
    }
    assert!(
        max_error <= 3.0,
        "near-plane clipped GPU/software mismatch at ({}, {}) channel {}: CPU {:.2}, GPU {:.2}, full CPU {:?}, full GPU {:?}; max {max_error:.3} 8-bit values",
        worst_pixel.0,
        worst_pixel.1,
        worst_pixel.2,
        worst_pixel.3,
        worst_pixel.4,
        &cpu[((worst_pixel.1 * WIDTH as usize + worst_pixel.0) * 4)..][..4],
        gpu[worst_pixel.1 * WIDTH as usize + worst_pixel.0]
    );
}

#[test]
fn a_3d_layer_behind_the_camera_is_not_rendered_as_2d() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping behind-camera 3D test: no adapter available");
        return;
    };
    let mut comp = Composition::new(
        "behind-camera-3d".into(),
        "Behind Camera 3D".into(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    comp.background_color = [0.03, 0.04, 0.05, 1.0];
    let mut layer = Layer::new(
        "behind-camera-solid".into(),
        "Behind Camera Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    layer.is_3d = true;
    layer.transform_3d.position = constant([0.0, 0.0, -100.0]);
    comp.layers.push(layer);

    let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
    let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
    let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);
    let background = [8u8, 10u8, 13u8];
    assert!(
        cpu.as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[..3] == background),
        "software renderer must not fall back to the unrelated 2D transform"
    );
    assert!(gpu.iter().all(|pixel| {
        (pixel[0] * 255.0 - background[0] as f32).abs() <= 1.0
            && (pixel[1] * 255.0 - background[1] as f32).abs() <= 1.0
            && (pixel[2] * 255.0 - background[2] as f32).abs() <= 1.0
    }));
}

#[test]
fn multi_effect_non_conflicting_chain_matches_software_reference_and_passes_capability_check() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU preview multi-effect test: no adapter available");
        return;
    };
    let mut comp = Composition::new("multi-fx".into(), "multi-fx".into(), WIDTH, HEIGHT, 30, 1);
    comp.background_color = [0.05, 0.05, 0.05, 1.0];
    let mut layer = Layer::new(
        "chain-layer".into(),
        "Chain Layer".into(),
        LayerType::Solid {
            color: [0.35, 0.55, 0.75, 1.0],
        },
        1,
    );
    layer.transform.position = Animatable::new_constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);
    layer.transform.anchor_point =
        Animatable::new_constant([WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5]);

    // Stack effects in the actual shader pipeline order. Lens Flare is inside
    // sample_layer_color, ahead of the fragment's color/levels passes.
    // Slot 1: Lens Flare
    layer.effects.push(Effect {
        id: "fx-flare".into(),
        name: "Lens Flare".into(),
        effect_type: EffectType::LensFlare {
            enabled: constant(1.0),
            position_x: constant(0.5),
            position_y: constant(0.5),
            intensity: constant(0.8),
            threshold: constant(0.9),
            color: constant([1.0, 0.65, 0.25, 1.0]),
            link_to_light: None,
        },
        enabled: true,
    });
    // Slot 2: ColorTint
    layer.effects.push(Effect {
        id: "fx-tint".into(),
        name: "Color Tint".into(),
        effect_type: EffectType::ColorTint {
            color: constant([0.8, 0.2, 0.4, 1.0]),
            intensity: constant(40.0),
        },
        enabled: true,
    });
    // Slot 3: Levels
    layer.effects.push(Effect {
        id: "fx-levels".into(),
        name: "Levels".into(),
        effect_type: EffectType::Levels {
            input_black: constant(0.05),
            input_white: constant(0.95),
            gamma: constant(1.2),
            output_black: constant(0.0),
            output_white: constant(1.0),
        },
        enabled: true,
    });
    // Slot 4: Invert
    layer.effects.push(Effect {
        id: "fx-invert".into(),
        name: "Invert".into(),
        effect_type: EffectType::Invert {
            invert_alpha: false,
        },
        enabled: true,
    });

    comp.layers.push(layer);

    // Verify render_capability allows GPU preview (no CPU fallback triggered!)
    assert!(
        !kagari_vfx::core::render_capability::software_preview_required(&comp, 0),
        "Non-conflicting multi-effect chain should NOT require software preview"
    );
    assert_eq!(
        kagari_vfx::core::render_capability::software_preview_reason(&comp, 0),
        None,
        "No software preview reason should be present for non-conflicting effects"
    );

    for chain_len in 1..=4 {
        comp.layers[0].effects.truncate(chain_len);
        let cpu = render_frame_to_pixels(&comp, 0, WIDTH, HEIGHT, 0.0, 0);
        let mut renderer = WgpuRenderer::new(device.clone(), queue.clone());
        let gpu = gpu_rgba16f_pixels(&device, &queue, &mut renderer, &comp);

        let mut max_error = 0.0f32;
        let mut worst_pixel = (0usize, 0usize, 0usize);
        let mut worst_expected = 0.0f32;
        let mut worst_actual = 0.0f32;
        for (index, actual_pixel) in gpu.iter().enumerate() {
            for channel in 0..4 {
                let expected = cpu[index * 4 + channel] as f32;
                let actual = actual_pixel[channel] * 255.0;
                let error = (expected - actual).abs();
                if error > max_error {
                    max_error = error;
                    worst_pixel = (index % WIDTH as usize, index / WIDTH as usize, channel);
                    worst_expected = expected;
                    worst_actual = actual;
                }
            }
        }
        assert!(
            max_error <= 2.0,
            "Multi-effect prefix length {chain_len} GPU/software mismatch at pixel ({}, {}), channel {}: CPU {worst_expected:.3}, GPU {worst_actual:.3}, max error {max_error:.3}",
            worst_pixel.0, worst_pixel.1, worst_pixel.2
        );
    }
}
