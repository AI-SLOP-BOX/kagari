use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_scene_camera(&mut self) {
        use kagari_vfx::core::timeline::{Camera3D, Layer, LayerType, SceneObjectRef};
        let composition = self.project_model.active_composition_mut();
        let camera = Camera3D {
            id: composition.next_camera_id(),
            name: format!("カメラ {}", composition.cameras.len() + 1),
            active: false,
            ..Camera3D::default()
        };
        let camera_id = camera.id.clone();
        let camera_name = camera.name.clone();
        let camera_transform = camera.transform.clone();
        composition.cameras.push(camera);
        let camera_index = composition.cameras.len() - 1;
        composition.set_active_camera(Some(camera_index));

        let mut layer = Layer::new(
            composition.next_layer_id("camera"),
            camera_name.clone(),
            LayerType::Null,
            composition.duration_frames,
        );
        layer.is_3d = true;
        layer.transform_3d = camera_transform;
        layer.scene_object = Some(SceneObjectRef::Camera { id: camera_id });
        let layer_id = layer.id.clone();
        composition.add_layer(layer);
        self.selected_layer_id = Some(layer_id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("3Dカメラを追加し、アクティブにしました: {camera_name}");
        self.mark_project_changed();
    }

    pub(crate) fn add_scene_light(&mut self) {
        use kagari_vfx::core::timeline::{Layer, LayerType, Light3D, SceneObjectRef};
        let composition = self.project_model.active_composition_mut();
        let light = Light3D {
            id: composition.next_light_id(),
            name: format!("ライト {}", composition.lights.len() + 1),
            ..Light3D::default()
        };
        let light_id = light.id.clone();
        let light_name = light.name.clone();
        let light_position = light.position.clone();
        composition.lights.push(light);

        let mut layer = Layer::new(
            composition.next_layer_id("light"),
            light_name.clone(),
            LayerType::Null,
            composition.duration_frames,
        );
        layer.is_3d = true;
        layer.transform_3d.position = light_position;
        layer.scene_object = Some(SceneObjectRef::Light { id: light_id });
        let layer_id = layer.id.clone();
        composition.add_layer(layer);
        self.selected_layer_id = Some(layer_id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("3Dライトを追加しました: {light_name}");
        self.mark_project_changed();
    }

    pub(crate) fn selected_scene_camera_id(&self) -> Option<String> {
        let layer_id = self.selected_layer_id.as_deref()?;
        let layer = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)?;
        match layer.scene_object.as_ref()? {
            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => Some(id.clone()),
            _ => None,
        }
    }

    pub(crate) fn add_selected_scene_camera_keyframe(&mut self, channel: GraphChannel) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let frame = self.current_frame;
        let selected_mask_id = self.selected_mask_id.clone();
        let selected_effect_id = self.selected_effect_id.clone();
        let graph_effect_parameter_id = self.graph_effect_parameter_id.clone();
        let composition = self.project_model.active_composition_mut();
        if let Some((layer, Some(camera), light)) = graph_targets_mut(composition, &layer_id) {
            add_graph_keyframe(
                layer,
                Some(camera),
                light,
                selected_mask_id.as_deref(),
                selected_effect_id.as_deref(),
                graph_effect_parameter_id.as_deref(),
                channel,
                frame,
            );
            self.mark_project_changed();
        }
    }

    pub(crate) fn activate_selected_scene_camera(&mut self) {
        let Some(camera_id) = self.selected_scene_camera_id() else {
            return;
        };
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition
            .cameras
            .iter()
            .position(|camera| camera.id == camera_id)
        else {
            return;
        };
        if composition.cameras[index].active {
            return;
        }
        composition.set_active_camera(Some(index));
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_scene_camera(&mut self, property: &str, value: f64) {
        if !value.is_finite() {
            return;
        }
        let Some(camera_id) = self.selected_scene_camera_id() else {
            return;
        };
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let frame = self.current_frame;
        let camera = self
            .project_model
            .active_composition_mut()
            .cameras
            .iter_mut()
            .find(|camera| camera.id == camera_id);
        let Some(camera) = camera else {
            return;
        };
        let value = value as f32;
        match property {
            "fov" => camera.set_fov_at(frame, value),
            "focus" => camera.set_focus_distance_at(frame, value.clamp(0.0, 100_000.0)),
            "aperture" => camera.set_aperture_at(frame, value.clamp(0.0, 500.0)),
            "dof_blur" => camera.set_dof_max_blur_at(frame, value.clamp(1.0, 64.0)),
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_scene_camera_dof(&mut self) {
        let Some(camera_id) = self.selected_scene_camera_id() else {
            return;
        };
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let frame = self.current_frame;
        let Some(camera) = self
            .project_model
            .active_composition_mut()
            .cameras
            .iter_mut()
            .find(|camera| camera.id == camera_id)
        else {
            return;
        };
        let enabled = !camera.dof_enabled_at(frame);
        camera.set_dof_enabled_at(frame, enabled);
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_scene_light_intensity(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let frame = self.current_frame;
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        let value = (value as f32).clamp(0.0, 500.0);
        if (light.intensity_at(frame) - value).abs() <= f32::EPSILON {
            return;
        }
        light.set_intensity_at(frame, value);
        self.mark_project_changed();
    }

    pub(crate) fn keyframe_selected_scene_light_intensity(&mut self) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let frame = self.current_frame;
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        let value = light.intensity_at(frame);
        light.set_intensity_at(frame, value);
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_scene_light_shadows(&mut self) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        light.casts_shadows = !light.casts_shadows;
        self.mark_project_changed();
    }

    pub(crate) fn cycle_selected_scene_light_type(&mut self) {
        use kagari_vfx::core::timeline::LightType;
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        light.light_type = match &light.light_type {
            LightType::Ambient => LightType::Point,
            LightType::Point => LightType::Spot {
                cone_angle_deg: 90.0,
                cone_feather_pct: 50.0,
            },
            LightType::Spot { .. } => LightType::Parallel,
            LightType::Parallel => LightType::Ambient,
        };
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_scene_light_color(&mut self, input: &str) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        let Some(color) = parse_hex_rgb(input, light.color[3]) else {
            return;
        };
        if light.color == color {
            return;
        }
        light.color = color;
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_scene_light_property(&mut self, property: &str, value: f64) {
        use kagari_vfx::core::timeline::LightType;
        if !value.is_finite() {
            return;
        }
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let light_id = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| match layer.scene_object.as_ref() {
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { id }) => Some(id.clone()),
                _ => None,
            });
        let Some(light_id) = light_id else {
            return;
        };
        let Some(light) = self
            .project_model
            .active_composition_mut()
            .lights
            .iter_mut()
            .find(|light| light.id == light_id)
        else {
            return;
        };
        let value = value as f32;
        let changed = match property {
            "cone_angle" => {
                if let LightType::Spot { cone_angle_deg, .. } = &mut light.light_type {
                    let next = value.clamp(0.0, 180.0);
                    let changed = (*cone_angle_deg - next).abs() > f32::EPSILON;
                    *cone_angle_deg = next;
                    changed
                } else {
                    false
                }
            }
            "cone_feather" => {
                if let LightType::Spot {
                    cone_feather_pct, ..
                } = &mut light.light_type
                {
                    let next = value.clamp(0.0, 100.0);
                    let changed = (*cone_feather_pct - next).abs() > f32::EPSILON;
                    *cone_feather_pct = next;
                    changed
                } else {
                    false
                }
            }
            "falloff" => {
                let next = value.clamp(0.0, 2.0);
                let changed = (light.falloff - next).abs() > f32::EPSILON;
                light.falloff = next;
                changed
            }
            "shadow_darkness" => {
                let next = value.clamp(0.0, 100.0);
                let changed = (light.shadow_darkness - next).abs() > f32::EPSILON;
                light.shadow_darkness = next;
                changed
            }
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }
}
