use super::*;

pub(super) fn add(comp: &mut Composition, ctr: [f32; 2], base: f32, lin: InterpolationType) {
    // flames: anticipation squash -> explosive overshoot -> recoil -> settle
    let flame_op = Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(40, 0.0, lin),
        k1(44, 45.0, ez4(E_SNAP)),
        k1(48, 100.0, ez4(E_SMOOTH)),
        k1(135, 100.0, lin),
        k1(150, 0.0, lin),
    ]);
    let burst = |d: u32, over: f32| {
        Animatable::new_animated(vec![
            k2(0, [0.0, 0.0], lin),
            k2(40 + d, [0.0, 0.0], lin),
            k2(44 + d, [base * 0.22, base * 0.13], ez4(E_SNAP)),
            k2(47 + d, [over, over * 0.96], ez4(E_SNAP)),
            k2(52 + d, [base * 0.95, base * 1.02], ez4(E_SMOOTH)),
            k2(58 + d, [base * 1.02, base * 0.99], ez4(E_SMOOTH)),
            k2(66 + d, [base, base], ez4(E_DRIFT)),
            k2(78 + d, [base * 1.035, base * 1.015], ez4(E_DRIFT)),
            k2(86 + d, [base * 0.99, base * 1.0], ez4(E_DRIFT)),
            k2(94 + d, [base * 1.012, base * 1.005], ez4(E_DRIFT)),
            k2(104 + d, [base, base], ez4(E_DRIFT)),
            k2(150, [base, base], lin),
        ])
    };
    let fo = flame_outer_pts();
    apart(
        comp,
        "flame_outer",
        fo.clone(),
        smooth_closed(&fo, &[0, 27]),
        [1.0; 4],
        flame_outer_fill(),
        c2(ctr),
        burst(0, base * 1.2),
        flame_op.clone(),
        150,
    );
    let su = shadow_ul_pts();
    apart(
        comp,
        "shade_ul",
        su.clone(),
        smooth_closed(&su, &[0, 11]),
        [1.0; 4],
        shadow_ul_fill(),
        c2(ctr),
        burst(1, base * 1.2),
        flame_op.clone(),
        150,
    );
    let fm = flame_mid_pts();
    apart(
        comp,
        "flame_mid",
        fm.clone(),
        smooth_closed(&fm, &[0, 10]),
        [1.0; 4],
        flame_mid_fill(),
        c2(ctr),
        burst(3, base * 1.17),
        flame_op.clone(),
        150,
    );
    let sh = half_light_l_pts();
    apart(
        comp,
        "light_l",
        sh.clone(),
        smooth_closed(&sh, &[0, 9]),
        [1.0; 4],
        half_light_l_fill(),
        c2(ctr),
        burst(2, base * 1.18),
        flame_op.clone(),
        150,
    );
    let lr = mirror(&sh);
    apart(
        comp,
        "shade_r",
        lr.clone(),
        smooth_closed(&lr, &[0, 9]),
        [1.0; 4],
        half_shade_r_fill(),
        c2(ctr),
        burst(2, base * 1.18),
        flame_op.clone(),
        150,
    );
    let core_scale = Animatable::new_animated(vec![
        k2(0, [0.0, 0.0], lin),
        k2(46, [0.0, 0.0], lin),
        k2(50, [base * 0.22, base * 0.13], ez4(E_SNAP)),
        k2(53, [base * 1.15, base * 1.1], ez4(E_SNAP)),
        k2(58, [base * 0.95, base * 1.02], ez4(E_SMOOTH)),
        k2(64, [base, base], ez4(E_SMOOTH)),
        k2(72, [base, base], ez4(E_DRIFT)),
        k2(80, [base, base], ez4(E_SNAP)),
        k2(85, [base * 1.12, base * 1.12], ez4(E_SNAP)),
        k2(91, [base, base], ez4(E_SMOOTH)),
        k2(104, [base, base], ez4(E_DRIFT)),
        k2(150, [base, base], lin),
    ]);
    let fc = flame_core_pts();
    apart(
        comp,
        "flame_core",
        fc.clone(),
        smooth_closed(&fc, &[0, 13]),
        [1.0, 0.973, 0.910, 1.0],
        flame_core_fill(),
        c2(ctr),
        core_scale,
        flame_op,
        150,
    );

    // ignition sparks
    particle_layer(
        comp,
        "sparks",
        ParticleEmitter {
            rate: 500.0,
            max_particles: 400,
            lifetime: 0.5,
            lifetime_variance: 0.35,
            speed: 550.0,
            speed_variance: 0.4,
            spread_degrees: 360.0,
            shape: EmitterShape::Point,
            gravity: [0.0, -60.0],
            wind: [0.0, 0.0],
            turbulence: 60.0,
            color_start: [1.0, 0.95, 0.85, 1.0],
            color_end: [1.0, 0.45, 0.1, 0.0],
            size_start: 6.0,
            size_end: 1.0,
            drag: 2.2,
            ..base_emitter()
        },
        ctr,
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(43, 0.0, lin),
            k1(45, 100.0, ez4(E_SNAP)),
            k1(50, 60.0, lin),
            k1(56, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // drifting embers
    particle_layer(
        comp,
        "embers",
        ParticleEmitter {
            rate: 22.0,
            max_particles: 120,
            lifetime: 3.2,
            lifetime_variance: 0.4,
            speed: 55.0,
            speed_variance: 0.5,
            spread_degrees: 360.0,
            shape: EmitterShape::Point,
            gravity: [0.0, -45.0],
            wind: [18.0, 0.0],
            turbulence: 40.0,
            color_start: [1.0, 0.6, 0.15, 0.8],
            color_end: [0.9, 0.2, 0.02, 0.0],
            size_start: 5.0,
            size_end: 1.0,
            drag: 0.4,
            ..base_emitter()
        },
        [960.0, 540.0],
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(46, 0.0, lin),
            k1(58, 65.0, ez4(E_SMOOTH)),
            k1(108, 65.0, lin),
            k1(122, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // titles
    let title_rise = |t0: u32, y_end: f32| {
        Animatable::new_animated(vec![
            k2(0, [960.0, y_end - 200.0], lin),
            k2(t0, [960.0, y_end - 200.0], lin),
            k2(t0 + 18, [960.0, y_end + 8.0], ez4(E_SNAP)),
            k2(t0 + 24, [960.0, y_end], ez4(E_SMOOTH)),
            k2(150, [960.0, y_end], lin),
        ])
    };
    let title_op = |t0: u32| {
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(t0, 0.0, lin),
            k1(t0 + 16, 100.0, ez4(E_SMOOTH)),
            k1(135, 100.0, lin),
            k1(150, 0.0, lin),
        ])
    };
    comp.add_layer(Layer::new(
        "title".into(),
        "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(),
            font_size: 80,
            color: [1.0; 4],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = title_rise(94, 835.0);
        t.transform.opacity = title_op(94);
    }
    comp.add_layer(Layer::new(
        "vfx".into(),
        "VFX".into(),
        LayerType::Text {
            text: "VFX".into(),
            font_size: 80,
            color: [0.9, 0.6, 0.1, 1.0],
            font_family: "SF Pro Display".into(),
            tracking: 20.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let v = comp.layers.last_mut().unwrap();
        v.transform.position = title_rise(98, 915.0);
        v.transform.opacity = title_op(98);
    }
    comp.add_layer(Layer::new(
        "line".into(),
        "Line".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_animated(vec![
                    k1(0, 0.0, lin),
                    k1(106, 0.0, lin),
                    k1(118, 62.0, ez4(E_SMOOTH)),
                    k1(150, 62.0, lin),
                ]),
                height: Animatable::new_constant(2.0),
            },
            color: [0.9, 0.6, 0.1, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 965.0]);
        l.transform.opacity = title_op(106);
    }
    comp.add_layer(Layer::new(
        "tag".into(),
        "Tagline".into(),
        LayerType::Text {
            text: "Motion Graphics & Compositing".into(),
            font_size: 24,
            color: [0.6; 4],
            font_family: "SF Pro Display".into(),
            tracking: 3.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 1010.0]);
        l.transform.opacity = title_op(114);
    }

    // photographic finish
    comp.add_layer(Layer::new(
        "finish".into(),
        "Finish".into(),
        LayerType::AdjustmentLayer,
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2([960.0, 540.0]);
        l.effects.push(Effect {
            id: "finish_grain".into(),
            name: "Film Grain".into(),
            effect_type: EffectType::FilmGrain {
                intensity: Animatable::new_animated(vec![
                    k1(0, 0.0, lin),
                    k1(40, 0.0, lin),
                    k1(70, 0.035, ez4(E_SMOOTH)),
                    k1(135, 0.035, lin),
                    k1(150, 0.0, lin),
                ]),
                grain_size: 1.0,
                color_film: false,
            },
            enabled: true,
        });
    }
}
