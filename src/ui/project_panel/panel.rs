use crate::core::timeline::{Layer, LayerType, ProjectItem, ProjectItemType};
use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::asset_card::*;
use super::asset_row::draw_asset_row;
use super::media_replace::*;

pub fn draw(app: &mut KagariApp, ui: &mut egui::Ui) {
    // ── Asset Search Filter ──
    ui.add_sized(
        [ui.available_width(), 26.0],
        egui::TextEdit::singleline(&mut app.project_search_query)
            .hint_text("プロジェクト素材を検索"),
    );

    let query = app.project_search_query.to_lowercase();
    let grid_view_id = egui::Id::new("project_asset_grid_view");
    let grid_view = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(grid_view_id))
        .unwrap_or(false);

    // Read current state without cloning upfront
    let current_project = app.history.current();

    // ── AE Top Asset Thumbnail & Meta Box ──
    let selected_asset_idx: Option<usize> = ui
        .ctx()
        .data_mut(|d| d.get_temp(egui::Id::new("selected_project_asset")));
    if !grid_view {
        if let Some(idx) = selected_asset_idx {
            if let Some(item) = current_project.assets.get(idx) {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Render miniature preview square box
                        let (thumb_rect, _) =
                            ui.allocate_exact_size(egui::vec2(44.0, 32.0), egui::Sense::hover());
                        ui.painter().rect_filled(thumb_rect, 2.0, colors::BG_DARK);
                        ui.painter().rect_stroke(
                            thumb_rect,
                            2.0,
                            egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
                        );
                        let center = thumb_rect.center();
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::FILM_STRIP,
                            egui::FontId::monospace(14.0),
                            colors::TEXT_ACCENT,
                        );

                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&item.name).strong().size(13.0));
                            match &item.item_type {
                                ProjectItemType::Composition { comp_idx } => {
                                    if let Some(c) = current_project.compositions.get(*comp_idx) {
                                        ui.small(format!(
                                            "{} x {} (1.00) | {:.2} fps | {}",
                                            c.width, c.height, c.fps, c.name
                                        ));
                                    }
                                }
                                ProjectItemType::Image { width, height, .. } => {
                                    ui.small(format!("{} x {} px | RGB 8-bpc", width, height));
                                }
                                ProjectItemType::Model3D { .. } => {
                                    ui.small("Wavefront OBJ | 3D Model");
                                }
                                ProjectItemType::Video { duration_sec, .. } => {
                                    if duration_sec.is_finite() && *duration_sec > 0.0 {
                                        ui.small(format!("Video | {:.1}s", duration_sec));
                                    } else {
                                        ui.small("Video | 長さ未取得");
                                    }
                                }
                                ProjectItemType::Audio { duration_sec, .. } => {
                                    if duration_sec.is_finite() && *duration_sec > 0.0 {
                                        ui.small(format!("Audio | {:.1}s", duration_sec));
                                    } else {
                                        ui.small("Audio | 長さ未取得");
                                    }
                                }
                                ProjectItemType::Solid { .. } => {
                                    ui.small("Solid Color Layer Footage");
                                }
                                ProjectItemType::Folder { .. } => {
                                    ui.small("Folder Directory Bin");
                                }
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        }
    }

    let mut add_comp_requested = false;
    let mut import_file_requested: Option<std::path::PathBuf> = None;
    let mut add_folder_requested = false;
    let mut remove_unused_requested = false;
    let mut reduce_project_requested = false;

    ui.horizontal(|ui| {
        if project_icon_action(
            ui,
            "project-new-comp",
            crate::ui::icons::SVG_FILE_PLUS,
            "新規コンポジションを作成",
        )
        .clicked()
        {
            add_comp_requested = true;
        }

        if project_icon_action(
            ui,
            "project-import",
            crate::ui::icons::SVG_IMPORT,
            "映像・音声を読み込む",
        )
        .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Media Footage",
                    &[
                        "png", "jpg", "jpeg", "webp", "obj", "wav", "mp3", "m4a", "flac", "ogg",
                        "aiff", "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpeg", "mpg", "ts",
                        "m2ts", "av1", "ivf",
                    ],
                )
                .pick_file()
            {
                import_file_requested = Some(path);
            }
        }

        ui.menu_button("...", |ui| {
            if ui.button("新規フォルダー").clicked() {
                add_folder_requested = true;
            }

            if ui
                .button("未使用素材を削除")
                .on_hover_text("プロジェクトで使われていない素材を削除します")
                .clicked()
            {
                remove_unused_requested = true;
            }

            if ui
                .button("プロジェクトを整理")
                .on_hover_text("アクティブなコンポジションと参照素材だけを残します")
                .clicked()
            {
                reduce_project_requested = true;
            }
        });
        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if project_view_toggle(
                ui,
                "project-view-list",
                SVG_LIST_VIEW,
                !grid_view,
                "一覧表示",
            )
            .clicked()
            {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(grid_view_id, false));
            }
            if project_view_toggle(
                ui,
                "project-view-grid",
                crate::ui::icons::SVG_GRID,
                grid_view,
                "カード表示",
            )
            .clicked()
            {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(grid_view_id, true));
            }
        });
    });

    ui.add_space(6.0);
    ui.separator();

    // ── Project Items List (Assets & Compositions Bin Tree) ──
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
                    let columns = if ui.available_width() >= 132.0 { 2 } else { 1 };
                    let gap = 6.0;
                    let card_width = ((ui.available_width() - gap * (columns - 1) as f32)
                        / columns as f32)
                        .max(62.0);
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
        let category_width = (ui.available_width() * 0.30).clamp(78.0, 94.0);
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
                        ("model3d", "3D", crate::ui::icons::SVG_SWITCH_3D),
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
    if let Some(idx) = selected_asset_idx {
        if let Some(item) = current_project.assets.get(idx) {
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(0.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&item.name).strong());
                        match &item.item_type {
                            ProjectItemType::Composition { comp_idx } => {
                                if let Some(c) = current_project.compositions.get(*comp_idx) {
                                    ui.small(format!("Resolution: {} x {} px", c.width, c.height));
                                    ui.small(format!("Frame Rate: {} fps", c.fps));
                                    ui.small(format!("Duration: {} frames", c.duration_frames));
                                }
                            }
                            ProjectItemType::Image {
                                path,
                                width,
                                height,
                            } => {
                                ui.small(format!("File: {}", path));
                                ui.small(format!("Dimensions: {} x {} px", width, height));
                            }
                            ProjectItemType::Model3D { path } => {
                                ui.small(format!("File: {}", path));
                                ui.small("Format: Wavefront OBJ");
                            }
                            ProjectItemType::Video { path, duration_sec } => {
                                ui.small(format!("File: {}", path));
                                if duration_sec.is_finite() && *duration_sec > 0.0 {
                                    ui.small(format!("Duration: {:.1}s", duration_sec));
                                } else {
                                    ui.small("Duration: not measured");
                                }
                            }
                            ProjectItemType::Audio { path, duration_sec } => {
                                ui.small(format!("File: {}", path));
                                if duration_sec.is_finite() && *duration_sec > 0.0 {
                                    ui.small(format!("Length: {:.2} seconds", duration_sec));
                                } else {
                                    ui.small("Length: not measured");
                                }
                            }
                            ProjectItemType::Solid { color } => {
                                ui.small(format!(
                                    "Color: R:{:.0} G:{:.0} B:{:.0}",
                                    color[0] * 255.0,
                                    color[1] * 255.0,
                                    color[2] * 255.0
                                ));
                            }
                            ProjectItemType::Folder { .. } => {
                                ui.small("Project Bin Directory");
                            }
                        }

                        ui.add_space(4.0);
                        if custom_widgets::ae_button(ui, "Add to Active Comp").clicked() {
                            add_to_timeline_item = Some(item.clone());
                        }
                    });
            }
        }

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

    // Lazy mutation: Clone project ONLY on action trigger!
    if add_comp_requested
        || import_file_requested.is_some()
        || add_folder_requested
        || add_to_timeline_item.is_some()
        || remove_unused_requested
        || reduce_project_requested
        || replace_info.is_some()
    {
        let mut temp_project = app.history.current().clone();
        let mut changed = false;

        if add_comp_requested {
            let comp_len = temp_project.compositions.len() + 1;
            let new_comp = crate::core::timeline::Composition::new(
                temp_project.next_composition_id("comp"),
                format!("Comp {}", comp_len),
                1920,
                1080,
                30,
                300,
            );
            temp_project.compositions.push(new_comp);
            let asset_id = temp_project.next_project_item_id("item_comp");
            temp_project.assets.push(ProjectItem::new(
                asset_id,
                format!("Comp {}", comp_len),
                ProjectItemType::Composition {
                    comp_idx: temp_project.compositions.len() - 1,
                },
            ));
            changed = true;
        }

        if let Some(path) = import_file_requested {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            match classify_imported_media(&path) {
                Ok(item_type) => {
                    let asset_id = temp_project.next_project_item_id("imported");
                    temp_project
                        .assets
                        .push(ProjectItem::new(asset_id, file_name, item_type));
                    changed = true;
                }
                Err(error) => app.toasts.error(error),
            }
        }

        if let Some((idx, target)) = move_to_folder {
            if let Some(it) = temp_project.assets.get_mut(idx) {
                it.parent_folder = target;
                changed = true;
            }
        }

        if add_folder_requested {
            let folder_count = temp_project.assets.len() + 1;
            let folder_id = temp_project.next_project_item_id("folder");
            temp_project.assets.push(ProjectItem::new(
                folder_id,
                format!("Assets Folder {}", folder_count),
                ProjectItemType::Folder {
                    name: format!("Folder {}", folder_count),
                },
            ));
            changed = true;
        }

        if let Some(item) = add_to_timeline_item {
            let comp = temp_project.active_composition_mut();
            let new_layer = match item.item_type {
                ProjectItemType::Image { path, .. } => Layer::new(
                    comp.next_layer_id("layer_asset"),
                    item.name,
                    LayerType::Image { path },
                    comp.duration_frames,
                ),
                ProjectItemType::Model3D { path } => {
                    let mut layer = Layer::new(
                        comp.next_layer_id("layer_model3d"),
                        item.name,
                        LayerType::Model3D { path },
                        comp.duration_frames,
                    );
                    layer.is_3d = true;
                    layer.transform.position = crate::core::property::Animatable::new_constant([
                        comp.width as f32 * 0.5,
                        comp.height as f32 * 0.5,
                    ]);
                    layer.transform_3d.position =
                        crate::core::property::Animatable::new_constant([
                            0.0,
                            0.0,
                            (comp.width.max(comp.height) as f32 * 0.6).max(600.0),
                        ]);
                    layer
                }
                ProjectItemType::Solid { color } => Layer::new(
                    comp.next_layer_id("layer_solid"),
                    item.name,
                    LayerType::Solid { color },
                    comp.duration_frames,
                ),
                ProjectItemType::Video { .. } => {
                    app.toasts
                        .error("動画の読み込みはバックグラウンドで開始してください。");
                    return;
                }
                ProjectItemType::Audio {
                    path,
                    duration_sec: _,
                } => Layer::new(
                    comp.next_layer_id("layer_audio"),
                    item.name,
                    LayerType::Audio {
                        path,
                        volume: crate::core::property::Animatable::new_constant(1.0),
                    },
                    comp.duration_frames,
                ),
                _ => Layer::new(
                    comp.next_layer_id("layer_gen"),
                    item.name,
                    LayerType::Solid {
                        color: [0.5, 0.5, 0.5, 1.0],
                    },
                    comp.duration_frames,
                ),
            };
            comp.add_layer(new_layer);
            changed = true;
        }

        if remove_unused_requested {
            let mut used_names = std::collections::HashSet::new();
            for c in &temp_project.compositions {
                for l in &c.layers {
                    used_names.insert(l.name.clone());
                    match &l.layer_type {
                        LayerType::Image { path } => {
                            used_names.insert(path.clone());
                        }
                        LayerType::Model3D { path } => {
                            used_names.insert(path.clone());
                        }
                        LayerType::Video { source, .. } => {
                            used_names.insert(source.clone());
                        }
                        LayerType::Audio { path, .. } => {
                            used_names.insert(path.clone());
                        }
                        _ => {}
                    }
                }
            }
            let before_count = temp_project.assets.len();
            temp_project.assets.retain(|a| {
                matches!(
                    a.item_type,
                    ProjectItemType::Composition { .. } | ProjectItemType::Folder { .. }
                ) || used_names.contains(&a.name)
                    || a.media_path().is_some_and(|path| used_names.contains(path))
            });
            let removed = before_count.saturating_sub(temp_project.assets.len());
            ui.ctx().data_mut(|data| {
                data.remove_temp::<usize>(egui::Id::new("selected_project_asset"));
            });
            app.toasts.info(format!("Removed {} unused items", removed));
            changed = true;
        }

        if reduce_project_requested {
            let active_idx = temp_project.active_composition_idx;
            if active_idx < temp_project.compositions.len() {
                let keep_comp = temp_project.compositions[active_idx].clone();
                temp_project.compositions = vec![keep_comp];
                temp_project.active_composition_idx = 0;
                app.toasts
                    .info("Project reduced to active composition and its dependencies");
                changed = true;
            }
        }

        if let Some((asset_idx, new_path, _new_name)) = replace_info {
            start_replace_footage_import(
                app,
                ui.ctx(),
                asset_idx,
                std::path::PathBuf::from(new_path),
            );
        }

        if changed {
            app.commit_project(temp_project);
        }
    }
}

