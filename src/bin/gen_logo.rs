use kagari_vfx::core::timeline::*;
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::project_migration::save_project_atomic;

// NOTE: FreeformBezier tangents are ABSOLUTE handle positions.
// Straight edge => handle == anchor. Smooth => handles along (next-prev).
type Pt2 = [f32; 2];

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
        [-0.3, -71.7],
        [1.3, -63.3],
        [2.7, -54.9],
        [4.4, -50.8],
        [6.4, -46.8],
        [8.8, -42.8],
        [11.8, -38.7],
        [14.8, -34.7],
        [17.8, -30.6],
        [20.9, -26.6],
        [23.9, -22.6],
        [26.3, -18.5],
        [27.9, -14.5],
        [29.3, -10.4],
        [30.0, -6.4],
        [30.0, -2.4],
        [29.3, 1.7],
        [27.6, 5.7],
        [25.3, 9.8],
        [22.2, 13.5],
        [18.9, 17.2],
        [14.8, 20.5],
        [10.1, 23.9],
        [5.4, 27.3],
        [2.0, 31.3],
        [-0.3, 35.4],
        [-2.0, 38.7],
        [-1.3, 39.1],
        [-4.4, 35.4],
        [-6.7, 31.3],
        [-10.1, 27.3],
        [-15.2, 21.9],
        [-20.9, 17.8],
        [-25.3, 13.5],
        [-28.3, 9.8],
        [-30.0, 5.7],
        [-31.0, 1.7],
        [-30.3, -2.7],
        [-31.0, -6.4],
        [-31.0, -10.4],
        [-30.3, -14.5],
        [-29.0, -18.5],
        [-26.9, -22.6],
        [-24.6, -26.6],
        [-21.5, -30.6],
        [-18.2, -34.7],
        [-15.2, -38.7],
        [-12.1, -42.8],
        [-9.4, -46.8],
        [-7.1, -50.8],
        [-5.1, -54.9],
        [-2.7, -63.3],
    ]
}

fn flame_mid_pts() -> Vec<Pt2> {
    vec![
        [-0.3, -48.1],
        [4.4, -38.0],
        [8.8, -27.9],
        [12.8, -17.8],
        [16.2, -7.7],
        [17.5, 2.4],
        [16.2, 12.5],
        [12.8, 20.9],
        [7.7, 27.6],
        [2.7, 32.7],
        [0.0, 35.4],
        [-2.7, 32.7],
        [-7.7, 27.6],
        [-12.8, 20.9],
        [-16.2, 12.5],
        [-17.5, 2.4],
        [-16.2, -7.7],
        [-12.8, -17.8],
        [-8.8, -27.9],
        [-4.4, -38.0],
    ]
}

fn flame_core_pts() -> Vec<Pt2> {
    vec![
        [0.3, -27.9],
        [1.3, -23.9],
        [3.7, -19.9],
        [6.1, -15.8],
        [8.8, -11.8],
        [10.8, -7.7],
        [12.1, -3.7],
        [11.8, 0.3],
        [10.8, 4.4],
        [8.8, 8.4],
        [6.1, 12.5],
        [3.7, 16.5],
        [1.7, 20.5],
        [0.0, 23.2],
        [-0.3, 19.2],
        [-0.3, 12.5],
        [-0.3, 5.7],
        [-0.3, -1.0],
        [-0.3, -7.7],
        [-0.3, -14.5],
        [-0.3, -21.2],
    ]
}

fn half_light_l_pts() -> Vec<Pt2> {
    vec![
        [0.0, -78.0],
        [-4.0, -64.0],
        [-9.0, -51.0],
        [-18.0, -39.0],
        [-27.0, -26.0],
        [-31.0, -13.0],
        [-31.0, -3.0],
        [-24.0, 16.0],
        [-8.0, 38.0],
        [0.0, 44.0],
        [1.5, 20.0],
        [1.5, -10.0],
        [1.5, -40.0],
        [1.5, -60.0],
    ]
}

fn half_light_l_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -370.0],
        end: [0.0, 220.0],
        colors: vec![
            [1.0, 0.85, 0.55, 0.0],
            [1.0, 0.85, 0.55, 0.0],
            [1.0, 0.82, 0.50, 0.50],
            [1.0, 0.82, 0.50, 0.52],
        ],
        stops: vec![0.0, 0.45, 0.75, 1.0],
    }
}

fn half_shade_r_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -370.0],
        end: [0.0, 220.0],
        colors: vec![
            [0.50, 0.05, 0.02, 0.0],
            [0.50, 0.05, 0.02, 0.0],
            [0.45, 0.04, 0.02, 0.40],
            [0.45, 0.04, 0.02, 0.45],
        ],
        stops: vec![0.0, 0.45, 0.75, 1.0],
    }
}

