#[test]
fn test_precomp_nested_rendering() {
    crate::core::frame_cache::bump_version();
    // Sub-comp: red ellipse at its own center
    let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
    let mut shape = Layer::new(
        "s1".to_string(),
        "Dot".to_string(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(40.0),
                height: Animatable::new_constant(40.0),
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
    shape.transform.position = Animatable::new_constant([32.0, 32.0]);
    sub.layers.push(shape);

    // Main comp: pre-comp layer referencing the sub-comp
    let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
    comp.sub_compositions.push(sub);
    let mut pc = Layer::new(
        "pc".to_string(),
        "Nested".to_string(),
        LayerType::PreComp {
            comp_id: "sub".to_string(),
        },
        30,
    );
    pc.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(pc);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let center = ((32 * 64 + 32) * 4) as usize;
    assert!(
        pixels[center] > 180,
        "PreComp should render nested shape (R={})",
        pixels[center]
    );
}
#[test]
fn test_precomp_essential_property_override_reaches_nested_render() {
    let mut sub = Composition::new(
        "essential_sub".into(),
        "Essential Sub".into(),
        16,
        16,
        30,
        30,
    );
    sub.background_color = [0.0, 0.0, 0.0, 0.0];
    let mut solid = Layer::new(
        "solid".into(),
        "Color Target".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    solid.transform.position = Animatable::new_constant([8.0, 8.0]);
    sub.layers.push(solid);

    let mut comp = Composition::new(
        "essential_main".into(),
        "Essential Main".into(),
        16,
        16,
        30,
        30,
    );
    comp.background_color = [0.0, 0.0, 0.0, 0.0];
    comp.sub_compositions.push(sub);
    let mut precomp = Layer::new(
        "precomp".into(),
        "Essential Instance".into(),
        LayerType::PreComp {
            comp_id: "essential_sub".into(),
        },
        30,
    );
    precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
    precomp
        .essential_properties
        .push(crate::core::essential_properties::EssentialProperty {
            name: "Color".into(),
            prop_type: crate::core::essential_properties::EssentialPropertyType::Color,
            value: crate::core::essential_properties::EssentialValue::Color([0.0, 1.0, 0.0, 1.0]),
            overridden: true,
            min_value: 0.0,
            max_value: 100.0,
            options: Vec::new(),
        });
    comp.layers.push(precomp);

    let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    let center = ((8 * 16 + 8) * 4) as usize;
    assert!(
        pixels[center + 1] > 180 && pixels[center] < 40,
        "Essential Property color should reach nested render: {:?}",
        &pixels[center..center + 4]
    );
}
#[test]
fn test_precomp_recurses_through_two_nested_compositions() {
    let mut inner = Composition::new("inner".into(), "Inner".into(), 32, 32, 30, 30);
    inner.background_color = [0.0, 0.0, 0.0, 0.0];
    let mut solid = Layer::new(
        "solid".into(),
        "Red".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    solid.transform.position = Animatable::new_constant([16.0, 16.0]);
    inner.layers.push(solid);

    let mut middle = Composition::new("middle".into(), "Middle".into(), 32, 32, 30, 30);
    middle.background_color = [0.0, 0.0, 0.0, 0.0];
    middle.sub_compositions.push(inner);
    let mut inner_layer = Layer::new(
        "inner-ref".into(),
        "Inner PreComp".into(),
        LayerType::PreComp {
            comp_id: "inner".into(),
        },
        30,
    );
    inner_layer.transform.position = Animatable::new_constant([16.0, 16.0]);
    middle.layers.push(inner_layer);

    let mut outer = Composition::new("outer".into(), "Outer".into(), 32, 32, 30, 30);
    outer.background_color = [0.0, 0.0, 0.0, 0.0];
    outer.sub_compositions.push(middle);
    let mut middle_layer = Layer::new(
        "middle-ref".into(),
        "Middle PreComp".into(),
        LayerType::PreComp {
            comp_id: "middle".into(),
        },
        30,
    );
    middle_layer.transform.position = Animatable::new_constant([16.0, 16.0]);
    outer.layers.push(middle_layer);

    let pixels = render_frame_to_pixels(&outer, 0, 32, 32, 0.0, 0);
    let center = ((16 * 32 + 16) * 4) as usize;
    assert!(
        pixels[center] > 180 && pixels[center + 3] > 180,
        "two-level PreComp should preserve nested pixels: {:?}",
        &pixels[center..center + 4]
    );
}
#[test]
fn test_precomp_applies_parent_effects_before_compositing() {
    let mut sub = Composition::new("pre_fx_sub".into(), "Pre FX Sub".into(), 16, 16, 30, 30);
    sub.background_color = [0.0, 0.0, 0.0, 0.0];
    let mut red = Layer::new(
        "red".into(),
        "Red".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    red.transform.position = Animatable::new_constant([8.0, 8.0]);
    sub.layers.push(red);

    let mut comp = Composition::new("pre_fx_main".into(), "Pre FX Main".into(), 16, 16, 30, 30);
    comp.sub_compositions.push(sub);
    let mut precomp = Layer::new(
        "pre_fx".into(),
        "Pre FX".into(),
        LayerType::PreComp {
            comp_id: "pre_fx_sub".into(),
        },
        30,
    );
    precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
    precomp.effects.push(crate::core::timeline::Effect {
        id: "invert_parent".into(),
        enabled: true,
        name: "Invert".into(),
        effect_type: crate::core::timeline::EffectType::Invert {
            invert_alpha: false,
        },
    });
    comp.layers.push(precomp);

    let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    let center = ((8 * 16 + 8) * 4) as usize;
    assert!(
        pixels[center] < 40 && pixels[center + 1] > 200 && pixels[center + 2] > 200,
        "parent PreComp effect should invert nested pixels: {:?}",
        &pixels[center..center + 4]
    );
}
#[test]
fn test_precomp_video_uses_sequence_and_frame_blending() {
    let dir =
        std::env::temp_dir().join(format!("kagari_precomp_video_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    write_gray_webp(&dir.join("frame_00000.webp"), 0);
    write_gray_webp(&dir.join("frame_00001.webp"), 100);

    let mut sub = Composition::new("sub_video".into(), "Sub Video".into(), 16, 16, 30, 30);
    let mut video = Layer::new(
        "video".into(),
        "Video".into(),
        LayerType::Video {
            source: "test".into(),
            frames_dir: dir.to_string_lossy().into_owned(),
            frame_count: 2,
            audio_wav: None,
            speed: 0.5,
        },
        30,
    );
    video.frame_blending = true;
    video.transform.position = Animatable::new_constant([8.0, 8.0]);
    sub.layers.push(video);

    let mut comp = Composition::new("main_video".into(), "Main".into(), 16, 16, 30, 30);
    comp.sub_compositions.push(sub);
    let mut precomp = Layer::new(
        "pc".into(),
        "Nested Video".into(),
        LayerType::PreComp {
            comp_id: "sub_video".into(),
        },
        30,
    );
    precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(precomp);

    let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
    assert_eq!(pixels[8 * 16 * 4 + 8 * 4], 50);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn precomp_proxy_cache_is_separate_from_final_cache() {
    let dir = std::env::temp_dir().join(format!(
        "kagari_precomp_proxy_cache_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let frames = dir.join("frames");
    std::fs::create_dir_all(&frames).unwrap();
    write_gray_png(&frames.join("frame_00000.png"), 40);
    let proxy = dir.join("proxy.webp");
    write_gray_webp(&proxy, 200);

    let mut sub = Composition::new("proxy_sub".into(), "Proxy Sub".into(), 16, 16, 30, 30);
    let mut video = Layer::new(
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
    video.proxy.enabled = true;
    video.proxy.proxy_path = Some(proxy.to_string_lossy().into_owned());
    video.transform.position = Animatable::new_constant([8.0, 8.0]);
    sub.layers.push(video);

    let mut comp = Composition::new("proxy_outer".into(), "Proxy Outer".into(), 16, 16, 30, 30);
    comp.sub_compositions.push(sub);
    let mut precomp = Layer::new(
        "proxy_ref".into(),
        "Proxy PreComp".into(),
        LayerType::PreComp {
            comp_id: "proxy_sub".into(),
        },
        30,
    );
    precomp.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(precomp);

    let final_before = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    let preview = render_frame_to_pixels_preview(&comp, 0, 16, 16, 0.0, 0);
    let final_after = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    let center = (8 * 16 * 4 + 8 * 4) as usize;
    assert_eq!(final_before[center], 40);
    assert_eq!(preview[center], 200);
    assert_eq!(final_after[center], 40);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn test_video_frame_blending_works_for_luma_matte() {
    let dir = std::env::temp_dir().join(format!("kagari_video_matte_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    write_gray_webp(&dir.join("frame_00000.webp"), 0);
    write_gray_webp(&dir.join("frame_00001.webp"), 255);

    let mut comp = Composition::new("video_matte".into(), "Video Matte".into(), 16, 16, 30, 30);
    comp.background_color = [0.0, 0.0, 0.0, 0.0];
    let mut matte = Layer::new(
        "matte".into(),
        "Matte".into(),
        LayerType::Video {
            source: "test".into(),
            frames_dir: dir.to_string_lossy().into_owned(),
            frame_count: 2,
            audio_wav: None,
            speed: 0.5,
        },
        30,
    );
    matte.frame_blending = true;
    matte.transform.position = Animatable::new_constant([8.0, 8.0]);
    comp.layers.push(matte);

    let mut content = Layer::new(
        "content".into(),
        "Content".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    content.transform.position = Animatable::new_constant([8.0, 8.0]);
    content.track_matte = TrackMatteMode::LumaMatte;
    comp.layers.push(content);

    let matte = render_matte_buffer(&comp, &comp.layers[1], 1, 16, 16).unwrap();
    let matte_center = (8 * 16 * 4 + 8 * 4) as usize;
    assert!((120..=136).contains(&matte[matte_center]));

    let pixels = render_frame_to_pixels(&comp, 1, 16, 16, 0.0, 0);
    let center = (8 * 16 * 4 + 8 * 4) as usize;
    assert!(
        (120..=136).contains(&pixels[center + 3]),
        "expected half matte alpha, got {}",
        pixels[center + 3]
    );

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn test_precomp_self_reference_cycle_is_safe() {
    // A comp whose pre-comp layer references ITSELF must render empty
    // (cycle guard) instead of overflowing the stack.
    let mut comp = Composition::new("selfref".to_string(), "Self".to_string(), 32, 32, 30, 30);
    let mut inner = Composition::new(
        "selfref_inner".to_string(),
        "Inner".to_string(),
        32,
        32,
        30,
        30,
    );
    // Inner contains a pre-comp pointing back at the outer comp id
    let cyc = Layer::new(
        "cyc".to_string(),
        "Cycle".to_string(),
        LayerType::PreComp {
            comp_id: "selfref".to_string(),
        },
        30,
    );
    inner.layers.push(cyc);
    comp.sub_compositions.push(inner);

    let mut pc = Layer::new(
        "pc".to_string(),
        "Loop".to_string(),
        LayerType::PreComp {
            comp_id: "selfref_inner".to_string(),
        },
        30,
    );
    pc.transform.position = Animatable::new_constant([16.0, 16.0]);
    comp.layers.push(pc);

    // Must terminate and produce a valid buffer
    let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(pixels.len(), (32 * 32 * 4) as usize);
}
#[test]
fn test_precomp_respects_layer_scale() {
    crate::core::frame_cache::bump_version();
    // Sub-comp: full-frame red solid
    let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
    let solid = Layer::new(
        "bg".to_string(),
        "Red".to_string(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    sub.layers.push(solid);

    // Main comp: pre-comp scaled to 50% — corners should stay background
    let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
    comp.sub_compositions.push(sub);
    let mut pc = Layer::new(
        "pc".to_string(),
        "Half".to_string(),
        LayerType::PreComp {
            comp_id: "sub".to_string(),
        },
        30,
    );
    pc.transform.position = Animatable::new_constant([32.0, 32.0]);
    pc.transform.scale = Animatable::new_constant([50.0, 50.0]);
    comp.layers.push(pc);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let corner = 0usize;
    let center = ((32 * 64 + 32) * 4) as usize;
    assert!(
        pixels[corner] < 60,
        "Corner should be background when pre-comp is scaled down (R={})",
        pixels[corner]
    );
    assert!(
        pixels[center] > 180,
        "Center should show scaled pre-comp content (R={})",
        pixels[center]
    );
}
