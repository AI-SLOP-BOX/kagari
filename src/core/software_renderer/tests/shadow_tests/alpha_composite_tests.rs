    use super::*;
    use crate::core::timeline::Effect;

    fn black_bg(w: u32, h: u32) -> Composition {
        let mut comp = Composition::new("t".into(), "AlphaTest".into(), w, h, 30, 30);
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

    fn gray_solid(id: &str, opacity: f32) -> Layer {
        let mut l = Layer::new(
            id.into(),
            id.into(),
            LayerType::Solid {
                color: [0.5, 0.5, 0.5, 1.0],
            },
            30,
        );
        l.transform.position = Animatable::new_constant([32.0, 32.0]);
        l.transform.opacity = Animatable::new_constant(opacity);
        l
    }

    fn mean_r(px: &[u8]) -> f32 {
        let n = px.len() / 4;
        px.as_chunks::<4>()
            .0
            .iter()
            .map(|c| c[0] as f32)
            .sum::<f32>()
            / n.max(1) as f32
    }

    #[test]
    fn generator_with_opacity_fade_is_proportional() {
        // FractalNoise used to force alpha=255, so a 50% layer rendered
        // at full strength. Now the fade must (about) halve the energy.
        let mk = |opacity: f32| {
            let mut comp = black_bg(64, 64);
            let mut l = gray_solid("n", opacity);
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.effects.push(Effect {
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
            comp.layers.push(l);
            render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
        };
        let full = mean_r(&mk(100.0));
        let half = mean_r(&mk(50.0));
        assert!(full > 20.0, "noise must lift the frame, got {full}");
        let ratio = half / full.max(1.0);
        assert!(
            (0.3..0.7).contains(&ratio),
            "50% fade should roughly halve noise energy, ratio={ratio} (half={half}, full={full})"
        );
    }

    #[test]
    fn sparse_painter_with_opacity_fade_is_proportional() {
        // StarField paints sparse bright pixels with full alpha; the
        // layer fade must still dim them via composite-time opacity.
        let mk = |opacity: f32| {
            let mut comp = black_bg(64, 64);
            let mut l = Layer::new(
                "s".into(),
                "Stars".into(),
                LayerType::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                30,
            );
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.transform.opacity = Animatable::new_constant(opacity);
            l.effects.push(Effect {
                id: "sf".into(),
                name: "Star Field".into(),
                effect_type: crate::core::timeline::EffectType::StarField {
                    num_stars: Animatable::new_constant(60.0),
                    depth_speed: Animatable::new_constant(0.0),
                },
                enabled: true,
            });
            comp.layers.push(l);
            render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
        };
        let full = mean_r(&mk(100.0));
        let half = mean_r(&mk(50.0));
        assert!(full > 1.0, "stars must lift the frame, got {full}");
        let ratio = half / full.max(0.001);
        assert!(
            (0.3..0.7).contains(&ratio),
            "50% fade should roughly halve star energy, ratio={ratio}"
        );
    }

    #[test]
    fn effect_stack_then_opacity_matches_manual_blend() {
        // Gray solid + noise, faded to 50%: must equal a 50/50 mix of the
        // full-strength result with black (linearity of composite-time op).
        let mk = |opacity: f32| {
            let mut comp = black_bg(64, 64);
            let mut l = gray_solid("n", opacity);
            l.transform.position = Animatable::new_constant([32.0, 32.0]);
            l.effects.push(Effect {
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
            comp.layers.push(l);
            render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0)
        };
        let full = mk(100.0);
        let half = mk(50.0);
        let mut max_dev = 0.0f32;
        for (i, px) in half.as_chunks::<4>().0.iter().enumerate() {
            let expect = full[i * 4] as f32 * 0.5;
            max_dev = max_dev.max((px[0] as f32 - expect).abs());
        }
        assert!(
            max_dev <= 3.0,
            "faded stack must match manual 50% mix, max_dev={max_dev}"
        );
    }

    #[test]
    fn add_blend_with_partial_opacity() {
        // White solid, Add over black at 50% -> ~127 gray everywhere.
        let mut comp = black_bg(32, 32);
        let mut l = Layer::new(
            "w".into(),
            "White".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        l.transform.position = Animatable::new_constant([16.0, 16.0]);
        l.transform.opacity = Animatable::new_constant(50.0);
        l.blend_mode = crate::core::timeline::BlendMode::Add;
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let m = mean_r(&px);
        assert!(
            (115.0..140.0).contains(&m),
            "Add@50% of white over black should be ~127, got {m}"
        );
    }

    #[test]
    fn mask_and_opacity_combine() {
        // Left-half rect mask + 50% opacity: kept side fades, cut side
        // stays at background black.
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
        l.masks.push(crate::core::mask::Mask::new_rect(
            "m".into(),
            "Left".into(),
            0.0,
            0.0,
            32.0,
            64.0,
        ));
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let at = |x: u32, y: u32| px[((y * 64 + x) * 4) as usize] as f32;
        assert!(
            (110.0..145.0).contains(&at(16, 32)),
            "masked-in side should fade to ~127, got {}",
            at(16, 32)
        );
        assert!(
            at(48, 32) < 8.0,
            "masked-out side must stay black, got {}",
            at(48, 32)
        );
    }

    #[test]
    fn zero_opacity_layer_is_invisible() {
        // Any layer at 0% must contribute nothing (no leaks, no residue
        // from alpha-writing effects).
        let mut comp = black_bg(32, 32);
        let mut l = gray_solid("n", 0.0);
        l.transform.position = Animatable::new_constant([16.0, 16.0]);
        l.effects.push(Effect {
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
        comp.layers.push(l);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert!(
            mean_r(&px) < 2.0,
            "0% layer must be invisible, mean={}",
            mean_r(&px)
        );
    }
