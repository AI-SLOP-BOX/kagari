//! Glitch reconstruct title: KAGARI VFX (5s, 1920x1080, 30fps).
//!
//! Scene-built only: per-character text layers, procedural shapes, animated
//! masks, and generic engine effects (no pre-rendered material, no hacks).
//! All "random" placement uses build-time hashing baked into keyframes, so
//! every render of this file is byte-identical.

use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::mask::{Mask, MaskMode, MaskPath};
use kagari_vfx::core::project_migration::save_project_atomic;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

const W: f32 = 1920.0;
const H: f32 = 1080.0;
const CX: f32 = 960.0;
const CY: f32 = 540.0;
const FPS: u32 = 30;
const DUR: u32 = 150;

// ─── deterministic build-time hash (splitmix64) ───
fn sh(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
fn bf(a: u64, b: u64) -> f32 {
    (sh(a.wrapping_mul(0x9E37_79B9).wrapping_add(b)) >> 11) as f32 / (1u64 << 53) as f32
}

fn lin() -> InterpolationType {
    InterpolationType::Linear
}
fn hold() -> InterpolationType {
    InterpolationType::Hold
}
fn ez(c: [f32; 4]) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        incoming: BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        custom_bezier: Some(c),
    }
}
const E_SMOOTH: [f32; 4] = [0.0, 0.0, 0.4, 1.0];
const E_SNAP: [f32; 4] = [0.3, 0.0, 0.2, 1.0];

fn k1(f: u32, v: f32, e: InterpolationType) -> Keyframe<f32> {
    Keyframe::new(f, v, e)
}
fn k2(f: u32, v: [f32; 2], e: InterpolationType) -> Keyframe<[f32; 2]> {
    Keyframe::new(f, v, e)
}
fn c1(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}
fn c2(v: [f32; 2]) -> Animatable<[f32; 2]> {
    Animatable::new_constant(v)
}
fn a1(keys: Vec<Keyframe<f32>>) -> Animatable<f32> {
    Animatable::new_animated(keys)
}
fn a2(keys: Vec<Keyframe<[f32; 2]>>) -> Animatable<[f32; 2]> {
    Animatable::new_animated(keys)
}
fn fx(id: &str, name: &str, et: EffectType) -> Effect {
    Effect {
        id: id.into(),
        name: name.into(),
        effect_type: et,
        enabled: true,
    }
}

// ─── title layout ───
struct CharSpec {
    ch: char,
    home: [f32; 2],
    color: [f32; 4],
    size: u32,
    land: u32, // frame the char locks home
}

fn char_specs() -> Vec<CharSpec> {
    let mut out = Vec::new();
    let word1: Vec<char> = "KAGARI".chars().collect();
    for (i, ch) in word1.iter().enumerate() {
        out.push(CharSpec {
            ch: *ch,
            home: [CX + (i as f32 - 2.5) * 128.0 + 46.0, 482.0],
            color: [1.0, 1.0, 1.0, 1.0],
            size: 168,
            land: 48 + i as u32 * 4,
        });
    }
    let word2: Vec<char> = "VFX".chars().collect();
    for (j, ch) in word2.iter().enumerate() {
        out.push(CharSpec {
            ch: *ch,
            home: [CX + (j as f32 - 1.0) * 104.0 + 8.0, 667.0],
            color: [1.0, 0.72, 0.25, 1.0],
            size: 132,
            land: 64 + j as u32 * 4,
        });
    }
    out
}

