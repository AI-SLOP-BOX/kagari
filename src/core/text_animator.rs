//! After Effects per-character Text Animator & Range Selector Engine.
//!
//! Computes per-character spatial offsets, scale scaling, opacity fading,
//! and tracking adjustments driven by animated Range Selector parameters.

pub mod engine;
pub mod types;

pub use engine::TextAnimatorEngine;
pub use types::{CharacterTransform, RangeSelector, SelectorShape, TextAnimatorSettings};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_animator_range_selector_amount() {
        let selector = RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: SelectorShape::RampUp,
            ..Default::default()
        };

        let amt_first = TextAnimatorEngine::compute_amount(0, 10, &selector, 0.0);
        let amt_mid = TextAnimatorEngine::compute_amount(5, 10, &selector, 0.0);
        let amt_last = TextAnimatorEngine::compute_amount(9, 10, &selector, 0.0);

        assert_eq!(amt_first, 0.0);
        assert!((amt_mid - 0.5).abs() < 1e-4);
        assert!((amt_last - 0.9).abs() < 1e-4);
    }

    #[test]
    fn test_char_range_selection() {
        let mut selector = RangeSelector {
            shape: SelectorShape::Square,
            char_start: 2,
            char_end: 4,
            ..Default::default()
        };

        let amounts: Vec<f32> = (0..6)
            .map(|i| TextAnimatorEngine::compute_amount(i, 6, &selector, 0.0))
            .collect();

        assert_eq!(amounts[0], 0.0);
        assert_eq!(amounts[1], 0.0);
        assert_eq!(amounts[2], 1.0);
        assert_eq!(amounts[3], 1.0);
        assert_eq!(amounts[4], 1.0);
        assert_eq!(amounts[5], 0.0);

        // char_start only (open-ended end)
        selector.char_end = -1;
        assert_eq!(
            TextAnimatorEngine::compute_amount(5, 6, &selector, 0.0),
            1.0
        );
    }

    #[test]
    fn test_wobble_shape_bounded_and_deterministic() {
        let selector = RangeSelector {
            shape: SelectorShape::Wobble,
            ..Default::default()
        };

        for i in 0..20 {
            let a = TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0);
            assert!(
                (0.0..=1.0).contains(&a),
                "amount {a} out of range at idx {i}"
            );
            assert_eq!(a, TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0));
        }
    }

    #[test]
    fn test_random_shape_deterministic() {
        let selector = RangeSelector {
            shape: SelectorShape::Random,
            ..Default::default()
        };

        for i in 0..20 {
            let a = TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0);
            assert!((0.0..1.0).contains(&a));
            assert_eq!(a, TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0));
        }
    }

    #[test]
    fn test_random_order_is_bijective_permutation() {
        let selector = RangeSelector {
            random_order: true,
            ..Default::default()
        };

        // Square shape with random_order: every char gets amount 1 exactly once.
        let ones = (0..10)
            .filter(|&i| TextAnimatorEngine::compute_amount(i, 10, &selector, 0.0) == 1.0)
            .count();
        assert_eq!(ones, 10);
    }

    #[test]
    fn test_blur_evaluation() {
        let transforms = TextAnimatorEngine::eval_character_transforms_extended(
            "abcd",
            &RangeSelector {
                shape: SelectorShape::RampUp,
                ..Default::default()
            },
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            0.0,
            0.0,
            8.0,
            false,
            0.0,
        );

        assert_eq!(transforms.len(), 4);
        assert_eq!(transforms[0].blur, 0.0);
        assert!((transforms[3].blur - 6.0).abs() < 1e-4); // 8.0 * 0.75

        // Legacy API yields zero blur.
        let legacy = TextAnimatorEngine::eval_character_transforms(
            "abcd",
            &RangeSelector::default(),
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(legacy.iter().all(|t| t.blur == 0.0));
    }

    #[test]
    fn test_cumulative_tracking() {
        let tracking = 2.0;
        let transforms = TextAnimatorEngine::eval_character_transforms_extended(
            "abc",
            &RangeSelector::default(), // Square, all chars amount = 1
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            tracking,
            0.0,
            0.0,
            true,
            0.0,
        );

        // AE-style accumulation: offsets are 2, 4, 6 instead of flat 2 each.
        assert!((transforms[0].tracking_offset - 2.0).abs() < 1e-4);
        assert!((transforms[1].tracking_offset - 4.0).abs() < 1e-4);
        assert!((transforms[2].tracking_offset - 6.0).abs() < 1e-4);
    }

    #[test]
    fn test_serde_backward_compat_without_new_fields() {
        // Old project JSON lacking blur_amount / char range fields must deserialize.
        let json = r#"{
            "enabled": true,
            "selector": {"start": 0.0, "end": 100.0, "offset": 0.0,
                         "shape": "Square", "ease_high": 0.0, "ease_low": 0.0,
                         "random_order": false},
            "position_offset": [0.0, -40.0],
            "scale": [1.0, 1.0],
            "opacity": 0.0,
            "tracking": 0.0,
            "rotation": 0.0
        }"#;
        let settings: TextAnimatorSettings = serde_json::from_str(json).unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.blur_amount, 0.0);
        assert_eq!(settings.selector.char_start, -1);
        assert_eq!(settings.selector.char_end, -1);
    }

    #[test]
    fn test_expression_selector_basic() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("index / total".into()),
            ..Default::default()
        };
        let a = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        let b = TextAnimatorEngine::compute_amount(5, 10, &sel, 0.0);
        let c = TextAnimatorEngine::compute_amount(9, 10, &sel, 0.0);
        assert!((a - 0.0).abs() < 0.01, "idx=0: {}", a);
        assert!((b - 0.5).abs() < 0.01, "idx=5: {}", b);
        assert!((c - 0.9).abs() < 0.01, "idx=9: {}", c);
    }

    #[test]
    fn test_expression_selector_uses_time() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("if time > 0.5 { 1.0 } else { 0.0 }".into()),
            ..Default::default()
        };
        let at_0 = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        let at_1 = TextAnimatorEngine::compute_amount(0, 10, &sel, 1.0);
        assert!((at_0 - 0.0).abs() < 0.01);
        assert!((at_1 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_expression_selector_clamped_01() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("2.0".into()),
            ..Default::default()
        };
        let v = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        assert!((v - 1.0).abs() < 0.01, "should clamp: {}", v);
    }

    #[test]
    fn test_expression_selector_no_expr_is_zero() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: None,
            ..Default::default()
        };
        let v = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        assert!((v - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_expression_selector_text_index_text_total() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("textIndex / textTotal".into()),
            ..Default::default()
        };
        let a = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0); // textIndex=1, textTotal=10 -> 0.1
        let b = TextAnimatorEngine::compute_amount(9, 10, &sel, 0.0); // textIndex=10, textTotal=10 -> 1.0
        assert!((a - 0.1).abs() < 0.01, "a={}", a);
        assert!((b - 1.0).abs() < 0.01, "b={}", b);
    }
}
