use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

pub fn draw_comp_settings_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_comp_settings {
        return;
    }

    let mut open = app.show_comp_settings;
    let mut temp_proj = app
        .comp_settings_draft
        .take()
        .unwrap_or_else(|| app.history.current().clone());
    let mut committed = false;
    let mut cancelled = false;
    let mut close_requested = false;
    crate::ui::modal::dialog(" ")
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(900.0, 500.0))
        .min_size(egui::vec2(720.0, 400.0))
        .max_size(egui::vec2(1150.0, 520.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
            let comp = temp_proj.active_composition_mut();
            ui.horizontal(|ui| {
                let icon = egui::Image::from_bytes(
                    "bytes://composition-settings-header.svg",
                    crate::ui::icons::SVG_COMPOSITION.as_bytes(),
                )
                .fit_to_exact_size(egui::vec2(24.0, 24.0));
                ui.add(icon);
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr("Composition Settings"))
                        .size(20.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(crate::ui::shortcuts::format_shortcut("E", true, true, false))
                        .size(13.0)
                        .color(crate::ui::theme::colors::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("×").clicked() {
                        close_requested = true;
                    }
                });
            });
            kui::separator(ui);
            ui.add_space(kui::space::SM);
            egui::ScrollArea::vertical()
                .id_salt("composition_settings_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
            ui.columns(2, |columns| {
            let (left_column, right_column) = columns.split_at_mut(1);
            let ui = &mut left_column[0];
            let summary_ui = &mut right_column[0];

            let preview = egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DARKEST)
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(12.0));
            preview.show(summary_ui, |ui| {
                ui.horizontal(|ui| {
                    let aspect = comp.width.max(1) as f32 * comp.pixel_aspect.clamp(0.1, 10.0) / comp.height.max(1) as f32;
                    let (frame_w, frame_h) = if aspect >= 220.0 / 112.0 {
                        (220.0, (220.0 / aspect).max(48.0))
                    } else {
                        ((112.0 * aspect).max(48.0), 112.0)
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(frame_w, 136.0), egui::Sense::hover());
                    let image_rect = egui::Rect::from_center_size(
                        rect.center(),
                        egui::vec2(frame_w - 10.0, frame_h),
                    );
                    let background = comp.background_color;
                    let color = egui::Color32::from_rgba_unmultiplied(
                        (background[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (background[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (background[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (background[3].clamp(0.0, 1.0) * 255.0) as u8,
                    );
                    ui.painter().rect_filled(image_rect, 2.0, color);
                    ui.painter().rect_stroke(
                        image_rect,
                        2.0,
                        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_STRONG),
                    );
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&comp.name).strong().size(17.0));
                        ui.add_space(4.0);
                        ui.label(format!("{} × {} px", comp.width, comp.height));
                        ui.label(format!("{} fps · {} フレーム · ピクセル比 {:.3}:1", comp.fps, comp.duration_frames, comp.pixel_aspect));
                        ui.label(format!("{:.2} 秒", comp.duration_frames as f64 / comp.fps.max(1) as f64));
                        let fps = comp.fps.max(1);
                        let tc = comp.start_timecode_frames;
                        ui.label(format!("開始 {:02}:{:02}:{:02}:{:02}", tc / (fps * 3600), (tc / (fps * 60)) % 60, (tc / fps) % 60, tc % fps));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(format!("{:?} · {:?}", comp.working_color_space, comp.bit_depth))
                            .small().color(crate::ui::theme::colors::TEXT_SECONDARY));
                    });
                });
            });
            ui.add_space(kui::space::SM);

            kui::section(ui, "Basic Composition Settings");
            let grid = kui::PropertyGrid::new(ui, &["Composition Name", "Preset", "Width", "Height", "Pixel Aspect", "Frame Rate (FPS)", "Duration", "Start Timecode", "Background Color", "Depth", "Working Color Space"]);

            grid.row(ui, "Composition Name", |ui| { kui::text_field(ui, &mut comp.name); });

            ui.add_space(kui::space::SM);
            let dimensions = [(1920, 1080, 30), (3840, 2160, 60), (1280, 720, 24), (1080, 1920, 30), (1080, 1080, 30)];
            let mut preset_choice = dimensions.iter().position(|dims| *dims == (comp.width, comp.height, comp.fps)).unwrap_or(dimensions.len());
            if grid.row(ui, "Preset", |ui| kui::combo(ui, "comp_preset_combo", &mut preset_choice, &[
                (0, "HDTV 1080 30fps (1920 x 1080)"), (1, "4K UHD 60fps (3840 x 2160)"),
                (2, "720p HD 24fps (1280 x 720)"), (3, "Shorts / Reels (1080 x 1920)"),
                (4, "Instagram Feed (1080 x 1080)"), (5, "Custom"),
            ])) {
                if let Some(&(width, height, fps)) = dimensions.get(preset_choice) {
                    comp.width = width; comp.height = height; comp.fps = fps;
                }
            }

            grid.row(ui, "Width", |ui| { ui.add(kui::drag(&mut comp.width).speed(1.0).unit(" px").range(16..=16384)); });
            grid.row(ui, "Height", |ui| { ui.add(kui::drag(&mut comp.height).speed(1.0).unit(" px").range(16..=16384)); });
            grid.row(ui, "Pixel Aspect", |ui| {
                let mut preset = if (comp.pixel_aspect - 1.0).abs() < 0.001 { 0 } else if (comp.pixel_aspect - 1.2).abs() < 0.01 { 1 } else { 2 };
                if kui::combo(ui, "comp_pixel_aspect", &mut preset, &[(0, "Square (1.0)"), (1, "HD Anamorphic (1.2)"), (2, "Custom")]) && preset < 2 {
                    comp.pixel_aspect = if preset == 0 { 1.0 } else { 1.2 };
                }
                if preset == 2 { ui.add(egui::DragValue::new(&mut comp.pixel_aspect).speed(0.01).range(0.1..=10.0)); }
            });
            grid.row(ui, "Frame Rate (FPS)", |ui| {
                ui.add(kui::drag(&mut comp.fps).speed(1.0).unit(" fps").range(1..=120));
            });
            comp.fps = comp.fps.max(1);
            grid.row(ui, "Start Timecode", |ui| {
                let fps = comp.fps.max(1);
                timecode_input(ui, ctx, "comp_start_timecode", &mut comp.start_timecode_frames, fps);
            });
            grid.row(ui, "Duration", |ui| {
                ui.vertical(|ui| {
                    timecode_input(ui, ctx, "comp_duration_timecode", &mut comp.duration_frames, comp.fps.max(1));
                    kui::caption(ui, format!("{} フレーム · {:.2} 秒", comp.duration_frames, comp.duration_frames as f64 / comp.fps.max(1) as f64));
                });
            });

            grid.row(ui, "Background Color", |ui| {
                let c = &mut comp.background_color;
                let mut palette = if c[3] < 0.01 { 2 } else if c[0] < 0.01 && c[1] < 0.01 && c[2] < 0.01 { 0 } else if c[0] > 0.99 && c[1] > 0.99 && c[2] > 0.99 { 1 } else { 3 };
                let options = [(0, "Black"), (1, "White"), (2, "Transparent"), (3, "Custom")];
                if kui::combo(ui, "comp_background_palette", &mut palette, &options) {
                    *c = match palette {
                        0 => [0.0, 0.0, 0.0, 1.0],
                        1 => [1.0, 1.0, 1.0, 1.0],
                        2 => [0.0, 0.0, 0.0, 0.0],
                        _ => *c,
                    };
                    crate::core::frame_cache::bump_version();
                }
                let mut col = egui::Color32::from_rgba_unmultiplied(
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                );
                if kui::color(ui, &mut col).changed() {
                    let [r, g, b, a] = col.to_array();
                    *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                    crate::core::frame_cache::bump_version();
                }
            });

            let color_grid = kui::PropertyGrid::new(ui, &["Working Color Space", "Depth"]);
            color_grid.row(ui, "Depth", |ui| {
                let before_depth = comp.bit_depth;
                let options = [
                    (crate::core::color_science::BitDepth::EightBit, "8 bpc (Integer)"),
                    (crate::core::color_science::BitDepth::SixteenBit, "16 bpc (Half Float)"),
                    (crate::core::color_science::BitDepth::ThirtyTwoBitFloat, "32 bpc (Float / HDR)"),
                ];
                kui::combo(ui, "comp_bit_depth_select", &mut comp.bit_depth, &options);
                if comp.bit_depth != before_depth { crate::core::frame_cache::bump_version(); }
            });
            color_grid.row(ui, "Working Color Space", |ui| {
                let before_space = comp.working_color_space;
                let options = [
                    (crate::core::color_science::WorkingColorSpace::Rec709, "Rec.709 / sRGB"),
                    (crate::core::color_science::WorkingColorSpace::DisplayP3, "Display P3"),
                    (crate::core::color_science::WorkingColorSpace::Rec2020, "Rec.2020 (UHDTV)"),
                    (crate::core::color_science::WorkingColorSpace::LinearSRGB, "Linear sRGB"),
                    (crate::core::color_science::WorkingColorSpace::AcesCG, "ACEScg (AP1)"),
                ];
                kui::combo(ui, "comp_working_color_space_select", &mut comp.working_color_space, &options);
                if comp.working_color_space != before_space { crate::core::frame_cache::bump_version(); }
            });

            kui::collapsing_section(ui, "comp_settings_advanced", "Advanced", false, |ui| {
            kui::collapsing_section(ui, "comp_settings_blending", "Blending", false, |ui| {
                let before = comp.blend_linear;
                kui::toggle(ui, &mut comp.blend_linear,
                    "Blend colors in linear light (1.0 gamma)")
                    .on_hover_text("Add / Screen / Glow blends compute in linear space — brighter, physically-plausible results. Matches AE's 'Blend Colors Using 1.0 Gamma'.");
                if comp.blend_linear != before {
                    crate::core::frame_cache::bump_version();
                }
                kui::caption(ui, "Legacy projects default to OFF (gamma-space) for identical output.");
                ui.add_space(kui::space::XS);
                kui::toggle(ui, &mut comp.dither_output, "Output dithering (smooth gradients)")
                    .on_hover_text("Triangular-PDF noise ±1/255 breaks banding in dark gradients — imperceptible, deterministic");
            });

            grid.row(ui, "Shutter Angle", |ui| {
                kui::slider(ui, &mut comp.motion_blur_shutter_angle, 0.0..=720.0, "°");
            });
            grid.row(ui, "Shutter Phase", |ui| {
                kui::slider(ui, &mut comp.motion_blur_shutter_phase, -360.0..=360.0, "°")
                    .on_hover_text("Shifts the shutter window relative to the frame time");
            });
            });

            });
                });

            ui.add_space(kui::space::MD);
            kui::separator(ui);

            let mut should_commit = false;
            let mut should_close = false;

            ui.horizontal_wrapped(|ui| {
                if kui::button(ui, "OK", ButtonKind::Accent).clicked() {
                    should_commit = true;
                    should_close = true;
                }
                if kui::button(ui, "Cancel", ButtonKind::Default).clicked() {
                    should_close = true;
                }
            });

            if should_commit {
                let (old_w, old_h) = {
                    let orig = app.history.current().active_composition();
                    (orig.width as f32, orig.height as f32)
                };

                let comp = temp_proj.active_composition_mut();
                let (new_w, new_h) = (comp.width as f32, comp.height as f32);

                if (old_w - new_w).abs() > 0.1 || (old_h - new_h).abs() > 0.1 {
                    for layer in &mut comp.layers {
                        let pos_now = layer.transform.position.evaluate(0);
                        let remapped = layer.constraints.remap_position(pos_now, old_w, old_h, new_w, new_h);
                        layer.transform.position = crate::core::property::Animatable::new_constant(remapped);
                    }
                }

                app.commit_project(temp_proj.clone());
                committed = true;
            }

            if should_close {
                cancelled = !should_commit;
            }
        });

    if close_requested {
        open = false;
    }
    if !open || committed || cancelled {
        for salt in ["comp_start_timecode", "comp_duration_timecode"] {
            let id = egui::Id::new(("composition_timecode", salt));
            ctx.data_mut(|data| data.remove_temp::<String>(id));
        }
        app.comp_settings_draft = None;
        app.show_comp_settings = false;
    } else {
        app.comp_settings_draft = Some(temp_proj);
    }
}

