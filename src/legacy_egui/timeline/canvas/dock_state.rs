use crate::KagariApp;
use eframe::egui;

pub(super) fn draw_special_state(app: &mut KagariApp, ui: &mut egui::Ui) -> bool {
    if app.ui_tabs.bottom_dock_tab != 1 && app.history.current().has_pristine_start_composition() {
        ui.allocate_ui_with_layout(
            egui::vec2(
                ui.available_width(),
                crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
            ),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| crate::ui::kui::title(ui, "タイムライン"),
        );
        crate::ui::theme::draw_separator(ui);
        crate::ui::kui::empty_state(
            ui,
            "コンポジションを作成するか、素材を追加するとここに表示されます",
            ui.available_height().max(56.0),
        );
        return true;
    }

    if app.ui_tabs.bottom_dock_tab == 1 {
        ui.allocate_ui_with_layout(
            egui::vec2(
                ui.available_width(),
                crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
            ),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                if crate::ui::theme::draw_custom_tab(ui, false, "タイムライン").clicked() {
                    app.ui_tabs.bottom_dock_tab = 0;
                }
                let _ = crate::ui::theme::draw_custom_tab(ui, true, "レンダーキュー");
            },
        );
        crate::ui::theme::draw_separator(ui);
        egui::ScrollArea::vertical()
            .id_salt("render_queue_content")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::ui::render_queue::draw_render_queue_panel(app, ui)
            });
        return true;
    }

    false
}
