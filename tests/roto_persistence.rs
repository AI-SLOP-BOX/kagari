use kagari_vfx::core::roto_brush_engine::{RotoStroke, RotoStrokeType};
use kagari_vfx::core::timeline::{Layer, LayerType};

#[test]
fn roto_brush_strokes_survive_layer_roundtrip() {
    let mut layer = Layer::new(
        "roto".into(),
        "Roto Subject".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    layer.roto_brush_strokes.push(RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[12.0, 8.0], [14.0, 9.0]],
        radius: 6.0,
    });
    layer.roto_brush_strokes.push(RotoStroke {
        stroke_type: RotoStrokeType::Background,
        points: vec![[2.0, 4.0]],
        radius: 3.0,
    });

    let json = serde_json::to_string(&layer).expect("layer should serialize");
    let restored: Layer = serde_json::from_str(&json).expect("layer should deserialize");
    assert_eq!(restored.roto_brush_strokes.len(), 2);
    assert_eq!(restored.roto_brush_strokes[0].stroke_type, RotoStrokeType::Foreground);
    assert_eq!(restored.roto_brush_strokes[0].points, vec![[12.0, 8.0], [14.0, 9.0]]);
}

#[test]
fn legacy_layers_without_roto_strokes_default_to_empty() {
    let layer = Layer::new(
        "legacy".into(),
        "Legacy".into(),
        LayerType::Null,
        30,
    );
    let mut value = serde_json::to_value(layer).expect("layer should serialize");
    value
        .as_object_mut()
        .expect("layer should be an object")
        .remove("roto_brush_strokes");
    let restored: Layer = serde_json::from_value(value).expect("legacy layer should deserialize");
    assert!(restored.roto_brush_strokes.is_empty());
}
