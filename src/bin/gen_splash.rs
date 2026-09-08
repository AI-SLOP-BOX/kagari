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

fn triangle_pts(size: f32) -> ShapeType {
    let h = size * 0.866;
    ShapeType::FreeformBezier {
        points: vec![[0.0, -h * 0.67], [-size * 0.5, h * 0.33], [size * 0.5, h * 0.33]],
        tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
        closed: true,
    }
}

fn add_shard(
    comp: &mut Composition,
    name: &str,
    shape: ShapeType,
    color: [f32; 4],
    from: [f32; 2],
    rot0: f32,
    sc: f32,
    arrive: u32,
) {
    comp.add_layer(Layer::new(
        name.into(), name.into(),
        LayerType::Shape {
            shape_type: shape, color,
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        },
        270,
    ));
    let l = comp.layers.last_mut().unwrap();
    let pre = arrive.saturating_sub(45);

    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, from, InterpolationType::Linear),
        Keyframe::new(pre, from, InterpolationType::Linear),
        Keyframe::new(arrive, [960.0, 480.0], bez_in(0.7)),
    ]);
    l.transform.rotation = Animatable::new_animated(vec![
        Keyframe::new(0, rot0, InterpolationType::Linear),
        Keyframe::new(pre, rot0, InterpolationType::Linear),
        Keyframe::new(arrive, rot0 + 540.0, bez_in(0.5)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [sc, sc], InterpolationType::Linear),
        Keyframe::new(pre, [sc, sc], InterpolationType::Linear),
        Keyframe::new(arrive, [sc * 1.5, sc * 1.5], bez_in(0.55)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(pre.max(5), 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 12, 100.0, InterpolationType::Linear),
        Keyframe::new(arrive + 30, 0.0, InterpolationType::Linear),
    ]);
}

