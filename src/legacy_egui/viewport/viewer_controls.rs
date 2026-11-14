use super::*;

pub(super) fn draw_composition_tabs(app: &mut KagariApp, ui: &mut egui::Ui) {
    // ── AE Viewport Composition Top Tabs & Breadcrumbs Bar ──
    if app.history.current().active_composition().id != "comp_empty_start"
        && app.history.current().compositions.len() > 1
    {
        let active_comp_idx = app.history.current().active_composition_idx;
        let comps_count = app.history.current().compositions.len();
        ui.allocate_ui_with_layout(
            egui::vec2(
                ui.available_width(),
                crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
            ),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("viewer_composition_tabs")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for idx in 0..comps_count {
                                let is_active = idx == active_comp_idx;
                                let c_name = app.history.current().compositions[idx].name.clone();
                                if crate::ui::theme::draw_custom_tab(ui, is_active, &c_name)
                                    .clicked()
                                    && idx != active_comp_idx
                                {
                                    app.dispatch(crate::app_state::AppCommand::SelectComposition(
                                        idx,
                                    ));
                                }
                            }
                        });
                    });
            },
        );
        crate::ui::theme::draw_separator(ui);
    }
}

pub(super) fn draw_overlay_controls(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    rect: egui::Rect,
    mut viewport_response: egui::Response,
) -> egui::Response {
    // Floating Modern Tool Strip (Glassmorphism Pill directly inside viewer top-left)
    let tool_rect = {
        let tool_strip_w = 34.0_f32;
        let tool_strip_h = 240.0_f32;
        let float_margin = 12.0_f32;
        let tool_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(float_margin, float_margin),
            egui::vec2(tool_strip_w, tool_strip_h),
        );
        ui.painter().rect_filled(
            tool_rect,
            8.0,
            egui::Color32::from_rgba_premultiplied(16, 18, 24, 230),
        );
        ui.painter().rect_stroke(
            tool_rect,
            8.0,
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        let mut tool_ui = ui.new_child(egui::UiBuilder::new().max_rect(tool_rect));
        crate::ui::toolbar::draw_viewer_tool_strip_vertical(app, &mut tool_ui);
        tool_rect
    };

    // If pointer is inside the floating tool strip, do not allow canvas viewport_response
    // to steal or trigger pan/zoom/selection clicks.
    let pointer_over_tool_strip = ui.input(|i| {
        i.pointer
            .hover_pos()
            .is_some_and(|pos| tool_rect.contains(pos))
            || i.pointer
                .interact_pos()
                .is_some_and(|pos| tool_rect.contains(pos))
    });
    if pointer_over_tool_strip {
        viewport_response.clicked = false;
        viewport_response.dragged = false;
        viewport_response.drag_started = false;
    }
    viewport_response.context_menu(|ui| {
        let layer_count = app.history.current().active_composition().layers.len();
        if ui.button("Select All Layers").clicked() {
            app.selection.selected_layers = (0..layer_count).collect();
            app.selection.selected_layer_idx = (layer_count > 0).then_some(0);
            ui.close_menu();
        }
        if ui.button("Deselect Layers").clicked() {
            app.selection.selected_layers.clear();
            app.selection.selected_layer_idx = None;
            app.mask_selected_vertices = None;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Fit Composition to Viewer").clicked() {
            app.ui_tabs.viewport_mag_ratio = 0.0;
            app.playback.viewport_pan = egui::Vec2::ZERO;
            ui.close_menu();
        }
        ui.checkbox(&mut app.show_grid, "Show Grid");
        ui.checkbox(&mut app.show_guides, "Show Guides");
        ui.checkbox(&mut app.snap_to_keyframes, "Snap to Keyframes");
    });
    viewport_response
}
