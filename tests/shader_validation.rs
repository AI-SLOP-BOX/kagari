//! Compile-time WGSL validation: catches shader syntax/type errors in CI
//! instead of at GUI startup (wgpu compiles lazily at runtime).

const SHADER: &str = include_str!("../src/core/shader.wgsl");

#[test]
fn shader_wgsl_parses_and_validates() {
    let module =
        naga::front::wgsl::parse_str(SHADER).expect("shader.wgsl must parse as valid WGSL");

    // Full type validation (entry point signatures, uniform layouts, etc.)
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let info = validator
        .validate(&module)
        .expect("shader.wgsl must pass naga type validation");

    // Sanity: the entry points we bind must exist
    let ep_names: Vec<&str> = module
        .entry_points
        .iter()
        .map(|ep| ep.name.as_str())
        .collect();
    assert!(ep_names.contains(&"vs_main"), "vs_main entry point missing");
    assert!(ep_names.contains(&"fs_main"), "fs_main entry point missing");
    let _ = info;
}

#[test]
fn levels_adjustment_is_applied_once_with_export_matching_guards() {
    assert_eq!(SHADER.matches("if (layer.levels_enabled == 1u)").count(), 1);
    assert!(SHADER.contains("if (layer.levels_gamma > 0.0)"));
    assert!(SHADER.contains("vec3<f32>(0.0),\n                vec3<f32>(1.0)"));
}

#[test]
fn color_tint_shader_matches_rgba8_color_and_amount_ranges() {
    let color_tint = SHADER
        .split("// --- Color Tint ---")
        .nth(1)
        .expect("Color Tint shader block should exist")
        .split("// --- Drop Shadow ---")
        .next()
        .expect("Color Tint block should end before Drop Shadow");

    assert!(color_tint.contains("round("));
    assert!(color_tint.contains("* 255.0"));
    assert!(color_tint.contains("clamp(layer.effect_tint_intensity, 0.0, 1.0)"));
}

#[test]
fn lens_flare_shader_matches_cpu_bounds_and_rgba8_writeback() {
    let lens_flare = SHADER
        .split("// ── Lens Flare ──")
        .nth(1)
        .expect("Lens Flare shader block should exist")
        .split("// ──")
        .next()
        .expect("Lens Flare block should have a following effect");

    assert!(lens_flare.contains("layer.flare_intensity > 0.0"));
    assert!(lens_flare.contains("vec2<f32>(0.0)"));
    assert!(lens_flare.contains("flare_total >= 0.002"));
    assert!(lens_flare.contains("* 255.0"));
    assert!(lens_flare.contains("flare_pixels / 255.0"));
}