fn mirror(pts: &[Pt2]) -> Vec<Pt2> {
    pts.iter().map(|p| [-p[0], p[1]]).collect()
}

fn shadow_ul_pts() -> Vec<Pt2> {
    vec![
        [-0.7, -71.0],
        [-2.4, -63.3],
        [-5.1, -54.9],
        [-7.1, -50.8],
        [-9.4, -46.8],
        [-12.1, -42.8],
        [-15.2, -38.7],
        [-18.2, -34.7],
        [-21.5, -30.6],
        [-24.6, -26.6],
        [-14.1, -24.6],
        [-5.4, -23.6],
        [-4.0, -26.3],
        [-2.7, -29.0],
        [-1.7, -31.6],
        [-1.0, -34.3],
        [-0.7, -44.8],
        [-0.7, -61.6],
    ]
}

fn shadow_ul_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -353.0],
        end: [0.0, -123.0],
        colors: vec![
            [0.80, 0.25, 0.06, 1.0],
            [0.95, 0.24, 0.04, 1.0],
            [1.0, 0.28, 0.06, 1.0],
        ],
        stops: vec![0.0, 0.5, 1.0],
    }
}

fn shield_l_pts() -> Vec<Pt2> {
    vec![[-13.5, 42.1], [-3.4, 42.1], [-3.4, 69.7], [-13.5, 69.7]]
}

fn cube_r_pts() -> Vec<Pt2> {
    vec![[-3.4, 42.1], [6.7, 42.1], [6.7, 69.7], [-3.4, 69.7]]
}

fn wing_dark_l_pts() -> Vec<Pt2> {
    vec![
        [-57.2, 10.8],
        [-52.5, 14.1],
        [-47.8, 17.8],
        [-43.1, 21.5],
        [-38.0, 25.6],
        [-32.7, 30.0],
        [-27.6, 34.0],
        [-22.9, 37.4],
        [-18.9, 40.1],
        [-16.2, 42.1],
        [-13.5, 44.1],
        [-10.8, 45.8],
        [-6.1, 44.8],
        [-1.3, 42.1],
        [-1.3, 44.1],
        [-7.5, 45.0],
        [-12.5, 45.5],
        [-16.2, 45.8],
        [-17.5, 44.8],
        [-20.9, 44.1],
        [-26.3, 41.8],
        [-31.6, 38.7],
        [-37.0, 35.0],
        [-42.4, 30.6],
        [-47.8, 25.6],
        [-53.2, 19.9],
        [-57.2, 14.5],
    ]
}

fn wing_blade_l_pts() -> Vec<Pt2> {
    vec![
        [-60.6, 8.1],
        [-54.5, 5.4],
        [-48.5, 2.4],
        [-43.4, -0.3],
        [-39.7, -1.0],
        [-38.7, 7.4],
        [-38.0, 15.8],
        [-35.0, 19.9],
        [-31.6, 23.9],
        [-27.6, 27.3],
        [-23.6, 30.6],
        [-19.5, 33.7],
        [-15.5, 36.0],
        [-12.1, 37.7],
        [-10.1, 39.1],
        [-6.1, 42.8],
        [-8.1, 44.1],
        [-10.8, 44.4],
        [-13.5, 42.8],
        [-16.2, 40.7],
        [-18.9, 38.7],
        [-22.9, 36.0],
        [-27.6, 32.7],
        [-32.7, 28.6],
        [-38.0, 24.2],
        [-43.1, 20.2],
        [-47.8, 16.5],
        [-52.5, 12.8],
        [-57.2, 9.4],
    ]
}

fn flame_outer_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -358.0],
        end: [0.0, 196.0],
        colors: vec![
            [1.0, 0.75, 0.24, 1.0],
            [1.0, 0.66, 0.21, 1.0],
            [1.0, 0.55, 0.16, 1.0],
            [1.0, 0.33, 0.08, 1.0],
            [1.0, 0.48, 0.15, 1.0],
            [1.0, 0.68, 0.23, 1.0],
            [1.0, 0.82, 0.29, 1.0],
            [1.0, 0.62, 0.18, 1.0],
            [0.93, 0.27, 0.08, 1.0],
        ],
        stops: vec![0.0, 0.20, 0.33, 0.42, 0.50, 0.60, 0.72, 0.85, 1.0],
    }
}

fn flame_mid_fill() -> ShapeFillType {
    ShapeFillType::LinearGradient {
        start: [0.0, -240.0],
        end: [0.0, 177.0],
        colors: vec![
            [1.0, 0.75, 0.43, 1.0],
            [1.0, 0.84, 0.59, 1.0],
            [1.0, 0.90, 0.73, 1.0],
        ],
        stops: vec![0.0, 0.55, 1.0],
    }
}