fn main() {
    let mut comp = Composition::new(
        "splash".into(), "Kagari VFX Splash".into(),
        1920, 1080, 30, 270,
    );

    // Background
    comp.add_layer(Layer::new("bg".into(), "Background".into(),
        LayerType::Solid { color: [0.0; 4] }, 270));
    comp.layers.last_mut().unwrap().transform.position =
        Animatable::new_constant([960.0, 540.0]);

    // === 8 SHARDS: scatter → converge → flash → logo ===

    // Shape 1: amber triangle, top-left
    add_shard(&mut comp, "Shard 1", triangle_pts(120.0),
        [1.0, 0.7, 0.1, 1.0], [180.0, 120.0], -45.0, 6.5, 55);

    // Shape 2: orange diamond, top-right
    add_shard(&mut comp, "Shard 2",
        ShapeType::Star {
            points: Animatable::new_constant(4.0),
            inner_radius: Animatable::new_constant(30.0),
            outer_radius: Animatable::new_constant(70.0),
        },
        [1.0, 0.5, 0.0, 1.0], [1720.0, 100.0], 30.0, 7.0, 58);

    // Shape 3: red-orange triangle, left
    add_shard(&mut comp, "Shard 3", triangle_pts(100.0),
        [0.9, 0.2, 0.1, 1.0], [100.0, 500.0], 120.0, 6.0, 62);

    // Shape 4: gold circle, right
    add_shard(&mut comp, "Shard 4",
        ShapeType::Ellipse {
            width: Animatable::new_constant(90.0),
            height: Animatable::new_constant(90.0),
        },
        [1.0, 0.85, 0.3, 1.0], [1780.0, 520.0], -60.0, 6.5, 66);

    // Shape 5: crimson diamond, bottom-left
    add_shard(&mut comp, "Shard 5",
        ShapeType::Star {
            points: Animatable::new_constant(4.0),
            inner_radius: Animatable::new_constant(25.0),
            outer_radius: Animatable::new_constant(60.0),
        },
        [0.8, 0.1, 0.1, 1.0], [220.0, 930.0], 90.0, 6.5, 70);

    // Shape 6: warm-yellow triangle, bottom-right
    add_shard(&mut comp, "Shard 6", triangle_pts(110.0),
        [1.0, 0.9, 0.4, 1.0], [1680.0, 910.0], -120.0, 6.0, 74);

    // Shape 7: cream circle, top-center
    add_shard(&mut comp, "Shard 7",
        ShapeType::Ellipse {
            width: Animatable::new_constant(60.0),
            height: Animatable::new_constant(60.0),
        },
        [1.0, 0.95, 0.8, 1.0], [960.0, -30.0], 0.0, 5.0, 60);

    // Shape 8: deep-red star, bottom-center
    add_shard(&mut comp, "Shard 8",
        ShapeType::Star {
            points: Animatable::new_constant(5.0),
            inner_radius: Animatable::new_constant(20.0),
            outer_radius: Animatable::new_constant(55.0),
        },
        [0.7, 0.05, 0.05, 1.0], [960.0, 1120.0], 45.0, 6.0, 78);

    // === FLASH at convergence (quick radial burst) ===
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
        f.transform.position = Animatable::new_constant([960.0, 480.0]);
        f.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(85, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(88, [8.0, 8.0], InterpolationType::Linear),
            Keyframe::new(90, [12.0, 12.0], InterpolationType::Linear),
            Keyframe::new(95, [14.0, 14.0], InterpolationType::Linear),
            Keyframe::new(108, [15.0, 15.0], InterpolationType::Linear),
        ]);
        f.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(85, 0.0, InterpolationType::Linear),
            Keyframe::new(88, 100.0, InterpolationType::Linear),
            Keyframe::new(90, 70.0, InterpolationType::Linear),
            Keyframe::new(95, 30.0, InterpolationType::Linear),
            Keyframe::new(108, 0.0, InterpolationType::Linear),
        ]);
    }

    // === LOGO reveal ===
    comp.add_layer(Layer::new("logo".into(), "Kagari Logo".into(),
        LayerType::Image { path: "assets/kagari_logo.png".into() }, 270));
    {
        let logo = comp.layers.last_mut().unwrap();
        logo.transform.position = Animatable::new_constant([960.0, 480.0]);
        logo.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(88, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(112, [38.0, 38.0], bez_in(0.55)),
            Keyframe::new(200, [40.0, 40.0], InterpolationType::Linear),
            Keyframe::new(250, [38.0, 38.0], InterpolationType::Linear),
        ]);
        logo.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(88, 0.0, InterpolationType::Linear),
            Keyframe::new(112, 100.0, bez_in(0.5)),
            Keyframe::new(215, 100.0, InterpolationType::Linear),
            Keyframe::new(255, 0.0, InterpolationType::Linear),
        ]);
    }

    // === "KAGARI" ===
    comp.add_layer(Layer::new("title".into(), "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(), font_size: 80,
            color: [1.0; 4], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 270));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = Animatable::new_animated(vec![
            Keyframe::new(108, [960.0, 500.0], InterpolationType::Linear),
            Keyframe::new(138, [960.0, 330.0], bez_in(0.6)),
        ]);
        t.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(108, 0.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(218, 100.0, InterpolationType::Linear),
            Keyframe::new(252, 0.0, InterpolationType::Linear),
        ]);
    }

    // === "VFX" gold ===
    comp.add_layer(Layer::new("vfx".into(), "VFX".into(),
        LayerType::Text {
            text: "VFX".into(), font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 270));
    {
        let v = comp.layers.last_mut().unwrap();
        v.transform.position = Animatable::new_animated(vec![
            Keyframe::new(112, [960.0, 500.0], InterpolationType::Linear),
            Keyframe::new(142, [960.0, 420.0], bez_in(0.6)),
        ]);
        v.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(112, 0.0, InterpolationType::Linear),
            Keyframe::new(142, 100.0, InterpolationType::Linear),
            Keyframe::new(218, 100.0, InterpolationType::Linear),
            Keyframe::new(252, 0.0, InterpolationType::Linear),
        ]);
    }

    // === Gold accent line ===
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
            Keyframe::new(218, 100.0, InterpolationType::Linear),
            Keyframe::new(252, 0.0, InterpolationType::Linear),
        ]);
    }

    // === Tagline ===
    comp.add_layer(Layer::new("tag".into(), "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(), font_size: 24,
            color: [0.6; 4], font_family: "SF Pro Display".into(),
            tracking: 3.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 270));
    {
        let tg = comp.layers.last_mut().unwrap();
        tg.transform.position = Animatable::new_constant([960.0, 630.0]);
        tg.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(150, 0.0, InterpolationType::Linear),
            Keyframe::new(175, 100.0, InterpolationType::Linear),
            Keyframe::new(218, 100.0, InterpolationType::Linear),
            Keyframe::new(252, 0.0, InterpolationType::Linear),
        ]);
    }

    let project = Project { compositions: vec![comp], ..Default::default() };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
