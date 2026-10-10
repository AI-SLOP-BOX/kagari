use super::super::keyframe_menu::KeyframeMenuAction;
use crate::core::timeline::Layer;

pub fn move_kf<T: Clone>(anim: &mut crate::core::property::Animatable<T>, old_f: u32, new_f: u32) {
    let _ = anim.move_keyframe(old_f, new_f);
}

pub fn set_kf_interp<T: Clone>(
    anim: &mut crate::core::property::Animatable<T>,
    frames: &[u32],
    mode: KeyframeMenuAction,
) {
    use crate::core::keyframe::{BezierControlPoint, InterpolationType};
    if let Some(kfs) = anim.keyframes_mut() {
        for kf in kfs.iter_mut() {
            if !frames.contains(&kf.frame) {
                continue;
            }
            kf.interpolation = match mode {
                KeyframeMenuAction::Linear => InterpolationType::Linear,
                KeyframeMenuAction::EasyEase => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::Standard.control_points(),
                    ),
                },
                KeyframeMenuAction::ToggleHold => {
                    if matches!(kf.interpolation, InterpolationType::Hold) {
                        InterpolationType::Linear
                    } else {
                        InterpolationType::Hold
                    }
                }
                KeyframeMenuAction::EaseIn => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::EaseIn.control_points(),
                    ),
                },
                KeyframeMenuAction::EaseOut => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::EaseOut.control_points(),
                    ),
                },
                KeyframeMenuAction::Overshoot => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::Overshoot.control_points(),
                    ),
                },
                KeyframeMenuAction::Bounce => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::Bounce.control_points(),
                    ),
                },
                KeyframeMenuAction::Elastic => InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: Some(
                        crate::core::keyframe::EasePreset::Elastic.control_points(),
                    ),
                },
                KeyframeMenuAction::Reverse | KeyframeMenuAction::Delete => kf.interpolation,
            };
        }
    }
}

pub fn reverse_track<T: Clone>(anim: &mut crate::core::property::Animatable<T>) {
    let _ = anim.reverse_keyframes();
}

pub fn delete_track_kf<T: Clone>(anim: &mut crate::core::property::Animatable<T>, frame: u32) {
    if let Some(kfs) = anim.keyframes_mut() {
        kfs.retain(|k| k.frame != frame);
    }
}

pub fn apply_kf_menu_commands(
    layer: &mut Layer,
    layer_idx: usize,
    selected_keyframes: &std::collections::HashSet<(usize, String, u32)>,
    commands: Vec<(String, u32, KeyframeMenuAction)>,
    project_changed: &mut bool,
) {
    for (pk, f, cmd) in commands {
        let t = &mut layer.transform;
        let sel_frames: Vec<u32> = selected_keyframes
            .iter()
            .filter(|(li, p, _)| *li == layer_idx && p == &pk)
            .map(|(_, _, fr)| *fr)
            .collect();
        match pk.as_str() {
            "position" => match cmd {
                KeyframeMenuAction::Reverse => reverse_track(&mut t.position),
                KeyframeMenuAction::Delete => delete_track_kf(&mut t.position, f),
                action => set_kf_interp(
                    &mut t.position,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    action,
                ),
            },
            "scale" => match cmd {
                KeyframeMenuAction::Reverse => reverse_track(&mut t.scale),
                KeyframeMenuAction::Delete => delete_track_kf(&mut t.scale, f),
                action => set_kf_interp(
                    &mut t.scale,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    action,
                ),
            },
            "rotation" => match cmd {
                KeyframeMenuAction::Reverse => reverse_track(&mut t.rotation),
                KeyframeMenuAction::Delete => delete_track_kf(&mut t.rotation, f),
                action => set_kf_interp(
                    &mut t.rotation,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    action,
                ),
            },
            "opacity" => match cmd {
                KeyframeMenuAction::Reverse => reverse_track(&mut t.opacity),
                KeyframeMenuAction::Delete => delete_track_kf(&mut t.opacity, f),
                action => set_kf_interp(
                    &mut t.opacity,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    action,
                ),
            },
            _ if pk.starts_with("pin_") => {
                let pid = pk.strip_prefix("pin_").unwrap_or("");
                if let Some(pin) = layer.puppet_pins.iter_mut().find(|p| p.id == pid) {
                    match cmd {
                        KeyframeMenuAction::Reverse => reverse_track(&mut pin.position),
                        KeyframeMenuAction::Delete => delete_track_kf(&mut pin.position, f),
                        action => set_kf_interp(
                            &mut pin.position,
                            &if sel_frames.is_empty() {
                                vec![f]
                            } else {
                                sel_frames
                            },
                            action,
                        ),
                    }
                }
            }
            "repeater_copies" | "repeater_position" | "repeater_scale" | "repeater_rotation"
            | "repeater_opacity" => {
                if let Some(repeater) = layer.shape_repeater.as_mut() {
                    macro_rules! apply_repeater {
                        ($track:expr) => {
                            match cmd {
                                KeyframeMenuAction::Reverse => reverse_track($track),
                                KeyframeMenuAction::Delete => delete_track_kf($track, f),
                                action => set_kf_interp(
                                    $track,
                                    &if sel_frames.is_empty() {
                                        vec![f]
                                    } else {
                                        sel_frames.clone()
                                    },
                                    action,
                                ),
                            }
                        };
                    }
                    match pk.as_str() {
                        "repeater_copies" => {
                            if let Some(a) = repeater.copies_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_position" => {
                            if let Some(a) = repeater.position_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_scale" => {
                            if let Some(a) = repeater.scale_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_rotation" => {
                            if let Some(a) = repeater.rotation_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_opacity" => {
                            if let Some(a) = repeater.opacity_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        *project_changed = true;
    }
}
