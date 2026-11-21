use super::*;

impl ProductionDocument {
    pub fn add_sample_automation_binding<I>(
        &mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        points: I,
        input_range: (f64, f64),
        output_range: (f64, f64),
    ) -> Result<(), String>
    where
        I: IntoIterator<Item = (i64, f64)>,
    {
        if self.bindings.len() >= Self::MAX_BINDINGS {
            return Err("too many automation bindings".into());
        }
        let curve = AutomationCurve::from_sample_points(points, self.audio.sample_rate)
            .map_err(str::to_owned)?;
        let binding = AutomationBinding {
            source: source.into(),
            target: target.into(),
            curve,
            input_min: input_range.0,
            input_max: input_range.1,
            output_min: output_range.0,
            output_max: output_range.1,
        };
        binding.validate().map_err(str::to_owned)?;
        self.bindings.push(binding);
        Ok(())
    }

    /// Validates every cross-domain lane before save or headless rendering.
    pub fn validate_bindings(&self) -> Result<(), String> {
        if self.bindings.len() > Self::MAX_BINDINGS {
            return Err("too many automation bindings".into());
        }
        for (index, binding) in self.bindings.iter().enumerate() {
            binding
                .validate()
                .map_err(|error| format!("binding {index}: {error}"))?;
        }
        Ok(())
    }

    pub fn evaluate_bindings(&self, source_values: &HashMap<String, f64>) -> HashMap<String, f64> {
        let mut targets = HashMap::new();
        for binding in &self.bindings {
            if let Some(source_value) = source_values.get(&binding.source) {
                if let Some(value) = binding.map_value(*source_value) {
                    targets.insert(binding.target.clone(), value);
                }
            }
        }
        targets
    }

