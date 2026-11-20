use super::*;

#[test]
fn column_major_matrix_multiplication_preserves_transform_order() {
    let scale = [
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let translate = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [5.0, 7.0, 0.0, 1.0],
    ];

    let combined = mat4_mul(translate, scale);
    assert_eq!(combined[0][0], 2.0);
    assert_eq!(combined[1][1], 3.0);
    assert_eq!(combined[3][0], 5.0);
    assert_eq!(combined[3][1], 7.0);
}

#[test]
fn test_layer_uniform_memory_alignment() {
    let size = std::mem::size_of::<LayerUniform>();
    let stride = LAYER_UNIFORM_STRIDE;
    assert!(size <= stride);
    assert_eq!(
        stride % 256,
        0,
        "Layer uniform stride ({} bytes) must be a multiple of 256 for WGPU dynamic uniform offset alignment",
        stride
    );
    for align in [64, 256, 512] {
        assert_eq!(
            align_uniform_buffer_offset(stride as u64, align) % align as u64,
            0
        );
    }
}

#[test]
fn composition_cache_key_is_deterministic_and_separates_comps() {
    let first = Composition::new("comp-a".into(), "A".into(), 16, 16, 30, 1);
    let mut changed = first.clone();
    changed.name = "B".into();
    assert_eq!(composition_cache_key(&first), composition_cache_key(&first));
    assert_ne!(
        composition_cache_key(&first),
        composition_cache_key(&changed)
    );
}

#[test]
fn composition_cache_key_changes_when_layer_blend_mode_changes() {
    let mut normal = Composition::new("blend-key".into(), "Blend Key".into(), 16, 16, 30, 1);
    normal.layers.push(crate::core::timeline::Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [0.2, 0.4, 0.6, 0.5],
        },
        1,
    ));
    let mut screen = normal.clone();
    screen.layers[0].blend_mode = crate::core::timeline::BlendMode::Screen;

    assert_ne!(
        composition_cache_key(&normal),
        composition_cache_key(&screen)
    );
}

