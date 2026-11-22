use super::*;

impl AfterEffectsStudio {
    pub(crate) fn apply_selected_transform_controls(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        {
            use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
            let position = [self.position_x as f32, self.position_y as f32];
            if layer.transform.position.keyframes().is_some() {
                layer.transform.position.set_keyframe(Keyframe::new(
                    frame,
                    position,
                    InterpolationType::Linear,
                ));
            } else {
                layer.transform.position =
                    kagari_vfx::core::timeline::Animatable::new_constant(position);
            }
            let anchor = [self.anchor_x as f32, self.anchor_y as f32];
            if layer.transform.anchor_point.keyframes().is_some() {
                layer.transform.anchor_point.set_keyframe(Keyframe::new(
                    frame,
                    anchor,
                    InterpolationType::Linear,
                ));
            } else {
                layer.transform.anchor_point =
                    kagari_vfx::core::timeline::Animatable::new_constant(anchor);
            }
            let scale = [self.scale_x as f32, self.scale_y as f32];
            if layer.transform.scale.keyframes().is_some() {
                layer.transform.scale.set_keyframe(Keyframe::new(
                    frame,
                    scale,
                    InterpolationType::Linear,
                ));
            } else {
                layer.transform.scale = kagari_vfx::core::timeline::Animatable::new_constant(scale);
            }
            let rotation = self.rotation_degrees.to_radians() as f32;
            if layer.transform.rotation.keyframes().is_some() {
                layer.transform.rotation.set_keyframe(Keyframe::new(
                    frame,
                    rotation,
                    InterpolationType::Linear,
                ));
            } else {
                layer.transform.rotation =
                    kagari_vfx::core::timeline::Animatable::new_constant(rotation);
            }
            if layer.transform.opacity.keyframes().is_some() {
                layer.transform.opacity.set_keyframe(Keyframe::new(
                    frame,
                    self.opacity as f32,
                    InterpolationType::Linear,
                ));
            } else {
                layer.transform.opacity =
                    kagari_vfx::core::timeline::Animatable::new_constant(self.opacity as f32);
            }
            self.mark_project_changed();
        }
    }

