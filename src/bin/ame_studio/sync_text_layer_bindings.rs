use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_text_layer_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });

        let text_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text {
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
            } => {
                let formatting = layer.text_formatting.as_ref();
                let effective_stroke_color = formatting
                    .and_then(|formatting| formatting.stroke_color)
                    .unwrap_or(*stroke_color);
                Some((
                    text.clone(),
                    font_size.to_string(),
                    rgba_to_hex(color),
                    formatting
                        .map(|formatting| formatting.font_family.clone())
                        .unwrap_or_else(|| font_family.clone()),
                    formatting
                        .map(|formatting| formatting.tracking)
                        .unwrap_or(*tracking)
                        .to_string(),
                    formatting
                        .map(|formatting| formatting.leading)
                        .unwrap_or(*leading)
                        .to_string(),
                    formatting
                        .map(|formatting| formatting.alignment as usize)
                        .unwrap_or(*align),
                    rgba_to_hex(&effective_stroke_color),
                    formatting
                        .map(|formatting| {
                            if formatting.stroke_color.is_some() {
                                formatting.stroke_width
                            } else {
                                0.0
                            }
                        })
                        .unwrap_or(*stroke_width)
                        .to_string(),
                ))
            }
            _ => None,
        });
        self.ui
            .set_binding("selected_is_text", Value::Bool(text_properties.is_some()))?;
        let active_text_family = text_properties
            .as_ref()
            .map(|properties| properties.3.as_str());
        let font_query = self.text_font_search.trim().to_lowercase();
        let mut visible_font_families = self
            .text_font_families
            .iter()
            .filter(|family| font_query.is_empty() || family.to_lowercase().contains(&font_query))
            .take(if font_query.is_empty() { 16 } else { 40 })
            .collect::<Vec<_>>();
        if font_query.is_empty() {
            if let Some(selected_family) = active_text_family {
                if !visible_font_families
                    .iter()
                    .any(|family| family.as_str() == selected_family)
                {
                    if let Some(family) = self
                        .text_font_families
                        .iter()
                        .find(|family| family.as_str() == selected_family)
                    {
                        visible_font_families.push(family);
                    }
                }
            }
        }
        self.ui.set_binding(
            "text_font_families",
            Value::List(
                visible_font_families
                    .iter()
                    .map(|family| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String((*family).clone())),
                            ("name".to_owned(), Value::String((*family).clone())),
                            (
                                "selected".to_owned(),
                                Value::Bool(active_text_family == Some(family.as_str())),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        )?;
        self.ui.set_binding(
            "has_text_font_families",
            Value::Bool(!visible_font_families.is_empty()),
        )?;
        self.ui.set_binding(
            "text_font_search",
            Value::String(self.text_font_search.clone()),
        )?;
        let text_formatting = selected_layer.and_then(|layer| layer.text_formatting.as_ref());
        for (name, value) in [
            (
                "text_faux_bold",
                text_formatting.is_some_and(|formatting| formatting.faux_bold),
            ),
            (
                "text_faux_italic",
                text_formatting.is_some_and(|formatting| formatting.faux_italic),
            ),
            (
                "text_all_caps",
                text_formatting.is_some_and(|formatting| formatting.all_caps),
            ),
            (
                "text_small_caps",
                text_formatting.is_some_and(|formatting| formatting.small_caps),
            ),
        ] {
            self.ui.set_binding(name, Value::Bool(value))?;
        }
        self.ui.set_binding(
            "text_box_width",
            Value::String(
                text_formatting
                    .map(|formatting| formatting.box_width.to_string())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "text_box_height",
            Value::String(
                text_formatting
                    .map(|formatting| formatting.box_height.to_string())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        )?;
        for (name, value) in [
            (
                "text_content",
                text_properties
                    .as_ref()
                    .map(|properties| properties.0.clone())
                    .unwrap_or_default(),
            ),
            (
                "text_font_size",
                text_properties
                    .as_ref()
                    .map(|properties| properties.1.clone())
                    .unwrap_or_else(|| "48".to_owned()),
            ),
            (
                "text_color",
                text_properties
                    .as_ref()
                    .map(|properties| properties.2.clone())
                    .unwrap_or_else(|| "#FFFFFF".to_owned()),
            ),
            (
                "text_font_family",
                text_properties
                    .as_ref()
                    .map(|properties| properties.3.clone())
                    .unwrap_or_else(|| "Inter".to_owned()),
            ),
            (
                "text_tracking",
                text_properties
                    .as_ref()
                    .map(|properties| properties.4.clone())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "text_leading",
                text_properties
                    .as_ref()
                    .map(|properties| properties.5.clone())
                    .unwrap_or_else(|| "1.2".to_owned()),
            ),
            (
                "text_stroke_color",
                text_properties
                    .as_ref()
                    .map(|properties| properties.7.clone())
                    .unwrap_or_else(|| "#000000".to_owned()),
            ),
            (
                "text_stroke_width",
                text_properties
                    .as_ref()
                    .map(|properties| properties.8.clone())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        ] {
            self.ui.set_binding(name, Value::String(value))?;
        }
        self.ui.set_binding(
            "selected_text_on_path",
            Value::Bool(selected_layer.is_some_and(|layer| {
                matches!(
                    &layer.layer_type,
                    kagari_vfx::core::timeline::LayerType::Text { text_on_path, .. }
                        if *text_on_path
                )
            })),
        )?;
        for (name, alignment) in [
            ("text_align_left", 0),
            ("text_align_center", 1),
            ("text_align_right", 2),
        ] {
            self.ui.set_binding(
                name,
                Value::Bool(
                    text_properties
                        .as_ref()
                        .is_some_and(|properties| properties.6 == alignment),
                ),
            )?;
        }
        let text_animator = selected_layer.and_then(|layer| layer.text_animator.as_ref());
        self.ui.set_binding(
            "text_animator_present",
            Value::Bool(text_animator.is_some()),
        )?;
        self.ui.set_binding(
            "text_animator_enabled",
            Value::Bool(text_animator.is_some_and(|animator| animator.enabled)),
        )?;
        let shape_label = text_animator
            .map(|animator| match animator.selector.shape {
                kagari_vfx::core::text_animator::SelectorShape::Square => "Square",
                kagari_vfx::core::text_animator::SelectorShape::RampUp => "Ramp Up",
                kagari_vfx::core::text_animator::SelectorShape::RampDown => "Ramp Down",
                kagari_vfx::core::text_animator::SelectorShape::Triangle => "Triangle",
                kagari_vfx::core::text_animator::SelectorShape::Round => "Round",
                kagari_vfx::core::text_animator::SelectorShape::Smooth => "Smooth",
                kagari_vfx::core::text_animator::SelectorShape::Wobble => "Wobble",
                kagari_vfx::core::text_animator::SelectorShape::Random => "Random",
                kagari_vfx::core::text_animator::SelectorShape::Expression => "Expression",
            })
            .unwrap_or("Square");
        self.ui.set_binding(
            "text_animator_shape_label",
            Value::String(shape_label.to_owned()),
        )?;
        self.ui.set_binding(
            "text_animator_random_order",
            Value::Bool(text_animator.is_some_and(|animator| animator.selector.random_order)),
        )?;
        self.ui.set_binding(
            "text_animator_expression",
            Value::String(
                text_animator
                    .and_then(|animator| animator.selector.expression.clone())
                    .unwrap_or_default(),
            ),
        )?;
        let text_animator_parameters = text_animator
            .filter(|animator| animator.enabled)
            .map(|animator| {
                [
                    ("selector.start", "範囲開始", animator.selector.start, "%"),
                    ("selector.end", "範囲終了", animator.selector.end, "%"),
                    (
                        "selector.offset",
                        "範囲オフセット",
                        animator.selector.offset,
                        "%",
                    ),
                    (
                        "selector.ease_high",
                        "上側イージング",
                        animator.selector.ease_high,
                        "%",
                    ),
                    (
                        "selector.ease_low",
                        "下側イージング",
                        animator.selector.ease_low,
                        "%",
                    ),
                    (
                        "selector.char_start",
                        "文字番号開始",
                        animator.selector.char_start as f32,
                        "(-1 = 全て)",
                    ),
                    (
                        "selector.char_end",
                        "文字番号終了",
                        animator.selector.char_end as f32,
                        "(-1 = 全て)",
                    ),
                    (
                        "position_offset.x",
                        "位置 X",
                        animator.position_offset[0],
                        "px",
                    ),
                    (
                        "position_offset.y",
                        "位置 Y",
                        animator.position_offset[1],
                        "px",
                    ),
                    ("scale.x", "倍率 X", animator.scale[0], "×"),
                    ("scale.y", "倍率 Y", animator.scale[1], "×"),
                    ("opacity", "不透明度変化", animator.opacity, "%"),
                    ("tracking", "文字間隔変化", animator.tracking, "px"),
                    ("rotation", "回転", animator.rotation, "°"),
                    ("blur_amount", "ぼかし", animator.blur_amount, "px"),
                ]
                .into_iter()
                .map(|(id, label, value, unit)| {
                    Value::Record(std::collections::BTreeMap::from([
                        ("id".to_owned(), Value::String(id.to_owned())),
                        ("label".to_owned(), Value::String(label.to_owned())),
                        ("value".to_owned(), Value::String(format!("{value:.2}"))),
                        ("unit".to_owned(), Value::String(unit.to_owned())),
                    ]))
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ui.set_binding(
            "has_text_animator_parameters",
            Value::Bool(!text_animator_parameters.is_empty()),
        )?;
        self.ui.set_binding(
            "text_animator_parameters",
            Value::List(text_animator_parameters),
        )?;
        Ok(())
    }
}
