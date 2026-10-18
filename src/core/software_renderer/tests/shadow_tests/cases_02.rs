    #[test]
    fn test_ellipse_caster_shadow_is_round_not_square() {
        // Ellipse caster: the four bbox corners of its bounding quad must stay
        // LIT (round shape doesn't reach them), while the center is darkened.
        let mut comp = Composition::new("e".into(), "EllipseShadow".into(), 96, 96, 30, 30);
        let mut recv = Layer::new(
            "r1".into(),
            "Floor".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
        recv.transform.position = Animatable::new_constant([48.0, 48.0]);
        comp.layers.push(recv);

        let mut caster = Layer::new(
            "c1".into(),
            "Disc".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Ellipse {
                    width: Animatable::new_constant(40.0),
                    height: Animatable::new_constant(40.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        caster.is_3d = true;
        caster.material.cast_shadows = true;
        caster.transform.scale = Animatable::new_constant([100.0, 100.0]);
        caster.transform.position = Animatable::new_constant([60.0, 60.0]);
        caster.transform_3d.position = Animatable::new_constant([60.0, 60.0, 100.0]);
        comp.layers.push(caster);
        comp.lights = vec![crate::core::timeline::Light3D {
            id: "k".into(),
            name: "K".into(),
            light_type: crate::core::timeline::LightType::Point,
            color: [1.0; 4],
            intensity: 100.0,
            position: Animatable::new_constant([48.0, 48.0, 300.0]),
            casts_shadows: true,
            shadow_darkness: 90.0,
            falloff: 1.0,
            max_radius: 0.0,
        }];

        let px = render_frame_to_pixels(&comp, 0, 96, 96, 0.0, 0);
        // Projection center ≈ light + t*(caster-light), t=300/200=1.5 →
        // (48+1.5*12, same) = (66,66); ellipse radius scales to ~30px.
        let in_shadow = ((70 * 96 + 68) * 4) as usize;
        // Bounding-quad corner of the projected ellipse (~±30 from center):
        // a square fallback would darken (92,92); the round shape must not.
        let quad_corner = ((90 * 96 + 90) * 4) as usize;
        assert!(
            px[in_shadow] < 210,
            "ellipse shadow core darkened, R={}",
            px[in_shadow]
        );
        assert!(
            px[quad_corner] > 235,
            "round shadow must spare quad corner, R={}",
            px[quad_corner]
        );
    }
    #[test]
    fn test_preserve_transparency_blends_only_onto_opaque_dest() {
        let mut comp = Composition::new("pt".into(), "PTComp".into(), 32, 32, 30, 30);
        // Base layer: small 16x16 white square in the center (16..32, 16..32), background transparent
        comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut base = Layer::new(
            "b".into(),
            "Base".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        base.transform.scale = Animatable::new_constant([50.0, 50.0]); // 16x16
        comp.layers.push(base);

        // Top layer: red solid covering entire 32x32 screen, but with preserve_transparency = true
        let mut top = Layer::new(
            "t".into(),
            "TopRed".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        top.transform.position = Animatable::new_constant([16.0, 16.0]);
        top.transform.scale = Animatable::new_constant([100.0, 100.0]);
        top.preserve_transparency = true;
        comp.layers.push(top);

        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel (16, 16): should be painted Red (R=255, G=0, B=0, A=255)
        let c_idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(px[c_idx], 255);
        assert_eq!(px[c_idx + 1], 0);

        // Corner pixel (2, 2): should remain transparent (A=0), not painted red
        let corner_idx = ((2 * 32 + 2) * 4) as usize;
        assert_eq!(px[corner_idx + 3], 0);
    }
    #[test]
    fn test_shape_units_stay_circular_on_wide_comp() {
        // Regression: shape units must be uniform (width-referenced) so an
        // equal-width/height ellipse renders as a circle on 16:9 comps.
        let mut comp = Composition::new("c".into(), "Wide".into(), 192, 108, 30, 2);
        comp.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut layer = Layer::new(
            "s".into(),
            "Circle".into(),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(100.0),
                    height: Animatable::new_constant(100.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            2,
        );
        layer.transform.position = Animatable::new_constant([96.0, 54.0]);
        layer.transform.scale = Animatable::new_constant([100.0, 100.0]);
        comp.layers.push(layer);

        let px = render_frame_to_pixels(&comp, 0, 192, 108, 0.0, 0);
        let bright = |x: u32, y: u32| px[((y * 192 + x) * 4) as usize] > 128;
        // 100 units at scale 100 => 96px span on a 192-wide comp.
        let row = (50..=142u32).filter(|&x| bright(x, 54)).count();
        let col = (6..=102u32).filter(|&y| bright(96, y)).count();
        assert!(
            (88..=100).contains(&row),
            "circle width unexpected: {row}px",
        );
        assert!(
            (row as i32 - col as i32).abs() <= 4,
            "circle distorted on wide comp: w={row} h={col}",
        );
    }
    #[test]
    fn test_guide_layer_skipped_in_precomp() {
        let mut sub_comp = Composition::new("sub".into(), "Sub".into(), 32, 32, 30, 30);
        let mut guide = Layer::new(
            "g".into(),
            "Guide".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        guide.is_guide_layer = true;
        sub_comp.layers.push(guide);

        let mut main_comp = Composition::new("main".into(), "Main".into(), 32, 32, 30, 30);
        main_comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let precomp_layer = Layer::new(
            "p".into(),
            "Pre".into(),
            LayerType::PreComp {
                comp_id: "sub".into(),
            },
            30,
        );
        main_comp.layers.push(precomp_layer);
        main_comp.sub_compositions.push(sub_comp);

        let px = render_frame_to_pixels(&main_comp, 0, 32, 32, 0.0, 0);
        // Entire buffer should be empty / transparent since the only sub-layer was a guide layer
        assert!(px.iter().all(|&b| b == 0));
    }

    /// Composite-time opacity regression tests.
    ///
    /// Pipeline rule under test: layer opacity is applied ONCE during
    /// compositing (Phase 3), never baked into raster alpha where effect
    /// kernels could clobber it. These tests fail on the old bake-first
    /// pipeline (e.g. generator + fade rendered at full strength).
    #[cfg(test)]
    #[path = "alpha_composite_tests.rs"]
    mod alpha_composite_tests;

    /// Mask ordering regression tests.
    ///
    /// Model under test: pre-effect mask isolates (during raster), effects
    /// run, then a post-effect re-mask contains the result before compositing
    /// (Phase 2.8). Blur/glow/displacement must not leak outside the final
    /// masked region. This intentionally differs from AE's default, where
    /// effects spill past masks.
    #[cfg(test)]
    #[path = "mask_ordering_tests.rs"]
    mod mask_ordering_tests;
