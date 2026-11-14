use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_graph_timeline_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> Result<Option<bool>, Box<dyn std::error::Error>> {
        match (action_name, action_value) {
            ("time_remap_reverse", _) => {
                self.apply_selected_time_remap_preset("reverse");
                Ok(Some(false))
            }
            ("time_remap_freeze", _) => {
                self.apply_selected_time_remap_preset("freeze");
                Ok(Some(false))
            }
            ("time_remap_slow", _) => {
                self.apply_selected_time_remap_preset("slow");
                Ok(Some(false))
            }
            ("time_remap_fast", _) => {
                self.apply_selected_time_remap_preset("fast");
                Ok(Some(false))
            }
            ("time_remap_value_changed", Some(ActionValue::Text(value))) => {
                self.set_selected_time_remap_keyframe(&value);
                Ok(Some(false))
            }
            ("graph_time_remap", _) => {
                if self.selected_layer_id.as_deref().is_some_and(|id| {
                    self.project_model
                        .active_composition()
                        .layers
                        .iter()
                        .any(|layer| {
                            layer.id == id
                                && !layer.locked
                                && layer.time_remap.is_some()
                                && matches!(
                                    &layer.layer_type,
                                    kagari_vfx::core::timeline::LayerType::Video { .. }
                                )
                        })
                }) {
                    self.graph_channel = GraphChannel::TimeRemap;
                    self.timeline_view = TimelineView::Graph;
                    self.workspace = Workspace::Graph;
                };
                Ok(Some(false))
            }
            ("timeline_view_all", _) => {
                self.timeline_view = TimelineView::All;
                Ok(Some(false))
            }
            ("timeline_view_animation", _) => {
                self.timeline_view = TimelineView::Animation;
                Ok(Some(false))
            }
            ("timeline_view_current_keyframes", _) => {
                self.timeline_view = TimelineView::CurrentKeyframes;
                Ok(Some(false))
            }
            ("timeline_view_audio", _) => {
                self.timeline_view = TimelineView::Audio;
                Ok(Some(false))
            }
            ("timeline_view_media", _) => {
                self.timeline_view = TimelineView::Media;
                Ok(Some(false))
            }
            ("timeline_view_text", _) => {
                self.timeline_view = TimelineView::Text;
                Ok(Some(false))
            }
            ("timeline_view_shapes", _) => {
                self.timeline_view = TimelineView::Shapes;
                Ok(Some(false))
            }
            ("timeline_view_3d", _) => {
                self.timeline_view = TimelineView::ThreeD;
                Ok(Some(false))
            }
            ("timeline_view_cameras", _) => {
                self.timeline_view = TimelineView::Cameras;
                Ok(Some(false))
            }
            ("timeline_view_lights", _) => {
                self.timeline_view = TimelineView::Lights;
                Ok(Some(false))
            }
            ("undo_project", _) => {
                self.undo_project();
                Ok(Some(false))
            }
            ("redo_project", _) => {
                self.redo_project();
                Ok(Some(false))
            }
            ("timeline_view_markers", _) => {
                self.timeline_view = TimelineView::Markers;
                Ok(Some(false))
            }
            ("timeline_view_graph", _) => {
                self.timeline_view = TimelineView::Graph;
                Ok(Some(false))
            }
            ("timeline_view_flowchart", _) => {
                self.timeline_view = TimelineView::Flowchart;
                Ok(Some(false))
            }
            ("properties_section_transform", _) => {
                self.properties_section = PropertiesSection::Transform;
                Ok(Some(false))
            }
            ("properties_section_content", _) => {
                self.properties_section = PropertiesSection::Content;
                Ok(Some(false))
            }
            ("properties_section_composite", _) => {
                self.properties_section = PropertiesSection::Composite;
                Ok(Some(false))
            }
            ("properties_section_advanced", _) => {
                self.properties_section = PropertiesSection::Advanced;
                Ok(Some(false))
            }
            ("toggle_timeline_snap", _) => {
                self.timeline_snap_enabled = !self.timeline_snap_enabled;
                self.timeline_snap_frame = None;
                Ok(Some(false))
            }
            ("split_selected_layer_at_playhead", _) => {
                self.split_selected_layer_at_playhead();
                Ok(Some(false))
            }
            ("set_selected_layer_in_to_playhead", _) => {
                self.set_selected_layer_in_to_playhead();
                Ok(Some(false))
            }
            ("set_selected_layer_out_to_playhead", _) => {
                self.set_selected_layer_out_to_playhead();
                Ok(Some(false))
            }
            ("graph_channel_next", _) => {
                let (channels, scalar_effect, scalar_params, color_effect, color_params) = self
                    .selected_layer_id
                    .as_deref()
                    .and_then(|id| {
                        self.project_model
                            .active_composition()
                            .layers
                            .iter()
                            .find(|layer| layer.id == id)
                    })
                    .map(|layer| {
                        let selected = self
                            .selected_effect_id
                            .as_deref()
                            .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                            .or_else(|| layer.effects.first());
                        let scalar = selected
                            .filter(|effect| {
                                !scalar_effect_parameters(&effect.effect_type).is_empty()
                            })
                            .or_else(|| {
                                layer.effects.iter().find(|effect| {
                                    !scalar_effect_parameters(&effect.effect_type).is_empty()
                                })
                            });
                        let colors = selected
                            .filter(|effect| {
                                !effect_color_parameters(&effect.effect_type).is_empty()
                            })
                            .or_else(|| {
                                layer.effects.iter().find(|effect| {
                                    !effect_color_parameters(&effect.effect_type).is_empty()
                                })
                            });
                        let params = scalar
                            .map(|effect| scalar_effect_parameters(&effect.effect_type))
                            .unwrap_or_default();
                        let color_params = colors
                            .map(|effect| effect_color_parameters(&effect.effect_type))
                            .unwrap_or_default();
                        let channels = GraphChannel::ALL
                            .iter()
                            .copied()
                            .filter(|channel| {
                                *channel != GraphChannel::EffectParameter
                                    && !matches!(
                                        channel,
                                        GraphChannel::EffectColorRed
                                            | GraphChannel::EffectColorGreen
                                            | GraphChannel::EffectColorBlue
                                            | GraphChannel::EffectColorAlpha
                                    )
                                    && channel.supports(layer)
                            })
                            .chain((!params.is_empty()).then_some(GraphChannel::EffectParameter))
                            .chain(
                                (!color_params.is_empty()).then_some(GraphChannel::EffectColorRed),
                            )
                            .chain(
                                (!color_params.is_empty())
                                    .then_some(GraphChannel::EffectColorGreen),
                            )
                            .chain(
                                (!color_params.is_empty()).then_some(GraphChannel::EffectColorBlue),
                            )
                            .chain(
                                (!color_params.is_empty())
                                    .then_some(GraphChannel::EffectColorAlpha),
                            )
                            .collect::<Vec<_>>();
                        (
                            channels,
                            scalar.map(|effect| effect.id.clone()),
                            params,
                            colors.map(|effect| effect.id.clone()),
                            color_params,
                        )
                    })
                    .unwrap_or_else(|| {
                        (
                            GraphChannel::ALL[..6].to_vec(),
                            None,
                            Vec::new(),
                            None,
                            Vec::new(),
                        )
                    });
                if channels.is_empty() {
                    return Ok(Some(true));
                }
                let index = channels
                    .iter()
                    .position(|channel| *channel == self.graph_channel)
                    .unwrap_or(0);
                self.graph_channel = channels[(index + 1) % channels.len()];
                if self.graph_channel == GraphChannel::EffectParameter {
                    self.selected_effect_id = scalar_effect;
                    self.graph_effect_parameter_id =
                        scalar_params.first().map(|(id, _)| id.clone());
                } else if effect_color_component(self.graph_channel).is_some() {
                    self.selected_effect_id = color_effect;
                    self.graph_effect_parameter_id = color_params.first().map(|(id, _)| id.clone());
                }
                self.timeline_view = TimelineView::Graph;
                Ok(Some(false))
            }
            ("graph_channel_select", _) => {
                let Some(channel) = selected_row_id
                    .as_deref()
                    .and_then(|id| id.parse::<usize>().ok())
                    .and_then(|index| GraphChannel::ALL.get(index).copied())
                else {
                    return Ok(Some(true));
                };
                let Some(layer) = self.selected_layer_id.as_deref().and_then(|id| {
                    self.project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == id)
                }) else {
                    return Ok(Some(true));
                };
                if !channel.supports(layer) {
                    return Ok(Some(true));
                }
                if channel == GraphChannel::EffectParameter
                    || effect_color_component(channel).is_some()
                {
                    let wants_color = effect_color_component(channel).is_some();
                    let Some(effect) = self
                        .selected_effect_id
                        .as_deref()
                        .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                        .filter(|effect| {
                            if wants_color {
                                !effect_color_parameters(&effect.effect_type).is_empty()
                            } else {
                                !scalar_effect_parameters(&effect.effect_type).is_empty()
                            }
                        })
                        .or_else(|| {
                            layer.effects.iter().find(|effect| {
                                if wants_color {
                                    !effect_color_parameters(&effect.effect_type).is_empty()
                                } else {
                                    !scalar_effect_parameters(&effect.effect_type).is_empty()
                                }
                            })
                        })
                    else {
                        return Ok(Some(true));
                    };
                    let params = if wants_color {
                        effect_color_parameters(&effect.effect_type)
                    } else {
                        scalar_effect_parameters(&effect.effect_type)
                    };
                    let Some((parameter_id, _)) = params.first() else {
                        return Ok(Some(true));
                    };
                    self.selected_effect_id = Some(effect.id.clone());
                    self.graph_effect_parameter_id = Some(parameter_id.clone());
                }
                if matches!(
                    channel,
                    GraphChannel::MaskFeather
                        | GraphChannel::MaskOpacity
                        | GraphChannel::MaskExpansion
                ) && self
                    .selected_mask_id
                    .as_deref()
                    .is_none_or(|id| layer.masks.iter().all(|mask| mask.id != id))
                {
                    self.selected_mask_id = layer.masks.first().map(|mask| mask.id.clone());
                }
                self.graph_channel = channel;
                self.timeline_view = TimelineView::Graph;
                Ok(Some(false))
            }
            ("graph_effect_parameter_select", _) => {
                let Some(parameter_id) = selected_row_id.as_deref() else {
                    return Ok(Some(true));
                };
                let Some(effect) = self
                    .selected_layer_id
                    .as_deref()
                    .and_then(|id| {
                        self.project_model
                            .active_composition()
                            .layers
                            .iter()
                            .find(|layer| layer.id == id)
                    })
                    .and_then(|layer| {
                        self.selected_effect_id.as_deref().and_then(|effect_id| {
                            layer.effects.iter().find(|effect| effect.id == effect_id)
                        })
                    })
                else {
                    return Ok(Some(true));
                };
                let params = if effect_color_component(self.graph_channel).is_some() {
                    effect_color_parameters(&effect.effect_type)
                } else if self.graph_channel == GraphChannel::EffectParameter {
                    scalar_effect_parameters(&effect.effect_type)
                } else {
                    Vec::new()
                };
                if params.iter().any(|(id, _)| id == parameter_id) {
                    self.graph_effect_parameter_id = Some(parameter_id.to_owned());
                    self.timeline_view = TimelineView::Graph;
                };
                Ok(Some(false))
            }
            ("graph_effect_parameter_next", _) => {
                if let Some(params) = self
                    .selected_layer_id
                    .as_deref()
                    .and_then(|id| {
                        self.project_model
                            .active_composition()
                            .layers
                            .iter()
                            .find(|layer| layer.id == id)
                    })
                    .and_then(|layer| {
                        self.selected_effect_id.as_deref().and_then(|effect_id| {
                            layer.effects.iter().find(|effect| effect.id == effect_id)
                        })
                    })
                    .map(|effect| {
                        if effect_color_component(self.graph_channel).is_some() {
                            effect_color_parameters(&effect.effect_type)
                        } else {
                            scalar_effect_parameters(&effect.effect_type)
                        }
                    })
                {
                    if !params.is_empty() {
                        let index = params
                            .iter()
                            .position(|(id, _)| {
                                Some(id.as_str()) == self.graph_effect_parameter_id.as_deref()
                            })
                            .unwrap_or(0);
                        self.graph_effect_parameter_id =
                            Some(params[(index + 1) % params.len()].0.clone());
                        self.timeline_view = TimelineView::Graph;
                    }
                };
                Ok(Some(false))
            }
            ("graph_add_keyframe", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some((layer, camera, light)) =
                        graph_targets_mut(self.project_model.active_composition_mut(), layer_id)
                    {
                        add_graph_keyframe(
                            layer,
                            camera,
                            light,
                            self.selected_mask_id.as_deref(),
                            self.selected_effect_id.as_deref(),
                            self.graph_effect_parameter_id.as_deref(),
                            self.graph_channel,
                            frame,
                        );
                        self.mark_project_changed();
                    }
                    self.timeline_view = TimelineView::Graph;
                };
                Ok(Some(false))
            }
            ("graph_remove_keyframe", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some((layer, camera, light)) =
                        graph_targets_mut(self.project_model.active_composition_mut(), layer_id)
                    {
                        if remove_graph_keyframe(
                            layer,
                            camera,
                            light,
                            self.selected_mask_id.as_deref(),
                            self.selected_effect_id.as_deref(),
                            self.graph_effect_parameter_id.as_deref(),
                            self.graph_channel,
                            frame,
                        ) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                };
                Ok(Some(false))
            }
            ("graph_cycle_interpolation", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    if let Some((layer, camera, light)) =
                        graph_targets_mut(self.project_model.active_composition_mut(), layer_id)
                    {
                        if cycle_graph_interpolation(
                            layer,
                            camera,
                            light,
                            self.selected_mask_id.as_deref(),
                            self.selected_effect_id.as_deref(),
                            self.graph_effect_parameter_id.as_deref(),
                            self.graph_channel,
                            self.current_frame,
                        ) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                };
                Ok(Some(false))
            }
            ("graph_set_easy_ease", _)
            | ("graph_set_ease_in", _)
            | ("graph_set_ease_out", _)
            | ("graph_set_overshoot", _)
            | ("graph_set_bounce", _)
            | ("graph_set_sine", _)
            | ("graph_set_linear", _)
            | ("graph_set_hold", _) => {
                let preset = match action_name {
                    "graph_set_easy_ease" => "easy_ease",
                    "graph_set_ease_in" => "ease_in",
                    "graph_set_ease_out" => "ease_out",
                    "graph_set_overshoot" => "overshoot",
                    "graph_set_bounce" => "bounce",
                    "graph_set_sine" => "sine",
                    "graph_set_linear" => "linear",
                    "graph_set_hold" => "hold",
                    _ => "easy_ease",
                };
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    if let Some((layer, camera, light)) =
                        graph_targets_mut(self.project_model.active_composition_mut(), layer_id)
                    {
                        if set_graph_interpolation_preset(
                            layer,
                            camera,
                            light,
                            self.selected_mask_id.as_deref(),
                            self.selected_effect_id.as_deref(),
                            self.graph_effect_parameter_id.as_deref(),
                            self.graph_channel,
                            self.current_frame,
                            preset,
                        ) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                };
                Ok(Some(false))
            }
            _ => Ok(None),
        }
    }
}
