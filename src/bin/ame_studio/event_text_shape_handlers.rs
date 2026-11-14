use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_text_shape_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("text_content_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("content", &value);
                true
            }
            ("text_font_family_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("font_family", &value);
                true
            }
            ("text_font_family_select", _) => {
                if let Some(family) = selected_row_id.as_deref() {
                    if self
                        .text_font_families
                        .iter()
                        .any(|installed| installed == family)
                    {
                        self.update_selected_text_property("font_family", family);
                    }
                };
                true
            }
            ("text_font_search_changed", Some(ActionValue::Text(value))) => {
                self.text_font_search = value.chars().take(100).collect();
                true
            }
            ("text_font_size_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("font_size", &value);
                true
            }
            ("text_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("color", &value);
                true
            }
            ("text_tracking_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("tracking", &value);
                true
            }
            ("text_leading_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("leading", &value);
                true
            }
            ("text_align_left", _) => {
                self.update_selected_text_property("align", "0");
                true
            }
            ("text_align_center", _) => {
                self.update_selected_text_property("align", "1");
                true
            }
            ("text_align_right", _) => {
                self.update_selected_text_property("align", "2");
                true
            }
            ("text_stroke_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("stroke_width", &value);
                true
            }
            ("text_stroke_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("stroke_color", &value);
                true
            }
            ("toggle_text_faux_bold", _) => {
                self.update_selected_text_formatting(|formatting| {
                    formatting.faux_bold = !formatting.faux_bold;
                    true
                });
                true
            }
            ("toggle_text_faux_italic", _) => {
                self.update_selected_text_formatting(|formatting| {
                    formatting.faux_italic = !formatting.faux_italic;
                    true
                });
                true
            }
            ("toggle_text_all_caps", _) => {
                self.update_selected_text_formatting(|formatting| {
                    formatting.all_caps = !formatting.all_caps;
                    if formatting.all_caps {
                        formatting.small_caps = false;
                    }
                    true
                });
                true
            }
            ("toggle_text_small_caps", _) => {
                self.update_selected_text_formatting(|formatting| {
                    formatting.small_caps = !formatting.small_caps;
                    if formatting.small_caps {
                        formatting.all_caps = false;
                    }
                    true
                });
                true
            }
            ("text_box_width_changed", Some(ActionValue::Text(value))) => {
                if let Some(next) = value
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|value| value.is_finite())
                {
                    self.update_selected_text_formatting(|formatting| {
                        let next = next.clamp(0.0, 10000.0);
                        if formatting.box_width == next {
                            false
                        } else {
                            formatting.box_width = next;
                            true
                        }
                    });
                };
                true
            }
            ("text_box_height_changed", Some(ActionValue::Text(value))) => {
                if let Some(next) = value
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|value| value.is_finite())
                {
                    self.update_selected_text_formatting(|formatting| {
                        let next = next.clamp(0.0, 10000.0);
                        if formatting.box_height == next {
                            false
                        } else {
                            formatting.box_height = next;
                            true
                        }
                    });
                };
                true
            }
            ("toggle_text_animator", _) => {
                self.toggle_selected_text_animator();
                true
            }
            ("cycle_text_animator_shape", _) => {
                self.cycle_selected_text_animator_shape();
                true
            }
            ("toggle_text_animator_random_order", _) => {
                self.toggle_selected_text_animator_random_order();
                true
            }
            ("text_animator_expression_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_animator_expression(&value);
                true
            }
            ("text_animator_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter) = selected_row_id.as_deref() {
                    self.update_selected_text_animator_parameter(parameter, &value);
                };
                true
            }
            ("shape_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("width", &value);
                true
            }
            ("shape_width_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeWidth);
                true
            }
            ("shape_height_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("height", &value);
                true
            }
            ("shape_height_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeHeight);
                true
            }
            ("shape_corner_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("corner_radius", &value);
                true
            }
            ("shape_corner_radius_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeCornerRadius);
                true
            }
            ("shape_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("color", &value);
                true
            }
            ("cycle_shape_fill_type", _) => {
                self.cycle_selected_shape_fill_type();
                true
            }
            ("shape_gradient_start_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_color(false, &value);
                true
            }
            ("shape_gradient_end_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_color(true, &value);
                true
            }
            ("shape_gradient_angle_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_geometry("angle", &value);
                true
            }
            ("shape_gradient_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_geometry("radius", &value);
                true
            }
            ("toggle_shape_trim_paths", _) => {
                self.toggle_selected_shape_trim_paths();
                true
            }
            ("shape_trim_start_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_trim_path("start", &value);
                true
            }
            ("shape_trim_end_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_trim_path("end", &value);
                true
            }
            ("shape_trim_offset_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_trim_path("offset", &value);
                true
            }
            ("shape_trim_start_keyframe", _) => {
                self.add_selected_shape_trim_keyframe("start");
                true
            }
            ("shape_trim_end_keyframe", _) => {
                self.add_selected_shape_trim_keyframe("end");
                true
            }
            ("shape_trim_offset_keyframe", _) => {
                self.add_selected_shape_trim_keyframe("offset");
                true
            }
            ("toggle_shape_repeater", _) => {
                self.toggle_selected_shape_repeater();
                true
            }
            ("toggle_shape_repeater_composite", _) => {
                self.toggle_selected_shape_repeater_composite();
                true
            }
            ("shape_repeater_copies_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("copies", &value);
                true
            }
            ("shape_repeater_x_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("position_x", &value);
                true
            }
            ("shape_repeater_y_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("position_y", &value);
                true
            }
            ("shape_repeater_scale_x_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("scale_x", &value);
                true
            }
            ("shape_repeater_scale_y_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("scale_y", &value);
                true
            }
            ("shape_repeater_rotation_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("rotation", &value);
                true
            }
            ("shape_repeater_start_opacity_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("start_opacity", &value);
                true
            }
            ("shape_repeater_end_opacity_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_repeater("end_opacity", &value);
                true
            }
            ("shape_repeater_copies_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("copies");
                true
            }
            ("shape_repeater_x_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("position_x");
                true
            }
            ("shape_repeater_y_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("position_y");
                true
            }
            ("shape_repeater_scale_x_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("scale_x");
                true
            }
            ("shape_repeater_scale_y_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("scale_y");
                true
            }
            ("shape_repeater_rotation_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("rotation");
                true
            }
            ("shape_repeater_start_opacity_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("start_opacity");
                true
            }
            ("shape_repeater_end_opacity_keyframe", _) => {
                self.add_selected_shape_repeater_keyframe("end_opacity");
                true
            }
            ("shape_stroke_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("stroke_color", &value);
                true
            }
            ("shape_stroke_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("stroke_width", &value);
                true
            }
            ("shape_extrusion_depth_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("extrusion_depth", &value);
                true
            }
            ("shape_bevel_depth_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("bevel_depth", &value);
                true
            }
            ("shape_points_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("points", &value);
                true
            }
            ("shape_points_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapePoints);
                true
            }
            ("shape_inner_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("inner_radius", &value);
                true
            }
            ("shape_inner_radius_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeInnerRadius);
                true
            }
            ("shape_outer_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("outer_radius", &value);
                true
            }
            ("shape_outer_radius_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeOuterRadius);
                true
            }
            ("shape_sides_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("sides", &value);
                true
            }
            ("shape_sides_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeSides);
                true
            }
            ("shape_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("radius", &value);
                true
            }
            ("shape_radius_keyframe", _) => {
                self.add_selected_shape_keyframe(GraphChannel::ShapeRadius);
                true
            }
            _ => false,
        }
    }
}
