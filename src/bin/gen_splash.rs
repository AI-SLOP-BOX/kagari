use kagari_vfx::core::timeline::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::project_migration::save_project_atomic;

fn main() {
    let mut comp = Composition::new(
        "splash".into(),
        "Kagari VFX Splash".into(),
        1920, 1080, 30, 240,
    );

    // 0: Black background
    comp.add_layer(Layer::new(
        "bg".into(), "Background".into(),
        LayerType::Solid { color: [0.0, 0.0, 0.0, 1.0] },
        240,
    ));
    {
        let bg = comp.layers.last_mut().unwrap();
        bg.transform.position = Animatable::new_constant([960.0, 540.0]);
    }

    // 1: Animated radial matte — ellipse expands from tiny to full
    comp.add_layer(Layer::new(
        "matte_shape".into(), "Radial Matte".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_animated(vec![
                    Keyframe::new(0, 0.0, InterpolationType::Linear),
                    Keyframe::new(5, 0.0, InterpolationType::Linear),
                    Keyframe::new(50, 2600.0, InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                        custom_bezier: None,
                    }),
                ]),
                height: Animatable::new_animated(vec![
                    Keyframe::new(0, 0.0, InterpolationType::Linear),
                    Keyframe::new(5, 0.0, InterpolationType::Linear),
                    Keyframe::new(50, 2600.0, InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                        custom_bezier: None,
                    }),
                ]),
            },
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        240,
    ));
    {
        let matte = comp.layers.last_mut().unwrap();
        matte.transform.position = Animatable::new_constant([960.0, 540.0]);
    }

    // 2: Logo with AlphaMatte — revealed by expanding ellipse
    comp.add_layer(Layer::new(
        "logo".into(), "Kagari Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() },
        240,
    ));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_constant([960.0, 540.0]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [20.0, 20.0], InterpolationType::Linear),
            Keyframe::new(50, [40.0, 40.0], InterpolationType::Bezier {
                outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                custom_bezier: None,
            }),
            Keyframe::new(180, [40.0, 40.0], InterpolationType::Linear),
            Keyframe::new(220, [38.0, 38.0], InterpolationType::Linear),
        ]);
        logo.track_matte = TrackMatteMode::AlphaMatte;
    }

    // 3: (Glow removed for fast render — re-add via GUI for final)

    // 4: Title "KAGARI VFX" — fades in after logo reveal
    comp.add_layer(Layer::new(
        "title".into(), "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(),
            font_size: 80,
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            text_on_path: false,
        },
        240,
    ));
    {
        let title = comp.layers.last_mut().unwrap();
        title.transform.position = Animatable::new_animated(vec![
            Keyframe::new(50, [960.0, 560.0], InterpolationType::Linear),
            Keyframe::new(80, [960.0, 380.0], InterpolationType::Bezier {
                outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                custom_bezier: None,
            }),
        ]);
        title.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(50, 0.0, InterpolationType::Linear),
            Keyframe::new(80, 100.0, InterpolationType::Linear),
            Keyframe::new(190, 100.0, InterpolationType::Linear),
            Keyframe::new(225, 0.0, InterpolationType::Linear),
        ]);
    }

    // 5: "VFX" subtitle — staggered fade-in
    comp.add_layer(Layer::new(
        "vfx_text".into(), "VFX".into(),
        LayerType::Text {
            text: "VFX".into(),
            font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            text_on_path: false,
        },
        240,
    ));
    {
        let vfx = comp.layers.last_mut().unwrap();
        vfx.transform.position = Animatable::new_animated(vec![
            Keyframe::new(55, [960.0, 560.0], InterpolationType::Linear),
            Keyframe::new(85, [960.0, 470.0], InterpolationType::Bezier {
                outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                custom_bezier: None,
            }),
        ]);
        vfx.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(55, 0.0, InterpolationType::Linear),
            Keyframe::new(85, 100.0, InterpolationType::Linear),
            Keyframe::new(190, 100.0, InterpolationType::Linear),
            Keyframe::new(225, 0.0, InterpolationType::Linear),
        ]);
    }

    // 6: Tagline — "Motion Graphics & Compositing"
    comp.add_layer(Layer::new(
        "tagline".into(), "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(),
            font_size: 24,
            color: [0.6, 0.6, 0.6, 1.0],
            font_family: "SF Pro Display".into(),
            tracking: 3.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            text_on_path: false,
        },
        240,
    ));
    {
        let tag = comp.layers.last_mut().unwrap();
        tag.transform.position = Animatable::new_constant([960.0, 680.0]);
        tag.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(90, 0.0, InterpolationType::Linear),
            Keyframe::new(115, 100.0, InterpolationType::Linear),
            Keyframe::new(190, 100.0, InterpolationType::Linear),
            Keyframe::new(225, 0.0, InterpolationType::Linear),
        ]);
    }

    // 7: Horizontal line accent — wipes in
    comp.add_layer(Layer::new(
        "line_accent".into(), "Line".into(),
        LayerType::Solid { color: [0.9, 0.6, 0.1, 1.0] },
        240,
    ));
    {
        let line = comp.layers.last_mut().unwrap();
        line.transform.position = Animatable::new_constant([960.0, 640.0]);
        line.transform.scale = Animatable::new_constant([300.0, 0.15]);
        line.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(85, 0.0, InterpolationType::Linear),
            Keyframe::new(105, 100.0, InterpolationType::Linear),
            Keyframe::new(190, 100.0, InterpolationType::Linear),
            Keyframe::new(225, 0.0, InterpolationType::Linear),
        ]);
        line.effects.push(Effect {
            id: "e_line_wipe".into(),
            name: "LinearWipe".into(),
            effect_type: EffectType::LinearWipe {
                completion: Animatable::new_animated(vec![
                    Keyframe::new(85, 100.0, InterpolationType::Linear),
                    Keyframe::new(110, 0.0, InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.42, speed: 0.0 },
                        custom_bezier: None,
                    }),
                ]),
                angle: Animatable::new_constant(90.0),
            },
            enabled: true,
        });
    }

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };

    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
