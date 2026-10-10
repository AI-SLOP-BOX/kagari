use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_camera_views(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::kui::section(ui, "3Dビューポート");

    let layout_grid = crate::ui::kui::PropertyGrid::new(ui, &["ビュー配置"]);
    layout_grid.row(ui, "ビュー配置", |ui| {
        ui.horizontal(|ui| {
            for (layout, label) in [
                (0, "単一ビュー"),
                (1, "左右分割"),
                (2, "上下分割"),
                (3, "4ビュー"),
            ] {
                if layout_button(ui, layout, app.camera_view_layout == layout, label).clicked() {
                    app.camera_view_layout = layout;
                }
            }
        });
    });

    // Viewport preview (visual indicator of split layout)
    let preview_rect = ui
        .allocate_space(egui::vec2(ui.available_width().min(200.0), 120.0))
        .1;
    ui.painter()
        .rect_filled(preview_rect, 4.0, colors::BG_DEEPEST);
    ui.painter().rect_stroke(
        preview_rect,
        4.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );

    let view_names = [
        "アクティブ",
        "正面",
        "左",
        "上",
        "右",
        "背面",
        "下",
        "カスタム 1",
        "カスタム 2",
        "カスタム 3",
    ];
    match app.camera_view_layout {
        0 => {
            // Single view
            ui.painter().text(
                preview_rect.center(),
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(view_names.len() - 1)],
                egui::FontId::proportional(11.0),
                colors::TEXT_SECONDARY,
            );
        }
        1 => {
            // Horizontal split
            let mid = preview_rect.center().x;
            ui.painter().line_segment(
                [
                    egui::pos2(mid, preview_rect.top()),
                    egui::pos2(mid, preview_rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
            );
            let left_center = egui::pos2(
                preview_rect.left() + preview_rect.width() * 0.25,
                preview_rect.center().y,
            );
            let right_center = egui::pos2(
                preview_rect.left() + preview_rect.width() * 0.75,
                preview_rect.center().y,
            );
            ui.painter().text(
                left_center,
                egui::Align2::CENTER_CENTER,
                view_names[0],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                right_center,
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(view_names.len() - 1)],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
        }
        2 => {
            // Vertical split
            let mid = preview_rect.center().y;
            ui.painter().line_segment(
                [
                    egui::pos2(preview_rect.left(), mid),
                    egui::pos2(preview_rect.right(), mid),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
            );
            let top_center = egui::pos2(
                preview_rect.center().x,
                preview_rect.top() + preview_rect.height() * 0.25,
            );
            let bot_center = egui::pos2(
                preview_rect.center().x,
                preview_rect.top() + preview_rect.height() * 0.75,
            );
            ui.painter().text(
                top_center,
                egui::Align2::CENTER_CENTER,
                view_names[0],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                bot_center,
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(view_names.len() - 1)],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
        }
        3 => {
            // 4 views
            let cx = preview_rect.center().x;
            let cy = preview_rect.center().y;
            ui.painter().line_segment(
                [
                    egui::pos2(cx, preview_rect.top()),
                    egui::pos2(cx, preview_rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(preview_rect.left(), cy),
                    egui::pos2(preview_rect.right(), cy),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
            );
            let positions = [
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.25,
                    preview_rect.top() + preview_rect.height() * 0.25,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.75,
                    preview_rect.top() + preview_rect.height() * 0.25,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.25,
                    preview_rect.top() + preview_rect.height() * 0.75,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.75,
                    preview_rect.top() + preview_rect.height() * 0.75,
                ),
            ];
            let quad_names = ["上", "正面", "左", "アクティブ"];
            for (pos, name) in positions.iter().zip(quad_names.iter()) {
                ui.painter().text(
                    *pos,
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(8.0),
                    colors::TEXT_MUTED,
                );
            }
        }
        _ => {}
    }

    ui.add_space(crate::ui::kui::space::XS);
    crate::ui::kui::section(ui, "アクティブビュー");
    let view_labels = [
        (0_usize, "アクティブカメラ"),
        (1, "正面"),
        (2, "左"),
        (3, "上"),
        (4, "右"),
        (5, "背面"),
        (6, "下"),
        (7, "カスタムビュー 1"),
        (8, "カスタムビュー 2"),
        (9, "カスタムビュー 3"),
    ];
    let active_view_grid = crate::ui::kui::PropertyGrid::new(ui, &["ビュー"]);
    active_view_grid.row(ui, "ビュー", |ui| {
        crate::ui::kui::combo(
            ui,
            "camera_active_view",
            &mut app.camera_view_angle,
            &view_labels,
        )
    });
}

fn layout_button(ui: &mut egui::Ui, layout: usize, selected: bool, hint: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 28.0), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect_filled(rect, 5.0, visuals.weak_bg_fill);
    ui.painter().rect_stroke(rect, 5.0, visuals.bg_stroke);
    let inner = rect.shrink2(egui::vec2(8.0, 6.0));
    let stroke = egui::Stroke::new(
        1.5_f32,
        if selected {
            colors::ACCENT_CYAN
        } else {
            colors::TEXT_SECONDARY
        },
    );
    ui.painter().rect_stroke(inner, 1.5, stroke);
    let center = inner.center();
    match layout {
        1 => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x, inner.top()),
                    egui::pos2(center.x, inner.bottom()),
                ],
                stroke,
            );
        }
        2 => {
            ui.painter().line_segment(
                [
                    egui::pos2(inner.left(), center.y),
                    egui::pos2(inner.right(), center.y),
                ],
                stroke,
            );
        }
        3 => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x, inner.top()),
                    egui::pos2(center.x, inner.bottom()),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(inner.left(), center.y),
                    egui::pos2(inner.right(), center.y),
                ],
                stroke,
            );
        }
        _ => {}
    }
    response.on_hover_text(hint)
}
