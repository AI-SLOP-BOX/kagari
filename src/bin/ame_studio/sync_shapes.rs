use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_shape_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });
        let shape_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Shape {
                shape_type,
                color,
                stroke_color,
                stroke_width,
                extrusion_depth,
                bevel_depth,
                fill_type,
            } => {
                let (
                    width,
                    height,
                    corner_radius,
                    is_rectangle,
                    is_star,
                    is_polygon,
                    points,
                    inner_radius,
                    outer_radius,
                    sides,
                    radius,
                ) = match shape_type {
                    kagari_vfx::core::timeline::ShapeType::Rectangle {
                        width,
                        height,
                        corner_radius,
                    } => (
                        Some(width.evaluate(self.current_frame)),
                        Some(height.evaluate(self.current_frame)),
                        Some(corner_radius.evaluate(self.current_frame)),
                        true,
                        false,
                        false,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => (
                        Some(width.evaluate(self.current_frame)),
                        Some(height.evaluate(self.current_frame)),
                        None,
                        false,
                        false,
                        false,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    kagari_vfx::core::timeline::ShapeType::Star {
                        points,
                        inner_radius,
                        outer_radius,
                    } => (
                        None,
                        None,
                        None,
                        false,
                        true,
                        false,
                        Some(points.evaluate(self.current_frame)),
                        Some(inner_radius.evaluate(self.current_frame)),
                        Some(outer_radius.evaluate(self.current_frame)),
                        None,
                        None,
                    ),
                    kagari_vfx::core::timeline::ShapeType::Polygon { sides, radius } => (
                        None,
                        None,
                        None,
                        false,
                        false,
                        true,
                        None,
                        None,
                        None,
                        Some(sides.evaluate(self.current_frame)),
                        Some(radius.evaluate(self.current_frame)),
                    ),
                    _ => (
                        None, None, None, false, false, false, None, None, None, None, None,
                    ),
                };
                Some((
                    width,
                    height,
                    corner_radius,
                    rgba_to_hex(color),
                    is_rectangle,
                    matches!(fill_type, kagari_vfx::core::timeline::ShapeFillType::Solid),
                    rgba_to_hex(stroke_color),
                    *stroke_width,
                    *extrusion_depth,
                    *bevel_depth,
                    is_star,
                    is_polygon,
                    points,
                    inner_radius,
                    outer_radius,
                    sides,
                    radius,
                ))
            }
            _ => None,
        });
        let shape_fill_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Shape {
                color, fill_type, ..
            } => {
                use kagari_vfx::core::timeline::ShapeFillType;
                let (label, start_color, end_color, angle, radius) = match fill_type {
                    ShapeFillType::Solid => {
                        ("単色", rgba_to_hex(color), rgba_to_hex(color), None, None)
                    }
                    ShapeFillType::LinearGradient {
                        start, end, colors, ..
                    } => (
                        "線形グラデーション",
                        colors
                            .first()
                            .map(rgba_to_hex)
                            .unwrap_or_else(|| rgba_to_hex(color)),
                        colors
                            .last()
                            .map(rgba_to_hex)
                            .unwrap_or_else(|| rgba_to_hex(color)),
                        Some((end[1] - start[1]).atan2(end[0] - start[0]).to_degrees()),
                        None,
                    ),
                    ShapeFillType::RadialGradient { radius, colors, .. } => (
                        "円形グラデーション",
                        colors
                            .first()
                            .map(rgba_to_hex)
                            .unwrap_or_else(|| rgba_to_hex(color)),
                        colors
                            .last()
                            .map(rgba_to_hex)
                            .unwrap_or_else(|| rgba_to_hex(color)),
                        None,
                        Some(*radius),
                    ),
                };
                Some((
                    label,
                    !matches!(fill_type, ShapeFillType::Solid),
                    start_color,
                    end_color,
                    angle,
                    radius,
                ))
            }
            _ => None,
        });
        let shape_trim_properties = selected_layer
            .and_then(|layer| layer.trim_paths.as_ref())
            .map(|trim| {
                (
                    trim.start.evaluate(self.current_frame),
                    trim.end.evaluate(self.current_frame),
                    trim.offset.evaluate(self.current_frame),
                )
            });
        let shape_repeater_properties = selected_layer
            .and_then(|layer| layer.shape_repeater.as_ref())
            .map(|repeater| {
                let copies = repeater
                    .copies_animation
                    .as_ref()
                    .map(|value| value.evaluate(self.current_frame))
                    .unwrap_or(repeater.copies as f32);
                let position = repeater
                    .position_offset_animation
                    .as_ref()
                    .map(|value| value.evaluate(self.current_frame))
                    .unwrap_or(repeater.position_offset);
                let scale = repeater
                    .scale_offset_animation
                    .as_ref()
                    .map(|value| value.evaluate(self.current_frame))
                    .unwrap_or(repeater.scale_offset);
                let rotation = repeater
                    .rotation_offset_animation
                    .as_ref()
                    .map(|value| value.evaluate(self.current_frame))
                    .unwrap_or(repeater.rotation_offset_deg);
                let opacities = repeater
                    .opacity_animation
                    .as_ref()
                    .map(|value| value.evaluate(self.current_frame))
                    .unwrap_or([repeater.start_opacity, repeater.end_opacity]);
                (
                    copies,
                    position,
                    scale,
                    rotation,
                    opacities[0],
                    opacities[1],
                    repeater.composite_below,
                )
            });
        let solid_color = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Solid { color } => Some(rgba_to_hex(color)),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_solid", Value::Bool(solid_color.is_some()))?;
        self.ui.set_binding(
            "solid_color",
            Value::String(solid_color.unwrap_or_else(|| "#20252A".to_owned())),
        )?;
        self.ui
            .set_binding("selected_is_shape", Value::Bool(shape_properties.is_some()))?;
        self.ui.set_binding(
            "selected_is_sized_shape",
            Value::Bool(
                shape_properties
                    .as_ref()
                    .is_some_and(|shape| shape.0.is_some()),
            ),
        )?;
        self.ui.set_binding(
            "selected_is_rectangle",
            Value::Bool(shape_properties.as_ref().is_some_and(|shape| shape.4)),
        )?;
        self.ui.set_binding(
            "selected_shape_solid",
            Value::Bool(shape_properties.as_ref().is_some_and(|shape| shape.5)),
        )?;
        self.ui.set_binding(
            "selected_shape_gradient",
            Value::Bool(shape_fill_properties.as_ref().is_some_and(|shape| shape.1)),
        )?;
        self.ui.set_binding(
            "shape_fill_type_label",
            Value::String(
                shape_fill_properties
                    .as_ref()
                    .map(|shape| shape.0.to_owned())
                    .unwrap_or_else(|| "単色".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "shape_gradient_start_color",
            Value::String(
                shape_fill_properties
                    .as_ref()
                    .map(|shape| shape.2.clone())
                    .unwrap_or_else(|| "#FF6B1A".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "shape_gradient_end_color",
            Value::String(
                shape_fill_properties
                    .as_ref()
                    .map(|shape| shape.3.clone())
                    .unwrap_or_else(|| "#FFFFFF".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "shape_gradient_angle",
            Value::String(
                shape_fill_properties
                    .as_ref()
                    .and_then(|shape| shape.4)
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "shape_gradient_radius",
            Value::String(
                shape_fill_properties
                    .as_ref()
                    .and_then(|shape| shape.5)
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "960".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "shape_trim_enabled",
            Value::Bool(shape_trim_properties.is_some()),
        )?;
        for (name, value, fallback) in [
            (
                "shape_trim_start",
                shape_trim_properties.map(|props| props.0),
                0.0,
            ),
            (
                "shape_trim_end",
                shape_trim_properties.map(|props| props.1),
                100.0,
            ),
            (
                "shape_trim_offset",
                shape_trim_properties.map(|props| props.2),
                0.0,
            ),
        ] {
            self.ui.set_binding(
                name,
                Value::String(
                    value
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| fallback.to_string()),
                ),
            )?;
        }
        self.ui.set_binding(
            "shape_repeater_enabled",
            Value::Bool(shape_repeater_properties.is_some()),
        )?;
        for (name, value, fallback) in [
            (
                "shape_repeater_copies",
                shape_repeater_properties.map(|props| props.0),
                3.0,
            ),
            (
                "shape_repeater_x",
                shape_repeater_properties.map(|props| props.1[0]),
                100.0,
            ),
            (
                "shape_repeater_y",
                shape_repeater_properties.map(|props| props.1[1]),
                0.0,
            ),
            (
                "shape_repeater_scale_x",
                shape_repeater_properties.map(|props| props.2[0] * 100.0),
                100.0,
            ),
            (
                "shape_repeater_scale_y",
                shape_repeater_properties.map(|props| props.2[1] * 100.0),
                100.0,
            ),
            (
                "shape_repeater_rotation",
                shape_repeater_properties.map(|props| props.3),
                0.0,
            ),
            (
                "shape_repeater_start_opacity",
                shape_repeater_properties.map(|props| props.4 * 100.0),
                100.0,
            ),
            (
                "shape_repeater_end_opacity",
                shape_repeater_properties.map(|props| props.5 * 100.0),
                100.0,
            ),
        ] {
            self.ui.set_binding(
                name,
                Value::String(
                    value
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| fallback.to_string()),
                ),
            )?;
        }
        self.ui.set_binding(
            "shape_repeater_composite_below",
            Value::Bool(shape_repeater_properties.is_some_and(|props| props.6)),
        )?;
        self.ui.set_binding(
            "selected_is_star",
            Value::Bool(shape_properties.as_ref().is_some_and(|shape| shape.10)),
        )?;
        self.ui.set_binding(
            "selected_is_polygon",
            Value::Bool(shape_properties.as_ref().is_some_and(|shape| shape.11)),
        )?;
        for (name, value) in [
            (
                "shape_width",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.0)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "100".to_owned()),
            ),
            (
                "shape_height",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.1)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "100".to_owned()),
            ),
            (
                "shape_corner_radius",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.2)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "shape_color",
                shape_properties
                    .as_ref()
                    .map(|shape| shape.3.clone())
                    .unwrap_or_else(|| "#FF6B1A".to_owned()),
            ),
            (
                "shape_stroke_color",
                shape_properties
                    .as_ref()
                    .map(|shape| shape.6.clone())
                    .unwrap_or_else(|| "#FFFFFF".to_owned()),
            ),
            (
                "shape_stroke_width",
                shape_properties
                    .as_ref()
                    .map(|shape| format!("{:.2}", shape.7))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "shape_extrusion_depth",
                shape_properties
                    .as_ref()
                    .map(|shape| format!("{:.2}", shape.8))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "shape_bevel_depth",
                shape_properties
                    .as_ref()
                    .map(|shape| format!("{:.2}", shape.9))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "shape_points",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.12)
                    .map(|value| format!("{value:.0}"))
                    .unwrap_or_else(|| "5".to_owned()),
            ),
            (
                "shape_inner_radius",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.13)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "30".to_owned()),
            ),
            (
                "shape_outer_radius",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.14)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "80".to_owned()),
            ),
            (
                "shape_sides",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.15)
                    .map(|value| format!("{value:.0}"))
                    .unwrap_or_else(|| "6".to_owned()),
            ),
            (
                "shape_radius",
                shape_properties
                    .as_ref()
                    .and_then(|shape| shape.16)
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "80".to_owned()),
            ),
        ] {
            self.ui.set_binding(name, Value::String(value))?;
        }
        Ok(())
    }
}
