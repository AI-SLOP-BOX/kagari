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

fn dot_layer(
    comp: &mut Composition, name: &str, color: [f32; 4], size: f32,
) {
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(size),
                height: Animatable::new_constant(size),
            },
            color,
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
}

/// Google式: 4つの残り火ドットが楕円軌道を回りながら中心へ吸い込まれる
#[allow(clippy::too_many_arguments)]
fn add_orbit_dot(
    comp: &mut Composition, name: &str, color: [f32; 4], size: f32,
    phase_deg: f32, r0: f32, r1: f32, f0: u32, f1: u32,
) {
    dot_layer(comp, name, color, size);
    let l = comp.layers.last_mut().unwrap();
    let mut pos_kf = vec![Keyframe::new(0, [960.0, 480.0], InterpolationType::Linear)];
    let mut t = f0;
    while t <= f1 {
        let k = (t - f0) as f32 / (f1 - f0).max(1) as f32;
        let ang = (phase_deg + k * 450.0) * std::f32::consts::PI / 180.0;
        let r = r0 + (r1 - r0) * k * k;
        let x = 960.0 + ang.cos() * r;
        let y = 480.0 + ang.sin() * r * 0.62;
        let interp = if t == f1 { bez_in(0.6) } else { InterpolationType::Linear };
        pos_kf.push(Keyframe::new(t, [x, y], interp));
        t += 4;
    }
    l.transform.position = Animatable::new_animated(pos_kf);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(f0, [60.0, 60.0], InterpolationType::Linear),
        Keyframe::new(f1, [130.0, 130.0], bez_in(0.5)),
        Keyframe::new(f1 + 4, [0.0, 0.0], InterpolationType::Linear),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(f0, 0.0, InterpolationType::Linear),
        Keyframe::new(f0 + 3, 100.0, InterpolationType::Linear),
        Keyframe::new(f1, 100.0, InterpolationType::Linear),
        Keyframe::new(f1 + 4, 0.0, InterpolationType::Linear),
    ]);
}

/// Netflix式の縦光柱
fn add_pillar(comp: &mut Composition, appear: u32, peak: u32, fade: u32) {
    comp.add_layer(Layer::new(
        "pillar".into(), "Light Pillar".into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: vec![[-10.0, 10.0], [10.0, 10.0], [5.0, -760.0], [-5.0, -760.0]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 4],
                closed: true,
            },
            color: [1.0, 0.85, 0.45, 0.85],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_constant([960.0, 480.0]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 20.0], InterpolationType::Linear),
        Keyframe::new(appear, [0.0, 20.0], InterpolationType::Linear),
        Keyframe::new(peak, [100.0, 100.0], bez_out(0.5)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear, 0.0, InterpolationType::Linear),
        Keyframe::new(peak, 45.0, InterpolationType::Linear),
        Keyframe::new(fade, 0.0, InterpolationType::Linear),
    ]);
}

/// Netflix式: ロゴの縁から光の筋が左右へ分解していく
fn add_edge_streak(
    comp: &mut Composition, name: &str, dir: f32, appear: u32, len: f32,
) {
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(len),
                height: Animatable::new_constant(3.0),
            },
            color: [1.0, 0.8, 0.35, 0.8],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    let l = comp.layers.last_mut().unwrap();
    let ex = 960.0 + dir * 420.0;
    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [960.0, 560.0], InterpolationType::Linear),
        Keyframe::new(appear, [960.0, 560.0], InterpolationType::Linear),
        Keyframe::new(appear + 12, [ex, 560.0], bez_out(0.4)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 100.0], InterpolationType::Linear),
        Keyframe::new(appear, [0.0, 100.0], InterpolationType::Linear),
        Keyframe::new(appear + 6, [70.0, 100.0], bez_out(0.4)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear, 0.0, InterpolationType::Linear),
        Keyframe::new(appear + 3, 60.0, InterpolationType::Linear),
        Keyframe::new(appear + 14, 0.0, InterpolationType::Linear),
    ]);
}

