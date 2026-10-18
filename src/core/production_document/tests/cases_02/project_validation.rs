#[test]
fn rejects_invalid_composition_and_layer_properties() {
    let mut invalid_project = Project::default();
    invalid_project.active_composition_idx = invalid_project.compositions.len();
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].width = 0;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].duration_frames = 0;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].background_color[0] = f32::NAN;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].duration_frames =
        ProductionDocument::MAX_COMPOSITION_FRAMES + 1;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].id.clear();
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].name = "  ".into();
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let duplicate = invalid_project.compositions[0].clone();
    invalid_project.compositions.push(duplicate);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "huge-duration",
            "Huge Duration",
            ProjectItemType::Video {
                path: "video.mp4".into(),
                duration_sec: ProductionDocument::MAX_ASSET_DURATION_SEC + 1.0,
            },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "missing-video-source".into(),
            "Missing Video Source".into(),
            LayerType::Video {
                source: "  ".into(),
                frames_dir: "frames".into(),
                frame_count: 1,
                audio_wav: None,
                speed: 1.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "mismatched-shape-tangents".into(),
            "Mismatched Shape Tangents".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::FreeformBezier {
                    points: vec![[0.0, 0.0], [10.0, 0.0]],
                    tangents: vec![([0.0, 0.0], [0.0, 0.0])],
                    closed: false,
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-video-speed".into(),
            "Invalid Video Speed".into(),
            LayerType::Video {
                source: "video.mp4".into(),
                frames_dir: "frames".into(),
                frame_count: 1,
                audio_wav: None,
                speed: 1_001.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "bad-comp",
            "Bad Comp",
            ProjectItemType::Composition { comp_idx: 99 },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "huge-image",
            "Huge Image",
            ProjectItemType::Image {
                path: "image.png".into(),
                width: 65_536,
                height: 1080,
            },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-shape-dimensions".into(),
            "Invalid Shape Dimensions".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Polygon {
                    sides: crate::core::property::Animatable::new_constant(2.0),
                    radius: crate::core::property::Animatable::new_constant(-1.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "bad-image",
            "Bad Image",
            ProjectItemType::Image {
                path: "image.png".into(),
                width: 0,
                height: 1080,
            },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "empty-media",
            "Empty Media",
            ProjectItemType::Audio {
                path: "  ".into(),
                duration_sec: 1.0,
            },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "bad-solid",
            "Bad Solid",
            ProjectItemType::Solid {
                color: [f32::INFINITY, 0.0, 0.0, 1.0],
            },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "item_comp1",
            "Duplicate",
            ProjectItemType::Folder { name: "x".into() },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "empty-folder",
            "Folder",
            ProjectItemType::Folder { name: "  ".into() },
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mut asset = crate::core::timeline::ProjectItem::new(
        "asset",
        "Asset",
        ProjectItemType::Folder { name: "x".into() },
    );
    asset.parent_folder = Some("missing".into());
    invalid_project.assets.push(asset);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mut folder_a = crate::core::timeline::ProjectItem::new(
        "folder-a",
        "A",
        ProjectItemType::Folder { name: "A".into() },
    );
    folder_a.parent_folder = Some("folder-b".into());
    let mut folder_b = crate::core::timeline::ProjectItem::new(
        "folder-b",
        "B",
        ProjectItemType::Folder { name: "B".into() },
    );
    folder_b.parent_folder = Some("folder-a".into());
    invalid_project.assets.extend([folder_a, folder_b]);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "missing-precomp".into(),
            "Missing Precomp".into(),
            LayerType::PreComp {
                comp_id: "does-not-exist".into(),
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-audio".into(),
            "Invalid Audio".into(),
            LayerType::Audio {
                path: "".into(),
                volume: crate::core::property::Animatable::new_constant(f32::NAN),
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].out_frame =
        invalid_project.compositions[0].layers[0].in_frame;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].out_frame =
        invalid_project.compositions[0].duration_frames + 1;
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let duplicate = invalid_project.compositions[0].layers[0].clone();
    invalid_project.compositions[0].layers.push(duplicate);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let comp_id = invalid_project.compositions[0].id.clone();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "self-precomp".into(),
            "Self Precomp".into(),
            LayerType::PreComp { comp_id },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let layer_id = invalid_project.compositions[0].layers[0].id.clone();
    invalid_project.compositions[0].layers[0].parent_id = Some(layer_id);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let effect = crate::core::timeline::Effect {
        id: "duplicate-effect".into(),
        name: "Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: crate::core::property::Animatable::new_constant(4.0),
        },
        enabled: true,
    };
    invalid_project.compositions[0].layers[0]
        .effects
        .extend([effect.clone(), effect]);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0]
        .effects
        .push(crate::core::timeline::Effect {
            id: "nonfinite-effect".into(),
            name: "Blur".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: crate::core::property::Animatable::new_constant(f32::NAN),
            },
            enabled: true,
        });
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    let mask = crate::core::mask::Mask::new_rect(
        "duplicate-mask".into(),
        "Mask".into(),
        0.0,
        0.0,
        100.0,
        100.0,
    );
    invalid_project.compositions[0].layers[0]
        .masks
        .extend([mask.clone(), mask]);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].transform.position =
        crate::core::property::Animatable::new_constant([f32::NAN, 0.0]);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].transform.opacity =
        crate::core::property::Animatable::new_constant(100.1);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].time_remap =
        Some(crate::core::property::Animatable::new_constant(f32::NAN));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].time_remap =
        Some(crate::core::property::Animatable::new_constant(-1.0));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0].time_remap =
        Some(crate::core::property::Animatable::new_constant(
            ProductionDocument::MAX_COMPOSITION_FRAMES as f32 + 1.0,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0].layers[0]
        .transform_3d
        .position = crate::core::property::Animatable::new_constant([0.0, f32::INFINITY, 0.0]);
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { font_size, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *font_size = 0;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { align, .. } = &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *align = 3;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { font_size, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *font_size = 16_385;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { stroke_width, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *stroke_width = 16_384.1;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { font_family, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *font_family = "   ".into();
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { tracking, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *tracking = 100_001.0;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { leading, .. } =
        &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *leading = 1_000.1;
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    if let LayerType::Text { text, .. } = &mut invalid_project.compositions[0].layers[1].layer_type
    {
        *text = "x".repeat(1_000_001);
    }
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-gradient-stops".into(),
            "Invalid Gradient Stops".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Rectangle {
                    width: crate::core::property::Animatable::new_constant(100.0),
                    height: crate::core::property::Animatable::new_constant(100.0),
                    corner_radius: crate::core::property::Animatable::new_constant(0.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: crate::core::timeline::ShapeFillType::LinearGradient {
                    start: [0.0, 0.0],
                    end: [100.0, 100.0],
                    colors: vec![[0.0, 0.0, 0.0, 1.0], [1.0, f32::NAN, 1.0, 1.0]],
                    stops: vec![0.0],
                },
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

    let mut invalid_project = Project::default();
    invalid_project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "invalid-shape-animation".into(),
            "Invalid Shape Animation".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Star {
                    points: crate::core::property::Animatable::new_constant(f32::NAN),
                    inner_radius: crate::core::property::Animatable::new_constant(10.0),
                    outer_radius: crate::core::property::Animatable::new_constant(20.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            300,
        ));
    assert!(ProductionDocument::new(invalid_project).validate().is_err());

}
