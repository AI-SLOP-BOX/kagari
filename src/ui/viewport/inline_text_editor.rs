use super::*;

pub(super) struct InlineTextEditorGeometry {
    pub(super) current_frame: u32,
    pub(super) origin: [f32; 2],
    pub(super) viewport_size: [f32; 2],
    pub(super) composition_size: [f32; 2],
}

pub(super) fn draw_inline_text_editor(
    app: &mut KagariApp,
    ctx: &egui::Context,
    geometry: InlineTextEditorGeometry,
) {
    let InlineTextEditorGeometry {
        current_frame,
        origin: [origin_x, origin_y],
        viewport_size: [draw_w, draw_h],
        composition_size: [comp_w, comp_h],
    } = geometry;
    let Some(idx) = app.inline_text_edit_layer else {
        return;
    };

    let (seed, screen_x, screen_y, layer_w) = {
        let comp = app.history.current().active_composition();
        let fps = comp.fps;
        let Some(l) = comp.layers.get(idx) else {
            app.inline_text_edit_layer = None;
            return;
        };
        let seed_text = match &l.layer_type {
            crate::core::timeline::LayerType::Text { text, .. } => text.clone(),
            _ => String::new(),
        };

        let pos = l.transform.eval_position(current_frame, fps);

        // Convert comp-space position to screen-space.
        let screen_x = origin_x + (pos[0] / comp_w) * draw_w;
        let screen_y = origin_y + (pos[1] / comp_h) * draw_h;

        let font_size = match &l.layer_type {
            crate::core::timeline::LayerType::Text { font_size, .. } => *font_size as f32,
            _ => 24.0,
        };
        let layer_w = (font_size * 20.0).max(80.0);

        (seed_text, screen_x, screen_y, layer_w)
    };

    let buf_id = egui::Id::new(("inline_text_buf", idx));
    let mut buf = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(buf_id, || seed.clone())
            .clone()
    });
    let mut should_close = false;
    let mut should_commit = false;

    let area_pos = egui::pos2(screen_x, screen_y);

    let response = egui::Area::new(egui::Id::new(("inline_text_area", idx)))
        .fixed_pos(area_pos)
        .show(ctx, |ui| {
            let frame_bg = ui.visuals().widgets.noninteractive.bg_fill;
            let frame_stroke = egui::Stroke::new(1.5_f32, crate::ui::theme::colors::ACCENT_BLUE);

            egui::Frame::none()
                .fill(frame_bg.linear_multiply(0.92))
                .stroke(frame_stroke)
                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                .show(ui, |ui| {
                    ui.set_min_width(layer_w.min(400.0));
                    ui.set_max_width(400.0);

                    let edit_response = ui.add_sized(
                        [layer_w.min(400.0), 80.0],
                        egui::TextEdit::multiline(&mut buf).hint_text("Type source text…"),
                    );

                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

                    if enter {
                        should_commit = true;
                        should_close = true;
                    }
                    if escape {
                        should_close = true;
                    }

                    // Live-update text on the layer as the user types (zero-clone in-place mutation)
                    if edit_response.changed() {
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        if let Some(l) = comp_mut.layers.get_mut(idx) {
                            if let crate::core::timeline::LayerType::Text { text: t, .. } =
                                &mut l.layer_type
                            {
                                *t = buf.clone();
                            }
                        }
                        crate::core::frame_cache::bump_version();
                        ctx.request_repaint();
                    }

                    ui.small(
                        egui::RichText::new("⏎ commit · ⇧⏎ newline · Esc cancel")
                            .color(crate::ui::theme::colors::TEXT_MUTED),
                    );
                });
        });

    // Close on click outside the editor area
    if ctx.input(|i| i.pointer.any_click()) {
        let clicked_pos = ctx.input(|i| i.pointer.interact_pos());
        if let Some(cp) = clicked_pos {
            if !response.response.rect.contains(cp) {
                should_close = true;
            }
        }
    }

    if should_commit {
        let mut temp_proj = app.history.current().clone();
        let comp_mut = temp_proj.active_composition_mut();
        if let Some(l) = comp_mut.layers.get_mut(idx) {
            if let crate::core::timeline::LayerType::Text { text: t, .. } = &mut l.layer_type {
                *t = buf.clone();
            }
        }
        app.commit_project(temp_proj);
        app.toasts.info("Source text updated");
    }

    if should_close {
        ctx.data_mut(|d| d.remove::<String>(buf_id));
        app.inline_text_edit_layer = None;
    }
}
