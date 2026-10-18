#[cfg(test)]
mod gpu_mask_tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<[f32; 2]> {
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    }

    #[test]
    fn evenodd_fill_rect_interior_only() {
        let cov = rasterize_polygon_evenodd(&rect(2.0, 2.0, 4.0, 4.0), 8, 8);
        assert_eq!(cov.len(), 64);
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 1.0, "interior must be covered");
        assert_eq!(get(0, 0), 0.0, "outside must be uncovered");
        assert_eq!(get(7, 7), 0.0, "far corner must be uncovered");
    }

    #[test]
    fn degenerate_polygon_yields_no_coverage() {
        let cov = rasterize_polygon_evenodd(&[[0.0, 0.0], [5.0, 5.0]], 8, 8);
        assert!(cov.iter().all(|&c| c == 0.0));
    }

    #[test]
    fn add_mode_unions_adjacent_rects() {
        let shapes = [
            MaskShape {
                poly: rect(0.0, 0.0, 4.0, 2.0),
                mode: crate::core::mask::MaskMode::Add,
                opacity: 1.0,
                inverted: false,
                feather: 0.0,
            },
            MaskShape {
                poly: rect(4.0, 0.0, 4.0, 2.0),
                mode: crate::core::mask::MaskMode::Add,
                opacity: 1.0,
                inverted: false,
                feather: 0.0,
            },
        ];
        let cov = combine_mask_shapes(&shapes, 8, 2).expect("masks present");
        assert!(
            cov.iter().all(|&c| c == 1.0),
            "union of both halves covers the row"
        );
    }

    #[test]
    fn subtract_first_mask_carves_full_frame() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Subtract,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        let cov = combine_mask_shapes(&shapes, 8, 8).expect("masks present");
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 0.0, "carved hole must be transparent");
        assert_eq!(get(0, 0), 1.0, "frame outside the hole stays opaque");
    }

    #[test]
    fn inverted_first_add_reveals_outside_only() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: true,
            feather: 0.0,
        }];
        let cov = combine_mask_shapes(&shapes, 8, 8).expect("masks present");
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 0.0);
        assert_eq!(get(0, 0), 1.0);
    }

    #[test]
    fn intersect_mode_takes_minimum() {
        let shapes = [
            MaskShape {
                poly: rect(0.0, 0.0, 6.0, 2.0),
                mode: crate::core::mask::MaskMode::Add,
                opacity: 1.0,
                inverted: false,
                feather: 0.0,
            },
            MaskShape {
                poly: rect(2.0, 0.0, 6.0, 2.0),
                mode: crate::core::mask::MaskMode::Intersect,
                opacity: 1.0,
                inverted: false,
                feather: 0.0,
            },
        ];
        let cov = combine_mask_shapes(&shapes, 8, 2).expect("masks present");
        assert_eq!(cov[1], 0.0, "only in first");
        assert_eq!(cov[4], 1.0, "in both");
        assert_eq!(cov[7], 0.0, "only in second");
    }

    #[test]
    fn none_and_empty_masks_return_none() {
        assert!(combine_mask_shapes(&[], 8, 8).is_none());
        let disabled = [MaskShape {
            poly: rect(0.0, 0.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::None,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        assert!(combine_mask_shapes(&disabled, 8, 8).is_none());
    }

    #[test]
    fn raster_key_is_deterministic_input_hash() {
        let shape = |feather: f32, mode: crate::core::mask::MaskMode| MaskShape {
            poly: rect(1.0, 1.0, 4.0, 4.0),
            mode,
            opacity: 1.0,
            inverted: false,
            feather,
        };
        let a = mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 8, 8);
        let b = mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 8, 8);
        assert_eq!(a, b, "identical inputs must hash identically");
        assert_ne!(
            mask_input_key(&[shape(2.5, crate::core::mask::MaskMode::Add)], 8, 8),
            a,
            "feather participates in the key"
        );
        assert_ne!(
            mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Subtract)], 8, 8),
            a,
            "mode participates in the key"
        );
        assert_ne!(
            mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 16, 8),
            a
        );
    }

    #[test]
    fn mask_raster_cache_fifo_evicts_oldest() {
        let mut cache = MaskRasterCache::default();
        let dummy = |k: u64| MaskRaster {
            key: k,
            pixels: vec![255, 255, 255, k as u8],
        };
        for k in 0..MaskRasterCache::CAP as u64 {
            cache.insert(std::sync::Arc::new(dummy(k)));
        }
        assert!(cache.get(0).is_some(), "all six fit within capacity");
        // Insert one more → oldest (key 0) evicted, newest present.
        cache.insert(std::sync::Arc::new(dummy(MaskRasterCache::CAP as u64)));
        assert!(cache.get(0).is_none(), "FIFO must drop the oldest");
        assert!(cache.get(MaskRasterCache::CAP as u64).is_some());
    }

    #[test]
    fn edt_distance_matches_manhattan_reference() {
        // 4x4 grid with a single covered pixel at (1,1): Euclidean distance to
        // it must beat or equal any Manhattan path and match exact diagonals.
        let bin = vec![
            false, false, false, false, false, true, false, false, false, false, false, false,
            false, false, false, false,
        ];
        let d = edt_2d(&bin, 4, 4, true);
        assert_eq!(d[4 + 1], 0.0);
        assert_eq!(d[4 + 2], 1.0);
        assert!((d[2 * 4 + 2] - std::f32::consts::SQRT_2).abs() < 1e-5);
        assert!((d[3 * 4 + 3] - 8.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn feather_zero_keeps_hard_edge() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        let out = combine_mask_shapes(&shapes, 8, 8).expect("mask present");
        let raw = rasterize_polygon_evenodd(&rect(2.0, 2.0, 4.0, 4.0), 8, 8);
        for (a, b) in raw.iter().zip(out.iter()) {
            assert_eq!(a, b, "zero feather must keep binary coverage intact");
        }
    }

    #[test]
    fn feather_creates_ramp_centered_on_boundary() {
        // Rect with > feather/2 margins on every side so the probe saturates.
        let poly = rect(4.0, 4.0, 14.0, 8.0);
        let shapes = [MaskShape {
            poly,
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: false,
            feather: 4.0,
        }];
        let out = combine_mask_shapes(&shapes, 24, 16).expect("mask present");
        let get = |px: usize, py: usize| out[py * 24 + px];
        assert_eq!(get(11, 8), 1.0, "feather/2 inside saturates opaque");
        assert_eq!(get(23, 8), 0.0, "far outside stays transparent");
        // Right boundary sits between columns 17 and 18: both land mid-ramp.
        assert!(get(17, 8) > 0.0 && get(17, 8) < 1.0, "inner edge pixel");
        assert!(get(18, 8) > 0.0 && get(18, 8) < 1.0, "outer edge pixel");
        assert!(
            (get(17, 8) - get(18, 8)).abs() < 0.35,
            "ramp symmetric around edge"
        );
    }
}

/// End-to-end parity checks between the CPU reference renderer's mask
/// handling and this file's coverage rasterizer (which feeds the GPU path).
///
/// Known CPU divergences (documented, not asserted here): the software
/// renderer honors only the FIRST enabled non-None mask and ignores
/// mode-based combining, per-mask opacity, expansion, and wiggle — the GPU
/// rasterizer implements the full superset.
#[cfg(test)]
mod cpu_parity_tests {
    use super::*;
    use crate::core::mask::Mask;
    use crate::core::timeline::{Composition, Layer, LayerType};

    const W: u32 = 64;
    const H: u32 = 64;

    fn solid_with_rect_mask(mode: crate::core::mask::MaskMode, inverted: bool) -> Composition {
        let mut comp = Composition::new("c".into(), "Parity".into(), W, H, 30, 30);
        comp.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut layer = Layer::new(
            "l".into(),
            "Solid".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        let mut mask = Mask::new_rect("m".into(), "M".into(), 8.0, 8.0, 24.0, 40.0);
        mask.mode = mode;
        mask.inverted = inverted;
        layer.masks.push(mask);
        comp.layers.push(layer);
        comp
    }

    /// Renders the comp through the CPU reference and through our coverage
    /// rasterizer, then asserts both agree at an interior and an exterior
    /// probe pixel.
    fn assert_cpu_and_gpu_agree(inverted: bool) {
        use crate::core::mask::MaskMode;
        let comp = solid_with_rect_mask(MaskMode::Add, inverted);
        let cpu = crate::core::software_renderer::render_frame_to_pixels(&comp, 0, W, H, 0.0, 0);

        let (key, shapes) =
            collect_mask_shapes(&comp.layers[0], 0, W, H, W, H).expect("mask present");
        let raster = rasterize_from_shapes(&shapes, W, H).expect("coverage");
        assert_eq!(raster.key, key, "key must derive from the same inputs");

        // Rect spans x[8..32) y[8..48): interior (20,32), exterior (52,32).
        for &(px, py, inside_polygon) in &[(20usize, 32usize, true), (52usize, 32usize, false)] {
            let i = (py * W as usize + px) * 4;
            let cpu_red = cpu[i];
            let my_alpha = raster.pixels[i + 3];
            let expect_white = inside_polygon != inverted;
            if expect_white {
                assert!(
                    cpu_red >= 250,
                    "CPU should show the solid at ({},{}), red={}",
                    px,
                    py,
                    cpu_red
                );
                assert_eq!(my_alpha, 255, "GPU coverage opaque at ({},{})", px, py);
            } else {
                assert!(
                    cpu_red <= 8,
                    "CPU should show background at ({},{}), red={}",
                    px,
                    py,
                    cpu_red
                );
                assert_eq!(my_alpha, 0, "GPU coverage clear at ({},{})", px, py);
            }
        }
    }

    #[test]
    fn add_mask_matches_cpu_reference() {
        assert_cpu_and_gpu_agree(false);
    }

    #[test]
    fn inverted_add_mask_matches_cpu_reference() {
        assert_cpu_and_gpu_agree(true);
    }
}
