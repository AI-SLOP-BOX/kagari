//! Centralized reflection over `EffectType` keyframeable parameters.
//!
//! The timeline property rows need mutable access to every `Animatable`
//! track inside an effect. Maintaining that match in the UI layer meant
//! new effect variants silently got no rows. This module is the single
//! place to register params: extend the match here when adding variants.
//!
//! Variants not listed fall through the catch-all and simply expose no
//! timeline rows (same as before) — adding a variant never breaks the build.

pub mod immutable;
pub mod keyframes;
pub mod mutable;
pub mod types;

pub use types::{ParamRef, ParamRefRef};

#[cfg(test)]
mod registration_tests {
    use super::*;
    use crate::core::effect_plugin::RenderEffectPlugin;
    use crate::core::property::Animatable;
    use crate::core::timeline::EffectType;

    fn c() -> Animatable<f32> {
        Animatable::new_constant(0.0)
    }

    #[test]
    fn formerly_unregistered_variants_expose_keyframe_rows() {
        let mut cases: Vec<EffectType> = vec![
            EffectType::StarField {
                num_stars: c(),
                depth_speed: c(),
            },
            EffectType::WaveWarp {
                wave_height: c(),
                wave_width: c(),
                speed: c(),
                direction_deg: c(),
                wave_type: 0,
                pinning: 0,
            },
            EffectType::Minimax {
                operation: c(),
                radius: c(),
            },
            EffectType::ShiftChannels {
                take_red: c(),
                take_green: c(),
                take_blue: c(),
                take_alpha: c(),
            },
            EffectType::LightSweep {
                direction_deg: c(),
                center: c(),
                width: c(),
                sweep_intensity: c(),
                edge_intensity: c(),
            },
            EffectType::WhiteBalance {
                temperature: c(),
                tint: c(),
            },
            EffectType::HslAdjust {
                hue_deg: c(),
                saturation: c(),
                lightness: c(),
            },
            EffectType::CompoundBlur {
                source_layer: c(),
                max_blur: c(),
            },
        ];
        for effect in &mut cases {
            let rows = effect.animatable_params().len();
            assert!(rows > 0, "{} exposes no timeline rows", effect_name(effect));
        }
    }

    fn effect_name(e: &EffectType) -> String {
        crate::core::effect_plugin::EnumEffectPlugin {
            effect_type: e.clone(),
        }
        .name()
        .to_string()
    }

    #[test]
    fn animatable_params_ref_matches_mutable_counts() {
        let mut mutable = EffectType::LightSweep {
            direction_deg: c(),
            center: c(),
            width: c(),
            sweep_intensity: c(),
            edge_intensity: c(),
        };
        let read_only = EffectType::LightSweep {
            direction_deg: c(),
            center: c(),
            width: c(),
            sweep_intensity: c(),
            edge_intensity: c(),
        };
        assert_eq!(
            mutable.animatable_params().len(),
            read_only.animatable_params_ref().len(),
        );
    }

