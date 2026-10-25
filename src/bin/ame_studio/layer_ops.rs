use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_layer_timing(&mut self, input: &str, is_in_point: bool) {
        let Ok(requested) = input.trim().parse::<u32>() else {
            return;
        };
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let max_frame = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        let (next_in, next_out) = if is_in_point {
            let next_in = requested
                .min(max_frame.saturating_sub(1))
                .min(layer.out_frame);
            (next_in, layer.out_frame)
        } else {
            let next_out = requested.clamp(layer.in_frame, max_frame);
            (layer.in_frame, next_out)
        };
        if layer.in_frame != next_in || layer.out_frame != next_out {
            layer.in_frame = next_in;
            layer.out_frame = next_out;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_selected_layer_style(&mut self, style_id: &str) {
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
        match style_id {
            "drop_shadow" => {
                let style = &mut layer.style.drop_shadow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 8.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 12.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.0, 0.0, 0.0, 1.0];
                    }
                }
            }
            "outer_glow" => {
                let style = &mut layer.style.outer_glow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 75.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 20.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 0.45, 0.12, 1.0];
                    }
                }
            }
            "stroke" => {
                let style = &mut layer.style.stroke;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.size <= 0.0 {
                        style.size = 4.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 1.0, 1.0, 1.0];
                    }
                }
            }
            "color_overlay" => {
                let style = &mut layer.style.color_overlay;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 100.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 0.45, 0.12, 1.0];
                    }
                }
            }
            "inner_shadow" => {
                let style = &mut layer.style.inner_shadow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 8.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 12.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.0, 0.0, 0.0, 1.0];
                    }
                }
            }
            "inner_glow" => {
                let style = &mut layer.style.inner_glow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 16.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 1.0, 1.0, 1.0];
                    }
                }
            }
            "satin" => {
                let style = &mut layer.style.satin;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 50.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 12.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 16.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.2, 0.2, 0.35, 1.0];
                    }
                }
            }
            "bevel_emboss" => {
                let style = &mut layer.style.bevel_emboss;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.depth <= 0.0 {
                        style.depth = 3.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 3.0;
                    }
                }
            }
            "gradient_overlay" => {
                let style = &mut layer.style.gradient_overlay;
                style.enabled = !style.enabled;
                if style.enabled && style.opacity <= 0.0 {
                    style.opacity = 100.0;
                }
            }
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_layer_style_parameter(&mut self, parameter: &str, input: &str) {
        let Some((style_id, field)) = parameter.split_once('.') else {
            return;
        };
        let color = field.starts_with("color");
        let numeric = if color {
            None
        } else {
            input
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
        };
        if !color && numeric.is_none() {
            return;
        }
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
        let mut changed = false;
        let mut set_color = |target: &mut [f32; 4]| {
            if let Some(next) = parse_hex_rgb(input, target[3].max(1.0)) {
                if *target != next {
                    *target = next;
                    changed = true;
                }
            }
        };
        macro_rules! set_scalar {
            ($target:expr, $min:expr, $max:expr) => {
                if let Some(value) = numeric {
                    let next = value.clamp($min, $max);
                    if ($target - next).abs() > f32::EPSILON {
                        $target = next;
                        changed = true;
                    }
                }
            };
        }
        match style_id {
            "drop_shadow" => {
                let style = &mut layer.style.drop_shadow;
                if field == "color" {
                    set_color(&mut style.color);
                } else if let Some(value) = numeric {
                    let target = match field {
                        "opacity" => Some((&mut style.opacity, value.clamp(0.0, 100.0))),
                        "angle" => Some((&mut style.angle, value.clamp(-360.0, 360.0))),
                        "distance" => Some((&mut style.distance, value.clamp(0.0, 2000.0))),
                        "size" => Some((&mut style.size, value.clamp(0.0, 500.0))),
                        _ => None,
                    };
                    if let Some((target, next)) = target {
                        if (*target - next).abs() > f32::EPSILON {
                            *target = next;
                            changed = true;
                        }
                    }
                }
            }
            "outer_glow" => {
                let style = &mut layer.style.outer_glow;
                if field == "color" {
                    set_color(&mut style.color);
                } else if let Some(value) = numeric {
                    let target = match field {
                        "opacity" => Some((&mut style.opacity, value.clamp(0.0, 100.0))),
                        "spread" => Some((&mut style.spread, value.clamp(0.0, 100.0))),
                        "size" => Some((&mut style.size, value.clamp(0.0, 500.0))),
                        _ => None,
                    };
                    if let Some((target, next)) = target {
                        if (*target - next).abs() > f32::EPSILON {
                            *target = next;
                            changed = true;
                        }
                    }
                }
            }
            "stroke" => {
                let style = &mut layer.style.stroke;
                if field == "color" {
                    set_color(&mut style.color);
                } else if field == "size" {
                    if let Some(value) = numeric {
                        let next = value.clamp(0.0, 500.0);
                        if (style.size - next).abs() > f32::EPSILON {
                            style.size = next;
                            changed = true;
                        }
                    }
                }
            }
            "color_overlay" => {
                let style = &mut layer.style.color_overlay;
                if field == "color" {
                    set_color(&mut style.color);
                } else if field == "opacity" {
                    set_scalar!(style.opacity, 0.0, 100.0);
                }
            }
            "inner_shadow" => {
                let style = &mut layer.style.inner_shadow;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "distance" => set_scalar!(style.distance, 0.0, 2000.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "inner_glow" => {
                let style = &mut layer.style.inner_glow;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "satin" => {
                let style = &mut layer.style.satin;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "distance" => set_scalar!(style.distance, 0.0, 2000.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "bevel_emboss" => {
                let style = &mut layer.style.bevel_emboss;
                match field {
                    "color_light" => set_color(&mut style.color_light),
                    "color_dark" => set_color(&mut style.color_dark),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "depth" => set_scalar!(style.depth, 0.0, 500.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    "highlight" => set_scalar!(style.highlight, 0.0, 100.0),
                    "shadow" => set_scalar!(style.shadow, 0.0, 100.0),
                    _ => {}
                }
            }
            "gradient_overlay" => {
                let style = &mut layer.style.gradient_overlay;
                match field {
                    "color_start" => set_color(&mut style.color_start),
                    "color_end" => set_color(&mut style.color_end),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "scale" => set_scalar!(style.scale, 1.0, 500.0),
                    _ => {}
                }
            }
            _ => {}
        }
        if changed {
            self.mark_project_changed();
        }
    }

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
            self.align_selected_layer_ids.iter().cloned().collect::<Vec<_>>()
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

    pub(crate) fn distribute_scene_layers(&mut self, mode: kagari_vfx::core::layer_align::DistributeMode) {
        use kagari_vfx::core::layer_align::{apply_to_composition, AlignOperation};

        let selected_ids = if self.align_selected_layer_ids.is_empty() {
            self.selected_layer_id.iter().cloned().collect::<Vec<_>>()
        } else {
            self.align_selected_layer_ids.iter().cloned().collect::<Vec<_>>()
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

    pub(crate) fn add_transform_property_keyframe(&mut self, property: &str) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self.project_model.active_composition_mut().layers.iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked) else {
            return;
        };
        match property {
            "position_2d" => add_current_keyframe(&mut layer.transform.position, frame),
            "anchor_point" => add_current_keyframe(&mut layer.transform.anchor_point, frame),
            "rotation_2d" => add_current_keyframe(&mut layer.transform.rotation, frame),
            "scale_2d" => add_current_keyframe(&mut layer.transform.scale, frame),
            "opacity" => add_current_keyframe(&mut layer.transform.opacity, frame),
            "position_3d" => add_current_keyframe(&mut layer.transform_3d.position, frame),
            "rotation_3d" => add_current_keyframe(&mut layer.transform_3d.rotation, frame),
            "scale_3d" => add_current_keyframe(&mut layer.transform_3d.scale, frame),
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn add_selected_shape_keyframe(&mut self, channel: GraphChannel) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let frame = self.current_frame;
        let changed = self.project_model.active_composition_mut().layers.iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| shape_parameter_track_mut(layer, channel))
            .is_some_and(|track| {
                add_current_keyframe(track, frame);
                true
            });
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn remove_selected_layer(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == id)
        else {
            self.selected_layer_id = None;
            return;
        };
        if layer.locked {
            self.asset_status = "ロック中のレイヤーは削除できません。".to_owned();
            return;
        }
        self.align_selected_layer_ids.remove(&id);
        self.selected_layer_id = None;
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition.layers.iter().position(|layer| layer.id == id) else {
            self.selected_layer_id = None;
            return;
        };
        composition.remove_layer_at(index);
        for layer in &mut composition.layers {
            if layer.parent_id.as_deref() == Some(id.as_str()) {
                layer.parent_id = None;
            }
            if layer.track_matte_layer_id.as_deref() == Some(id.as_str()) {
                layer.track_matte_layer_id = None;
                layer.track_matte = kagari_vfx::core::timeline::TrackMatteMode::None;
            }
        }
        self.collapsed_scene_layers.remove(&id);
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .last()
            .map(|layer| layer.id.clone());
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn request_remove_selected_layer(&mut self) {
        if self.selected_layer_id.as_deref().is_some_and(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .any(|layer| layer.id == id && !layer.locked)
        }) {
            self.layer_remove_confirm_open = true;
        }
    }

    pub(crate) fn move_selected_layer(&mut self, direction: isize) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        self.move_layer_by_id(&id, direction);
    }

    pub(crate) fn move_selected_layer_to_edge(&mut self, direction: isize) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは並べ替えできません。".to_owned();
            return;
        }

        let moved = {
            let layers = &mut self.project_model.active_composition_mut().layers;
            let Some(mut index) = layers.iter().position(|layer| layer.id == id) else {
                return;
            };
            let mut moved = false;
            if direction < 0 {
                while index > 0 && !layers[index - 1].locked {
                    layers.swap(index, index - 1);
                    index -= 1;
                    moved = true;
                }
            } else {
                while index + 1 < layers.len() && !layers[index + 1].locked {
                    layers.swap(index, index + 1);
                    index += 1;
                    moved = true;
                }
            }
            moved
        };

        if moved {
            self.selected_layer_id = Some(id);
            self.sync_selected_layer_controls();
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn move_layer_by_id(&mut self, id: &str, direction: isize) {
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは並べ替えできません。".to_owned();
            return;
        }
        let layers = &mut self.project_model.active_composition_mut().layers;
        let Some(index) = layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let next = (index as isize + direction).clamp(0, layers.len() as isize - 1) as usize;
        if index != next && layers[next].locked {
            self.asset_status = "ロック中のレイヤーをまたいで並べ替えできません。".to_owned();
            return;
        }
        if index != next {
            layers.swap(index, next);
            self.selected_layer_id = Some(id.to_owned());
            self.sync_selected_layer_controls();
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn duplicate_selected_layer(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは複製できません。".to_owned();
            return;
        }
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition.layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let new_id = composition.next_layer_id("layer_copy");
        let mut duplicate = composition.layers[index].clone();
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} copy", duplicate.name);
        duplicate.parent_id = None;
        duplicate.track_matte_layer_id = None;
        duplicate.track_matte = kagari_vfx::core::timeline::TrackMatteMode::None;
        composition.layers.insert(index + 1, duplicate);
        self.selected_layer_id = Some(new_id);
        self.selected_effect_id = None;
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn split_selected_layer_at_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.asset_status = "分割するレイヤーを選択してください。".to_owned();
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition
            .layers
            .iter()
            .position(|layer| layer.id == layer_id)
        else {
            return;
        };
        let original = composition.layers[index].clone();
        if original.locked {
            self.asset_status = "ロック中のレイヤーは分割できません。".to_owned();
            return;
        }
        if original.scene_object.is_some() {
            self.asset_status = "カメラ・ライトレイヤーは分割できません。".to_owned();
            return;
        }
        if frame <= original.in_frame || frame > original.out_frame {
            self.asset_status = "再生位置をレイヤーの範囲内に置いてください。".to_owned();
            return;
        }
        let new_id = composition.next_layer_id("split");
        let mut second = original.clone();
        let original_out = original.out_frame;
        let original_name = original.name.clone();
        composition.layers[index].out_frame = frame - 1;
        second.id = new_id.clone();
        second.name = format!("{} · 分割", original_name);
        second.in_frame = frame;
        second.out_frame = original_out;
        composition.layers.insert(index + 1, second);
        self.selected_layer_id = Some(new_id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("{original_name} を {frame} フレームで分割しました。");
        self.mark_project_changed();
        self.sync_bindings_logged();
    }

    pub(crate) fn set_selected_layer_in_to_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition.layers.iter_mut().find(|l| l.id == layer_id && !l.locked) else {
            return;
        };
        if frame < layer.out_frame {
            layer.in_frame = frame;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn set_selected_layer_out_to_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition.layers.iter_mut().find(|l| l.id == layer_id && !l.locked) else {
            return;
        };
        if frame > layer.in_frame {
            layer.out_frame = frame;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn toggle_scene_group(&mut self, layer_id: &str) {
        let has_children = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.parent_id.as_deref() == Some(layer_id));
        if !has_children {
            return;
        }
        if !self.collapsed_scene_layers.remove(layer_id) {
            self.collapsed_scene_layers.insert(layer_id.to_owned());
        }
        self.sync_bindings_logged();
    }

    pub(crate) fn set_all_scene_layer_visibility(&mut self, visible: bool) {
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .fold(false, |changed, layer| {
                let layer_changed = layer.visible != visible;
                layer.visible = visible;
                changed || layer_changed
            });
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn parent_selected_layer(&mut self) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let layers = &self.project_model.active_composition().layers;
        let query = self.search.trim().to_lowercase();
        let layer_by_id = layers
            .iter()
            .map(|layer| (layer.id.as_str(), layer))
            .collect::<std::collections::HashMap<_, _>>();
        let search_visible = scene_search_visible_layer_ids(layers, &layer_by_id, &query);
        let visible_ids = layers
            .iter()
            .filter(|layer| search_visible.contains(&layer.id))
            .filter(|layer| {
                !query.is_empty()
                    || !scene_layer_hidden_by_collapse(&layer_by_id, layer, &self.collapsed_scene_layers)
            })
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>();
        let Some(index) = visible_ids.iter().position(|id| *id == selected_id) else {
            return;
        };
        let Some(parent_id) = index
            .checked_sub(1)
            .and_then(|previous| visible_ids.get(previous))
            .map(|layer_id| (*layer_id).to_owned())
        else {
            return;
        };
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, Some(parent_id.clone()));
        if changed {
            self.collapsed_scene_layers.remove(&parent_id);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn set_selected_layer_parent(&mut self, parent_id: Option<&str>) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let parent_id = parent_id.map(str::to_owned);
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, parent_id.clone());
        if changed {
            if let Some(parent_id) = parent_id {
                self.collapsed_scene_layers.remove(&parent_id);
            }
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn unparent_selected_layer(&mut self) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, None);
        if changed {
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

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
        let Some(layer) = layers.iter_mut().find(|layer| layer.id == id && !layer.locked) else {
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

    pub(crate) fn add_utility_layer(&mut self, name: &str, layer_type: kagari_vfx::core::timeline::LayerType) {
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_utility_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name.to_owned(),
            layer_type,
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        layer.is_adjustment_layer = matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer
        );
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn add_canvas_layer(
        &mut self,
        name: String,
        layer_type: kagari_vfx::core::timeline::LayerType,
        position: [f64; 2],
    ) {
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_canvas_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name,
            layer_type,
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            position[0] as f32,
            position[1] as f32,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn add_text_at(&mut self, position: [f64; 2]) {
        use kagari_vfx::core::timeline::LayerType;
        self.add_canvas_layer(
            "テキスト".to_owned(),
            LayerType::Text {
                text: "新しいテキスト".to_owned(),
                font_size: 48,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Inter".to_owned(),
                tracking: 0.0,
                leading: 1.2,
                align: 0,
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                text_on_path: false,
            },
            position,
        );
        self.sync_bindings_logged();
    }

    pub(crate) fn cycle_layer_blend_mode(&mut self) {
        use kagari_vfx::core::timeline::BlendMode;
        const MODES: &[BlendMode] = &[
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Add,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::SoftLight,
            BlendMode::HardLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Divide,
            BlendMode::Subtract,
            BlendMode::ColorBurn,
            BlendMode::LinearBurn,
            BlendMode::VividLight,
            BlendMode::ColorDodge,
            BlendMode::LinearDodge,
            BlendMode::Color,
            BlendMode::Hue,
            BlendMode::Saturation,
            BlendMode::Luminosity,
            BlendMode::StencilAlpha,
            BlendMode::StencilLuma,
            BlendMode::SilhouetteAlpha,
            BlendMode::SilhouetteLuma,
            BlendMode::Behind,
            BlendMode::AlphaAdd,
            BlendMode::LinearLight,
        ];
        if let Some(id) = self.selected_layer_id.as_deref() {
            let selected = self.project_model.active_composition_mut().layers
                .iter_mut().find(|layer| layer.id == id && !layer.locked);
            if let Some(layer) = selected {
                let index = MODES.iter().position(|mode| *mode == layer.blend_mode).unwrap_or(0);
                layer.blend_mode = MODES[(index + 1) % MODES.len()];
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn cycle_layer_track_matte(&mut self) {
        use kagari_vfx::core::timeline::TrackMatteMode as Matte;
        const MODES: &[Matte] = &[
            Matte::None,
            Matte::AlphaMatte,
            Matte::AlphaMatteInverted,
            Matte::LumaMatte,
            Matte::LumaMatteInverted,
        ];
        if let Some(id) = self.selected_layer_id.as_deref() {
            let selected = self.project_model.active_composition_mut().layers
                .iter_mut().find(|layer| layer.id == id && !layer.locked);
            if let Some(layer) = selected {
                let index = MODES.iter().position(|mode| *mode == layer.track_matte).unwrap_or(0);
                layer.track_matte = MODES[(index + 1) % MODES.len()];
                if layer.track_matte == Matte::None {
                    layer.track_matte_layer_id = None;
                }
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn select_track_matte_source(&mut self, source_id_opt: Option<&str>) {
        if let (Some(layer_id), Some(source_id)) =
            (self.selected_layer_id.as_deref(), source_id_opt)
        {
            if layer_id != source_id {
                let selected = self.project_model.active_composition_mut().layers
                    .iter_mut().find(|layer| layer.id == layer_id && !layer.locked);
                if let Some(layer) = selected {
                    layer.track_matte_layer_id = Some(source_id.to_owned());
                    self.mark_project_changed();
                }
            }
        }
    }

    pub(crate) fn clear_track_matte_source(&mut self) {
        if let Some(layer_id) = self.selected_layer_id.as_deref() {
            let selected = self.project_model.active_composition_mut().layers
                .iter_mut().find(|layer| layer.id == layer_id && !layer.locked);
            if let Some(layer) = selected {
                if layer.track_matte_layer_id.take().is_some() {
                    self.mark_project_changed();
                }
            }
        }
    }
}

