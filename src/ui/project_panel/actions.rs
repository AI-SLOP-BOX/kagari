use crate::core::timeline::{Layer, LayerType, ProjectItem, ProjectItemType};
use crate::ui::project_panel::media_replace::{classify_imported_media, start_replace_footage_import};
use crate::KagariApp;
use eframe::egui;

#[allow(clippy::too_many_arguments)]
pub fn apply_project_mutations(
    app: &mut KagariApp,
    ctx: &egui::Context,
    add_comp_requested: bool,
    import_file_requested: Option<std::path::PathBuf>,
    add_folder_requested: bool,
    add_to_timeline_item: Option<ProjectItem>,
    move_to_folder: Option<(usize, Option<String>)>,
    remove_unused_requested: bool,
    reduce_project_requested: bool,
    replace_info: Option<(usize, String, String)>,
) {
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
            ctx.data_mut(|data| {
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
                ctx,
                asset_idx,
                std::path::PathBuf::from(new_path),
            );
        }

        if changed {
            app.commit_project(temp_project);
        }
    }
}
