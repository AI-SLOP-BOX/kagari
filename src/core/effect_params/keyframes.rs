use super::types::{ParamRef, ParamRefRef};
use crate::core::property::Animatable;
use crate::core::timeline::EffectType;

impl EffectType {
    /// Apply one interpolation mode to every existing keyframe in this effect.
    /// Returns whether at least one keyframe changed.
    pub fn set_keyframe_interpolation(
        &mut self,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        self.set_parameter_keyframe_interpolation(None, interpolation)
    }

    pub fn set_parameter_keyframe_interpolation(
        &mut self,
        parameter_name: Option<&str>,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        fn apply<T: Clone>(
            track: &mut Animatable<T>,
            interpolation: crate::core::keyframe::InterpolationType,
        ) -> bool {
            let Some(keyframes) = track.keyframes_mut() else {
                return false;
            };
            let mut changed = false;
            for keyframe in keyframes {
                if keyframe.interpolation != interpolation {
                    keyframe.interpolation = interpolation;
                    changed = true;
                }
            }
            changed
        }

        let mut changed = false;
        for (name, parameter) in self.animatable_params() {
            if parameter_name.is_some_and(|wanted| wanted != name) {
                continue;
            }
            changed |= match parameter {
                ParamRef::Scalar(track) => apply(track, interpolation),
                ParamRef::Vec2(track) => apply(track, interpolation),
                ParamRef::Vec3(track) => apply(track, interpolation),
                ParamRef::Vec4Color(track) => apply(track, interpolation),
            };
        }
        changed
    }

