use super::types::*;

fn gradient(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.push(((x * 255) / w.max(1)) as u8);
            v.push(((y * 255) / h.max(1)) as u8);
            v.push(128);
            v.push(255);
        }
    }
    v
}

fn sample_effects() -> Vec<ExtEffect> {
    vec![
        ExtEffect::WaveWarp {
            wave_height: 5.0,
            wave_width: 10.0,
            speed: 1.0,
            direction_deg: 90.0,
            wave_type: 0,
            pinning: 0,
        },
        ExtEffect::CcLens {
            convergence: 40.0,
            zoom: 1.0,
        },
        ExtEffect::PolarCoordinates {
            to_polar: true,
            interpolation: 100.0,
        },
        ExtEffect::OpticsCompensation {
            field_of_view_deg: 60.0,
            reverse: false,
            zoom: 1.0,
        },
        ExtEffect::Curves {
            master: vec![[0.0, 0.0], [0.5, 0.45], [1.0, 1.0]],
            red: vec![],
            green: vec![],
            blue: vec![],
        },
        ExtEffect::ColorBalance {
            shadows: [-20.0, 0.0, 20.0],
            midtones: [0.0; 3],
            highlights: [10.0, 0.0, -10.0],
            preserve_luminosity: true,
        },
        ExtEffect::ChannelMixer {
            matrix: identity_matrix(),
            monochrome: false,
        },
        ExtEffect::LightSweep {
            direction_deg: 0.0,
            center: 0.5,
            width: 0.25,
            sweep_intensity: 0.6,
            edge_intensity: 0.3,
        },
        ExtEffect::RadialFastBlur {
            amount: 0.3,
            samples: 8,
        },
        ExtEffect::BendIt {
            top_offset: 4.0,
            bottom_offset: -4.0,
        },
        ExtEffect::Tiler {
            scale_percent: 250.0,
            mirror: true,
        },
        ExtEffect::GlowPro {
            threshold: 0.5,
            radius: 3,
            intensity: 0.9,
        },
        ExtEffect::Vibrance { amount: 40.0 },
        ExtEffect::WhiteBalance {
            temperature: 30.0,
            tint: -10.0,
        },
        ExtEffect::HslAdjust {
            hue_deg: 45.0,
            saturation: 25.0,
            lightness: -8.0,
        },
    ]
}

#[test]
fn test_all_variants_apply_without_panic_and_deterministic() {
    for e in sample_effects() {
        let run = || {
            let mut img = gradient(24, 24);
            e.apply(&mut img, 24, 24, 1.0);
            img
        };
        let a = run();
        let b = run();
        assert_eq!(a, b, "{} not deterministic", e.type_id());
        assert_eq!(a.len(), 24 * 24 * 4, "{} changed buffer size", e.type_id());
    }
}

#[test]
fn test_metadata_catalog_complete() {
    for e in sample_effects() {
        assert!(!e.type_id().is_empty());
        assert!(!e.display_name().is_empty());
        assert!(
            ["Distort", "Stylize", "Blur & Sharpen", "Color Correction"].contains(&e.category()),
            "unknown category {}",
            e.category()
        );
    }
}

#[test]
fn test_serde_roundtrip_preserves_params() {
    for e in sample_effects() {
        let json = serde_json::to_string(&e).unwrap_or_default();
        let back: ExtEffect = serde_json::from_str(&json).unwrap_or(ExtEffect::BendIt {
            top_offset: f32::NAN,
            bottom_offset: 0.0,
        });
        assert_eq!(&back, &e, "{} roundtrip mismatch", e.type_id());
    }
}

#[test]
fn test_serde_backward_compat_minimal_json() {
    // Old/lean JSON relying on serde defaults must deserialize.
    let json = r#"{"WaveWarp":{}}"#;
    let e: ExtEffect = serde_json::from_str(json).unwrap_or(ExtEffect::BendIt {
        top_offset: 0.0,
        bottom_offset: 0.0,
    });
    match e {
        ExtEffect::WaveWarp {
            wave_height,
            wave_width,
            direction_deg,
            ..
        } => {
            assert_eq!(wave_height, 50.0);
            assert_eq!(wave_width, 100.0);
            assert_eq!(direction_deg, 90.0);
        }
        other => panic!("wrong variant deserialized: {other:?}"),
    }

    let json2 = r#"{"ChannelMixer":{"monochrome":true}}"#;
    let e2: ExtEffect = serde_json::from_str(json2).unwrap_or(ExtEffect::BendIt {
        top_offset: 0.0,
        bottom_offset: 0.0,
    });
    match e2 {
        ExtEffect::ChannelMixer { matrix, monochrome } => {
            assert!(monochrome);
            assert_eq!(matrix, identity_matrix());
        }
        other => panic!("wrong variant deserialized: {other:?}"),
    }
}

#[test]
fn test_identity_params_leave_image_unchanged() {
    let src = gradient(16, 16);

    let mut out = src.clone();
    ExtEffect::CcLens {
        convergence: 0.0,
        zoom: 1.0,
    }
    .apply(&mut out, 16, 16, 0.0);
    assert_eq!(out, src);

    let mut out = src.clone();
    ExtEffect::OpticsCompensation {
        field_of_view_deg: 0.0,
        reverse: false,
        zoom: 1.0,
    }
    .apply(&mut out, 16, 16, 0.0);
    assert_eq!(out, src);

    let mut out = src.clone();
    ExtEffect::ChannelMixer {
        matrix: identity_matrix(),
        monochrome: false,
    }
    .apply(&mut out, 16, 16, 0.0);
    assert_eq!(out, src);

    let mut out = src.clone();
    ExtEffect::ColorBalance {
        shadows: [0.0; 3],
        midtones: [0.0; 3],
        highlights: [0.0; 3],
        preserve_luminosity: true,
    }
    .apply(&mut out, 16, 16, 0.0);
    assert_eq!(out, src);
}

#[test]
fn test_wave_warp_time_drives_animation() {
    let mk = |t: f32| {
        let mut img = gradient(32, 32);
        ExtEffect::WaveWarp {
            wave_height: 8.0,
            wave_width: 12.0,
            speed: 1.0,
            direction_deg: 90.0,
            wave_type: 0,
            pinning: 3,
        }
        .apply(&mut img, 32, 32, t);
        img
    };
    let t0 = mk(0.0);
    let t_half = mk(0.5);
    assert_ne!(t0, t_half, "time must advance the wave phase");
}