#[test]
fn motion_velocity_tracks_animated_layer_position() {
    let mut comp = Composition::new("comp".into(), "Motion".into(), 64, 64, 30, 60);
    let mut layer = crate::core::timeline::Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        60,
    );
    layer.transform.position = crate::core::property::Animatable::new_animated(vec![
        crate::core::keyframe::Keyframe::new(
            0,
            [10.0, 20.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
        crate::core::keyframe::Keyframe::new(
            30,
            [40.0, 20.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
    ]);
    comp.layers.push(layer);

    let velocity = layer_motion_velocity(&comp, &comp.layers[0], 15);
    assert!((velocity[0] - 1.0).abs() < 1e-5);
    assert!(velocity[1].abs() < 1e-5);
}

#[test]
fn gpu_draw_order_uses_composition_time_depth_and_keeps_2d_stable() {
    let mut comp = Composition::new("comp".into(), "Depth order".into(), 64, 64, 30, 30);
    let mut far = Layer::new(
        "far".into(),
        "Far".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    far.is_3d = true;
    far.transform_3d.position = crate::core::property::Animatable::new_animated(vec![
        crate::core::keyframe::Keyframe::new(
            0,
            [32.0, 32.0, 10.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
        crate::core::keyframe::Keyframe::new(
            20,
            [32.0, 32.0, 90.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
    ]);
    far.freeze_at(20);

    let mut near = Layer::new(
        "near".into(),
        "Near".into(),
        LayerType::Solid {
            color: [0.0, 1.0, 0.0, 1.0],
        },
        30,
    );
    near.is_3d = true;
    near.transform_3d.position =
        crate::core::property::Animatable::new_constant([32.0, 32.0, 50.0]);

    comp.layers.push(far);
    comp.layers.push(near);
    let layers: Vec<&Layer> = comp.layers.iter().collect();
    assert_eq!(gpu_layer_draw_order(&comp, &layers, 0), vec![0, 1]);
}

#[cfg(feature = "cli-gpu")]
#[test]
fn gpu_renders_multiply_stroked_shape_expanded_mask_and_dither() {
    let Ok(mut gpu) = crate::core::headless_gpu::HeadlessGpuRenderer::new() else {
        return;
    };
    let mut comp = Composition::new(
        "gpu-parity".into(),
        "GPU Parity".into(),
        64,
        64,
        30,
        10,
    );
    comp.background_color = [0.5, 0.5, 0.5, 1.0];
    comp.dither_output = true;
    comp.working_color_space = crate::core::color_science::WorkingColorSpace::DisplayP3;

    let mut solid = Layer::new(
        "mult".into(),
        "Multiply Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.5, 0.5, 1.0],
        },
        10,
    );
    solid.blend_mode = crate::core::timeline::BlendMode::Multiply;
    comp.layers.push(solid);

    let mut shape = Layer::new(
        "shape".into(),
        "Stroked Shape".into(),
        LayerType::Shape {
            shape_type: crate::core::timeline::ShapeType::Rectangle {
                width: crate::core::property::Animatable::new_constant(32.0),
                height: crate::core::property::Animatable::new_constant(32.0),
                corner_radius: crate::core::property::Animatable::new_constant(6.0),
            },
            color: [0.2, 0.4, 0.8, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 4.0,
            fill_type: crate::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        10,
    );
    shape.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
    let mut mask = crate::core::mask::Mask::new_rect(
        "m1".into(),
        "Mask 1".into(),
        8.0,
        8.0,
        48.0,
        48.0,
    );
    mask.expansion = crate::core::property::Animatable::new_constant(2.0);
    shape.masks.push(mask);
    comp.layers.push(shape);

    assert!(!crate::core::render_capability::software_preview_required(
        &comp, 0
    ));
    let rgba = gpu
        .render_rgba8(&comp, 0, 0.0, 0)
        .expect("headless GPU render should succeed");
    assert_eq!(rgba.len(), 64 * 64 * 4);

    // Corner pixel (2, 2) should reflect Multiply over 0.5 gray background (approx R=128, G=64, B=64)
    let corner_idx = (2 * 64 + 2) * 4;
    assert!(
        rgba[corner_idx] > rgba[corner_idx + 1] + 20,
        "Expected red-dominant multiply result at corner, got {:?}",
        &rgba[corner_idx..corner_idx + 4]
    );
    // Center pixel (32, 32) should reflect blue-dominant shape fill
    let center_idx = (32 * 64 + 32) * 4;
    assert!(
        rgba[center_idx + 2] > rgba[center_idx] + 40,
        "Expected blue-dominant shape fill at center, got {:?}",
        &rgba[center_idx..center_idx + 4]
    );
}

#[cfg(feature = "cli-gpu")]
#[test]
fn gpu_renders_programmable_blend_modes_with_cpu_parity() {
    let Ok(mut gpu) = crate::core::headless_gpu::HeadlessGpuRenderer::new() else {
        return;
    };
    for mode in [
        crate::core::timeline::BlendMode::Overlay,
        crate::core::timeline::BlendMode::Difference,
        crate::core::timeline::BlendMode::Darken,
        crate::core::timeline::BlendMode::Lighten,
        crate::core::timeline::BlendMode::SoftLight,
        crate::core::timeline::BlendMode::Exclusion,
        crate::core::timeline::BlendMode::Color,
        crate::core::timeline::BlendMode::Hue,
        crate::core::timeline::BlendMode::Saturation,
        crate::core::timeline::BlendMode::Luminosity,
    ] {
        let mut comp = Composition::new(
            format!("blend-{mode:?}"),
            "Blend Parity".into(),
            32,
            32,
            30,
            10,
        );
        comp.background_color = [0.6, 0.4, 0.2, 1.0];
        let mut layer = Layer::new(
            "top".into(),
            "Top".into(),
            LayerType::Solid {
                color: [0.3, 0.8, 0.5, 0.75],
            },
            10,
        );
        layer.blend_mode = mode;
        comp.layers.push(layer);

        assert!(!crate::core::render_capability::software_preview_required(
            &comp, 0
        ));
        let gpu_rgba = gpu
            .render_rgba8(&comp, 0, 0.0, 0)
            .expect("GPU blend render should succeed");
        let cpu_rgba =
            crate::core::software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let center = (16 * 32 + 16) * 4;
        for ch in 0..4 {
            let diff = (gpu_rgba[center + ch] as i32 - cpu_rgba[center + ch] as i32).abs();
            assert!(
                diff <= 3,
                "GPU/CPU mismatch for {mode:?} ch={ch}: gpu={} cpu={}",
                gpu_rgba[center + ch],
                cpu_rgba[center + ch]
            );
        }
    }
}

#[cfg(feature = "cli-gpu")]
#[test]
fn gpu_renders_adjustment_layer_with_cpu_parity() {
    let Ok(mut gpu) = crate::core::headless_gpu::HeadlessGpuRenderer::new() else {
        return;
    };
    let mut comp = Composition::new(
        "adj-parity".into(),
        "Adjustment Parity".into(),
        32,
        32,
        30,
        10,
    );
    comp.background_color = [0.2, 0.4, 0.8, 1.0];
    let mut adj = Layer::new(
        "adj".into(),
        "Adjustment".into(),
        LayerType::AdjustmentLayer,
        10,
    );
    adj.effects.push(crate::core::timeline::Effect {
        id: "tint".into(),
        name: "Color Tint".into(),
        effect_type: crate::core::timeline::EffectType::ColorTint {
            color: crate::core::property::Animatable::new_constant([1.0, 0.2, 0.2, 1.0]),
            intensity: crate::core::property::Animatable::new_constant(50.0),
        },
        enabled: true,
    });
    comp.layers.push(adj);

    assert!(!crate::core::render_capability::software_preview_required(
        &comp, 0
    ));
    let gpu_rgba = gpu
        .render_rgba8(&comp, 0, 0.0, 0)
        .expect("GPU adjustment layer render should succeed");
    let cpu_rgba =
        crate::core::software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    let center = (16 * 32 + 16) * 4;
    for ch in 0..4 {
        let diff = (gpu_rgba[center + ch] as i32 - cpu_rgba[center + ch] as i32).abs();
        assert!(
            diff <= 3,
            "GPU/CPU mismatch for AdjustmentLayer ch={ch}: gpu={} cpu={}",
            gpu_rgba[center + ch],
            cpu_rgba[center + ch]
        );
    }
}
