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
        self.ensure_composition_project_items();
        self.sync_environment_bindings()?;
        let visible_timeline_layers = self.sync_project_timeline_bindings()?;
        self.sync_scene_asset_bindings(visible_timeline_layers)?;

        self.sync_particle_layer_bindings()?;
        self.sync_graph_layer_bindings()?;
        self.sync_media_layer_bindings()?;
        self.sync_text_layer_bindings()?;

        self.sync_shape_bindings()?;
        self.sync_effect_bindings()?;
        self.sync_recent_projects_bindings()?;
        Ok(())
    }
}
