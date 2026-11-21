use super::cinema_log::*;
use super::color_spaces::*;
use super::curves_lut::*;
use super::hdr_buffer::*;

#[test]
fn test_hsl_roundtrip() {
    let original = [0.2, 0.5, 0.8]; // Soft blue
    let hsl = rgb_to_hsl(original[0], original[1], original[2]);
    let roundtrip = hsl_to_rgb(hsl[0], hsl[1], hsl[2]);

    assert!((original[0] - roundtrip[0]).abs() < 1e-4);
    assert!((original[1] - roundtrip[1]).abs() < 1e-4);
    assert!((original[2] - roundtrip[2]).abs() < 1e-4);
}

#[test]
fn test_levels_mapping() {
    // High contrast mapping
    let output = apply_levels(0.5, 0.2, 0.8, 1.0, 0.0, 1.0);
    assert!((output - 0.5).abs() < 1e-4);
    // Black/white points pin the intercept: below-black clamps to out_black.
    assert!((apply_levels(0.2, 0.2, 0.8, 1.0, 0.0, 1.0)).abs() < 1e-4);
    assert!((apply_levels(0.8, 0.2, 0.8, 1.0, 0.0, 1.0) - 1.0).abs() < 1e-4);
    assert!((apply_levels(0.0, 0.2, 0.8, 1.0, 0.0, 1.0)).abs() < 1e-4);
    // Gamma: normalized 0.5 raised to 1/2.
    let g = apply_levels(0.5, 0.0, 1.0, 2.0, 0.0, 1.0);
    assert!((g - 0.5f32.sqrt()).abs() < 1e-4, "gamma curve wrong: {}", g);
    // Output range remap: normalized 0.25 lands at 0.2 + 0.25 * 0.6.
    let o = apply_levels(0.25, 0.0, 1.0, 1.0, 0.2, 0.8);
    assert!((o - 0.35).abs() < 1e-4, "output range wrong: {}", o);
}

#[test]
fn test_lut3d_tetrahedral_identity() {
    let lut = Lut3D::identity(17);
    let mapped = lut.apply(0.5, 0.25, 0.75);
    assert!((mapped.0 - 0.5).abs() < 1e-3);
    assert!((mapped.1 - 0.25).abs() < 1e-3);
    assert!((mapped.2 - 0.75).abs() < 1e-3);
}

#[test]
fn test_lut3d_nan_input_sanitization() {
    let lut = Lut3D::identity(4);
    let (r, g, b) = lut.apply(f32::NAN, f32::NAN, f32::NAN);
    // NaN inputs must be sanitized to 0.0 and produce valid (non-NaN) outputs
    assert!(!r.is_nan(), "Lut3D::apply returned NaN for NaN r input");
    assert!(!g.is_nan(), "Lut3D::apply returned NaN for NaN g input");
    assert!(!b.is_nan(), "Lut3D::apply returned NaN for NaN b input");
    // Identity LUT with 0.0 input → should return near 0.0
    assert!(r.abs() < 0.01);
    assert!(g.abs() < 0.01);
    assert!(b.abs() < 0.01);
}

#[test]
fn test_cinema_log_roundtrip() {
    let profiles = [
        CinemaLogProfile::ArriLogC3,
        CinemaLogProfile::ArriLogC4,
        CinemaLogProfile::SonySLog3,
        CinemaLogProfile::RedLog3G10,
        CinemaLogProfile::CanonCLog2,
        CinemaLogProfile::CanonCLog3,
        CinemaLogProfile::BmdFilmGen5,
    ];
    let test_values = [0.01f32, 0.18f32, 0.5f32, 1.0f32, 2.0f32];

    for &profile in &profiles {
        for &val in &test_values {
            let log = linear_to_cinema_log(val, profile);
            let roundtrip = cinema_log_to_linear(log, profile);
            assert!(
                (roundtrip - val).abs() < 1e-3 * val.max(1.0),
                "profile {:?}: linear {} -> log {} -> linear {}",
                profile,
                val,
                log,
                roundtrip
            );
        }
    }
}

