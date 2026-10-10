//! Top and bottom header/footer bars for tutorial workspace.

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_topbar(
    ui: &mut egui::Ui,
    width: f32,
    mobile: bool,
    app: &mut KagariApp,
    ctx: &egui::Context,
) {
    if !mobile && width >= 1100.0 {
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
        tutorial_header_action(
            ui,
            open_project,
            icons::SVG_FOLDER,
            "プロジェクトを開く",
            "tutorial-open-project",
            app,
            ctx,
        );
        tutorial_header_action(
            ui,
            new_project,
            icons::SVG_FILE_PLUS,
            "新規プロジェクト",
            "tutorial-new-project",
            app,
            ctx,
        );
        tutorial_header_action(
            ui,
            settings,
            icons::SVG_SETTINGS,
            "設定",
            "tutorial-settings",
            app,
            ctx,
        );
    }
    let p = ui.painter().clone();
    p.text(
        egui::pos2(96.0, 24.0),
        egui::Align2::CENTER_CENTER,
        "Kagari VFX",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    if !mobile && width >= 1100.0 {
        p.line_segment(
            [egui::pos2(264.0, 14.0), egui::pos2(264.0, 45.0)],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(291.0, 24.0),
            egui::Align2::LEFT_CENTER,
            "映像に、まだ見ぬ世界を。",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        let close =
            egui::Rect::from_center_size(egui::pos2(width - 18.0, 24.0), egui::vec2(28.0, 32.0));
        if ui
            .interact(
                close,
                egui::Id::new("tutorial-window-close"),
                egui::Sense::click(),
            )
            .clicked()
        {
            app.show_guided_tutorial = false;
            app.show_home = true;
            ctx.request_repaint();
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

fn tutorial_header_action(
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
            "tutorial-open-project" => crate::ui::home_screen::open_project_dialog(app),
            "tutorial-new-project" => crate::ui::home_screen::new_project_from_header(app),
            "tutorial-settings" => app.show_preferences = true,
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
    let icon_size = egui::vec2(20.0, 20.0);
    icons::render_svg_at(
        ui,
        id.to_string(),
        icon,
        icon_size,
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

pub fn draw_footer(ui: &mut egui::Ui, width: f32, mobile: bool) {
    let p = ui.painter();
    if mobile {
        p.text(
            egui::pos2(18.0, 19.0),
            egui::Align2::LEFT_CENTER,
            "Kagari VFX  |  チュートリアル",
            egui::FontId::proportional(10.0),
            colors::TEXT_SECONDARY,
        );
        return;
    }
    p.text(
        egui::pos2(38.0, 34.0),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX  |  映像で、まだ見ぬ世界を。",
        egui::FontId::proportional(11.0),
        colors::TEXT_SECONDARY,
    );
    p.line_segment(
        [
            egui::pos2(width * 0.325, 34.0),
            egui::pos2(width * 0.39, 34.0),
        ],
        egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY),
    );
    p.text(
        egui::pos2(width * 0.5, 34.0),
        egui::Align2::CENTER_CENTER,
        "チュートリアル画面",
        egui::FontId::proportional(25.0),
        colors::TEXT_PRIMARY,
    );
    p.line_segment(
        [
            egui::pos2(width * 0.61, 34.0),
            egui::pos2(width * 0.675, 34.0),
        ],
        egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY),
    );
    p.text(
        egui::pos2(width - 38.0, 34.0),
        egui::Align2::RIGHT_CENTER,
        "VFX  /  合成  /  表現の可能性",
        egui::FontId::proportional(9.0),
        colors::TEXT_MUTED,
    );
}
