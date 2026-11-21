use super::elements::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

pub fn add_title_and_tagline(comp: &mut Composition) {
    // === タイトル ===
    comp.add_layer(Layer::new(
        "title".into(),
        "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(),
            font_size: 80,
            color: [1.0; 4],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = Animatable::new_animated(vec![
            Keyframe::new(78, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(102, [960.0, 330.0], bez_both(0.15, 0.65)),
        ]);
        t.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(78, 0.0, InterpolationType::Linear),
            Keyframe::new(102, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new(
        "vfx".into(),
        "VFX".into(),
        LayerType::Text {
            text: "VFX".into(),
            font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let v = comp.layers.last_mut().unwrap();
        v.transform.position = Animatable::new_animated(vec![
            Keyframe::new(81, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(106, [960.0, 420.0], bez_both(0.15, 0.65)),
        ]);
        v.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(81, 0.0, InterpolationType::Linear),
            Keyframe::new(106, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new(
        "line".into(),
        "Line".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(600.0),
                height: Animatable::new_constant(2.0),
            },
            color: [0.9, 0.6, 0.1, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let ln = comp.layers.last_mut().unwrap();
        ln.transform.position = Animatable::new_constant([960.0, 590.0]);
        ln.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(102, 0.0, InterpolationType::Linear),
            Keyframe::new(116, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new(
        "tag".into(),
        "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(),
            font_size: 24,
            color: [0.6; 4],
            font_family: "SF Pro Display".into(),
            tracking: 3.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let tg = comp.layers.last_mut().unwrap();
        tg.transform.position = Animatable::new_constant([960.0, 630.0]);
        tg.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(112, 0.0, InterpolationType::Linear),
            Keyframe::new(128, 100.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }
}
