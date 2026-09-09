use kagari_vfx::core::timeline::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::project_migration::save_project_atomic;

fn bez_in(inf: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
        incoming: BezierControlPoint { influence: inf, speed: 0.0 },
        custom_bezier: None,
    }
}
fn bez_out(inf: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: inf, speed: 0.0 },
        incoming: BezierControlPoint { influence: 0.0, speed: 0.0 },
        custom_bezier: None,
    }
}
fn bez_both(a: f32, b: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: a, speed: 0.0 },
        incoming: BezierControlPoint { influence: b, speed: 0.0 },
        custom_bezier: None,
    }
}

fn main() {
    let mut comp = Composition::new(
        "splash".into(), "Kagari VFX Splash".into(),
        1920, 1080, 30, 150,
    );

    // BG
    comp.add_layer(Layer::new("bg".into(), "BG".into(),
        LayerType::Solid { color: [0.0; 4] }, 150));
    comp.layers.last_mut().unwrap().transform.position =
        Animatable::new_constant([960.0, 540.0]);

    // Ambient glow
    comp.add_layer(Layer::new("ambient".into(), "Ambient".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(800.0),
                height: Animatable::new_constant(800.0),
            },
            color: [0.12, 0.06, 0.01, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let a = comp.layers.last_mut().unwrap();
        a.transform.position = Animatable::new_constant([960.0, 480.0]);
        a.transform.opacity = Animatable::new_constant(25.0);
    }

    // === SPARK: 2 layers (white core + orange glow) ===
    comp.add_layer(Layer::new("spark1".into(), "Spark1".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(8.0),
                height: Animatable::new_constant(8.0),
            },
            color: [1.0, 1.0, 0.9, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let s = comp.layers.last_mut().unwrap();
        s.transform.position = Animatable::new_constant([960.0, 480.0]);
        s.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [30.0, 30.0], InterpolationType::Linear),
            Keyframe::new(8, [80.0, 80.0], InterpolationType::Linear),
            Keyframe::new(14, [40.0, 40.0], InterpolationType::Linear),
            Keyframe::new(22, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(30, [35.0, 35.0], InterpolationType::Linear),
            Keyframe::new(38, [150.0, 150.0], bez_in(0.4)),
            Keyframe::new(46, [0.0, 0.0], InterpolationType::Linear),
        ]);
        s.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(3, 100.0, InterpolationType::Linear),
            Keyframe::new(38, 100.0, InterpolationType::Linear),
            Keyframe::new(46, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new("spark2".into(), "Spark2".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(50.0),
                height: Animatable::new_constant(50.0),
            },
            color: [1.0, 0.5, 0.1, 0.7],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let s = comp.layers.last_mut().unwrap();
        s.transform.position = Animatable::new_constant([960.0, 480.0]);
        s.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [15.0, 15.0], InterpolationType::Linear),
            Keyframe::new(12, [50.0, 50.0], InterpolationType::Linear),
            Keyframe::new(20, [25.0, 25.0], InterpolationType::Linear),
            Keyframe::new(35, [70.0, 70.0], InterpolationType::Linear),
            Keyframe::new(46, [0.0, 0.0], InterpolationType::Linear),
        ]);
        s.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(5, 70.0, InterpolationType::Linear),
            Keyframe::new(38, 70.0, InterpolationType::Linear),
            Keyframe::new(46, 0.0, InterpolationType::Linear),
        ]);
    }

    // === 8 ENERGY STREAKS ===
    let streaks: [(f32, f32, [f32; 4], u32); 8] = [
        (0.0,   500.0, [1.0, 0.8, 0.2, 1.0], 0),
        (45.0,  420.0, [1.0, 0.5, 0.1, 0.9], 2),
        (90.0,  480.0, [0.9, 0.3, 0.0, 0.8], 1),
        (135.0, 440.0, [1.0, 0.7, 0.15, 1.0], 3),
        (180.0, 510.0, [1.0, 0.4, 0.05, 0.9], 0),
        (225.0, 430.0, [0.9, 0.2, 0.0, 0.8], 2),
        (270.0, 470.0, [1.0, 0.6, 0.1, 1.0], 1),
        (315.0, 450.0, [0.85, 0.25, 0.05, 0.9], 3),
    ];
    for (i, &(ang, len, color, delay)) in streaks.iter().enumerate() {
        let rad = ang * std::f32::consts::PI / 180.0;
        comp.add_layer(Layer::new(
            format!("st{}", i).into(), format!("Streak {}", i).into(),
            LayerType::Shape {
                shape_type: ShapeType::FreeformBezier {
                    points: vec![[-2.0, 0.0], [2.0, 0.0], [1.0, -len], [-1.0, -len]],
                    tangents: vec![([0.0, 0.0], [0.0, 0.0]); 4],
                    closed: true,
                },
                color,
                stroke_color: [0.0; 4], stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0, bevel_depth: 0.0,
            }, 150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 480.0]);
        l.transform.rotation = Animatable::new_constant(ang + 180.0);
        let app = 36 + delay;
        l.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(app, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(app + 6, [100.0, 100.0], bez_out(0.5)),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(app, 0.0, InterpolationType::Linear),
            Keyframe::new(app + 4, 85.0, InterpolationType::Linear),
            Keyframe::new(app + 18, 0.0, InterpolationType::Linear),
        ]);
    }

    // === 3 SHOCKWAVE RINGS ===
    let rings: [(f32, f32, [f32; 4], u32, u32, u32); 3] = [
        (400.0, 5.0, [1.0, 0.7, 0.2, 1.0], 38, 52, 70),
        (900.0, 3.5, [1.0, 0.4, 0.05, 0.7], 42, 58, 76),
        (1400.0, 2.0, [0.9, 0.2, 0.0, 0.4], 46, 64, 82),
    ];
    for (i, &(size, thick, color, app, exp, fade)) in rings.iter().enumerate() {
        comp.add_layer(Layer::new(
            format!("ring{}", i).into(), format!("Ring {}", i).into(),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(size),
                    height: Animatable::new_constant(size),
                },
                color: [color[0], color[1], color[2], 0.0],
                stroke_color: color, stroke_width: thick,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0, bevel_depth: 0.0,
            }, 150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 480.0]);
        l.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(app, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(exp, [1.0, 1.0], bez_out(0.55)),
            Keyframe::new(fade, [1.1, 1.1], InterpolationType::Linear),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(app, 0.0, InterpolationType::Linear),
            Keyframe::new(app + 2, 100.0, InterpolationType::Linear),
            Keyframe::new(fade - 6, 80.0, InterpolationType::Linear),
            Keyframe::new(fade, 0.0, InterpolationType::Linear),
        ]);
    }

    // === LOGO with scale overshoot ===
    comp.add_layer(Layer::new("logo".into(), "Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() }, 150));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_constant([960.0, 480.0]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(40, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(62, [46.0, 46.0], bez_both(0.1, 0.65)),
            Keyframe::new(78, [42.0, 42.0], bez_in(0.4)),
            Keyframe::new(130, [43.0, 43.0], InterpolationType::Linear),
            Keyframe::new(150, [42.0, 42.0], InterpolationType::Linear),
        ]);
        logo.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(40, 0.0, InterpolationType::Linear),
            Keyframe::new(58, 100.0, bez_in(0.4)),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    // Center flash
    comp.add_layer(Layer::new("flash".into(), "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(300.0),
                height: Animatable::new_constant(300.0),
            },
            color: [1.0, 0.95, 0.7, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let f = comp.layers.last_mut().unwrap();
        f.transform.position = Animatable::new_constant([960.0, 480.0]);
        f.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(42, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(47, [8.0, 8.0], InterpolationType::Linear),
            Keyframe::new(55, [12.0, 12.0], InterpolationType::Linear),
            Keyframe::new(70, [14.0, 14.0], InterpolationType::Linear),
        ]);
        f.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(42, 0.0, InterpolationType::Linear),
            Keyframe::new(46, 75.0, InterpolationType::Linear),
            Keyframe::new(52, 35.0, InterpolationType::Linear),
            Keyframe::new(70, 0.0, InterpolationType::Linear),
        ]);
    }

    // === 10 TRAIL PARTICLES ===
    let pc = [
        [1.0, 0.7, 0.1, 1.0], [1.0, 0.5, 0.0, 1.0],
        [0.9, 0.3, 0.0, 1.0], [1.0, 0.85, 0.3, 1.0],
        [0.8, 0.15, 0.05, 1.0],
    ];
    for i in 0..10 {
        let ang = (i as f32) * 36.0;
        let color = pc[i % pc.len()];
        let speed = 5.0 + (i as f32 * 0.9) % 4.0;
        let sz = 2.5 + (i as f32 * 0.5) % 3.0;
        let rad = ang * std::f32::consts::PI / 180.0;
        let dist = speed * 10.0;
        let ex = 960.0 + rad.cos() * dist;
        let ey = 480.0 + rad.sin() * dist;

        comp.add_layer(Layer::new(
            format!("pt{}", i).into(), format!("Particle {}", i).into(),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(sz),
                    height: Animatable::new_constant(sz * 2.5),
                },
                color,
                stroke_color: [0.0; 4], stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0, bevel_depth: 0.0,
            }, 150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [960.0, 480.0], InterpolationType::Linear),
            Keyframe::new(44, [960.0, 480.0], InterpolationType::Linear),
            Keyframe::new(60, [ex, ey], bez_out(0.35)),
        ]);
        l.transform.rotation = Animatable::new_constant(ang + 90.0);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(44, 0.0, InterpolationType::Linear),
            Keyframe::new(46, 100.0, InterpolationType::Linear),
            Keyframe::new(54, 50.0, InterpolationType::Linear),
            Keyframe::new(65, 0.0, InterpolationType::Linear),
        ]);
    }

    // === TEXT ===
    comp.add_layer(Layer::new("title".into(), "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(), font_size: 80,
            color: [1.0; 4], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 150));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = Animatable::new_animated(vec![
            Keyframe::new(75, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(100, [960.0, 330.0], bez_both(0.15, 0.65)),
        ]);
        t.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(75, 0.0, InterpolationType::Linear),
            Keyframe::new(100, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new("vfx".into(), "VFX".into(),
        LayerType::Text {
            text: "VFX".into(), font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 150));
    {
        let v = comp.layers.last_mut().unwrap();
        v.transform.position = Animatable::new_animated(vec![
            Keyframe::new(78, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(105, [960.0, 420.0], bez_both(0.15, 0.65)),
        ]);
        v.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(78, 0.0, InterpolationType::Linear),
            Keyframe::new(105, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    // Line
    comp.add_layer(Layer::new("line".into(), "Line".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(600.0),
                height: Animatable::new_constant(2.0),
            },
            color: [0.9, 0.6, 0.1, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let ln = comp.layers.last_mut().unwrap();
        ln.transform.position = Animatable::new_constant([960.0, 590.0]);
        ln.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(100, 0.0, InterpolationType::Linear),
            Keyframe::new(115, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    // Tagline
    comp.add_layer(Layer::new("tag".into(), "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(), font_size: 24,
            color: [0.6; 4], font_family: "SF Pro Display".into(),
            tracking: 3.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 150));
    {
        let tg = comp.layers.last_mut().unwrap();
        tg.transform.position = Animatable::new_constant([960.0, 630.0]);
        tg.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(110, 0.0, InterpolationType::Linear),
            Keyframe::new(128, 100.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    let project = Project { compositions: vec![comp], ..Default::default() };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
