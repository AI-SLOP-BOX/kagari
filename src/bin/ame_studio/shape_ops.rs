use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_solid_color(&mut self, input: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
            .and_then(|layer| match &mut layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Solid { color } => {
                    let next = parse_hex_rgb(input, color[3])?;
                    if *color == next {
                        Some(false)
                    } else {
                        *color = next;
                        Some(true)
                    }
                }
                _ => None,
            })
            .unwrap_or(false);
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_shape_property(&mut self, field: &str, input: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let numeric_value = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite());
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let kagari_vfx::core::timeline::LayerType::Shape {
            shape_type,
            color,
            stroke_color,
            stroke_width,
            extrusion_depth,
            bevel_depth,
            fill_type,
        } = &mut layer.layer_type
        else {
            return;
        };
        let changed = match field {
            "color" if matches!(fill_type, kagari_vfx::core::timeline::ShapeFillType::Solid) => {
                parse_hex_rgb(input, color[3]).is_some_and(|next| {
                    if *color == next {
                        false
                    } else {
                        *color = next;
                        true
                    }
                })
            }
            "stroke_color" => parse_hex_rgb(input, stroke_color[3]).is_some_and(|next| {
                if *stroke_color == next {
                    false
                } else {
                    *stroke_color = next;
                    true
                }
            }),
            "stroke_width" => numeric_value.is_some_and(|value| {
                let next = value.clamp(0.0, 512.0);
                if (*stroke_width - next).abs() <= f32::EPSILON {
                    false
                } else {
                    *stroke_width = next;
                    true
                }
            }),
            "extrusion_depth" => numeric_value.is_some_and(|value| {
                let next = value.clamp(0.0, 10_000.0);
                if (*extrusion_depth - next).abs() <= f32::EPSILON {
                    false
                } else {
                    *extrusion_depth = next;
                    true
                }
            }),
            "bevel_depth" => numeric_value.is_some_and(|value| {
                let next = value.clamp(0.0, 1_000.0);
                if (*bevel_depth - next).abs() <= f32::EPSILON {
                    false
                } else {
                    *bevel_depth = next;
                    true
                }
            }),
            "width" => numeric_value.is_some_and(|value| {
                let value = value.clamp(0.1, 16_384.0);
                match shape_type {
                    kagari_vfx::core::timeline::ShapeType::Rectangle { width, .. }
                    | kagari_vfx::core::timeline::ShapeType::Ellipse { width, .. } => {
                        set_animatable_at_frame(width, frame, value)
                    }
                    _ => false,
                }
            }),
            "height" => numeric_value.is_some_and(|value| {
                let value = value.clamp(0.1, 16_384.0);
                match shape_type {
                    kagari_vfx::core::timeline::ShapeType::Rectangle { height, .. }
                    | kagari_vfx::core::timeline::ShapeType::Ellipse { height, .. } => {
                        set_animatable_at_frame(height, frame, value)
                    }
                    _ => false,
                }
            }),
            "corner_radius" => numeric_value.is_some_and(|value| {
                let value = value.clamp(0.0, 16_384.0);
                match shape_type {
                    kagari_vfx::core::timeline::ShapeType::Rectangle { corner_radius, .. } => {
                        set_animatable_at_frame(corner_radius, frame, value)
                    }
                    _ => false,
                }
            }),
            "points" => numeric_value.is_some_and(|value| match shape_type {
                kagari_vfx::core::timeline::ShapeType::Star { points, .. } => {
                    set_animatable_at_frame(points, frame, value.clamp(3.0, 128.0).round())
                }
                _ => false,
            }),
            "inner_radius" => numeric_value.is_some_and(|value| match shape_type {
                kagari_vfx::core::timeline::ShapeType::Star { inner_radius, .. } => {
                    set_animatable_at_frame(inner_radius, frame, value.clamp(0.1, 16_384.0))
                }
                _ => false,
            }),
            "outer_radius" => numeric_value.is_some_and(|value| match shape_type {
                kagari_vfx::core::timeline::ShapeType::Star { outer_radius, .. } => {
                    set_animatable_at_frame(outer_radius, frame, value.clamp(0.1, 16_384.0))
                }
                _ => false,
            }),
            "sides" => numeric_value.is_some_and(|value| match shape_type {
                kagari_vfx::core::timeline::ShapeType::Polygon { sides, .. } => {
                    set_animatable_at_frame(sides, frame, value.clamp(3.0, 128.0).round())
                }
                _ => false,
            }),
            "radius" => numeric_value.is_some_and(|value| match shape_type {
                kagari_vfx::core::timeline::ShapeType::Polygon { radius, .. } => {
                    set_animatable_at_frame(radius, frame, value.clamp(0.1, 16_384.0))
                }
                _ => false,
            }),
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn cycle_selected_shape_fill_type(&mut self) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType};
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let composition_width = self.project_model.active_composition().width as f32;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let LayerType::Shape {
            color, fill_type, ..
        } = &mut layer.layer_type
        else {
            return;
        };
        match fill_type {
            ShapeFillType::Solid => {
                let start_color = *color;
                let end_color = [1.0, 1.0, 1.0, color[3]];
                *fill_type = ShapeFillType::LinearGradient {
                    start: [-composition_width * 0.5, 0.0],
                    end: [composition_width * 0.5, 0.0],
                    colors: vec![start_color, end_color],
                    stops: vec![0.0, 1.0],
                };
            }
            ShapeFillType::LinearGradient { colors, stops, .. } => {
                let mut colors = colors.clone();
                if colors.is_empty() {
                    colors.push(*color);
                }
                let mut stops = stops.clone();
                if stops.len() != colors.len() {
                    stops = (0..colors.len())
                        .map(|i| i as f32 / (colors.len().saturating_sub(1).max(1)) as f32)
                        .collect();
                }
                *fill_type = ShapeFillType::RadialGradient {
                    center: [0.0, 0.0],
                    radius: (composition_width * 0.5).max(1.0),
                    colors,
                    stops,
                };
            }
            ShapeFillType::RadialGradient { colors, .. } => {
                if let Some(first) = colors.first() {
                    *color = *first;
                }
                *fill_type = ShapeFillType::Solid;
            }
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_shape_gradient_color(&mut self, end_color: bool, input: &str) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType};
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
        let LayerType::Shape { fill_type, .. } = &mut layer.layer_type else {
            return;
        };
        let colors = match fill_type {
            ShapeFillType::LinearGradient { colors, .. }
            | ShapeFillType::RadialGradient { colors, .. } => colors,
            ShapeFillType::Solid => return,
        };
        if colors.is_empty() {
            return;
        }
        let index = if end_color { colors.len() - 1 } else { 0 };
        let Some(next) = parse_hex_rgb(input, colors[index][3]) else {
            return;
        };
        if colors[index] != next {
            colors[index] = next;
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_selected_shape_gradient_geometry(&mut self, field: &str, input: &str) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType};
        let Some(value) = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let composition_width = self.project_model.active_composition().width as f32;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let LayerType::Shape { fill_type, .. } = &mut layer.layer_type else {
            return;
        };
        let changed = match (field, fill_type) {
            ("angle", ShapeFillType::LinearGradient { start, end, .. }) => {
                let old_dx = end[0] - start[0];
                let old_dy = end[1] - start[1];
                let current_length = old_dx.hypot(old_dy);
                let length = if current_length <= f32::EPSILON {
                    composition_width.max(1.0)
                } else {
                    current_length
                };
                let angle = value.rem_euclid(360.0).to_radians();
                let dx = angle.cos() * length * 0.5;
                let dy = angle.sin() * length * 0.5;
                let center_x = (start[0] + end[0]) * 0.5;
                let center_y = (start[1] + end[1]) * 0.5;
                let next_start = [center_x - dx, center_y - dy];
                let next_end = [center_x + dx, center_y + dy];
                if (start[0] - next_start[0]).abs() <= f32::EPSILON
                    && (start[1] - next_start[1]).abs() <= f32::EPSILON
                    && (end[0] - next_end[0]).abs() <= f32::EPSILON
                    && (end[1] - next_end[1]).abs() <= f32::EPSILON
                {
                    false
                } else {
                    *start = next_start;
                    *end = next_end;
                    true
                }
            }
            ("radius", ShapeFillType::RadialGradient { radius, .. }) => {
                let next = value.clamp(1.0, (composition_width * 2.0).max(2.0));
                if (*radius - next).abs() <= f32::EPSILON {
                    false
                } else {
                    *radius = next;
                    true
                }
            }
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_selected_shape_trim_paths(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Shape { .. }
                    )
            })
        else {
            return;
        };
        if layer.trim_paths.is_some() {
            layer.trim_paths = None;
        } else {
            layer.trim_paths = Some(kagari_vfx::core::timeline::TrimPaths::default());
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_shape_trim_path(&mut self, field: &str, input: &str) {
        let Some(value) = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Shape { .. }
                    )
            })
        else {
            return;
        };
        let Some(trim) = layer.trim_paths.as_mut() else {
            return;
        };
        let changed = match field {
            "start" => {
                let next = value.clamp(0.0, 100.0);
                if (trim.start.evaluate(frame) - next).abs() <= f32::EPSILON {
                    false
                } else {
                    set_animatable_at_frame(&mut trim.start, frame, next)
                }
            }
            "end" => {
                let next = value.clamp(0.0, 100.0);
                if (trim.end.evaluate(frame) - next).abs() <= f32::EPSILON {
                    false
                } else {
                    set_animatable_at_frame(&mut trim.end, frame, next)
                }
            }
            "offset" => {
                let next = value.clamp(-36_000.0, 36_000.0);
                if (trim.offset.evaluate(frame) - next).abs() <= f32::EPSILON {
                    false
                } else {
                    set_animatable_at_frame(&mut trim.offset, frame, next)
                }
            }
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn add_selected_shape_trim_keyframe(&mut self, field: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let Some(trim) = layer.trim_paths.as_mut() else {
            return;
        };
        let track = match field {
            "start" => &mut trim.start,
            "end" => &mut trim.end,
            "offset" => &mut trim.offset,
            _ => return,
        };
        add_current_keyframe(track, frame);
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_shape_repeater(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Shape { .. }
                    )
            })
        else {
            return;
        };
        if layer.shape_repeater.is_some() {
            layer.shape_repeater = None;
        } else {
            layer.shape_repeater =
                Some(kagari_vfx::core::shape_repeater::ShapeRepeaterOptions::default());
        }
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_shape_repeater_composite(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Shape { .. }
                    )
            })
        else {
            return;
        };
        let Some(repeater) = layer.shape_repeater.as_mut() else {
            return;
        };
        repeater.composite_below = !repeater.composite_below;
        self.mark_project_changed();
    }

    pub(crate) fn add_selected_shape_repeater_keyframe(&mut self, field: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let Some(repeater) = layer.shape_repeater.as_mut() else {
            return;
        };
        use kagari_vfx::core::property::Animatable;
        match field {
            "copies" => {
                let track = repeater
                    .copies_animation
                    .get_or_insert_with(|| Animatable::new_constant(repeater.copies as f32));
                add_current_keyframe(track, frame);
            }
            "position_x" | "position_y" => {
                let track = repeater.position_offset_animation.get_or_insert_with(|| {
                    Animatable::new_constant(repeater.position_offset)
                });
                add_current_keyframe(track, frame);
            }
            "scale_x" | "scale_y" => {
                let track = repeater.scale_offset_animation.get_or_insert_with(|| {
                    Animatable::new_constant(repeater.scale_offset)
                });
                add_current_keyframe(track, frame);
            }
            "rotation" => {
                let track = repeater.rotation_offset_animation.get_or_insert_with(|| {
                    Animatable::new_constant(repeater.rotation_offset_deg)
                });
                add_current_keyframe(track, frame);
            }
            "start_opacity" | "end_opacity" => {
                let track = repeater.opacity_animation.get_or_insert_with(|| {
                    Animatable::new_constant([repeater.start_opacity, repeater.end_opacity])
                });
                add_current_keyframe(track, frame);
            }
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_shape_repeater(&mut self, field: &str, input: &str) {
        let Some(value) = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Shape { .. }
                    )
            })
        else {
            return;
        };
        let Some(repeater) = layer.shape_repeater.as_mut() else {
            return;
        };
        let changed = match field {
            "copies" => {
                let next = value.round().clamp(1.0, 1000.0);
                if let Some(track) = repeater.copies_animation.as_mut() {
                    if (track.evaluate(frame) - next).abs() <= f32::EPSILON {
                        false
                    } else {
                        set_animatable_at_frame(track, frame, next)
                    }
                } else {
                    let next = next as u32;
                    if repeater.copies == next {
                        false
                    } else {
                        repeater.copies = next;
                        true
                    }
                }
            }
            "position_x" | "position_y" | "scale_x" | "scale_y" => {
                let is_position = field.starts_with("position");
                let axis = if field.ends_with('y') { 1 } else { 0 };
                let next = if is_position {
                    value.clamp(-100_000.0, 100_000.0)
                } else {
                    (value / 100.0).clamp(0.01, 10.0)
                };
                let track = if is_position {
                    repeater.position_offset_animation.as_mut()
                } else {
                    repeater.scale_offset_animation.as_mut()
                };
                let static_values = if is_position {
                    &mut repeater.position_offset
                } else {
                    &mut repeater.scale_offset
                };
                if let Some(track) = track {
                    let mut values = track.evaluate(frame);
                    if (values[axis] - next).abs() <= f32::EPSILON {
                        false
                    } else {
                        values[axis] = next;
                        set_animatable_at_frame(track, frame, values)
                    }
                } else if (static_values[axis] - next).abs() <= f32::EPSILON {
                    false
                } else {
                    static_values[axis] = next;
                    true
                }
            }
            "rotation" => {
                let next = value.clamp(-36_000.0, 36_000.0);
                if let Some(track) = repeater.rotation_offset_animation.as_mut() {
                    if (track.evaluate(frame) - next).abs() <= f32::EPSILON {
                        false
                    } else {
                        set_animatable_at_frame(track, frame, next)
                    }
                } else if (repeater.rotation_offset_deg - next).abs() <= f32::EPSILON {
                    false
                } else {
                    repeater.rotation_offset_deg = next;
                    true
                }
            }
            "start_opacity" | "end_opacity" => {
                let axis = if field == "end_opacity" { 1 } else { 0 };
                let next = (value / 100.0).clamp(0.0, 1.0);
                if let Some(track) = repeater.opacity_animation.as_mut() {
                    let mut values = track.evaluate(frame);
                    if (values[axis] - next).abs() <= f32::EPSILON {
                        false
                    } else {
                        values[axis] = next;
                        set_animatable_at_frame(track, frame, values)
                    }
                } else {
                    let opacity = if axis == 0 {
                        &mut repeater.start_opacity
                    } else {
                        &mut repeater.end_opacity
                    };
                    if (*opacity - next).abs() <= f32::EPSILON {
                        false
                    } else {
                        *opacity = next;
                        true
                    }
                }
            }
            _ => false,
        };
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn add_shape_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let position = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        self.add_shape_at(position);
    }

    pub(crate) fn add_ellipse_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Ellipse {
                width: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                height: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("楕円".to_owned(), layer_type, center);
        let _ = self.sync_bindings();
    }

    pub(crate) fn add_star_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Star {
                points: kagari_vfx::core::timeline::Animatable::new_constant(5.0),
                inner_radius: kagari_vfx::core::timeline::Animatable::new_constant(32.0),
                outer_radius: kagari_vfx::core::timeline::Animatable::new_constant(80.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("星形".to_owned(), layer_type, center);
    }

    pub(crate) fn add_polygon_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Polygon {
                sides: kagari_vfx::core::timeline::Animatable::new_constant(6.0),
                radius: kagari_vfx::core::timeline::Animatable::new_constant(80.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("多角形".to_owned(), layer_type, center);
    }

    pub(crate) fn add_solid_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        self.add_canvas_layer(
            "平面".to_owned(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.08, 0.10, 0.12, 1.0],
            },
            center,
        );
    }

    pub(crate) fn add_shape_at(&mut self, position: [f64; 2]) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType, ShapeType};
        self.add_canvas_layer(
            "長方形".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                    height: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                    corner_radius: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                },
                color: [1.0, 0.42, 0.12, 1.0],
                stroke_color: [1.0, 1.0, 1.0, 1.0],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            position,
        );
        let _ = self.sync_bindings();
    }

    pub(crate) fn add_pen_line(&mut self, start: [f64; 2], end: [f64; 2]) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType, ShapeType};
        let delta = [end[0] - start[0], end[1] - start[1]];
        let points = vec![
            [0.0, 0.0],
            [(delta[0] * 0.5) as f32, (delta[1] * 0.5) as f32],
            [delta[0] as f32, delta[1] as f32],
        ];
        self.add_canvas_layer(
            "ペンの線".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::FreeformBezier {
                    points,
                    tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                    closed: false,
                },
                color: [0.0, 0.0, 0.0, 0.0],
                stroke_color: [1.0, 0.42, 0.12, 1.0],
                stroke_width: 6.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            start,
        );
        let _ = self.sync_bindings();
    }
}
