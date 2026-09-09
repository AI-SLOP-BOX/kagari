use kagari_vfx::core::timeline::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::project_migration::save_project_atomic;

// NOTE: FreeformBezier tangents are ABSOLUTE handle positions.
// Straight edge => handle == anchor. Smooth => handles along (next-prev).
type Pt2 = [f32; 2];

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

fn smooth_closed(points: &[Pt2], corners: &[usize]) -> Vec<(Pt2, Pt2)> {
    let n = points.len();
    (0..n)
        .map(|i| {
            if corners.contains(&i) {
                (points[i], points[i])
            } else {
                let prev = points[(i + n - 1) % n];
                let next = points[(i + 1) % n];
                let t = [(next[0] - prev[0]) * 0.22, (next[1] - prev[1]) * 0.22];
                (
                    [points[i][0] - t[0], points[i][1] - t[1]],
                    [points[i][0] + t[0], points[i][1] + t[1]],
                )
            }
        })
        .collect()
}

fn sharp(points: &[Pt2]) -> Vec<(Pt2, Pt2)> {
    points.iter().map(|p| (*p, *p)).collect()
}

// ─── Logo part path data (units; 1 unit == comp/200 px at scale 100) ───

fn flame_outer_pts() -> Vec<Pt2> {
    vec![
        [0.0, -78.0],
        [4.0, -64.0],
        [9.0, -51.0],
        [18.0, -39.0],
        [27.0, -26.0],
        [31.0, -13.0],
        [31.0, -3.0],
        [24.0, 16.0],
        [8.0, 38.0],
        [0.0, 44.0],
        [-8.0, 38.0],
        [-24.0, 16.0],
        [-31.0, -3.0],
        [-31.0, -13.0],
        [-27.0, -26.0],
        [-18.0, -39.0],
        [-9.0, -51.0],
        [-4.0, -64.0],
    ]
}

fn flame_mid_pts() -> Vec<Pt2> {
    vec![
        [0.0, -44.0],
        [3.0, -36.0],
        [6.0, -28.0],
        [11.0, -21.0],
        [16.0, -13.0],
        [19.0, -5.0],
        [19.0, -2.0],
        [14.0, 10.0],
        [5.0, 23.0],
        [0.0, 26.0],
        [-5.0, 23.0],
        [-14.0, 10.0],
        [-19.0, -2.0],
        [-19.0, -5.0],
        [-16.0, -13.0],
        [-11.0, -21.0],
        [-6.0, -28.0],
        [-3.0, -36.0],
    ]
}

fn flame_core_pts() -> Vec<Pt2> {
    vec![
        [0.0, -22.0],
        [2.0, -18.0],
        [4.0, -14.0],
        [7.0, -10.0],
        [9.0, -4.0],
        [9.0, 0.0],
        [7.0, 6.0],
        [3.0, 11.0],
        [0.0, 13.0],
        [-3.0, 11.0],
        [-7.0, 6.0],
        [-9.0, 0.0],
        [-9.0, -4.0],
        [-7.0, -10.0],
        [-4.0, -14.0],
        [-2.0, -18.0],
    ]
}

fn shade_l_pts() -> Vec<Pt2> {
    vec![
        [-9.0, -50.0],
        [-18.0, -36.0],
        [-26.0, -20.0],
        [-31.0, -4.0],
        [-30.0, 12.0],
        [-24.0, 26.0],
        [-14.0, 36.0],
        [-10.0, 30.0],
        [-20.0, 18.0],
        [-24.0, 4.0],
        [-22.0, -12.0],
        [-15.0, -28.0],
        [-7.0, -44.0],
    ]
}

fn mirror(pts: &[Pt2]) -> Vec<Pt2> {
    pts.iter().map(|p| [-p[0], p[1]]).collect()
}

fn shield_l_pts() -> Vec<Pt2> {
    vec![[0.0, 45.0], [-12.0, 51.0], [-12.0, 65.0], [0.0, 70.0]]
}

fn wing_dark_l_pts() -> Vec<Pt2> {
    vec![[-62.0, 12.0], [-39.0, 2.0], [-32.0, 31.0]]
}

