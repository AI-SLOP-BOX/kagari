use super::common::*;
use kagari_vfx::core::timeline::*;

pub fn add_signals_and_background(comp: &mut Composition) {
    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid {
            color: [0.012, 0.014, 0.018, 1.0],
        },
        DUR,
    ));
    comp.layers.last_mut().unwrap().transform.position = const_vec2([CX, CY]);

    // ---- 2. animated noise field (procedural, temporal) ----
    comp.add_layer(Layer::new(
        "noise_field".into(),
        "Noise Field".into(),
        LayerType::Solid {
            color: [0.5, 0.5, 0.5, 1.0],
        },
        DUR,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = const_vec2([CX, CY]);
        l.transform.opacity = anim_scalar(vec![
            key_scalar(0, 9.0, lin()),
            key_scalar(54, 5.0, lin()),
            key_scalar(126, 3.0, lin()),
            key_scalar(150, 3.0, lin()),
        ]);
        l.effects.push(make_effect(
            "n_fbm",
            "Fractal Noise",
            EffectType::FractalNoise {
                fractal_type: const_scalar(0.0),
                contrast: const_scalar(0.6),
                brightness: const_scalar(0.5),
                complexity: const_scalar(4.0),
                evolution: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(150, 6.0, lin()),
                ]),
            },
        ));
    }

    // ---- 3. signal light-lines (seeded, converging) ----
    let line_cols: [[f32; 4]; 3] = [
        [0.45, 0.9, 1.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
        [1.0, 0.62, 0.2, 1.0],
    ];
    for k in 0..18u64 {
        let x0 = 300.0 + hash_to_unit(k, 1) * 1300.0;
        let y0 = 150.0 + hash_to_unit(k, 2) * 780.0;
        let hero = k < 4;
        let wline = if hero {
            600.0 + hash_to_unit(k, 3) * 300.0
        } else {
            120.0 + hash_to_unit(k, 3) * 400.0
        };
        let hline = if hero {
            5.0
        } else if hash_to_unit(k, 4) > 0.6 {
            3.0
        } else {
            2.0
        };
        let col = line_cols[(hash_to_unit(k, 5) * 3.0) as usize % 3];
        let id = format!("sig{k}");
        comp.add_layer(Layer::new(
            id.clone(),
            id.clone(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    // SDF shape units: 100 = half comp width (960px here)
                    width: const_scalar(wline / 9.6),
                    height: const_scalar(hline / 9.6),
                    corner_radius: const_scalar(0.0),
                },
                color: col,
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            DUR,
        ));
        {
            let l = comp.layers.last_mut().unwrap();
            l.blend_mode = BlendMode::Add;
            let x1 = x0 * 0.3 + CX * 0.7;
            let y1 = y0 * 0.3 + CY * 0.7;
            l.transform.position = anim_vec2(vec![
                key_vec2(0, [x0, y0], hold()),
                key_vec2(100, [x1, y1], ease_bezier(E_SMOOTH)),
                key_vec2(150, [x1, y1], lin()),
            ]);
            // 2-3 hard-cut opacity spikes before the sync pulse
            let s1 = 4 + ((k * 7) % 20) as u32;
            let s2 = s1 + 6 + (hash_to_unit(k, 7) * 30.0) as u32;
            let o1 = if hero {
                100.0
            } else {
                70.0 + hash_to_unit(k, 8) * 30.0
            };
            let o2 = 35.0 + hash_to_unit(k, 9) * 40.0;
            l.transform.opacity = anim_scalar(vec![
                key_scalar(0, 0.0, hold()),
                key_scalar(s1, o1, hold()),
                key_scalar(s1 + 2, 0.0, hold()),
                key_scalar(s2.min(96), o2, hold()),
                key_scalar((s2 + 2).min(98), 0.0, hold()),
                key_scalar(150, 0.0, lin()),
            ]);
        }
    }

    // ---- 4. signal sweep band + sync converge lines ----
    comp.add_layer(Layer::new(
        "sweep".into(),
        "Sweep".into(),
        LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: const_scalar(200.0),
                height: const_scalar(6.0),
                corner_radius: const_scalar(0.0),
            },
            color: [0.7, 0.85, 1.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        DUR,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.blend_mode = BlendMode::Add;
        l.transform.position = anim_vec2(vec![
            key_vec2(0, [CX, 100.0], lin()),
            key_vec2(4, [CX, 100.0], lin()),
            key_vec2(22, [CX, 980.0], lin()),
            key_vec2(23, [CX, 980.0], lin()),
            key_vec2(150, [CX, 980.0], lin()),
        ]);
        l.transform.opacity = anim_scalar(vec![
            key_scalar(0, 0.0, lin()),
            key_scalar(4, 12.0, lin()),
            key_scalar(22, 12.0, lin()),
            key_scalar(23, 0.0, lin()),
            key_scalar(150, 0.0, lin()),
        ]);
    }

    // ---- 4b. sync converge lines (f96-104) ----
    for (k, y0) in [(0u64, 180.0), (1, 900.0)] {
        let id = format!("syncline{k}");
        comp.add_layer(Layer::new(
            id.clone(),
            id.clone(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: const_scalar(198.0),
                    height: const_scalar(0.21),
                    corner_radius: const_scalar(0.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            DUR,
        ));
        {
            let l = comp.layers.last_mut().unwrap();
            l.blend_mode = BlendMode::Add;
            l.transform.position = anim_vec2(vec![
                key_vec2(0, [CX, y0], lin()),
                key_vec2(95, [CX, y0], lin()),
                key_vec2(103, [CX, CY], ease_bezier(E_SMOOTH)),
                key_vec2(150, [CX, CY], lin()),
            ]);
            l.transform.opacity = anim_scalar(vec![
                key_scalar(0, 0.0, lin()),
                key_scalar(95, 0.0, lin()),
                key_scalar(97, 70.0, ease_bezier(E_SNAP)),
                key_scalar(103, 55.0, lin()),
                key_scalar(106, 0.0, lin()),
                key_scalar(150, 0.0, lin()),
            ]);
        }
    }
}
