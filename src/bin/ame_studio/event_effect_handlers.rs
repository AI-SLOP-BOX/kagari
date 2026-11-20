use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_effect_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("effect_search_changed", Some(ActionValue::Text(value))) => {
                self.effect_search = value;
                true
            }
            ("animation_preset_search_changed", Some(ActionValue::Text(value))) => {
                self.animation_preset_search = value;
                true
            }
            ("apply_animation_preset", _) => {
                if let Some(preset_id) = selected_row_id.as_deref() {
                    self.apply_animation_preset(preset_id);
                };
                true
            }
            ("add_effect_catalog", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.add_effect(effect_id);
                };
                true
            }
            ("add_gaussian_blur", _) => {
                self.add_effect("blur");
                true
            }
            ("add_color_tint", _) => {
                self.add_effect("tint");
                true
            }
            ("add_drop_shadow", _) => {
                self.add_effect("shadow");
                true
            }
            ("add_glow", _) => {
                self.add_effect("glow");
                true
            }
            ("add_vignette", _) => {
                self.add_effect("vignette");
                true
            }
            ("add_hue_saturation", _) => {
                self.add_effect("hue_saturation");
                true
            }
            ("add_levels", _) => {
                self.add_effect("levels");
                true
            }
            ("add_motion_blur", _) => {
                self.add_effect("motion_blur");
                true
            }
            ("add_film_grain", _) => {
                self.add_effect("film_grain");
                true
            }
            ("add_posterize", _) => {
                self.add_effect("posterize");
                true
            }
            ("add_directional_blur", _) => {
                self.add_effect("directional_blur");
                true
            }
            ("add_radial_blur", _) => {
                self.add_effect("radial_blur");
                true
            }
            ("add_sharpen", _) => {
                self.add_effect("sharpen");
                true
            }
            ("add_chroma_key", _) => {
                self.add_effect("chroma_key");
                true
            }
            ("add_curves", _) => {
                self.add_effect("curves");
                true
            }
            ("add_chromatic_aberration", _) => {
                self.add_effect("chromatic_aberration");
                true
            }
            ("add_lens_correction", _) => {
                self.add_effect("lens_correction");
                true
            }
            ("add_fractal_noise", _) => {
                self.add_effect("fractal_noise");
                true
            }
            ("add_glitch_displacement", _) => {
                self.add_effect("glitch_displacement");
                true
            }
            ("add_glow_pro", _) => {
                self.add_effect("glow_pro");
                true
            }
            ("add_white_balance", _) => {
                self.add_effect("white_balance");
                true
            }
            ("add_turbulent_displace", _) => {
                self.add_effect("turbulent_displace");
                true
            }
            ("add_cc_lens", _) => {
                self.add_effect("cc_lens");
                true
            }
            ("add_vortex", _) => {
                self.add_effect("vortex");
                true
            }
            ("add_light_sweep", _) => {
                self.add_effect("light_sweep");
                true
            }
            ("add_luma_key_range", _) => {
                self.add_effect("luma_key_range");
                true
            }
            ("add_tritone", _) => {
                self.add_effect("tritone");
                true
            }
            ("add_displacement_map", _) => {
                self.add_effect("displacement_map");
                true
            }
            ("add_compound_blur", _) => {
                self.add_effect("compound_blur");
                true
            }
            ("add_crt_scanlines", _) => {
                self.add_effect("crt_scanlines");
                true
            }
            ("add_polar_coordinates", _) => {
                self.add_effect("polar_coordinates");
                true
            }
            ("add_wave_warp", _) => {
                self.add_effect("wave_warp");
                true
            }
            ("add_mosaic", _) => {
                self.add_effect("mosaic");
                true
            }
            ("add_emboss", _) => {
                self.add_effect("emboss");
                true
            }
            ("add_bevel_alpha", _) => {
                self.add_effect("bevel_alpha");
                true
            }
            ("add_median_filter", _) => {
                self.add_effect("median_filter");
                true
            }
            ("add_sobel_edges", _) => {
                self.add_effect("sobel_edges");
                true
            }
            ("add_corner_pin", _) => {
                self.add_effect("corner_pin");
                true
            }
            ("add_colorama", _) => {
                self.add_effect("colorama");
                true
            }
            ("select_effect", _) => {
                self.selected_effect_id = selected_row_id;
                true
            }
            ("toggle_effect", _) => {
                if let Some(effect_id) = selected_row_id {
                    let layer_id = self.selected_layer_id.clone();
                    let changed = layer_id
                        .as_deref()
                        .and_then(|layer_id| {
                            self.project_model
                                .active_composition_mut()
                                .layers
                                .iter_mut()
                                .find(|layer| layer.id == layer_id)
                        })
                        .and_then(|layer| {
                            layer
                                .effects
                                .iter_mut()
                                .find(|effect| effect.id == effect_id)
                        })
                        .map(|effect| effect.enabled = !effect.enabled)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("move_effect_up", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.move_effect(effect_id, -1);
                };
                true
            }
            ("move_effect_down", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.move_effect(effect_id, 1);
                };
                true
            }
            ("remove_effect", _) => {
                if let (Some(layer_id), Some(effect_id)) =
                    (self.selected_layer_id.clone(), selected_row_id)
                {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let old_len = layer.effects.len();
                        layer.effects.retain(|effect| effect.id != effect_id);
                        if layer.effects.len() != old_len {
                            if self.selected_effect_id.as_deref() == Some(effect_id.as_str()) {
                                self.selected_effect_id = None;
                            }
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("copy_selected_effect", _) => {
                if let (Some(layer_id), Some(effect_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_effect_id.as_deref(),
                ) {
                    if let Some(effect) = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == layer_id)
                        .and_then(|layer| layer.effects.iter().find(|e| e.id == effect_id))
                    {
                        self.effect_clipboard = Some(effect.clone());
                        self.asset_status =
                            format!("エフェクト「{}」をコピーしました。", effect.name);
                    }
                }
                true
            }
            ("paste_effect_to_layer", _) => {
                if let (Some(layer_id), Some(template)) =
                    (self.selected_layer_id.clone(), self.effect_clipboard.clone())
                {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let mut cloned = template;
                        let mut suffix = layer.effects.len() + 1;
                        let base_id = cloned.id.clone();
                        while layer.effects.iter().any(|e| e.id == cloned.id) {
                            cloned.id = format!("{base_id}-paste-{suffix}");
                            suffix += 1;
                        }
                        let new_id = cloned.id.clone();
                        let name = cloned.name.clone();
                        layer.effects.push(cloned);
                        self.selected_effect_id = Some(new_id);
                        self.mark_project_changed();
                        self.asset_status =
                            format!("エフェクト「{}」を貼り付けました。", name);
                    }
                }
                true
            }
            ("duplicate_selected_effect", _) => {
                if let (Some(layer_id), Some(effect_id)) = (
                    self.selected_layer_id.clone(),
                    self.selected_effect_id.clone(),
                ) {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        if let Some(pos) = layer.effects.iter().position(|e| e.id == effect_id) {
                            let mut cloned = layer.effects[pos].clone();
                            let mut suffix = layer.effects.len() + 1;
                            let base_id = cloned.id.clone();
                            while layer.effects.iter().any(|e| e.id == cloned.id) {
                                cloned.id = format!("{base_id}-dup-{suffix}");
                                suffix += 1;
                            }
                            let new_id = cloned.id.clone();
                            layer.effects.insert(pos + 1, cloned);
                            self.selected_effect_id = Some(new_id);
                            self.mark_project_changed();
                        }
                    }
                }
                true
            }
            ("effect_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter_id) = selected_row_id {
                    self.update_selected_effect_parameter(&parameter_id, &value)
                };
                true
            }
            ("effect_parameter_slider_changed", Some(ActionValue::Number(norm))) => {
                if let Some(parameter_id) = selected_row_id {
                    let range = self
                        .selected_layer_id
                        .as_deref()
                        .and_then(|layer_id| {
                            self.project_model
                                .active_composition()
                                .layers
                                .iter()
                                .find(|layer| layer.id == layer_id)
                        })
                        .and_then(|layer| {
                            self.selected_effect_id.as_deref().and_then(|effect_id| {
                                layer.effects.iter().find(|effect| effect.id == effect_id)
                            })
                        })
                        .map(|effect| effect_parameter_range(&effect.effect_type, &parameter_id))
                        .unwrap_or((0.0, 100.0));
                    let actual = range.0 + (norm.clamp(0.0, 1.0) as f32) * (range.1 - range.0);
                    self.update_selected_effect_parameter(&parameter_id, &format!("{actual:.3}"));
                }
                true
            }
            ("effect_parameter_keyframe", _) => {
                if let Some(parameter_id) = selected_row_id.as_deref() {
                    self.keyframe_selected_effect_parameter(parameter_id);
                };
                true
            }
            ("effect_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_effect_color(&value);
                true
            }
            ("color_scope_parade", _) => {
                self.color_scope_mode = ColorScopeMode::Parade;
                true
            }
            ("color_scope_waveform", _) => {
                self.color_scope_mode = ColorScopeMode::Waveform;
                true
            }
            ("color_scope_vectorscope", _) => {
                self.color_scope_mode = ColorScopeMode::Vectorscope;
                true
            }
            ("color_scope_histogram", _) => {
                self.color_scope_mode = ColorScopeMode::Histogram;
                true
            }
            ("reset_color_wheels", _) => {
                self.reset_color_wheels();
                true
            }
            _ => false,
        }
    }
}
