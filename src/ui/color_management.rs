use crate::ui::localization::{tr, tr_format};
use crate::KagariApp;
use eframe::egui;

pub fn draw_color_management(app: &mut KagariApp, ui: &mut egui::Ui) {
    app.ensure_panel_edit_snapshot("color_management", ui);
    let active_lut_path = app.color_lut_path.clone();
    let active_lut_mode = app.color_lut_mode;
    let (comp_bg, comp_shutter, comp_name) = {
        let comp = app.history.current().active_composition();
        (comp.background_color, comp.motion_blur_shutter_angle, comp.name.clone())
    };
    let mut pending_lut_path = None;
    let mut new_bg = None;
    let mut new_shutter = None;

    crate::ui::kui::section(ui, "Working Color Space");
    let grid = crate::ui::kui::PropertyGrid::new(ui, &["Working Color Space", "Bit Depth"]);
    grid.row(ui, "Working Color Space", |ui| {
        crate::ui::kui::secondary(ui, "Not applied (color transform not implemented)");
    });

    grid.row(ui, "Bit Depth", |ui| {
        crate::ui::kui::secondary(ui, "8-bit output (16/32-bit processing unsupported)");
    });

    crate::ui::kui::section(ui, "Display");
    let display_grid = crate::ui::kui::PropertyGrid::new(ui, &["Display Simulation"]);
    display_grid.row(ui, "Display Simulation", |ui| {
        crate::ui::kui::secondary(ui, "Unsupported");
    });
    crate::ui::kui::caption(ui, "Color space conversion and display simulation are not applied yet. Only the loaded LUT affects the preview.");

    crate::ui::kui::section(ui, "Background");
    let background_grid = crate::ui::kui::PropertyGrid::new(ui, &["Color"]);
    let mut col = egui::Color32::from_rgba_unmultiplied(
        (comp_bg[0].clamp(0.0, 1.0) * 255.0) as u8,
        (comp_bg[1].clamp(0.0, 1.0) * 255.0) as u8,
        (comp_bg[2].clamp(0.0, 1.0) * 255.0) as u8,
        (comp_bg[3].clamp(0.0, 1.0) * 255.0) as u8,
    );
    if background_grid
        .row(ui, "Color", |ui| crate::ui::kui::color(ui, &mut col))
        .changed()
    {
        let [r, g, b, a] = col.to_array();
        new_bg = Some([
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]);
    }

    crate::ui::kui::section(ui, "Motion Blur");
    let motion_grid = crate::ui::kui::PropertyGrid::new(ui, &["Shutter Angle"]);
    let mut shutter_angle = comp_shutter;
    if motion_grid
        .row(ui, "Shutter Angle", |ui| {
            crate::ui::kui::number(
                ui,
                &mut shutter_angle,
                0.0..=720.0,
                1.0,
                "°",
            )
        })
        .changed()
    {
        new_shutter = Some(shutter_angle);
    }

    ui.add_space(crate::ui::kui::space::XS);
    ui.add_space(crate::ui::kui::space::XS);
    crate::ui::kui::section(ui, "ACES / OCIO Pipeline");
    crate::ui::kui::panel(ui, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["Input Transform (IDT)"]);
        grid.row(ui, "Input Transform (IDT)", |ui| {
            crate::ui::kui::secondary(ui, "Unsupported");
        });
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["Output Transform (ODT)"]);
        grid.row(ui, "Output Transform (ODT)", |ui| {
            crate::ui::kui::secondary(ui, "Unsupported");
        });

        ui.horizontal(|ui| {
            crate::ui::kui::secondary(ui, "OCIO config:");
            if crate::ui::kui::button(
                ui,
                "Load config.ocio",
                crate::ui::kui::ButtonKind::Default,
            )
            .on_hover_text(tr("Load a custom OpenColorIO configuration"))
            .clicked()
            {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("OpenColorIO config", &["ocio"])
                    .pick_file()
                else {
                    return;
                };
                match crate::core::ocio_color::LoadedOcioConfig::load(&path) {
                    Ok(config) => {
                        let lut_size = config.lut.size;
                        let lut_path = config.lut_path.to_string_lossy().into_owned();
                        crate::core::ocio_color::set_active_lut(Some(std::sync::Arc::new(
                            config.lut,
                        )));
                        pending_lut_path = Some(lut_path.clone());
                        crate::core::frame_cache::bump_version();
                        let size = lut_size.to_string();
                        app.toasts.info(tr_format(
                            "Loaded OCIO LUT ({}³): {}",
                            &[&size, &lut_path],
                        ));
                    }
                    Err(error) => {
                        let error = error.to_string();
                        app.toasts
                            .error(tr_format("Could not load OCIO config: {}", &[&error]));
                    }
                }
            }
            if let Some(path) = active_lut_path.as_ref() {
                crate::ui::kui::caption(
                    ui,
                    format!("{}: {}", tr(if active_lut_mode == 3 { "Applied" } else { "Saved, not loaded" }), path),
                );
            }
            crate::ui::kui::caption(ui, "The LUT path and application state are saved in the project. The LUT file itself remains an external reference.");
        });
    });

    ui.add_space(crate::ui::kui::space::XS);
    crate::ui::kui::section(ui, "3D LUT (.cube)");
    crate::ui::kui::panel(ui, |ui| {
        ui.horizontal(|ui| {
            if crate::ui::kui::button(ui, "Load .cube LUT", crate::ui::kui::ButtonKind::Default)
                .on_hover_text(tr("Load a 3D LUT for film looks"))
                .clicked()
            {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("3D LUT", &["cube"])
                    .pick_file()
                else {
                    return;
                };

                match crate::core::ocio_color::Lut3D::load_cube_file(&path) {
                    Ok(lut) => {
                        let lut_size = lut.size;
                        crate::core::ocio_color::set_active_lut(Some(std::sync::Arc::new(lut)));
                        let lut_path = path.to_string_lossy().into_owned();
                        pending_lut_path = Some(lut_path.clone());
                        crate::core::frame_cache::bump_version();
                        let size = lut_size.to_string();
                        app.toasts.info(tr_format("Loaded {}³ LUT: {}", &[&size, &lut_path]));
                    }
                    Err(error) => {
                        let error = error.to_string();
                        app.toasts.error(tr_format("Could not load LUT: {}", &[&error]));
                    }
                }
            }
            if let Some(path) = active_lut_path.as_ref() {
                crate::ui::kui::caption(
                    ui,
                    format!(
                        "{}: {}",
                        tr(if active_lut_mode == 3 { "Applied" } else { "Saved, not loaded" }),
                        path
                    ),
                );
            }
            let active_lut = crate::core::ocio_color::active_lut();
            if crate::ui::kui::button(ui, "Export .cube LUT", crate::ui::kui::ButtonKind::Default)
                .on_hover_text(tr("Save the currently loaded LUT as a .cube file"))
                .clicked()
            {
                if let Some(lut) = active_lut {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("3D LUT", &["cube"])
                        .set_file_name(format!("{}.cube", comp_name.replace(['\0', '\n', '\r'], "_")))
                        .save_file()
                    {
                        let title = comp_name.replace(['\0', '\n', '\r', '"'], "_");
                        let mut content = format!(
                            "# Created by Kagari VFX\nTITLE \"{}\"\nLUT_3D_SIZE {}\nDOMAIN_MIN 0.0 0.0 0.0\nDOMAIN_MAX 1.0 1.0 1.0\n",
                            title, lut.size
                        );
                        for rgb in lut.data.as_chunks::<3>().0 {
                            use std::fmt::Write;
                            let _ = writeln!(content, "{:.8} {:.8} {:.8}", rgb[0], rgb[1], rgb[2]);
                        }
                        match std::fs::write(&path, content) {
                            Ok(()) => {
                                crate::ui::project_io::reveal_in_file_manager(&path);
                                let path = path.display().to_string();
                                app.toasts.info(tr_format("Exported LUT: {}", &[&path]));
                            }
                            Err(error) => {
                                let error = error.to_string();
                                app.toasts.error(tr_format("Could not export LUT: {}", &[&error]));
                            }
                        }
                    }
                } else {
                    app.toasts.error(tr("Load a 3D LUT first."));
                }
            }
        });
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["Interpolation"]);
        grid.row(ui, "Interpolation", |ui| {
            crate::ui::kui::secondary(ui, "Tetrahedral interpolation");
        });
    });

    if let Some(path) = pending_lut_path {
        app.persist_color_lut_path(path);
    }

    if let Some(bg) = new_bg {
        app.dispatch(crate::app_state::AppCommand::SetBackgroundColor(bg));
    }
    if let Some(shutter) = new_shutter {
        app.dispatch(crate::app_state::AppCommand::SetMotionBlurShutterAngle(shutter));
    }
}

