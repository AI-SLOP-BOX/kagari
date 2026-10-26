#[test]
fn test_visual_headless_pixel_comparison() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
    let layer = Layer::new(
        "l1".to_string(),
        "Solid".to_string(),
        LayerType::Solid {
            color: [0.5, 0.5, 0.5, 1.0],
        },
        30,
    );
    comp.layers.push(layer);

    let p1 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
    let p2 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);

    let mut mse = 0.0f32;
    for i in 0..p1.len() {
        let diff = (p1[i] as f32 - p2[i] as f32) / 255.0;
        mse += diff * diff;
    }
    mse /= p1.len() as f32;

    assert!(
        mse < 1e-5,
        "Visual regression test failed! Pixel MSE ({}) exceeds threshold 1e-5",
        mse
    );
}
#[test]
fn test_invert_effect_changes_pixels() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 20, 20, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Solid".to_string(),
        LayerType::Solid {
            color: [0.8, 0.2, 0.4, 1.0],
        },
        30,
    );
    layer.effects.push(Effect {
        id: "e1".to_string(),
        name: "Invert".to_string(),
        effect_type: EffectType::Invert {
            invert_alpha: false,
        },
        enabled: true,
    });
    comp.layers.push(layer);

    let inverted = render_frame_to_pixels(&comp, 0, 20, 20, 0.0, 0);
    let ci = ((10 * 20 + 10) * 4) as usize;
    // 0.8 -> inverted ~0.2 (255*0.2=51); allow small tolerance.
    assert!(
        (inverted[ci] as i32 - (255 - (0.8 * 255.0) as u8) as i32).abs() <= 3,
        "invert effect did not apply: got {}",
        inverted[ci]
    );
}
#[test]
fn test_twirl_effect_shifts_pixels() {
    use crate::core::cpu_effects;

    // Place a red pixel slightly off-center so the twirl displaces it.
    // (The exact center pixel at r=0 is the twirl axis and doesn't move.)
    let mut buf = vec![0u8; 16 * 16 * 4];
    let px_x = 8;
    let px_y = 10; // 2 pixels below center — r=2, within radius=30
    let idx = ((px_y * 16 + px_x) * 4) as usize;
    buf[idx] = 255;
    buf[idx + 1] = 40;
    buf[idx + 2] = 40;
    buf[idx + 3] = 255;

    let effects = vec![Effect {
        id: "e2".to_string(),
        name: "Twirl".to_string(),
        effect_type: EffectType::Twirl {
            angle: Animatable::new_constant(90.0),
            radius: Animatable::new_constant(30.0),
        },
        enabled: true,
    }];

    cpu_effects::apply_layer_effects(None, None, &mut buf, 16, 16, &effects, 0, 30);

    // The twirl should have moved the red pixel away from (8,10).
    let orig_val = ((px_y * 16 + px_x) * 4) as usize;
    assert_ne!(
        buf[orig_val], 255,
        "red pixel should have moved from original position"
    );

    // A nearby pixel should now be red (displaced).
    let mut found = false;
    for y in 0..16u32 {
        for x in 0..16u32 {
            let i = ((y * 16 + x) * 4) as usize;
            if buf[i] > 200 && buf[i + 1] < 80 && buf[i + 3] > 200 {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(found, "displaced red pixel not found in buffer after twirl");
}
#[test]
fn test_adjustment_layer_applies_effects_to_composite() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
    // Bottom layer: red solid
    comp.layers.push(Layer::new(
        "l1".to_string(),
        "Red".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    ));
    // Top layer: adjustment layer with Invert effect
    let mut adj = Layer::new_adjustment("adj1".to_string(), "Adj Invert".to_string(), 30);
    adj.effects.push(Effect {
        id: "inv".to_string(),
        name: "Invert".to_string(),
        effect_type: EffectType::Invert {
            invert_alpha: false,
        },
        enabled: true,
    });
    comp.layers.push(adj);

    let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
    // Red (255,0,0) inverted should be (0,255,255)
    let idx = (5 * 10 + 5) * 4;
    assert_eq!(pixels[idx], 0, "R should be inverted");
    assert_eq!(pixels[idx + 1], 255, "G should be 255");
    assert_eq!(pixels[idx + 2], 255, "B should be 255");
}
#[test]
fn test_fractal_noise_generates_output() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Solid".to_string(),
        LayerType::Solid {
            color: [0.5, 0.5, 0.5, 1.0],
        },
        30,
    );
    layer.effects.push(Effect {
        id: "fn1".to_string(),
        name: "FractalNoise".to_string(),
        effect_type: EffectType::FractalNoise {
            fractal_type: Animatable::new_constant(0.0),
            contrast: Animatable::new_constant(100.0),
            brightness: Animatable::new_constant(0.0),
            complexity: Animatable::new_constant(3.0),
            evolution: Animatable::new_constant(0.0),
        },
        enabled: true,
    });
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(pixels.len(), 32 * 32 * 4);

    // At least some pixels should differ from the original gray
    let mut non_gray = 0;
    for p in pixels.as_chunks::<4>().0 {
        if (p[0] as i32 - p[1] as i32).abs() > 5
            || (p[1] as i32 - p[2] as i32).abs() > 5
            || (p[0] as i32 - 128).abs() > 20
        {
            non_gray += 1;
        }
    }
    assert!(
        non_gray > 0,
        "FractalNoise should produce varied pixel values"
    );
}
#[test]
fn test_background_colour_is_composited() {
    let mut comp = Composition::new("c1".to_string(), "BgComp".to_string(), 8, 8, 30, 30);
    comp.background_color = [1.0, 0.0, 0.0, 1.0];

    let pixels = render_frame_to_pixels(&comp, 0, 8, 8, 0.0, 0);
    assert_eq!(pixels.len(), 8 * 8 * 4);
    for p in pixels.as_chunks::<4>().0 {
        assert_eq!(p[0], 255, "background red must fill an empty composition");
        assert_eq!(p[3], 255, "background alpha must be opaque");
    }
}
#[test]
fn test_time_remap_shifts_layer_time() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Red".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    // Set time remap: at frame 0, remap to frame 10
    layer.time_remap = Some(crate::core::property::Animatable::new_constant(10.0));
    comp.layers.push(layer);

    // At frame 0, the layer should evaluate at frame 10 (still active, since duration=30)
    let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
    // Should still have red pixels since layer is active at frame 10
    let idx = (5 * 10 + 5) * 4;
    assert!(
        pixels[idx] > 200,
        "Time remapped layer should still render at frame 0"
    );
}
#[test]
fn test_softlight_blend_uses_both_src_and_dst() {
    // Create a red layer over a green layer with SoftLight blend
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
    // Bottom: green solid
    let mut bottom = Layer::new(
        "l1".to_string(),
        "Green".to_string(),
        LayerType::Solid {
            color: [0.0, 1.0, 0.0, 1.0],
        },
        30,
    );
    bottom.blend_mode = BlendMode::Normal;
    comp.layers.push(bottom);
    // Top: red solid with SoftLight
    let mut top = Layer::new(
        "l2".to_string(),
        "Red".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    top.blend_mode = BlendMode::SoftLight;
    comp.layers.push(top);

    let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
    let idx = (5 * 10 + 5) * 4;
    // SoftLight with src=red(1,0,0) over dst=green(0,1,0):
    // Red channel: s=1.0 > 0.5, d=0.0, a=0.0 → d + (2*s-1)*(a-d) = 0 + 1*(0-0) = 0
    // Green channel: s=0.0 <= 0.5, d=1.0 → d - (1-2*s)*d*(1-d) = 1 - 1*1*0 = 1
    // Blue channel: both 0 → 0
    // Result should be (0, 1, 0) which is green (unchanged since src red doesn't affect green via SoftLight)
    assert!(
        pixels[idx + 1] > 200,
        "Green channel should be preserved by SoftLight"
    );
}
#[test]
fn test_shape_ellipse_renders_pixels() {
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Ellipse".to_string(),
        LayerType::Shape {
            shape_type: crate::core::timeline::ShapeType::Ellipse {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    // Center pixel should be red (shape color)
    let center = (32 * 64 + 32) * 4;
    assert!(
        pixels[center] > 200,
        "Center of ellipse should be red (R={})",
        pixels[center]
    );
    // Corner pixel should be background color, not red
    let corner = 0;
    assert!(
        pixels[corner] < 50,
        "Corner should be background color (R={})",
        pixels[corner]
    );
}
#[test]
fn test_shape_rectangle_renders_pixels() {
    use crate::core::timeline::ShapeType;
    let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
    let mut layer = Layer::new(
        "l1".to_string(),
        "Rect".to_string(),
        LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
                corner_radius: crate::core::property::Animatable::new_constant(0.0),
            },
            color: [0.0, 1.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let center = (32 * 64 + 32) * 4;
    assert!(
        pixels[center + 1] > 200,
        "Center of rectangle should be green"
    );
}
