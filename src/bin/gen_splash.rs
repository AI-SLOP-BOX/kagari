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

fn add_ring(
    comp: &mut Composition, name: &str, color: [f32; 4],
    ring_size: f32, thickness: f32,
    appear: u32, expand_end: u32, fade_end: u32,
) {
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(ring_size),
                height: Animatable::new_constant(ring_size),
            },
            color: [color[0], color[1], color[2], 0.0],
            stroke_color: color, stroke_width: thickness,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 180,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_constant([960.0, 480.0]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(appear, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(expand_end, [1.0, 1.0], bez_out(0.6)),
        Keyframe::new(fade_end - 5, [1.1, 1.1], InterpolationType::Linear),
        Keyframe::new(fade_end, [1.2, 1.2], InterpolationType::Linear),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear, 0.0, InterpolationType::Linear),
        Keyframe::new(appear + 3, 100.0, InterpolationType::Linear),
        Keyframe::new(fade_end - 10, 100.0, InterpolationType::Linear),
        Keyframe::new(fade_end, 0.0, InterpolationType::Linear),
    ]);
}

fn add_particle(
    comp: &mut Composition, name: &str, color: [f32; 4],
    angle: f32, speed: f32, size: f32,
    burst_frame: u32,
) {
    let rad = angle * std::f32::consts::PI / 180.0;
    let dist = speed * 8.0;
    let end_x = 960.0 + rad.cos() * dist;
    let end_y = 480.0 + rad.sin() * dist;

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
        }, 180,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [960.0, 480.0], InterpolationType::Linear),
        Keyframe::new(burst_frame, [960.0, 480.0], InterpolationType::Linear),
        Keyframe::new(burst_frame + 20, [end_x, end_y], bez_out(0.4)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(burst_frame, 0.0, InterpolationType::Linear),
        Keyframe::new(burst_frame + 2, 100.0, InterpolationType::Linear),
        Keyframe::new(burst_frame + 12, 60.0, InterpolationType::Linear),
        Keyframe::new(burst_frame + 25, 0.0, InterpolationType::Linear),
    ]);
}

