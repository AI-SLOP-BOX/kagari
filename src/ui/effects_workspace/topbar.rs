use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_topbar(
    ui: &mut egui::Ui,
    width: f32,
    mobile: bool,
    app: &mut KagariApp,
    ctx: &egui::Context,
) {
    let p = ui.painter().clone();
    for (x, c) in [
        (23.0, (255, 82, 78)),
        (46.0, (255, 190, 45)),
        (69.0, (42, 211, 86)),
    ] {
        p.circle_filled(
            egui::pos2(x, 23.0),
            6.5,
            egui::Color32::from_rgb(c.0, c.1, c.2),
        );
    }
    p.text(
        egui::pos2(96.0, 23.0),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    if !mobile && width >= 1100.0 {
        p.line_segment(
            [egui::pos2(270.0, 12.0), egui::pos2(270.0, 47.0)],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(290.0, 23.0),
            egui::Align2::LEFT_CENTER,
            "映像に、まだ見ぬ世界を。",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        let right = width - 42.0;
        let settings =
            egui::Rect::from_min_max(egui::pos2(right - 72.0, 5.0), egui::pos2(right, 55.0));
        let new_project = egui::Rect::from_min_max(
            egui::pos2(settings.left() - 152.0, 5.0),
            egui::pos2(settings.left() - 8.0, 55.0),
        );
        let open_project = egui::Rect::from_min_max(
            egui::pos2(new_project.left() - 182.0, 5.0),
            egui::pos2(new_project.left() - 8.0, 55.0),
        );
        effects_header_action(
            ui,
            open_project,
            icons::SVG_FOLDER,
            "プロジェクトを開く",
            "effects-open-project",
            app,
            ctx,
        );
        effects_header_action(
            ui,
            new_project,
            icons::SVG_FILE_PLUS,
            "新規プロジェクト",
            "effects-new-project",
            app,
            ctx,
        );
        effects_header_action(
            ui,
            settings,
            icons::SVG_SETTINGS,
            "設定",
            "effects-settings",
            app,
            ctx,
        );
        let close =
            egui::Rect::from_center_size(egui::pos2(width - 18.0, 23.0), egui::vec2(28.0, 32.0));
        if ui
            .interact(
                close,
                egui::Id::new("effects-window-close"),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        p.text(
            close.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(18.0),
            colors::TEXT_MUTED,
        );
    }
}

pub(crate) fn effects_header_action(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    icon: &'static str,
    label: &str,
    id: &str,
    app: &mut KagariApp,
    ctx: &egui::Context,
) {
    let response = ui.interact(rect, egui::Id::new(id), egui::Sense::click());
    if response.clicked() {
        match id {
            "effects-open-project" => crate::ui::home_screen::open_project_dialog(app),
            "effects-new-project" => crate::ui::home_screen::new_project_from_header(app),
            "effects-settings" => app.show_preferences = true,
            _ => {}
        }
        ctx.request_repaint();
    }
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.0,
            egui::Color32::from_rgba_unmultiplied(48, 61, 72, 110),
        );
    }
    icons::render_svg_at(
        ui,
        id.to_string(),
        icon,
        egui::vec2(20.0, 20.0),
        colors::TEXT_SECONDARY,
        egui::pos2(rect.left() + 8.0, rect.center().y - 10.0),
    );
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 36.0, rect.top()),
        egui::pos2(rect.right() - 4.0, rect.bottom()),
    );
    ui.put(
        text_rect,
        egui::Label::new(
            egui::RichText::new(label)
                .size(13.0)
                .color(colors::TEXT_SECONDARY),
        )
        .truncate(),
    );
}