    /// Evaluate the document-owned automation curves at a shared timeline time.
    /// Live source values should use `evaluate_bindings`; this path is for
    /// deterministic playback, preview, and headless rendering.
    pub fn evaluate_bindings_at_time(
        &self,
        time: crate::core::unified_time::Time,
    ) -> HashMap<String, f64> {
        self.bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .evaluate(time)
                    .filter(|value| value.is_finite())
                    .map(|value| (binding.target.clone(), value))
            })
            .collect()
    }

    /// Build the render snapshot for a frame with document-owned bindings applied.
    /// The source document is never mutated, so preview and headless rendering can
    /// safely evaluate the same production state in parallel.
    pub fn composition_for_frame(
        &self,
        composition_index: usize,
        frame: u32,
    ) -> Option<Composition> {
        let composition = self.project.compositions.get(composition_index)?;
        let rate = FrameRate::new(composition.fps.max(1), 1)?;
        let time = crate::core::unified_time::Time::from_frame(i64::from(frame), rate);
        let targets = self.evaluate_bindings_at_time(time);
        self.composition_with_targets(composition_index, frame, &targets)
    }

    /// Build a frame snapshot using live Audio/MIDI source values. Sources not
    /// present in `source_values` continue to use their saved automation curve.
    pub fn composition_for_frame_with_sources(
        &self,
        composition_index: usize,
        frame: u32,
        source_values: &HashMap<String, f64>,
    ) -> Option<Composition> {
        let composition = self.project.compositions.get(composition_index)?;
        let rate = FrameRate::new(composition.fps.max(1), 1)?;
        let time = crate::core::unified_time::Time::from_frame(i64::from(frame), rate);
        crate::core::expression_engine::set_audio_from_binding_sources(source_values);
        let mut targets = self.evaluate_bindings_at_time(time);
        targets.extend(self.evaluate_bindings(source_values));
        self.composition_with_targets(composition_index, frame, &targets)
    }

    fn composition_with_targets(
        &self,
        composition_index: usize,
        frame: u32,
        targets: &HashMap<String, f64>,
    ) -> Option<Composition> {
        let composition = self.project.compositions.get(composition_index)?;
        let mut snapshot = Self::new(Project {
            compositions: vec![composition.clone()],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: self.project.use_gpu_compute,
        });
        snapshot.apply_binding_targets_at_frame_in_composition(targets, frame, 0);
        snapshot.project.compositions.pop()
    }

    pub fn apply_binding_targets_at_frame(
        &mut self,
        targets: &HashMap<String, f64>,
        frame: u32,
    ) -> usize {
        self.apply_binding_targets_at_frame_scoped(targets, frame, None)
    }

    fn apply_binding_targets_at_frame_in_composition(
        &mut self,
        targets: &HashMap<String, f64>,
        frame: u32,
        composition_index: usize,
    ) -> usize {
        self.apply_binding_targets_at_frame_scoped(targets, frame, Some(composition_index))
    }

    fn apply_binding_targets_at_frame_scoped(
        &mut self,
        targets: &HashMap<String, f64>,
        frame: u32,
        scoped_composition: Option<usize>,
    ) -> usize {
        let mut applied = 0;
        for (target, value) in targets {
            if target.starts_with("vfx.comp.") && target.contains(".camera.") {
                if let Some(camera_path) = target.strip_prefix("vfx.comp.") {
                    if value.is_finite() {
                        for (composition_index, composition) in
                            self.project.compositions.iter_mut().enumerate()
                        {
                            if scoped_composition.is_some_and(|index| index != composition_index) {
                                continue;
                            }
                            let prefix = format!("{}.", composition.id);
                            let Some(property) = camera_path
                                .strip_prefix(&prefix)
                                .and_then(|path| path.strip_prefix("camera."))
                            else {
                                continue;
                            };
                            let camera = composition.resolve_camera_mut();
                            let camera_value = *value as f32;
                            match property {
                                "fov" | "fov_degrees" => {
                                    let value = camera_value.clamp(1.0, 179.0);
                                    let legacy_value = camera.fov_degrees;
                                    camera
                                        .fov_animation
                                        .get_or_insert_with(|| {
                                            Animatable::new_constant(legacy_value)
                                        })
                                        .set_keyframe(Keyframe::new(
                                            frame,
                                            value,
                                            InterpolationType::Linear,
                                        ));
                                    camera.fov_degrees = value;
                                    applied += 1;
                                }
                                "focus_distance" => {
                                    let value = camera_value.max(0.0);
                                    let legacy_value = camera.focus_distance;
                                    camera
                                        .focus_distance_animation
                                        .get_or_insert_with(|| {
                                            Animatable::new_constant(legacy_value)
                                        })
                                        .set_keyframe(Keyframe::new(
                                            frame,
                                            value,
                                            InterpolationType::Linear,
                                        ));
                                    camera.focus_distance = value;
                                    applied += 1;
                                }
                                "aperture" => {
                                    let value = camera_value.max(0.0);
                                    let legacy_value = camera.aperture;
                                    camera
                                        .aperture_animation
                                        .get_or_insert_with(|| {
                                            Animatable::new_constant(legacy_value)
                                        })
                                        .set_keyframe(Keyframe::new(
                                            frame,
                                            value,
                                            InterpolationType::Linear,
                                        ));
                                    camera.aperture = value;
                                    applied += 1;
                                }
                                "dof_enabled" => {
                                    camera.set_dof_enabled_at(frame, camera_value >= 0.5);
                                    applied += 1;
                                }
                                "dof_max_blur" => {
                                    camera.set_dof_max_blur_at(frame, camera_value);
                                    applied += 1;
                                }
                                "position.x" | "position.y" | "position.z" => {
                                    let mut position = camera.transform.position.evaluate(frame);
                                    let axis = match property.as_bytes().last().copied() {
                                        Some(b'x') => 0,
                                        Some(b'y') => 1,
                                        _ => 2,
                                    };
                                    position[axis] = camera_value;
                                    camera.transform.position.set_keyframe(Keyframe::new(
                                        frame,
                                        position,
                                        InterpolationType::Linear,
                                    ));
                                    applied += 1;
                                }
                                "rotation.x" | "rotation.y" | "rotation.z" => {
                                    let mut rotation = camera.transform.rotation.evaluate(frame);
                                    let axis = match property.as_bytes().last().copied() {
                                        Some(b'x') => 0,
                                        Some(b'y') => 1,
                                        _ => 2,
                                    };
                                    rotation[axis] = camera_value;
                                    camera.transform.rotation.set_keyframe(Keyframe::new(
                                        frame,
                                        rotation,
                                        InterpolationType::Linear,
                                    ));
                                    applied += 1;
                                }
                                "scale.x" | "scale.y" | "scale.z" => {
                                    let mut scale = camera.transform.scale.evaluate(frame);
                                    let axis = match property.as_bytes().last().copied() {
                                        Some(b'x') => 0,
                                        Some(b'y') => 1,
                                        _ => 2,
                                    };
                                    scale[axis] = camera_value;
                                    camera.transform.scale.set_keyframe(Keyframe::new(
                                        frame,
                                        scale,
                                        InterpolationType::Linear,
                                    ));
                                    applied += 1;
                                }
                                _ => {}
                            }
                            break;
                        }
                    }
                }
                continue;
            }
            let legacy_layer_path = target.strip_prefix("vfx.layer.");
            let composition_target = target.strip_prefix("vfx.comp.");
            if legacy_layer_path.is_none() && composition_target.is_none() {
                continue;
            }
            if !value.is_finite() {
                continue;
            }
            for (composition_index, composition) in self.project.compositions.iter_mut().enumerate()
            {
                if scoped_composition.is_some_and(|index| index != composition_index) {
                    continue;
                }
                let layer_path = if let Some(path) = legacy_layer_path {
                    path
                } else {
                    let prefix = format!("{}.", composition.id);
                    let Some(path) = composition_target
                        .and_then(|path| path.strip_prefix(&prefix))
                        .and_then(|path| path.strip_prefix("layer."))
                    else {
                        continue;
                    };
                    path
                };
                let Some((layer_index, property)) =
                    composition
                        .layers
                        .iter()
                        .enumerate()
                        .find_map(|(index, layer)| {
                            let prefix = format!("{}.", layer.id);
                            layer_path
                                .strip_prefix(&prefix)
                                .map(|property| (index, property))
                        })
                else {
                    continue;
                };
                if let Some(layer) = composition.layers.get_mut(layer_index) {
                    let value = *value as f32;
                    let changed = match property {
                        "opacity" => {
                            layer.transform.opacity.set_keyframe(Keyframe::new(
                                frame,
                                value.clamp(0.0, 100.0),
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "position.x" | "position.y" => {
                            let mut position = layer.transform.position.evaluate(frame);
                            position[usize::from(property.ends_with('y'))] = value;
                            layer.transform.position.set_keyframe(Keyframe::new(
                                frame,
                                position,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "scale.x" | "scale.y" => {
                            let mut scale = layer.transform.scale.evaluate(frame);
                            scale[usize::from(property.ends_with('y'))] = value;
                            layer.transform.scale.set_keyframe(Keyframe::new(
                                frame,
                                scale,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "anchor_point.x" | "anchor_point.y" => {
                            let mut anchor = layer.transform.anchor_point.evaluate(frame);
                            anchor[usize::from(property.ends_with('y'))] = value;
                            layer.transform.anchor_point.set_keyframe(Keyframe::new(
                                frame,
                                anchor,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "position.z" => {
                            let mut position = layer.transform_3d.position.evaluate(frame);
                            position[2] = value;
                            layer.transform_3d.position.set_keyframe(Keyframe::new(
                                frame,
                                position,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "rotation.x" | "rotation.y" | "rotation.z" => {
                            let mut rotation = layer.transform_3d.rotation.evaluate(frame);
                            let axis = match property.as_bytes().last().copied() {
                                Some(b'x') => 0,
                                Some(b'y') => 1,
                                _ => 2,
                            };
                            rotation[axis] = value;
                            layer.transform_3d.rotation.set_keyframe(Keyframe::new(
                                frame,
                                rotation,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "scale.z" => {
                            let mut scale = layer.transform_3d.scale.evaluate(frame);
                            scale[2] = value;
                            layer.transform_3d.scale.set_keyframe(Keyframe::new(
                                frame,
                                scale,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "rotation" => {
                            layer.transform.rotation.set_keyframe(Keyframe::new(
                                frame,
                                value,
                                InterpolationType::Linear,
                            ));
                            true
                        }
                        "visible" | "enabled" => {
                            layer.visible = value >= 0.5;
                            true
                        }
                        "motion_blur" => {
                            layer.motion_blur = value >= 0.5;
                            true
                        }
                        "solo" => {
                            layer.solo = value >= 0.5;
                            true
                        }
                        "is_3d" => {
                            layer.is_3d = value >= 0.5;
                            true
                        }
                        "effects_enabled" => {
                            layer.effects_enabled = value >= 0.5;
                            true
                        }
                        "preserve_transparency" => {
                            layer.preserve_transparency = value >= 0.5;
                            true
                        }
                        "in_frame" => {
                            let max_frame = layer.out_frame.saturating_sub(1);
                            layer.in_frame = value.max(0.0).round() as u32;
                            layer.in_frame = layer.in_frame.min(max_frame);
                            true
                        }
                        "out_frame" => {
                            let min_frame = layer.in_frame.saturating_add(1);
                            let max_frame = composition.duration_frames;
                            layer.out_frame = value.max(0.0).round() as u32;
                            layer.out_frame = layer.out_frame.clamp(min_frame, max_frame);
                            true
                        }
                        property if property.starts_with("effect.") => {
                            let mut parts = property.split('.');
                            let (_, effect_id, effect_property) =
                                (parts.next(), parts.next(), parts.next());
                            if parts.next().is_some()
                                || effect_id.is_none()
                                || effect_property.is_none()
                            {
                                false
                            } else if let (Some(prop), Some(effect)) = (
                                effect_property,
                                layer
                                    .effects
                                    .iter_mut()
                                    .find(|effect| Some(effect.id.as_str()) == effect_id),
                            ) {
                                match (&mut effect.effect_type, prop) {
                                    (_, "enabled") => {
                                        effect.enabled = value >= 0.5;
                                        true
                                    }
                                    (EffectType::GaussianBlur { blur_radius }, "blur_radius")
                                    | (
                                        EffectType::ColorTint {
                                            intensity: blur_radius,
                                            ..
                                        },
                                        "intensity",
                                    )
                                    | (
                                        EffectType::Vignette {
                                            intensity: blur_radius,
                                            ..
                                        },
                                        "intensity",
                                    )
                                    | (
                                        EffectType::Glow {
                                            intensity: blur_radius,
                                            ..
                                        },
                                        "intensity",
                                    ) => {
                                        blur_radius.set_keyframe(Keyframe::new(
                                            frame,
                                            value.max(0.0),
                                            InterpolationType::Linear,
                                        ));
                                        true
                                    }
                                    (EffectType::DropShadow { opacity, .. }, "opacity")
                                    | (
                                        EffectType::HueSaturation {
                                            saturation: opacity,
                                            ..
                                        },
                                        "saturation",
                                    )
                                    | (EffectType::Levels { gamma: opacity, .. }, "gamma") => {
                                        opacity.set_keyframe(Keyframe::new(
                                            frame,
                                            value,
                                            InterpolationType::Linear,
                                        ));
                                        true
                                    }
                                    _ => false,
                                }
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    if changed {
                        applied += 1;
                    }
                    break;
                }
            }
        }
        applied
    }
}
