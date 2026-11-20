use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_editor_workspace_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("toggle_command_palette", _) => {
                self.command_palette_open = !self.command_palette_open;
                true
            }
            ("execute_command", Some(ActionValue::Text(command_id))) => {
                self.command_palette_open = false;
                if !self.handle_editor_workspace_action(&command_id, None)
                    && !self.handle_navigation_setup_action(&command_id, None)
                    && !self.handle_engine_hub_action(&command_id)
                {
                    self.handle_effect_action(&command_id, None, None);
                }
                true
            }
            ("back_to_editor", _) | ("timeline_tab", _) => {
                self.screen = Screen::Editor;
                self.render_queue = false;
                true
            }
            ("tool_select", _) => {
                self.selected_tool = "Selection";
                true
            }
            ("tool_hand", _) => {
                self.selected_tool = "Hand";
                true
            }
            ("tool_zoom", _) => {
                self.selected_tool = "Zoom";
                true
            }
            ("tool_text", _) => {
                self.selected_tool = "Text";
                true
            }
            ("tool_shape", _) => {
                self.selected_tool = "Shape";
                true
            }
            ("tool_pen", _) => {
                self.selected_tool = "Pen";
                true
            }
            ("tool_roto", _) => {
                self.selected_tool = "RotoBrush";
                true
            }
            ("roto_mode_foreground", _) => {
                self.roto_foreground = true;
                self.roto_status = "前景ストロークを選択".to_owned();
                true
            }
            ("roto_mode_background", _) => {
                self.roto_foreground = false;
                self.roto_status = "背景ストロークを選択".to_owned();
                true
            }
            ("roto_brush_radius_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.roto_brush_radius = value.clamp(1.0, 128.0);
                true
            }
            ("roto_feather_radius_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.roto_feather_radius = value.clamp(0.0, 32.0);
                true
            }
            ("roto_clear", _) => {
                self.clear_roto_brush();
                true
            }
            ("roto_propagate", _) => {
                if let Err(error) = self.start_roto_propagation() {
                    self.roto_status = error;
                };
                true
            }
            ("roto_cancel", _) => {
                self.cancel_roto_jobs();
                true
            }
            ("workspace_default", _) => {
                self.workspace = Workspace::Default;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_composition", _) => {
                self.workspace = Workspace::Composition;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_scene", _) => {
                self.workspace = Workspace::Scene;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_timeline", _) => {
                self.workspace = Workspace::Timeline;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_animation", _) => {
                self.workspace = Workspace::Animation;
                self.timeline_view = TimelineView::Animation;
                true
            }
            ("workspace_graph", _) => {
                self.workspace = Workspace::Graph;
                self.timeline_view = TimelineView::Graph;
                true
            }
            ("workspace_effects", _) => {
                self.workspace = Workspace::Effects;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_audio", _) => {
                self.workspace = Workspace::Audio;
                self.timeline_view = TimelineView::Audio;
                true
            }
            ("workspace_time_remap", _) => {
                self.workspace = Workspace::TimeRemap;
                self.timeline_view = TimelineView::Media;
                true
            }
            ("workspace_mask", _) => {
                self.workspace = Workspace::Mask;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_assets", _) => {
                self.workspace = Workspace::Assets;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_text", _) => {
                self.workspace = Workspace::Text;
                self.timeline_view = TimelineView::Text;
                true
            }
            ("workspace_layer_style", _) => {
                self.workspace = Workspace::LayerStyle;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_color", _) => {
                self.workspace = Workspace::Color;
                self.timeline_view = TimelineView::All;
                true
            }
            ("workspace_camera_light", _) => {
                self.workspace = Workspace::CameraLight;
                self.timeline_view = TimelineView::All;
                true
            }
            _ => false,
        }
    }
}
