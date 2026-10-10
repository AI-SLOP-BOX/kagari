use super::elements::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

pub fn add_embers_and_burst(comp: &mut Composition) {
    // === Google式: 4つの残り火が渦を巻いて合体 ===
    add_orbit_dot(
        comp,
        "ember_w",
        [1.0, 0.95, 0.85, 1.0],
        10.0,
        0.0,
        320.0,
        15.0,
        5,
        40,
    );
    add_orbit_dot(
        comp,
        "ember_y",
        [1.0, 0.75, 0.2, 1.0],
        12.0,
        90.0,
        300.0,
        15.0,
        8,
        41,
    );
    add_orbit_dot(
        comp,
        "ember_o",
        [1.0, 0.45, 0.08, 1.0],
        13.0,
        180.0,
        310.0,
        15.0,
        11,
        42,
    );
    add_orbit_dot(
        comp,
        "ember_r",
        [0.85, 0.15, 0.03, 1.0],
        12.0,
        270.0,
        290.0,
        15.0,
        14,
        43,
    );

    // 合体フラッシュ
    comp.add_layer(Layer::new(
        "flash".into(),
        "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(260.0),
                height: Animatable::new_constant(260.0),
            },
            color: [1.0, 0.97, 0.85, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
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
    add_pillar(comp, 40, 48, 75);

    comp.add_layer(Layer::new(
        "logo".into(),
        "Logo".into(),
        LayerType::Image {
            path: "assets/kagari_logo.webp".into(),
        },
        150,
    ));
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
    add_edge_streak(comp, "edge_l1", -1.0, 66, 320.0);
    add_edge_streak(comp, "edge_r1", 1.0, 68, 320.0);
    add_edge_streak(comp, "edge_l2", -1.0, 71, 220.0);
    add_edge_streak(comp, "edge_r2", 1.0, 73, 220.0);

    // === 衝撃波リング ===
    for (i, (size, thick, alpha, app, exp, fade)) in [
        (420.0, 5.0, 0.9, 44, 56, 72),
        (950.0, 3.0, 0.6, 47, 62, 78),
        (1500.0, 2.0, 0.35, 50, 68, 84),
    ]
    .into_iter()
    .enumerate()
    {
        let color = [1.0, 0.55, 0.12, alpha];
        comp.add_layer(Layer::new(
            format!("ring{}", i),
            format!("Ring {}", i),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(size),
                    height: Animatable::new_constant(size),
                },
                color: [color[0], color[1], color[2], 0.0],
                stroke_color: color,
                stroke_width: thick,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            150,
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
        [1.0, 0.7, 0.1, 1.0],
        [1.0, 0.5, 0.0, 1.0],
        [0.9, 0.3, 0.0, 1.0],
        [1.0, 0.85, 0.3, 1.0],
    ];
    for i in 0..8 {
        let ang = (i as f32) * 45.0 + 12.0;
        let rad = ang * std::f32::consts::PI / 180.0;
        let dist = 55.0 + (i as f32 * 13.0) % 45.0;
        let ex = 960.0 + rad.cos() * dist * 10.0;
        let ey = 480.0 + rad.sin() * dist * 10.0;
        let sz = 2.5 + (i as f32 * 0.6) % 3.0;
        comp.add_layer(Layer::new(
            format!("pt{}", i),
            format!("Particle {}", i),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(sz),
                    height: Animatable::new_constant(sz * 2.5),
                },
                color: pc[i % pc.len()],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            150,
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

}
