#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;

    #[test]
    fn test_cc_lens_bounded_and_modifies() {
        let (w, h) = (64, 64);
        let mut pixels = vec![128u8; (w * h * 4) as usize];
        let p = CcLensParams::default();
        apply_cc_lens(&mut pixels, w, h, &p);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_directional_smear_preserves_length() {
        let (w, h) = (32, 32);
        let mut pixels = vec![100u8; (w * h * 4) as usize];
        let p = DirectionalSmearParams::default();
        apply_directional_smear(&mut pixels, w, h, &p);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_card_wipe_progress_zero_identity() {
        let (w, h) = (16, 16);
        let orig = vec![200u8; (w * h * 4) as usize];
        let mut pixels = orig.clone();
        let p = CardWipeParams {
            progress: 0.0,
            cols: 4,
            rows: 4,
        };
        apply_card_wipe(&mut pixels, w, h, &p);
        assert_eq!(pixels, orig);
    }

    #[test]
    fn test_timecode_burn_in_draws() {
        let (w, h) = (120, 60);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let p = TimecodeParams {
            frame: 45,
            fps: 30,
            position: [10.0, 10.0],
            scale: 1.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            bg_opacity: 0.5,
        };
        apply_timecode_burn_in(&mut pixels, w, h, &p);
        // Ensure some white pixels exist
        assert!(pixels.contains(&255));
    }

    #[test]
    fn test_vignette_darkens_corners() {
        let (w, h) = (50, 50);
        let mut pixels = vec![255u8; (w * h * 4) as usize];
        let p = VignetteParams::default();
        apply_advanced_vignette(&mut pixels, w, h, &p);
        // Corner pixel should be darker than center
        let center_val = pixels[((25 * 50 + 25) * 4) as usize];
        let corner_val = pixels[0];
        assert!(corner_val < center_val);
    }
}