fn main() {
    let mut comp = Composition::new(
        "splash".into(), "Kagari VFX Splash".into(),
        1920, 1080, 30, 150,
    );

    comp.add_layer(Layer::new("bg".into(), "BG".into(),
        LayerType::Solid { color: [0.0; 4] }, 150));
    comp.layers.last_mut().unwrap().transform.position =
        Animatable::new_constant([960.0, 540.0]);

    comp.add_layer(Layer::new("ambient".into(), "Ambient".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(900.0),
                height: Animatable::new_constant(900.0),
            },
            color: [0.10, 0.05, 0.01, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let a = comp.layers.last_mut().unwrap();
        a.transform.position = Animatable::new_constant([960.0, 480.0]);
        a.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(30, 30.0, InterpolationType::Linear),
            Keyframe::new(135, 30.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    // === Google式: 4つの残り火が渦を巻いて合体 ===
    add_orbit_dot(&mut comp, "ember_w", [1.0, 0.95, 0.85, 1.0], 10.0, 0.0, 320.0, 15.0, 5, 40);
    add_orbit_dot(&mut comp, "ember_y", [1.0, 0.75, 0.2, 1.0], 12.0, 90.0, 300.0, 15.0, 8, 41);
    add_orbit_dot(&mut comp, "ember_o", [1.0, 0.45, 0.08, 1.0], 13.0, 180.0, 310.0, 15.0, 11, 42);
    add_orbit_dot(&mut comp, "ember_r", [0.85, 0.15, 0.03, 1.0], 12.0, 270.0, 290.0, 15.0, 14, 43);

    // 合体フラッシュ
    comp.add_layer(Layer::new("flash".into(), "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(260.0),
                height: Animatable::new_constant(260.0),
            },
            color: [1.0, 0.97, 0.85, 1.0],
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
            Keyframe::new(46, [9.0, 9.0], InterpolationType::Linear),
            Keyframe::new(54, [13.0, 13.0], InterpolationType::Linear),
            Keyframe::new(70, [15.0, 15.0], InterpolationType::Linear),
        ]);
        f.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(42, 0.0, InterpolationType::Linear),
            Keyframe::new(45, 85.0, InterpolationType::Linear),
            Keyframe::new(52, 40.0, InterpolationType::Linear),
            Keyframe::new(70, 0.0, InterpolationType::Linear),
        ]);
    }

    // === Netflix式: 光柱 + 奥から手前へのドリー ===
    add_pillar(&mut comp, 40, 48, 75);

    comp.add_layer(Layer::new("logo".into(), "Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() }, 150));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(40, [960.0, 520.0], InterpolationType::Linear),
            Keyframe::new(72, [960.0, 478.0], bez_both(0.1, 0.6)),
            Keyframe::new(90, [960.0, 482.0], bez_in(0.4)),
        ]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [4.0, 4.0], InterpolationType::Linear),
            Keyframe::new(40, [4.0, 4.0], InterpolationType::Linear),
            Keyframe::new(72, [47.0, 47.0], bez_both(0.1, 0.65)),
            Keyframe::new(88, [42.0, 42.0], bez_in(0.4)),
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

    // === Netflix式: ロゴの輪郭が光の筋に分解 ===
    add_edge_streak(&mut comp, "edge_l1", -1.0, 66, 320.0);
    add_edge_streak(&mut comp, "edge_r1", 1.0, 68, 320.0);
    add_edge_streak(&mut comp, "edge_l2", -1.0, 71, 220.0);
    add_edge_streak(&mut comp, "edge_r2", 1.0, 73, 220.0);

    // === 衝撃波リング ===
    for (i, (size, thick, alpha, app, exp, fade)) in [
        (420.0, 5.0, 0.9, 44, 56, 72),
        (950.0, 3.0, 0.6, 47, 62, 78),
        (1500.0, 2.0, 0.35, 50, 68, 84),
    ].into_iter().enumerate() {
        let color = [1.0, 0.55, 0.12, alpha];
        comp.add_layer(Layer::new(
            format!("ring{}", i), format!("Ring {}", i),
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
            Keyframe::new(fade, [1.08, 1.08], InterpolationType::Linear),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(app, 0.0, InterpolationType::Linear),
            Keyframe::new(app + 2, 100.0, InterpolationType::Linear),
            Keyframe::new(fade - 6, 80.0, InterpolationType::Linear),
            Keyframe::new(fade, 0.0, InterpolationType::Linear),
        ]);
    }

    // === 火の粉 ===
    let pc = [
        [1.0, 0.7, 0.1, 1.0], [1.0, 0.5, 0.0, 1.0],
        [0.9, 0.3, 0.0, 1.0], [1.0, 0.85, 0.3, 1.0],
    ];
    for i in 0..8 {
        let ang = (i as f32) * 45.0 + 12.0;
        let rad = ang * std::f32::consts::PI / 180.0;
        let dist = 55.0 + (i as f32 * 13.0) % 45.0;
        let ex = 960.0 + rad.cos() * dist * 10.0;
        let ey = 480.0 + rad.sin() * dist * 10.0;
        let sz = 2.5 + (i as f32 * 0.6) % 3.0;
        comp.add_layer(Layer::new(
            format!("pt{}", i), format!("Particle {}", i),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(sz),
                    height: Animatable::new_constant(sz * 2.5),
                },
                color: pc[i % pc.len()],
                stroke_color: [0.0; 4], stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0, bevel_depth: 0.0,
            }, 150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [960.0, 480.0], InterpolationType::Linear),
            Keyframe::new(46, [960.0, 480.0], InterpolationType::Linear),
            Keyframe::new(62, [ex, ey], bez_out(0.35)),
        ]);
        l.transform.rotation = Animatable::new_constant(ang + 90.0);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(46, 0.0, InterpolationType::Linear),
            Keyframe::new(48, 100.0, InterpolationType::Linear),
            Keyframe::new(56, 45.0, InterpolationType::Linear),
            Keyframe::new(66, 0.0, InterpolationType::Linear),
        ]);
    }

    // === タイトル ===
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
            Keyframe::new(102, 0.0, InterpolationType::Linear),
            Keyframe::new(116, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

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
            Keyframe::new(112, 0.0, InterpolationType::Linear),
            Keyframe::new(128, 100.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    let project = Project { compositions: vec![comp], ..Default::default() };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
