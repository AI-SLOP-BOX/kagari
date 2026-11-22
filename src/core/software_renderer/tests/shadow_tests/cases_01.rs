    #[test]
    fn test_shadow_falls_away_from_light() {
        let comp = shadow_test_comp(true);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Projection of caster center through L(16,16,400) onto z=0:
        // t=400/300 → (16+1.333*20, ...) ≈ (42.7, 42.7)
        let in_shadow = ((46 * 64 + 44) * 4) as usize;
        let lit_corner = ((8 * 64 + 8) * 4) as usize;
        assert!(
            px[in_shadow] < 200,
            "shadow region darkened, R={}",
            px[in_shadow]
        );
        assert!(
            px[lit_corner] >= 240,
            "far corner stays lit, R={}",
            px[lit_corner]
        );
    }
    #[test]
    fn test_no_shadow_when_caster_disabled() {
        let comp = shadow_test_comp(false);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let probe = ((46 * 64 + 44) * 4) as usize;
        assert!(px[probe] >= 240, "no caster -> no shadow, R={}", px[probe]);
    }
    #[test]
    fn shadow_map_keeps_caster_transform_on_composition_time() {
        let mut comp = shadow_test_comp(true);
        let caster = comp.layers.get_mut(1).expect("caster layer");
        caster.transform_3d.position = Animatable::new_animated(vec![
            Keyframe::new(0, [36.0, 36.0, 100.0], InterpolationType::Linear),
            Keyframe::new(20, [48.0, 48.0, 100.0], InterpolationType::Linear),
        ]);
        caster.freeze_at(20);

        let map = build_shadow_map(&comp, 0, 64, 64);
        let sum_near = |cx: usize, cy: usize| {
            let mut sum = 0.0;
            for y in cy.saturating_sub(2)..=(cy + 2).min(63) {
                for x in cx.saturating_sub(2)..=(cx + 2).min(63) {
                    sum += map[y * 64 + x];
                }
            }
            sum
        };

        // Time remapping freezes the caster's source content, not its layer
        // transform. The composition-time position [36,36] projects near
        // [43,43], while the source-frame position [48,48] is near [59,59].
        assert!(
            sum_near(43, 43) > sum_near(59, 59) * 1.5,
            "shadow must follow composition-time caster transform"
        );
    }

    #[test]
    fn shadow_map_uses_3d_caster_transform_not_2d_layer_transform() {
        let mut comp = shadow_test_comp(true);
        let caster = comp.layers.get_mut(1).expect("caster layer");
        // Keep the 2D transform far away and make the actual 3D transform
        // smaller, rotated, and displaced to a separate shadow location.
        caster.transform.position = Animatable::new_constant([8.0, 8.0]);
        caster.transform.scale = Animatable::new_constant([20.0, 20.0]);
        caster.transform_3d.position = Animatable::new_constant([44.0, 30.0, 100.0]);
        caster.transform_3d.rotation = Animatable::new_constant([0.0, 0.0, 20.0]);
        caster.transform_3d.scale = Animatable::new_constant([20.0, 20.0, 100.0]);

        let map = build_shadow_map(&comp, 0, 64, 64);
        let sum_near = |cx: usize, cy: usize| {
            let mut sum = 0.0;
            for y in cy.saturating_sub(2)..=(cy + 2).min(63) {
                for x in cx.saturating_sub(2)..=(cx + 2).min(63) {
                    sum += map[y * 64 + x];
                }
            }
            sum
        };
        // The 3D center projects from [44,30,100] through light [16,16,400]
        // to approximately [53,35]. The stale 2D transform would place it
        // near [5,5] and use the wrong size/orientation.
        assert!(
            sum_near(53, 35) > sum_near(5, 5) * 2.0,
            "shadow should follow the evaluated 3D transform"
        );
    }

    #[test]
    fn spot_light_shadow_respects_its_cone() {
        let mut comp = Composition::new("spot".into(), "Spot shadow".into(), 128, 64, 30, 30);
        let mut caster = Layer::new(
            "card".into(),
            "Card".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        caster.is_3d = true;
        caster.material.cast_shadows = true;
        caster.transform_3d.position = Animatable::new_constant([40.0, 32.0, 100.0]);
        caster.transform_3d.scale = Animatable::new_constant([6.0, 6.0, 100.0]);
        comp.layers.push(caster);
        comp.lights = vec![crate::core::timeline::Light3D {
            id: "spot".into(),
            name: "Spot".into(),
            light_type: crate::core::timeline::LightType::Spot {
                cone_angle_deg: 20.0,
                cone_feather_pct: 0.0,
            },
            color: [1.0; 4],
            intensity: 100.0,
            intensity_animation: None,
            position: Animatable::new_constant([32.0, 32.0, 300.0]),
            casts_shadows: true,
            shadow_darkness: 100.0,
            falloff: 0.0,
            max_radius: 0.0,
        }];

        // This caster projects near [44,32], inside the 20-degree cone.
        let inside = build_shadow_map(&comp, 0, 128, 64);
        assert!(inside[32 * 128 + 44] > 0.2, "in-cone shadow should appear");

        // Moving it to x=74 projects the complete card beyond the cone edge.
        comp.layers[0].transform_3d.position =
            Animatable::new_constant([74.0, 32.0, 100.0]);
        let outside = build_shadow_map(&comp, 0, 128, 64);
        let mut outside_max = 0.0f32;
        for y in 28..=36 {
            for x in 90..=100 {
                outside_max = outside_max.max(outside[y * 128 + x]);
            }
        }
        assert!(outside_max < 0.001, "out-of-cone shadow leaked: {outside_max}");
    }

    #[test]
    fn overlapping_shadow_casters_accumulate_density() {
        let single = shadow_test_comp(true);
        let single_map = build_shadow_map(&single, 0, 64, 64);

        let mut overlapping = single.clone();
        let second_caster = overlapping.layers[1].clone();
        overlapping.layers.push(second_caster);
        let overlapping_map = build_shadow_map(&overlapping, 0, 64, 64);

        let center = 43 * 64 + 43;
        assert!(
            overlapping_map[center] > single_map[center] + 0.1,
            "overlapping casters should deepen the shared shadow: one={}, two={}",
            single_map[center],
            overlapping_map[center]
        );
    }

    #[test]
    fn parallel_light_shadow_uses_parallel_rays_not_light_position() {
        let mut comp = shadow_test_comp(true);
        comp.layers[1].transform_3d.position =
            Animatable::new_constant([16.0, 16.0, 10.0]);
        comp.layers[1].transform_3d.scale =
            Animatable::new_constant([20.0, 20.0, 100.0]);
        comp.lights[0].light_type = crate::core::timeline::LightType::Parallel;
        // A directional light's location must not steer its rays or attenuate
        // the shadow; the renderer defines its fixed ray direction as +X,+Y,-Z.
        comp.lights[0].position = Animatable::new_constant([-1000.0, -1000.0, 100.0]);
        comp.lights[0].falloff = 2.0;
        comp.lights[0].max_radius = 1.0;

        let map = build_shadow_map(&comp, 0, 64, 64);
        assert!(
            map[26 * 64 + 26] > 0.2,
            "parallel shadow should shift by caster height along the fixed ray"
        );
    }

    #[test]
    fn parallel_layer_lighting_ignores_light_position() {
        let mut comp = Composition::new("parallel".into(), "Parallel lighting".into(), 64, 64, 30, 1);
        comp.lights[0].light_type = crate::core::timeline::LightType::Parallel;
        comp.lights[0].intensity = 100.0;
        comp.lights[0].position = Animatable::new_constant([-1000.0, -1000.0, -800.0]);
        let layer = Layer::new(
            "solid".into(),
            "Solid".into(),
            LayerType::Solid { color: [1.0; 4] },
            1,
        );
        let first = crate::core::software_renderer::layer_3d_shading(&comp, &layer, 0, 32.0, 32.0);

        comp.lights[0].position = Animatable::new_constant([1000.0, 1000.0, 800.0]);
        let second = crate::core::software_renderer::layer_3d_shading(&comp, &layer, 0, 32.0, 32.0);
        assert_eq!(first, second);
        assert!(first[0] > layer.material.ambient);
    }

    #[test]
    fn ambient_layer_lighting_ignores_position_and_uses_light_color() {
        let mut comp = Composition::new("ambient".into(), "Ambient lighting".into(), 64, 64, 30, 1);
        comp.lights[0].light_type = crate::core::timeline::LightType::Ambient;
        comp.lights[0].color = [1.0, 0.0, 0.0, 1.0];
        comp.lights[0].intensity = 50.0;
        comp.lights[0].position = Animatable::new_constant([-800.0, 1000.0, -500.0]);
        let layer = Layer::new(
            "solid".into(),
            "Solid".into(),
            LayerType::Solid { color: [1.0; 4] },
            1,
        );
        let first = crate::core::software_renderer::layer_3d_shading(&comp, &layer, 0, 32.0, 32.0);

        comp.lights[0].position = Animatable::new_constant([800.0, -1000.0, 500.0]);
        let second = crate::core::software_renderer::layer_3d_shading(&comp, &layer, 0, 32.0, 32.0);
        assert_eq!(first, second);
        assert!(first[0] > first[1], "ambient light color must tint the layer");
    }

    #[test]
    fn layer_metalness_reduces_diffuse_response() {
        let mut comp = Composition::new("metal-layer".into(), "Metal layer".into(), 64, 64, 30, 1);
        comp.lights[0].light_type = crate::core::timeline::LightType::Parallel;
        comp.lights[0].color = [1.0; 4];
        let mut layer = Layer::new(
            "solid".into(),
            "Solid".into(),
            LayerType::Solid { color: [1.0; 4] },
            1,
        );
        layer.material.ambient = 0.0;
        layer.material.diffuse = 1.0;
        layer.material.specular = 0.0;
        let dielectric = crate::core::software_renderer::layer_3d_shading(
            &comp, &layer, 0, 32.0, 32.0,
        );
        layer.material.metalness = 1.0;
        let metallic = crate::core::software_renderer::layer_3d_shading(
            &comp, &layer, 0, 32.0, 32.0,
        );
        assert!(dielectric[0] > metallic[0]);
    }

    #[test]
    fn depth_sort_keeps_layer_transform_on_composition_time() {
        let mut comp = Composition::new("depth-remap".into(), "Depth remap".into(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "depth".into(),
            "Animated depth".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        layer.is_3d = true;
        layer.transform_3d.position = Animatable::new_animated(vec![
            Keyframe::new(0, [32.0, 32.0, 10.0], InterpolationType::Linear),
            Keyframe::new(20, [32.0, 32.0, 90.0], InterpolationType::Linear),
        ]);
        layer.freeze_at(20);
        comp.layers.push(layer);

        assert_eq!(layer_depth_for_sort(&comp, &comp.layers[0], 0), 10.0);
    }
    #[test]
    fn test_collapse_transformations_3d_z_continuity() {
        // Draw-order rule (renderer sorts far-first): SMALLER effective z is
        // nearer and paints last. Collapsed child lifts to pz+cz=200, nearer
        // than green 350 -> child wins. Uncollapsed card sits at pz=400,
        // farther than green -> green wins. Identical stack, opposite winner.
        let build = |collapsed: bool| {
            let mut comp = Composition::new("cz".into(), "Collapse3D".into(), 64, 64, 30, 30);
            let mut green = Layer::new(
                "green".into(),
                "G".into(),
                LayerType::Solid {
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                30,
            );
            green.is_3d = true;
            green.transform.scale = Animatable::new_constant([100.0, 100.0]);
            green.transform.position = Animatable::new_constant([32.0, 32.0]);
            green.transform_3d.position = Animatable::new_constant([32.0, 32.0, 350.0]);
            comp.layers.push(green);

            let mut sub = Composition::new("subz".into(), "S".into(), 64, 64, 30, 30);
            let mut blue_child = Layer::new(
                "bc".into(),
                "B".into(),
                LayerType::Solid {
                    color: [0.0, 0.0, 1.0, 1.0],
                },
                30,
            );
            blue_child.is_3d = true;
            blue_child.transform.scale = Animatable::new_constant([100.0, 100.0]);
            blue_child.transform.position = Animatable::new_constant([32.0, 32.0]);
            blue_child.transform_3d.position = Animatable::new_constant([32.0, 32.0, -200.0]);
            sub.layers.push(blue_child);
            comp.sub_compositions.push(sub);

            let mut pc = Layer::new(
                "pc".into(),
                "P".into(),
                LayerType::PreComp {
                    comp_id: "subz".into(),
                },
                30,
            );
            pc.is_collapsed = collapsed;
            pc.is_3d = true;
            pc.transform.scale = Animatable::new_constant([100.0, 100.0]);
            pc.transform.position = Animatable::new_constant([32.0, 32.0]);
            pc.transform_3d.position = Animatable::new_constant([32.0, 32.0, 400.0]);
            comp.layers.push(pc);
            comp
        };

        let px_on = render_frame_to_pixels(&build(true), 0, 64, 64, 0.0, 0);
        let i = ((16 * 64 + 16) * 4) as usize;
        assert!(
            px_on[i + 2] > px_on[i + 1],
            "collapsed child (nearer) beats green: {:?}",
            &px_on[i..i + 3]
        );

        // Structural check: flattening composes parent z + child z and maps
        // the child into parent space around the sub-comp center.
        let flat = flatten_collapsed(&build(true), 0);
        let red_child = flat
            .layers
            .iter()
            .find(|l| l.name == "B")
            .expect("expanded child");
        assert!(red_child.is_3d);
        let lifted = red_child.transform_3d.position.evaluate(0);
        assert!(
            (lifted[2] - 200.0).abs() < 0.01,
            "pz+cz lift: {}",
            lifted[2]
        );
        assert!(
            (lifted[0] - 32.0).abs() < 0.01 && (lifted[1] - 32.0).abs() < 0.01,
            "child mapped around parent center: {:?}",
            lifted
        );
    }
    #[test]
    fn collapsed_precomp_uses_parent_time_remap_for_child_frame() {
        let mut comp = Composition::new(
            "collapse-remap".into(),
            "Collapse remap".into(),
            64,
            64,
            30,
            30,
        );
        let mut sub = Composition::new("sub-remap".into(), "Sub".into(), 64, 64, 30, 30);
        let mut child = Layer::new(
            "child".into(),
            "Animated child".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        // The child only exists at source frame 20. A collapsed parent frozen
        // at frame 20 must still expand it while rendering composition frame 0.
        child.in_frame = 20;
        child.out_frame = 30;
        child.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [8.0, 32.0], InterpolationType::Linear),
            Keyframe::new(20, [48.0, 32.0], InterpolationType::Linear),
        ]);
        sub.layers.push(child);
        comp.sub_compositions.push(sub);

        let mut precomp = Layer::new(
            "precomp".into(),
            "Frozen collapsed".into(),
            LayerType::PreComp {
                comp_id: "sub-remap".into(),
            },
            30,
        );
        precomp.is_collapsed = true;
        precomp.transform.position = Animatable::new_constant([32.0, 32.0]);
        precomp.freeze_at(20);
        comp.layers.push(precomp);

        let flat = flatten_collapsed(&comp, 0);
        let expanded = flat
            .layers
            .iter()
            .find(|layer| layer.id == "child")
            .expect("child active at the remapped source frame");
        let position = expanded.transform.position.evaluate(0);
        assert_eq!(position, [48.0, 32.0]);
    }
    #[test]
    fn nested_collapsed_precomp_inherits_parent_source_frame() {
        let mut inner = Composition::new("inner-remap".into(), "Inner".into(), 64, 64, 30, 30);
        let mut child = Layer::new(
            "nested-child".into(),
            "Nested child".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        child.in_frame = 20;
        child.out_frame = 30;
        inner.layers.push(child);

        let mut middle = Composition::new("middle-remap".into(), "Middle".into(), 64, 64, 30, 30);
        middle.sub_compositions.push(inner);
        let mut nested = Layer::new(
            "nested-precomp".into(),
            "Nested collapsed".into(),
            LayerType::PreComp {
                comp_id: "inner-remap".into(),
            },
            30,
        );
        nested.is_collapsed = true;
        middle.layers.push(nested);

        let mut root = Composition::new("root-remap".into(), "Root".into(), 64, 64, 30, 30);
        root.sub_compositions.push(middle);
        let mut outer = Layer::new(
            "outer-precomp".into(),
            "Outer collapsed".into(),
            LayerType::PreComp {
                comp_id: "middle-remap".into(),
            },
            30,
        );
        outer.is_collapsed = true;
        outer.freeze_at(20);
        root.layers.push(outer);

        let flat = flatten_collapsed(&root, 0);
        assert!(
            flat.layers.iter().any(|layer| layer.id == "nested-child"),
            "nested collapsed content must be evaluated at the outer mapped frame"
        );
    }
    #[test]
    fn test_render_frame_to_pixels_f32_returns_linear() {
        let mut comp = Composition::new("f32t".into(), "F32".into(), 8, 8, 30, 30);
        comp.background_color = [1.0, 1.0, 1.0, 1.0];
        let f32_px = render_frame_to_pixels_f32(&comp, 0, 8, 8, 0.0, 0);
        assert_eq!(f32_px.len(), 64);
        // White bg -> linear sRGB decode of 1.0 should be 1.0
        for p in &f32_px {
            assert!(p[0] > 0.99, "expected ~1.0 linear, got {}", p[0]);
            assert!(p[3] > 0.99, "expected opaque alpha, got {}", p[3]);
        }
        // Empty render returns empty
        let empty = render_frame_to_pixels_f32(&comp, 0, 0, 0, 0.0, 0);
        assert!(empty.is_empty());
    }
    #[test]
    fn test_render_frame_to_pixels_f32_exposure_boosts() {
        let mut comp = Composition::new("f32exp".into(), "F32E".into(), 4, 4, 30, 30);
        comp.background_color = [0.5, 0.5, 0.5, 1.0];
        let no_ev = render_frame_to_pixels_f32(&comp, 0, 4, 4, 0.0, 0);
        let plus2 = render_frame_to_pixels_f32(&comp, 0, 4, 4, 2.0, 0);
        // +2 EV boosts values; with sRGB encode/decode + dithering
        // the ratio isn't exactly 4x, but must be significantly > 1.
        assert!(
            plus2[0][0] > no_ev[0][0] * 2.0,
            "expected >2x, got {}",
            plus2[0][0] / no_ev[0][0]
        );
        // All values in valid range
        for p in &plus2 {
            for c in p {
                assert!(*c >= 0.0 && !c.is_nan());
            }
        }
    }
    #[test]
    fn test_lut_mode_2_aces_pipeline_matches_reference() {
        let mut comp = Composition::new("aces".into(), "A".into(), 16, 16, 30, 30);
        let mut s = Layer::new(
            "s".into(),
            "S".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        s.transform.scale = Animatable::new_constant([100.0, 100.0]);
        s.transform.position = Animatable::new_constant([8.0, 8.0]);
        s.transform.anchor_point = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(s);

        let px = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 2);
        let i = ((4 * 16 + 4) * 4) as usize;

        // Reference: white → decode(1)=1 → tonemap ≈0.8019776 → encode
        let expected = crate::core::aces::aces_preview_transform([1.0, 1.0, 1.0]);
        let want = (expected[0] * 255.0).round().clamp(0.0, 255.0) as u8;
        assert!(
            (px[i] as i32 - want as i32).abs() <= 1,
            "ACES preview white: got {} want {}",
            px[i],
            want
        );
        // And it must differ from plain passthrough (255)
        assert!(px[i] < 250, "tonemap must not be identity on white");
    }

    fn adjustment_test_comp(opacity: f32) -> Composition {
        let mut comp = Composition::new("adj".into(), "Adj".into(), 32, 32, 30, 30);
        // Bottom: pure white solid covering the frame
        let mut base = Layer::new(
            "b1".into(),
            "Base".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        base.transform.scale = Animatable::new_constant([100.0, 100.0]);
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        base.transform.anchor_point = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(base);
        // Adjustment layer with Invert at the given opacity
        let mut adj = Layer::new("a1".into(), "Adjust".into(), LayerType::AdjustmentLayer, 30);
        adj.effects.push(crate::core::timeline::Effect {
            id: "fx_inv".into(),
            enabled: true,
            name: "Invert".into(),
            effect_type: crate::core::timeline::EffectType::Invert {
                invert_alpha: false,
            },
        });
        adj.transform.opacity = Animatable::new_constant(opacity);
        comp.layers.push(adj);
        comp
    }
    #[test]
    fn test_adjustment_layer_inverts_below_at_full_opacity() {
        let comp = adjustment_test_comp(100.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(px[i] < 20, "white inverted to near-black, R={}", px[i]);
    }
    #[test]
    fn test_layer_effects_toggle_bypasses_adjustment_stack() {
        let mut comp = adjustment_test_comp(100.0);
        comp.layers[1].effects_enabled = false;
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(
            px[i] > 235,
            "disabled adjustment effects must preserve white, R={}",
            px[i]
        );
    }
    #[test]
    fn test_adjustment_layer_opacity_blends() {
        let comp = adjustment_test_comp(50.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(
            px[i] > 100 && px[i] < 160,
            "50% invert of white ~127, R={}",
            px[i]
        );
    }
