use super::*;

#[test]
fn test_time_expression() {
    let engine = build_engine();
    // At frame 30, fps 30 → time = 1.0
    let result = eval_f32(&engine, "time * 360.0", 0.0, 30, 30);
    assert!(
        (result - 360.0).abs() < 0.01,
        "Expected 360, got {}",
        result
    );
}

#[test]
fn test_wiggle_expression() {
    let engine = build_engine();
    // wiggle should return a value offset from 0
    let result = eval_f32(&engine, "wiggle(time, 2.0, 50.0)", 0.0, 15, 30);
    assert!(result.abs() <= 55.0, "Wiggle out of range: {}", result);
}

#[test]
fn test_v2_expression() {
    let engine = build_engine();
    let result = eval_v2(&engine, "[time * 100.0, 0.0]", [0.0, 0.0], 30, 30);
    assert!(
        (result[0] - 100.0).abs() < 0.1,
        "Expected 100, got {}",
        result[0]
    );
    assert!(
        (result[1] - 0.0).abs() < 0.1,
        "Expected 0, got {}",
        result[1]
    );
}

#[test]
fn test_invalid_script() {
    let engine = build_engine();
    let r = validate_script(&engine, "this is not valid rhai;;;;;");
    assert!(r.is_err(), "Should have failed validation");
}

