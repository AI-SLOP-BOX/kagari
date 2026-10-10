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
    comp.layers.last_mut().unwrap().transform.position = c2([CX, CY]);

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
        l.transform.position = c2([CX, CY]);
        l.transform.opacity = a1(vec![
            k1(0, 9.0, lin()),
            k1(54, 5.0, lin()),
            k1(126, 3.0, lin()),
            k1(150, 3.0, lin()),
        ]);
        l.effects.push(fx(
            "n_fbm",
            "Fractal Noise",
            EffectType::FractalNoise {
                fractal_type: c1(0.0),
                contrast: c1(0.6),
                brightness: c1(0.5),
                complexity: c1(4.0),
                evolution: a1(vec![k1(0, 0.0, lin()), k1(150, 6.0, lin())]),
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
        let x0 = 300.0 + bf(k, 1) * 1300.0;
        let y0 = 150.0 + bf(k, 2) * 780.0;
        let hero = k < 4;
        let wline = if hero {
            600.0 + bf(k, 3) * 300.0
        } else {
            120.0 + bf(k, 3) * 400.0
        };
        let hline = if hero {
            5.0
        } else if bf(k, 4) > 0.6 {
            3.0
        } else {
            2.0
        };
        let col = line_cols[(bf(k, 5) * 3.0) as usize % 3];
        let id = format!("sig{k}");
        comp.add_layer(Layer::new(
            id.clone(),
            id.clone(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    // SDF shape units: 100 = half comp width (960px here)
                    width: c1(wline / 9.6),
                    height: c1(hline / 9.6),
                    corner_radius: c1(0.0),
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
            l.transform.position = a2(vec![
                k2(0, [x0, y0], hold()),
                k2(100, [x1, y1], ez(E_SMOOTH)),
                k2(150, [x1, y1], lin()),
            ]);
            // 2-3 hard-cut opacity spikes before the sync pulse
            let s1 = 4 + ((k * 7) % 20) as u32;
            let s2 = s1 + 6 + (bf(k, 7) * 30.0) as u32;
            let o1 = if hero { 100.0 } else { 70.0 + bf(k, 8) * 30.0 };
            let o2 = 35.0 + bf(k, 9) * 40.0;
            l.transform.opacity = a1(vec![
                k1(0, 0.0, hold()),
                k1(s1, o1, hold()),
                k1(s1 + 2, 0.0, hold()),
                k1(s2.min(96), o2, hold()),
                k1((s2 + 2).min(98), 0.0, hold()),
                k1(150, 0.0, lin()),
            ]);
        }
    }

    // ---- 4. signal sweep band + sync converge lines ----
    comp.add_layer(Layer::new(
        "sweep".into(),
        "Sweep".into(),
        LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: c1(200.0),
                height: c1(6.0),
                corner_radius: c1(0.0),
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
        l.transform.position = a2(vec![
            k2(0, [CX, 100.0], lin()),
            k2(4, [CX, 100.0], lin()),
            k2(22, [CX, 980.0], lin()),
            k2(23, [CX, 980.0], lin()),
            k2(150, [CX, 980.0], lin()),
        ]);
        l.transform.opacity = a1(vec![
            k1(0, 0.0, lin()),
            k1(4, 12.0, lin()),
            k1(22, 12.0, lin()),
            k1(23, 0.0, lin()),
            k1(150, 0.0, lin()),
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
                    width: c1(198.0),
                    height: c1(0.21),
                    corner_radius: c1(0.0),
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
            l.transform.position = a2(vec![
                k2(0, [CX, y0], lin()),
                k2(95, [CX, y0], lin()),
                k2(103, [CX, CY], ez(E_SMOOTH)),
                k2(150, [CX, CY], lin()),
            ]);
            l.transform.opacity = a1(vec![
                k1(0, 0.0, lin()),
                k1(95, 0.0, lin()),
                k1(97, 70.0, ez(E_SNAP)),
                k1(103, 55.0, lin()),
                k1(106, 0.0, lin()),
                k1(150, 0.0, lin()),
            ]);
        }
    }

}
