use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn invoke(app: &mut AfterEffectsStudio, name: &str, value: Option<ActionValue>) {
        app.handle_action(HostAction {
            name: name.to_owned(),
            value,
            target: None,
        })
        .unwrap();
    }

    #[path = "ameui_reload.rs"]
    mod ameui_reload;
    #[path = "camera_projects.rs"]
    mod camera_projects;
    #[path = "color_grading.rs"]
    mod color_grading;
    #[path = "graph_navigation_tracker.rs"]
    mod graph_navigation_tracker;
    #[path = "playback_actions.rs"]
    mod playback_actions;
    #[path = "preview_cache.rs"]
    mod preview_cache;
    #[path = "roto_brush.rs"]
    mod roto_brush;
    #[path = "ui_render.rs"]
    mod ui_render;
    #[path = "ui_render_e2e.rs"]
    mod ui_render_e2e;
}
