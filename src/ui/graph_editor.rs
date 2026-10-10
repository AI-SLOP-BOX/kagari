use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

mod camera;
mod canvas;
pub(crate) mod channel_ops;
pub(crate) mod interpolation_ops;
pub(crate) mod property;
pub(crate) mod toolbar;
pub(crate) mod velocity_ease;
pub(crate) mod workspace;

pub use camera::draw_camera_lens_graph;
pub use canvas::draw_graph_editor;
pub use channel_ops::{
    axis_3d, clamp_bezier_x, graph_keyframe_frame, is_effect_property, move_and_set_channel,
    parse_effect_property, pin_anim_mut, pin_component, reverse_effect_property,
    reverse_keyframes, rove_across_time, rove_keyframes, GraphChannelValue,
};
pub use interpolation_ops::{
    effect_channel_bezier_points, effect_parameter_channel_keyframes,
    effect_parameter_channel_value, map_layer_interpolation, move_effect_channel_keyframe,
    move_effect_scalar_keyframe, remove_effect_channel_at_frame, set_effect_channel_at_frame,
    set_effect_channel_bezier, set_layer_interpolation,
};
pub(crate) use property::GraphProperty;
pub use velocity_ease::{
    compute_velocity_curve, draw_ease_thumbnail, layer_velocity, remove_camera_key,
    set_camera_key_ease, set_camera_key_interpolation, track_velocity,
};
pub use workspace::{draw_animation_graph_editor, draw_automation_curve};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphKeyframeDrag {
    pub original_frame: u32,
    pub current_frame: u32,
    pub original_value: f32,
    pub current_value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::timeline::{Layer, LayerType};

    #[test]
    fn bezier_x_clamp_handles_collapsed_bounds_without_panicking() {
        assert_eq!(clamp_bezier_x(0.5, 1.01, 1.0), 1.0);
        assert_eq!(clamp_bezier_x(0.5, 0.0, -0.01), 0.0);
        assert_eq!(clamp_bezier_x(0.5, 0.2, 0.8), 0.5);
    }

    #[test]
    fn active_drag_visual_follows_keyframe_after_sorting() {
        let mut layer = Layer::new(
            "layer".into(),
            "Layer".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            120,
        );
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [100.0, 540.0], InterpolationType::Linear),
            Keyframe::new(60, [600.0, 540.0], InterpolationType::Linear),
            Keyframe::new(90, [350.0, 540.0], InterpolationType::Linear),
        ]);
        let drag_id = eframe::egui::Id::new(("graph_keyframe_drag", &layer.id, "Position X"));
        let context = eframe::egui::Context::default();
        context.data_mut(|data| {
            data.insert_temp(
                drag_id,
                GraphKeyframeDrag {
                    original_frame: 30,
                    current_frame: 90,
                    original_value: 300.0,
                    current_value: 350.0,
                },
            )
        });
        let mut selected_property = Some("Position X".to_owned());
        let mut changed = false;
        let mut linked_tangent = true;
        let screen_rect = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(900.0, 600.0),
        );
        let _ = context.run(
            eframe::egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |context| {
                eframe::egui::CentralPanel::default().show(context, |ui| {
                    draw_graph_editor(
                        &mut selected_property,
                        ui,
                        120,
                        30,
                        &mut layer,
                        &mut changed,
                        &mut linked_tangent,
                    );
                });
            },
        );

        let rect_at = |frame| {
            context
                .data(|data| {
                    data.get_temp::<eframe::egui::Rect>(eframe::egui::Id::new((
                        "ae_graph_anchor_rect",
                        &layer.id,
                        "Position X",
                        frame,
                    )))
                })
                .expect("each visible keyframe has a rendered graph anchor")
        };
        let frame_60 = rect_at(60);
        let frame_90 = rect_at(90);
        assert!(
            frame_90.center().x - frame_60.center().x > 10.0,
            "the drag state must stay attached to its retimed key, not the key now at its old index"
        );
    }

    #[test]
    fn interpolation_commands_respect_selected_key_scope() {
        let mut layer = Layer::new(
            "layer".into(),
            "Layer".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            60,
        );
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(20, [20.0, 0.0], InterpolationType::Linear),
        ]);

        let mut make_hold = |interpolation: &mut InterpolationType| {
            *interpolation = InterpolationType::Hold;
        };
        assert!(map_layer_interpolation(
            &mut layer,
            "Position X",
            Some(20),
            &mut make_hold,
        ));
        let keys = layer.transform.position.keyframes().unwrap();
        assert_eq!(keys[0].interpolation, InterpolationType::Linear);
        assert_eq!(keys[1].interpolation, InterpolationType::Hold);

        let mut make_linear = |interpolation: &mut InterpolationType| {
            *interpolation = InterpolationType::Linear;
        };
        assert!(map_layer_interpolation(
            &mut layer,
            "Position X",
            None,
            &mut make_linear,
        ));
        assert!(layer
            .transform
            .position
            .keyframes()
            .unwrap()
            .iter()
            .all(|key| key.interpolation == InterpolationType::Linear));
    }

    #[test]
    fn move_and_set_channel_reacquires_keyframe_after_sorting() {
        let mut track = Animatable::new_animated(vec![
            Keyframe::new(10, [1.0, 2.0], InterpolationType::Linear),
            Keyframe::new(20, [3.0, 4.0], InterpolationType::Linear),
            Keyframe::new(30, [5.0, 6.0], InterpolationType::Linear),
        ]);

        assert!(move_and_set_channel(&mut track, 20, 40, 0, 9.0));
        let keyframes = track.keyframes().unwrap();
        assert_eq!(
            keyframes.iter().map(|key| key.frame).collect::<Vec<_>>(),
            vec![10, 30, 40]
        );
        assert_eq!(
            keyframes.iter().find(|key| key.frame == 30).unwrap().value,
            [5.0, 6.0]
        );
        assert_eq!(
            keyframes.iter().find(|key| key.frame == 40).unwrap().value,
            [9.0, 4.0]
        );
    }

    #[test]
    fn effect_property_uses_instance_id_and_component() {
        assert_eq!(
            parse_effect_property("fxid:effect-2::Glow Radius|1"),
            Some(("effect-2", "Glow Radius", Some(1)))
        );
        assert_eq!(
            parse_effect_property("fxid:effect-2::Opacity"),
            Some(("effect-2", "Opacity", None))
        );
        assert!(parse_effect_property("fx_Glow_Opacity").is_none());
    }

    #[test]
    fn rove_keyframes_uses_cumulative_spatial_distance() {
        let mut keys = vec![
            Keyframe::new(0, [0.0_f32, 0.0], InterpolationType::Linear),
            Keyframe::new(10, [100.0_f32, 0.0], InterpolationType::Linear),
            Keyframe::new(20, [900.0_f32, 0.0], InterpolationType::Linear),
        ];
        assert!(rove_keyframes(&mut keys, |a, b| (b[0] - a[0]).abs()));
        assert_eq!(
            keys.iter().map(|key| key.frame).collect::<Vec<_>>(),
            vec![0, 2, 20]
        );
    }

    #[test]
    fn velocity_preserves_fields_and_uses_segment_units() {
        let mut track = Animatable::new_animated(vec![
            Keyframe::new(10, 100.0, InterpolationType::Linear),
            Keyframe::new(130, 500.0, InterpolationType::Linear),
        ]);
        let values = [25.0, 40.0, 50.0, 100.0];
        track_velocity(&mut track, 0, 60.0, |v| *v, Some(values)).unwrap();
        assert_eq!(
            track_velocity(&mut track, 0, 60.0, |v| *v, None),
            Some(values)
        );
        let mut edited = values;
        edited[0] = 30.0;
        track_velocity(&mut track, 0, 60.0, |v| *v, Some(edited)).unwrap();
        for (actual, expected) in track_velocity(&mut track, 0, 60.0, |v| *v, None)
            .unwrap()
            .into_iter()
            .zip(edited)
        {
            assert!((actual - expected).abs() < 1e-4);
        }
        let InterpolationType::Bezier {
            custom_bezier: Some(cp),
            ..
        } = track.keyframes().unwrap()[0].interpolation
        else {
            panic!("Expected Bezier");
        };
        for (actual, expected) in cp.into_iter().zip([0.4, 0.2, 0.7, 0.925]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert!(track_velocity(&mut track, 1, 60.0, |v| *v, Some(values)).is_none());
    }

    #[test]
    fn velocity_updates_each_three_d_track_and_axis() {
        for property in ["3D Position Z", "3D Rotation Y", "3D Scale X"] {
            let mut layer = Layer::new(
                "layer".into(),
                "Layer".into(),
                LayerType::Solid { color: [1.0; 4] },
                120,
            );
            let track = Animatable::new_animated(vec![
                Keyframe::new(0, [0.0; 3], InterpolationType::Linear),
                Keyframe::new(60, [100.0, 200.0, 400.0], InterpolationType::Linear),
            ]);
            layer.transform_3d.position = track.clone();
            layer.transform_3d.rotation = track.clone();
            layer.transform_3d.scale = track;
            let values = [30.0, 40.0, 50.0, 100.0];
            assert_eq!(
                layer_velocity(&mut layer, property, 0, 60.0, Some(values)),
                Some(values)
            );
            for (actual, expected) in layer_velocity(&mut layer, property, 0, 60.0, None)
                .unwrap()
                .into_iter()
                .zip(values)
            {
                assert!((actual - expected).abs() < 1e-4);
            }
            let changed = if property.starts_with("3D Position") {
                &layer.transform_3d.position
            } else if property.starts_with("3D Rotation") {
                &layer.transform_3d.rotation
            } else {
                &layer.transform_3d.scale
            };
            let InterpolationType::Bezier {
                custom_bezier: Some(cp),
                ..
            } = changed.keyframes().unwrap()[0].interpolation
            else {
                panic!("Expected Bezier");
            };
            let expected = if property.ends_with('Z') {
                0.1
            } else if property.ends_with('Y') {
                0.2
            } else {
                0.4
            };
            assert!((cp[1] - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn three_d_property_axis_selection_is_stable() {
        assert_eq!(axis_3d("3D Position X"), 0);
        assert_eq!(axis_3d("3D Rotation Y"), 1);
        assert_eq!(axis_3d("3D Scale Z"), 2);
    }

    #[test]
    fn unknown_axis_defaults_to_x_for_backward_compatibility() {
        assert_eq!(axis_3d("Position X"), 0);
        assert_eq!(axis_3d("3D Position"), 0);
    }

    #[test]
    fn removing_missing_camera_key_is_a_noop() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(!remove_camera_key(&mut track, 11));
        assert_eq!(
            track
                .as_ref()
                .and_then(|value| value.keyframes())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn removing_last_camera_key_clears_empty_track() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(remove_camera_key(&mut track, 10));
        assert!(track.is_none());
    }

    #[test]
    fn camera_key_interpolation_changes_only_existing_key() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(set_camera_key_interpolation(
            &mut track,
            10,
            InterpolationType::Hold
        ));
        assert!(!set_camera_key_interpolation(
            &mut track,
            11,
            InterpolationType::Linear
        ));
        assert_eq!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Hold
        );
    }

    #[test]
    fn camera_key_ease_creates_bezier_only_at_current_frame() {
        let mut track = Some(Animatable::new_animated(vec![
            Keyframe::new(10, 50.0, InterpolationType::Linear),
            Keyframe::new(20, 60.0, InterpolationType::Linear),
        ]));

        assert!(set_camera_key_ease(
            &mut track,
            10,
            crate::core::keyframe::EasePreset::EaseIn
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Bezier { .. }
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[1].interpolation,
            InterpolationType::Linear
        ));
        assert!(!set_camera_key_ease(
            &mut track,
            99,
            crate::core::keyframe::EasePreset::EaseOut
        ));
    }
}
