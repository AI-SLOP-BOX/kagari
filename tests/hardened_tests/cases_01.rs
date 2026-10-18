#[test]
fn shader_wgsl_parses_and_has_both_entry_points() {
    let source = include_str!("../../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source);
    assert!(
        module.is_ok(),
        "shader.wgsl failed to parse: {:?}",
        module.err()
    );
    let module = module.unwrap();
    let names: Vec<&str> = module
        .entry_points
        .iter()
        .map(|ep| ep.name.as_str())
        .collect();
    assert!(names.contains(&"vs_main"), "shader.wgsl missing vs_main");
    assert!(names.contains(&"fs_main"), "shader.wgsl missing fs_main");
}
#[test]
fn shader_bind_groups_within_limits() {
    let source = include_str!("../../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let max_group = module
        .global_variables
        .iter()
        .filter_map(|(_, gv)| gv.binding.as_ref())
        .map(|b| b.group)
        .max()
        .unwrap_or(0);
    // max_bind_groups=6 (0..5 inclusive), shader uses @group(5) for matte texture
    assert!(
        max_group < 6,
        "shader.wgsl uses @group({max_group}) but max_bind_groups=6 allows groups 0..5"
    );
}
#[test]
fn shader_passes_full_validation() {
    let source = include_str!("../../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let _info = validator
        .validate(&module)
        .expect("shader.wgsl must pass naga type validation");
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Effect Parameter Boundary Sweeps
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn effects_extreme_values_no_panic() {
    let params = vec![
        (
            "blur_zero",
            vec![fx(EffectType::GaussianBlur {
                blur_radius: c32(0.0),
            })],
        ),
        (
            "blur_max",
            vec![fx(EffectType::GaussianBlur {
                blur_radius: c32(1000.0),
            })],
        ),
        (
            "tint_zero",
            vec![fx(EffectType::ColorTint {
                color: c32a4([1.0, 0.0, 0.0, 1.0]),
                intensity: c32(0.0),
            })],
        ),
        (
            "tint_max",
            vec![fx(EffectType::ColorTint {
                color: c32a4([1.0, 1.0, 1.0, 1.0]),
                intensity: c32(100.0),
            })],
        ),
        (
            "shadow_extreme",
            vec![fx(EffectType::DropShadow {
                color: c32a4([0.0, 0.0, 0.0, 1.0]),
                opacity: c32(999.0),
                direction: c32(-999.0),
                distance: c32(9999.0),
                softness: c32(999.0),
            })],
        ),
        (
            "ca_extreme",
            vec![fx(EffectType::ChromaticAberration {
                shift_r: c32(9999.0),
                shift_b: c32(-9999.0),
                edge_falloff: c32(0.0),
                iris_linked: false,
            })],
        ),
        (
            "vignette_extreme",
            vec![fx(EffectType::Vignette {
                intensity: c32(9999.0),
                roundness: c32(-100.0),
                feather: c32(0.0),
                color: c32a4([1.0, 0.0, 0.0, 1.0]),
            })],
        ),
        (
            "levels_inverted",
            vec![fx(EffectType::Levels {
                input_black: c32(1.0),
                input_white: c32(0.0),
                gamma: c32(-5.0),
                output_black: c32(999.0),
                output_white: c32(-999.0),
            })],
        ),
        (
            "hue_sat_extreme",
            vec![fx(EffectType::HueSaturation {
                hue_shift: c32(99999.0),
                saturation: c32(-100.0),
                lightness: c32(999.0),
            })],
        ),
        (
            "glow_extreme",
            vec![fx(EffectType::Glow {
                threshold: c32(0.0),
                radius: c32(9999.0),
                intensity: c32(999.0),
                color: c32a4([1.0, 1.0, 1.0, 1.0]),
            })],
        ),
        (
            "twirl_extreme",
            vec![fx(EffectType::Twirl {
                angle: c32(99999.0),
                radius: c32(0.0),
            })],
        ),
        (
            "bulge_extreme",
            vec![fx(EffectType::Bulge {
                amount: c32(-999.0),
                radius: c32(0.001),
            })],
        ),
        (
            "posterize_zero",
            vec![fx(EffectType::Posterize { levels: c32(0.0) })],
        ),
        (
            "threshold_extreme",
            vec![fx(EffectType::Threshold {
                threshold: c32(999.0),
            })],
        ),
        (
            "dir_blur_extreme",
            vec![fx(EffectType::DirectionalBlur {
                angle: c32(720.0),
                length: c32(9999.0),
            })],
        ),
        (
            "choker_extreme",
            vec![fx(EffectType::SimpleChoker {
                choke_amount: c32(9999.0),
            })],
        ),
        (
            "turb_displace_extreme",
            vec![fx(EffectType::TurbulentDisplace {
                amount: c32(99999.0),
                size: c32(0.001),
                evolution: c32(std::f32::consts::PI),
                complexity: c32(99.0),
            })],
        ),
        (
            "film_grain_extreme",
            vec![fx(EffectType::FilmGrain {
                intensity: c32(999.0),
                grain_size: 0.001,
                color_film: true,
            })],
        ),
        (
            "sharpen_extreme",
            vec![fx(EffectType::Sharpen {
                amount: c32(9999.0),
            })],
        ),
        (
            "invert",
            vec![fx(EffectType::Invert { invert_alpha: true })],
        ),
        (
            "invert_alpha",
            vec![fx(EffectType::Invert {
                invert_alpha: false,
            })],
        ),
        (
            "radial_blur_extreme",
            vec![fx(EffectType::RadialBlur {
                amount: c32(9999.0),
            })],
        ),
        (
            "linear_wipe_extreme",
            vec![fx(EffectType::LinearWipe {
                completion: c32(999.0),
                angle: c32(-720.0),
            })],
        ),
        (
            "offset_extreme",
            vec![fx(EffectType::Offset {
                shift_x: c32(99999.0),
                shift_y: c32(-99999.0),
            })],
        ),
        (
            "motion_blur_extreme",
            vec![fx(EffectType::MotionBlur {
                shutter_angle: c32(9999.0),
                samples: 256,
            })],
        ),
    ];
    effects_boundary_sweep(params);
}
#[test]
fn representative_effects_preserve_their_pixel_semantics() {
    let w = 4;
    let h = 1;
    let input = vec![
        10, 20, 30, 255, // below the threshold
        120, 120, 120, 200, 200, 180, 160, 128, 250, 240, 230, 64,
    ];

    let mut inverted = input.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut inverted,
        w,
        h,
        &[fx(EffectType::Invert {
            invert_alpha: false,
        })],
        0,
        30,
    );
    assert_eq!(
        inverted,
        vec![245, 235, 225, 255, 135, 135, 135, 200, 55, 75, 95, 128, 5, 15, 25, 64],
        "Invert must transform RGB exactly and preserve alpha when disabled"
    );

    let mut thresholded = input.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut thresholded,
        w,
        h,
        &[fx(EffectType::Threshold {
            threshold: c32(128.0),
        })],
        0,
        30,
    );
    assert_eq!(
        thresholded,
        vec![0, 0, 0, 255, 0, 0, 0, 200, 255, 255, 255, 128, 255, 255, 255, 64],
        "Threshold must produce binary RGB values from luma without changing alpha"
    );

    let mut neutral_tint = input.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut neutral_tint,
        w,
        h,
        &[fx(EffectType::ColorTint {
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
            intensity: c32(0.0),
        })],
        0,
        30,
    );
    assert_eq!(
        neutral_tint, input,
        "zero tint intensity must be an identity operation"
    );
}
#[test]
fn effects_nan_safety() {
    let w = 8u32;
    let h = 8u32;
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(f32::NAN),
        }),
        fx(EffectType::ColorTint {
            color: c32a4([f32::NAN, 0.0, 0.0, 1.0]),
            intensity: c32(f32::NAN),
        }),
        fx(EffectType::DropShadow {
            color: c32a4([0.0, 0.0, 0.0, f32::NAN]),
            opacity: c32(f32::NAN),
            direction: c32(f32::NAN),
            distance: c32(f32::NAN),
            softness: c32(f32::NAN),
        }),
        fx(EffectType::Glow {
            threshold: c32(f32::NAN),
            radius: c32(f32::NAN),
            intensity: c32(f32::NAN),
            color: c32a4([f32::NAN; 4]),
        }),
        fx(EffectType::Vignette {
            intensity: c32(f32::NAN),
            roundness: c32(f32::NAN),
            feather: c32(f32::NAN),
            color: c32a4([f32::NAN; 4]),
        }),
    ];
    let mut pixels = vec![128u8; (w * h * 4) as usize];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels,
        w,
        h,
        &effects,
        0,
        30,
    );
    assert_eq!(pixels.len(), (w * h * 4) as usize);
}
#[test]
fn effects_inf_safety() {
    let w = 8u32;
    let h = 8u32;
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(f32::INFINITY),
        }),
        fx(EffectType::Twirl {
            angle: c32(f32::INFINITY),
            radius: c32(f32::INFINITY),
        }),
        fx(EffectType::Bulge {
            amount: c32(f32::NEG_INFINITY),
            radius: c32(f32::INFINITY),
        }),
    ];
    let mut pixels = vec![128u8; (w * h * 4) as usize];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels,
        w,
        h,
        &effects,
        0,
        30,
    );
    assert_eq!(pixels.len(), (w * h * 4) as usize);
}
#[test]
fn effects_tiny_buffer_boundary() {
    let effects = vec![fx(EffectType::GaussianBlur {
        blur_radius: c32(5.0),
    })];
    let mut pixels_1x1 = vec![200u8; 4];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels_1x1,
        1,
        1,
        &effects,
        0,
        30,
    );
    assert_eq!(pixels_1x1.len(), 4);

    let mut pixels_2x2 = vec![200u8; 16];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut pixels_2x2,
        2,
        2,
        &effects,
        0,
        30,
    );
    assert_eq!(pixels_2x2.len(), 16);

    let mut empty = vec![0u8; 0];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut empty, 0, 0, &effects, 0, 30,
    );
}
