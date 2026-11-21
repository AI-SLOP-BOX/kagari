use super::*;
use std::f32::consts::{PI, TAU};

fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        v.extend_from_slice(&rgba);
    }
    v
}

#[test]
fn test_waveforms_bounded_and_periodic() {
    for wt in [
        WaveType::Sine,
        WaveType::Triangle,
        WaveType::Square,
        WaveType::Sawtooth,
    ] {
        for i in 0..360 {
            // Half-degree offsets avoid landing exactly on the Square
            // waveform's discontinuity at phase = PI.
            let phase = (i as f32 + 0.5) / 360.0 * TAU;
            let v = wt.waveform(phase);
            assert!((-1.0..=1.0).contains(&v), "{wt:?} out of range at {i}");
            // Periodicity: phase + full turn reproduces the value (within fp tolerance).
            let shifted = wt.waveform(phase + TAU);
            assert!(
                (v - shifted).abs() < 1e-4,
                "{wt:?} not periodic at {i}: {v} vs {shifted}"
            );
        }
    }
    assert_eq!(WaveType::Square.waveform(0.0), 1.0);
    assert_eq!(WaveType::Square.waveform(PI), -1.0);
    // Triangle starts at -1 and peaks at half period (phase = PI).
    assert!((WaveType::Triangle.waveform(0.0) + 1.0).abs() < 1e-6);
    assert!((WaveType::Triangle.waveform(PI) - 1.0).abs() < 1e-6);
    assert!((WaveType::Triangle.waveform(PI * 0.5) - 0.0).abs() < 1e-6);
}

#[test]
fn test_wave_warp_preserves_size_and_is_deterministic() {
    let src = solid(16, 16, [128, 64, 32, 255]);
    let mut a = src.clone();
    let mut b = src.clone();
    let params = WaveWarpParams {
        wave_height: 6.0,
        wave_width: 8.0,
        time: 1.5,
        ..Default::default()
    };
    apply_wave_warp_pro(&mut a, 16, 16, &params);
    apply_wave_warp_pro(&mut b, 16, 16, &params);
    assert_eq!(a, b);
    assert_eq!(a.len(), src.len());
}

#[test]
fn test_wave_warp_pin_all_freezes_corners() {
    // Gradient image so corners are distinguishable.
    let mut img = vec![0u8; 24 * 24 * 4];
    for y in 0..24u32 {
        for x in 0..24u32 {
            let i = ((y * 24 + x) * 4) as usize;
            img[i] = (x * 10) as u8;
            img[i + 1] = (y * 10) as u8;
            img[i + 3] = 255;
        }
    }
    let mut warped = img.clone();
    let params = WaveWarpParams {
        wave_height: 20.0,
        wave_width: 10.0,
        pinning: PinKind::All,
        ..Default::default()
    };
    apply_wave_warp_pro(&mut warped, 24, 24, &params);
    for (x, y) in [(0u32, 0u32), (23, 0), (0, 23), (23, 23)] {
        let i = ((y * 24 + x) * 4) as usize;
        assert_eq!(warped[i], img[i], "corner R changed");
        assert_eq!(warped[i + 1], img[i + 1], "corner G changed");
    }
}

#[test]
fn test_cc_lens_center_invariant_and_identity_at_zero() {
    let mut img = vec![0u8; 17 * 17 * 4];
    for y in 0..17u32 {
        for x in 0..17u32 {
            let i = ((y * 17 + x) * 4) as usize;
            img[i] = (x * 15) as u8;
            img[i + 1] = (y * 15) as u8;
            img[i + 3] = 255;
        }
    }
    // Centre pixel must survive any convergence.
    for conv in [-80.0f32, 0.0, 80.0] {
        let mut out = img.clone();
        apply_cc_lens_pro(
            &mut out,
            17,
            17,
            &CcLensParams {
                convergence: conv,
                zoom: 1.0,
            },
        );
        let c = ((8 * 17 + 8) * 4) as usize;
        assert_eq!(out[c], img[c]);
        assert_eq!(out[c + 1], img[c + 1]);
    }
    // Zero convergence + zoom 1 == identity everywhere.
    let mut out = img.clone();
    apply_cc_lens_pro(
        &mut out,
        17,
        17,
        &CcLensParams {
            convergence: 0.0,
            zoom: 1.0,
        },
    );
    assert_eq!(out, img);
}

#[test]
fn test_polar_solid_image_is_stable() {
    let src = solid(20, 20, [10, 200, 30, 255]);
    for mode in [PolarMode::RectToPolar, PolarMode::PolarToRect] {
        let mut out = src.clone();
        apply_polar_coordinates_pro(&mut out, 20, 20, mode, 1.0);
        assert!(
            out.chunks(4).all(|px| px == [10, 200, 30, 255]),
            "{mode:?} broke solid"
        );
    }
    // Interpolation 0 == identity even on structured input.
    let mut grad = vec![0u8; 20 * 20 * 4];
    for (i, px) in grad.chunks_mut(4).enumerate() {
        px[0] = (i % 255) as u8;
        px[3] = 255;
    }
    let mut out = grad.clone();
    apply_polar_coordinates_pro(&mut out, 20, 20, PolarMode::RectToPolar, 0.0);
    assert_eq!(out, grad);
}

#[test]
fn test_optics_compensation_identity_and_symmetry() {
    let mut img = vec![0u8; 16 * 16 * 4];
    for y in 0..16u32 {
        for x in 0..16u32 {
            let i = ((y * 16 + x) * 4) as usize;
            img[i] = (x * 16 + y) as u8;
            img[i + 3] = 255;
        }
    }
    let mut out = img.clone();
    apply_optics_compensation(&mut out, 16, 16, &OpticsCompensationParams::default());
    assert_eq!(out, img, "FOV 0 must be identity");

    // Centre pixel invariant under any FOV.
    for fov in [-120.0f32, 120.0] {
        let mut o = img.clone();
        apply_optics_compensation(
            &mut o,
            16,
            16,
            &OpticsCompensationParams {
                field_of_view_deg: fov,
                ..Default::default()
            },
        );
        let c = ((8 * 16 + 8) * 4) as usize;
        assert_eq!(o[c], img[c]);
    }
}

#[test]
fn test_sample_bilinear_clamps_and_interpolates() {
    let img = solid(2, 2, [0, 0, 0, 255]);
    let mut out = [0u8; 4];
    wave_warp::sample_bilinear(&img, 2, 2, -5.0, -5.0, &mut out);
    assert_eq!(out, [0, 0, 0, 255]);

    // Horizontal midpoint between black and white columns.
    let mut bw = vec![0u8; 2 * 4];
    bw[0..4].copy_from_slice(&[0, 0, 0, 255]);
    bw[4..8].copy_from_slice(&[200, 200, 200, 255]);
    wave_warp::sample_bilinear(&bw, 2, 1, 0.5, 0.0, &mut out);
    assert_eq!(out[0], 100);
}

#[test]
fn test_degenerate_inputs_do_not_panic() {
    let mut empty: Vec<u8> = vec![];
    apply_wave_warp_pro(&mut empty, 0, 0, &WaveWarpParams::default());
    apply_cc_lens_pro(&mut empty, 0, 0, &CcLensParams::default());
    apply_polar_coordinates_pro(&mut empty, 0, 0, PolarMode::RectToPolar, 1.0);
    apply_optics_compensation(&mut empty, 0, 0, &OpticsCompensationParams::default());
}
