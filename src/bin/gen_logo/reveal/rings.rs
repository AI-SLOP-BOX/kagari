use super::*;

pub(super) fn add(comp: &mut Composition, ctr: [f32; 2], base: f32, lin: InterpolationType) {
    // ring arcs IGNITE behind the traveling comets (fronts track comet angle).
    let front = |v0: f32, v1: f32, f0: u32, f1: u32| {
        Animatable::new_animated(vec![
            k1(0, v0, lin),
            k1(f0, v0, lin),
            k1(f1, v1, ez4(E_FASTIN)),
            k1(150, v1, lin),
        ])
    };
    let ring_op = Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(9, 0.0, lin),
        k1(14, 100.0, ez4(E_SMOOTH)),
        k1(135, 100.0, lin),
        k1(150, 0.0, lin),
    ]);
    let ring_draws: &[(RingDraw, bool)] = &[
        (
            ("ring_lb2", 34.9, 36.5, [1.0, 0.235, 0.04, 1.0], 10, 15),
            true,
        ),
        (
            ("ring_lb", 36.5, 39.0, [1.0, 0.267, 0.043, 1.0], 13, 19),
            true,
        ),
        (
            ("ring_lb2x", 39.0, 41.5, [1.0, 0.373, 0.059, 1.0], 16, 21),
            true,
        ),
        (
            ("ring_lb3", 41.5, 45.5, [1.0, 0.57, 0.106, 1.0], 19, 24),
            true,
        ),
        (
            ("ring_lm3", 45.5, 50.0, [1.0, 0.765, 0.157, 1.0], 22, 27),
            true,
        ),
        (
            ("ring_lm", 50.0, 54.5, [1.0, 0.745, 0.153, 1.0], 25, 30),
            true,
        ),
        (
            ("ring_lm2b", 54.5, 60.5, [1.0, 0.66, 0.133, 1.0], 28, 32),
            true,
        ),
        (
            ("ring_lm2a", 60.5, 66.5, [1.0, 0.62, 0.122, 1.0], 31, 35),
            true,
        ),
        (
            ("ring_lt", 66.5, 71.3, [1.0, 0.63, 0.125, 1.0], 34, 38),
            true,
        ),
        (
            ("ring_rt", 78.5, 84.0, [1.0, 0.64, 0.126, 1.0], 34, 38),
            false,
        ),
        (
            ("ring_rm", 84.0, 89.5, [1.0, 0.628, 0.126, 1.0], 32, 36),
            false,
        ),
        (
            ("ring_rm2", 89.5, 95.0, [1.0, 0.675, 0.137, 1.0], 30, 34),
            false,
        ),
        (
            ("ring_rm3a", 95.0, 100.0, [1.0, 0.745, 0.153, 1.0], 28, 33),
            false,
        ),
        (
            ("ring_rm3b", 0.0, 2.5, [1.0, 0.804, 0.161, 1.0], 25, 30),
            false,
        ),
        (
            ("ring_rm4", 2.5, 6.5, [1.0, 0.698, 0.137, 1.0], 22, 27),
            false,
        ),
        (
            ("ring_rb2", 6.5, 10.0, [1.0, 0.49, 0.09, 1.0], 19, 24),
            false,
        ),
        (
            ("ring_rb3", 10.0, 12.5, [1.0, 0.306, 0.051, 1.0], 16, 21),
            false,
        ),
        (
            ("ring_rb", 12.5, 15.1, [1.0, 0.235, 0.04, 1.0], 13, 19),
            false,
        ),
    ];
    for ((id, s, e, sc, d0, d1), grow_end) in ring_draws.iter().copied() {
        let (st, en) = if grow_end {
            (c1(s), front(s, e, d0, d1))
        } else {
            (front(e, s, d0, d1), c1(e))
        };
        ring_layer(
            comp,
            id,
            sc,
            RING_U,
            11.0,
            st,
            en,
            ctr,
            c2([base, base]),
            ring_op.clone(),
            false,
            150,
        );
    }

    // ring charge: a thicker, hotter arc set fades in over the base ring as
    // the climax charge (thin -> luminous band), then settles to a glow.
    let charge_op = Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(68, 0.0, lin),
        k1(86, 80.0, ez4(E_SMOOTH)),
        k1(100, 55.0, ez4(E_DRIFT)),
        k1(135, 55.0, lin),
        k1(150, 0.0, lin),
    ]);
    let charge_cols: &[[f32; 4]] = &[
        [1.0, 0.90, 0.55, 1.0],
        [1.0, 0.68, 0.25, 1.0],
        [1.0, 0.40, 0.12, 1.0],
    ];
    for ((id, s, e, _, _, _), _) in ring_draws.iter().copied() {
        let ci = if id == "ring_lt" || id == "ring_rt" {
            0
        } else if id == "ring_lb" || id == "ring_lb2" || id == "ring_rb" || id == "ring_rb3" {
            2
        } else {
            1
        };
        let cid = format!("{id}_chg");
        comp.add_layer(Layer::new(
            cid.clone(),
            cid,
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(RING_U),
                    height: Animatable::new_constant(RING_U),
                },
                color: [0.0; 4],
                stroke_color: charge_cols[ci.min(2)],
                stroke_width: 16.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.scale = c2([base, base]);
        l.transform.opacity = charge_op.clone();
        l.trim_paths = Some(TrimPaths {
            start: c1(s),
            end: c1(e),
            offset: Animatable::new_constant(0.0),
        });
    }

    // traveling energy pulses: bright windows riding just ahead of each front.
    pulse_layer(
        comp,
        "pulse_l",
        front(17.0, 65.0, 10, 35),
        front(27.0, 75.0, 10, 35),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(10, 0.0, lin),
            k1(13, 100.0, ez4(E_SMOOTH)),
            k1(35, 100.0, lin),
            k1(39, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
    pulse_layer(
        comp,
        "pulse_r1",
        front(13.0, 0.0, 12, 24),
        front(25.0, 12.0, 12, 24),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(12, 0.0, lin),
            k1(15, 100.0, ez4(E_SMOOTH)),
            k1(22, 100.0, lin),
            k1(26, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
    pulse_layer(
        comp,
        "pulse_r2",
        front(88.0, 77.0, 24, 37),
        front(100.0, 89.0, 24, 37),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(23, 0.0, lin),
            k1(26, 100.0, ez4(E_SMOOTH)),
            k1(37, 100.0, lin),
            k1(41, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
    pulse_layer(
        comp,
        "pulse_back",
        front(20.0, 75.0, 70, 86),
        front(32.0, 87.0, 70, 86),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(70, 0.0, lin),
            k1(73, 85.0, ez4(E_SMOOTH)),
            k1(84, 85.0, lin),
            k1(88, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // comet heads: white-hot cores riding the fronts, then diving to center.
    let comet_keys_l = vec![
        k2(0, comet_pos(25.0), lin),
        k2(10, comet_pos(25.0), lin),
        k2(16, comet_pos(32.0), ez4(E_FASTIN)),
        k2(22, comet_pos(41.0), ez4(E_FASTIN)),
        k2(27, comet_pos(52.0), ez4(E_FASTIN)),
        k2(31, comet_pos(62.0), ez4(E_FASTIN)),
        k2(34, comet_pos(70.0), ez4(E_FASTIN)),
        k2(36, comet_pos(75.0), ez4(E_FASTIN)),
        k2(40, [960.0, 220.0], ez4(E_FASTIN)),
        k2(44, ctr, ez4(E_SNAP)),
        k2(150, ctr, lin),
    ];
    let comet_keys_r = vec![
        k2(0, comet_pos(25.0), lin),
        k2(12, comet_pos(25.0), lin),
        k2(16, comet_pos(20.0), ez4(E_FASTIN)),
        k2(20, comet_pos(10.0), ez4(E_FASTIN)),
        k2(23, comet_pos(2.0), ez4(E_FASTIN)),
        k2(26, comet_pos(94.0), ez4(E_FASTIN)),
        k2(30, comet_pos(86.0), ez4(E_FASTIN)),
        k2(34, comet_pos(80.0), ez4(E_FASTIN)),
        k2(38, comet_pos(77.0), ez4(E_FASTIN)),
        k2(42, [962.0, 222.0], ez4(E_FASTIN)),
        k2(46, ctr, ez4(E_SNAP)),
        k2(150, ctr, lin),
    ];
    for (side, keys, d) in [("l", comet_keys_l, 0), ("r", comet_keys_r, 2)] {
        let head_op = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(9 + d, 0.0, lin),
            k1(12 + d, 100.0, ez4(E_SMOOTH)),
            k1(42 + d, 100.0, lin),
            k1(48 + d, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]);
        comp.add_layer(Layer::new(
            format!("comet_{side}"),
            format!("Comet {side}"),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(7.0),
                    height: Animatable::new_constant(7.0),
                },
                color: [1.0, 0.98, 0.92, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            150,
        ));
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_animated(keys.clone());
        l.transform.opacity = head_op;
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [100.0, 100.0], lin),
            k2(35 + d, [100.0, 100.0], lin),
            k2(40 + d, [70.0, 190.0], ez4(E_FASTIN)),
            k2(45 + d, [110.0, 90.0], ez4(E_SNAP)),
            k2(50 + d, [100.0, 100.0], ez4(E_SMOOTH)),
            k2(150, [100.0, 100.0], lin),
        ]);
        halo(
            comp,
            &format!("comet_halo_{side}"),
            46.0,
            230.0,
            [1.0, 0.72, 0.3, 0.85],
            Animatable::new_animated(keys),
            c2([100.0, 100.0]),
            Animatable::new_animated(vec![
                k1(0, 0.0, lin),
                k1(9 + d, 0.0, lin),
                k1(13 + d, 85.0, ez4(E_SMOOTH)),
                k1(42 + d, 85.0, lin),
                k1(49 + d, 0.0, ez4(E_FASTIN)),
                k1(150, 0.0, lin),
            ]),
            150,
        );
    }

    // convergence streams: ring-shaped emitter pulled to center (f34-50).
    particle_layer(
        comp,
        "converge",
        ParticleEmitter {
            rate: 260.0,
            max_particles: 400,
            lifetime: 0.35,
            lifetime_variance: 0.25,
            speed: 30.0,
            speed_variance: 0.6,
            spread_degrees: 360.0,
            shape: EmitterShape::Ring,
            emitter_size: [606.0, 606.0],
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            turbulence: 20.0,
            color_start: [1.0, 0.78, 0.32, 0.9],
            color_end: [1.0, 0.3, 0.05, 0.0],
            size_start: 7.0,
            size_end: 1.0,
            opacity_start: 1.0,
            opacity_end: 0.0,
            drag: 1.2,
            attract_strength: 1400.0,
            attract_center: [960.0, 430.0],
            ..base_emitter()
        },
        ctr,
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(33, 0.0, lin),
            k1(38, 90.0, ez4(E_SMOOTH)),
            k1(44, 70.0, lin),
            k1(50, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
}
