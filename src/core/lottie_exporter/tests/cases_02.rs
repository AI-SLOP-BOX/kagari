#[test]
fn test_lottie_animated_mask_path_becomes_shape_keyframes() {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::mask::MaskPath;
    let mut comp = Composition::new("c".into(), "AnimMask".into(), 100, 100, 30, 30);
    let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
    let mut path = MaskPath::new_rect(0.0, 0.0, 40.0, 40.0);
    path.vertices = Animatable::new_animated(vec![
        Keyframe::new(
            0,
            vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
            InterpolationType::Linear,
        ),
        Keyframe::new(
            15,
            vec![[10.0, 5.0], [60.0, 5.0], [60.0, 55.0], [10.0, 55.0]],
            InterpolationType::Linear,
        ),
    ]);
    layer.masks.push(crate::core::mask::Mask {
        id: "ma".into(),
        name: "grow".into(),
        enabled: true,
        mode: crate::core::mask::MaskMode::Add,
        path,
        feather: Animatable::new_constant(0.0),
        opacity: Animatable::new_constant(100.0),
        expansion: Animatable::new_constant(0.0),
        inverted: false,
        wiggle: None,
    });
    comp.layers.push(layer);

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    let pt = &v["layers"][0]["masksProperties"][0]["pt"];
    assert_eq!(pt["a"], 1, "animated vertices → shape keyframes");
    let kfs = pt["k"].as_array().unwrap();
    assert_eq!(kfs.len(), 2);
    assert_eq!(kfs[0]["t"], 0);
    assert_eq!(kfs[1]["t"], 15);
    assert_eq!(
        kfs[1]["s"][0]["v"][2],
        json!([60.0, 55.0]),
        "second keyframe carries moved vertices"
    );
}
#[test]
fn test_importer_invalid_inputs_return_none() {
    assert!(LottieImporter::import_from_str("").is_none());
    assert!(LottieImporter::import_from_str("{not json").is_none());
    assert!(
        LottieImporter::import_from_str("{}").is_none(),
        "no layers key"
    );
    assert!(LottieImporter::import_from_str("{\"fr\":30}").is_none());
}
#[test]
fn test_import_minimal_animated_solid() {
    let json = r##"{
            "v": "5.7.4", "fr": 30, "ip": 0, "op": 20, "w": 64, "h": 64,
            "nm": "Rot", "bg": "#101010",
            "layers": [{
                "ty": 1, "nm": "S", "ind": 1, "ip": 0, "op": 20, "bm": 2,
                "ks": {
                    "o": {"a": 0, "k": 1},
                    "r": {"a": 1, "k": [{"t": 0, "s": [0]}, {"t": 10, "s": [45]}]},
                    "p": {"a": 0, "k": [10, 20]},
                    "a": {"a": 0, "k": [0, 0]},
                    "s": {"a": 0, "k": [100, 100]}
                },
                "sw": 8, "sh": 8, "sc": "#ff0000"
            }]
        }"##;
    let comp = LottieImporter::import_from_str(json).expect("valid lottie");
    assert_eq!(comp.name, "Rot");
    assert_eq!(comp.fps, 30);
    assert_eq!(comp.duration_frames, 20);
    assert!(comp.background_color[0] > 0.0, "background colour imported");
    assert_eq!(comp.layers.len(), 1);
    let layer = &comp.layers[0];
    match &layer.layer_type {
        LayerType::Solid { color } => {
            assert!((color[0] - 1.0).abs() < 1e-6 && color[1].abs() < 1e-6);
        }
        other => panic!("expected solid, got {other:?}"),
    }
    // Animated rotation lands on its keyframe values.
    assert_eq!(layer.transform.rotation.evaluate(10), 45.0);
    assert_eq!(layer.transform.rotation.evaluate(0), 0.0);
    assert_eq!(layer.transform.position.evaluate(5), [10.0, 20.0]);
    assert_eq!(layer.transform.scale.evaluate(0), [100.0, 100.0]);
    assert_eq!(layer.blend_mode, BlendMode::Screen, "bm=2 → Screen");
}
#[test]
fn test_export_import_roundtrip_preserves_structure() {
    // Build a rich comp: parented text with track matte + masked shape
    // with animated position — then verify the round trip.
    let mut comp = Composition::new("c".into(), "RT".into(), 200, 200, 24, 30);
    let mut src = Layer::new("src".into(), "Src".into(), LayerType::Null, 30);
    src.transform.position = Animatable::new_constant([5.0, 6.0]);
    let mut txt = Layer::new(
        "txt".into(),
        "T".into(),
        LayerType::Text {
            text: "RoundTrip".into(),
            font_size: 20,
            color: [0.2, 0.4, 0.8, 1.0],
            font_family: "Inter".into(),
            tracking: 3.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        30,
    );
    txt.parent_id = Some("src".into());
    txt.track_matte = TrackMatteMode::AlphaMatte;
    let mut shape = Layer::new(
        "shp".into(),
        "Sh".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(60.0),
                height: Animatable::new_constant(40.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 1.0, 0.0, 1.0],
            stroke_width: 2.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    shape.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [10.0, 10.0], InterpolationType::Linear),
        Keyframe::new(12, [80.0, 40.0], InterpolationType::Linear),
    ]);
    shape.masks.push(crate::core::mask::Mask {
        id: "m".into(),
        name: "win".into(),
        enabled: true,
        mode: crate::core::mask::MaskMode::Add,
        path: crate::core::mask::MaskPath::new_rect(0.0, 0.0, 50.0, 50.0),
        feather: Animatable::new_constant(0.0),
        opacity: Animatable::new_constant(100.0),
        expansion: Animatable::new_constant(0.0),
        inverted: false,
        wiggle: None,
    });
    comp.layers.push(src);
    comp.layers.push(txt);
    comp.layers.push(shape);

    let exported = LottieExporter::export_to_json(&comp).expect("export succeeds");
    let back = LottieImporter::import_from_str(&exported).expect("roundtrip parses");
    assert_eq!(back.layers.len(), 3);
    assert_eq!(back.fps, 24);

    // Parenting survives (text → source layer id).
    assert_eq!(
        back.layers[1].parent_id.as_deref(),
        Some(back.layers[0].id.as_str())
    );
    // Track matte survives.
    assert_eq!(back.layers[1].track_matte, TrackMatteMode::AlphaMatte);
    // Text content survives.
    match &back.layers[1].layer_type {
        LayerType::Text {
            text, font_size, ..
        } => {
            assert_eq!(text, "RoundTrip");
            assert_eq!(*font_size, 20);
        }
        other => panic!("expected text, got {other:?}"),
    }
    // Animated position keyframes survive with values and frames.
    assert_eq!(back.layers[2].transform.position.evaluate(12), [80.0, 40.0]);
    assert_eq!(back.layers[2].transform.position.evaluate(0), [10.0, 10.0]);
    // Mask survives with vertex count and mode.
    assert_eq!(back.layers[2].masks.len(), 1);
    let m = &back.layers[2].masks[0];
    if let Animatable::Constant(v) = &m.path.vertices {
        assert_eq!(v.len(), 4);
    } else {
        panic!("expected constant mask vertices");
    }
}
#[test]
fn test_export_project_with_precomp_tree() {
    use crate::core::timeline::Project;
    // B: nested comp with one shape; C: deeper comp referenced from B.
    let mut b = Composition::new("B".into(), "NestedB".into(), 100, 100, 24, 30);
    b.layers.push(Layer::new(
        "bs".into(),
        "BShape".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(20.0),
                height: Animatable::new_constant(20.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));
    let mut c = Composition::new("C".into(), "DeepC".into(), 80, 80, 24, 30);
    c.layers
        .push(Layer::new("cl".into(), "CNull".into(), LayerType::Null, 30));
    b.sub_compositions.push(c);
    b.layers.push(Layer::new(
        "bpc".into(),
        "ToC".into(),
        LayerType::PreComp {
            comp_id: "C".into(),
        },
        30,
    ));

    let mut a = Composition::new("A".into(), "Root".into(), 400, 300, 24, 60);
    a.layers.push(Layer::new(
        "apc".into(),
        "ToB".into(),
        LayerType::PreComp {
            comp_id: "B".into(),
        },
        60,
    ));
    a.layers.push(Layer::new(
        "as".into(),
        "ASolid".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    ));

    let project = Project {
        compositions: vec![a, b],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };

    let v: Value =
        serde_json::from_str(&export_project_to_json(&project).expect("export succeeds")).unwrap();
    // Root precomp slot.
    let root_pc = &v["layers"][0];
    assert_eq!(root_pc["ty"], 0, "precomp layer type");
    assert_eq!(root_pc["refId"], "B");
    assert_eq!(root_pc["w"], 400, "slot sized to containing comp");
    // Both referenced comps emitted as assets (transitively through B→C).
    let assets = v["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 2, "B and C must both be assets");
    let ids: Vec<&str> = assets.iter().filter_map(|a| a["id"].as_str()).collect();
    assert!(ids.contains(&"B") && ids.contains(&"C"), "ids {ids:?}");
    let b_asset = assets.iter().find(|a| a["id"] == "B").unwrap();
    assert_eq!(b_asset["layers"][0]["ty"], 4, "shape inside asset");
    assert_eq!(
        b_asset["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["refId"] == "C")
            .map(|l| l["ty"].clone()),
        Some(json!(0)),
        "nested precomp slot inside asset B"
    );

    // Missing reference degrades gracefully: no crash, no phantom asset.
    let mut a2 = Composition::new("A2".into(), "R2".into(), 10, 10, 24, 5);
    a2.layers.push(Layer::new(
        "x".into(),
        "Ghost".into(),
        LayerType::PreComp {
            comp_id: "NOPE".into(),
        },
        5,
    ));
    let p2 = Project {
        compositions: vec![a2],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };
    let v2: Value =
        serde_json::from_str(&export_project_to_json(&p2).expect("export succeeds")).unwrap();
    assert_eq!(v2["layers"][0]["ty"], 0);
    assert_eq!(v2["assets"].as_array().unwrap().len(), 0);

    // Empty project → error payload, no panic.
    let empty = Project {
        compositions: vec![],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };
    assert!(export_project_to_json(&empty).is_err());
}
#[test]
fn test_project_roundtrip_through_precomp_export() {
    use crate::core::timeline::Project;
    let mut b = Composition::new("B".into(), "NestedB".into(), 100, 100, 24, 30);
    b.layers.push(Layer::new(
        "bs".into(),
        "BShape".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(20.0),
                height: Animatable::new_constant(20.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));
    let mut c = Composition::new("C".into(), "DeepC".into(), 80, 80, 24, 30);
    c.layers
        .push(Layer::new("cl".into(), "CNull".into(), LayerType::Null, 30));
    b.sub_compositions.push(c);
    b.layers.push(Layer::new(
        "bpc".into(),
        "ToC".into(),
        LayerType::PreComp {
            comp_id: "C".into(),
        },
        30,
    ));

    let mut a = Composition::new("A".into(), "Root".into(), 400, 300, 24, 60);
    a.layers.push(Layer::new(
        "apc".into(),
        "ToB".into(),
        LayerType::PreComp {
            comp_id: "B".into(),
        },
        60,
    ));
    let project = Project {
        compositions: vec![a, b],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };

    let exported = export_project_to_json(&project).expect("export succeeds");
    let back =
        LottieImporter::import_project_from_str(&exported).expect("project roundtrip parses");
    // Root + B + C all present; root is active.
    assert_eq!(back.compositions.len(), 3);
    assert_eq!(back.active_composition_idx, 0);
    let root = &back.compositions[0];
    assert_eq!(root.name, "Root"); // bodymovin roots carry a name, not an id
                                   // Root's precomp slot points at imported asset B.
    match &root.layers[0].layer_type {
        LayerType::PreComp { comp_id } => assert_eq!(comp_id, "B"),
        other => panic!("expected precomp, got {other:?}"),
    }
    // Asset B kept its bodymovin id and nested precomp reference to C.
    let b_back = back
        .compositions
        .iter()
        .find(|c| c.id == "B")
        .expect("asset B");
    match &b_back.layers[1].layer_type {
        LayerType::PreComp { comp_id } => assert_eq!(comp_id, "C"),
        other => panic!("expected nested precomp in B, got {other:?}"),
    }
    assert!(
        back.compositions.iter().any(|c| c.id == "C"),
        "deep asset C imported"
    );

    // Single-comp importer still ignores assets gracefully.
    let single = LottieImporter::import_from_str(&exported).expect("single-comp import");
    assert_eq!(single.name, "Root");
}