fn main() {
    let mut comp = Composition::new(
        "splash".into(), "Kagari VFX Splash".into(),
        1920, 1080, 30, 180,
    );

    // === BACKGROUND ===
    comp.add_layer(Layer::new("bg".into(), "BG".into(),
        LayerType::Solid { color: [0.0; 4] }, 180));
    comp.layers.last_mut().unwrap().transform.position =
        Animatable::new_constant([960.0, 540.0]);

    // === PHASE 1: SPARK — a tiny bright dot pulses at center (frames 0-40) ===
    comp.add_layer(Layer::new("spark_core".into(), "Spark".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(6.0),
                height: Animatable::new_constant(6.0),
            },
            color: [1.0, 1.0, 0.9, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 180,
    ));
    {
        let s = comp.layers.last_mut().unwrap();
        s.transform.position = Animatable::new_constant([960.0, 480.0]);
        // Pulse: grow-shrink-grow
        s.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [50.0, 50.0], InterpolationType::Linear),
            Keyframe::new(8, [120.0, 120.0], InterpolationType::Linear),
            Keyframe::new(14, [80.0, 80.0], InterpolationType::Linear),
            Keyframe::new(22, [150.0, 150.0], InterpolationType::Linear),
            Keyframe::new(30, [60.0, 60.0], InterpolationType::Linear),
            Keyframe::new(38, [200.0, 200.0], bez_in(0.5)),
            Keyframe::new(48, [0.0, 0.0], InterpolationType::Linear),
        ]);
        s.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(3, 100.0, InterpolationType::Linear),
            Keyframe::new(38, 100.0, InterpolationType::Linear),
            Keyframe::new(48, 0.0, InterpolationType::Linear),
        ]);
    }

    // Spark glow (larger, softer)
    comp.add_layer(Layer::new("spark_glow".into(), "SparkGlow".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(40.0),
                height: Animatable::new_constant(40.0),
            },
            color: [1.0, 0.7, 0.2, 0.6],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 180,
    ));
    {
        let g = comp.layers.last_mut().unwrap();
        g.transform.position = Animatable::new_constant([960.0, 480.0]);
        g.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [20.0, 20.0], InterpolationType::Linear),
            Keyframe::new(10, [60.0, 60.0], InterpolationType::Linear),
            Keyframe::new(20, [30.0, 30.0], InterpolationType::Linear),
            Keyframe::new(35, [80.0, 80.0], InterpolationType::Linear),
            Keyframe::new(48, [0.0, 0.0], InterpolationType::Linear),
        ]);
        g.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(5, 80.0, InterpolationType::Linear),
            Keyframe::new(38, 80.0, InterpolationType::Linear),
            Keyframe::new(48, 0.0, InterpolationType::Linear),
        ]);
    }

    // === PHASE 2: SHOCKWAVE RINGS expand outward from center (frames 38-80) ===
    add_ring(&mut comp, "ring1", [1.0, 0.6, 0.1, 0.9], 800.0, 8.0, 38, 55, 75);
    add_ring(&mut comp, "ring2", [1.0, 0.4, 0.05, 0.7], 1200.0, 5.0, 42, 62, 82);
    add_ring(&mut comp, "ring3", [0.9, 0.2, 0.0, 0.5], 1600.0, 3.0, 46, 70, 88);

    // Inner bright ring
    add_ring(&mut comp, "ring_inner", [1.0, 0.9, 0.5, 1.0], 300.0, 4.0, 40, 58, 78);

    // === PHASE 3: LOGO REVEAL — fades in from center as rings expand (frames 50-180) ===
    comp.add_layer(Layer::new("logo".into(), "Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() }, 180));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_constant([960.0, 480.0]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(48, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(75, [42.0, 42.0], bez_both(0.15, 0.6)),
            Keyframe::new(140, [44.0, 44.0], InterpolationType::Linear),
            Keyframe::new(170, [42.0, 42.0], InterpolationType::Linear),
        ]);
        logo.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(48, 0.0, InterpolationType::Linear),
            Keyframe::new(75, 100.0, bez_in(0.5)),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    // === PARTICLES — burst outward from center on ignition (frame 42) ===
    let colors = [
        [1.0, 0.7, 0.1, 1.0],
        [1.0, 0.5, 0.0, 1.0],
        [0.9, 0.3, 0.0, 1.0],
        [1.0, 0.85, 0.3, 1.0],
        [0.8, 0.15, 0.05, 1.0],
    ];
    let angles: [f32; 12] = [
        0.0, 30.0, 60.0, 90.0, 120.0, 150.0,
        180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
    ];
    for (i, &angle) in angles.iter().enumerate() {
        let color = colors[i % colors.len()];
        let speed = 5.0 + (i as f32 * 1.1) % 4.0;
        let size = 3.0 + (i as f32 * 1.5) % 5.0;
        add_particle(&mut comp, &format!("p{}", i), color, angle, speed, size, 42);
    }

    // === PHASE 4: TEXT (frames 90-180) ===
    comp.add_layer(Layer::new("title".into(), "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(), font_size: 80,
            color: [1.0; 4], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 180));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = Animatable::new_animated(vec![
            Keyframe::new(85, [960.0, 510.0], InterpolationType::Linear),
            Keyframe::new(110, [960.0, 330.0], bez_both(0.2, 0.6)),
        ]);
        t.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(85, 0.0, InterpolationType::Linear),
            Keyframe::new(110, 100.0, InterpolationType::Linear),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    comp.add_layer(Layer::new("vfx".into(), "VFX".into(),
        LayerType::Text {
            text: "VFX".into(), font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 180));
    {
        let v = comp.layers.last_mut().unwrap();
        v.transform.position = Animatable::new_animated(vec![
            Keyframe::new(88, [960.0, 510.0], InterpolationType::Linear),
            Keyframe::new(115, [960.0, 420.0], bez_both(0.2, 0.6)),
        ]);
        v.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(88, 0.0, InterpolationType::Linear),
            Keyframe::new(115, 100.0, InterpolationType::Linear),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    // Gold accent line
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
        }, 180,
    ));
    {
        let ln = comp.layers.last_mut().unwrap();
        ln.transform.position = Animatable::new_constant([960.0, 590.0]);
        ln.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(110, 0.0, InterpolationType::Linear),
            Keyframe::new(125, 100.0, InterpolationType::Linear),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    // Tagline
    comp.add_layer(Layer::new("tag".into(), "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(), font_size: 24,
            color: [0.6; 4], font_family: "SF Pro Display".into(),
            tracking: 3.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 180));
    {
        let tg = comp.layers.last_mut().unwrap();
        tg.transform.position = Animatable::new_constant([960.0, 630.0]);
        tg.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(120, 0.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    let project = Project { compositions: vec![comp], ..Default::default() };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