fn build_glitch() -> Composition {
    let mut comp = Composition::new(
        "GlitchTitle".into(),
        "KAGARI VFX reconstruct".into(),
        W as u32,
        H as u32,
        FPS,
        DUR,
    );

    // ---- 1. background ----
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

    // ---- 5. per-character text (fragmentation via layers) ----
    for (ci, spec) in char_specs().iter().enumerate() {
        let id = format!("ch{ci}");
        let sx = if bf(ci as u64, 20) > 0.5 { 1.0 } else { -1.0 };
        let scatter = [
            spec.home[0] + sx * (200.0 + bf(ci as u64, 21) * 320.0),
            spec.home[1] + (bf(ci as u64, 22) * 2.0 - 1.0) * 260.0,
        ];
        comp.add_layer(Layer::new(
            id.clone(),
            format!("char {}", spec.ch),
            LayerType::Text {
                text: spec.ch.to_string(),
                font_size: spec.size,
                color: spec.color,
                font_family: "Helvetica".into(),
                tracking: 0.0,
                leading: 1.2,
                align: 1,
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                text_on_path: false,
            },
            DUR,
        ));
        {
            let l = comp.layers.last_mut().unwrap();
            // slice-reveal mask: a slit that TRACKS the char (world coords),
            // then opens fully once it locks home
            let mut pos_keys = vec![k2(0, scatter, hold())];
            let mut f = 3u32;
            while f < spec.land.saturating_sub(6) {
                let t = f as f32 / spec.land as f32;
                let amp = 1.0 - t;
                let jx = scatter[0]
                    + (spec.home[0] - scatter[0]) * t * 0.35
                    + (bf(ci as u64, 100 + f as u64) * 2.0 - 1.0) * 60.0 * amp;
                let jy = scatter[1]
                    + (spec.home[1] - scatter[1]) * t * 0.35
                    + (bf(ci as u64, 200 + f as u64) * 2.0 - 1.0) * 44.0 * amp;
                pos_keys.push(k2(f, [jx, jy], hold()));
                f += 3;
            }
            pos_keys.push(k2(spec.land, spec.home, ez(E_SMOOTH)));
            pos_keys.push(k2(150, spec.home, lin()));
            // slice-reveal mask: rect corners follow the same keys as the
            // char (slit tracks it), slightly smaller so edges stay clipped
            // until the lock-open envelope releases it
            let slit = |cx: f32, cy: f32| -> Vec<[f32; 2]> {
                vec![
                    [cx - 40.0, cy - 65.0],
                    [cx + 40.0, cy - 65.0],
                    [cx + 40.0, cy + 65.0],
                    [cx - 40.0, cy + 65.0],
                ]
            };
            let mask_path_keys: Vec<Keyframe<Vec<[f32; 2]>>> = pos_keys
                .iter()
                .map(|k| Keyframe::new(k.frame, slit(k.value[0], k.value[1]), k.interpolation))
                .collect();
            l.transform.position = a2(pos_keys);
            // fragment flicker -> solid
            let mut op_keys = vec![k1(0, 30.0, hold())];
            let mut f = 2u32;
            while f < 24 {
                op_keys.push(k1(f, 24.0 + bf(ci as u64, 300 + f as u64) * 14.0, hold()));
                f += 2;
            }
            let mut f = 24u32;
            while f < spec.land {
                let t = (f - 24) as f32 / (spec.land - 24).max(1) as f32;
                let v = 42.0 + t * 50.0 + (bf(ci as u64, 400 + f as u64) - 0.5) * 26.0;
                op_keys.push(k1(f, v.clamp(12.0, 96.0), hold()));
                f += 3;
            }
            op_keys.push(k1(spec.land, 88.0, ez(E_SMOOTH)));
            op_keys.push(k1(spec.land + 8, 100.0, lin()));
            op_keys.push(k1(150, 100.0, lin()));
            l.transform.opacity = a1(op_keys);
            let mask = Mask {
                id: format!("m{ci}"),
                name: format!("mask {ci}"),
                enabled: true,
                mode: MaskMode::Add,
                path: MaskPath {
                    vertices: Animatable::new_animated(mask_path_keys),
                    tangents: None,
                    is_closed: true,
                },
                feather: c1(2.0),
                opacity: c1(100.0),
                expansion: a1(vec![
                    k1(0, -10.0, lin()),
                    k1(spec.land + 2, -10.0, lin()),
                    k1(spec.land + 10, 25.0, ez(E_SMOOTH)),
                    k1(150, 25.0, lin()),
                ]),
                inverted: false,
                wiggle: None,
            };
            l.masks.push(mask);
            // per-char RGB split: separated -> merged (staggered)
            let split = 10.0 + bf(ci as u64, 23) * 10.0;
            l.effects.push(fx(
                &format!("ch{ci}_rgb"),
                "RGB Split",
                EffectType::RGBSplit {
                    red_offset: a2(vec![
                        k2(0, [sx * split, 0.0], hold()),
                        k2(spec.land.saturating_sub(8), [sx * split, 0.0], hold()),
                        k2(spec.land + 4, [0.0, 0.0], ez(E_SMOOTH)),
                        k2(150, [0.0, 0.0], lin()),
                    ]),
                    green_offset: c2([0.0, 0.0]),
                    blue_offset: a2(vec![
                        k2(0, [-sx * split, 0.0], hold()),
                        k2(spec.land.saturating_sub(8), [-sx * split, 0.0], hold()),
                        k2(spec.land + 4, [0.0, 0.0], ez(E_SMOOTH)),
                        k2(150, [0.0, 0.0], lin()),
                    ]),
                },
            ));
            // glow follows formation with a delay, then settles low
            let glow_col = if ci < 6 {
                [1.0, 0.93, 0.8, 1.0]
            } else {
                [1.0, 0.8, 0.5, 1.0]
            };
            l.effects.push(fx(
                &format!("ch{ci}_glow"),
                "Glow",
                EffectType::Glow {
                    threshold: c1(100.0),
                    radius: c1(22.0),
                    intensity: a1(vec![
                        k1(0, 0.0, lin()),
                        k1(spec.land.saturating_sub(4), 0.0, lin()),
                        k1(spec.land + 6, 75.0, ez(E_SNAP)),
                        k1(126, 18.0, lin()),
                        k1(150, 14.0, lin()),
                    ]),
                    color: Animatable::new_constant(glow_col),
                },
            ));
        }
    }

    // ---- 6. text-group distortion (adjustment: order matters) ----
    comp.add_layer(Layer::new(
        "text_fx".into(),
        "Text FX".into(),
        LayerType::AdjustmentLayer,
        DUR,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2([CX, CY]);
        l.effects.push(fx(
            "tf_block",
            "Block Glitch",
            EffectType::BlockGlitch {
                block_size: c1(26.0),
                amount: a1(vec![
                    k1(0, 0.0, lin()),
                    k1(20, 0.0, lin()),
                    k1(24, 0.5, lin()),
                    k1(30, 0.3, lin()),
                    k1(36, 0.62, lin()),
                    k1(42, 0.4, lin()),
                    k1(48, 0.55, lin()),
                    k1(54, 0.4, lin()),
                    k1(66, 0.3, lin()),
                    k1(78, 0.22, lin()),
                    k1(90, 0.15, lin()),
                    k1(99, 0.5, lin()),
                    k1(103, 0.12, lin()),
                    k1(115, 0.05, lin()),
                    k1(126, 0.0, lin()),
                    k1(150, 0.0, lin()),
                ]),
                seed: c1(7.0),
                corruption: c1(0.75),
            },
        ));
        l.effects.push(fx(
            "tf_slice",
            "Slice Tear",
            EffectType::SliceTear {
                slices: a1(vec![
                    k1(0, 0.0, lin()),
                    k1(20, 0.0, lin()),
                    k1(24, 7.0, lin()),
                    k1(36, 10.0, lin()),
                    k1(48, 6.0, lin()),
                    k1(60, 4.0, lin()),
                    k1(78, 3.0, lin()),
                    k1(90, 2.0, lin()),
                    k1(99, 12.0, lin()),
                    k1(103, 2.0, lin()),
                    k1(115, 1.0, lin()),
                    k1(126, 0.0, lin()),
                    k1(150, 0.0, lin()),
                ]),
                max_offset: a1(vec![
                    k1(0, 60.0, lin()),
                    k1(99, 95.0, lin()),
                    k1(126, 0.0, lin()),
                    k1(150, 0.0, lin()),
                ]),
                seed: c1(11.0),
                vertical: false,
            },
        ));
        l.effects.push(fx(
            "tf_turb",
            "Turbulent Displace",
            EffectType::TurbulentDisplace {
                amount: a1(vec![
                    k1(0, 0.0, lin()),
                    k1(20, 0.0, lin()),
                    k1(30, 14.0, lin()),
                    k1(48, 18.0, lin()),
                    k1(70, 8.0, lin()),
                    k1(90, 4.0, lin()),
                    k1(99, 22.0, lin()),
                    k1(104, 3.0, lin()),
                    k1(115, 0.0, lin()),
                    k1(150, 0.0, lin()),
                ]),
                size: c1(130.0),
                evolution: a1(vec![k1(0, 0.0, lin()), k1(150, 4.0, lin())]),
                complexity: c1(3.0),
            },
        ));
        l.effects.push(fx(
            "tf_blinds",
            "Venetian Blinds",
            EffectType::VenetianBlinds {
                completion: a1(vec![
                    k1(0, 0.0, lin()),
                    k1(54, 0.0, lin()),
                    k1(60, 45.0, lin()),
                    k1(72, 30.0, lin()),
                    k1(84, 15.0, lin()),
                    k1(96, 0.0, lin()),
                    k1(150, 0.0, lin()),
                ]),
                width: c1(18.0),
            },
        ));
    }

    // ---- 7. master signal chain (adjustment on top) ----
    comp.add_layer(Layer::new(
        "master".into(),
        "Master".into(),
        LayerType::AdjustmentLayer,
        DUR,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2([CX, CY]);
        l.effects.push(fx(
            "m_rgb",
            "RGB Split",
            EffectType::RGBSplit {
                red_offset: a2(vec![
                    k2(0, [0.0, 0.0], hold()),
                    k2(24, [8.0, 0.0], hold()),
                    k2(27, [0.0, 0.0], hold()),
                    k2(32, [12.0, 0.0], hold()),
                    k2(35, [0.0, 0.0], hold()),
                    k2(42, [9.0, 0.0], hold()),
                    k2(45, [0.0, 0.0], hold()),
                    k2(60, [4.0, 0.0], hold()),
                    k2(72, [3.0, 0.0], hold()),
                    k2(84, [2.0, 0.0], hold()),
                    k2(96, [0.0, 0.0], hold()),
                    k2(99, [26.0, 0.0], hold()),
                    k2(100, [18.0, 0.0], hold()),
                    k2(102, [0.0, 0.0], hold()),
                    k2(150, [0.0, 0.0], lin()),
                ]),
                green_offset: c2([0.0, 0.0]),
                blue_offset: a2(vec![
                    k2(0, [0.0, 0.0], hold()),
                    k2(24, [-8.0, 0.0], hold()),
                    k2(27, [0.0, 0.0], hold()),
                    k2(32, [-12.0, 0.0], hold()),
                    k2(35, [0.0, 0.0], hold()),
                    k2(42, [-9.0, 0.0], hold()),
                    k2(45, [0.0, 0.0], hold()),
                    k2(60, [-4.0, 0.0], hold()),
                    k2(72, [-3.0, 0.0], hold()),
                    k2(84, [-2.0, 0.0], hold()),
                    k2(96, [0.0, 0.0], hold()),
                    k2(99, [-26.0, 0.0], hold()),
                    k2(100, [-18.0, 0.0], hold()),
                    k2(102, [0.0, 0.0], hold()),
                    k2(150, [0.0, 0.0], lin()),
                ]),
            },
        ));
        l.effects.push(fx(
            "m_flick",
            "Flicker",
            EffectType::Flicker {
                amount: a1(vec![
                    k1(0, 0.22, lin()),
                    k1(24, 0.34, lin()),
                    k1(54, 0.16, lin()),
                    k1(96, 0.10, lin()),
                    k1(99, 0.55, lin()),
                    k1(103, 0.08, lin()),
                    k1(126, 0.04, lin()),
                    k1(150, 0.04, lin()),
                ]),
                speed: c1(14.0),
                seed: c1(5.0),
            },
        ));
        l.effects.push(fx(
            "m_crt",
            "CRT Scanlines",
            EffectType::CrtScanlines {
                line_spacing: c1(4.0),
                intensity: c1(0.14),
            },
        ));
        l.effects.push(fx(
            "m_scan",
            "Scanline Glitch",
            EffectType::ScanlineGlitch {
                jitter_amount: a1(vec![
                    k1(0, 2.0, lin()),
                    k1(24, 9.0, lin()),
                    k1(36, 13.0, lin()),
                    k1(48, 8.0, lin()),
                    k1(60, 5.0, lin()),
                    k1(90, 3.0, lin()),
                    k1(99, 20.0, lin()),
                    k1(103, 3.0, lin()),
                    k1(126, 1.5, lin()),
                    k1(150, 1.5, lin()),
                ]),
                seed: c1(3.0),
            },
        ));
        l.effects.push(fx(
            "m_grain",
            "Film Grain",
            EffectType::FilmGrain {
                intensity: c1(0.03),
                grain_size: 1.0,
                color_film: false,
            },
        ));
    }

    // ---- 8. sync flash (restrained) ----
    comp.add_layer(Layer::new(
        "flash".into(),
        "Flash".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        DUR,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2([CX, CY]);
        l.transform.opacity = a1(vec![
            k1(0, 0.0, lin()),
            k1(53, 0.0, hold()),
            k1(54, 6.0, hold()),
            k1(56, 0.0, hold()),
            k1(96, 0.0, hold()),
            k1(97, 8.0, hold()),
            k1(99, 60.0, hold()),
            k1(101, 0.0, hold()),
            k1(150, 0.0, lin()),
        ]);
    }

    comp
}

fn main() {
    let project = Project {
        compositions: vec![build_glitch()],
        ..Default::default()
    };
    save_project_atomic(&project, "glitch_project.json").unwrap();
    println!("Wrote glitch_project.json");
}
