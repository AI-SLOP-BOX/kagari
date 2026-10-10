use super::common::*;
use kagari_vfx::core::timeline::*;

pub fn add_distortion_and_master(comp: &mut Composition) {
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
}
