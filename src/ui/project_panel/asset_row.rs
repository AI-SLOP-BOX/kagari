use crate::core::timeline::{ProjectItem, ProjectItemType};
use crate::ui::theme::colors;
use eframe::egui;
use super::media_replace::replacement_filter;

/// One row of the asset list; shared by root listing and folder bins.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_asset_row(
    ui: &mut egui::Ui,
    i: usize,
    item: &ProjectItem,
    is_selected: bool,
    selected_idx_update: &mut Option<Option<usize>>,
    add_to_timeline_item: &mut Option<ProjectItem>,
    move_to_folder: &mut Option<(usize, Option<String>)>,
    folders: &[(usize, String, String)],
) {
    let row_probe =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 20.0));
    if !ui.is_rect_visible(row_probe) {
        ui.add_space(20.0);
        return;
    }
    let show_row_actions = is_selected || ui.rect_contains_pointer(row_probe);

    use ProjectItemType as T;
    let (icon_svg, item_tag) = match &item.item_type {
        T::Composition { .. } => (crate::ui::icons::SVG_COMPOSITION, "Composition"),
        T::Image { .. } => (crate::ui::icons::SVG_FILE, "Footage Image"),
        T::Model3D { .. } => (crate::ui::icons::SVG_SWITCH_3D, "3D Model"),
        T::Video { .. } => (crate::ui::icons::SVG_FILE, "Footage Video"),
        T::Audio { .. } => (crate::ui::icons::SVG_AUDIO, "Audio File"),
        T::Solid { .. } => (crate::ui::icons::SVG_WINDOW_MAXIMIZE, "Solid Color"),
        T::Folder { .. } => (crate::ui::icons::SVG_FOLDER, "Folder Bin"),
    };

    let mut replace_footage_req: Option<(usize, std::path::PathBuf)> = None;

    ui.horizontal(|ui| {
        let row_width = ui.available_width();
        let compact_row = row_width < 260.0;
        let tag_width = if compact_row { 72.0 } else { 92.0 };
        let action_width = if !folders.is_empty() && show_row_actions {
            34.0
        } else {
            0.0
        };
        let name_width = (row_width - 16.0 - 4.0 - tag_width - 4.0 - action_width).max(48.0);
        crate::ui::icons::render_svg_bytes(
            ui,
            &format!("project-asset-icon-{i}"),
            icon_svg,
            egui::vec2(16.0, 16.0),
            if is_selected {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
        let (name_rect, response) =
            ui.allocate_exact_size(egui::vec2(name_width, 20.0), egui::Sense::click());
        if is_selected || response.hovered() {
            ui.painter().rect_filled(
                name_rect,
                3.0,
                if is_selected {
                    colors::BG_HOVER.linear_multiply(0.72)
                } else {
                    colors::BG_HOVER.linear_multiply(0.45)
                },
            );
        }
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(name_rect), |name_ui| {
            name_ui.add(
                egui::Label::new(egui::RichText::new(&item.name).size(12.0).color(
                    if is_selected {
                        colors::TEXT_PRIMARY
                    } else {
                        colors::TEXT_SECONDARY
                    },
                ))
                .truncate(),
            );
        });
        if response.clicked() {
            *selected_idx_update = Some(Some(i));
        }
        if response.double_clicked() {
            *add_to_timeline_item = Some(item.clone());
        }
        response.context_menu(|ui| {
            if ui.button("➕ Add to Composition").clicked() {
                *add_to_timeline_item = Some(item.clone());
                ui.close_menu();
            }
            if let Some((filter_name, extensions)) = replacement_filter(&item.item_type) {
                if ui.button("Replace Footage...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(filter_name, extensions)
                        .pick_file()
                    {
                        replace_footage_req = Some((i, path));
                    }
                    ui.close_menu();
                }
            }
        });
        ui.add_sized(
            [tag_width, 20.0],
            egui::Label::new(
                egui::RichText::new(format!("({})", item_tag))
                    .size(11.0)
                    .color(colors::TEXT_MUTED),
            )
            .truncate(),
        );
        if item.is_media_missing() {
            ui.label(
                egui::RichText::new(if compact_row { "⚠" } else { "⚠ Missing" })
                    .small()
                    .strong()
                    .color(colors::ACCENT_RED),
            )
            .on_hover_text(format!(
                "File not found:\n{}\nRight-click → Replace Footage to relink.",
                item.media_path().unwrap_or_default()
            ));
        }

        // Move-to-bin dropdown
        if !folders.is_empty() && show_row_actions {
            let mb = ui.menu_button("Move ▾", |ui| {
                if ui
                    .selectable_label(item.parent_folder.is_none(), "(project root)")
                    .clicked()
                {
                    *move_to_folder = Some((i, None));
                    ui.close_menu();
                }
                for (_, fid, fname) in folders {
                    let inside = item.parent_folder.as_deref() == Some(fname.as_str());
                    if ui.selectable_label(inside, fname).clicked() {
                        *move_to_folder = Some((i, Some(fid.clone())));
                        ui.close_menu();
                    }
                }
            });
            mb.response
                .on_hover_text("Move this asset into/out of a bin");
        }
    });

    if let Some((asset_idx, new_path)) = replace_footage_req {
        let new_file_name = new_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let new_path_str = new_path.to_string_lossy().to_string();
        // Request replace footage handling
        *selected_idx_update = Some(Some(asset_idx));
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new("ae_replace_footage_asset_idx"), asset_idx);
            d.insert_temp(egui::Id::new("ae_replace_footage_path"), new_path_str);
            d.insert_temp(egui::Id::new("ae_replace_footage_name"), new_file_name);
        });
    }
}
