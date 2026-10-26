use super::*;
use ame_studio_app::rgba_to_hex;

impl AfterEffectsStudio {
    pub(crate) fn sync_layer_styles_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_style = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
                .map(|layer| layer.style.clone())
        });
        let layer_style_items = selected_style
            .as_ref()
            .map(|style| {
                [
                    ("drop_shadow", "ドロップシャドウ", style.drop_shadow.enabled),
                    ("outer_glow", "外側グロー", style.outer_glow.enabled),
                    ("stroke", "境界線", style.stroke.enabled),
                    (
                        "color_overlay",
                        "カラーオーバーレイ",
                        style.color_overlay.enabled,
                    ),
                    ("inner_shadow", "内側シャドウ", style.inner_shadow.enabled),
                    ("inner_glow", "内側グロー", style.inner_glow.enabled),
                    ("satin", "サテン", style.satin.enabled),
                    (
                        "bevel_emboss",
                        "ベベルとエンボス",
                        style.bevel_emboss.enabled,
                    ),
                    (
                        "gradient_overlay",
                        "グラデーションオーバーレイ",
                        style.gradient_overlay.enabled,
                    ),
                ]
                .into_iter()
                .map(|(id, label, enabled)| {
                    Value::Record(std::collections::BTreeMap::from([
                        ("id".to_owned(), Value::String(id.to_owned())),
                        ("label".to_owned(), Value::String(label.to_owned())),
                        ("enabled".to_owned(), Value::Bool(enabled)),
                    ]))
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut style_params: Vec<(String, String, String, String)> = Vec::new();
        if let Some(style) = selected_style.as_ref() {
            if style.drop_shadow.enabled {
                style_params.extend([
                    (
                        "drop_shadow.opacity".into(),
                        "影の不透明度".into(),
                        format!("{:.1}", style.drop_shadow.opacity),
                        "%".into(),
                    ),
                    (
                        "drop_shadow.angle".into(),
                        "影の角度".into(),
                        format!("{:.1}", style.drop_shadow.angle),
                        "°".into(),
                    ),
                    (
                        "drop_shadow.distance".into(),
                        "影の距離".into(),
                        format!("{:.1}", style.drop_shadow.distance),
                        "px".into(),
                    ),
                    (
                        "drop_shadow.size".into(),
                        "影のぼかし".into(),
                        format!("{:.1}", style.drop_shadow.size),
                        "px".into(),
                    ),
                    (
                        "drop_shadow.color".into(),
                        "影の色".into(),
                        rgba_to_hex(&style.drop_shadow.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.outer_glow.enabled {
                style_params.extend([
                    (
                        "outer_glow.opacity".into(),
                        "グロー不透明度".into(),
                        format!("{:.1}", style.outer_glow.opacity),
                        "%".into(),
                    ),
                    (
                        "outer_glow.size".into(),
                        "グローサイズ".into(),
                        format!("{:.1}", style.outer_glow.size),
                        "px".into(),
                    ),
                ]);
            }
            if style.stroke.enabled {
                style_params.extend([
                    (
                        "stroke.size".into(),
                        "境界線幅".into(),
                        format!("{:.1}", style.stroke.size),
                        "px".into(),
                    ),
                    (
                        "stroke.color".into(),
                        "境界線色".into(),
                        rgba_to_hex(&style.stroke.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.color_overlay.enabled {
                style_params.extend([
                    (
                        "color_overlay.opacity".into(),
                        "カラー不透明度".into(),
                        format!("{:.1}", style.color_overlay.opacity),
                        "%".into(),
                    ),
                    (
                        "color_overlay.color".into(),
                        "カラー".into(),
                        rgba_to_hex(&style.color_overlay.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.inner_shadow.enabled {
                style_params.extend([
                    (
                        "inner_shadow.opacity".into(),
                        "内側影不透明度".into(),
                        format!("{:.1}", style.inner_shadow.opacity),
                        "%".into(),
                    ),
                    (
                        "inner_shadow.angle".into(),
                        "内側影角度".into(),
                        format!("{:.1}", style.inner_shadow.angle),
                        "°".into(),
                    ),
                    (
                        "inner_shadow.distance".into(),
                        "内側影距離".into(),
                        format!("{:.1}", style.inner_shadow.distance),
                        "px".into(),
                    ),
                    (
                        "inner_shadow.size".into(),
                        "内側影サイズ".into(),
                        format!("{:.1}", style.inner_shadow.size),
                        "px".into(),
                    ),
                    (
                        "inner_shadow.color".into(),
                        "内側影色".into(),
                        rgba_to_hex(&style.inner_shadow.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.inner_glow.enabled {
                style_params.extend([
                    (
                        "inner_glow.opacity".into(),
                        "内側グロー不透明度".into(),
                        format!("{:.1}", style.inner_glow.opacity),
                        "%".into(),
                    ),
                    (
                        "inner_glow.size".into(),
                        "内側グローサイズ".into(),
                        format!("{:.1}", style.inner_glow.size),
                        "px".into(),
                    ),
                    (
                        "inner_glow.color".into(),
                        "内側グロー色".into(),
                        rgba_to_hex(&style.inner_glow.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.satin.enabled {
                style_params.extend([
                    (
                        "satin.opacity".into(),
                        "サテン不透明度".into(),
                        format!("{:.1}", style.satin.opacity),
                        "%".into(),
                    ),
                    (
                        "satin.angle".into(),
                        "サテン角度".into(),
                        format!("{:.1}", style.satin.angle),
                        "°".into(),
                    ),
                    (
                        "satin.distance".into(),
                        "サテン距離".into(),
                        format!("{:.1}", style.satin.distance),
                        "px".into(),
                    ),
                    (
                        "satin.size".into(),
                        "サテンサイズ".into(),
                        format!("{:.1}", style.satin.size),
                        "px".into(),
                    ),
                    (
                        "satin.color".into(),
                        "サテン色".into(),
                        rgba_to_hex(&style.satin.color),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.bevel_emboss.enabled {
                style_params.extend([
                    (
                        "bevel_emboss.angle".into(),
                        "ベベル角度".into(),
                        format!("{:.1}", style.bevel_emboss.angle),
                        "°".into(),
                    ),
                    (
                        "bevel_emboss.depth".into(),
                        "ベベル深さ".into(),
                        format!("{:.1}", style.bevel_emboss.depth),
                        "px".into(),
                    ),
                    (
                        "bevel_emboss.size".into(),
                        "ベベルサイズ".into(),
                        format!("{:.1}", style.bevel_emboss.size),
                        "px".into(),
                    ),
                    (
                        "bevel_emboss.highlight".into(),
                        "ハイライト".into(),
                        format!("{:.1}", style.bevel_emboss.highlight),
                        "%".into(),
                    ),
                    (
                        "bevel_emboss.shadow".into(),
                        "シャドウ".into(),
                        format!("{:.1}", style.bevel_emboss.shadow),
                        "%".into(),
                    ),
                    (
                        "bevel_emboss.color_light".into(),
                        "ハイライト色".into(),
                        rgba_to_hex(&style.bevel_emboss.color_light),
                        "#RRGGBB".into(),
                    ),
                    (
                        "bevel_emboss.color_dark".into(),
                        "シャドウ色".into(),
                        rgba_to_hex(&style.bevel_emboss.color_dark),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
            if style.gradient_overlay.enabled {
                style_params.extend([
                    (
                        "gradient_overlay.opacity".into(),
                        "グラデーション不透明度".into(),
                        format!("{:.1}", style.gradient_overlay.opacity),
                        "%".into(),
                    ),
                    (
                        "gradient_overlay.angle".into(),
                        "グラデーション角度".into(),
                        format!("{:.1}", style.gradient_overlay.angle),
                        "°".into(),
                    ),
                    (
                        "gradient_overlay.scale".into(),
                        "グラデーション範囲".into(),
                        format!("{:.1}", style.gradient_overlay.scale),
                        "%".into(),
                    ),
                    (
                        "gradient_overlay.color_start".into(),
                        "開始色".into(),
                        rgba_to_hex(&style.gradient_overlay.color_start),
                        "#RRGGBB".into(),
                    ),
                    (
                        "gradient_overlay.color_end".into(),
                        "終了色".into(),
                        rgba_to_hex(&style.gradient_overlay.color_end),
                        "#RRGGBB".into(),
                    ),
                ]);
            }
        }

        let style_parameter_items = style_params
            .into_iter()
            .map(|(id, label, value, unit)| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(id)),
                    ("label".to_owned(), Value::String(label)),
                    ("value".to_owned(), Value::String(value)),
                    ("unit".to_owned(), Value::String(unit)),
                ]))
            })
            .collect::<Vec<_>>();

        self.ui
            .set_binding("layer_style_items", Value::List(layer_style_items))?;
        self.ui.set_binding(
            "has_layer_style_parameters",
            Value::Bool(!style_parameter_items.is_empty()),
        )?;
        self.ui
            .set_binding("layer_style_parameters", Value::List(style_parameter_items))?;
        Ok(())
    }

    pub(crate) fn sync_composite_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (blend_mode_label_str, track_matte_label_str, has_track_matte, matte_source_layers) = {
            let active_comp = self.project_model.active_composition();
            let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
                active_comp.layers.iter().find(|layer| layer.id == id)
            });

            let blend_label = selected_layer
                .map(|layer| blend_mode_label(layer.blend_mode).to_owned())
                .unwrap_or_else(|| "通常".to_owned());

            let matte_label = selected_layer
                .map(|layer| match layer.track_matte {
                    kagari_vfx::core::timeline::TrackMatteMode::None => "なし",
                    kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte => "アルファ",
                    kagari_vfx::core::timeline::TrackMatteMode::AlphaMatteInverted => "アルファ反転",
                    kagari_vfx::core::timeline::TrackMatteMode::LumaMatte => "輝度",
                    kagari_vfx::core::timeline::TrackMatteMode::LumaMatteInverted => "輝度反転",
                })
                .unwrap_or("なし")
                .to_owned();

            let has_matte = selected_layer.is_some_and(|layer| {
                layer.track_matte != kagari_vfx::core::timeline::TrackMatteMode::None
            });

            let matte_sources = selected_layer
                .map(|selected| {
                    active_comp
                        .layers
                        .iter()
                        .filter(|candidate| candidate.id != selected.id)
                        .map(|candidate| {
                            Value::Record(std::collections::BTreeMap::from([
                                ("id".to_owned(), Value::String(candidate.id.clone())),
                                ("name".to_owned(), Value::String(candidate.name.clone())),
                                (
                                    "selected".to_owned(),
                                    Value::Bool(
                                        selected.track_matte_layer_id.as_deref()
                                            == Some(candidate.id.as_str()),
                                    ),
                                ),
                            ]))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            (blend_label, matte_label, has_matte, matte_sources)
        };

        self.ui
            .set_binding("selected_blend_mode_label", Value::String(blend_mode_label_str))?;
        self.ui
            .set_binding("selected_track_matte_label", Value::String(track_matte_label_str))?;
        self.ui
            .set_binding("selected_has_track_matte", Value::Bool(has_track_matte))?;
        self.ui
            .set_binding("matte_source_layers", Value::List(matte_source_layers))?;

        Ok(())
    }
}