    pub(crate) fn add_transform_keyframes_at_current_frame(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let selected_mask_id = self.selected_mask_id.clone();
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let camera_id = composition
            .layers
            .iter()
            .find(|layer| layer.id == id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { id }) => Some(id.clone()),
                _ => None,
            });
        let (layers, cameras) = (&mut composition.layers, &mut composition.cameras);
        let Some(layer) = layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};

        let anchor_point = layer.transform.anchor_point.evaluate(frame);
        let position = layer.transform.position.evaluate(frame);
        let scale = layer.transform.scale.evaluate(frame);
        let rotation = layer.transform.rotation.evaluate(frame);
        let opacity = layer.transform.opacity.evaluate(frame);
        layer.transform.anchor_point.set_keyframe(Keyframe::new(
            frame,
            anchor_point,
            InterpolationType::Linear,
        ));
        layer.transform.position.set_keyframe(Keyframe::new(
            frame,
            position,
            InterpolationType::Linear,
        ));
        layer
            .transform
            .scale
            .set_keyframe(Keyframe::new(frame, scale, InterpolationType::Linear));
        layer.transform.rotation.set_keyframe(Keyframe::new(
            frame,
            rotation,
            InterpolationType::Linear,
        ));
        layer.transform.opacity.set_keyframe(Keyframe::new(
            frame,
            opacity,
            InterpolationType::Linear,
        ));
        if layer.is_3d {
            let position = layer.transform_3d.position.evaluate(frame);
            let rotation = layer.transform_3d.rotation.evaluate(frame);
            let scale = layer.transform_3d.scale.evaluate(frame);
            layer.transform_3d.position.set_keyframe(Keyframe::new(
                frame,
                position,
                InterpolationType::Linear,
            ));
            layer.transform_3d.rotation.set_keyframe(Keyframe::new(
                frame,
                rotation,
                InterpolationType::Linear,
            ));
            layer.transform_3d.scale.set_keyframe(Keyframe::new(
                frame,
                scale,
                InterpolationType::Linear,
            ));
        }
        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                add_current_keyframe(volume, frame);
            }
            kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
                kagari_vfx::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => {
                    add_current_keyframe(width, frame);
                    add_current_keyframe(height, frame);
                    add_current_keyframe(corner_radius, frame);
                }
                kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                    add_current_keyframe(width, frame);
                    add_current_keyframe(height, frame);
                }
                _ => {}
            },
            _ => {}
        }
        if let Some(effect_id) = self.selected_effect_id.as_deref() {
            if let Some(effect) = layer
                .effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
            {
                use kagari_vfx::core::effect_params::ParamRef;
                for (_, parameter) in effect.effect_type.animatable_params() {
                    match parameter {
                        ParamRef::Scalar(track) => add_current_keyframe(track, frame),
                        ParamRef::Vec2(track) => add_current_keyframe(track, frame),
                        ParamRef::Vec3(track) => add_current_keyframe(track, frame),
                        ParamRef::Vec4Color(track) => add_current_keyframe(track, frame),
                    }
                }
            }
        }
        if let Some(mask_id) = selected_mask_id.as_deref() {
            if let Some(mask) = layer.masks.iter_mut().find(|mask| mask.id == mask_id) {
                add_current_keyframe(&mut mask.path.vertices, frame);
                add_current_keyframe(&mut mask.feather, frame);
                add_current_keyframe(&mut mask.opacity, frame);
                add_current_keyframe(&mut mask.expansion, frame);
            }
        }
        if let Some(camera_id) = camera_id {
            if let Some(camera) = cameras.iter_mut().find(|camera| camera.id == camera_id) {
                let fov = camera.fov_at(frame);
                let focus = camera.focus_distance_at(frame);
                let aperture = camera.aperture_at(frame);
                let dof_max_blur = camera.dof_max_blur_at(frame);
                let dof_enabled = camera.dof_enabled_at(frame);
                camera.set_fov_at(frame, fov);
                camera.set_focus_distance_at(frame, focus);
                camera.set_aperture_at(frame, aperture);
                camera.set_dof_max_blur_at(frame, dof_max_blur);
                camera.set_dof_enabled_at(frame, dof_enabled);
            }
        }
        self.mark_project_changed();
        self.sync_selected_layer_controls();
        self.sync_bindings_logged();
    }

    pub(crate) fn ease_selected_layer_keyframes(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let composition = self.project_model.active_composition_mut();
        let camera_id = composition
            .layers
            .iter()
            .find(|layer| layer.id == id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { id }) => Some(id.clone()),
                _ => None,
            });
        let (layers, cameras) = (&mut composition.layers, &mut composition.cameras);
        let Some(layer) = layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        layer.easy_ease_transform();
        if let Some(track) = &mut layer.time_remap {
            track.easy_ease();
        }
        if layer.is_3d {
            layer.transform_3d.position.easy_ease();
            layer.transform_3d.rotation.easy_ease();
            layer.transform_3d.scale.easy_ease();
        }
        for effect in &mut layer.effects {
            for (_, parameter) in effect.effect_type.animatable_params() {
                use kagari_vfx::core::effect_params::ParamRef;
                match parameter {
                    ParamRef::Scalar(track) => track.easy_ease(),
                    ParamRef::Vec2(track) => track.easy_ease(),
                    ParamRef::Vec3(track) => track.easy_ease(),
                    ParamRef::Vec4Color(track) => track.easy_ease(),
                }
            }
        }
        for mask in &mut layer.masks {
            mask.path.vertices.easy_ease();
            mask.feather.easy_ease();
            mask.opacity.easy_ease();
            mask.expansion.easy_ease();
        }
        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume.easy_ease(),
            kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
                kagari_vfx::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => {
                    width.easy_ease();
                    height.easy_ease();
                    corner_radius.easy_ease();
                }
                kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                    width.easy_ease();
                    height.easy_ease();
                }
                kagari_vfx::core::timeline::ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => {
                    points.easy_ease();
                    inner_radius.easy_ease();
                    outer_radius.easy_ease();
                }
                kagari_vfx::core::timeline::ShapeType::Polygon { sides, radius } => {
                    sides.easy_ease();
                    radius.easy_ease();
                }
                kagari_vfx::core::timeline::ShapeType::FreeformBezier { .. } => {}
            },
            _ => {}
        }
        if let Some(camera_id) = camera_id {
            if let Some(camera) = cameras.iter_mut().find(|camera| camera.id == camera_id) {
                for track in [
                    camera.fov_animation.as_mut(),
                    camera.focus_distance_animation.as_mut(),
                    camera.aperture_animation.as_mut(),
                    camera.dof_max_blur_animation.as_mut(),
                ]
                .into_iter()
                .flatten()
                {
                    track.easy_ease();
                }
            }
        }
        self.mark_project_changed();
    }

    pub(crate) fn jump_to_selected_keyframe(&mut self, forward: bool) {
        let Some(layer) = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        }) else {
            return;
        };
        let frames =
            layer_keyframe_frames_for_composition(self.project_model.active_composition(), layer);
        self.current_frame = if forward {
            frames.into_iter().find(|frame| *frame > self.current_frame)
        } else {
            frames
                .into_iter()
                .rev()
                .find(|frame| *frame < self.current_frame)
        }
        .unwrap_or(self.current_frame);
        self.current_time = format_timecode(self.current_frame, self.project_fps);
    }
}
