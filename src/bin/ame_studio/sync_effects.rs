use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_effect_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });
        let selected_effect = selected_layer.and_then(|layer| {
            self.selected_effect_id
                .as_deref()
                .and_then(|effect_id| layer.effects.iter().find(|effect| effect.id == effect_id))
        });
        let selected_effect_color = selected_effect.and_then(|effect| {
            use kagari_vfx::core::timeline::EffectType;
            match &effect.effect_type {
                EffectType::ColorTint { color, .. }
                | EffectType::DropShadow { color, .. }
                | EffectType::Vignette { color, .. }
                | EffectType::Glow { color, .. }
                | EffectType::LensFlare { color, .. } => Some(color.evaluate(self.current_frame)),
                _ => None,
            }
        });
        self.ui.set_binding(
            "has_selected_effect_color",
            Value::Bool(selected_effect_color.is_some()),
        )?;
        self.ui.set_binding(
            "selected_effect_color",
            Value::String(
                selected_effect_color
                    .as_ref()
                    .map(rgba_to_hex)
                    .unwrap_or_else(|| "#FFFFFF".to_owned()),
            ),
        )?;
        let effect_query = self.effect_search.trim().to_lowercase();
        let effect_catalog = [
            ("blur", "ぼかし"),
            ("tint", "色調"),
            ("shadow", "影"),
            ("glow", "Glow"),
            ("vignette", "ビネット"),
            ("hue_saturation", "HSL"),
            ("levels", "Levels"),
            ("motion_blur", "モーションブラー"),
            ("film_grain", "フィルムグレイン"),
            ("posterize", "ポスタライズ"),
            ("directional_blur", "方向ぼかし"),
            ("radial_blur", "放射状ぼかし"),
            ("sharpen", "シャープ"),
            ("twirl", "渦巻き"),
            ("bulge", "膨張・収縮"),
            ("invert", "反転"),
            ("offset", "オフセット"),
            ("threshold", "しきい値"),
            ("linear_wipe", "リニアワイプ"),
            ("spherize", "球面化"),
            ("vibrance", "自然な彩度"),
            ("white_balance", "ホワイトバランス"),
            ("chromatic_aberration", "色収差"),
            ("chroma_key", "クロマキー"),
            ("turbulent_displace", "タービュレントディスプレイス"),
            ("cc_lens", "CC Lens"),
            ("polar_coordinates", "極座標"),
            ("wave_warp", "波形ワープ"),
            ("vortex", "渦"),
            ("fisheye", "魚眼レンズ"),
            ("lens_correction", "レンズ補正"),
            ("glitch_displacement", "グリッチディスプレイス"),
            ("tiler", "タイル"),
            ("hsl_adjust", "HSL 調整"),
            ("crt_scanlines", "CRT スキャンライン"),
            ("shift_channels", "チャンネルシフト"),
            ("minimax", "ミニマックス"),
            ("fractal_noise", "フラクタルノイズ"),
            ("curves", "トーンカーブ"),
            ("compound_blur", "コンパウンドブラー"),
            ("displacement_map", "ディスプレイスメントマップ"),
            ("mosaic", "モザイク"),
            ("flicker", "フリッカー"),
            ("directional_sharpen", "方向シャープ"),
            ("emboss", "エンボス"),
            ("bevel_alpha", "アルファベベル"),
            ("letterbox", "レターボックス"),
            ("luma_key_range", "輝度キー"),
            ("tritone", "トライトーン"),
            ("gradient_map", "グラデーションマップ"),
            ("light_sweep", "ライトスイープ"),
            ("radial_fast_blur", "ラジアル高速ぼかし"),
            ("bend_it", "曲げ"),
            ("venetian_blinds", "ベネチアンブラインド"),
            ("glow_pro", "Glow Pro"),
            ("sobel_edges", "Sobel Edges"),
            ("find_edges", "Find Edges"),
            ("median_filter", "メディアンフィルター"),
        ];
        let effect_catalog_items = effect_catalog
            .iter()
            .filter(|(id, label)| {
                effect_query.is_empty()
                    || id.contains(&effect_query)
                    || label.to_lowercase().contains(&effect_query)
            })
            .map(|(id, label)| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String((*id).to_owned())),
                    ("label".to_owned(), Value::String((*label).to_owned())),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_effect_catalog_items",
            Value::Bool(!effect_catalog_items.is_empty()),
        )?;
        self.ui
            .set_binding("effect_catalog_items", Value::List(effect_catalog_items))?;
        let effect_count = selected_layer.map_or(0, |layer| layer.effects.len());
        let effect_items = selected_layer
            .into_iter()
            .flat_map(|layer| layer.effects.iter().enumerate())
            .filter(|(_, effect)| {
                effect_query.is_empty() || effect.name.to_lowercase().contains(&effect_query)
            })
            .map(|(index, effect)| {
                let kind = effect_type_label(&effect.effect_type);
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(effect.id.clone())),
                    ("name".to_owned(), Value::String(effect.name.clone())),
                    ("kind".to_owned(), Value::String(kind.to_owned())),
                    ("enabled".to_owned(), Value::Bool(effect.enabled)),
                    ("can_move_up".to_owned(), Value::Bool(index > 0)),
                    (
                        "can_move_down".to_owned(),
                        Value::Bool(index + 1 < effect_count),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_effect_id.as_deref() == Some(effect.id.as_str())),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui
            .set_binding("has_effect_items", Value::Bool(!effect_items.is_empty()))?;
        self.ui
            .set_binding("effect_items", Value::List(effect_items))?;
        let effect_parameters = selected_effect
            .map(|effect| effect_parameter_specs(&effect.effect_type, self.current_frame))
            .unwrap_or_default()
            .into_iter()
            .map(|(id, label, value, unit)| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(id.to_owned())),
                    ("label".to_owned(), Value::String(label.to_owned())),
                    ("value".to_owned(), Value::String(format!("{value:.2}"))),
                    ("unit".to_owned(), Value::String(unit.to_owned())),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_effect_parameters",
            Value::Bool(!effect_parameters.is_empty()),
        )?;
        self.sync_tracker_bindings()?;
        self.sync_animation_preset_bindings()?;
        Ok(())
    }

    pub(crate) fn sync_animation_preset_bindings(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let query = self.animation_preset_search.trim().to_lowercase();
        let animation_preset_items = AfterEffectsStudio::animation_preset_catalog()
            .iter()
            .filter(|(id, label, category, keywords)| {
                query.is_empty()
                    || id.to_lowercase().contains(&query)
                    || label.to_lowercase().contains(&query)
                    || category.to_lowercase().contains(&query)
                    || keywords.contains(&query)
            })
            .map(|(id, label, category, _)| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String((*id).to_owned())),
                    ("label".to_owned(), Value::String((*label).to_owned())),
                    ("category".to_owned(), Value::String((*category).to_owned())),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_animation_preset_items",
            Value::Bool(!animation_preset_items.is_empty()),
        )?;
        self.ui.set_binding(
            "animation_preset_items",
            Value::List(animation_preset_items),
        )?;
        self.ui.set_binding(
            "animation_preset_search",
            Value::String(self.animation_preset_search.clone()),
        )?;
        self.ui.set_binding(
            "animation_preset_status",
            Value::String(self.animation_preset_status.clone()),
        )?;
        Ok(())
    }

    pub(crate) fn sync_tracker_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });

        let tracker_items = selected_layer
            .map(|layer| {
                layer
                    .trackers
                    .iter()
                    .enumerate()
                    .map(|(index, tracker)| {
                        let points_count = match &tracker.position {
                            kagari_vfx::core::timeline::Animatable::Constant(_) => 1,
                            kagari_vfx::core::timeline::Animatable::Animated(kfs) => kfs.len(),
                        };
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(tracker.id.clone())),
                            ("name".to_owned(), Value::String(tracker.name.clone())),
                            (
                                "points_count".to_owned(),
                                Value::String(points_count.to_string()),
                            ),
                            (
                                "selected".to_owned(),
                                Value::Bool(self.selected_tracker_idx == index),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        self.ui
            .set_binding("has_tracker_items", Value::Bool(!tracker_items.is_empty()))?;
        self.ui
            .set_binding("tracker_items", Value::List(tracker_items))?;

        let current_layer_id = self.selected_layer_id.clone();
        let tracker_target_layers = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .filter(|layer| Some(&layer.id) != current_layer_id.as_ref())
            .map(|layer| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(layer.id.clone())),
                    ("name".to_owned(), Value::String(layer.name.clone())),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_tracker_target_id.as_deref() == Some(&layer.id)),
                    ),
                ]))
            })
            .collect::<Vec<_>>();

        self.ui
            .set_binding("tracker_target_layers", Value::List(tracker_target_layers))?;
        self.ui
            .set_binding("tracker_status", Value::String(self.tracker_status.clone()))?;
        Ok(())
    }
}