    pub fn set_scalar_parameter_keyframe(
        &mut self,
        parameter_name: &str,
        frame: u32,
        value: f32,
    ) -> bool {
        if !value.is_finite() {
            return false;
        }
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            if let ParamRef::Scalar(track) = parameter {
                let before = track.evaluate(frame);
                let had_key = track
                    .keyframes()
                    .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                if let Some(key) = track
                    .keyframes_mut()
                    .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                {
                    key.value = value;
                } else {
                    track.add_keyframe(crate::core::keyframe::Keyframe::new(
                        frame,
                        value,
                        crate::core::keyframe::InterpolationType::Linear,
                    ));
                }
                return !had_key || before != value;
            }
        }
        false
    }

    pub fn remove_scalar_parameter_keyframe(&mut self, parameter_name: &str, frame: u32) -> bool {
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            if let ParamRef::Scalar(track) = parameter {
                let fallback = track.evaluate(frame);
                if track.keyframes().is_some() {
                    let (removed, became_constant) = {
                        let keyframes = track.keyframes_mut().expect("checked above");
                        let before = keyframes.len();
                        keyframes.retain(|keyframe| keyframe.frame != frame);
                        (
                            keyframes.len() != before,
                            keyframes.is_empty() && keyframes.len() != before,
                        )
                    };
                    if became_constant {
                        *track = Animatable::Constant(fallback);
                    }
                    return removed;
                }
            }
        }
        false
    }

    pub fn set_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        component: usize,
        frame: u32,
        value: f32,
    ) -> bool {
        if !value.is_finite() {
            return false;
        }
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            match parameter {
                ParamRef::Vec2(track) => {
                    if component >= 2 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    if !next.iter().all(|component| component.is_finite()) {
                        return false;
                    }
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Vec3(track) => {
                    if component >= 3 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    if !next.iter().all(|component| component.is_finite()) {
                        return false;
                    }
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Vec4Color(track) => {
                    if component >= 4 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    if !next.iter().all(|component| component.is_finite()) {
                        return false;
                    }
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Scalar(_) => return false,
            }
        }
        false
    }

    pub fn move_scalar_parameter_keyframe(
        &mut self,
        parameter_name: &str,
        from_frame: u32,
        to_frame: u32,
    ) -> bool {
        if from_frame == to_frame {
            return false;
        }
        let Some((value, interpolation)) =
            self.animatable_params_ref()
                .into_iter()
                .find_map(|(name, parameter)| {
                    if name != parameter_name {
                        return None;
                    }
                    match parameter {
                        ParamRefRef::Scalar(track) => track
                            .keyframes()?
                            .iter()
                            .find(|k| k.frame == from_frame)
                            .filter(|k| k.value.is_finite())
                            .map(|k| (k.value, k.interpolation)),
                        _ => None,
                    }
                })
        else {
            return false;
        };
        if !self.remove_scalar_parameter_keyframe(parameter_name, from_frame) {
            return false;
        }
        if !self.set_scalar_parameter_keyframe(parameter_name, to_frame, value) {
            return false;
        }
        self.set_parameter_keyframe_interpolation_at_frame(parameter_name, to_frame, interpolation);
        true
    }

    pub fn move_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        component: usize,
        from_frame: u32,
        to_frame: u32,
    ) -> bool {
        // Vector parameters have one shared keyframe stream. Moving a graph
        // channel therefore moves the whole vector keyframe; the caller then
        // updates the selected component at the destination. This keeps the
        // other channels from disappearing at the source frame.
        if from_frame == to_frame {
            return false;
        }
        macro_rules! move_vector_track {
            ($track:expr, $component_count:expr) => {{
                if component >= $component_count {
                    false
                } else {
                    let source_is_finite = $track.keyframes().is_some_and(|keyframes| {
                        keyframes
                            .iter()
                            .find(|keyframe| keyframe.frame == from_frame)
                            .is_some_and(|keyframe| {
                                keyframe.value.iter().all(|value| value.is_finite())
                            })
                    });
                    source_is_finite && $track.move_keyframe(from_frame, to_frame)
                }
            }};
        }
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            return match parameter {
                ParamRef::Vec2(track) => move_vector_track!(track, 2),
                ParamRef::Vec3(track) => move_vector_track!(track, 3),
                ParamRef::Vec4Color(track) => move_vector_track!(track, 4),
                ParamRef::Scalar(_) => false,
            };
        }
        false
    }

    pub fn set_parameter_keyframe_interpolation_at_frame(
        &mut self,
        parameter_name: &str,
        frame: u32,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        fn apply<T: Clone>(
            track: &mut Animatable<T>,
            frame: u32,
            interpolation: crate::core::keyframe::InterpolationType,
        ) -> bool {
            let Some(keys) = track.keyframes_mut() else {
                return false;
            };
            let Some(key) = keys.iter_mut().find(|key| key.frame == frame) else {
                return false;
            };
            let changed = key.interpolation != interpolation;
            key.interpolation = interpolation;
            changed
        }
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            return match parameter {
                ParamRef::Scalar(track) => apply(track, frame, interpolation),
                ParamRef::Vec2(track) => apply(track, frame, interpolation),
                ParamRef::Vec3(track) => apply(track, frame, interpolation),
                ParamRef::Vec4Color(track) => apply(track, frame, interpolation),
            };
        }
        false
    }

    pub fn set_parameter_keyframe_bezier_at_frame(
        &mut self,
        parameter_name: &str,
        frame: u32,
        points: [f32; 4],
    ) -> bool {
        let interpolation = crate::core::keyframe::InterpolationType::Bezier {
            outgoing: Default::default(),
            incoming: Default::default(),
            custom_bezier: Some(points),
        };
        self.set_parameter_keyframe_interpolation_at_frame(parameter_name, frame, interpolation)
    }

    pub fn remove_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        frame: u32,
    ) -> bool {
        fn remove_key_and_restore_constant<T: Clone + crate::core::property::Interpolate>(
            track: &mut Animatable<T>,
            frame: u32,
        ) -> bool {
            let fallback = track.evaluate(frame);
            let (removed, became_empty) = {
                let Some(keys) = track.keyframes_mut() else {
                    return false;
                };
                let before = keys.len();
                keys.retain(|key| key.frame != frame);
                let removed = keys.len() != before;
                (removed, keys.is_empty() && removed)
            };
            if became_empty {
                *track = Animatable::Constant(fallback);
            }
            removed
        }

        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            let removed = match parameter {
                ParamRef::Vec2(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Vec3(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Vec4Color(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Scalar(_) => false,
            };
            return removed;
        }
        false
    }
}
