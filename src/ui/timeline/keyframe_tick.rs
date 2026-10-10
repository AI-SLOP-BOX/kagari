use super::geometry::TimelineViewport;
use crate::core::keyframe::InterpolationType;
use crate::ui::theme::colors;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeTickResult {
    None,
    Clicked { shift: bool, cmd: bool },
    RightClicked,
    Dragged { new_frame: u32 },
}

#[allow(clippy::too_many_arguments)]
pub fn draw_keyframe_tick(
    ui: &mut egui::Ui,
    viewport: TimelineViewport,
    is_sub_prop: bool,
    current_frame: &mut u32,
    kf_frame: u32,
    interpolation: Option<InterpolationType>,
    is_selected: bool,
) -> (KeyframeTickResult, egui::Response) {
    let x = viewport.frame_to_x(kf_frame);
    let y = viewport.rect.center().y;
    let size = if is_sub_prop { 5.0 } else { 7.0 };
    let rect = egui::Rect::from_center_size(
        egui::pos2(x, y),
        egui::vec2(size * 2.0 + 4.0, size * 2.0 + 4.0),
    );
    let color = if is_selected {
        colors::ACCENT_SELECTION
    } else if *current_frame == kf_frame {
        colors::TIMELINE_KEYFRAME
    } else {
        colors::TEXT_SECONDARY
    };

    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let is_hovered = response.hovered();

    let painter = ui.painter();
    let is_linear = matches!(interpolation, Some(InterpolationType::Linear));
    let is_hold = matches!(interpolation, Some(InterpolationType::Hold));

    if is_linear {
        painter.circle_filled(egui::pos2(x, y), size * 0.8, color);
    } else if is_hold {
        painter.rect_filled(
            egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 1.4, size * 1.4)),
            1.0,
            color,
        );
    } else {
        let points = [
            egui::pos2(x, y - size),
            egui::pos2(x + size, y),
            egui::pos2(x, y + size),
            egui::pos2(x - size, y),
        ];
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color,
            egui::Stroke::NONE,
        ));
    }

    if is_hovered || is_selected {
        painter.circle_filled(
            egui::pos2(x, y),
            size + 3.5,
            egui::Color32::from_rgba_premultiplied(255, 140, 0, if is_selected { 60 } else { 35 }),
        );
        painter.circle_stroke(
            egui::pos2(x, y),
            size + 2.0,
            egui::Stroke::new(
                1.5_f32,
                if is_selected {
                    colors::ACCENT_SELECTION
                } else {
                    colors::ACCENT_BRAND_HOVER
                },
            ),
        );
    }
    let interpolation_name = match interpolation {
        Some(InterpolationType::Linear) => "Linear",
        Some(InterpolationType::Hold) => "Hold",
        Some(InterpolationType::Bezier {
            custom_bezier: Some(_),
            ..
        }) => "Bezier (custom)",
        Some(InterpolationType::Bezier { .. }) => "Bezier / Ease",
        None => "Keyframe",
    };
    response
        .clone()
        .on_hover_text(format!("Frame {} · {}", kf_frame, interpolation_name));

    if response.secondary_clicked() {
        return (KeyframeTickResult::RightClicked, response);
    }
    if response.clicked() {
        *current_frame = kf_frame;
        let modifiers = ui.input(|input| input.modifiers);
        return (
            KeyframeTickResult::Clicked {
                shift: modifiers.shift,
                cmd: modifiers.command || modifiers.ctrl,
            },
            response,
        );
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let new_frame = viewport.x_to_frame(pointer.x);
            if new_frame != kf_frame {
                return (KeyframeTickResult::Dragged { new_frame }, response);
            }
        }
    }
    (KeyframeTickResult::None, response)
}
