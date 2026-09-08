use kagari_vfx::core::timeline::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::project_migration::save_project_atomic;

fn bez_in(influence: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
        incoming: BezierControlPoint { influence, speed: 0.0 },
        custom_bezier: None,
    }
}

fn bez_out(influence: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence, speed: 0.0 },
        incoming: BezierControlPoint { influence: 0.0, speed: 0.0 },
        custom_bezier: None,
    }
}

fn add_ring_segment(
    comp: &mut Composition,
    name: &str,
    color: [f32; 4],
    from_pos: [f32; 2],
    from_rot: f32,
    from_scale: f32,
    arrive: u32,
    scale_at_arrive: f32,
) {
    // Ring segment = thin arc shape (ellipse with high aspect ratio)
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(160.0),
                height: Animatable::new_constant(160.0),
            },
            color,
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    let l = comp.layers.last_mut().unwrap();
    let pre = arrive.saturating_sub(40);

    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, from_pos, InterpolationType::Linear),
        Keyframe::new(pre, from_pos, InterpolationType::Linear),
        Keyframe::new(arrive, [960.0, 540.0], bez_in(0.6)),
    ]);
    l.transform.rotation = Animatable::new_animated(vec![
        Keyframe::new(0, from_rot, InterpolationType::Linear),
        Keyframe::new(pre, from_rot, InterpolationType::Linear),
        Keyframe::new(arrive, from_rot + 180.0, bez_in(0.5)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(pre, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(arrive, [scale_at_arrive, scale_at_arrive], bez_in(0.55)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(pre.max(3), 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 8, 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 25, 0.0, InterpolationType::Linear),
    ]);
}

fn add_flame_piece(
    comp: &mut Composition,
    name: &str,
    color: [f32; 4],
    from_pos: [f32; 2],
    from_rot: f32,
    from_scale: f32,
    arrive: u32,
    scale_at_arrive: f32,
) {
    // Flame piece = triangle (pointed top)
    let h = 100.0 * 0.866;
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: vec![[0.0, -h * 0.67], [-50.0, h * 0.33], [50.0, h * 0.33]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                closed: true,
            },
            color,
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    let l = comp.layers.last_mut().unwrap();
    let pre = arrive.saturating_sub(40);

    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, from_pos, InterpolationType::Linear),
        Keyframe::new(pre, from_pos, InterpolationType::Linear),
        Keyframe::new(arrive, [960.0, 500.0], bez_in(0.65)),
    ]);
    l.transform.rotation = Animatable::new_animated(vec![
        Keyframe::new(0, from_rot, InterpolationType::Linear),
        Keyframe::new(pre, from_rot, InterpolationType::Linear),
        Keyframe::new(arrive, from_rot + 360.0, bez_in(0.5)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(pre, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(arrive, [scale_at_arrive, scale_at_arrive], bez_in(0.5)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(pre.max(3), 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 8, 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 22, 0.0, InterpolationType::Linear),
    ]);
}

fn add_wing_piece(
    comp: &mut Composition,
    name: &str,
    from_pos: [f32; 2],
    from_rot: f32,
    from_scale: f32,
    arrive: u32,
    scale_at_arrive: f32,
) {
    // Wing = small triangle (the white fang shape)
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: vec![[0.0, -40.0], [-30.0, 30.0], [30.0, 30.0]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                closed: true,
            },
            color: [0.95, 0.95, 0.9, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    let l = comp.layers.last_mut().unwrap();
    let pre = arrive.saturating_sub(40);

    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, from_pos, InterpolationType::Linear),
        Keyframe::new(pre, from_pos, InterpolationType::Linear),
        Keyframe::new(arrive, [960.0, 540.0], bez_in(0.65)),
    ]);
    l.transform.rotation = Animatable::new_animated(vec![
        Keyframe::new(0, from_rot, InterpolationType::Linear),
        Keyframe::new(pre, from_rot, InterpolationType::Linear),
        Keyframe::new(arrive, from_rot + 270.0, bez_in(0.5)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(pre, [from_scale, from_scale], InterpolationType::Linear),
        Keyframe::new(arrive, [scale_at_arrive, scale_at_arrive], bez_in(0.5)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(pre.max(3), 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 8, 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 22, 0.0, InterpolationType::Linear),
    ]);
}

fn add_light_ray(
    comp: &mut Composition,
    name: &str,
    angle: f32,
    delay: u32,
) {
    // Thin tall triangle radiating from center
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: vec![[-3.0, 0.0], [3.0, 0.0], [0.0, -600.0]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                closed: true,
            },
            color: [1.0, 0.7, 0.2, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_constant([960.0, 540.0]);
    l.transform.rotation = Animatable::new_constant(angle);

    let appear = 88 + delay;
    let fade_end = appear + 20;
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(appear - 2, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(appear + 5, [100.0, 100.0], bez_out(0.5)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear - 2, 0.0, InterpolationType::Linear),
        Keyframe::new(appear + 3, 60.0, InterpolationType::Linear),
        Keyframe::new(fade_end, 0.0, InterpolationType::Linear),
    ]);
}

fn main() {
    let mut comp = Composition::new(
        "splash".into(), "Kagari VFX Splash".into(),
        1920, 1080, 30, 180,
    );

    // Background
    comp.add_layer(Layer::new("bg".into(), "Background".into(),
        LayerType::Solid { color: [0.0; 4] }, 180));
    comp.layers.last_mut().unwrap().transform.position =
        Animatable::new_constant([960.0, 540.0]);

    // ===== PHASE 1: Logo pieces fly in (frames 0-85) =====
    // Ring segments: orange/amber ellipses converging from edges
    add_ring_segment(&mut comp, "ring1", [1.0, 0.5, 0.0, 1.0],
        [-200.0, -100.0], -30.0, 4.0, 55, 22.0);
    add_ring_segment(&mut comp, "ring2", [1.0, 0.35, 0.0, 1.0],
        [2100.0, -50.0], 45.0, 4.0, 58, 22.0);
    add_ring_segment(&mut comp, "ring3", [0.9, 0.2, 0.05, 1.0],
        [-150.0, 1100.0], 120.0, 4.0, 61, 22.0);
    add_ring_segment(&mut comp, "ring4", [1.0, 0.6, 0.1, 1.0],
        [2050.0, 1150.0], -90.0, 4.0, 64, 22.0);

    // Flame pieces: warm-colored triangles converging
    add_flame_piece(&mut comp, "flame1", [1.0, 0.6, 0.0, 1.0],
        [400.0, -200.0], -20.0, 3.5, 50, 18.0);
    add_flame_piece(&mut comp, "flame2", [0.95, 0.3, 0.0, 1.0],
        [1500.0, -150.0], 60.0, 3.5, 53, 18.0);
    add_flame_piece(&mut comp, "flame3", [1.0, 0.45, 0.05, 1.0],
        [200.0, 600.0], -45.0, 3.0, 56, 18.0);
    add_flame_piece(&mut comp, "flame4", [0.85, 0.15, 0.05, 1.0],
        [1700.0, 550.0], 30.0, 3.0, 59, 18.0);

    // Wing pieces: white triangles from sides
    add_wing_piece(&mut comp, "wing1",
        [-100.0, 400.0], 0.0, 3.0, 62, 15.0);
    add_wing_piece(&mut comp, "wing2",
        [2020.0, 400.0], 180.0, 3.0, 65, 15.0);

    // ===== PHASE 2: Impact flash + light rays (frames 85-110) =====
    // Flash: white ellipse expanding from center
    comp.add_layer(Layer::new("flash".into(), "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(200.0),
                height: Animatable::new_constant(200.0),
            },
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    {
        let f = comp.layers.last_mut().unwrap();
        f.transform.position = Animatable::new_constant([960.0, 540.0]);
        f.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(85, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(88, [10.0, 10.0], InterpolationType::Linear),
            Keyframe::new(90, [14.0, 14.0], InterpolationType::Linear),
            Keyframe::new(95, [16.0, 16.0], InterpolationType::Linear),
            Keyframe::new(110, [18.0, 18.0], InterpolationType::Linear),
        ]);
        f.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(85, 0.0, InterpolationType::Linear),
            Keyframe::new(88, 100.0, InterpolationType::Linear),
            Keyframe::new(90, 60.0, InterpolationType::Linear),
            Keyframe::new(95, 25.0, InterpolationType::Linear),
            Keyframe::new(110, 0.0, InterpolationType::Linear),
        ]);
    }

    // Light rays: thin triangles radiating outward
    add_light_ray(&mut comp, "ray1", 0.0, 0);
    add_light_ray(&mut comp, "ray2", 45.0, 1);
    add_light_ray(&mut comp, "ray3", 90.0, 2);
    add_light_ray(&mut comp, "ray4", 135.0, 3);
    add_light_ray(&mut comp, "ray5", 180.0, 4);
    add_light_ray(&mut comp, "ray6", 225.0, 5);
    add_light_ray(&mut comp, "ray7", 270.0, 6);
    add_light_ray(&mut comp, "ray8", 315.0, 7);

    // ===== PHASE 3: Logo reveal (frames 88-255) =====
    comp.add_layer(Layer::new("logo".into(), "Kagari Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() }, 180));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_constant([960.0, 480.0]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(86, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(110, [40.0, 40.0], bez_in(0.5)),
            Keyframe::new(155, [42.0, 42.0], InterpolationType::Linear),
            Keyframe::new(180, [40.0, 40.0], InterpolationType::Linear),
        ]);
        logo.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(86, 0.0, InterpolationType::Linear),
            Keyframe::new(110, 100.0, bez_in(0.5)),
            Keyframe::new(160, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    // ===== PHASE 4: Text + accents (frames 108-255) =====
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
            Keyframe::new(108, [960.0, 500.0], InterpolationType::Linear),
            Keyframe::new(138, [960.0, 330.0], bez_in(0.6)),
        ]);
        t.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(108, 0.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(162, 100.0, InterpolationType::Linear),
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
            Keyframe::new(112, [960.0, 500.0], InterpolationType::Linear),
            Keyframe::new(142, [960.0, 420.0], bez_in(0.6)),
        ]);
        v.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(112, 0.0, InterpolationType::Linear),
            Keyframe::new(142, 100.0, InterpolationType::Linear),
            Keyframe::new(162, 100.0, InterpolationType::Linear),
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
        },
        270,
    ));
    {
        let ln = comp.layers.last_mut().unwrap();
        ln.transform.position = Animatable::new_constant([960.0, 590.0]);
        ln.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(135, 0.0, InterpolationType::Linear),
            Keyframe::new(155, 100.0, InterpolationType::Linear),
            Keyframe::new(162, 100.0, InterpolationType::Linear),
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
            Keyframe::new(150, 0.0, InterpolationType::Linear),
            Keyframe::new(165, 100.0, InterpolationType::Linear),
            Keyframe::new(170, 100.0, InterpolationType::Linear),
            Keyframe::new(180, 0.0, InterpolationType::Linear),
        ]);
    }

    let project = Project { compositions: vec![comp], ..Default::default() };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
