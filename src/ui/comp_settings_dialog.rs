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
    crate::ui::modal::dialog(format!(
        "⚙ Composition Settings ({})",
        crate::ui::shortcuts::format_shortcut("E", true, true, false)
    ))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            let comp = temp_proj.active_composition_mut();

            kui::section(ui, "Basic Composition Settings");
            let grid = kui::PropertyGrid::new(ui, &["Composition Name", "Preset", "Width", "Height", "Frame Rate (FPS)", "Duration (Frames)", "Working Color Space", "Depth", "Shutter Angle", "Shutter Phase", "Background Color"]);

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

            kui::group(ui, "One-Tap Smart SNS Reframer", |ui| {
                kui::caption(ui, "Auto-remap layer positions to new Aspect Ratio:");
                ui.horizontal_wrapped(|ui| {
                    if kui::button(ui, "Shorts 9:16", ButtonKind::Default).on_hover_text("Vertical (1080 x 1920) for TikTok/Reels/Shorts").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(1080, 1920, old_w, old_h);
                    }
                    if kui::button(ui, "Square 1:1", ButtonKind::Default).on_hover_text("Square (1080 x 1080) for Instagram Feed").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(1080, 1080, old_w, old_h);
                    }
                    if kui::button(ui, "Cinema 21:9", ButtonKind::Default).on_hover_text("Ultrawide Cinematic (2560 x 1080)").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(2560, 1080, old_w, old_h);
                    }
                });
            });
            ui.add_space(kui::space::SM);
            grid.row(ui, "Width", |ui| { ui.add(kui::drag(&mut comp.width).speed(1.0).unit(" px").range(16..=16384)); });
            grid.row(ui, "Height", |ui| { ui.add(kui::drag(&mut comp.height).speed(1.0).unit(" px").range(16..=16384)); });
            grid.row(ui, "Frame Rate (FPS)", |ui| {
                ui.vertical(|ui| {
                    ui.add(kui::drag(&mut comp.fps).speed(1.0).range(1..=120));
                    let mut options = vec![(24, "Film (24 fps)"), (25, "PAL (25 fps)"), (30, "NTSC Video (30 fps)"), (60, "Smooth (60 fps)"), (120, "High-Speed (120 fps)")];
                    if !options.iter().any(|(fps, _)| *fps == comp.fps) { options.push((comp.fps, "Custom")); }
                    kui::combo(ui, "ae_fps_preset_choice", &mut comp.fps, &options);
                });
            });
            comp.fps = comp.fps.max(1);
            grid.row(ui, "Duration (Frames)", |ui| {
                ui.vertical(|ui| {
                    ui.add(kui::drag(&mut comp.duration_frames).speed(1.0).range(1..=100000));
                    kui::caption(ui, format!("({:.2} seconds)", comp.duration_frames as f64 / comp.fps as f64));
                });
            });

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

            kui::collapsing_section(ui, "comp_settings_color_management", "Color Management & Working Space", false, |ui| {
                let color_grid = kui::PropertyGrid::new(ui, &["Working Color Space", "Depth"]);
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
                    if comp.working_color_space != before_space {
                        crate::core::frame_cache::bump_version();
                    }
                });
                kui::caption(ui, "Working color space converts scene pixels into the target color gamut and monitor display profile.");

                color_grid.row(ui, "Depth", |ui| {
                    let before_depth = comp.bit_depth;
                    let options = [
                        (crate::core::color_science::BitDepth::EightBit, "8 bpc (Integer)"),
                        (crate::core::color_science::BitDepth::SixteenBit, "16 bpc (Half Float)"),
                        (crate::core::color_science::BitDepth::ThirtyTwoBitFloat, "32 bpc (Float / HDR)"),
                    ];
                    kui::combo(ui, "comp_bit_depth_select", &mut comp.bit_depth, &options);
                    if comp.bit_depth != before_depth {
                        crate::core::frame_cache::bump_version();
                    }
                });
            });
            grid.row(ui, "Shutter Angle", |ui| {
                kui::slider(ui, &mut comp.motion_blur_shutter_angle, 0.0..=720.0, "°");
            });
            grid.row(ui, "Shutter Phase", |ui| {
                kui::slider(ui, &mut comp.motion_blur_shutter_phase, -360.0..=360.0, "°")
                    .on_hover_text("Shifts the shutter window relative to the frame time");
            });
            grid.row(ui, "Background Color", |ui| {
                let c = &mut comp.background_color;
                let mut col = egui::Color32::from_rgba_premultiplied(
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                );
                if kui::color(ui, &mut col).changed() {
                    let [r, g, b, a] = col.to_array();
                    *c = [
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    ];
                    crate::core::frame_cache::bump_version();
                }
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

    if !open || committed || cancelled {
        app.comp_settings_draft = None;
        app.show_comp_settings = false;
    } else {
        app.comp_settings_draft = Some(temp_proj);
    }
}