fn wing_blade_l_pts() -> Vec<Pt2> {
    vec![[-60.0, 6.0], [-56.0, 8.0], [-39.0, 2.0], [-44.0, 10.0]]
}

fn flame_outer_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -370.0],
        end: [0.0, 220.0],
        colors: vec![
            [0.72, 0.07, 0.03, 1.0],
            [0.95, 0.28, 0.04, 1.0],
            [1.0, 0.55, 0.08, 1.0],
            [1.0, 0.76, 0.26, 1.0],
        ],
        stops: vec![0.0, 0.45, 0.75, 1.0],
    }
}

fn flame_mid_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -220.0],
        end: [0.0, 130.0],
        colors: vec![
            [1.0, 0.55, 0.08, 1.0],
            [1.0, 0.78, 0.28, 1.0],
            [1.0, 0.94, 0.70, 1.0],
        ],
        stops: vec![0.0, 0.55, 1.0],
    }
}

fn flame_core_fill() -> ShapeFillType {
    ShapeFillType::RadialGradient {
        center: [0.0, 30.0],
        radius: 110.0,
        colors: vec![[1.0, 1.0, 0.98, 1.0], [1.0, 0.90, 0.68, 1.0]],
        stops: vec![0.0, 1.0],
    }
}

#[allow(clippy::too_many_arguments)]
fn apart(
    comp: &mut Composition,
    id: &str,
    pts: Vec<Pt2>,
    tans: Vec<(Pt2, Pt2)>,
    color: [f32; 4],
    fill: ShapeFillType,
    pos: Animatable<[f32; 2]>,
    scale: Animatable<[f32; 2]>,
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: pts,
                tangents: tans,
                closed: true,
            },
            color,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: fill,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = pos;
    l.transform.scale = scale;
    l.transform.opacity = opacity;
}

#[allow(clippy::too_many_arguments)]
fn ring_layer(
    comp: &mut Composition,
    id: &str,
    color: [f32; 4],
    start: Animatable<f32>,
    end: Animatable<f32>,
    pos: [f32; 2],
    scale: Animatable<[f32; 2]>,
    opacity: Animatable<f32>,
    glow: bool,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(140.0),
                height: Animatable::new_constant(140.0),
            },
            color: [0.0; 4],
            stroke_color: color,
            stroke_width: 11.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_constant(pos);
    l.transform.scale = scale;
    l.transform.opacity = opacity;
    l.trim_paths = Some(TrimPaths {
        start,
        end,
        offset: Animatable::new_constant(0.0),
    });
    if glow {
        l.effects.push(Effect {
            id: format!("{id}_glow"),
            name: "Ring Glow".into(),
            effect_type: EffectType::Glow {
                threshold: Animatable::new_constant(62.0),
                radius: Animatable::new_constant(10.0),
                intensity: Animatable::new_constant(38.0),
                color: Animatable::new_constant([1.0, 0.5, 0.1, 1.0]),
            },
            enabled: true,
        });
    }
}