fn flame_core_fill() -> ShapeFillType {
    ShapeFillType::Solid
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
    apart(&mut comp, "cube_l", sl.clone(), sharp(&sl), [0.757, 0.722, 0.655, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let sr = cube_r_pts();
    apart(&mut comp, "cube_r", sr.clone(), sharp(&sr), [0.122, 0.141, 0.169, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);

    // wings (navy under, white blade over)
    let wd = wing_dark_l_pts();
    apart(&mut comp, "wing_dl", wd.clone(), sharp(&wd), [0.122, 0.141, 0.169, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wdr = mirror(&wd);
    apart(&mut comp, "wing_dr", wdr.clone(), sharp(&wdr), [0.122, 0.141, 0.169, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wb = wing_blade_l_pts();
    apart(&mut comp, "wing_l", wb.clone(), sharp(&wb), [1.0, 0.973, 0.910, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);
    let wbr = mirror(&wb);
    apart(&mut comp, "wing_r", wbr.clone(), sharp(&wbr), [1.0, 0.973, 0.910, 1.0], ShapeFillType::Solid, c2(ctr), s100.clone(), o100.clone(), 2);

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
        ring_layer(&mut comp, id, sc, 137.5, 20.0, c1(s), c1(e), [487.7, 476.6], s100.clone(), o100.clone(), false, 2);
    }

    // flames (flat vector zones: red shell / upper-left shadow / orange band / cream core)
    let fo = flame_outer_pts();
    apart(&mut comp, "flame_outer", fo.clone(), smooth_closed(&fo, &[0, 27]), [1.0; 4], flame_outer_fill(), c2(ctr), s100.clone(), o100.clone(), 2);
    let su = shadow_ul_pts();
    apart(&mut comp, "shade_ul", su.clone(), smooth_closed(&su, &[0, 11]), [1.0; 4], shadow_ul_fill(), c2(ctr), s100.clone(), o100.clone(), 2);
    let fm = flame_mid_pts();
    apart(&mut comp, "flame_mid", fm.clone(), smooth_closed(&fm, &[0, 10]), [1.0; 4], flame_mid_fill(), c2(ctr), s100.clone(), o100.clone(), 2);
    let fc = flame_core_pts();
    apart(&mut comp, "flame_core", fc.clone(), smooth_closed(&fc, &[0, 13]), [1.0, 0.973, 0.910, 1.0], flame_core_fill(), c2(ctr), s100.clone(), o100.clone(), 2);

    comp
}

// ─── Reveal composition (1920x1080): one energy phenomenon forms the mark ───
// trim% maps to ring angle: 0% = 3 o'clock, increasing clockwise (y-down).
// Ring center (960,430), radius ~202px in this comp.

use kagari_vfx::core::particle_system::{EmitterShape, FadeCurve, ParticleEmitter};

const R_CTR: [f32; 2] = [960.0, 430.0];
const R_RING: f32 = 303.0;
const RING_U: f32 = 140.0;

fn ez4(c: [f32; 4]) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: 0.33, speed: 0.0 },
        incoming: BezierControlPoint { influence: 0.33, speed: 0.0 },
        custom_bezier: Some(c),
    }
}

const E_FASTIN: [f32; 4] = [0.55, 0.0, 1.0, 1.0];
const E_SMOOTH: [f32; 4] = [0.0, 0.0, 0.4, 1.0];
const E_SNAP: [f32; 4] = [0.3, 0.0, 0.2, 1.0];
const E_HEAVY: [f32; 4] = [0.5, 0.0, 0.3, 1.0];
const E_DRIFT: [f32; 4] = [0.37, 0.0, 0.63, 1.0];

fn k1(f: u32, v: f32, e: InterpolationType) -> Keyframe<f32> {
    Keyframe::new(f, v, e)
}
fn k2(f: u32, v: [f32; 2], e: InterpolationType) -> Keyframe<[f32; 2]> {
    Keyframe::new(f, v, e)
}

/// Position on the ring for a trim percentage (energy comet follow-path).
fn comet_pos(trim_pct: f32) -> [f32; 2] {
    let th = trim_pct * 3.6 * std::f32::consts::PI / 180.0;
    [R_CTR[0] + R_RING * th.cos(), R_CTR[1] + R_RING * th.sin()]
}

/// Cheap glow: radial-gradient halo with its own opacity/scale envelopes.
#[allow(clippy::too_many_arguments)]
fn halo(
    comp: &mut Composition,
    id: &str,
    size: f32,
    radius: f32,
    core: [f32; 4],
    pos: Animatable<[f32; 2]>,
    scale: Animatable<[f32; 2]>,
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(size),
                height: Animatable::new_constant(size),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::RadialGradient {
                center: [0.0, 0.0],
                radius,
                colors: vec![core, [0.0, 0.0, 0.0, 0.0]],
                stops: vec![0.0, 1.0],
            },
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

/// Bright traveling window on the ring (energy pulse): thin white-gold arc.
fn pulse_layer(
    comp: &mut Composition,
    id: &str,
    start: Animatable<f32>,
    end: Animatable<f32>,
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(RING_U),
                height: Animatable::new_constant(RING_U),
            },
            color: [0.0; 4],
            stroke_color: [1.0, 0.92, 0.75, 1.0],
            stroke_width: 5.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = c2(R_CTR);
    l.transform.opacity = opacity;
    l.trim_paths = Some(TrimPaths {
        start,
        end,
        offset: Animatable::new_constant(0.0),
    });
}

fn particle_layer(
    comp: &mut Composition,
    id: &str,
    emitter: ParticleEmitter,
    pos: [f32; 2],
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Particle { emitter },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = c2(pos);
    l.transform.opacity = opacity;
}

fn base_emitter() -> ParticleEmitter {
    ParticleEmitter {
        fade_curve: FadeCurve::EaseOut,
        blend_mode: 1,
        ..Default::default()
    }
}

fn build_reveal() -> Composition {
    let mut comp = Composition::new("LogoReveal".into(), "Kagari Logo Reveal".into(), 1920, 1080, 30, 150);
    let ctr = R_CTR;
    let base = 45.0;
    let lin = InterpolationType::Linear;

    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid {
            color: [0.015, 0.01, 0.01, 1.0],
        },
        150,
    ));
    comp.layers.last_mut().unwrap().transform.position = c2([960.0, 540.0]);

    // atmosphere: breathes with the ignition instead of a plain fade
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
            k1(0, 0.0, lin),
            k1(8, 0.0, lin),
            k1(30, 85.0, ez4(E_DRIFT)),
            k1(44, 85.0, lin),
            k1(50, 100.0, ez4(E_SNAP)),
            k1(135, 100.0, lin),
            k1(150, 0.0, lin),
        ]);
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [100.0, 100.0], lin),
            k2(44, [100.0, 100.0], lin),
            k2(56, [109.0, 109.0], ez4(E_SMOOTH)),
            k2(80, [100.0, 100.0], ez4(E_DRIFT)),
            k2(150, [100.0, 100.0], lin),
        ]);
    }

    // luminous bed: tight warm light under the logo once ignited (two-component
    // falloff with the wide atmosphere above). This carries the mid-tones.
    halo(&mut comp, "bed", 120.0, 300.0, [0.65, 0.28, 0.08, 0.6], c2([960.0, 430.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(46, 0.0, lin),
            k1(60, 85.0, ez4(E_SMOOTH)),
            k1(135, 85.0, lin),
            k1(150, 0.0, lin),
        ]), 150);

    // flame skirt: tight hot light clinging to the flame body (permanent).
    halo(&mut comp, "skirt", 120.0, 340.0, [0.7, 0.3, 0.08, 0.55], c2([960.0, 415.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(48, 0.0, lin),
            k1(62, 55.0, ez4(E_SMOOTH)),
            k1(135, 55.0, lin),
            k1(150, 0.0, lin),
        ]), 150);

    // energy seed: a faint ember wakes at the ring bottom (f6)
    halo(&mut comp, "seed", 30.0, 150.0, [1.0, 0.62, 0.2, 0.9], c2([960.0, 733.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(5, 0.0, lin),
            k1(10, 70.0, ez4(E_SMOOTH)),
            k1(16, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]), 150);

    // ignition flash: caused by the converging energy arriving at center
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
            k2(0, [0.0, 0.0], lin),
            k2(43, [0.0, 0.0], lin),
            k2(46, [30.0, 30.0], ez4(E_SNAP)),
            k2(54, [36.0, 36.0], ez4(E_SMOOTH)),
            k2(66, [0.0, 0.0], ez4(E_DRIFT)),
            k2(150, [0.0, 0.0], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(43, 0.0, lin),
            k1(45, 95.0, ez4(E_SNAP)),
            k1(52, 45.0, ez4(E_SMOOTH)),
            k1(66, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]);
    }

    // shockwave #1: ignition blast (fast, hot)
    comp.add_layer(Layer::new(
        "shock1".into(),
        "Shockwave1".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(700.0),
                height: Animatable::new_constant(700.0),
            },
            color: [1.0, 0.6, 0.15, 0.0],
            stroke_color: [1.0, 0.75, 0.35, 0.95],
            stroke_width: 7.0,
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
            k2(0, [0.0, 0.0], lin),
            k2(44, [0.0, 0.0], lin),
            k2(62, [9.0, 9.0], ez4(E_SMOOTH)),
            k2(74, [10.0, 10.0], ez4(E_DRIFT)),
            k2(150, [10.0, 10.0], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(44, 0.0, lin),
            k1(46, 100.0, ez4(E_SNAP)),
            k1(66, 45.0, ez4(E_SMOOTH)),
            k1(74, 0.0, lin),
            k1(150, 0.0, lin),
        ]);
    }

    // shockwave #2: wing-lock return pulse (slower, thinner, cooler)
    comp.add_layer(Layer::new(
        "shock2".into(),
        "Shockwave2".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(700.0),
                height: Animatable::new_constant(700.0),
            },
            color: [1.0, 0.6, 0.15, 0.0],
            stroke_color: [1.0, 0.55, 0.2, 0.85],
            stroke_width: 4.0,
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
            k2(0, [0.18, 0.18], lin),
            k2(69, [0.18, 0.18], lin),
            k2(92, [8.5, 8.5], ez4(E_SMOOTH)),
            k2(150, [8.5, 8.5], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(69, 0.0, lin),
            k1(72, 55.0, ez4(E_SNAP)),
            k1(88, 18.0, ez4(E_SMOOTH)),
            k1(94, 0.0, lin),
            k1(150, 0.0, lin),
        ]);
    }

    // white-hot peak: the whole logo overexposes briefly, then decays
    halo(&mut comp, "whitepeak", 240.0, 600.0, [1.0, 0.98, 0.94, 1.0], c2([960.0, 440.0]),
        Animatable::new_animated(vec![
            k2(0, [40.0, 40.0], lin),
            k2(80, [40.0, 40.0], lin),
            k2(85, [58.0, 58.0], ez4(E_SNAP)),
            k2(95, [55.0, 55.0], ez4(E_DRIFT)),
            k2(150, [55.0, 55.0], lin),
        ]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(80, 0.0, lin),
            k1(83, 30.0, ez4(E_SNAP)),
            k1(89, 14.0, ez4(E_SMOOTH)),
            k1(94, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]), 150);

    // sideways release streaks: flame expansion fires energy to both sides.
    // The wings deploy BECAUSE these streaks arrive (cause -> effect).
    for (side, x0) in [("l", 960.0), ("r", 960.0)] {
        let dir = if side == "l" { -1.0 } else { 1.0 };
        let id = format!("streak_{side}");
        let pts = vec![[0.0, -3.0], [dir * 46.0, -3.0], [dir * 46.0, 3.0], [0.0, 3.0]];
        let d = if side == "l" { 0 } else { 2 };
        apart(&mut comp, &id, pts.clone(), sharp(&pts), [1.0, 0.8, 0.45, 1.0],
            ShapeFillType::Solid, c2([x0, 444.0]),
            Animatable::new_animated(vec![
                k2(0, [0.0, base], lin),
                k2(52 + d, [0.0, base], lin),
                k2(59 + d, [base * 1.1, base], ez4(E_FASTIN)),
                k2(150, [base * 1.1, base], lin),
            ]),
            Animatable::new_animated(vec![
                k1(0, 0.0, lin),
                k1(52 + d, 0.0, lin),
                k1(55 + d, 90.0, ez4(E_SNAP)),
                k1(62 + d, 0.0, ez4(E_SMOOTH)),
                k1(150, 0.0, lin),
            ]), 150);
    }

    // wings materialize at the streak impacts: short travel, then opacity pops
    // at arrival (no long dark slides). Dark mass first, bright blade unfolds
    // 3f later with a wider swing (follow-through). Left leads by 2f.
    let deploy = |from_x: f32, t0: u32, rot0: f32| {
        (Animatable::new_animated(vec![
            k2(0, [ctr[0] + from_x, ctr[1]], lin),
            k2(t0, [ctr[0] + from_x, ctr[1]], lin),
            k2(t0 + 12, [ctr[0] + from_x * 0.12, ctr[1]], ez4(E_SNAP)),
            k2(t0 + 16, ctr, ez4(E_SMOOTH)),
            k2(150, ctr, lin),
        ]),
        Animatable::new_animated(vec![
            k1(0, rot0, lin),
            k1(t0, rot0, lin),
            k1(t0 + 12, -rot0 * 0.22, ez4(E_SNAP)),
            k1(t0 + 18, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]))
    };
    let wing_op = |t0: u32| {
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(t0 + 8, 0.0, lin),
            k1(t0 + 12, 100.0, ez4(E_SNAP)),
            k1(135, 100.0, lin),
            k1(150, 0.0, lin),
        ])
    };
    let unfold = |t0: u32| {
        Animatable::new_animated(vec![
            k2(0, [base * 0.35, base], lin),
            k2(t0 + 8, [base * 0.35, base], lin),
            k2(t0 + 13, [base * 1.12, base], ez4(E_SNAP)),
            k2(t0 + 18, [base, base], ez4(E_SMOOTH)),
            k2(150, [base, base], lin),
        ])
    };
    let wd = wing_dark_l_pts();
    let (wp, wr) = deploy(-225.0, 56, -16.0);
    apart(&mut comp, "wing_dl", wd.clone(), sharp(&wd), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid,
        wp, c2([base, base]), wing_op(56), 150);
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wdr = mirror(&wd);
    let (wp, wr) = deploy(225.0, 58, 16.0);
    apart(&mut comp, "wing_dr", wdr.clone(), sharp(&wdr), [0.08, 0.10, 0.16, 1.0], ShapeFillType::Solid,
        wp, c2([base, base]), wing_op(58), 150);
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wb = wing_blade_l_pts();
    let (wp, wr) = deploy(-225.0, 59, -23.0);
    apart(&mut comp, "wing_l", wb.clone(), sharp(&wb), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid,
        wp, unfold(59), wing_op(59), 150);
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wbr = mirror(&wb);
    let (wp, wr) = deploy(225.0, 61, 23.0);
    apart(&mut comp, "wing_r", wbr.clone(), sharp(&wbr), [0.96, 0.95, 0.90, 1.0], ShapeFillType::Solid,
        wp, unfold(61), wing_op(61), 150);
    comp.layers.last_mut().unwrap().transform.rotation = wr;

    // wing-root impact glows: reaction to each lock (left f72, right f74)
    halo(&mut comp, "root_l", 40.0, 200.0, [1.0, 0.8, 0.5, 1.0], c2([792.0, 445.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(68, 0.0, lin),
            k1(72, 100.0, ez4(E_SNAP)),
            k1(80, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]), 150);
    halo(&mut comp, "root_r", 40.0, 200.0, [1.0, 0.8, 0.5, 1.0], c2([1128.0, 445.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(70, 0.0, lin),
            k1(74, 100.0, ez4(E_SNAP)),
            k1(82, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]), 150);

    // shield rises heavy: slow-in, small overshoot bounce, settles
    let rise = Animatable::new_animated(vec![
        k2(0, [ctr[0], ctr[1] + 330.0], lin),
        k2(60, [ctr[0], ctr[1] + 330.0], lin),
        k2(78, [ctr[0], ctr[1] - 7.0], ez4(E_HEAVY)),
        k2(84, ctr, ez4(E_SMOOTH)),
        k2(150, ctr, lin),
    ]);
    let shield_op = Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(60, 0.0, lin),
        k1(70, 100.0, ez4(E_SMOOTH)),
        k1(135, 100.0, lin),
        k1(150, 0.0, lin),
    ]);
    let sl = shield_l_pts();
    apart(&mut comp, "cube_l", sl.clone(), sharp(&sl), [0.757, 0.722, 0.655, 1.0], ShapeFillType::Solid,
        rise.clone(), c2([base, base]), shield_op.clone(), 150);
    let sr = cube_r_pts();
    apart(&mut comp, "cube_r", sr.clone(), sharp(&sr), [0.122, 0.141, 0.169, 1.0], ShapeFillType::Solid,
        rise, c2([base, base]), shield_op, 150);

    // ring arcs IGNITE behind the traveling comets (fronts track comet angle).
    // Left comet f10->36 (trim 25->75); right comet f12->38 (trim 25->0/100->77).
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
    // (id, fixed_start, front_end_anim, fixed_end, front_start_anim, color)
    let ring_draws: &[(RingDraw, bool)] = &[
        (("ring_lb2", 34.9, 36.5, [1.0, 0.235, 0.04, 1.0], 10, 15), true),
        (("ring_lb", 36.5, 39.0, [1.0, 0.267, 0.043, 1.0], 13, 19), true),
        (("ring_lb2x", 39.0, 41.5, [1.0, 0.373, 0.059, 1.0], 16, 21), true),
        (("ring_lb3", 41.5, 45.5, [1.0, 0.57, 0.106, 1.0], 19, 24), true),
        (("ring_lm3", 45.5, 50.0, [1.0, 0.765, 0.157, 1.0], 22, 27), true),
        (("ring_lm", 50.0, 54.5, [1.0, 0.745, 0.153, 1.0], 25, 30), true),
        (("ring_lm2b", 54.5, 60.5, [1.0, 0.66, 0.133, 1.0], 28, 32), true),
        (("ring_lm2a", 60.5, 66.5, [1.0, 0.62, 0.122, 1.0], 31, 35), true),
        (("ring_lt", 66.5, 71.3, [1.0, 0.63, 0.125, 1.0], 34, 38), true),
        (("ring_rt", 78.5, 84.0, [1.0, 0.64, 0.126, 1.0], 34, 38), false),
        (("ring_rm", 84.0, 89.5, [1.0, 0.628, 0.126, 1.0], 32, 36), false),
        (("ring_rm2", 89.5, 95.0, [1.0, 0.675, 0.137, 1.0], 30, 34), false),
        (("ring_rm3a", 95.0, 100.0, [1.0, 0.745, 0.153, 1.0], 28, 33), false),
        (("ring_rm3b", 0.0, 2.5, [1.0, 0.804, 0.161, 1.0], 25, 30), false),
        (("ring_rm4", 2.5, 6.5, [1.0, 0.698, 0.137, 1.0], 22, 27), false),
        (("ring_rb2", 6.5, 10.0, [1.0, 0.49, 0.09, 1.0], 19, 24), false),
        (("ring_rb3", 10.0, 12.5, [1.0, 0.306, 0.051, 1.0], 16, 21), false),
        (("ring_rb", 12.5, 15.1, [1.0, 0.235, 0.04, 1.0], 13, 19), false),
    ];
    for ((id, s, e, sc, d0, d1), grow_end) in ring_draws.iter().copied() {
        let (st, en) = if grow_end {
            (c1(s), front(s, e, d0, d1))
        } else {
            (front(e, s, d0, d1), c1(e))
        };
        ring_layer(&mut comp, id, sc, RING_U, 11.0, st, en, ctr,
            c2([base, base]), ring_op.clone(), false, 150);
    }

    // ring charge: a thicker, hotter arc set fades in over the base ring as
    // the climax charge (thin -> luminous band), then settles to a glow.
    // Same segments, static trims — the base draw-on already ran.
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
            cid.clone(), cid,
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
    // Left side (wrap-free): window [t, t+10], t 17->65, f10->35.
    pulse_layer(&mut comp, "pulse_l", front(17.0, 65.0, 10, 35), front(27.0, 75.0, 10, 35),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin), k1(10, 0.0, lin), k1(13, 100.0, ez4(E_SMOOTH)),
            k1(35, 100.0, lin), k1(39, 0.0, ez4(E_FASTIN)), k1(150, 0.0, lin),
        ]), 150);
    // Right lower (25->0, wrap-free): window [t-12, t], t 25->12, f12->24.
    pulse_layer(&mut comp, "pulse_r1", front(13.0, 0.0, 12, 24), front(25.0, 12.0, 12, 24),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin), k1(12, 0.0, lin), k1(15, 100.0, ez4(E_SMOOTH)),
            k1(22, 100.0, lin), k1(26, 0.0, ez4(E_FASTIN)), k1(150, 0.0, lin),
        ]), 150);
    // Right upper (100->77, wrap-free): window [t-12, t], t 100->89, f24->37.
    pulse_layer(&mut comp, "pulse_r2", front(88.0, 77.0, 24, 37), front(100.0, 89.0, 24, 37),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin), k1(23, 0.0, lin), k1(26, 100.0, ez4(E_SMOOTH)),
            k1(37, 100.0, lin), k1(41, 0.0, ez4(E_FASTIN)), k1(150, 0.0, lin),
        ]), 150);
    // Return sweep: wing-lock energy travels back through the full ring (f70-86).
    pulse_layer(&mut comp, "pulse_back", front(20.0, 75.0, 70, 86), front(32.0, 87.0, 70, 86),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin), k1(70, 0.0, lin), k1(73, 85.0, ez4(E_SMOOTH)),
            k1(84, 85.0, lin), k1(88, 0.0, ez4(E_DRIFT)), k1(150, 0.0, lin),
        ]), 150);

    // comet heads: white-hot cores riding the fronts, then diving to center.
    // Left comet: arc f10->36, dive f36->44.
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
    // Right comet: arc f12->38 (through 0/100), dive f38->46. Leads no one: 2f late.
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
        // dive stretch: elongate along the fall (vertical), relax on arrival
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [100.0, 100.0], lin),
            k2(35 + d, [100.0, 100.0], lin),
            k2(40 + d, [70.0, 190.0], ez4(E_FASTIN)),
            k2(45 + d, [110.0, 90.0], ez4(E_SNAP)),
            k2(50 + d, [100.0, 100.0], ez4(E_SMOOTH)),
            k2(150, [100.0, 100.0], lin),
        ]);
        halo(&mut comp, &format!("comet_halo_{side}"), 46.0, 230.0,
            [1.0, 0.72, 0.3, 0.85], Animatable::new_animated(keys),
            c2([100.0, 100.0]),
            Animatable::new_animated(vec![
                k1(0, 0.0, lin),
                k1(9 + d, 0.0, lin),
                k1(13 + d, 85.0, ez4(E_SMOOTH)),
                k1(42 + d, 85.0, lin),
                k1(49 + d, 0.0, ez4(E_FASTIN)),
                k1(150, 0.0, lin),
            ]), 150);
    }

    // convergence streams: ring-shaped emitter pulled to center (f34-50).
    // Visible ONLY as inward-flying energy because lifetime is short.
    particle_layer(&mut comp, "converge", ParticleEmitter {
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
    }, ctr,
    Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(33, 0.0, lin),
        k1(38, 90.0, ez4(E_SMOOTH)),
        k1(44, 70.0, lin),
        k1(50, 0.0, ez4(E_FASTIN)),
        k1(150, 0.0, lin),
    ]), 150);

    // flames: anticipation squash (f40-44) -> explosive overshoot -> recoil ->
    // settle -> damped breathing. Mid/core stagger inside-out; shade lags 2f.
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
    apart(&mut comp, "flame_outer", fo.clone(), smooth_closed(&fo, &[0, 27]), [1.0; 4], flame_outer_fill(),
        c2(ctr), burst(0, base * 1.2), flame_op.clone(), 150);
    let su = shadow_ul_pts();
    apart(&mut comp, "shade_ul", su.clone(), smooth_closed(&su, &[0, 11]), [1.0; 4], shadow_ul_fill(),
        c2(ctr), burst(1, base * 1.2), flame_op.clone(), 150);
    let fm = flame_mid_pts();
    apart(&mut comp, "flame_mid", fm.clone(), smooth_closed(&fm, &[0, 10]), [1.0; 4], flame_mid_fill(),
        c2(ctr), burst(3, base * 1.17), flame_op.clone(), 150);
    let sh = half_light_l_pts();
    apart(&mut comp, "light_l", sh.clone(), smooth_closed(&sh, &[0, 9]), [1.0; 4], half_light_l_fill(),
        c2(ctr), burst(2, base * 1.18), flame_op.clone(), 150);
    let lr = mirror(&sh);
    apart(&mut comp, "shade_r", lr.clone(), smooth_closed(&lr, &[0, 9]), [1.0; 4], half_shade_r_fill(),
        c2(ctr), burst(2, base * 1.18), flame_op.clone(), 150);
    // core flares again at the white-hot peak (reaction to the return sweep)
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
    apart(&mut comp, "flame_core", fc.clone(), smooth_closed(&fc, &[0, 13]), [1.0, 0.973, 0.910, 1.0], flame_core_fill(),
        c2(ctr), core_scale, flame_op.clone(), 150);

    // ignition sparks: burst REACTING to the ignition (short life + opacity gate)
    particle_layer(&mut comp, "sparks", ParticleEmitter {
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
    }, ctr,
    Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(43, 0.0, lin),
        k1(45, 100.0, ez4(E_SNAP)),
        k1(50, 60.0, lin),
        k1(56, 0.0, ez4(E_SMOOTH)),
        k1(150, 0.0, lin),
    ]), 150);

    // drifting embers: slow residue of the ignition, floating upward
    particle_layer(&mut comp, "embers", ParticleEmitter {
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
    }, [960.0, 540.0],
    Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(46, 0.0, lin),
        k1(58, 65.0, ez4(E_SMOOTH)),
        k1(108, 65.0, lin),
        k1(122, 0.0, ez4(E_DRIFT)),
        k1(150, 0.0, lin),
    ]), 150);

    // ---- titles: rise with overshoot after the logo settles, staggered ----
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
    comp.add_layer(Layer::new("title".into(), "Title".into(),
        LayerType::Text {
            text: "KAGARI".into(), font_size: 80,
            color: [1.0; 4], font_family: "SF Pro Display".into(),
            tracking: 20.0, leading: 1.2, align: 1,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 150));
    {
        let t = comp.layers.last_mut().unwrap();
        t.transform.position = title_rise(94, 835.0);
        t.transform.opacity = title_op(94);
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
        v.transform.position = title_rise(98, 915.0);
        v.transform.opacity = title_op(98);
    }
    comp.add_layer(Layer::new("line".into(), "Line".into(),
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
            stroke_color: [0.0; 4], stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0, bevel_depth: 0.0,
        }, 150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = Animatable::new_constant([960.0, 965.0]);
        l.transform.opacity = title_op(106);
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
        l.transform.position = Animatable::new_constant([960.0, 1010.0]);
        l.transform.opacity = title_op(114);
    }

    // photographic finish: subtle vignette + fine grain over everything.
    // Grain ramps in with the light (deterministic, frame-seeded).
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
