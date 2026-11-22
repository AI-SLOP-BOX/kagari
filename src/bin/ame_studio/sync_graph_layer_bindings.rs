use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_graph_layer_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });
        self.ui.set_binding(
            "layer_in_frame",
            Value::String(selected_layer.map_or(0, |layer| layer.in_frame).to_string()),
        )?;
        self.ui.set_binding(
            "layer_out_frame",
            Value::String(
                selected_layer
                    .map_or(0, |layer| layer.out_frame)
                    .to_string(),
            ),
        )?;
        let horizontal_constraint = selected_layer
            .map(|layer| match layer.constraints.horizontal {
                kagari_vfx::core::layer_constraints::HorizontalPin::Left => "左固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Center => "中央固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Right => "右固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Scale => "幅に比例",
            })
            .unwrap_or("左固定");
        let vertical_constraint = selected_layer
            .map(|layer| match layer.constraints.vertical {
                kagari_vfx::core::layer_constraints::VerticalPin::Top => "上固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Center => "中央固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Bottom => "下固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Scale => "高さに比例",
            })
            .unwrap_or("上固定");
        self.ui.set_binding(
            "layer_constraint_horizontal",
            Value::String(horizontal_constraint.to_owned()),
        )?;
        self.ui.set_binding(
            "layer_constraint_vertical",
            Value::String(vertical_constraint.to_owned()),
        )?;
        let active_mask_id = selected_layer.and_then(|layer| {
            layer
                .masks
                .iter()
                .find(|mask| self.selected_mask_id.as_deref() == Some(mask.id.as_str()))
                .or_else(|| layer.masks.first())
                .map(|mask| mask.id.clone())
        });
        self.selected_mask_id = active_mask_id.clone();
        let selected_mask = selected_layer.and_then(|layer| {
            active_mask_id
                .as_deref()
                .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
        });
        let layer_masks = selected_layer
            .map(|layer| {
                layer
                    .masks
                    .iter()
                    .map(|mask| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(mask.id.clone())),
                            ("name".to_owned(), Value::String(mask.name.clone())),
                            (
                                "selected".to_owned(),
                                Value::Bool(active_mask_id.as_deref() == Some(mask.id.as_str())),
                            ),
                            ("enabled".to_owned(), Value::Bool(mask.enabled)),
                            (
                                "mode".to_owned(),
                                Value::String(mask_mode_label(mask.mode).to_owned()),
                            ),
                            ("inverted".to_owned(), Value::Bool(mask.inverted)),
                        ]))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ui
            .set_binding("has_layer_masks", Value::Bool(!layer_masks.is_empty()))?;
        self.ui
            .set_binding("layer_masks", Value::List(layer_masks))?;
        self.ui
            .set_binding("hide_shy_layers", Value::Bool(self.hide_shy_layers))?;
        self.ui.set_binding(
            "layer_frame_blending",
            Value::Bool(selected_layer.is_some_and(|layer| layer.frame_blending)),
        )?;
        self.ui.set_binding(
            "layer_motion_blur",
            Value::Bool(selected_layer.is_some_and(|layer| layer.motion_blur)),
        )?;
        self.ui.set_binding(
            "layer_preserve_transparency",
            Value::Bool(selected_layer.is_some_and(|layer| layer.preserve_transparency)),
        )?;
        self.ui.set_binding(
            "layer_guide",
            Value::Bool(selected_layer.is_some_and(|layer| layer.is_guide_layer)),
        )?;
        self.ui.set_binding(
            "layer_shy",
            Value::Bool(selected_layer.is_some_and(|layer| layer.is_shy)),
        )?;
        self.ui.set_binding(
            "layer_effects_enabled",
            Value::Bool(selected_layer.is_some_and(|layer| layer.effects_enabled)),
        )?;
        let auto_orient = selected_layer.map(|layer| layer.auto_orient);
        let auto_orient_label = auto_orient.map(auto_orient_label).unwrap_or("なし");
        self.ui.set_binding(
            "layer_auto_orient_label",
            Value::String(auto_orient_label.to_owned()),
        )?;
        let auto_orient_point = auto_orient.and_then(|mode| match mode {
            kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint { target_point } => {
                Some(target_point)
            }
            _ => None,
        });
        self.ui.set_binding(
            "layer_auto_orient_point_x",
            Value::String(
                auto_orient_point
                    .map(|point| format!("{:.1}", point[0]))
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "layer_auto_orient_point_y",
            Value::String(
                auto_orient_point
                    .map(|point| format!("{:.1}", point[1]))
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "layer_label_color_label",
            Value::String(
                selected_layer
                    .map(|layer| layer_label_color_label(layer.label).to_owned())
                    .unwrap_or_else(|| "なし".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "selected_mask_name",
            Value::String(
                selected_mask
                    .map(|mask| mask.name.clone())
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "mask_mode_label",
            Value::String(
                selected_mask
                    .map(|mask| mask_mode_label(mask.mode))
                    .unwrap_or("加算")
                    .to_owned(),
            ),
        )?;
        self.ui.set_binding(
            "mask_inverted",
            Value::Bool(selected_mask.is_some_and(|mask| mask.inverted)),
        )?;
        for (name, value) in [
            (
                "mask_feather",
                selected_mask.map(|mask| mask.feather.evaluate(self.current_frame)),
            ),
            (
                "mask_opacity",
                selected_mask.map(|mask| mask.opacity.evaluate(self.current_frame)),
            ),
            (
                "mask_expansion",
                selected_mask.map(|mask| mask.expansion.evaluate(self.current_frame)),
            ),
        ] {
            self.ui.set_binding(
                name,
                Value::String(value.map(|value| format!("{value:.2}")).unwrap_or_default()),
            )?;
        }
        let selected_keyframes = selected_layer
            .map(|layer| {
                layer_keyframe_frames_for_composition(
                    self.project_model.active_composition(),
                    layer,
                )
            })
            .unwrap_or_default();
        self.ui.set_binding(
            "keyframe_status",
            Value::String(if selected_keyframes.is_empty() {
                "選択レイヤーにキーなし".to_owned()
            } else {
                format!("{} キー", selected_keyframes.len())
            }),
        )?;
        let graph_effect = selected_layer.and_then(|layer| {
            self.selected_effect_id
                .as_deref()
                .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                .filter(|effect| {
                    !scalar_effect_parameters(&effect.effect_type).is_empty()
                        || !effect_color_parameters(&effect.effect_type).is_empty()
                })
                .or_else(|| {
                    layer.effects.iter().find(|effect| {
                        !scalar_effect_parameters(&effect.effect_type).is_empty()
                            || !effect_color_parameters(&effect.effect_type).is_empty()
                    })
                })
        });
        let graph_effect_parameters = graph_effect
            .map(|effect| scalar_effect_parameters(&effect.effect_type))
            .unwrap_or_default();
        let graph_effect_colors = graph_effect
            .map(|effect| effect_color_parameters(&effect.effect_type))
            .unwrap_or_default();
        let mut graph_color_channel = matches!(
            self.graph_channel,
            GraphChannel::EffectColorRed
                | GraphChannel::EffectColorGreen
                | GraphChannel::EffectColorBlue
                | GraphChannel::EffectColorAlpha
        );
        if selected_layer.is_some_and(|layer| !self.graph_channel.supports(layer))
            || (self.graph_channel == GraphChannel::EffectParameter
                && graph_effect_parameters.is_empty())
            || (graph_color_channel && graph_effect_colors.is_empty())
        {
            self.graph_channel = GraphChannel::PositionX;
            self.graph_effect_parameter_id = None;
            graph_color_channel = false;
        }
        if (self.graph_channel == GraphChannel::EffectParameter || graph_color_channel)
            && graph_effect.is_some_and(|effect| {
                Some(effect.id.as_str()) != self.selected_effect_id.as_deref()
            })
        {
            self.selected_effect_id = graph_effect.map(|effect| effect.id.clone());
        }
        if self.graph_channel == GraphChannel::EffectParameter || graph_color_channel {
            let params = if self.graph_channel == GraphChannel::EffectParameter {
                &graph_effect_parameters
            } else {
                &graph_effect_colors
            };
            let current_exists = params
                .iter()
                .any(|(id, _)| Some(id.as_str()) == self.graph_effect_parameter_id.as_deref());
            if !current_exists {
                self.graph_effect_parameter_id = params.first().map(|(id, _)| id.clone());
            }
        }
        let graph_channel_label = if self.graph_channel == GraphChannel::EffectParameter {
            selected_layer
                .and_then(|layer| {
                    graph_effect_parameter_label(
                        layer,
                        graph_effect.map(|effect| effect.id.as_str()),
                        self.graph_effect_parameter_id.as_deref(),
                    )
                })
                .unwrap_or_else(|| self.graph_channel.label().to_owned())
        } else if graph_color_channel {
            graph_effect
                .and_then(|effect| {
                    effect_color_parameters(&effect.effect_type)
                        .into_iter()
                        .find(|(id, _)| {
                            Some(id.as_str()) == self.graph_effect_parameter_id.as_deref()
                        })
                        .map(|(_, label)| {
                            format!(
                                "{} · {} · {}",
                                effect.name,
                                label,
                                graph_color_component_name(self.graph_channel)
                            )
                        })
                })
                .unwrap_or_else(|| self.graph_channel.label().to_owned())
        } else {
            self.graph_channel.label().to_owned()
        };
        let effect_value_channel =
            self.graph_channel == GraphChannel::EffectParameter || graph_color_channel;
        self.ui
            .set_binding("graph_effect_channel", Value::Bool(effect_value_channel))?;
        self.ui
            .set_binding("graph_channel_label", Value::String(graph_channel_label))?;
        let graph_channels = selected_layer
            .map(|layer| {
                GraphChannel::ALL
                    .iter()
                    .enumerate()
                    .filter(|(_, channel)| {
                        channel.supports(layer)
                            && (**channel != GraphChannel::EffectParameter
                                || !graph_effect_parameters.is_empty())
                            && (!matches!(
                                **channel,
                                GraphChannel::EffectColorRed
                                    | GraphChannel::EffectColorGreen
                                    | GraphChannel::EffectColorBlue
                                    | GraphChannel::EffectColorAlpha
                            ) || !graph_effect_colors.is_empty())
                    })
                    .map(|(index, channel)| {
                        let label = if *channel == GraphChannel::EffectParameter {
                            selected_layer
                                .and_then(|layer| {
                                    graph_effect_parameter_label(
                                        layer,
                                        graph_effect.map(|effect| effect.id.as_str()),
                                        self.graph_effect_parameter_id.as_deref(),
                                    )
                                })
                                .unwrap_or_else(|| channel.label().to_owned())
                        } else if matches!(
                            *channel,
                            GraphChannel::EffectColorRed
                                | GraphChannel::EffectColorGreen
                                | GraphChannel::EffectColorBlue
                                | GraphChannel::EffectColorAlpha
                        ) {
                            graph_effect
                                .and_then(|effect| {
                                    graph_effect_colors
                                        .iter()
                                        .find(|(id, _)| {
                                            Some(id.as_str())
                                                == self.graph_effect_parameter_id.as_deref()
                                        })
                                        .map(|(_, label)| {
                                            format!(
                                                "{} · {} · {}",
                                                effect.name,
                                                label,
                                                graph_color_component_name(*channel)
                                            )
                                        })
                                })
                                .unwrap_or_else(|| channel.label().to_owned())
                        } else {
                            channel.label().to_owned()
                        };
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(index.to_string())),
                            ("label".to_owned(), Value::String(label)),
                            (
                                "selected".to_owned(),
                                Value::Bool(*channel == self.graph_channel),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                GraphChannel::ALL[..6]
                    .iter()
                    .enumerate()
                    .map(|(index, channel)| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(index.to_string())),
                            (
                                "label".to_owned(),
                                Value::String(channel.label().to_owned()),
                            ),
                            (
                                "selected".to_owned(),
                                Value::Bool(*channel == self.graph_channel),
                            ),
                        ]))
                    })
                    .collect()
            });
        self.ui.set_binding(
            "has_graph_channels",
            Value::Bool(!graph_channels.is_empty()),
        )?;
        self.ui
            .set_binding("graph_channels", Value::List(graph_channels))?;
        let graph_parameters = if graph_color_channel {
            &graph_effect_colors
        } else {
            &graph_effect_parameters
        };
        let graph_parameter_items = if effect_value_channel {
            graph_parameters
                .iter()
                .map(|(id, label)| {
                    let label = if graph_color_channel {
                        format!(
                            "{} · {}",
                            label,
                            graph_color_component_name(self.graph_channel)
                        )
                    } else {
                        label.clone()
                    };
                    Value::Record(std::collections::BTreeMap::from([
                        ("id".to_owned(), Value::String(id.clone())),
                        ("label".to_owned(), Value::String(label)),
                        (
                            "selected".to_owned(),
                            Value::Bool(
                                Some(id.as_str()) == self.graph_effect_parameter_id.as_deref(),
                            ),
                        ),
                    ]))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.ui
            .set_binding("graph_parameter_items", Value::List(graph_parameter_items))?;
        let graph_interpolation = selected_layer
            .and_then(|layer| {
                graph_interpolation_label(
                    layer,
                    camera_for_layer(self.project_model.active_composition(), layer),
                    light_for_layer(self.project_model.active_composition(), layer),
                    self.selected_mask_id.as_deref(),
                    self.selected_effect_id.as_deref(),
                    self.graph_effect_parameter_id.as_deref(),
                    self.graph_channel,
                    self.current_frame,
                )
            })
            .unwrap_or_else(|| "キー未選択".to_owned());
        self.ui.set_binding(
            "graph_interpolation_label",
            Value::String(graph_interpolation),
        )?;
        self.ui.set_binding(
            "graph_has_keyframes",
            Value::Bool(selected_layer.is_some_and(|layer| {
                !graph_channel_keys(
                    layer,
                    camera_for_layer(self.project_model.active_composition(), layer),
                    light_for_layer(self.project_model.active_composition(), layer),
                    self.selected_mask_id.as_deref(),
                    self.selected_effect_id.as_deref(),
                    self.graph_effect_parameter_id.as_deref(),
                    self.graph_channel,
                )
                .is_empty()
            })),
        )?;
        self.ui.set_binding(
            "selected_time_remap",
            Value::Bool(selected_layer.is_some_and(|layer| layer.time_remap.is_some())),
        )?;
        self.ui.set_binding(
            "selected_is_video_layer",
            Value::Bool(selected_layer.is_some_and(|layer| {
                matches!(
                    &layer.layer_type,
                    kagari_vfx::core::timeline::LayerType::Video { .. }
                )
            })),
        )?;
        self.ui.set_binding(
            "selected_time_remap_source_frame",
            Value::String(
                selected_layer
                    .and_then(|layer| {
                        layer
                            .time_remap
                            .as_ref()
                            .map(|track| track.evaluate(self.current_frame))
                    })
                    .map(|frame| format!("{frame:.2}"))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        )?;
        Ok(())
    }
}