type RingDraw = (&'static str, f32, f32, [f32; 4], u32, u32);

fn c2(v: [f32; 2]) -> Animatable<[f32; 2]> {
    Animatable::new_constant(v)
}
fn c1(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

// ─── Still composition (1000x1000, quality reference) ───

fn build_still() -> Composition {
    let mut comp = Composition::new("LogoStill".into(), "Kagari Vector Logo".into(), 1000, 1000, 30, 2);
    let ctr = [500.0, 500.0];
    let s100 = c2([100.0, 100.0]);
    let o100 = c1(100.0);

    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid {
            color: [0.015, 0.01, 0.01, 1.0],
        },
        2,
    ));
    comp.layers.last_mut().unwrap().transform.position = c2(ctr);

    // atmosphere
    comp.add_layer(Layer::new(
        "atmos".into(),
        "Atmosphere".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(170.0),
                height: Animatable::new_constant(170.0),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::RadialGradient {
                center: [0.0, 0.0],
                radius: 430.0,
                colors: vec![[0.55, 0.20, 0.04, 0.55], [0.0, 0.0, 0.0, 0.0]],
                stops: vec![0.0, 1.0],
            },
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        2,
    ));
    comp.layers.last_mut().unwrap().transform.position = c2(ctr);

    // shield
    let sl = shield_l_pts();
    apart(&mut comp, "shield_l", sl.clone(), sharp(&sl), [0.55, 0.53, 0.48, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let sr = mirror(&sl);
    apart(&mut comp, "shield_r", sr.clone(), sharp(&sr), [0.09, 0.10, 0.15, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);

    // wings
    let wd = wing_dark_l_pts();
    apart(&mut comp, "wing_dl", wd.clone(), sharp(&wd), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wdr = mirror(&wd);
    apart(&mut comp, "wing_dr", wdr.clone(), sharp(&wdr), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wb = wing_blade_l_pts();
    apart(&mut comp, "wing_l", wb.clone(), sharp(&wb), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wbr = mirror(&wb);
    apart(&mut comp, "wing_r", wbr.clone(), sharp(&wbr), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);

    // ring arcs (static trims)
    let segs: &[(&str, f32, f32, [f32; 4])] = &[
        ("ring_lt", 60.0, 73.0, [1.0, 0.78, 0.25, 1.0]),
        ("ring_lm", 40.0, 60.0, [1.0, 0.52, 0.10, 1.0]),
        ("ring_lb", 27.0, 40.0, [0.88, 0.22, 0.06, 1.0]),
        ("ring_rt", 77.0, 90.0, [1.0, 0.78, 0.25, 1.0]),
        ("ring_rm1", 90.0, 100.0, [1.0, 0.52, 0.10, 1.0]),
        ("ring_rm2", 0.0, 10.0, [1.0, 0.52, 0.10, 1.0]),
        ("ring_rb", 10.0, 23.0, [0.88, 0.22, 0.06, 1.0]),
    ];
    for (id, s, e, sc) in segs.iter().copied() {
        ring_layer(&mut comp, id, sc, c1(s), c1(e), ctr, s100.clone(), o100.clone(), true, 2);
    }

    // flames
    let fo = flame_outer_pts();
    apart(&mut comp, "flame_outer", fo.clone(), smooth_closed(&fo, &[0, 9]), [1.0; 4], flame_outer_fill(), c2(ctr), s100.clone(), o100.clone(), 2);
    let fm = flame_mid_pts();
    apart(&mut comp, "flame_mid", fm.clone(), smooth_closed(&fm, &[0, 9]), [1.0; 4], flame_mid_fill(), c2(ctr), s100.clone(), o100.clone(), 2);
    let sh = shade_l_pts();
    apart(&mut comp, "shade_l", sh.clone(), smooth_closed(&sh, &[]), [0.55, 0.05, 0.02, 0.38], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let lr = mirror(&sh);
    apart(&mut comp, "light_r", lr.clone(), smooth_closed(&lr, &[]), [1.0, 0.75, 0.35, 0.30], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let fc = flame_core_pts();
    apart(&mut comp, "flame_core", fc.clone(), smooth_closed(&fc, &[0, 8]), [1.0; 4], flame_core_fill(), c2(ctr), s100.clone(), o100.clone(), 2);

    comp
}

// ─── Reveal composition (1920x1080): parts assemble into the mark ───

fn appear_op(delay: u32, full: u32, out: u32) -> Animatable<f32> {
    Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(delay, 0.0, InterpolationType::Linear),
        Keyframe::new(full, 100.0, bez_in(0.5)),
        Keyframe::new(out, 100.0, InterpolationType::Linear),
        Keyframe::new(150, 0.0, InterpolationType::Linear),
    ])
}

fn grow_scale(delay: u32, over: u32, settle: u32, base: f32) -> Animatable<[f32; 2]> {
    Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(delay, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(over, [base * 1.15, base * 1.15], bez_both(0.1, 0.65)),
        Keyframe::new(settle, [base, base], bez_in(0.4)),
        Keyframe::new(150, [base, base], InterpolationType::Linear),
    ])
}

fn build_reveal() -> Composition {
    let mut comp = Composition::new("LogoReveal".into(), "Kagari Logo Reveal".into(), 1920, 1080, 30, 150);
    let ctr = [960.0, 430.0];
    let base = 30.0;

    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid {
            color: [0.015, 0.01, 0.01, 1.0],
        },
        150,
    ));
    comp.layers.last_mut().unwrap().transform.position = c2(ctr);

    // atmosphere fades in early
    comp.add_layer(Layer::new(
        "atmos".into(),
        "Atmosphere".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(170.0),
                height: Animatable::new_constant(170.0),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::RadialGradient {
                center: [0.0, 0.0],
                radius: 430.0,
                colors: vec![[0.55, 0.20, 0.04, 0.55], [0.0, 0.0, 0.0, 0.0]],
                stops: vec![0.0, 1.0],
            },
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(10, 0.0, InterpolationType::Linear),
            Keyframe::new(35, 100.0, InterpolationType::Linear),
            Keyframe::new(135, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    // ignition flash
    comp.add_layer(Layer::new(
        "flash".into(),
        "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(120.0),
                height: Animatable::new_constant(120.0),
            },
            color: [1.0, 0.97, 0.88, 1.0],
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
        l.transform.position = c2(ctr);
        l.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(24, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(28, [10.0, 10.0], InterpolationType::Linear),
            Keyframe::new(36, [14.0, 14.0], InterpolationType::Linear),
            Keyframe::new(52, [16.0, 16.0], InterpolationType::Linear),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(24, 0.0, InterpolationType::Linear),
            Keyframe::new(27, 85.0, InterpolationType::Linear),
            Keyframe::new(34, 40.0, InterpolationType::Linear),
            Keyframe::new(52, 0.0, InterpolationType::Linear),
        ]);
    }

    // shockwave ring stroke
    comp.add_layer(Layer::new(
        "shock".into(),
        "Shockwave".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(700.0),
                height: Animatable::new_constant(700.0),
            },
            color: [1.0, 0.6, 0.15, 0.0],
            stroke_color: [1.0, 0.6, 0.15, 0.9],
            stroke_width: 6.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.scale = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(26, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(48, [1.0, 1.0], bez_out(0.55)),
            Keyframe::new(64, [1.1, 1.1], InterpolationType::Linear),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(26, 0.0, InterpolationType::Linear),
            Keyframe::new(28, 100.0, InterpolationType::Linear),
            Keyframe::new(56, 60.0, InterpolationType::Linear),
            Keyframe::new(64, 0.0, InterpolationType::Linear),
        ]);
    }

    // wings slide in from the sides
    let slide = |from: f32, delay: u32| {
        Animatable::new_animated(vec![
            Keyframe::new(0, [ctr[0] + from, ctr[1]], InterpolationType::Linear),
            Keyframe::new(delay, [ctr[0] + from, ctr[1]], InterpolationType::Linear),
            Keyframe::new(delay + 26, ctr, bez_both(0.15, 0.6)),
        ])
    };
    let wd = wing_dark_l_pts();
    apart(&mut comp, "wing_dl", wd.clone(), sharp(&wd), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid,
        slide(-320.0, 18), c2([base, base]), appear_op(18, 40, 135), 150);
    let wdr = mirror(&wd);
    apart(&mut comp, "wing_dr", wdr.clone(), sharp(&wdr), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid,
        slide(320.0, 18), c2([base, base]), appear_op(18, 40, 135), 150);
    let wb = wing_blade_l_pts();
    apart(&mut comp, "wing_l", wb.clone(), sharp(&wb), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid,
        slide(-320.0, 22), c2([base, base]), appear_op(22, 44, 135), 150);
    let wbr = mirror(&wb);
    apart(&mut comp, "wing_r", wbr.clone(), sharp(&wbr), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid,
        slide(320.0, 22), c2([base, base]), appear_op(22, 44, 135), 150);

    // shield rises from below
    let rise = Animatable::new_animated(vec![
        Keyframe::new(0, [ctr[0], ctr[1] + 220.0], InterpolationType::Linear),
        Keyframe::new(18, [ctr[0], ctr[1] + 220.0], InterpolationType::Linear),
        Keyframe::new(44, ctr, bez_both(0.15, 0.6)),
    ]);
    let sl = shield_l_pts();
    apart(&mut comp, "shield_l", sl.clone(), sharp(&sl), [0.55, 0.53, 0.48, 1.0], ShapeFillType::Solid,
        rise.clone(), c2([base, base]), appear_op(18, 40, 135), 150);
    let sr = mirror(&sl);
    apart(&mut comp, "shield_r", sr.clone(), sharp(&sr), [0.09, 0.10, 0.15, 1.0], ShapeFillType::Solid,
        rise, c2([base, base]), appear_op(18, 40, 135), 150);

    // ring arcs DRAW ON (trim end grows)
    let draw_on = |s: f32, e: f32, d0: u32, d1: u32| {
        Animatable::new_animated(vec![
            Keyframe::new(0, s, InterpolationType::Linear),
            Keyframe::new(d0, s, InterpolationType::Linear),
            Keyframe::new(d1, e, bez_in(0.45)),
        ])
    };
    let ring_draws: &[RingDraw] = &[
        ("ring_lt", 60.0, 73.0, [1.0, 0.78, 0.25, 1.0], 12, 38),
        ("ring_lm", 40.0, 60.0, [1.0, 0.52, 0.10, 1.0], 16, 42),
        ("ring_lb", 27.0, 40.0, [0.88, 0.22, 0.06, 1.0], 20, 46),
        ("ring_rt", 77.0, 90.0, [1.0, 0.78, 0.25, 1.0], 12, 38),
        ("ring_rm1", 90.0, 100.0, [1.0, 0.52, 0.10, 1.0], 16, 42),
        ("ring_rm2", 0.0, 10.0, [1.0, 0.52, 0.10, 1.0], 16, 42),
        ("ring_rb", 10.0, 23.0, [0.88, 0.22, 0.06, 1.0], 20, 46),
    ];
    for (id, s, e, sc, d0, d1) in ring_draws.iter().copied() {
        ring_layer(&mut comp, id, sc, c1(s), draw_on(s, e, d0, d1), ctr,
            grow_scale(10, 40, 60, base), appear_op(10, 30, 135), false, 150);
    }

    // flames ignite with staggered scale-up
    let fo = flame_outer_pts();
    apart(&mut comp, "flame_outer", fo.clone(), smooth_closed(&fo, &[0, 9]), [1.0; 4], flame_outer_fill(),
        c2(ctr), grow_scale(24, 52, 66, base), appear_op(24, 44, 135), 150);
    let fm = flame_mid_pts();
    apart(&mut comp, "flame_mid", fm.clone(), smooth_closed(&fm, &[0, 9]), [1.0; 4], flame_mid_fill(),
        c2(ctr), grow_scale(29, 56, 68, base), appear_op(29, 47, 135), 150);
    let sh = shade_l_pts();
    apart(&mut comp, "shade_l", sh.clone(), smooth_closed(&sh, &[]), [0.55, 0.05, 0.02, 0.38], ShapeFillType::Solid,
        c2(ctr), grow_scale(33, 58, 70, base), appear_op(33, 50, 135), 150);
    let lr = mirror(&sh);
    apart(&mut comp, "light_r", lr.clone(), smooth_closed(&lr, &[]), [1.0, 0.75, 0.35, 0.30], ShapeFillType::Solid,
        c2(ctr), grow_scale(33, 58, 70, base), appear_op(33, 50, 135), 150);
    let fc = flame_core_pts();
    apart(&mut comp, "flame_core", fc.clone(), smooth_closed(&fc, &[0, 8]), [1.0; 4], flame_core_fill(),
        c2(ctr), grow_scale(36, 60, 72, base), appear_op(36, 52, 135), 150);

    // ---- titles ----
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
            Keyframe::new(78, [960.0, 600.0], InterpolationType::Linear),
            Keyframe::new(102, [960.0, 800.0], bez_both(0.15, 0.65)),
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
            Keyframe::new(81, [960.0, 600.0], InterpolationType::Linear),
            Keyframe::new(106, [960.0, 880.0], bez_both(0.15, 0.65)),
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
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 950.0]);
        l.transform.opacity = Animatable::new_animated(vec![
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
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 995.0]);
        l.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(112, 0.0, InterpolationType::Linear),
            Keyframe::new(128, 100.0, InterpolationType::Linear),
            Keyframe::new(138, 100.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    comp
}

fn main() {
    let project = Project {
        compositions: vec![build_still(), build_reveal()],
        ..Default::default()
    };
    save_project_atomic(&project, "logo_project.json").unwrap();
    println!("Wrote logo_project.json");
}
