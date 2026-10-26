use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_text_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let position = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        self.add_text_at(position);
    }

    pub(crate) fn toggle_selected_text_animator(&mut self) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == layer_id
                    && !layer.locked
                    && matches!(
                        layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Text { .. }
                    )
            })
        else {
            return;
        };
        let animator = layer.text_animator.get_or_insert_with(Default::default);
        animator.enabled = !animator.enabled;
        self.mark_project_changed();
    }

    pub(crate) fn cycle_selected_text_animator_shape(&mut self) {
        use kagari_vfx::core::text_animator::SelectorShape;
        const SHAPES: [SelectorShape; 9] = [
            SelectorShape::Square,
            SelectorShape::RampUp,
            SelectorShape::RampDown,
            SelectorShape::Triangle,
            SelectorShape::Round,
            SelectorShape::Smooth,
            SelectorShape::Wobble,
            SelectorShape::Random,
            SelectorShape::Expression,
        ];
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
        let Some(animator) = layer
            .text_animator
            .as_mut()
            .filter(|animator| animator.enabled)
        else {
            return;
        };
        let index = SHAPES
            .iter()
            .position(|shape| *shape == animator.selector.shape)
            .unwrap_or(0);
        animator.selector.shape = SHAPES[(index + 1) % SHAPES.len()];
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_text_animator_random_order(&mut self) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(animator) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| layer.text_animator.as_mut())
        else {
            return;
        };
        animator.selector.random_order = !animator.selector.random_order;
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_text_animator_expression(&mut self, input: &str) {
        let expression = input.trim().chars().take(2000).collect::<String>();
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(animator) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| layer.text_animator.as_mut())
        else {
            return;
        };
        if animator.selector.expression.as_deref() != Some(expression.as_str()) {
            animator.selector.expression = (!expression.is_empty()).then_some(expression);
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_text_animator_parameter(&mut self, field: &str, input: &str) {
        let Ok(value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
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
        let Some(animator) = layer
            .text_animator
            .as_mut()
            .filter(|animator| animator.enabled)
        else {
            return;
        };
        let changed = match field {
            "selector.start" => {
                set_f32_if_changed(&mut animator.selector.start, value.clamp(0.0, 100.0))
            }
            "selector.end" => {
                set_f32_if_changed(&mut animator.selector.end, value.clamp(0.0, 100.0))
            }
            "selector.offset" => {
                set_f32_if_changed(&mut animator.selector.offset, value.clamp(-100.0, 100.0))
            }
            "selector.ease_high" => {
                set_f32_if_changed(&mut animator.selector.ease_high, value.clamp(0.0, 100.0))
            }
            "selector.ease_low" => {
                set_f32_if_changed(&mut animator.selector.ease_low, value.clamp(0.0, 100.0))
            }
            "selector.char_start" => {
                let mut next = value.round().clamp(-1.0, 1_000_000.0) as i32;
                if next >= 0 && animator.selector.char_end >= 0 {
                    next = next.min(animator.selector.char_end);
                }
                let changed = animator.selector.char_start != next;
                animator.selector.char_start = next;
                changed
            }
            "selector.char_end" => {
                let mut next = value.round().clamp(-1.0, 1_000_000.0) as i32;
                if next >= 0 && animator.selector.char_start >= 0 {
                    next = next.max(animator.selector.char_start);
                }
                let changed = animator.selector.char_end != next;
                animator.selector.char_end = next;
                changed
            }
            "position_offset.x" => set_f32_if_changed(
                &mut animator.position_offset[0],
                value.clamp(-10_000.0, 10_000.0),
            ),
            "position_offset.y" => set_f32_if_changed(
                &mut animator.position_offset[1],
                value.clamp(-10_000.0, 10_000.0),
            ),
            "scale.x" => set_f32_if_changed(&mut animator.scale[0], value.clamp(0.0, 10.0)),
            "scale.y" => set_f32_if_changed(&mut animator.scale[1], value.clamp(0.0, 10.0)),
            "opacity" => set_f32_if_changed(&mut animator.opacity, value.clamp(-100.0, 100.0)),
            "tracking" => set_f32_if_changed(&mut animator.tracking, value.clamp(-1000.0, 1000.0)),
            "rotation" => set_f32_if_changed(&mut animator.rotation, value.clamp(-360.0, 360.0)),
            "blur_amount" => set_f32_if_changed(&mut animator.blur_amount, value.clamp(0.0, 500.0)),
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_text_property(&mut self, field: &str, input: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let kagari_vfx::core::timeline::LayerType::Text {
            text,
            font_size,
            color,
            font_family,
            tracking,
            leading,
            align,
            stroke_color,
            stroke_width,
            ..
        } = &mut layer.layer_type
        else {
            return;
        };
        let changed = match field {
            "content" => {
                let next = input.chars().take(10_000).collect::<String>();
                if *text == next {
                    false
                } else {
                    *text = next;
                    true
                }
            }
            "font_family" => {
                let next = input.trim().chars().take(128).collect::<String>();
                if next.is_empty() || *font_family == next {
                    false
                } else {
                    *font_family = next;
                    true
                }
            }
            "font_size" => input.trim().parse::<u32>().ok().is_some_and(|value| {
                let next = value.clamp(1, 1024);
                if *font_size == next {
                    false
                } else {
                    *font_size = next;
                    true
                }
            }),
            "color" => parse_hex_rgb(input, color[3]).is_some_and(|next| {
                if *color == next {
                    false
                } else {
                    *color = next;
                    true
                }
            }),
            "tracking" => input
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .is_some_and(|value| {
                    let next = value.clamp(-1000.0, 1000.0);
                    if *tracking == next {
                        false
                    } else {
                        *tracking = next;
                        true
                    }
                }),
            "leading" => input
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .is_some_and(|value| {
                    let next = value.clamp(0.1, 10.0);
                    if *leading == next {
                        false
                    } else {
                        *leading = next;
                        true
                    }
                }),
            "align" => input.trim().parse::<usize>().ok().is_some_and(|value| {
                let next = value.min(2);
                if *align == next {
                    false
                } else {
                    *align = next;
                    true
                }
            }),
            "stroke_width" => input
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .is_some_and(|value| {
                    let next = value.clamp(0.0, 200.0);
                    if *stroke_width == next {
                        false
                    } else {
                        *stroke_width = next;
                        true
                    }
                }),
            "stroke_color" => parse_hex_rgb(input, stroke_color[3]).is_some_and(|next| {
                if *stroke_color == next {
                    false
                } else {
                    *stroke_color = next;
                    true
                }
            }),
            _ => false,
        };
        if changed {
            if let Some(formatting) = layer.text_formatting.as_mut() {
                match field {
                    "font_family" => formatting.font_family.clone_from(font_family),
                    "tracking" => formatting.tracking = *tracking,
                    "leading" => formatting.leading = *leading,
                    "align" => formatting.alignment = *align as u32,
                    "stroke_width" => formatting.stroke_width = *stroke_width,
                    "stroke_color" => formatting.stroke_color = Some(*stroke_color),
                    _ => {}
                }
            }
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_text_formatting(
        &mut self,
        update: impl FnOnce(&mut kagari_vfx::core::timeline::TextFormatting) -> bool,
    ) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        if !matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Text { .. }
        ) {
            return;
        }
        let changed = if let Some(formatting) = layer.text_formatting.as_mut() {
            update(formatting)
        } else {
            let initial = match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Text {
                    font_family,
                    tracking,
                    leading,
                    align,
                    stroke_color,
                    stroke_width,
                    ..
                } => kagari_vfx::core::timeline::TextFormatting {
                    font_family: font_family.clone(),
                    tracking: *tracking,
                    leading: *leading,
                    stroke_color: (*stroke_width > 0.0).then_some(*stroke_color),
                    stroke_width: *stroke_width,
                    alignment: *align as u32,
                    ..Default::default()
                },
                _ => return,
            };
            let mut next = initial;
            if update(&mut next) {
                layer.text_formatting = Some(next);
                true
            } else {
                false
            }
        };
        if changed {
            self.mark_project_changed();
        }
    }
}
