use crate::ui::localization::tr;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_metadata_panel(app: &KagariApp, ui: &mut egui::Ui) {
    let comp = app.history.current().active_composition();

    crate::ui::custom_widgets::ae_section_header(ui, &tr("Composition Info"), "📋");

    egui::Grid::new("metadata_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr("Property"))
                    .small()
                    .strong()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(tr("Value"))
                    .small()
                    .strong()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Name"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(&comp.name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Resolution"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{} × {} px", comp.width, comp.height))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Frame Rate"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{} fps", comp.fps))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            let duration_sec = comp.duration_frames as f64 / comp.fps as f64;
            let dur_min = (duration_sec / 60.0) as u32;
            let dur_sec = duration_sec % 60.0;
            ui.label(
                egui::RichText::new(tr("Duration"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} frames ({:02}:{:05.2})",
                    comp.duration_frames, dur_min, dur_sec
                ))
                .small()
                .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Pixel Aspect"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(tr("1.0 (Square)"))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, &tr("Layer Stats"), "📊");

    let total_layers = comp.layers.len();
    let visible_layers = comp.layers.iter().filter(|l| l.visible).count();
    let locked_layers = comp.layers.iter().filter(|l| l.locked).count();
    let total_effects: usize = comp.layers.iter().map(|l| l.effects.len()).sum();

    egui::Grid::new("layer_stats_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr("Total Layers"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", total_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Visible"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", visible_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Locked"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", locked_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Effects"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", total_effects))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Lights"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", comp.lights.len()))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Sub-Comps"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", comp.sub_compositions.len()))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, &tr("Render Settings"), "⚙");

    egui::Grid::new("render_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr("Color Space"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(tr("Not applied"))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Bit Depth"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let depth_name = app.history.current().active_composition().bit_depth.label();
            ui.label(
                egui::RichText::new(depth_name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new(tr("Shutter Angle"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}°", comp.motion_blur_shutter_angle))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, &tr("Export Info"), "📤");

    let format_name = match app.export_format_preset {
        0 => "H.264 MP4",
        1 => "ProRes 422",
        2 => "ProRes 4444",
        3 => "GIF",
        _ => "PNG Sequence",
    };
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("Format"))
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.label(
            egui::RichText::new(tr(format_name))
                .small()
                .color(colors::TEXT_PRIMARY),
        );
    });
}
