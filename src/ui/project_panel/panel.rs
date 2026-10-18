use crate::core::timeline::{ProjectItem, ProjectItemType};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::actions::apply_project_mutations;
use super::asset_card::*;
use super::asset_row::draw_asset_row;
use super::media_replace::*;
use super::preview::draw_metadata_preview;

#[path = "panel_toolbar.rs"]
mod toolbar;

pub fn draw(app: &mut KagariApp, ui: &mut egui::Ui) {
    // ── Asset Search Filter & View Toggle Row (Matching Ideal Mockup) ──
    let grid_view_id = egui::Id::new("project_asset_grid_view");
    let grid_view = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(grid_view_id))
        .unwrap_or(true);

    let row_w = ui.available_width();
    let toggle_w = 28.0;
    let s_w = (row_w - toggle_w - 6.0).max(60.0);
    ui.horizontal(|ui| {
        let (s_rect, s_resp) = ui.allocate_exact_size(egui::vec2(s_w, 28.0), egui::Sense::click());
        let border_c = if s_resp.hovered() { colors::BORDER_MEDIUM } else { colors::BORDER_SUBTLE };
        ui.painter().rect_filled(s_rect, 6.0, colors::BG_SURFACE);
        ui.painter().rect_stroke(s_rect, 6.0, egui::Stroke::new(1.0_f32, border_c));
        crate::ui::icons::render_svg_at(
            ui,
            "asset-search-icon".into(),
            crate::ui::icons::SVG_SEARCH,
            egui::vec2(13.0, 13.0),
            colors::TEXT_MUTED,
            egui::pos2(s_rect.left() + 8.0, s_rect.center().y - 6.5),
        );
        let mut search_ui = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
            egui::pos2(s_rect.left() + 26.0, s_rect.top() + 2.0),
            egui::pos2(s_rect.right() - 6.0, s_rect.bottom() - 2.0),
        )));
        search_ui.add(
            egui::TextEdit::singleline(&mut app.project_search_query)
                .hint_text(egui::RichText::new("素材を検索...").color(colors::TEXT_MUTED))
                .frame(false),
        );

        let toggle_resp = super::asset_card::project_view_toggle(
            ui,
            "asset-grid-list-toggle",
            if grid_view { crate::ui::icons::SVG_GRID } else { super::asset_card::SVG_LIST_VIEW },
            grid_view,
            "グリッド / リスト表示切り替え",
        );
        if toggle_resp.clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(grid_view_id, !grid_view));
        }
    });
    ui.add_space(8.0);

    let query = app.project_search_query.to_lowercase();
    let grid_view_id = egui::Id::new("project_asset_grid_view");
    let grid_view = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(grid_view_id))
        .unwrap_or(true);

    // Read current state without cloning upfront
    let current_project = app.history.current();

    // Toolbar actions (New comp, import file, folder)
    let toolbar_actions = toolbar::draw(ui);

    // ── Project Items List (Assets & Compositions Bin Tree) ──
    let selected_asset_idx = ui
        .ctx()
        .data(|d| d.get_temp::<Option<usize>>(egui::Id::new("selected_project_asset")))
        .flatten();
    let mut selected_idx_update: Option<Option<usize>> = None;
    let mut add_to_timeline_item: Option<ProjectItem> = None;

    // Asset mutation requests collected during the render pass
    let mut move_to_folder: Option<(usize, Option<String>)> = None;

    let asset_list_height = if grid_view {
        ui.available_height().max(120.0)
    } else {
        280.0
    };
    let mut draw_asset_items = |ui: &mut egui::Ui| {
        egui::ScrollArea::vertical()
            .max_height(asset_list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Pre-compute folder ids for nesting
                let folders: Vec<(usize, String, String)> = current_project
                    .assets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, it)| match &it.item_type {
                        ProjectItemType::Folder { .. } => Some((i, it.id.clone(), it.name.clone())),
                        _ => None,
                    })
                    .collect();
                let in_folder = |i: usize| -> Option<String> {
                    current_project
                        .assets
                        .get(i)
                        .and_then(|a| a.parent_folder.clone())
                };

                if grid_view {
                    let kind_id = egui::Id::new("project_asset_kind_filter");
                    let selected_kind = ui
                        .ctx()
                        .data(|data| data.get_temp::<String>(kind_id))
                        .unwrap_or_else(|| "all".to_owned());
                    let card_items = current_project
                        .assets
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| {
                            !matches!(item.item_type, ProjectItemType::Folder { .. })
                                && (query.is_empty() || item.name.to_lowercase().contains(&query))
                                && (selected_kind == "all" || asset_kind(item) == selected_kind)
                        })
                        .collect::<Vec<_>>();
                    let columns = if ui.available_width() >= 240.0 { 2 } else { 1 };
                    let gap = 6.0;
                    let card_width = ((ui.available_width() - gap * (columns - 1) as f32)
                        / columns as f32)
                        .max(110.0);
                    let row_height = 72.0;
                    let mut replace_footage_req: Option<(usize, std::path::PathBuf)> = None;
                    let thumbs = &mut app.home_thumbs;
                    let failed_preview_id = egui::Id::new("project_asset_failed_previews");
                    let mut failed_previews = ui
                        .ctx()
                        .data(|data| {
                            data.get_temp::<std::collections::HashSet<std::path::PathBuf>>(
                                failed_preview_id,
                            )
                        })
                        .unwrap_or_default();
                    for row_start in (0..card_items.len()).step_by(columns) {
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), row_height),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            ui.add_space(row_height);
                            continue;
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for offset in 0..columns {
                                let Some((index, item)) =
                                    card_items.get(row_start + offset).copied()
                                else {
                                    break;
                                };
                                let preview =
                                    asset_preview_path(current_project, item).and_then(|path| {
                                        asset_preview_texture(
                                            thumbs,
                                            &mut failed_previews,
                                            ui.ctx(),
                                            &path,
                                        )
                                    });
                                let selected = selected_asset_idx == Some(index);
                                let response =
                                    draw_asset_card(ui, item, index, card_width, selected, preview);
                                if response.clicked() {
                                    selected_idx_update = Some(Some(index));
                                }
                                if response.double_clicked() {
                                    add_to_timeline_item = Some(item.clone());
                                }
                                response.context_menu(|menu| {
                                    if menu.button("コンポジションに追加").clicked() {
                                        add_to_timeline_item = Some(item.clone());
                                        menu.close_menu();
                                    }
                                    if let Some((filter_name, extensions)) =
                                        replacement_filter(&item.item_type)
                                    {
                                        if menu.button("素材を置き換え…").clicked() {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter(filter_name, extensions)
                                                .pick_file()
                                            {
                                                replace_footage_req = Some((index, path));
                                            }
                                            menu.close_menu();
                                        }
                                    }
                                    if !folders.is_empty() {
                                        menu.separator();
                                        menu.menu_button("フォルダーへ移動", |menu| {
                                            if menu
                                                .selectable_label(
                                                    item.parent_folder.is_none(),
                                                    "ルート",
                                                )
                                                .clicked()
                                            {
                                                move_to_folder = Some((index, None));
                                                menu.close_menu();
                                            }
                                            for (_, folder_id, folder_name) in &folders {
                                                let selected = item.parent_folder.as_deref()
                                                    == Some(folder_id.as_str());
                                                if menu
                                                    .selectable_label(selected, folder_name)
                                                    .clicked()
                                                {
                                                    move_to_folder =
                                                        Some((index, Some(folder_id.clone())));
                                                    menu.close_menu();
                                                }
                                            }
                                        });
                                    }
                                });
                            }
                        });
                        if let Some((asset_idx, new_path)) = replace_footage_req.take() {
                            let file_name = new_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            let path = new_path.to_string_lossy().into_owned();
                            selected_idx_update = Some(Some(asset_idx));
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(
                                    egui::Id::new("ae_replace_footage_asset_idx"),
                                    asset_idx,
                                );
                                data.insert_temp(egui::Id::new("ae_replace_footage_path"), path);
                                data.insert_temp(
                                    egui::Id::new("ae_replace_footage_name"),
                                    file_name,
                                );
                            });
                        }
                    }
                    if card_items.is_empty() {
                        crate::ui::kui::empty_state(ui, "条件に一致する素材はありません", 96.0);
                    }
                    if failed_previews.len() > 256 {
                        failed_previews.clear();
                    }
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(failed_preview_id, failed_previews));
                } else {
                    // Root-level items first (folders rendered as headers below their root slot)
                    for (i, item) in current_project.assets.iter().enumerate() {
                        if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                            continue;
                        }
                        if matches!(item.item_type, ProjectItemType::Folder { .. }) {
                            continue;
                        }
                        if in_folder(i).is_some() && query.is_empty() {
                            continue; // nested under its folder header below
                        }

                        let is_selected = selected_asset_idx == Some(i);
                        draw_asset_row(
                            ui,
                            i,
                            item,
                            is_selected,
                            &mut selected_idx_update,
                            &mut add_to_timeline_item,
                            &mut move_to_folder,
                            &folders,
                        );
                    }

                    // Folder bins with nested children
                    for (_fi, folder_id, folder_name) in &folders {
                        if !query.is_empty() && !folder_name.to_lowercase().contains(&query) {
                            continue;
                        }
                        let children: Vec<(usize, &ProjectItem)> = current_project
                            .assets
                            .iter()
                            .enumerate()
                            .filter(|(_i, it)| {
                                it.parent_folder.as_deref() == Some(folder_id.as_str())
                            })
                            .collect();

                        egui::CollapsingHeader::new(format!(
                            "{} ({})",
                            folder_name,
                            children.len()
                        ))
                        .default_open(!query.is_empty())
                        .show(ui, |ui| {
                            if children.is_empty() {
                                ui.small("empty bin");
                            }
                            for (ci, child) in children {
                                let is_sel = selected_asset_idx == Some(ci);
                                draw_asset_row(
                                    ui,
                                    ci,
                                    child,
                                    is_sel,
                                    &mut selected_idx_update,
                                    &mut add_to_timeline_item,
                                    &mut move_to_folder,
                                    &folders,
                                );
                            }
                            // Un-bin shortcut on the folder itself
                            if ui.small_button("⤴ Move selection out").clicked() {
                                if let Some(sel) = selected_asset_idx {
                                    if let Some(it) = current_project.assets.get(sel) {
                                        if it.parent_folder.as_deref() == Some(folder_id.as_str()) {
                                            move_to_folder = Some((sel, None));
                                        }
                                    }
                                }
                            }
                        });
                    }

                    if current_project.assets.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("素材はまだありません")
                                    .color(colors::TEXT_SECONDARY),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("映像を読み込むか、コンポジションを作成")
                                    .small()
                                    .color(colors::TEXT_MUTED),
                            );
                        });
                        ui.add_space(20.0);
                    }
                }
            });
    };

    if grid_view {
        let kind_id = egui::Id::new("project_asset_kind_filter");
        let category_width = (ui.available_width() * 0.34).clamp(96.0, 108.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.allocate_ui_with_layout(
                egui::vec2(category_width, asset_list_height),
                egui::Layout::top_down(egui::Align::Min),
                |category_ui| {
                    let selected_kind = category_ui
                        .ctx()
                        .data(|data| data.get_temp::<String>(kind_id))
                        .unwrap_or_else(|| "all".to_owned());
                    for (kind, label, icon) in [
                        ("all", "すべて", crate::ui::icons::SVG_FOLDER),
                        ("video", "映像", crate::ui::icons::SVG_FILE),
                        ("image", "画像", crate::ui::icons::SVG_SCREEN),
                        ("audio", "オーディオ", crate::ui::icons::SVG_AUDIO),
                        ("text", "テキスト", crate::ui::icons::SVG_TOOL_TEXT),
                        ("preset", "プリセット", crate::ui::icons::SVG_EFFECTS),
                    ] {
                        let (row, response) = category_ui.allocate_exact_size(
                            egui::vec2(category_ui.available_width(), 23.0),
                            egui::Sense::click(),
                        );
                        let active = selected_kind == kind;
                        if active || response.hovered() {
                            category_ui.painter().rect_filled(
                                row,
                                4.0,
                                if active {
                                    colors::BG_ACTIVE
                                } else {
                                    colors::BG_HOVER
                                },
                            );
                        }
                        if active {
                            category_ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    row.left_top(),
                                    egui::pos2(row.left() + 2.0, row.bottom()),
                                ),
                                1.0,
                                colors::ACCENT_SELECTION,
                            );
                        }
                        crate::ui::icons::render_svg_at(
                            category_ui,
                            format!("project-asset-category-{kind}"),
                            icon,
                            egui::vec2(14.0, 14.0),
                            if active {
                                colors::TEXT_PRIMARY
                            } else {
                                colors::TEXT_SECONDARY
                            },
                            row.left_center() + egui::vec2(9.0, -7.0),
                        );
                        category_ui.painter().text(
                            egui::pos2(row.left() + 29.0, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(12.0),
                            if active {
                                colors::TEXT_PRIMARY
                            } else {
                                colors::TEXT_SECONDARY
                            },
                        );
                        if response.clicked() {
                            category_ui
                                .ctx()
                                .data_mut(|data| data.insert_temp(kind_id, kind.to_owned()));
                        }
                    }
                },
            );
            ui.separator();
            let content_width = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(content_width, asset_list_height),
                egui::Layout::top_down(egui::Align::Min),
                |asset_ui| draw_asset_items(asset_ui),
            );
        });
    } else {
        draw_asset_items(ui);
    }

    if let Some(update) = selected_idx_update {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("selected_project_asset"), update));
    }

    // Replace requests are posted by the asset row menu during the render pass
    // and consumed by the project mutation block below on the same frame.
    let replace_info: Option<(usize, String, String)> = ui.ctx().data_mut(|d| {
        let idx = d.remove_temp::<usize>(egui::Id::new("ae_replace_footage_asset_idx"));
        let path = d.remove_temp::<String>(egui::Id::new("ae_replace_footage_path"));
        let name = d.remove_temp::<String>(egui::Id::new("ae_replace_footage_name"));
        match (idx, path, name) {
            (Some(i), Some(p), Some(n)) => Some((i, p, n)),
            _ => None,
        }
    });

    ui.add_space(8.0);
    ui.separator();

    // ── Selected Asset Metadata Preview Header ──
    draw_metadata_preview(
        ui,
        current_project,
        selected_asset_idx,
        &mut add_to_timeline_item,
    );

    let pending_video_import = add_to_timeline_item.as_ref().and_then(|item| {
        if let ProjectItemType::Video { path, .. } = &item.item_type {
            Some((std::path::PathBuf::from(path), item.name.clone()))
        } else {
            None
        }
    });
    if let Some((path, name)) = pending_video_import {
        add_to_timeline_item = None;
        crate::ui::drop_import::start_video_import(app, ui.ctx(), path, name);
    }

    apply_project_mutations(
        app,
        ui.ctx(),
        toolbar_actions.add_comp_requested,
        toolbar_actions.import_file_requested,
        toolbar_actions.add_folder_requested,
        add_to_timeline_item,
        move_to_folder,
        toolbar_actions.remove_unused_requested,
        toolbar_actions.reduce_project_requested,
        replace_info,
    );
}
