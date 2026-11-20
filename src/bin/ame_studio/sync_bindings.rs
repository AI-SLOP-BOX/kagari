use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_bindings_logged(&mut self) {
        if let Err(error) = self.sync_bindings() {
            log::error!("[kagari-studio] sync_bindings failed: {error}");
            #[cfg(debug_assertions)]
            {
                eprintln!("[kagari-studio:debug] sync_bindings failed: {error}");
            }
        }
    }

    pub(crate) fn sync_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.sync_environment_bindings()?;
        let _ = self.ui.set_binding(
            "engine_hub_status",
            Value::String(self.engine_hub_status.clone()),
        );
        let _ = self.ui.set_binding(
            "node_graph_status",
            Value::String(self.node_graph_status.clone()),
        );
        let _ = self.ui.set_binding(
            "node_graph_live_preview",
            Value::Bool(self.node_graph_live_preview),
        );
        let _ = self.ui.set_binding("effect_cat_all", Value::Bool(self.effect_category == "all"));
        let _ = self.ui.set_binding("effect_cat_color", Value::Bool(self.effect_category == "color"));
        let _ = self.ui.set_binding("effect_cat_blur", Value::Bool(self.effect_category == "blur"));
        let _ = self.ui.set_binding("effect_cat_distort", Value::Bool(self.effect_category == "distort"));
        let _ = self.ui.set_binding("effect_cat_key", Value::Bool(self.effect_category == "key"));
        if self.screen == Screen::Editor {
            self.ensure_composition_project_items();
            let visible_timeline_layers = self.sync_project_timeline_bindings()?;
            self.sync_scene_asset_bindings(visible_timeline_layers)?;

            self.sync_particle_layer_bindings()?;
            self.sync_graph_layer_bindings()?;
            self.sync_media_layer_bindings()?;
            self.sync_text_layer_bindings()?;

            self.sync_shape_bindings()?;
            self.sync_effect_bindings()?;
        } else {
            self.sync_recent_projects_bindings()?;
        }
        Ok(())
    }
}