fn timecode_input(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    salt: &'static str,
    frames: &mut u32,
    fps: u32,
) {
    let fps = fps.max(1);
    let id = egui::Id::new(("composition_timecode", salt));
    let mut draft = ctx
        .data(|data| data.get_temp::<String>(id))
        .unwrap_or_else(|| format_timecode(*frames, fps));
    let response = ui.add(egui::TextEdit::singleline(&mut draft).desired_width(140.0));
    if response.changed() {
        if let Some(parsed) = parse_timecode(&draft, fps) {
            *frames = parsed;
        }
        ctx.data_mut(|data| data.insert_temp(id, draft.clone()));
    }
    if response.lost_focus() {
        if let Some(parsed) = parse_timecode(&draft, fps) {
            *frames = parsed;
        }
        ctx.data_mut(|data| data.remove_temp::<String>(id));
    } else if !ctx.data(|data| data.get_temp::<String>(id).is_some()) {
        ctx.data_mut(|data| data.insert_temp(id, draft));
    }
}

fn format_timecode(frames: u32, fps: u32) -> String {
    let fps = fps.max(1);
    let total_seconds = frames / fps;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        total_seconds / 3600,
        (total_seconds / 60) % 60,
        total_seconds % 60,
        frames % fps
    )
}

fn parse_timecode(value: &str, fps: u32) -> Option<u32> {
    let parts: Vec<u64> = value
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.len() != 4 || parts[1] >= 60 || parts[2] >= 60 || parts[3] >= fps.max(1) as u64 {
        return None;
    }
    let total_seconds = parts[0]
        .checked_mul(3600)?
        .checked_add(parts[1] * 60 + parts[2])?;
    u32::try_from(
        total_seconds
            .checked_mul(fps.max(1) as u64)?
            .checked_add(parts[3])?,
    )
    .ok()
}
