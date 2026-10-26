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
