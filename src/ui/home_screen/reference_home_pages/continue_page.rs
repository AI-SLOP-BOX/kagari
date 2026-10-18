use super::*;

pub(in crate::ui::home_screen) fn draw_reference_continue_body(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    compact: bool,
    vertical: bool,
    preview_height: f32,
) {
    if vertical {
        let height = (ui.available_width() / 1.82).clamp(76.0, preview_height);
        draw_reference_continue_preview(app, ctx, ui, height);
        ui.add_space(8.0);
        draw_reference_continue_details(app, ctx, ui, compact);
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let preview_width = ui.available_width() * if compact { 0.54 } else { 0.567 };
            ui.allocate_ui(egui::vec2(preview_width, preview_height), |ui| {
                draw_reference_continue_preview(app, ctx, ui, preview_height);
            });
            ui.add_space(if compact { 12.0 } else { 24.0 });
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width().max(1.0), preview_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| draw_reference_continue_details(app, ctx, ui, compact),
            );
        });
    }
}

pub(in crate::ui::home_screen) fn draw_reference_continue_preview(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    height: f32,
) {
    if let Some(id) = reference_texture(app, ctx, "continue_working_preview.webp") {
        let image_width = ui.available_width().max(1.0);
        egui::Frame::none()
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(54, 72, 91),
            ))
            .rounding(5.0)
            .show(ui, |ui| {
                reference_cover_image(ui, id, egui::vec2(image_width, height), 406.0 / 189.0);
            });
    }
}

pub(in crate::ui::home_screen) fn draw_reference_continue_details(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    compact: bool,
) {
    ui.label(
        egui::RichText::new("Eclipse")
            .size(if compact { 18.0 } else { 20.0 })
            .strong(),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "comp-meta",
            crate::ui::icons::SVG_COMP,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("SH_0140 / portal_final_v18")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.horizontal_wrapped(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "resolution-meta",
            crate::ui::icons::SVG_RESOLUTION,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("3840 × 2160")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
        ui.add_space(if compact { 4.0 } else { 12.0 });
        crate::ui::icons::render_svg_bytes(
            ui,
            "frame-meta",
            crate::ui::icons::SVG_FRAME,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("1001 – 1148")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "clock-meta",
            crate::ui::icons::SVG_CLOCK,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("Edited 18 minutes ago")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.add_space(if compact { 4.0 } else { 13.0 });
    if reference_cta_button(ui, "Resume Composition", compact).clicked() {
        if let Some(top) = recent_project_entries(app, ctx)
            .into_iter()
            .find(|e| e.path.is_file())
        {
            open_project_path(app, &top.path);
        }
    }
}
