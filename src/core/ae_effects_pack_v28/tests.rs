#[cfg(test)]
mod tests {
    use super::super::*;

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

    #[test]
    fn test_light_sweep_adds_brightness_in_band_only() {
        let mut img = gradient(32, 32);
        let before = img.clone();
        apply_light_sweep(
            &mut img,
            32,
            32,
            &LightSweepParams {
                direction_deg: 0.0,
                center: 0.5,
                width: 0.2,
                sweep_intensity: 0.8,
                edge_intensity: 0.0,
            },
        );
        // Centre column must brighten; far-left column must stay identical.
        let mid = ((16 * 32 + 16) * 4) as usize;
        assert!(img[mid] > before[mid], "band centre should brighten");
        let left = ((16 * 32) * 4) as usize;
        assert_eq!(img[left], before[left], "outside band must be untouched");
    }

    #[test]
    fn test_light_sweep_zero_intensity_is_identity() {
        let mut img = gradient(16, 16);
        let before = img.clone();
        apply_light_sweep(
            &mut img,
            16,
            16,
            &LightSweepParams {
                sweep_intensity: 0.0,
                edge_intensity: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(img, before);
    }

    #[test]
    fn test_radial_fast_blur_preserves_center_and_size() {
        let mut img = gradient(24, 24);
        let before = img.clone();
        // Integer-aligned centre: every radial sample lands exactly on the
        // same pixel, so that pixel must be bit-identical.
        apply_radial_fast_blur(&mut img, 24, 24, [12.0, 12.0], 0.5, 8);
        assert_eq!(img.len(), before.len());
        let c = ((12 * 24 + 12) * 4) as usize;
        assert_eq!(img[c], before[c]);
        // Zero amount is identity.
        let mut id = before.clone();
        apply_radial_fast_blur(&mut id, 24, 24, [12.0, 12.0], 0.0, 8);
        assert_eq!(id, before);
    }

    #[test]
    fn test_bend_it_shifts_edges_by_exact_offsets() {
        let mut img = gradient(20, 20);
        let before = img.clone();
        apply_cc_bend_it_pro(&mut img, 20, 20, 5.0, -5.0);
        // Sampling is src = dst + shift:
        // Top row (+5): destination (6,0) shows source (11,0).
        let top_after = (6 * 4) as usize;
        let top_src = (11 * 4) as usize;
        assert!(
            (img[top_after] as i32 - before[top_src] as i32).abs() <= 1,
            "top row must shift by +5px"
        );
        // Bottom row (-5): destination (14,19) shows source (9,19).
        let bot_after = ((19 * 20 + 14) * 4) as usize;
        let bot_src = ((19 * 20 + 9) * 4) as usize;
        assert!(
            (img[bot_after] as i32 - before[bot_src] as i32).abs() <= 1,
            "bottom row must shift by -5px"
        );
        // Middle row eased shift ≈ ±0.4px on a ~12.75 level/px gradient → small drift.
        let mid_before = ((10 * 20 + 10) * 4) as usize;
        let mid_after = ((10 * 20 + 10) * 4) as usize;
        assert!(
            (img[mid_after] as i32 - before[mid_before] as i32).abs() <= 8,
            "middle row must move far less than the edges"
        );
    }

    #[test]
    fn test_tiler_repeat_and_mirror_cover_canvas() {
        let mut a = gradient(16, 16);
        apply_cc_tiler_pro(&mut a, 16, 16, 400.0, TileEdgeMode::Repeat);
        assert!(a.chunks(4).all(|px| px[3] == 255));

        let mut b = gradient(16, 16);
        apply_cc_tiler_pro(&mut b, 16, 16, 400.0, TileEdgeMode::Mirror);
        // Mirror tiles are symmetric about tile boundaries: row 0 equals row 7 mirrored.
        let (r0, r7) = (0usize, (7 * 16) * 4);
        assert_eq!(b[r0], b[r7]);
    }

    #[test]
    fn test_glow_pro_brightens_bright_areas_only() {
        // Dark background with a bright square.
        let mut img = vec![10u8; 32 * 32 * 4];
        for y in 12..20 {
            for x in 12..20 {
                let i = ((y * 32 + x) * 4) as usize;
                img[i] = 250;
                img[i + 1] = 250;
                img[i + 2] = 250;
            }
        }
        let before = img.clone();
        apply_glow_pro(&mut img, 32, 32, 0.7, 4, 1.0);

        // Bright core stays bright (screen keeps ≥ original).
        let core = ((16 * 32 + 16) * 4) as usize;
        assert!(img[core] >= before[core]);
        // Pixel adjacent to the square (outside it) must gain brightness.
        let halo = ((16 * 32 + 21) * 4) as usize;
        assert!(img[halo] > before[halo], "halo should receive bloom");
        // Far corner barely affected.
        let far = ((32 + 1) * 4) as usize;
        assert!(img[far] <= before[far] + 30);
    }

    #[test]
    fn test_glow_pro_identity_when_intensity_zero_or_below_threshold() {
        let mut img = gradient(16, 16);
        let before = img.clone();
        apply_glow_pro(&mut img, 16, 16, 0.5, 4, 0.0);
        assert_eq!(img, before);
        // Everything below threshold → no bright pass → identity.
        let mut dark = vec![5u8; 16 * 16 * 4];
        let dark_before = dark.clone();
        apply_glow_pro(&mut dark, 16, 16, 0.9, 4, 1.0);
        assert_eq!(dark, dark_before);
    }

    #[test]
    fn test_degenerate_inputs_do_not_panic() {
        let mut empty: Vec<u8> = vec![];
        apply_light_sweep(&mut empty, 0, 0, &LightSweepParams::default());
        apply_radial_fast_blur(&mut empty, 0, 0, [0.0, 0.0], 0.5, 8);
        apply_cc_bend_it_pro(&mut empty, 0, 0, 10.0, -10.0);
        apply_cc_tiler_pro(&mut empty, 0, 0, 200.0, TileEdgeMode::Mirror);
        apply_glow_pro(&mut empty, 0, 0, 0.5, 4, 1.0);
    }

    #[test]
    fn test_all_deterministic() {
        let run = || {
            let mut img = gradient(24, 24);
            apply_light_sweep(&mut img, 24, 24, &LightSweepParams::default());
            apply_radial_fast_blur(&mut img, 24, 24, [12.0, 12.0], 0.4, 6);
            apply_cc_bend_it_pro(&mut img, 24, 24, 3.0, -3.0);
            apply_cc_tiler_pro(&mut img, 24, 24, 150.0, TileEdgeMode::Mirror);
            apply_glow_pro(&mut img, 24, 24, 0.4, 2, 0.8);
            img
        };
        assert_eq!(run(), run());
    }
}
