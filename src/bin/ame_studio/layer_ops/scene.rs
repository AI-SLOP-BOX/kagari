use super::*;

impl AfterEffectsStudio {
    pub(crate) fn toggle_selected_layer_flag(&mut self, toggle: LayerToggle) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        {
            let flag = match toggle {
                LayerToggle::FrameBlending => &mut layer.frame_blending,
                LayerToggle::MotionBlur => &mut layer.motion_blur,
                LayerToggle::PreserveTransparency => &mut layer.preserve_transparency,
                LayerToggle::Guide => &mut layer.is_guide_layer,
                LayerToggle::Shy => &mut layer.is_shy,
                LayerToggle::Effects => &mut layer.effects_enabled,
            };
            *flag = !*flag;
            self.mark_project_changed();
        }
    }

    pub(crate) fn cycle_selected_auto_orient(&mut self) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let comp_center = {
            let composition = self.project_model.active_composition();
            [
                composition.width as f32 * 0.5,
                composition.height as f32 * 0.5,
            ]
        };
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        {
            layer.auto_orient = match layer.auto_orient {
                kagari_vfx::core::auto_orient::AutoOrientMode::Off => {
                    kagari_vfx::core::auto_orient::AutoOrientMode::OrientAlongPath
                }
                kagari_vfx::core::auto_orient::AutoOrientMode::OrientAlongPath => {
                    kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint {
                        target_point: comp_center,
                    }
                }
                kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint { .. } => {
                    kagari_vfx::core::auto_orient::AutoOrientMode::Off
                }
            };
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_auto_orient_target(&mut self, input: &str, axis: usize) {
        let Ok(value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        {
            if let kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint {
                target_point,
            } = &mut layer.auto_orient
            {
                target_point[axis] = value.clamp(-100000.0, 100000.0);
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn cycle_selected_layer_constraint(&mut self, horizontal: bool) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        if horizontal {
            use kagari_vfx::core::layer_constraints::HorizontalPin;
            const PINS: [HorizontalPin; 4] = [
                HorizontalPin::Left,
                HorizontalPin::Center,
                HorizontalPin::Right,
                HorizontalPin::Scale,
            ];
            let index = PINS
                .iter()
                .position(|pin| *pin == layer.constraints.horizontal)
                .unwrap_or(0);
            layer.constraints.horizontal = PINS[(index + 1) % PINS.len()];
        } else {
            use kagari_vfx::core::layer_constraints::VerticalPin;
            const PINS: [VerticalPin; 4] = [
                VerticalPin::Top,
                VerticalPin::Center,
                VerticalPin::Bottom,
                VerticalPin::Scale,
            ];
            let index = PINS
                .iter()
                .position(|pin| *pin == layer.constraints.vertical)
                .unwrap_or(0);
            layer.constraints.vertical = PINS[(index + 1) % PINS.len()];
        }
        self.mark_project_changed();
    }

    pub(crate) fn set_selected_layer_constraint(&mut self, horizontal: bool, label: &str) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        if horizontal {
            use kagari_vfx::core::layer_constraints::HorizontalPin;
            let next = match label {
                "左固定" => HorizontalPin::Left,
                "中央固定" => HorizontalPin::Center,
                "右固定" => HorizontalPin::Right,
                "幅に比例" => HorizontalPin::Scale,
                _ => return,
            };
            if layer.constraints.horizontal != next {
                layer.constraints.horizontal = next;
                self.mark_project_changed();
            }
        } else {
            use kagari_vfx::core::layer_constraints::VerticalPin;
            let next = match label {
                "上固定" => VerticalPin::Top,
                "中央固定" => VerticalPin::Center,
                "下固定" => VerticalPin::Bottom,
                "高さに比例" => VerticalPin::Scale,
                _ => return,
            };
            if layer.constraints.vertical != next {
                layer.constraints.vertical = next;
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn toggle_alignment_layer_selection(&mut self, layer_id: &str) {
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == layer_id)
            && !self.align_selected_layer_ids.remove(layer_id)
        {
            self.align_selected_layer_ids.insert(layer_id.to_owned());
        }
    }

    pub(crate) fn align_scene_layers(&mut self, mode: kagari_vfx::core::layer_align::AlignMode) {
        use kagari_vfx::core::layer_align::{apply_to_composition, AlignOperation};

        let selected_ids = if self.align_selected_layer_ids.is_empty() {
            self.selected_layer_id.iter().cloned().collect::<Vec<_>>()
        } else {
            self.align_selected_layer_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };
        if selected_ids.is_empty() {
            return;
        }
        let indices = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                (selected_ids.contains(&layer.id) && !layer.locked).then_some(index)
            })
            .collect::<Vec<_>>();
        let changed = apply_to_composition(
            self.project_model.active_composition_mut(),
            &indices,
            self.current_frame,
            AlignOperation::Align(mode),
            self.align_to_composition,
        );
        if changed {
            self.mark_project_changed();
            self.sync_selected_layer_controls();
        }
    }

    pub(crate) fn distribute_scene_layers(
        &mut self,
        mode: kagari_vfx::core::layer_align::DistributeMode,
    ) {
        use kagari_vfx::core::layer_align::{apply_to_composition, AlignOperation};

        let selected_ids = if self.align_selected_layer_ids.is_empty() {
            self.selected_layer_id.iter().cloned().collect::<Vec<_>>()
        } else {
            self.align_selected_layer_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };
        if selected_ids.len() < 3 {
            return;
        }
        let indices = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                (selected_ids.contains(&layer.id) && !layer.locked).then_some(index)
            })
            .collect::<Vec<_>>();
        let changed = apply_to_composition(
            self.project_model.active_composition_mut(),
            &indices,
            self.current_frame,
            AlignOperation::Distribute(mode),
            self.align_to_composition,
        );
        if changed {
            self.mark_project_changed();
            self.sync_selected_layer_controls();
        }
    }

    pub(crate) fn cycle_layer_label_color_for(&mut self, layer_id: &str) {
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
        {
            if layer.locked {
                self.asset_status = format!(
                    "「{}」はロック中のためラベル色を変更できません。",
                    layer.name
                );
                return;
            }
            let colors = kagari_vfx::core::timeline::LabelColor::ALL;
            let index = colors
                .iter()
                .position(|color| *color == layer.label)
                .unwrap_or(0);
            layer.label = colors[(index + 1) % colors.len()];
            self.mark_project_changed();
        }
    }

    pub(crate) fn set_selected_auto_orient(&mut self, label: &str) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let comp_center = {
            let composition = self.project_model.active_composition();
            [
                composition.width as f32 * 0.5,
                composition.height as f32 * 0.5,
            ]
        };
        let next = match label {
            "なし" => kagari_vfx::core::auto_orient::AutoOrientMode::Off,
            "パスに沿う" => kagari_vfx::core::auto_orient::AutoOrientMode::OrientAlongPath,
            "点を向く" => kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint {
                target_point: comp_center,
            },
            _ => return,
        };
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        {
            layer.auto_orient = next;
            self.mark_project_changed();
        }
    }

    pub(crate) fn set_layer_label_color_for(&mut self, layer_id: &str, label: &str) {
        let Some(&next) = kagari_vfx::core::timeline::LabelColor::ALL
            .iter()
            .find(|color| layer_label_color_label(**color) == label)
        else {
            return;
        };
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
        {
            if layer.locked {
                self.asset_status = format!(
                    "「{}」はロック中のためラベル色を変更できません。",
                    layer.name
                );
                return;
            }
            if layer.label != next {
                layer.label = next;
                self.mark_project_changed();
            }
        }
    }
}
