use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use super::effects_io::*;

pub fn draw_look_presets(app: &mut KagariApp, ui: &mut egui::Ui) {
    let looks: [LookPreset<'_>; 5] = [
        (
            "Sunset",
            [25.0, 10.0, -15.0],
            [10.0, 0.0, -5.0],
            [-10.0, -5.0, 20.0],
        ),
        (
            "Teal & Orange",
            [-12.0, 6.0, 16.0],
            [-5.0, 0.0, 10.0],
            [15.0, 5.0, -22.0],
        ),
        (
            "Film",
            [20.0, 18.0, 24.0],
            [5.0, 5.0, 5.0],
            [-8.0, -8.0, -6.0],
        ),
        (
            "Arctic",
            [-6.0, 0.0, 18.0],
            [-3.0, 0.0, 8.0],
            [-5.0, 2.0, 14.0],
        ),
        (
            "Ember",
            [30.0, 8.0, -25.0],
            [12.0, -2.0, -10.0],
            [5.0, -5.0, -18.0],
        ),
    ];
    for (name, shadows, midtones, highlights) in looks {
        if crate::ui::kui::button(ui, name, crate::ui::kui::ButtonKind::Default).clicked() {
            write_cb(app, shadows, midtones, highlights, true);
            app.toasts.info(format!("{name} を適用しました"));
        }
    }
    if crate::ui::kui::button(ui, "Reset to Neutral", crate::ui::kui::ButtonKind::Quiet).clicked() {
        write_cb(app, [0.0; 3], [0.0; 3], [0.0; 3], true);
    }
}

/// Interactive RGB wheel: drag edits R/G, Shift+drag edits B. Returns delta.
pub(crate) fn wheel_widget(ui: &mut egui::Ui, label: &str, arr: &mut [f32; 3]) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(crate::ui::localization::tr(label))
                .small()
                .color(colors::TEXT_PRIMARY),
        );
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(74.0, 74.0), egui::Sense::drag());
        let painter = ui.painter();
        painter.rect_filled(rect, 6.0, colors::BG_DEEPEST);
        painter.circle_stroke(rect.center(), 30.0, (1.5, colors::TEXT_SECONDARY));
        painter.line_segment(
            [
                egui::pos2(rect.left() + 6.0, rect.center().y),
                egui::pos2(rect.right() - 6.0, rect.center().y),
            ],
            (0.5, colors::TEXT_MUTED),
        );
        painter.line_segment(
            [
                egui::pos2(rect.center().x, rect.top() + 6.0),
                egui::pos2(rect.center().x, rect.bottom() - 6.0),
            ],
            (0.5, colors::TEXT_MUTED),
        );
        if resp.dragged() {
            let d = resp.drag_delta();
            arr[0] = (arr[0] + d.x).clamp(-100.0, 100.0);
            if ui.input(|i| i.modifiers.shift) {
                arr[2] = (arr[2] - d.y).clamp(-100.0, 100.0);
            } else {
                arr[1] = (arr[1] - d.y).clamp(-100.0, 100.0);
            }
            changed = true;
        }
        let off = egui::vec2(arr[0] / 100.0 * 30.0, -arr[1] / 100.0 * 30.0);
        painter.circle_filled(rect.center() + off, 4.5, colors::ACCENT_BLUE);
        ui.label(
            egui::RichText::new(format!("R{:+.0} G{:+.0} B{:+.0}", arr[0], arr[1], arr[2]))
                .small()
                .color(colors::TEXT_MUTED),
        );
    });
    changed
}

