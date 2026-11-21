#[test]
fn test_lottie_export() {
    let comp = Composition::new("c1".into(), "TestComp".into(), 1920, 1080, 30, 300);
    let json_str = LottieExporter::export_to_json(&comp).expect("export succeeds");

    assert!(json_str.contains("\"v\": \"5.7.4\""));
    assert!(json_str.contains("\"TestComp\""));
}
#[test]
fn test_lottie_shape_colors_keep_alpha_and_animated_dimensions() {
    let exported_color = color_to_lottie(&[0.1, 0.2, 0.3, 0.4]);
    let exported_color = exported_color.as_array().expect("RGBA array");
    for (actual, expected) in exported_color.iter().zip([0.1, 0.2, 0.3, 0.4]) {
        assert!((actual.as_f64().unwrap() - expected).abs() < 1e-6);
    }

    let width = Animatable::new_animated(vec![
        Keyframe::new(0, 10.0, InterpolationType::Linear),
        Keyframe::new(12, 30.0, InterpolationType::Linear),
    ]);
    let height = Animatable::new_constant(20.0);
    let Animatable::Animated(keyframes) = merge_dims(&width, &height) else {
        panic!("one animated axis should produce animated dimensions");
    };
    assert_eq!(keyframes.len(), 2);
    assert_eq!(keyframes[0].value, [10.0, 20.0]);
    assert_eq!(keyframes[1].value, [30.0, 20.0]);
}
#[test]
fn test_lottie_exports_animated_keyframes() {
    let mut comp = Composition::new("c".into(), "Anim".into(), 640, 360, 30, 60);
    let mut l = Layer::new(
        "l1".into(),
        "Mover".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(30, [640.0, 360.0], InterpolationType::Linear),
    ]);
    comp.layers.push(l);

    let json_str = LottieExporter::export_to_json(&comp).expect("export succeeds");
    let v: Value = serde_json::from_str(&json_str).expect("valid JSON");
    let layer = &v["layers"][0];
    assert_eq!(layer["ty"], 1, "solid layer type");
    assert_eq!(layer["ks"]["p"]["a"], 1, "position must be animated");
    let kfs = layer["ks"]["p"]["k"].as_array().expect("keyframe array");
    assert_eq!(kfs.len(), 2);
    assert_eq!(kfs[0]["t"], 0);
    assert_eq!(kfs[1]["s"][0], 640.0);
}
#[test]
fn test_lottie_layer_types_and_parent() {
    let mut comp = Composition::new("c".into(), "Types".into(), 100, 100, 30, 30);
    let parent = Layer::new("p".into(), "ParentNull".into(), LayerType::Null, 30);
    let mut child = Layer::new(
        "ch".into(),
        "Child".into(),
        LayerType::Text {
            text: "Hi".into(),
            font_size: 24,
            color: [1.0; 4],
            font_family: "Arial".into(),
            tracking: 0.0,
            leading: 1.0,
            align: 0,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        30,
    );
    child.parent_id = Some("p".into());
    comp.layers.push(parent);
    comp.layers.push(child);

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    assert_eq!(v["layers"][0]["ty"], 3, "null type");
    assert_eq!(v["layers"][1]["ty"], 5, "text type");
    assert_eq!(v["layers"][1]["parent"], 1, "parent mapped to index");
}
#[test]
fn test_lottie_shape_layer_geometry() {
    use crate::core::timeline::ShapeType;
    let mut comp = Composition::new("c".into(), "Shapes".into(), 100, 100, 30, 30);
    comp.layers.push(Layer::new(
        "s1".into(),
        "Circle".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(50.0),
                height: Animatable::new_constant(80.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 1.0, 1.0],
            stroke_width: 4.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));
    comp.layers.push(Layer::new(
        "s2".into(),
        "Rect".into(),
        LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: Animatable::new_constant(20.0),
                height: Animatable::new_constant(10.0),
                corner_radius: Animatable::new_constant(5.0),
            },
            color: [0.0, 1.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));
    comp.layers.push(Layer::new(
        "s3".into(),
        "Poly".into(),
        LayerType::Shape {
            shape_type: ShapeType::Polygon {
                sides: Animatable::new_constant(6.0),
                radius: Animatable::new_constant(30.0),
            },
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));
    comp.layers.push(Layer::new(
        "s4".into(),
        "Star".into(),
        LayerType::Shape {
            shape_type: ShapeType::Star {
                points: Animatable::new_constant(5.0),
                inner_radius: Animatable::new_constant(15.0),
                outer_radius: Animatable::new_constant(40.0),
            },
            color: [0.5, 0.5, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    ));

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();

    let circle = &v["layers"][0]["shapes"][0];
    assert_eq!(circle["ty"], "gr");
    let geom = &circle["it"][0];
    assert_eq!(geom["ty"], "el");
    assert_eq!(geom["s"]["k"], json!([50.0, 80.0]));
    let fill = &circle["it"][1];
    assert_eq!(fill["ty"], "fl");
    assert_eq!(fill["c"]["k"], json!([1.0, 0.0, 0.0, 1.0]));
    let stroke = &circle["it"][2];
    assert_eq!(stroke["ty"], "st");
    assert_eq!(stroke["w"]["k"], 4.0);

    let rect_geom = &v["layers"][1]["shapes"][0]["it"][0];
    assert_eq!(rect_geom["ty"], "rc");
    assert_eq!(rect_geom["r"]["k"], 5.0);
    // stroke_width == 0 → no stroke item
    assert_eq!(v["layers"][1]["shapes"][0]["it"][1]["ty"], "fl");

    let poly_geom = &v["layers"][2]["shapes"][0]["it"][0];
    assert_eq!(poly_geom["ty"], "sr");
    assert_eq!(poly_geom["sy"], 1);
    assert_eq!(poly_geom["pt"]["k"], 6.0);

    let star_geom = &v["layers"][3]["shapes"][0]["it"][0];
    assert_eq!(star_geom["ty"], "sr");
    assert_eq!(star_geom["sy"], 2);
    assert_eq!(star_geom["pt"]["k"], 5.0);
}
#[test]
fn test_lottie_background_color() {
    let mut comp = Composition::new("c".into(), "BG".into(), 100, 100, 30, 30);
    comp.background_color = [0.05, 0.05, 0.08, 1.0];
    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    assert_eq!(v["bg"], "#0d0d14", "background as hex string");
}
#[test]
fn test_lottie_text_document_is_exported() {
    let mut comp = Composition::new("c".into(), "Txt".into(), 640, 360, 30, 60);
    comp.layers.push(Layer::new(
        "t1".into(),
        "Title".into(),
        LayerType::Text {
            text: "Hello Lottie".into(),
            font_size: 48,
            color: [1.0, 0.5, 0.0, 1.0],
            font_family: "Inter".into(),
            tracking: 12.0,
            leading: 1.4,
            align: 1, // Center
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        60,
    ));

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    let doc = &v["layers"][0]["t"];
    assert_eq!(
        doc["d"]["k"][0]["s"]["t"], "Hello Lottie",
        "text string preserved"
    );
    assert_eq!(doc["d"]["k"][0]["s"]["f"], "Inter");
    assert_eq!(doc["d"]["k"][0]["s"]["s"], 48);
    assert_eq!(doc["d"]["k"][0]["s"]["fc"], json!([1.0, 0.5, 0.0]));
    // lh crosses f32→f64 JSON serialization, so compare with tolerance.
    let lh = doc["d"]["k"][0]["s"]["lh"].as_f64().expect("numeric lh");
    assert!(
        (lh - (1.4f32 * 48.0) as f64).abs() < 1e-3,
        "line height = leading × size, got {lh}"
    );
    // Tracking rides under its own "tr" key so it cannot clobber the text.
    assert_eq!(doc["d"]["k"][0]["s"]["tr"], 12.0);
    // App Center(1) maps to Lottie j=2.
    assert_eq!(doc["d"]["k"][0]["s"]["j"], 2);

    // Non-text layers must NOT carry a text document.
    let mut plain = Composition::new("c".into(), "NoTxt".into(), 10, 10, 30, 10);
    plain.layers.push(Layer::new(
        "s".into(),
        "Solid".into(),
        LayerType::Solid { color: [1.0; 4] },
        10,
    ));
    let v2: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&plain).expect("export succeeds"))
            .unwrap();
    assert!(v2["layers"][0].get("t").is_none());
}
#[test]
fn test_lottie_track_matte_flags() {
    let mut comp = Composition::new("c".into(), "Matte".into(), 100, 100, 30, 30);
    // Layer 0 = matte source (above), layer 1 consumes it via alpha matte.
    comp.layers.push(Layer::new(
        "src".into(),
        "MatteSource".into(),
        LayerType::Null,
        30,
    ));
    let mut consumer = Layer::new(
        "dst".into(),
        "Consumer".into(),
        LayerType::Solid { color: [1.0; 4] },
        30,
    );
    consumer.track_matte = TrackMatteMode::AlphaMatte;
    comp.layers.push(consumer);

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    assert_eq!(v["layers"][0]["td"], 1, "source layer flagged td=1");
    assert_eq!(v["layers"][1]["tt"], 1, "alpha matte → tt=1");

    // Luma inverted maps to 4.
    let mut comp2 = Composition::new("c".into(), "Matte2".into(), 100, 100, 30, 30);
    comp2
        .layers
        .push(Layer::new("src".into(), "S".into(), LayerType::Null, 30));
    let mut c2 = Layer::new("dst".into(), "C".into(), LayerType::Null, 30);
    c2.track_matte = TrackMatteMode::LumaMatteInverted;
    comp2.layers.push(c2);
    let v2: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp2).expect("export succeeds"))
            .unwrap();
    assert_eq!(v2["layers"][0]["td"], 1);
    assert_eq!(v2["layers"][1]["tt"], 4);

    // No matte anywhere → no flags at all.
    let mut comp3 = Composition::new("c".into(), "NoMatte".into(), 100, 100, 30, 30);
    comp3
        .layers
        .push(Layer::new("a".into(), "A".into(), LayerType::Null, 30));
    comp3
        .layers
        .push(Layer::new("b".into(), "B".into(), LayerType::Null, 30));
    let v3: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp3).expect("export succeeds"))
            .unwrap();
    assert!(v3["layers"][0].get("td").is_none());
    assert!(v3["layers"][1].get("tt").is_none());
}
#[test]
fn test_lottie_masks_are_exported() {
    use crate::core::mask::MaskPath;
    let mut comp = Composition::new("c".into(), "Masked".into(), 200, 200, 30, 30);
    let mut layer = Layer::new(
        "l".into(),
        "M".into(),
        LayerType::Solid { color: [1.0; 4] },
        30,
    );
    layer.masks.push(crate::core::mask::Mask {
        id: "m1".into(),
        name: "Window".into(),
        enabled: true,
        mode: crate::core::mask::MaskMode::Add,
        path: MaskPath::new_rect(10.0, 20.0, 100.0, 80.0),
        feather: Animatable::new_constant(0.0),
        opacity: Animatable::new_constant(100.0),
        expansion: Animatable::new_constant(-4.0),
        inverted: false,
        wiggle: None,
    });
    comp.layers.push(layer);

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    assert_eq!(v["layers"][0]["hasMask"], true);
    let m = &v["layers"][0]["masksProperties"][0];
    assert_eq!(m["nm"], "Window");
    assert_eq!(m["mode"], "a", "Add → 'a'");
    assert_eq!(m["inv"], false);
    assert_eq!(m["o"]["k"], 100.0, "opacity preserved");
    assert_eq!(m["x"]["k"], -4.0, "expansion preserved");
    // Rect mask: 4 closed vertices.
    assert_eq!(m["pt"]["a"], 0, "static path");
    let shape = &m["pt"]["k"];
    assert_eq!(shape["c"], true);
    assert_eq!(shape["v"].as_array().unwrap().len(), 4);
    assert_eq!(shape["v"][0], json!([10.0, 20.0]));
}
#[test]
fn test_lottie_mask_modes_and_disabled() {
    use crate::core::mask::{Mask, MaskMode, MaskPath};
    let mut comp = Composition::new("c".into(), "Modes".into(), 100, 100, 30, 30);
    let mk = |mode: MaskMode, name: &str| Mask {
        id: format!("id-{name}"),
        name: name.into(),
        enabled: true,
        mode,
        path: MaskPath::new_rect(0.0, 0.0, 50.0, 50.0),
        feather: Animatable::new_constant(0.0),
        opacity: Animatable::new_constant(100.0),
        expansion: Animatable::new_constant(0.0),
        inverted: true,
        wiggle: None,
    };
    let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
    layer.masks.push(mk(MaskMode::Subtract, "cut"));
    layer.masks.push(mk(MaskMode::Intersect, "keep"));
    layer.masks.push(mk(MaskMode::Difference, "xor"));
    let mut disabled = mk(MaskMode::Add, "off");
    disabled.enabled = false;
    layer.masks.push(disabled);
    comp.layers.push(layer);

    let v: Value =
        serde_json::from_str(&LottieExporter::export_to_json(&comp).expect("export succeeds"))
            .unwrap();
    let props = &v["layers"][0]["masksProperties"];
    assert_eq!(props.as_array().unwrap().len(), 4);
    assert_eq!(props[0]["mode"], "s");
    assert_eq!(props[1]["mode"], "i");
    assert_eq!(props[2]["mode"], "f");
    assert_eq!(props[3]["mode"], "n", "disabled masks become 'none'");
    assert_eq!(props[2]["inv"], true);
}