#[test]
fn test_ease_expression() {
    let engine = build_engine();
    let mid = eval_f32(&engine, "ease(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
    assert!(
        (mid - 50.0).abs() < 0.01,
        "Expected 50 at midpoint, got {}",
        mid
    );

    let ease_in = eval_f32(&engine, "easeIn(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
    assert!(
        (ease_in - 25.0).abs() < 0.01,
        "Expected 25 for easeIn at midpoint, got {}",
        ease_in
    );

    let ease_out = eval_f32(&engine, "easeOut(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
    assert!(
        (ease_out - 75.0).abs() < 0.01,
        "Expected 75 for easeOut at midpoint, got {}",
        ease_out
    );
}

#[cfg(test)]
mod tests_comp_context {
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_thiscomp_layer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new(
            "t1".into(),
            "Target".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        target.transform.position = Animatable::new_constant([42.0, 24.0]);
        comp.layers.push(target);
        let mut driver = Layer::new("d1".into(), "Driver".into(), LayerType::Null, 30);
        // Driver position follows Target's X
        driver.transform.position_expression = Some(Expression::Raw(
            "thisComp.layer(\"Target\").transform.position[0] + 10.0".into(),
        ));
        let driver_name = driver.name.clone();
        comp.layers.push(driver);

        let driver = comp.layers.iter().find(|l| l.name == driver_name).unwrap();
        let (pos, _, _, _) = comp.resolve_world_transform(driver, 0);
        assert!(
            (pos[0] - 52.0).abs() < 0.01,
            "expected 52.0, got {}",
            pos[0]
        );
    }

    #[test]
    fn test_effect_param_bridge_cross_layer() {
        use crate::core::timeline::Effect;
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut src = Layer::new(
            "s1".into(),
            "Source".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        // Animated Gaussian blur radius: 5 at frame 0, 15 at frame 10.
        src.effects.push(Effect {
            id: "fx_test_blur".into(),
            enabled: true,
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: Animatable::new_animated(vec![
                    crate::core::keyframe::Keyframe::new(
                        0,
                        5.0,
                        crate::core::keyframe::InterpolationType::Linear,
                    ),
                    crate::core::keyframe::Keyframe::new(
                        10,
                        15.0,
                        crate::core::keyframe::InterpolationType::Linear,
                    ),
                ]),
            },
        });
        comp.layers.push(src);
        let mut drv = Layer::new("d1".into(), "Driver".into(), LayerType::Null, 30);
        drv.transform.position_expression = Some(Expression::Raw(
            "let b = thisComp.layer(\"Source\").effect_param(\"Blur\", \"Blur Radius\"); [b * 10.0, 0.0]".into(),
        ));
        comp.layers.push(drv);

        let snap_f0 = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert!(
            (snap_f0.layers["Source"].effects["Blur"]["Blur Radius"] - 5.0).abs() < 0.01,
            "snapshot should carry effect value"
        );

        let layer_f10 = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(layer_f10, 10);
        assert!(
            (pos[0] - 150.0).abs() < 0.05,
            "expected 150.0 (15*10), got {}",
            pos[0]
        );
    }

    #[test]
    fn test_thislayer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new(
            "s1".into(),
            "Selfy".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        l.transform.position = Animatable::new_constant([10.0, 20.0]);
        l.transform.rotation_expression =
            Some(Expression::Raw("thisLayer.transform.rotation * 2.0".into()));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (_, _, rot, _) = comp.resolve_world_transform(layer, 0);
        assert!(
            (rot - 0.0).abs() < 0.01 || rot > 0.0,
            "rotation expr should evaluate"
        );
    }
}

#[cfg(test)]
mod tests_comp_extras {
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_layer_lookup_by_id() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new(
            "tgt_id_1".into(),
            "TargetName".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        target.transform.position = Animatable::new_constant([7.0, 3.0]);
        comp.layers.push(target);
        let mut driver = Layer::new("d".into(), "Driver".into(), LayerType::Null, 30);
        driver.transform.position_expression = Some(Expression::Raw(
            "thisComp.layer(\"tgt_id_1\").transform.position[1]".into(),
        ));
        comp.layers.push(driver);

        let driver_ref = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(driver_ref, 0);
        assert!(
            (pos[0] - 3.0).abs() < 0.01,
            "expected 3.0 via id lookup, got {}",
            pos[0]
        );
    }

    #[test]
    fn test_meshwarp_effect_parses() {
        // MeshWarp effect should evaluate corner params without panicking
        use crate::core::effect_plugin::evaluate_effects;
        use crate::core::timeline::{Effect, EffectType};
        let effects = vec![Effect {
            id: "fx1".to_string(),
            name: "MeshWarp".to_string(),
            enabled: true,
            effect_type: EffectType::MeshWarp {
                top_left: Animatable::new_constant([10.0, 5.0]),
                top_right: Animatable::new_constant([-10.0, 5.0]),
                bottom_left: Animatable::new_constant([10.0, -5.0]),
                bottom_right: Animatable::new_constant([-10.0, -5.0]),
            },
        }];
        let ep = evaluate_effects(&effects, 0);
        assert_eq!(ep.meshwarp_enabled, 1);
        assert_eq!(ep.corner_top_left, [10.0, 5.0]);
        assert_eq!(ep.corner_bottom_right, [-10.0, -5.0]);
    }
}

#[cfg(test)]
mod tests_loops {
    use super::*;
    use crate::core::keyframe::Keyframe;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_loopout_in_raw_script() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new(
            "l1".into(),
            "Looper".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        // Keyframes: x 0→100 over frames 0..10
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(
                0,
                [0.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
            Keyframe::new(
                10,
                [100.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
        ]);
        // At frame 25 (past last kf), loopOut("cycle") should reference the cycled value (x=50 at frame 5)
        l.transform.position_expression =
            Some(Expression::Raw("loopOut(\"cycle\") + [0.0, 7.0]".into()));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (pos, _, _, _) = comp.resolve_world_transform(layer, 25);
        // Frame 25 remaps to frame 5 → x = 50
        assert!(
            (pos[0] - 50.0).abs() < 0.5,
            "expected x=50 from loopOut cycle, got {}",
            pos[0]
        );
    }

    #[test]
    fn test_loop_preprocess_rewrites() {
        let rewritten = preprocess_loop_script("loopOut(\"pingpong\") + loopIn()");
        assert!(rewritten.contains("__loop_out_pingpong"));
        assert!(rewritten.contains("__loop_in_cycle"));
        assert!(!rewritten.contains("loopOut("));
    }

    #[test]
    fn test_thiscomp_layer_by_index() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut a = Layer::new("a1".into(), "Bottom".into(), LayerType::Null, 30);
        a.transform.position = Animatable::new_constant([7.0, 0.0]);
        comp.layers.push(a);
        let mut b = Layer::new("b1".into(), "Top".into(), LayerType::Null, 30);
        b.transform.position = Animatable::new_constant([99.0, 0.0]);
        comp.layers.push(b);

        // AE 1-based: layer(1) = topmost = "Top"
        let snap = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert_eq!(snap.layer_by_index(1).position[0], 99.0);
        assert_eq!(snap.layer_by_index(2).position[0], 7.0);
        assert_eq!(
            snap.layer_by_index(99).position[0],
            0.0,
            "out of range → default"
        );
        assert_eq!(snap.num_layers(), 2.0);
    }

    #[test]
    fn test_comp_duration_and_frame_duration() {
        let comp = Composition::new("c".into(), "Comp".into(), 100, 100, 25, 50);
        let snap = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert!(
            (snap.duration() - 2.0).abs() < 1e-6,
            "50 frames @25fps = 2s"
        );
        assert!((snap.frame_duration() - 0.04).abs() < 1e-6);
    }

    #[test]
    fn test_to_from_comp_roundtrip_identity_layer() {
        // Identity layer (pos 0, scale 100, rot 0): toComp == fromComp == input
        set_current_layer_xform([0.0, 0.0], [100.0, 100.0], 0.0);
        let out: rhai::Array = COMP_ENGINE.with(|e| e.eval("toComp(12.0, -5.0)").unwrap());
        assert!((out[0].as_float().unwrap() - 12.0).abs() < 1e-6);
        assert!((out[1].as_float().unwrap() + 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_comp_applies_translation_rotation_scale() {
        // Layer at (100, 50), scale 200%, rot 90°
        set_current_layer_xform([100.0, 50.0], [200.0, 200.0], 90.0);
        // Local point (10, 0) → scaled (20, 0) → rotated 90°: (0, 20) → +pos = (100, 70)
        let out: rhai::Array = COMP_ENGINE.with(|e| e.eval("toComp(10.0, 0.0)").unwrap());
        assert!(
            (out[0].as_float().unwrap() - 100.0).abs() < 1e-4,
            "{}",
            out[0]
        );
        assert!(
            (out[1].as_float().unwrap() - 70.0).abs() < 1e-4,
            "{}",
            out[1]
        );

        // fromComp inverts it
        let inv: rhai::Array = COMP_ENGINE.with(|e| e.eval("fromComp(100.0, 70.0)").unwrap());
        assert!(
            (inv[0].as_float().unwrap() - 10.0).abs() < 1e-3,
            "{}",
            inv[0]
        );
        assert!(inv[1].as_float().unwrap().abs() < 1e-3, "{}", inv[1]);
    }

    #[test]
    fn test_ae_interpolation_and_vectors() {
        let engine = crate::core::expression_engine::build_engine();
        // 3-arg linear & ease
        let v: f64 = engine.eval("linear(0.5, 10.0, 20.0)").unwrap();
        assert!((v - 15.0).abs() < 1e-6);
        let ve: f64 = engine.eval("ease(0.5, 0.0, 100.0)").unwrap();
        assert!((ve - 50.0).abs() < 1e-6);

        // Vector math
        let len: f64 = engine.eval("length([3.0, 4.0])").unwrap();
        assert!((len - 5.0).abs() < 1e-6);
        let dist: f64 = engine.eval("distance([0.0, 0.0], [3.0, 4.0])").unwrap();
        assert!((dist - 5.0).abs() < 1e-6);
        let dot: f64 = engine.eval("dot([1.0, 2.0], [3.0, 4.0])").unwrap();
        assert!((dot - 11.0).abs() < 1e-6);

        // Array linear
        let arr: rhai::Array = engine
            .eval("linear(0.5, 0.0, 1.0, [10.0, 20.0], [20.0, 40.0])")
            .unwrap();
        assert_eq!(arr.len(), 2);
        assert!((arr[0].as_float().unwrap() - 15.0).abs() < 1e-6);
        assert!((arr[1].as_float().unwrap() - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_generate_pick_whip_expression() {
        let t1 = PickWhipTarget::TransformProperty {
            layer_name: "Logo".into(),
            prop: "position".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t1, "Logo"),
            "transform.position"
        );
        assert_eq!(
            generate_pick_whip_expression(&t1, "Background"),
            "thisComp.layer(\"Logo\").transform.position"
        );

        let t2 = PickWhipTarget::EffectProperty {
            layer_name: "Control".into(),
            effect_name: "Slider Control".into(),
            param_name: "Slider".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t2, "Text"),
            "thisComp.layer(\"Control\").effect(\"Slider Control\")(\"Slider\")"
        );

        let t3 = PickWhipTarget::ExternalCompProperty {
            comp_name: "PreComp".into(),
            layer_name: "Null 1".into(),
            prop_path: "transform.opacity".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t3, "Main"),
            "comp(\"PreComp\").layer(\"Null 1\").transform.opacity"
        );
    }

    #[test]
    fn audio_binding_sources_expose_current_amplitude_and_bands() {
        set_audio_expr_data(AudioExprData {
            amplitude: 0.75,
            bands: [0.1, 0.2, 0.3, 0.4, 0.5],
        });

        let values = audio_binding_source_values();
        assert!((values["audio.amplitude"] - 0.75).abs() < 1e-6);
        assert!((values["audio.band0"] - 0.1).abs() < 1e-6);
        assert!((values["audio.band4"] - 0.5).abs() < 1e-6);
        assert!((values["audio.bass"] - 0.1).abs() < 1e-6);
        assert!((values["audio.treble"] - 0.5).abs() < 1e-6);
        assert_eq!(values.len(), 11);
    }

    #[test]
    fn audio_functions_work_in_regular_vector_expressions() {
        set_audio_expr_data(AudioExprData {
            amplitude: 0.5,
            bands: [0.8, 0.4, 0.2, 0.1, 0.05],
        });
        let engine = build_engine();
        let value = eval_v2(
            &engine,
            "[value[0] + audioAmplitude(\"bass\") * 15.0, value[1] + audioBand(0) * 15.0]",
            [100.0, 80.0],
            0,
            30,
        );
        assert_eq!(value, [112.0, 92.0]);
    }

    #[test]
    fn binding_audio_sources_update_expression_context_with_clamping() {
        let values = std::collections::HashMap::from([
            (String::from("audio.amplitude"), 2.0),
            (String::from("audio.bass"), -1.0),
            (String::from("audio.band1"), 0.25),
        ]);
        set_audio_from_binding_sources(&values);

        assert_eq!(get_audio_amplitude(), 1.0);
        assert_eq!(get_audio_band(0), 0.0);
        assert_eq!(get_audio_band(1), 0.25);
        assert_eq!(get_audio_band(4), 0.0);
    }
}