    #[test]
    fn parameter_interpolation_changes_only_selected_effect_track() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
            intensity: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                0,
                0.5,
                crate::core::keyframe::InterpolationType::Linear,
            )]),
        };

        assert!(effect.set_parameter_keyframe_interpolation(
            Some("Intensity"),
            crate::core::keyframe::InterpolationType::Hold,
        ));
        let params = effect.animatable_params_ref();
        let intensity = params
            .into_iter()
            .find_map(|(name, parameter)| (name == "Intensity").then_some(parameter));
        match intensity.expect("intensity parameter should be registered") {
            ParamRefRef::Scalar(track) => assert!(matches!(
                track.keyframes().unwrap()[0].interpolation,
                crate::core::keyframe::InterpolationType::Hold
            )),
            _ => panic!("intensity must be scalar"),
        }
    }

    #[test]
    fn removing_last_scalar_key_restores_constant_value() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                12,
                7.5,
                crate::core::keyframe::InterpolationType::Linear,
            )]),
        };

        assert!(effect.remove_scalar_parameter_keyframe("Blur Radius", 12));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => {
                assert!(matches!(track, Animatable::Constant(value) if *value == 7.5));
            }
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn component_keyframe_api_supports_color_and_rejects_invalid_channel() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            intensity: c(),
        };

        assert!(effect.set_parameter_component_keyframe("Tint Color", 1, 24, 0.75));
        assert!(!effect.set_parameter_component_keyframe("Tint Color", 4, 24, 0.5));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert_eq!(track.evaluate(24)[1], 0.75),
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn component_keyframe_api_preserves_unedited_vector_channels() {
        let mut effect = EffectType::DropShadow {
            color: Animatable::new_constant([0.1, 0.2, 0.3, 1.0]),
            opacity: c(),
            direction: c(),
            distance: Animatable::new_constant(10.0),
            softness: c(),
        };

        assert!(effect.set_parameter_component_keyframe("Shadow Color", 2, 30, 0.9));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert_eq!(track.evaluate(30), [0.1, 0.2, 0.9, 1.0]),
            _ => panic!("shadow color must be a color track"),
        }
        assert!(effect.set_parameter_keyframe_interpolation(
            Some("Shadow Color"),
            crate::core::keyframe::InterpolationType::Hold,
        ));
        assert!(effect.set_parameter_component_keyframe("Shadow Color", 0, 30, 0.8));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(30), [0.8, 0.2, 0.9, 1.0]);
                assert_eq!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .filter(|key| key.frame == 30)
                        .count(),
                    1
                );
            }
            _ => panic!("shadow color must be a color track"),
        }
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert!(matches!(
                track.keyframes().unwrap()[0].interpolation,
                crate::core::keyframe::InterpolationType::Hold
            )),
            _ => panic!("shadow color must be a color track"),
        }
    }

    #[test]
    fn component_interpolation_can_be_restored_at_one_frame() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                8,
                [0.2, 0.3, 0.4, 1.0],
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: Default::default(),
                    incoming: Default::default(),
                    custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
                },
            )]),
            intensity: c(),
        };

        assert!(effect.set_parameter_keyframe_interpolation_at_frame(
            "Tint Color",
            8,
            crate::core::keyframe::InterpolationType::Hold,
        ));
        assert!(!effect.set_parameter_keyframe_interpolation_at_frame(
            "Tint Color",
            99,
            crate::core::keyframe::InterpolationType::Linear,
        ));
    }

    #[test]
    fn scalar_keyframe_addition_is_idempotent_for_same_value() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(4.0),
        };

        assert!(effect.set_scalar_parameter_keyframe("Blur Radius", 5, 4.0));
        assert!(!effect.set_scalar_parameter_keyframe("Blur Radius", 5, 4.0));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => assert_eq!(
                track
                    .keyframes()
                    .unwrap()
                    .iter()
                    .filter(|key| key.frame == 5)
                    .count(),
                1
            ),
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn non_finite_keyframe_edits_are_rejected_without_mutation() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                10,
                f32::NAN,
                crate::core::keyframe::InterpolationType::Hold,
            )]),
        };

        assert!(!effect.set_scalar_parameter_keyframe("Blur Radius", 20, f32::NAN));
        assert!(!effect.set_scalar_parameter_keyframe("Blur Radius", 20, f32::INFINITY));
        assert!(!effect.move_scalar_parameter_keyframe("Blur Radius", 10, 20));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => {
                let keys = track.keyframes().expect("track remains animated");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].frame, 10);
                assert!(keys[0].value.is_nan());
            }
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn non_finite_component_edits_are_rejected_without_mutation() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                10,
                [0.1, f32::NAN, 0.3, 1.0],
                crate::core::keyframe::InterpolationType::Linear,
            )]),
            intensity: c(),
        };

        assert!(!effect.set_parameter_component_keyframe("Tint Color", 1, 20, f32::NAN));
        assert!(!effect.move_parameter_component_keyframe("Tint Color", 1, 10, 20));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                let keys = track.keyframes().expect("track remains animated");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].frame, 10);
                assert!(keys[0].value[1].is_nan());
            }
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn component_keyframe_move_preserves_the_shared_vector_keyframe() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                10,
                [0.1, 0.2, 0.3, 1.0],
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: Default::default(),
                    incoming: Default::default(),
                    custom_bezier: Some([0.2, 0.1, 0.8, 0.9]),
                },
            )]),
            intensity: c(),
        };
        assert!(effect.move_parameter_component_keyframe("Tint Color", 2, 10, 20));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(20), [0.1, 0.2, 0.3, 1.0]);
                assert!(track.keyframes().unwrap().iter().all(|key| key.frame != 10));
                assert!(matches!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .find(|key| key.frame == 20)
                        .unwrap()
                        .interpolation,
                    crate::core::keyframe::InterpolationType::Bezier { .. }
                ));
            }
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn component_keyframe_move_rejects_missing_or_same_frame_without_mutation() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([0.1, 0.2, 0.3, 1.0]),
            intensity: c(),
        };
        assert!(!effect.move_parameter_component_keyframe("Tint Color", 0, 4, 4));
        assert!(!effect.move_parameter_component_keyframe("Tint Color", 0, 4, 8));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert!(track.keyframes().is_none()),
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn scalar_keyframe_move_preserves_value_and_interpolation() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                12,
                18.0,
                crate::core::keyframe::InterpolationType::Hold,
            )]),
        };
        assert!(effect.move_scalar_parameter_keyframe("Blur Radius", 12, 24));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => {
                assert_eq!(track.evaluate(24), 18.0);
                let key = track
                    .keyframes()
                    .unwrap()
                    .iter()
                    .find(|key| key.frame == 24)
                    .unwrap();
                assert!(matches!(
                    key.interpolation,
                    crate::core::keyframe::InterpolationType::Hold
                ));
            }
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn component_keyframe_move_replaces_existing_destination_vector_keyframe() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![
                crate::core::keyframe::Keyframe::new(
                    5,
                    [0.9, 0.2, 0.3, 1.0],
                    crate::core::keyframe::InterpolationType::Linear,
                ),
                crate::core::keyframe::Keyframe::new(
                    15,
                    [0.1, 0.8, 0.7, 1.0],
                    crate::core::keyframe::InterpolationType::Hold,
                ),
            ]),
            intensity: c(),
        };
        assert!(effect.move_parameter_component_keyframe("Tint Color", 0, 5, 15));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(15), [0.9, 0.2, 0.3, 1.0]);
                assert_eq!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .filter(|key| key.frame == 15)
                        .count(),
                    1
                );
                assert!(track.keyframes().unwrap().iter().all(|key| key.frame != 5));
            }
            _ => panic!("tint color must be a color track"),
        }
    }
}
