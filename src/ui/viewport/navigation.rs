use super::*;

pub(super) fn handle_navigation(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    viewport_response: &egui::Response,
    rect: egui::Rect,
) {
    // ── Viewport zoom: scroll wheel scales magnification anchored at the pointer ──
    if viewport_response.hovered() {
        let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_y != 0.0 && !ui.input(|i| i.modifiers.command) {
            // Fit mode (0.0) resolves to a concrete ratio first so zooming feels continuous
            let current = if app.ui_tabs.viewport_mag_ratio == 0.0 {
                1.0
            } else {
                app.ui_tabs.viewport_mag_ratio
            };
            let factor = if scroll_y > 0.0 { 1.1 } else { 1.0 / 1.1 };
            let new_mag = (current * factor).clamp(0.05, 8.0);

            // Keep the content point under the cursor stationary:
            // screen_delta = content_pos * (new_scale - old_scale), where scale
            // is measured relative to the fit size.
            if let Some(pointer) = viewport_response.hover_pos() {
                let aspect = {
                    let comp = app.history.current().active_composition();
                    comp.width as f32 / comp.height as f32
                };
                let (fit_w, fit_h) = {
                    let mut fw = rect.width();
                    let mut fh = fw / aspect;
                    if fh > rect.height() {
                        fh = rect.height();
                        fw = fh * aspect;
                    }
                    (fw, fh)
                };
                let center = rect.center();
                // Content coordinate under the pointer at the OLD zoom
                let old_scale = if app.ui_tabs.viewport_mag_ratio == 0.0 {
                    1.0
                } else {
                    app.ui_tabs.viewport_mag_ratio
                };
                let content = egui::vec2(
                    (pointer.x - center.x) / (fit_w * old_scale),
                    (pointer.y - center.y) / (fit_h * old_scale),
                );
                // Pan adjustment keeps that content point at the pointer
                app.playback.viewport_pan.x += content.x * fit_w * (old_scale - new_mag);
                app.playback.viewport_pan.y += content.y * fit_h * (old_scale - new_mag);
            }
            app.ui_tabs.viewport_mag_ratio = new_mag;
        }
        // Middle-drag pans regardless of active tool (AE behavior)
        if ui.input(|i| i.pointer.middle_down()) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Hand;
        }

        // ── Hand tool drag pans the view ──
        if app.active_tool == crate::ui::toolbar::ActiveTool::Hand {
            let delta = viewport_response.drag_delta();
            app.playback.viewport_pan.x += delta.x;
            app.playback.viewport_pan.y += delta.y;
            if viewport_response.hovered() {
                ui.output_mut(|o| {
                    o.cursor_icon = if viewport_response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    }
                });
            }
        }

        // ── Zoom tool: click zooms in 2x at pointer, Alt-click out ──
        if app.active_tool == crate::ui::toolbar::ActiveTool::Zoom {
            if viewport_response.clicked() || viewport_response.secondary_clicked() {
                let zoom_in =
                    !ui.input(|i| i.modifiers.alt) && !viewport_response.secondary_clicked();
                let factor = if zoom_in { 2.0 } else { 0.5 };
                let current = if app.ui_tabs.viewport_mag_ratio == 0.0 {
                    1.0
                } else {
                    app.ui_tabs.viewport_mag_ratio
                };
                let new_mag = (current * factor).clamp(0.05, 8.0);
                if let Some(pointer) = viewport_response.interact_pointer_pos() {
                    let aspect2 = app.history.current().active_composition().width as f32
                        / app.history.current().active_composition().height as f32;
                    let mut fw = rect.width();
                    let mut fh = fw / aspect2;
                    if fh > rect.height() {
                        fh = rect.height();
                        fw = fh * aspect2;
                    }
                    let center = rect.center();
                    let content = egui::vec2(
                        (pointer.x - center.x) / (fw * current),
                        (pointer.y - center.y) / (fh * current),
                    );
                    app.playback.viewport_pan.x += content.x * fw * (current - new_mag);
                    app.playback.viewport_pan.y += content.y * fh * (current - new_mag);
                }
                app.ui_tabs.viewport_mag_ratio = new_mag;
            }
            if viewport_response.hovered() {
                ui.output_mut(|o| {
                    o.cursor_icon = if ui.input(|i| i.modifiers.alt) {
                        egui::CursorIcon::ZoomOut
                    } else {
                        egui::CursorIcon::ZoomIn
                    }
                });
            }
        }
        // ── Cursor feedback for Selection / Rotation / AnchorPoint / Pen ──
        if viewport_response.hovered()
            && app.viewport_drag_state.is_none()
            && app.viewport_multi_drag.is_none()
        {
            match app.active_tool {
                crate::ui::toolbar::ActiveTool::Selection => {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                }
                crate::ui::toolbar::ActiveTool::Rotation => {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                }
                crate::ui::toolbar::ActiveTool::AnchorPoint => {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
                }
                crate::ui::toolbar::ActiveTool::Pen
                | crate::ui::toolbar::ActiveTool::Brush
                | crate::ui::toolbar::ActiveTool::Eraser => {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
                }
                _ => {}
            }
        }
    }
}