#[test]
fn test_working_color_space_conversions() {
    let white = [1.0f32, 1.0, 1.0];
    let p3 = convert_color_space(
        white,
        WorkingColorSpace::Rec709,
        WorkingColorSpace::DisplayP3,
    );
    assert!((p3[0] - 1.0).abs() < 0.05);
    assert!((p3[1] - 1.0).abs() < 0.05);
    assert!((p3[2] - 1.0).abs() < 0.05);

    let black = [0.0f32, 0.0, 0.0];
    let lin_black = convert_color_space(
        black,
        WorkingColorSpace::Rec709,
        WorkingColorSpace::LinearSRGB,
    );
    assert_eq!(lin_black, [0.0, 0.0, 0.0]);

    // Saturated primaries must survive a roundtrip: catches transposed or
    // mis-scaled conversion matrices without needing reference values.
    for primary in [[1.0f32, 0.0, 0.0], [0.0, 0.0, 1.0]] {
        let there = convert_color_space(
            primary,
            WorkingColorSpace::Rec709,
            WorkingColorSpace::DisplayP3,
        );
        // P3 gamut differs, so the roundtrip (not the forward hop) is pinned.
        let back = convert_color_space(
            there,
            WorkingColorSpace::DisplayP3,
            WorkingColorSpace::Rec709,
        );
        for c in 0..3 {
            assert!(
                (back[c] - primary[c]).abs() < 1e-3,
                "roundtrip drifted channel {}: {:?} -> {:?} -> {:?}",
                c,
                primary,
                there,
                back
            );
        }
    }
}

#[test]
fn test_hdr_f32_buffer_roundtrip() {
    let rgba8 = vec![255, 128, 64, 255];
    let hdr = HdrF32Buffer::from_rgba8(&rgba8, 1, 1);
    assert!(hdr.data[0] > 0.9); // Linear white
    let back = hdr.to_rgba8(false, 0.0);
    assert_eq!(back[0], 255);
    assert!((back[1] as i32 - 128).abs() <= 1);
}

#[test]
fn test_hdr_linear_alpha_blend() {
    let mut bg = HdrF32Buffer::new(1, 1);
    bg.data[0] = 1.0; // Red
    bg.data[3] = 1.0;

    let mut fg = HdrF32Buffer::new(1, 1);
    fg.data[1] = 1.0; // Green
    fg.data[3] = 0.5;

    bg.blend_over(&fg, 1.0);
    assert!(bg.data[0] > 0.0 && bg.data[1] > 0.0);
    assert_eq!(bg.data[3], 1.0);
}

#[test]
fn test_hdr_32bpc_overbright_preservation() {
    let mut bg = HdrF32Buffer::new(1, 1);
    bg.data[0] = 2.5; // Over-bright HDR red (exceeds 1.0)
    bg.data[1] = 1.2;
    bg.data[2] = 0.0;
    bg.data[3] = 1.0;

    let mut fg = HdrF32Buffer::new(1, 1);
    fg.data[0] = 1.0;
    fg.data[1] = 3.0; // Over-bright HDR green
    fg.data[2] = 0.0;
    fg.data[3] = 1.0;

    // Additive blend in 32bpc float space preserves HDR values
    bg.blend_layer_mode(&fg, 1.0, crate::core::timeline::BlendMode::Add);
    assert!(bg.data[0] >= 3.5);
    assert!(bg.data[1] >= 4.2);

    // Tonemapping compresses into [0, 255] without clipping artifacts
    let tonemapped = bg.to_rgba8(true, 0.0);
    assert_eq!(tonemapped[3], 255);
}

#[test]
fn test_bit_depth_cycle() {
    let b = BitDepth::EightBit;
    assert_eq!(b.next(), BitDepth::SixteenBit);
    assert_eq!(b.next().next(), BitDepth::ThirtyTwoBitFloat);
    assert_eq!(b.next().next().next(), BitDepth::EightBit);
    assert!(BitDepth::ThirtyTwoBitFloat.is_32bpc());
    assert!(BitDepth::ThirtyTwoBitFloat.is_hdr());
    assert!(!BitDepth::EightBit.is_hdr());
}
