use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_composition_setting(&mut self, field: &str, input: &str) {
        if field == "duration" {
            let Ok(seconds) = input.trim().parse::<f64>() else {
                self.composition_settings_status = "尺は0.01〜3600秒の数値で入力してください。".to_owned();
                return;
            };
            if !seconds.is_finite() || !(0.01..=3_600.0).contains(&seconds) {
                self.composition_settings_status = "尺は0.01〜3600秒の範囲で入力してください。".to_owned();
                return;
            }
            let fps = self.project_model.active_composition().fps.max(1);
            let next_duration_frames = (seconds * f64::from(fps))
                .round()
                .clamp(1.0, f64::from(u32::MAX)) as u32;
            if self.project_model.active_composition().duration_frames == next_duration_frames {
                self.composition_settings_status.clear();
                return;
            }
            self.composition_settings_status.clear();
            let composition = self.project_model.active_composition_mut();
            composition.duration_frames = next_duration_frames;
            let duration_frames = composition.duration_frames;
            self.project_fps = composition.fps.max(1);
            self.current_frame = self.current_frame.min(duration_frames.saturating_sub(1));
            self.current_time = format_timecode(self.current_frame, self.project_fps);
            self.sync_selected_layer_controls();
            self.mark_project_changed();
            return;
        }
        let Ok(value) = input.trim().parse::<u32>() else {
            self.composition_settings_status = match field {
                "width" | "height" => "幅と高さは1〜16384の整数で入力してください。".to_owned(),
                "fps" => "fpsは1〜240の整数で入力してください。".to_owned(),
                _ => "数値を確認してください。".to_owned(),
            };
            return;
        };
        let (minimum, maximum, label) = match field {
            "width" => (1, 16_384, "幅"),
            "height" => (1, 16_384, "高さ"),
            "fps" => (1, 240, "fps"),
            _ => return,
        };
        if !(minimum..=maximum).contains(&value) {
            self.composition_settings_status = format!("{label}は{minimum}〜{maximum}の範囲で入力してください。");
            return;
        }
        let unchanged = match field {
            "width" => self.project_model.active_composition().width == value,
            "height" => self.project_model.active_composition().height == value,
            "fps" => self.project_model.active_composition().fps == value,
            _ => false,
        };
        if unchanged {
            self.composition_settings_status.clear();
            return;
        }
        self.composition_settings_status.clear();
        let composition = self.project_model.active_composition_mut();
        match field {
            "width" => {
                let old_w = composition.width;
                let old_h = composition.height;
                composition.resize_and_remap(value, old_h, old_w, old_h);
            }
            "height" => {
                let old_w = composition.width;
                let old_h = composition.height;
                composition.resize_and_remap(old_w, value, old_w, old_h);
            }
            "fps" => {
                let old_fps = composition.fps.max(1);
                let duration_seconds =
                    f64::from(composition.duration_frames.max(1)) / f64::from(old_fps);
                let start_timecode_seconds =
                    f64::from(composition.start_timecode_frames) / f64::from(old_fps);
                composition.fps = value;
                composition.duration_frames = (duration_seconds * f64::from(composition.fps))
                    .round()
                    .clamp(1.0, f64::from(u32::MAX))
                    as u32;
                composition.start_timecode_frames =
                    (start_timecode_seconds * f64::from(composition.fps))
                        .round()
                        .clamp(0.0, f64::from(u32::MAX)) as u32;
            }
            _ => return,
        }
        let duration_frames = composition.duration_frames;
        self.project_fps = composition.fps.max(1);
        self.current_frame = self.current_frame.min(duration_frames.saturating_sub(1));
        self.current_time = format_timecode(self.current_frame, self.project_fps);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn update_composition_start_timecode(&mut self, input: &str) {
        let fps = self.project_model.active_composition().fps.max(1);
        let Some(start_frame) = parse_timecode(input.trim(), fps) else {
            self.composition_settings_status =
                "開始タイムコードは HH:MM:SS:FF 形式で入力してください。FF は fps 未満です。"
                    .to_owned();
            return;
        };
        self.composition_settings_status.clear();
        if self.project_model.active_composition().start_timecode_frames == start_frame {
            return;
        }
        self.project_model
            .active_composition_mut()
            .start_timecode_frames = start_frame;
        self.mark_project_changed();
    }

    pub(crate) fn set_composition_preset(&mut self, width: u32, height: u32) {
        self.composition_settings_status.clear();
        let composition = self.project_model.active_composition_mut();
        let size = (width.clamp(1, 16_384), height.clamp(1, 16_384));
        if (composition.width, composition.height) == size {
            return;
        }
        let old_w = composition.width;
        let old_h = composition.height;
        composition.resize_and_remap(size.0, size.1, old_w, old_h);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn update_composition_float_setting(&mut self, field: &str, input: &str) {
        let Ok(value) = input.trim().parse::<f32>() else {
            self.composition_settings_status = "数値を確認してください。".to_owned();
            return;
        };
        if !value.is_finite() {
            self.composition_settings_status = "有限の数値を入力してください。".to_owned();
            return;
        }
        let (minimum, maximum, label) = match field {
            "pixel_aspect" => (0.1, 10.0, "ピクセル比"),
            "shutter_angle" => (0.0, 720.0, "シャッター角"),
            "shutter_phase" => (-360.0, 360.0, "シャッター位相"),
            _ => return,
        };
        if !(minimum..=maximum).contains(&value) {
            self.composition_settings_status = format!("{label}は{minimum}〜{maximum}の範囲で入力してください。");
            return;
        }
        self.composition_settings_status.clear();
        let composition = self.project_model.active_composition_mut();
        let target = match field {
            "pixel_aspect" => &mut composition.pixel_aspect,
            "shutter_angle" => &mut composition.motion_blur_shutter_angle,
            "shutter_phase" => &mut composition.motion_blur_shutter_phase,
            _ => return,
        };
        let next = match field {
            "pixel_aspect" => value,
            "shutter_angle" => value,
            "shutter_phase" => value,
            _ => return,
        };
        if (*target - next).abs() <= f32::EPSILON {
            return;
        }
        *target = next;
        self.mark_project_changed();
    }

    pub(crate) fn update_composition_background(&mut self, input: &str) {
        let composition = self.project_model.active_composition_mut();
        let Some(color) = parse_hex_rgb(input, composition.background_color[3]) else {
            self.composition_settings_status = "背景色は #RRGGBB 形式で入力してください。".to_owned();
            return;
        };
        self.composition_settings_status.clear();
        if composition.background_color != color {
            composition.background_color = color;
            self.mark_project_changed();
        }
    }

    pub(crate) fn set_composition_proxy_resolution(
        &mut self,
        resolution: kagari_vfx::core::proxy::ProxyResolution,
    ) {
        let proxy = &mut self.project_model.active_composition_mut().comp_proxy;
        if proxy.global_resolution != resolution {
            proxy.global_resolution = resolution;
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_composition_proxy_preview(&mut self) {
        let proxy = &mut self.project_model.active_composition_mut().comp_proxy;
        proxy.active_in_preview = !proxy.active_in_preview;
        self.mark_project_changed();
    }

    pub(crate) fn toggle_composition_render_option(&mut self, linear_blend: bool) {
        let composition = self.project_model.active_composition_mut();
        if linear_blend {
            composition.blend_linear = !composition.blend_linear;
        } else {
            composition.dither_output = !composition.dither_output;
        }
        self.mark_project_changed();
    }

    pub(crate) fn cycle_composition_bit_depth(&mut self) {
        let composition = self.project_model.active_composition_mut();
        composition.bit_depth = composition.bit_depth.next();
        self.mark_project_changed();
    }

    pub(crate) fn cycle_composition_working_space(&mut self) {
        use kagari_vfx::core::color_science::WorkingColorSpace;
        let composition = self.project_model.active_composition_mut();
        let index = WorkingColorSpace::ALL
            .iter()
            .position(|space| *space == composition.working_color_space)
            .unwrap_or(0);
        composition.working_color_space =
            WorkingColorSpace::ALL[(index + 1) % WorkingColorSpace::ALL.len()];
        self.mark_project_changed();
    }

    pub(crate) fn switch_composition(&mut self, direction: i32) {
        let count = self.project_model.compositions.len();
        if count < 2 {
            return;
        }
        let current = self.project_model.active_composition_idx.min(count - 1);
        let next = if direction < 0 {
            (current + count - 1) % count
        } else {
            (current + 1) % count
        };
        self.activate_composition(next);
    }

    pub(crate) fn activate_composition(&mut self, index: usize) {
        let count = self.project_model.compositions.len();
        if index >= count || index == self.project_model.active_composition_idx {
            return;
        }
        self.project_model.active_composition_idx = index;
        self.playback_range = None;
        self.collapsed_scene_layers.clear();
        self.align_selected_layer_ids.clear();
        self.reset_active_composition_view_state();
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .first()
            .map(|layer| layer.id.clone());
        self.selected_effect_id = None;
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn reset_active_composition_view_state(&mut self) {
        self.current_frame = 0;
        self.project_fps = self.project_model.active_composition().fps.max(1);
        self.current_time = format_timecode(0, self.project_fps);
        self.timeline_zoom = 1.0;
        self.timeline_start_frame = 0.0;
        self.timeline_snap_frame = None;
        self.timeline_clip_drag = None;
        self.graph_key_drag = None;
        self.timeline_scrubbing = false;
        self.selected_marker_frame = None;
        self.selected_layer_marker_frame = None;
        self.is_playing = false;
        self.playback_anchor = None;
        self.viewport_mapping = None;
        self.viewport_drag = None;
        self.mask_vertex_drag = None;
        self.viewport_pan_drag = None;
        self.viewport_pen_start = None;
    }

    pub(crate) fn open_project_composition_asset(&mut self, asset_id: &str) {
        let comp_idx = self.project_model.assets.iter().find_map(|item| {
            if item.id != asset_id {
                return None;
            }
            match &item.item_type {
                kagari_vfx::core::timeline::ProjectItemType::Composition { comp_idx } => {
                    Some(*comp_idx)
                }
                _ => None,
            }
        });
        if let Some(index) = comp_idx {
            self.activate_composition(index);
        }
    }

    pub(crate) fn add_composition(&mut self) {
        let active = self.project_model.active_composition();
        let width = active.width;
        let height = active.height;
        let fps = active.fps.max(1);
        let duration_frames = active.duration_frames.max(1);
        let mut index = self.project_model.compositions.len() + 1;
        while self
            .project_model
            .compositions
            .iter()
            .any(|composition| composition.name == format!("コンポジション {index}"))
        {
            index += 1;
        }
        let id = self.project_model.next_composition_id("ameui_comp");
        let composition_index = self.project_model.compositions.len();
        self.project_model
            .compositions
            .push(kagari_vfx::core::timeline::Composition::new(
                id,
                format!("コンポジション {index}"),
                width,
                height,
                fps,
                duration_frames,
            ));
        let asset_id = self.project_model.next_project_item_id("item_comp");
        let mut composition_asset = kagari_vfx::core::timeline::ProjectItem::new(
            asset_id,
            format!("コンポジション {index}"),
            kagari_vfx::core::timeline::ProjectItemType::Composition {
                comp_idx: composition_index,
            },
        );
        composition_asset.parent_folder = self.asset_folder_id.clone();
        self.project_model.assets.push(composition_asset);
        self.project_model.active_composition_idx = self.project_model.compositions.len() - 1;
        self.reset_active_composition_view_state();
        self.selected_layer_id = None;
        self.selected_effect_id = None;
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn duplicate_active_composition(&mut self) {
        if self.project_model.compositions.is_empty() {
            self.asset_status = "複製するコンポジションがありません。".to_owned();
            return;
        }
        let source_index = self
            .project_model
            .active_composition_idx
            .min(self.project_model.compositions.len().saturating_sub(1));
        let source_name = self.project_model.compositions[source_index].name.clone();
        let root_id = self.project_model.next_composition_id("ameui_comp_copy");
        let mut cloned = match clone_composition_asset_tree(
            &self.project_model.compositions,
            source_index,
            &root_id,
        ) {
            Ok(compositions) => compositions,
            Err(reason) => {
                self.asset_status = reason.to_owned();
                return;
            }
        };

        let base_name = format!("{source_name} copy");
        let mut name = base_name.clone();
        let mut suffix = 2;
        while self
            .project_model
            .compositions
            .iter()
            .any(|composition| composition.name == name)
        {
            name = format!("{base_name} {suffix}");
            suffix += 1;
        }
        let Some(root) = cloned.first_mut() else {
            self.asset_status = "コンポジションを複製できませんでした。".to_owned();
            return;
        };
        root.name = name.clone();

        let root_index = self.project_model.compositions.len();
        self.project_model.compositions.extend(cloned);
        let asset_id = self.project_model.next_project_item_id("item_comp");
        let mut composition_asset = kagari_vfx::core::timeline::ProjectItem::new(
            asset_id,
            name.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Composition {
                comp_idx: root_index,
            },
        );
        composition_asset.parent_folder = self.asset_folder_id.clone();
        self.project_model.assets.push(composition_asset);
        self.project_model.active_composition_idx = root_index;
        self.reset_active_composition_view_state();
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .first()
            .map(|layer| layer.id.clone());
        self.selected_effect_id = None;
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.sync_selected_layer_controls();
        self.asset_status = format!("コンポジションを複製しました: {name}");
        self.mark_project_changed();
    }

    pub(crate) fn remove_active_composition(&mut self) {
        let count = self.project_model.compositions.len();
        if count <= 1 {
            return;
        }
        let index = self.project_model.active_composition_idx.min(count - 1);
        let removed_id = self.project_model.compositions[index].id.clone();
        self.project_model.compositions.remove(index);
        let removed_precomp_refs =
            remove_precomp_references(&mut self.project_model.compositions, &removed_id);
        self.project_model.assets.retain_mut(|item| {
            let kagari_vfx::core::timeline::ProjectItemType::Composition { comp_idx } =
                &mut item.item_type
            else {
                return true;
            };
            if *comp_idx == index {
                return false;
            }
            if *comp_idx > index {
                *comp_idx -= 1;
            }
            true
        });
        self.project_model.active_composition_idx = index.min(count - 2);
        self.asset_status = if removed_precomp_refs > 0 {
            format!("コンポジションを削除し、参照していたプリコンポーズを {removed_precomp_refs} 件削除しました。")
        } else {
            "コンポジションを削除しました。".to_owned()
        };
        self.reset_active_composition_view_state();
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .first()
            .map(|layer| layer.id.clone());
        self.selected_effect_id = None;
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn request_remove_active_composition(&mut self) {
        if self.project_model.compositions.len() > 1 {
            self.composition_remove_confirm_open = true;
        }
    }

    pub(crate) fn rename_active_composition(&mut self, name: &str) {
        let name = name.trim().chars().take(128).collect::<String>();
        if name.is_empty() {
            return;
        }
        let comp_idx = self.project_model.active_composition_idx;
        let changed = {
            let composition = self.project_model.active_composition_mut();
            if composition.name == name {
                false
            } else {
                composition.name = name.clone();
                true
            }
        };
        if changed {
            for item in &mut self.project_model.assets {
                if matches!(
                    &item.item_type,
                    kagari_vfx::core::timeline::ProjectItemType::Composition { comp_idx: item_idx }
                        if *item_idx == comp_idx
                ) {
                    item.name = name.clone();
                }
            }
            self.mark_project_changed();
        }
    }

    pub(crate) fn place_project_composition_asset(&mut self, comp_idx: usize, name: String) {
        let nested_id = self.project_model.next_composition_id("ameui_nested_comp");
        let mut nested_compositions = match clone_composition_asset_tree(
            &self.project_model.compositions,
            comp_idx,
            &nested_id,
        ) {
            Ok(compositions) => compositions,
            Err(reason) => {
                self.asset_status = reason.to_owned();
                return;
            }
        };
        let nested_id = nested_compositions[0].id.clone();
        let (duration, width, height, layer_id) = {
            let composition = self.project_model.active_composition();
            (
                composition.duration_frames,
                composition.width,
                composition.height,
                composition.next_layer_id("ameui_precomp"),
            )
        };
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            layer_id.clone(),
            name.clone(),
            kagari_vfx::core::timeline::LayerType::PreComp {
                comp_id: nested_id.clone(),
            },
            duration,
        );
        layer.transform.position =
            kagari_vfx::core::timeline::Animatable::new_constant([
                width as f32 * 0.5,
                height as f32 * 0.5,
            ]);
        let composition = self.project_model.active_composition_mut();
        composition.sub_compositions.append(&mut nested_compositions);
        composition.add_layer(layer);
        self.selected_layer_id = Some(layer_id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("コンポジションをプリコンポーズとして配置しました: {name}");
        self.mark_project_changed();
    }
}
