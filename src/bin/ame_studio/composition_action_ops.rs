use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_composition_action(
        &mut self,
        name: &str,
        value: Option<&ActionValue>,
        selected_row_id: Option<&str>,
    ) -> bool {
        match (name, value) {
            ("composition_width_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_setting("width", val);
                true
            }
            ("composition_height_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_setting("height", val);
                true
            }
            ("composition_preset_hd", _) => {
                self.set_composition_preset(1280, 720);
                true
            }
            ("composition_preset_full_hd", _) => {
                self.set_composition_preset(1920, 1080);
                true
            }
            ("composition_preset_4k", _) => {
                self.set_composition_preset(3840, 2160);
                true
            }
            ("composition_preset_vertical", _) => {
                self.set_composition_preset(1080, 1920);
                true
            }
            ("composition_preset_square", _) => {
                self.set_composition_preset(1080, 1080);
                true
            }
            ("composition_fps_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_setting("fps", val);
                true
            }
            ("composition_fps_24", _) => {
                self.update_composition_setting("fps", "24");
                true
            }
            ("composition_fps_25", _) => {
                self.update_composition_setting("fps", "25");
                true
            }
            ("composition_fps_30", _) => {
                self.update_composition_setting("fps", "30");
                true
            }
            ("composition_fps_50", _) => {
                self.update_composition_setting("fps", "50");
                true
            }
            ("composition_fps_60", _) => {
                self.update_composition_setting("fps", "60");
                true
            }
            ("composition_duration_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_setting("duration", val);
                true
            }
            ("composition_start_timecode_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_start_timecode(val);
                true
            }
            ("composition_pixel_aspect_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_float_setting("pixel_aspect", val);
                true
            }
            ("composition_shutter_angle_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_float_setting("shutter_angle", val);
                true
            }
            ("composition_shutter_phase_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_float_setting("shutter_phase", val);
                true
            }
            ("composition_background_changed", Some(ActionValue::Text(val))) => {
                self.update_composition_background(val);
                true
            }
            ("composition_proxy_full", _) => {
                self.set_composition_proxy_resolution(kagari_vfx::core::proxy::ProxyResolution::Full);
                true
            }
            ("composition_proxy_half", _) => {
                self.set_composition_proxy_resolution(kagari_vfx::core::proxy::ProxyResolution::Half);
                true
            }
            ("composition_proxy_quarter", _) => {
                self.set_composition_proxy_resolution(kagari_vfx::core::proxy::ProxyResolution::Quarter);
                true
            }
            ("composition_proxy_eighth", _) => {
                self.set_composition_proxy_resolution(kagari_vfx::core::proxy::ProxyResolution::Eighth);
                true
            }
            ("toggle_composition_proxy_preview", _) => {
                self.toggle_composition_proxy_preview();
                true
            }
            ("toggle_composition_blend_linear", _) => {
                self.toggle_composition_render_option(true);
                true
            }
            ("toggle_composition_dither_output", _) => {
                self.toggle_composition_render_option(false);
                true
            }
            ("composition_cycle_bit_depth", _) => {
                self.cycle_composition_bit_depth();
                true
            }
            ("composition_cycle_working_space", _) => {
                self.cycle_composition_working_space();
                true
            }
            ("composition_name_changed", Some(ActionValue::Text(val))) => {
                self.rename_active_composition(val);
                true
            }
            ("composition_previous", _) => {
                self.switch_composition(-1);
                true
            }
            ("composition_next", _) => {
                self.switch_composition(1);
                true
            }
            ("composition_select", _) => {
                if let Some(index) = selected_row_id.and_then(|id| id.parse::<usize>().ok()) {
                    self.activate_composition(index);
                }
                true
            }
            ("composition_add", _) => {
                self.add_composition();
                true
            }
            ("composition_duplicate", _) => {
                self.duplicate_active_composition();
                true
            }
            ("composition_remove", _) => {
                self.request_remove_active_composition();
                true
            }
            ("composition_remove_confirm", _) => {
                self.composition_remove_confirm_open = false;
                self.remove_active_composition();
                true
            }
            ("composition_remove_cancel", _) => {
                self.composition_remove_confirm_open = false;
                true
            }
            ("composition_zoom_in", _) => {
                self.adjust_composition_zoom(1.25);
                true
            }
            ("composition_zoom_out", _) => {
                self.adjust_composition_zoom(0.8);
                true
            }
            ("composition_zoom_fit", _) => {
                self.zoom = "100%".to_owned();
                self.zoom_manually_adjusted = true;
                self.viewport_pan = [0.0, 0.0];
                true
            }
            _ => false,
        }
    }
}
