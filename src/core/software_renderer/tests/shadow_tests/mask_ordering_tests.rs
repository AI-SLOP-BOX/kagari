    use super::*;
    use crate::core::timeline::Effect;

    fn black_bg(w: u32, h: u32) -> Composition {
        let mut comp = Composition::new("t".into(), "MaskTest".into(), w, h, 30, 30);
        comp.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut bg = Layer::new(
            "bg".into(),
            "BG".into(),
            LayerType::Solid {
                color: [0.0, 0.0, 0.0, 1.0],
            },
            30,
        );
        bg.transform.position = Animatable::new_constant([w as f32 * 0.5, h as f32 * 0.5]);
        comp.layers.push(bg);
        comp
    }

    fn rect_mask(
        id: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        feather: f32,
    ) -> crate::core::mask::Mask {
        let mut m = crate::core::mask::Mask::new_rect(id.into(), id.into(), x, y, w, h);
        m.feather = Animatable::new_constant(feather);
        m
    }

    fn at(px: &[u8], w: u32, x: u32, y: u32) -> f32 {
        px[((y * w + x) * 4) as usize] as f32
    }

    #[test]
    fn blurred_masked_shape_does_not_leak() {
        // White 32x32 rect + identical rect mask + blur 6: the blur would
        // smear ~18px past the mask without the post-effect re-mask.
        let mut comp = black_bg(64, 64);
        let mut l = Layer::new(
            "r".into(),
            "Rect".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Rectangle {
                    width: Animatable::new_constant(100.0),
                    height: Animatable::new_constant(100.0),
                    corner_radius: Animatable::new_constant(0.0),
                },
                color: [1.0, 1.0, 1.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        l.transform.position = Animatable::new_constant([32.0, 32.0]);
        l.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
        l.effects.push(Effect {
            id: "blur".into(),
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(12.0),
            },
            enabled: true,
        });
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        assert!(
            at(&px, 64, 32, 32) > 150.0,
            "masked interior must stay bright, got {}",
            at(&px, 64, 32, 32)
        );
        // 4px outside the mask edge: blur spill would read ~40+ here
        // without the post-effect re-mask.
        assert!(
            at(&px, 64, 12, 32) < 14.0,
            "blur must not leak outside final mask, got {}",
            at(&px, 64, 12, 32)
        );
    }

    #[test]
    fn glow_on_masked_layer_respects_final_mask() {
        // Blur spreads alpha past the mask, then glow blooms the spill:
        // without the post-effect re-mask the halo would show outside.
        // (Pure glow alone cannot leak: RGB-only energy is gated by zero
        // alpha at composite. The stack is the meaningful case.)
        let mut comp = black_bg(64, 64);
        let mut l = Layer::new(
            "w".into(),
            "White".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        l.transform.position = Animatable::new_constant([32.0, 32.0]);
        l.masks.push(rect_mask("m", 0.0, 0.0, 32.0, 64.0, 0.0));
        l.effects.push(Effect {
            id: "blur".into(),
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(6.0),
            },
            enabled: true,
        });
        l.effects.push(Effect {
            id: "glow".into(),
            name: "Glow".into(),
            effect_type: crate::core::timeline::EffectType::Glow {
                threshold: Animatable::new_constant(30.0),
                radius: Animatable::new_constant(12.0),
                intensity: Animatable::new_constant(200.0),
                color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
            enabled: true,
        });
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        assert!(
            at(&px, 64, 16, 32) > 150.0,
            "masked-in side must stay bright, got {}",
            at(&px, 64, 16, 32)
        );
        // 4px past the mask edge: blur+glow halo would read bright here
        // without the post-effect re-mask.
        assert!(
            at(&px, 64, 36, 32) < 15.0,
            "glow must not leak past final mask, got {}",
            at(&px, 64, 36, 32)
        );
    }

    #[test]
    fn displacement_on_masked_layer_stays_contained() {
        // Masked white solid shifted +12px by Offset: without containment
        // the shifted content would appear past the final mask edge.
        // (TurbulentDisplace is covered by the same remask path; Offset
        // is used here because its placement is exactly deterministic.)
        let mut comp = black_bg(64, 64);
        let mut l = Layer::new(
            "w".into(),
            "White".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        l.transform.position = Animatable::new_constant([32.0, 32.0]);
        l.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
        l.effects.push(Effect {
            id: "off".into(),
            name: "Offset".into(),
            effect_type: crate::core::timeline::EffectType::Offset {
                shift_x: Animatable::new_constant(12.0),
                shift_y: Animatable::new_constant(0.0),
            },
            enabled: true,
        });
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Mask spans x16..48; content shifted +12 would reach x60.
        assert!(
            at(&px, 64, 52, 32) < 12.0,
            "displaced content must not appear outside mask, got {}",
            at(&px, 64, 52, 32)
        );
        assert!(
            at(&px, 64, 56, 32) < 12.0,
            "displaced content must not appear outside mask, got {}",
            at(&px, 64, 56, 32)
        );
        assert!(
            at(&px, 64, 32, 32) > 150.0,
            "masked interior must stay bright, got {}",
            at(&px, 64, 32, 32)
        );
    }

    #[test]
    fn feathered_mask_with_opacity() {
        // Feathered rect + 50% opacity: soft edge survives the remask
        // (squared falloff stays smooth), interior fades, exterior black.
        let mut comp = black_bg(64, 64);
        let mut l = Layer::new(
            "w".into(),
            "White".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        l.transform.position = Animatable::new_constant([32.0, 32.0]);
        l.transform.opacity = Animatable::new_constant(50.0);
        l.masks.push(rect_mask("m", 0.0, 0.0, 32.0, 64.0, 8.0));
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let inside = at(&px, 64, 8, 32);
        // Sample inside the feather band (rect edge x=32, feather 8):
        // geometric edge itself is ~0 by definition.
        let edge = at(&px, 64, 28, 32);
        let outside = at(&px, 64, 56, 32);
        assert!(
            (100.0..150.0).contains(&inside),
            "interior should fade to ~127, got {inside}"
        );
        assert!(
            edge > 15.0 && edge < 240.0,
            "feather edge must be a soft ramp, got {edge}"
        );
        assert!(outside < 8.0, "exterior must stay black, got {outside}");
    }

    #[test]
    fn precomp_mask_with_inner_effects() {
        // Mask on the precomp layer clips the sub-render (which runs its
        // own inner effects); inner noise must show inside, never outside.
        let mut sub = Composition::new("sub".into(), "Sub".into(), 64, 64, 30, 30);
        sub.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut inner = Layer::new(
            "g".into(),
            "Gray".into(),
            LayerType::Solid {
                color: [0.6, 0.6, 0.6, 1.0],
            },
            30,
        );
        inner.transform.position = Animatable::new_constant([32.0, 32.0]);
        inner.effects.push(Effect {
            id: "fbm".into(),
            name: "Fractal Noise".into(),
            effect_type: crate::core::timeline::EffectType::FractalNoise {
                fractal_type: Animatable::new_constant(0.0),
                contrast: Animatable::new_constant(0.6),
                brightness: Animatable::new_constant(0.5),
                complexity: Animatable::new_constant(2.0),
                evolution: Animatable::new_constant(0.0),
            },
            enabled: true,
        });
        sub.layers.push(inner);

        let mut comp = black_bg(64, 64);
        let mut pre = Layer::new(
            "p".into(),
            "Pre".into(),
            LayerType::PreComp {
                comp_id: "sub".into(),
            },
            30,
        );
        pre.transform.position = Animatable::new_constant([32.0, 32.0]);
        pre.masks.push(rect_mask("m", 16.0, 16.0, 32.0, 32.0, 0.0));
        comp.layers.push(pre);
        comp.sub_compositions.push(sub);

        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        assert!(
            at(&px, 64, 4, 4) < 8.0,
            "masked-out side must stay black, got {}",
            at(&px, 64, 4, 4)
        );
        // Inner noise must survive inside the mask (variance check).
        let mut vals = Vec::new();
        for y in 24..40 {
            for x in 24..40 {
                vals.push(at(&px, 64, x, y));
            }
        }
        let mean = vals.iter().sum::<f32>() / vals.len() as f32;
        let var = vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / vals.len() as f32;
        assert!(
            mean > 10.0 && var > 5.0,
            "inner effect result must show inside mask (mean={mean}, var={var})"
        );
    }
