use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_layer_timing(&mut self, input: &str, is_in_point: bool) {
        let Ok(requested) = input.trim().parse::<u32>() else {
            return;
        };
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let max_frame = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        let (next_in, next_out) = if is_in_point {
            let next_in = requested
                .min(max_frame.saturating_sub(1))
                .min(layer.out_frame);
            (next_in, layer.out_frame)
        } else {
            let next_out = requested.clamp(layer.in_frame, max_frame);
            (layer.in_frame, next_out)
        };
        if layer.in_frame != next_in || layer.out_frame != next_out {
            layer.in_frame = next_in;
            layer.out_frame = next_out;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_selected_layer_style(&mut self, style_id: &str) {
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
        match style_id {
            "drop_shadow" => {
                let style = &mut layer.style.drop_shadow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 8.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 12.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.0, 0.0, 0.0, 1.0];
                    }
                }
            }
            "outer_glow" => {
                let style = &mut layer.style.outer_glow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 75.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 20.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 0.45, 0.12, 1.0];
                    }
                }
            }
            "stroke" => {
                let style = &mut layer.style.stroke;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.size <= 0.0 {
                        style.size = 4.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 1.0, 1.0, 1.0];
                    }
                }
            }
            "color_overlay" => {
                let style = &mut layer.style.color_overlay;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 100.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 0.45, 0.12, 1.0];
                    }
                }
            }
            "inner_shadow" => {
                let style = &mut layer.style.inner_shadow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 8.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 12.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.0, 0.0, 0.0, 1.0];
                    }
                }
            }
            "inner_glow" => {
                let style = &mut layer.style.inner_glow;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 70.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 16.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [1.0, 1.0, 1.0, 1.0];
                    }
                }
            }
            "satin" => {
                let style = &mut layer.style.satin;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.opacity <= 0.0 {
                        style.opacity = 50.0;
                    }
                    if style.distance <= 0.0 {
                        style.distance = 12.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 16.0;
                    }
                    if style.color[3] <= 0.0 {
                        style.color = [0.2, 0.2, 0.35, 1.0];
                    }
                }
            }
            "bevel_emboss" => {
                let style = &mut layer.style.bevel_emboss;
                style.enabled = !style.enabled;
                if style.enabled {
                    if style.depth <= 0.0 {
                        style.depth = 3.0;
                    }
                    if style.size <= 0.0 {
                        style.size = 3.0;
                    }
                }
            }
            "gradient_overlay" => {
                let style = &mut layer.style.gradient_overlay;
                style.enabled = !style.enabled;
                if style.enabled && style.opacity <= 0.0 {
                    style.opacity = 100.0;
                }
            }
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_layer_style_parameter(&mut self, parameter: &str, input: &str) {
        let Some((style_id, field)) = parameter.split_once('.') else {
            return;
        };
        let color = field.starts_with("color");
        let numeric = if color {
            None
        } else {
            input
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
        };
        if !color && numeric.is_none() {
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
        let mut changed = false;
        let mut set_color = |target: &mut [f32; 4]| {
            if let Some(next) = parse_hex_rgb(input, target[3].max(1.0)) {
                if *target != next {
                    *target = next;
                    changed = true;
                }
            }
        };
        macro_rules! set_scalar {
            ($target:expr, $min:expr, $max:expr) => {
                if let Some(value) = numeric {
                    let next = value.clamp($min, $max);
                    if ($target - next).abs() > f32::EPSILON {
                        $target = next;
                        changed = true;
                    }
                }
            };
        }
        match style_id {
            "drop_shadow" => {
                let style = &mut layer.style.drop_shadow;
                if field == "color" {
                    set_color(&mut style.color);
                } else if let Some(value) = numeric {
                    let target = match field {
                        "opacity" => Some((&mut style.opacity, value.clamp(0.0, 100.0))),
                        "angle" => Some((&mut style.angle, value.clamp(-360.0, 360.0))),
                        "distance" => Some((&mut style.distance, value.clamp(0.0, 2000.0))),
                        "size" => Some((&mut style.size, value.clamp(0.0, 500.0))),
                        _ => None,
                    };
                    if let Some((target, next)) = target {
                        if (*target - next).abs() > f32::EPSILON {
                            *target = next;
                            changed = true;
                        }
                    }
                }
            }
            "outer_glow" => {
                let style = &mut layer.style.outer_glow;
                if field == "color" {
                    set_color(&mut style.color);
                } else if let Some(value) = numeric {
                    let target = match field {
                        "opacity" => Some((&mut style.opacity, value.clamp(0.0, 100.0))),
                        "spread" => Some((&mut style.spread, value.clamp(0.0, 100.0))),
                        "size" => Some((&mut style.size, value.clamp(0.0, 500.0))),
                        _ => None,
                    };
                    if let Some((target, next)) = target {
                        if (*target - next).abs() > f32::EPSILON {
                            *target = next;
                            changed = true;
                        }
                    }
                }
            }
            "stroke" => {
                let style = &mut layer.style.stroke;
                if field == "color" {
                    set_color(&mut style.color);
                } else if field == "size" {
                    if let Some(value) = numeric {
                        let next = value.clamp(0.0, 500.0);
                        if (style.size - next).abs() > f32::EPSILON {
                            style.size = next;
                            changed = true;
                        }
                    }
                }
            }
            "color_overlay" => {
                let style = &mut layer.style.color_overlay;
                if field == "color" {
                    set_color(&mut style.color);
                } else if field == "opacity" {
                    set_scalar!(style.opacity, 0.0, 100.0);
                }
            }
            "inner_shadow" => {
                let style = &mut layer.style.inner_shadow;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "distance" => set_scalar!(style.distance, 0.0, 2000.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "inner_glow" => {
                let style = &mut layer.style.inner_glow;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "satin" => {
                let style = &mut layer.style.satin;
                match field {
                    "color" => set_color(&mut style.color),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "distance" => set_scalar!(style.distance, 0.0, 2000.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    _ => {}
                }
            }
            "bevel_emboss" => {
                let style = &mut layer.style.bevel_emboss;
                match field {
                    "color_light" => set_color(&mut style.color_light),
                    "color_dark" => set_color(&mut style.color_dark),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "depth" => set_scalar!(style.depth, 0.0, 500.0),
                    "size" => set_scalar!(style.size, 0.0, 500.0),
                    "highlight" => set_scalar!(style.highlight, 0.0, 100.0),
                    "shadow" => set_scalar!(style.shadow, 0.0, 100.0),
                    _ => {}
                }
            }
            "gradient_overlay" => {
                let style = &mut layer.style.gradient_overlay;
                match field {
                    "color_start" => set_color(&mut style.color_start),
                    "color_end" => set_color(&mut style.color_end),
                    "opacity" => set_scalar!(style.opacity, 0.0, 100.0),
                    "angle" => set_scalar!(style.angle, -360.0, 360.0),
                    "scale" => set_scalar!(style.scale, 1.0, 500.0),
                    _ => {}
                }
            }
            _ => {}
        }
        if changed {
            self.mark_project_changed();
        }
    }
}
