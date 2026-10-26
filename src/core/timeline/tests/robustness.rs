#[cfg(test)]
mod robustness_tests {
    use super::*;
    use crate::core::property::Animatable;

    #[test]
    fn test_cyclic_parent_chain_does_not_recurse_forever() {
        // Hand-edited / corrupted project: A -> B -> A
        let mut comp = Composition::new("c".into(), "Comp".into(), 64, 64, 30, 30);
        let mut a = Layer::new(
            "a".into(),
            "A".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        let mut b = Layer::new("b".into(), "B".into(), LayerType::Null, 30);
        a.parent_id = Some("b".to_string());
        b.parent_id = Some("a".to_string());
        comp.layers.push(a);
        comp.layers.push(b);

        // Must not stack overflow; returns some sane transform
        let layer = &comp.layers[0];
        let (pos, scale, _rot, _opa) = comp.resolve_world_transform(layer, 0);
        assert!(pos.iter().all(|v| v.is_finite()), "position must be finite");
        assert!(scale.iter().all(|v| v.is_finite()), "scale must be finite");
    }

    #[test]
    fn test_self_parent_is_safe_at_render() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 32, 32, 30, 30);
        let mut l = Layer::new(
            "s".into(),
            "SelfParent".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        l.parent_id = Some("s".to_string());
        comp.layers.push(l);

        let pixels =
            crate::core::software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_cyclic_precomp_nesting_terminates() {
        // Sub-comp A contains a PreComp layer pointing to B, B points back to A
        let mut comp_a = Composition::new("A".into(), "CompA".into(), 32, 32, 30, 30);
        let comp_b = Composition::new("B".into(), "CompB".into(), 32, 32, 30, 30);
        let pc = Layer::new(
            "pc".into(),
            "Nested".into(),
            LayerType::PreComp {
                comp_id: "B".into(),
            },
            30,
        );
        comp_a.layers.push(pc);
        comp_a.sub_compositions.push(comp_b);
        let pc_back = Layer::new(
            "pcb".into(),
            "Back".into(),
            LayerType::PreComp {
                comp_id: "A".into(),
            },
            30,
        );
        comp_a.sub_compositions[0].layers.push(pc_back);

        // Must terminate without stack overflow
        let pixels =
            crate::core::software_renderer::render_frame_to_pixels(&comp_a, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_nan_and_infinite_property_values_render_safely() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 32, 32, 30, 30);
        let mut l = Layer::new(
            "n".into(),
            "NaN Layer".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        // Corrupted values via extreme keyframes
        l.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(
                0,
                [f32::NAN, f32::INFINITY],
                crate::core::keyframe::InterpolationType::Linear,
            ),
            crate::core::keyframe::Keyframe::new(
                10,
                [f32::NEG_INFINITY, 1e30],
                crate::core::keyframe::InterpolationType::Linear,
            ),
        ]);
        l.transform.scale = Animatable::new_constant([f32::NAN, -1e20]);
        l.transform.opacity = Animatable::new_constant(f32::NAN);
        comp.layers.push(l);

        // Must not panic or hang
        let pixels =
            crate::core::software_renderer::render_frame_to_pixels(&comp, 5, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_malformed_project_json_fails_gracefully() {
        let bad_payloads = [
            "",
            "{",
            "null",
            r#"{"compositions": "not-an-array"}"#,
            r#"{"compositions":[{"layers": 42}]}"#,
            "\u{0}\u{1}\u{2}",
        ];
        for payload in bad_payloads {
            let result: Result<Project, _> = serde_json::from_str(payload);
            assert!(
                result.is_err(),
                "malformed JSON must be rejected: {:?}",
                payload
            );
        }
    }

    #[test]
    fn test_expression_injection_is_contained() {
        // Expression sandbox must reject resource-abusive scripts without panicking
        use crate::core::expression_engine::{build_engine, eval_f32};
        let engine = build_engine();
        // Deep recursion attempt — must be capped to the base fallback, never crash
        let evil = "fn f(n) { if n <= 0 { 0 } else { f(n-1) + 1 } } f(100000)";
        let r = eval_f32(&engine, evil, 0.0, 0, 30);
        assert!(
            r.is_finite(),
            "recursion bomb must resolve to a finite fallback, got {}",
            r
        );
        assert_eq!(
            r, 0.0,
            "uncapped recursion would return 100000; sandbox must fall back to base"
        );
        // Huge loop via string building — capped by max_operations
        let evil2 = "let s = \"\"; for i in 0..1000000 { s += \"x\"; } 42";
        let r2 = eval_f32(&engine, evil2, 7.5, 0, 30);
        assert_eq!(r2, 7.5, "runaway loop must fall back to base, got {}", r2);
        // Engine must not be poisoned: valid scripts still evaluate afterwards.
        assert_eq!(eval_f32(&engine, "1 + 2", 0.0, 0, 30), 3.0);
    }
}
