use crate::core::timeline::{Layer, LayerType, ProjectItemType};
use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

/// Local asset library: quick-create solids/text from swatches & styles, and
/// browse the project's imported assets. (Replaces the fake "Creative Cloud"
/// panel — everything here works offline against the real project.)
pub fn draw_cc_libraries(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "Asset Library (Local)");

    kui::search_field(ui, &mut app.cc_libraries_search, "Search assets...");
    let query = app.cc_libraries_search.to_lowercase();
    ui.add_space(kui::space::SM);
    kui::separator(ui);

    // ── Color swatches: click adds a Solid layer ──
    kui::collapsing_section(ui, "cc_color_swatches", "Color Swatches", false, |ui| {
        if query.is_empty()
            || "color swatch solid".contains(&query)
            || crate::ui::localization::tr("Color Swatches")
                .to_lowercase()
                .contains(&query)
        {
            ui.horizontal_wrapped(|ui| {
                let swatches: [([f32; 4], &str); 6] = [
                    ([0.08, 0.45, 0.90, 1.0], "Studio Blue"),
                    ([0.90, 0.72, 0.00, 1.0], "Brand Gold"),
                    ([0.90, 0.20, 0.20, 1.0], "Alert Red"),
                    ([0.10, 0.80, 0.45, 1.0], "Mint Green"),
                    ([0.60, 0.30, 0.90, 1.0], "Violet"),
                    ([0.95, 0.95, 0.95, 1.0], "Off White"),
                ];
                for (color, name) in swatches {
                    let resp = kui::color_swatch(
                        ui,
                        egui::Color32::from_rgb(
                            (color[0] * 255.0) as u8,
                            (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8,
                        ),
                        28.0,
                    );
                    if resp.clicked() {
                        let lname = format!("Solid: {}", name);
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            let layer = Layer::new(
                                comp.next_layer_id("solid_lib"),
                                lname,
                                LayerType::Solid { color },
                                comp.duration_frames,
                            );
                            comp.layers.push(layer);
                        });
                    }
                    resp.on_hover_text(crate::ui::localization::tr_format(
                        "Add {} solid",
                        &[&crate::ui::localization::tr(name)],
                    ));
                }
            });
        }
    });

    // ── Character styles: click adds a styled Text layer ──
    kui::collapsing_section(ui, "cc_character_styles", "Character Styles", false, |ui| {
        let styles: [(&str, &str, u32, [f32; 4]); 3] = [
            ("Header Bold 48pt", "Header", 48, [1.0, 1.0, 1.0, 1.0]),
            (
                "Subhead Regular 24pt",
                "Subhead",
                24,
                [0.85, 0.85, 0.85, 1.0],
            ),
            ("Caption Small 16pt", "Caption", 16, [0.6, 0.6, 0.6, 1.0]),
        ];
        for (name, text, size, color) in styles {
            if (query.is_empty()
                || name.to_lowercase().contains(&query)
                || crate::ui::localization::tr(name)
                    .to_lowercase()
                    .contains(&query))
                && kui::button(
                    ui,
                    &format!("T  {}", crate::ui::localization::tr(name)),
                    kui::ButtonKind::Default,
                )
                .clicked()
            {
                let lname = format!("Text: {}", name);
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    let layer = Layer::new(
                        comp.next_layer_id("text_lib"),
                        lname,
                        LayerType::Text {
                            text: text.to_string(),
                            font_size: size,
                            color,
                            font_family: "Arial".into(),
                            tracking: 0.0,
                            leading: 1.2,
                            align: 1,
                            stroke_color: [0.0; 4],
                            stroke_width: 0.0,
                            text_on_path: false,
                        },
                        comp.duration_frames,
                    );
                    comp.layers.push(layer);
                });
            }
        }
    });

    // ── Project assets: browse + add to comp ──
    kui::collapsing_section(ui, "cc_project_assets", "Project Assets", false, |ui| {
        let project = app.history.current();
        let assets: Vec<String> = project
            .assets
            .iter()
            .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
            .map(|a| match &a.item_type {
                ProjectItemType::Image { path, .. } => format!(
                    "{} — {}: {}",
                    a.name,
                    crate::ui::localization::tr("Image"),
                    path
                ),
                ProjectItemType::Model3D { path } => format!(
                    "{} — {}: {} (OBJ)",
                    a.name,
                    crate::ui::localization::tr("Model 3D"),
                    path
                ),
                ProjectItemType::Video { path, duration_sec } => {
                    if duration_sec.is_finite() && *duration_sec > 0.0 {
                        format!(
                            "{} — {}: {} · {:.1}s",
                            a.name,
                            crate::ui::localization::tr("Video"),
                            path,
                            duration_sec
                        )
                    } else {
                        format!(
                            "{} — {}: {} ({})",
                            a.name,
                            crate::ui::localization::tr("Video"),
                            path,
                            crate::ui::localization::tr("Duration not measured")
                        )
                    }
                }
                ProjectItemType::Audio { path, duration_sec } => {
                    if duration_sec.is_finite() && *duration_sec > 0.0 {
                        format!(
                            "{} — {}: {} · {:.1}s",
                            a.name,
                            crate::ui::localization::tr("Audio"),
                            path,
                            duration_sec
                        )
                    } else {
                        format!(
                            "{} — {}: {} ({})",
                            a.name,
                            crate::ui::localization::tr("Audio"),
                            path,
                            crate::ui::localization::tr("Duration not measured")
                        )
                    }
                }
                ProjectItemType::Solid { .. } => {
                    format!("{} — {}", a.name, crate::ui::localization::tr("Solid"))
                }
                ProjectItemType::Folder { name } => {
                    format!("{} — {}", crate::ui::localization::tr("Folder"), name)
                }
                ProjectItemType::Composition { comp_idx } => {
                    format!(
                        "{} — {} #{}",
                        a.name,
                        crate::ui::localization::tr("Comp"),
                        comp_idx
                    )
                }
            })
            .collect();
        if assets.is_empty() {
            kui::empty_state(ui, "No assets — import media from the File menu", 48.0);
        }
        let row_height = ui.text_style_height(&egui::TextStyle::Small);
        egui::ScrollArea::vertical()
            .id_salt("local_library_assets")
            .show_rows(ui, row_height, assets.len(), |ui, rows| {
                for index in rows {
                    let asset = &assets[index];
                    kui::truncated_secondary(ui, asset);
                }
            });
    });
}
