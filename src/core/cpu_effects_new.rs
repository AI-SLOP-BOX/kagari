//! Extended CPU-rendered effects collection.
//!
//! Modularized into submodules:
//! - `color`: Levels, Hue/Saturation, Color Grade LUT, Color Space Convert, Curves
//! - `lens`: Chromatic Aberration, Vignette, Film Grain
//! - `warp`: Motion Blur, Mesh Warp, Displacement Map, Bilinear sampling
//! - `procedural`: Fractal Noise, Compound Blur, Minimax, Shift Channels

#![allow(dead_code)]

pub mod color;
pub mod lens;
pub mod procedural;
pub mod warp;

// Re-export all public functions to keep full backwards compatibility
pub use color::{
    apply_color_grade_lut, apply_color_space_convert, apply_curves, apply_hue_saturation,
    apply_levels,
};
pub use lens::{apply_chromatic_aberration, apply_film_grain, apply_vignette};
pub use procedural::{
    apply_compound_blur, apply_compound_blur_with_map, apply_fractal_noise, apply_minimax,
    apply_shift_channels,
};
pub use warp::{apply_displacement_map, apply_mesh_warp, apply_motion_blur, sample_bilinear};

#[cfg(test)]
mod ca_radial_tests {
    use super::*;

    #[test]
    fn compound_blur_uses_external_layer_as_intensity_map() {
        let mut target = vec![0u8, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 255];
        let map = vec![0u8, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255];

        apply_compound_blur_with_map(&mut target, 3, 1, Some(&map), 2.0);

        // Only the bright center of the source map receives blur. The dark
        // neighbors stay black, proving the map is not read from the target.
        assert!(target[4] < 255);
        assert_eq!(target[0], 0);
        assert_eq!(target[8], 0);
    }

    #[test]
    fn test_ca_center_pixel_unchanged() {
        // At the exact center, falloff direction is zero → no shift anywhere
        let w = 33u32;
        let h = 33u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        for i in (0..px.len()).step_by(4) {
            px[i] = 200;
            px[i + 1] = 100;
            px[i + 2] = 50;
            px[i + 3] = 255;
        }
        let before = px.clone();
        apply_chromatic_aberration(&mut px, w, h, 20.0, 20.0, 0.0);
        let center = ((16 * w + 16) * 4) as usize;
        assert_eq!(px[center], before[center], "center R unchanged");
        assert_eq!(px[center + 1], before[center + 1]);
    }

    #[test]
    fn test_ca_radial_fringes_left_and_right_symmetrically() {
        // A white block on black: radial fringing must push red outward on BOTH
        // sides of the block (left side of block gains red on its left, right
        // side loses red beyond its right edge).
        let w = 64u32;
        let h = 32u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 12..20 {
            for x in 16..48 {
                let idx = ((y * w + x) * 4) as usize;
                px[idx] = 255;
                px[idx + 1] = 255;
                px[idx + 2] = 255;
                px[idx + 3] = 255;
            }
        }
        apply_chromatic_aberration(&mut px, w, h, 6.0, 6.0, 0.0);

        // Probe row through the block's vertical center.
        // Radial CA creates mirrored channel separation around the center:
        // whichever way each side fringes, the pattern must be point-symmetric.
        let row = 15usize;
        let separation_at = |x: usize| -> i32 {
            let idx = (row * w as usize + x) * 4;
            px[idx] as i32 - px[idx + 2] as i32 // R - B
        };
        // Sum |separation| near both edges of the block
        let sep_left: i32 = (14..20).map(|x| separation_at(x).abs()).sum();
        let sep_right: i32 = (44..50).map(|x| separation_at(x).abs()).sum();
        assert!(
            sep_left > 30,
            "channel separation expected at left edge, got {}",
            sep_left
        );
        assert!(
            sep_right > 30,
            "channel separation expected at right edge, got {}",
            sep_right
        );
    }
}
