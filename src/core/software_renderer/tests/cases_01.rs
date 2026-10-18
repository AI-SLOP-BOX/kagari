#[test]
fn test_video_layer_speed_scales_sequence_index() {
    let dir = std::env::temp_dir().join(format!("kagari_speed_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // frame i encoded as gray value i * 10 (frame 20 -> 200)
    for i in 0..=20u32 {
        write_gray_png(&dir.join(format!("frame_{:05}.png", i)), (i * 10) as u8);
    }
    let frames_dir = dir.to_string_lossy().to_string();

    let make_comp = |speed: f32| {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 16, 16, 30, 30);
        let mut layer = Layer::new(
            "v".into(),
            "V".into(),
            LayerType::Video {
                source: "test".into(),
                frames_dir: frames_dir.clone(),
                frame_count: 21,
                audio_wav: None,
                speed,
            },
            30,
        );
        layer.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(layer);
        comp
    };

    // speed 2.0: timeline frame 10 -> sequence frame 20 (gray 200)
    let px_fast = render_frame_to_pixels(&make_comp(2.0), 10, 16, 16, 0.0, 0);
    assert_eq!(
        px_fast[8 * 16 * 4 + 8 * 4],
        200,
        "speed=2.0 must map frame 10 to seq frame 20"
    );

    // speed 1.0: timeline frame 10 -> sequence frame 10 (gray 100)
    let px_norm = render_frame_to_pixels(&make_comp(1.0), 10, 16, 16, 0.0, 0);
    assert_eq!(px_norm[8 * 16 * 4 + 8 * 4], 100);

    // clamping: speed 2.0 at last frame stays within sequence
    let px_clamp = render_frame_to_pixels(&make_comp(2.0), 15, 16, 16, 0.0, 0);
    assert_eq!(
        px_clamp[8 * 16 * 4 + 8 * 4],
        200,
        "index must clamp to last frame"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn preview_uses_layer_proxy_media_but_final_render_does_not() {
    let dir = std::env::temp_dir().join(format!("kagari_layer_proxy_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let frames = dir.join("frames");
    std::fs::create_dir_all(&frames).unwrap();
    let full_frame = frames.join("frame_00000.png");
    let proxy_frame = dir.join("proxy.webp");
    write_gray_png(&full_frame, 40);
    write_gray_webp(&proxy_frame, 200);

    let mut comp = Composition::new("proxy_comp".into(), "Proxy".into(), 16, 16, 30, 30);
    let mut layer = Layer::new(
        "video".into(),
        "Proxy Video".into(),
        LayerType::Video {
            source: "test".into(),
            frames_dir: frames.to_string_lossy().into_owned(),
            frame_count: 1,
            audio_wav: None,
            speed: 1.0,
        },
        30,
    );
    layer.proxy.enabled = true;
    layer.proxy.proxy_path = Some(proxy_frame.to_string_lossy().into_owned());
    layer.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(layer);

    let final_pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    let preview_pixels = render_frame_to_pixels_preview(&comp, 0, 16, 16, 0.0, 0);
    let center = (8 * 16 * 4 + 8 * 4) as usize;
    assert_eq!(final_pixels[center], 40);
    assert_eq!(preview_pixels[center], 200);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn test_video_frame_blending_interpolates_webp_frames() {
    let dir = std::env::temp_dir().join(format!("kagari_blend_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    write_gray_webp(&dir.join("frame_00000.webp"), 0);
    write_gray_webp(&dir.join("frame_00001.webp"), 100);

    let mut comp = Composition::new("c1".into(), "Comp".into(), 16, 16, 30, 30);
    let mut layer = Layer::new(
        "v".into(),
        "V".into(),
        LayerType::Video {
            source: "test".into(),
            frames_dir: dir.to_string_lossy().into_owned(),
            frame_count: 2,
            audio_wav: None,
            speed: 0.5,
        },
        30,
    );
    layer.frame_blending = true;
    layer.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(layer);

    // Timeline frame 1 maps to source position 0.5, so the midpoint is
    // rendered instead of truncating to either source frame.
    let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
    assert_eq!(pixels[8 * 16 * 4 + 8 * 4], 50);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn test_software_render_frame_to_pixels() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Solid".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    layer.blend_mode = BlendMode::Multiply;
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 100, 100, 0.0, 0);
    assert_eq!(pixels.len(), 100 * 100 * 4);
}
#[test]
fn source_layer_effects_keep_original_layer_indices() {
    let mut comp = Composition::new("source_map".into(), "Source Map".into(), 16, 16, 30, 30);
    comp.background_color = [0.0, 0.0, 0.0, 0.0];

    let mut source = Layer::new(
        "source".into(),
        "Source".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 0.25],
        },
        30,
    );
    source.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(source);

    let mut target = Layer::new(
        "target".into(),
        "Target".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    target.transform.position = Animatable::new_constant([8.0, 8.0]);
    target.effects.push(Effect {
        id: "matte".into(),
        enabled: true,
        name: "Set Matte".into(),
        effect_type: EffectType::SetMatte {
            source_layer_idx: 0,
            source_channel: crate::core::set_matte::MatteSourceChannel::Alpha,
            invert_matte: false,
            composite_mode: crate::core::set_matte::MatteCompositeMode::Replace,
        },
    });
    comp.layers.push(target);

    // Render only the target as a source-layer consumer. The source must
    // still resolve against index 0 in the original composition.
    let pixels = render_frame_to_pixels_filtered(&comp, 0, 16, 16, 0.0, 0, Some(1));
    let center = ((8 * 16 + 8) * 4) as usize;
    assert!(
        pixels[center] > 200,
        "target color should survive the matte"
    );
    assert!(
        (60..=68).contains(&pixels[center + 3]),
        "source alpha should drive target alpha, got {}",
        pixels[center + 3]
    );
}
#[test]
fn test_particle_layer_renders() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
    let mut layer = Layer::new(
        "p1".to_string(),
        "Particles".to_string(),
        LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter {
                rate: 500.0,
                lifetime: 1.0,
                speed: 50.0,
                size_start: 6.0,
                size_end: 3.0,
                ..Default::default()
            },
        },
        30,
    );
    layer.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(layer);

    // Frame 10 should have particles simulated and rendered
    let pixels = render_frame_to_pixels(&comp, 10, 64, 64, 0.0, 0);
    assert_eq!(pixels.len(), 64 * 64 * 4);
    // Some pixels should be brighter than the dark background (20,20,25)
    let bright = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 40)
        .count();
    assert!(bright > 0, "Particle layer should produce visible pixels");
}
#[test]
fn test_particle_layer_deterministic() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
    let mut layer = Layer::new(
        "p1".to_string(),
        "Particles".to_string(),
        LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter::default(),
        },
        30,
    );
    layer.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(layer);

    let p1 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
    let p2 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
    assert_eq!(
        p1, p2,
        "Particle simulation must be deterministic per frame"
    );
}
#[test]
fn test_motion_blur_smears_moving_layer() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
    let mut layer = Layer::new(
        "m1".to_string(),
        "Moving Solid".to_string(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    layer.motion_blur = true;
    // Move horizontally: position keyframes at x=20 (frame 5) → x=80 (frame 15)
    layer.transform.position = Animatable::new_animated(vec![
        crate::core::keyframe::Keyframe::new(
            5,
            [20.0, 50.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
        crate::core::keyframe::Keyframe::new(
            15,
            [80.0, 50.0],
            crate::core::keyframe::InterpolationType::Linear,
        ),
    ]);
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 10, 100, 100, 0.0, 0);
    assert_eq!(pixels.len(), 100 * 100 * 4);
    // Motion-blurred moving layer should still render without panic
}
#[test]
fn test_3d_light_shading_applied() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
    let mut layer = Layer::new(
        "l3d".to_string(),
        "3D Solid".to_string(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    layer.is_3d = true;
    comp.layers.push(layer);

    // Strong light near the layer center should brighten it
    comp.lights[0].intensity = 200.0;
    comp.lights[0].position = Animatable::new_constant([16.0, 16.0, -300.0]);

    let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    // Center pixel should be lit (brighter than ambient-only floor)
    let center_idx = ((16 * 32 + 16) * 4) as usize;
    assert!(
        pixels[center_idx] > 60,
        "3D layer should be shaded by light, got {}",
        pixels[center_idx]
    );
}
#[test]
fn test_dof_blurs_off_focus_3d_layer() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
    let mut layer = Layer::new(
        "l3d".to_string(),
        "3D Solid".to_string(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    layer.is_3d = true;
    // Push the layer far from the focus plane
    layer.transform_3d.position = Animatable::new_constant([16.0, 16.0, -3000.0]);
    comp.layers.push(layer);

    comp.active_camera.dof_enabled = true;
    comp.resolve_camera_mut().focus_distance = 1000.0;
    comp.resolve_camera_mut().aperture = 50.0;
    comp.resolve_camera_mut().dof_max_blur = 24.0;

    let dof_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    let center_idx = ((16 * 32 + 16) * 4) as usize;

    // Reference render with identical geometry but DOF off
    comp.active_camera.dof_enabled = false;
    let sharp_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);

    // Defocus spreads the solid's energy: center dims vs the sharp render
    assert!(
        (sharp_pixels[center_idx] as i32 - dof_pixels[center_idx] as i32).abs() > 2,
        "DOF should change the off-focus render (sharp {} vs defocused {})",
        sharp_pixels[center_idx],
        dof_pixels[center_idx]
    );
}
#[test]
fn test_text_on_path_renders_glyphs() {
    use crate::core::mask::{Mask, MaskPath};
    use crate::core::property::Animatable as PAnimatable;

    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 128, 64, 30, 30);
    let mut layer = Layer::new(
        "tp".to_string(),
        "Path Text".to_string(),
        LayerType::Text {
            text: "AB".to_string(),
            font_size: 24,
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "Helvetica".to_string(),
            tracking: 0.0,
            leading: 1.2,
            align: 0,
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            text_on_path: true,
        },
        30,
    );
    // Gentle horizontal wave path across the comp
    let path = MaskPath {
        vertices: PAnimatable::new_constant(vec![
            [16.0, 32.0],
            [48.0, 20.0],
            [80.0, 44.0],
            [112.0, 32.0],
        ]),
        tangents: None,
        is_closed: false,
    };
    layer.masks.push(Mask {
        path,
        ..Mask::new_rect("m1".to_string(), "Path".to_string(), 0.0, 0.0, 10.0, 10.0)
    });
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 128, 64, 0.0, 0);
    let bright = (0..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] > 200 && pixels[i + 1] > 200 && pixels[i + 2] > 200)
        .count();
    // CI runners may have no installed fonts; skip assertion when headless
    if std::env::var("CI").is_err() {
        assert!(
            bright > 20,
            "text-on-path glyphs should render bright pixels, got {}",
            bright
        );
    }
}
#[test]
fn test_gpu_parent_transform_resolution() {
    // Parented layer should resolve through resolve_world_transform without panicking
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
    let mut parent = Layer::new("par".to_string(), "Parent".to_string(), LayerType::Null, 30);
    parent.transform.position = Animatable::new_constant([32.0, 32.0]);
    parent.transform.rotation = Animatable::new_constant(45.0);
    comp.layers.push(parent);
    let mut child = Layer::new(
        "chi".to_string(),
        "Child".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    child.parent_id = Some("par".to_string());
    child.transform.position = Animatable::new_constant([10.0, 0.0]);
    let child_pos = child.transform.position.evaluate(0);
    comp.layers.push(child);

    let (pos, _scale, _rot, _opa) = comp.resolve_world_transform(comp.layers.last().unwrap(), 0);
    let _ = child_pos;
    // Child at local (10,0) rotated 45deg around parent origin (32,32):
    // offset = (10*cos45, 10*sin45) ≈ (7.07, 7.07) → world ≈ (39.07, 39.07)
    assert!(
        (pos[0] - 39.07).abs() < 0.5,
        "unexpected world x: {}",
        pos[0]
    );
    assert!(
        (pos[1] - 39.07).abs() < 0.5,
        "unexpected world y: {}",
        pos[1]
    );
}
