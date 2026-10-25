use crate::core::timeline::{ColorConversionMode, EffectType};
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::super::common::draw_prop;

pub fn draw_basic_color(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) -> bool {
    match effect_type {
        EffectType::GaussianBlur { blur_radius } => {
            let val_before = blur_radius.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Blur Radius", blur_radius, |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if val_before != *blur_radius {
                *project_changed = true;
            }
            true
        }
        EffectType::ColorTint { color, intensity } => {
            let color_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Tint Color", color, |ui, val| {
                crate::ui::kui::color_rgba(ui, val);
            }) {
                *next_frame = Some(nf);
            }
            if color_before != *color {
                *project_changed = true;
            }

            let intensity_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if intensity_before != *intensity {
                *project_changed = true;
            }
            true
        }
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => {
            let ib_before = input_black.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Input Black", input_black, |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if ib_before != *input_black {
                *project_changed = true;
            }

            let iw_before = input_white.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Input White", input_white, |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if iw_before != *input_white {
                *project_changed = true;
            }

            let g_before = gamma.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Gamma", gamma, |ui, val| {
                crate::ui::kui::slider(ui, val, 0.1..=10.0, "");
            }) {
                *next_frame = Some(nf);
            }
            if g_before != *gamma {
                *project_changed = true;
            }

            let ob_before = output_black.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Output Black",
                output_black,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if ob_before != *output_black {
                *project_changed = true;
            }

            let ow_before = output_white.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Output White",
                output_white,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=1.0, "");
                },
            ) {
                *next_frame = Some(nf);
            }
            if ow_before != *output_white {
                *project_changed = true;
            }
            true
        }
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => {
            let h_before = hue_shift.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Hue Shift", hue_shift, |ui, val| {
                    crate::ui::kui::slider(ui, val, -180.0..=180.0, "°");
                })
            {
                *next_frame = Some(nf);
            }
            if h_before != *hue_shift {
                *project_changed = true;
            }

            let s_before = saturation.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Saturation", saturation, |ui, val| {
                    crate::ui::kui::slider(ui, val, -100.0..=100.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if s_before != *saturation {
                *project_changed = true;
            }

            let l_before = lightness.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Lightness", lightness, |ui, val| {
                    crate::ui::kui::slider(ui, val, -100.0..=100.0, "");
                })
            {
                *next_frame = Some(nf);
            }
            if l_before != *lightness {
                *project_changed = true;
            }
            true
        }
        EffectType::ColorGradeLUT {
            lut_path,
            intensity,
        } => {
            let mut lut_error = None;
            let mut browse_lut = false;
            crate::ui::kui::property(ui, "LUT File", |ui| {
                let path_before = lut_path.clone();
                let response = crate::ui::kui::file_field(
                    ui,
                    lut_path,
                    "No LUT selected",
                    "Choose and validate a .cube 3D LUT",
                );
                browse_lut = response.clicked();
                if path_before != *lut_path || response.changed() {
                    *project_changed = true;
                }
            });
            if browse_lut {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("3D LUT", &["cube"])
                    .pick_file()
                {
                    const MAX_LUT_BYTES: u64 = 64 * 1024 * 1024;
                    let size_ok = std::fs::metadata(&path)
                        .map(|metadata| metadata.len() <= MAX_LUT_BYTES)
                        .unwrap_or(false);
                    if !size_ok {
                        lut_error = Some("LUT file is missing or larger than 64 MiB".into());
                    } else {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => {
                                match crate::core::ocio_color::Lut3D::parse_cube(&contents) {
                                    Ok(_) => {
                                        *lut_path = path.to_string_lossy().into_owned();
                                        *project_changed = true;
                                    }
                                    Err(error) => lut_error = Some(format!("Invalid LUT: {error}")),
                                }
                            }
                            Err(error) => lut_error = Some(format!("Cannot read LUT: {error}")),
                        }
                    }
                }
            }
            if let Some(error) = lut_error {
                crate::ui::kui::status(ui, crate::ui::kui::StatusKind::Error, error);
            } else if lut_path.is_empty() {
                crate::ui::kui::hint(
                    ui,
                    "No LUT selected; choose a .cube file to enable this effect",
                );
            }

            let i_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "%");
                })
            {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }
            true
        }
        EffectType::ColorSpaceConvert { mode } => {
            let options = [
                (ColorConversionMode::LogCToLinear, "LogC → Linear"),
                (ColorConversionMode::LinearToLogC, "Linear → LogC"),
                (ColorConversionMode::SLog3ToLinear, "S-Log3 → Linear"),
                (ColorConversionMode::LinearToSLog3, "Linear → S-Log3"),
            ];
            let grid = crate::ui::kui::PropertyGrid::new(ui, &["Conversion"]);
            *project_changed |= grid.row(ui, "Conversion", |ui| {
                crate::ui::kui::combo(ui, "color_space_conversion", mode, &options)
            });
            true
        }
        EffectType::FilmGrain {
            intensity,
            grain_size,
            color_film,
        } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Grain Intensity",
                intensity,
                |ui, val| {
                    crate::ui::kui::slider(ui, val, 0.0..=100.0, "%");
                },
            ) {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }

            crate::ui::kui::property(ui, "Grain Size", |ui| {
                let before = *grain_size;
                crate::ui::kui::slider(ui, grain_size, 1.0..=5.0, "");
                if before != *grain_size {
                    *project_changed = true;
                }
            });

            if crate::ui::kui::toggle(ui, color_film, "Color Film Grain").changed() {
                *project_changed = true;
            }
            true
        }
        EffectType::Posterize { levels } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Posterize Levels",
                levels,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 2.0..=32.0, "");
                },
            );
            true
        }
        EffectType::Invert { invert_alpha } => {
            if crate::ui::kui::toggle(ui, invert_alpha, "Invert Alpha").changed() {
                *project_changed = true;
            }
            true
        }
        EffectType::Threshold { threshold } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=255.0, "");
                },
            );
            true
        }
        _ => false,
    }
}