pub fn draw_lut_controls(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::kui::section(ui, "3D LUT");
    crate::ui::kui::caption(ui, "Load a .cube LUT and apply it to the viewer.");

    let loaded_lut = crate::core::ocio_color::active_lut();
    let saved_path = app.color_lut_path.clone();
    let (kind, status) = match (&saved_path, loaded_lut.as_ref()) {
        (Some(_), Some(_)) => (crate::ui::kui::StatusKind::Success, "LUT loaded"),
        (Some(path), None) if std::path::Path::new(path).exists() => (
            crate::ui::kui::StatusKind::Warning,
            "Saved LUT is not loaded",
        ),
        (Some(_), None) => (crate::ui::kui::StatusKind::Error, "LUT file not found"),
        (None, _) => (crate::ui::kui::StatusKind::Info, "No LUT loaded"),
    };
    crate::ui::kui::panel(ui, |ui| {
        crate::ui::kui::status(ui, kind, status);
        if let (Some(path), Some(lut)) = (saved_path.as_deref(), loaded_lut.as_ref()) {
            crate::ui::kui::secondary(ui, format!("{}³", lut.size));
            crate::ui::kui::truncated_secondary(ui, path);
        } else if let Some(path) = saved_path.as_deref() {
            crate::ui::kui::truncated_secondary(ui, path);
        }
        ui.add_space(crate::ui::kui::space::XS);
        ui.horizontal_wrapped(|ui| {
            if crate::ui::kui::button(ui, "Load LUT", crate::ui::kui::ButtonKind::Accent).clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("3D LUT", &["cube"])
                    .pick_file()
                {
                    match crate::core::ocio_color::Lut3D::load_cube_file(&path) {
                        Ok(lut) => {
                            crate::core::ocio_color::set_active_lut(Some(std::sync::Arc::new(lut)));
                            app.persist_color_lut_path(path.to_string_lossy().into_owned());
                            crate::core::frame_cache::bump_version();
                            app.toasts
                                .info(crate::ui::localization::tr("Applied 3D LUT"));
                        }
                        Err(error) => app.toasts.error(format!(
                            "{}: {error}",
                            crate::ui::localization::tr("Could not load LUT")
                        )),
                    }
                }
            }
            if saved_path.is_some()
                && crate::ui::kui::button(ui, "Clear LUT", crate::ui::kui::ButtonKind::Quiet)
                    .clicked()
            {
                crate::core::ocio_color::set_active_lut(None);
                app.color_lut_path = None;
                app.color_lut_mode = 0;
                if let Some(document) = app.production_document.as_mut() {
                    document.color_lut_path = None;
                }
                app.autosave.mark_dirty();
                crate::core::frame_cache::bump_version();
            }
        });
    });
    crate::ui::kui::caption(
        ui,
        "The LUT selection is saved with the project; the .cube file remains external.",
    );
}
