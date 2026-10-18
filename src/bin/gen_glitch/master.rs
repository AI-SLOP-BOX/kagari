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
        l.transform.position = const_vec2([CX, CY]);
        l.effects.push(make_effect(
            "tf_block",
            "Block Glitch",
            EffectType::BlockGlitch {
                block_size: const_scalar(26.0),
                amount: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(20, 0.0, lin()),
                    key_scalar(24, 0.5, lin()),
                    key_scalar(30, 0.3, lin()),
                    key_scalar(36, 0.62, lin()),
                    key_scalar(42, 0.4, lin()),
                    key_scalar(48, 0.55, lin()),
                    key_scalar(54, 0.4, lin()),
                    key_scalar(66, 0.3, lin()),
                    key_scalar(78, 0.22, lin()),
                    key_scalar(90, 0.15, lin()),
                    key_scalar(99, 0.5, lin()),
                    key_scalar(103, 0.12, lin()),
                    key_scalar(115, 0.05, lin()),
                    key_scalar(126, 0.0, lin()),
                    key_scalar(150, 0.0, lin()),
                ]),
                seed: const_scalar(7.0),
                corruption: const_scalar(0.75),
            },
        ));
        l.effects.push(make_effect(
            "tf_slice",
            "Slice Tear",
            EffectType::SliceTear {
                slices: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(20, 0.0, lin()),
                    key_scalar(24, 7.0, lin()),
                    key_scalar(36, 10.0, lin()),
                    key_scalar(48, 6.0, lin()),
                    key_scalar(60, 4.0, lin()),
                    key_scalar(78, 3.0, lin()),
                    key_scalar(90, 2.0, lin()),
                    key_scalar(99, 12.0, lin()),
                    key_scalar(103, 2.0, lin()),
                    key_scalar(115, 1.0, lin()),
                    key_scalar(126, 0.0, lin()),
                    key_scalar(150, 0.0, lin()),
                ]),
                max_offset: anim_scalar(vec![
                    key_scalar(0, 60.0, lin()),
                    key_scalar(99, 95.0, lin()),
                    key_scalar(126, 0.0, lin()),
                    key_scalar(150, 0.0, lin()),
                ]),
                seed: const_scalar(11.0),
                vertical: false,
            },
        ));
        l.effects.push(make_effect(
            "tf_turb",
            "Turbulent Displace",
            EffectType::TurbulentDisplace {
                amount: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(20, 0.0, lin()),
                    key_scalar(30, 14.0, lin()),
                    key_scalar(48, 18.0, lin()),
                    key_scalar(70, 8.0, lin()),
                    key_scalar(90, 4.0, lin()),
                    key_scalar(99, 22.0, lin()),
                    key_scalar(104, 3.0, lin()),
                    key_scalar(115, 0.0, lin()),
                    key_scalar(150, 0.0, lin()),
                ]),
                size: const_scalar(130.0),
                evolution: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(150, 4.0, lin()),
                ]),
                complexity: const_scalar(3.0),
            },
        ));
        l.effects.push(make_effect(
            "tf_blinds",
            "Venetian Blinds",
            EffectType::VenetianBlinds {
                completion: anim_scalar(vec![
                    key_scalar(0, 0.0, lin()),
                    key_scalar(54, 0.0, lin()),
                    key_scalar(60, 45.0, lin()),
                    key_scalar(72, 30.0, lin()),
                    key_scalar(84, 15.0, lin()),
                    key_scalar(96, 0.0, lin()),
                    key_scalar(150, 0.0, lin()),
                ]),
                width: const_scalar(18.0),
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
        l.transform.position = const_vec2([CX, CY]);
        l.effects.push(make_effect(
            "m_rgb",
            "RGB Split",
            EffectType::RGBSplit {
                red_offset: anim_vec2(vec![
                    key_vec2(0, [0.0, 0.0], hold()),
                    key_vec2(24, [8.0, 0.0], hold()),
                    key_vec2(27, [0.0, 0.0], hold()),
                    key_vec2(32, [12.0, 0.0], hold()),
                    key_vec2(35, [0.0, 0.0], hold()),
                    key_vec2(42, [9.0, 0.0], hold()),
                    key_vec2(45, [0.0, 0.0], hold()),
                    key_vec2(60, [4.0, 0.0], hold()),
                    key_vec2(72, [3.0, 0.0], hold()),
                    key_vec2(84, [2.0, 0.0], hold()),
                    key_vec2(96, [0.0, 0.0], hold()),
                    key_vec2(99, [26.0, 0.0], hold()),
                    key_vec2(100, [18.0, 0.0], hold()),
                    key_vec2(102, [0.0, 0.0], hold()),
                    key_vec2(150, [0.0, 0.0], lin()),
                ]),
                green_offset: const_vec2([0.0, 0.0]),
                blue_offset: anim_vec2(vec![
                    key_vec2(0, [0.0, 0.0], hold()),
                    key_vec2(24, [-8.0, 0.0], hold()),
                    key_vec2(27, [0.0, 0.0], hold()),
                    key_vec2(32, [-12.0, 0.0], hold()),
                    key_vec2(35, [0.0, 0.0], hold()),
                    key_vec2(42, [-9.0, 0.0], hold()),
                    key_vec2(45, [0.0, 0.0], hold()),
                    key_vec2(60, [-4.0, 0.0], hold()),
                    key_vec2(72, [-3.0, 0.0], hold()),
                    key_vec2(84, [-2.0, 0.0], hold()),
                    key_vec2(96, [0.0, 0.0], hold()),
                    key_vec2(99, [-26.0, 0.0], hold()),
                    key_vec2(100, [-18.0, 0.0], hold()),
                    key_vec2(102, [0.0, 0.0], hold()),
                    key_vec2(150, [0.0, 0.0], lin()),
                ]),
            },
        ));
        l.effects.push(make_effect(
            "m_flick",
            "Flicker",
            EffectType::Flicker {
                amount: anim_scalar(vec![
                    key_scalar(0, 0.22, lin()),
                    key_scalar(24, 0.34, lin()),
                    key_scalar(54, 0.16, lin()),
                    key_scalar(96, 0.10, lin()),
                    key_scalar(99, 0.55, lin()),
                    key_scalar(103, 0.08, lin()),
                    key_scalar(126, 0.04, lin()),
                    key_scalar(150, 0.04, lin()),
                ]),
                speed: const_scalar(14.0),
                seed: const_scalar(5.0),
            },
        ));
        l.effects.push(make_effect(
            "m_crt",
            "CRT Scanlines",
            EffectType::CrtScanlines {
                line_spacing: const_scalar(4.0),
                intensity: const_scalar(0.14),
            },
        ));
        l.effects.push(make_effect(
            "m_scan",
            "Scanline Glitch",
            EffectType::ScanlineGlitch {
                jitter_amount: anim_scalar(vec![
                    key_scalar(0, 2.0, lin()),
                    key_scalar(24, 9.0, lin()),
                    key_scalar(36, 13.0, lin()),
                    key_scalar(48, 8.0, lin()),
                    key_scalar(60, 5.0, lin()),
                    key_scalar(90, 3.0, lin()),
                    key_scalar(99, 20.0, lin()),
                    key_scalar(103, 3.0, lin()),
                    key_scalar(126, 1.5, lin()),
                    key_scalar(150, 1.5, lin()),
                ]),
                seed: const_scalar(3.0),
            },
        ));
        l.effects.push(make_effect(
            "m_grain",
            "Film Grain",
            EffectType::FilmGrain {
                intensity: const_scalar(0.03),
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
        l.transform.position = const_vec2([CX, CY]);
        l.transform.opacity = anim_scalar(vec![
            key_scalar(0, 0.0, lin()),
            key_scalar(53, 0.0, hold()),
            key_scalar(54, 6.0, hold()),
            key_scalar(56, 0.0, hold()),
            key_scalar(96, 0.0, hold()),
            key_scalar(97, 8.0, hold()),
            key_scalar(99, 60.0, hold()),
            key_scalar(101, 0.0, hold()),
            key_scalar(150, 0.0, lin()),
        ]);
    }
}
