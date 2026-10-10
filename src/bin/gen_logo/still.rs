use super::geometry::*;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::timeline::*;

pub fn c2(v: [f32; 2]) -> Animatable<[f32; 2]> {
    Animatable::new_constant(v)
}

pub fn c1(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

#[allow(clippy::too_many_arguments)]
pub fn apart(
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
pub fn ring_layer(
    comp: &mut Composition,
    id: &str,
    color: [f32; 4],
    diam: f32,
    sw: f32,
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
                width: Animatable::new_constant(diam),
                height: Animatable::new_constant(diam),
            },
            color: [0.0; 4],
            stroke_color: color,
            stroke_width: sw,
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

// ─── Still composition (1000x1000, quality reference) ───

pub fn build_still() -> Composition {
    let mut comp = Composition::new(
        "LogoStill".into(),
        "Kagari Vector Logo".into(),
        1000,
        1000,
        30,
        2,
    );
    let ctr = [492.0, 476.0];
    let s100 = c2([100.0, 100.0]);
    let o100 = c1(100.0);

    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid {
            color: [0.247, 0.247, 0.247, 1.0],
        },
        2,
    ));
    comp.layers.last_mut().unwrap().transform.position = c2([500.0, 500.0]);

    // cube pedestal (light left face / navy right face)
    let sl = shield_l_pts();
    apart(
        &mut comp,
        "cube_l",
        sl.clone(),
        sharp(&sl),
        [0.757, 0.722, 0.655, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let sr = cube_r_pts();
    apart(
        &mut comp,
        "cube_r",
        sr.clone(),
        sharp(&sr),
        [0.122, 0.141, 0.169, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );

    // wings (navy under, white blade over)
    let wd = wing_dark_l_pts();
    apart(
        &mut comp,
        "wing_dl",
        wd.clone(),
        sharp(&wd),
        [0.122, 0.141, 0.169, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let wdr = mirror(&wd);
    apart(
        &mut comp,
        "wing_dr",
        wdr.clone(),
        sharp(&wdr),
        [0.122, 0.141, 0.169, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let wb = wing_blade_l_pts();
    apart(
        &mut comp,
        "wing_l",
        wb.clone(),
        sharp(&wb),
        [1.0, 0.973, 0.910, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let wbr = mirror(&wb);
    apart(
        &mut comp,
        "wing_r",
        wbr.clone(),
        sharp(&wbr),
        [1.0, 0.973, 0.910, 1.0],
        ShapeFillType::Solid,
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );

    // ring arcs (two tapered gradient arcs, flat vector look)
    let segs: &[(&str, f32, f32, [f32; 4])] = &[
        ("ring_lb2", 34.9, 36.5, [1.0, 0.235, 0.04, 1.0]),
        ("ring_lb", 36.5, 39.0, [1.0, 0.267, 0.043, 1.0]),
        ("ring_lb2x", 39.0, 41.5, [1.0, 0.373, 0.059, 1.0]),
        ("ring_lb3", 41.5, 45.5, [1.0, 0.57, 0.106, 1.0]),
        ("ring_lm3", 45.5, 50.0, [1.0, 0.765, 0.157, 1.0]),
        ("ring_lm", 50.0, 54.5, [1.0, 0.745, 0.153, 1.0]),
        ("ring_lm2b", 54.5, 60.5, [1.0, 0.66, 0.133, 1.0]),
        ("ring_lm2a", 60.5, 66.5, [1.0, 0.62, 0.122, 1.0]),
        ("ring_lt", 66.5, 71.3, [1.0, 0.63, 0.125, 1.0]),
        ("ring_rt", 78.5, 84.0, [1.0, 0.64, 0.126, 1.0]),
        ("ring_rm", 84.0, 89.5, [1.0, 0.628, 0.126, 1.0]),
        ("ring_rm2", 89.5, 95.0, [1.0, 0.675, 0.137, 1.0]),
        ("ring_rm3a", 95.0, 100.0, [1.0, 0.745, 0.153, 1.0]),
        ("ring_rm3b", 0.0, 2.5, [1.0, 0.804, 0.161, 1.0]),
        ("ring_rm4", 2.5, 6.5, [1.0, 0.698, 0.137, 1.0]),
        ("ring_rb2", 6.5, 10.0, [1.0, 0.49, 0.09, 1.0]),
        ("ring_rb3", 10.0, 12.5, [1.0, 0.306, 0.051, 1.0]),
        ("ring_rb", 12.5, 15.1, [1.0, 0.235, 0.04, 1.0]),
    ];
    for (id, s, e, sc) in segs.iter().copied() {
        ring_layer(
            &mut comp,
            id,
            sc,
            137.5,
            20.0,
            c1(s),
            c1(e),
            [487.7, 476.6],
            s100.clone(),
            o100.clone(),
            false,
            2,
        );
    }

    // flames (flat vector zones: red shell / upper-left shadow / orange band / cream core)
    let fo = flame_outer_pts();
    apart(
        &mut comp,
        "flame_outer",
        fo.clone(),
        smooth_closed(&fo, &[0, 27]),
        [1.0; 4],
        flame_outer_fill(),
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let su = shadow_ul_pts();
    apart(
        &mut comp,
        "shade_ul",
        su.clone(),
        smooth_closed(&su, &[0, 11]),
        [1.0; 4],
        shadow_ul_fill(),
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let fm = flame_mid_pts();
    apart(
        &mut comp,
        "flame_mid",
        fm.clone(),
        smooth_closed(&fm, &[0, 10]),
        [1.0; 4],
        flame_mid_fill(),
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );
    let fc = flame_core_pts();
    apart(
        &mut comp,
        "flame_core",
        fc.clone(),
        smooth_closed(&fc, &[0, 13]),
        [1.0, 0.973, 0.910, 1.0],
        flame_core_fill(),
        c2(ctr),
        s100.clone(),
        o100.clone(),
        2,
    );

    comp
}
