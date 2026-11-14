use super::*;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum ProjectAssetUsageKey<'a> {
    Composition(&'a str),
    Image(&'a str),
    Video(&'a str),
    Audio(&'a str),
    Model3D(&'a str),
}

fn project_asset_folder_path_indexed(
    folder_id: Option<&str>,
    asset_by_id: &std::collections::HashMap<&str, &kagari_vfx::core::timeline::ProjectItem>,
    cache: &mut std::collections::HashMap<String, String>,
) -> String {
    let mut chain = Vec::new();
    let mut next = folder_id;
    let mut visited = std::collections::HashSet::new();
    let mut path = String::new();
    while let Some(id) = next {
        if let Some(cached) = cache.get(id) {
            path.clone_from(cached);
            break;
        }
        if !visited.insert(id) {
            break;
        }
        let Some(folder) = asset_by_id.get(id) else {
            break;
        };
        chain.push((id, folder.name.as_str()));
        next = folder.parent_folder.as_deref();
    }
    for (id, name) in chain.into_iter().rev() {
        if !path.is_empty() {
            path.push_str(" / ");
        }
        path.push_str(name);
        cache.insert(id.to_owned(), path.clone());
    }
    path
}

impl AfterEffectsStudio {
    pub(crate) fn sync_project_asset_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_folder = self
            .asset_folder_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id));
        self.ui.set_binding(
            "asset_folder_name",
            Value::String(
                current_folder
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| "プロジェクト素材".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "asset_folder_has_parent",
            Value::Bool(current_folder.is_some_and(|item| item.parent_folder.is_some())),
        )?;
        self.ui.set_binding(
            "new_asset_folder_name",
            Value::String(self.new_asset_folder_name.clone()),
        )?;
        self.ui.set_binding(
            "project_asset_search",
            Value::String(self.project_asset_search.clone()),
        )?;
        self.ui.set_binding(
            "project_asset_search_all_folders",
            Value::Bool(self.project_asset_search_all_folders),
        )?;
        self.ui.set_binding(
            "project_assets_sort_by_type",
            Value::Bool(self.project_assets_sort_by_type),
        )?;
        let selected_project_asset_name = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .map(|item| item.name.clone())
            .unwrap_or_default();
        let selected_project_asset_details = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .map(|item| {
                use kagari_vfx::core::timeline::ProjectItemType;
                let file_size = |path: &str| {
                    std::fs::metadata(path).ok().map(|metadata| {
                        let bytes = metadata.len();
                        if bytes >= 1024 * 1024 {
                            format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
                        } else if bytes >= 1024 {
                            format!("{:.0} KiB", bytes as f64 / 1024.0)
                        } else {
                            format!("{bytes} B")
                        }
                    })
                };
                match &item.item_type {
                    ProjectItemType::Composition { comp_idx } => self
                        .project_model
                        .compositions
                        .get(*comp_idx)
                        .map(|composition| {
                            format!(
                                "{} × {} px · {} fps · {:.1} 秒 · {} レイヤー",
                                composition.width,
                                composition.height,
                                composition.fps,
                                composition.duration_frames as f64 / composition.fps.max(1) as f64,
                                composition.layers.len()
                            )
                        })
                        .unwrap_or_else(|| "コンポジション".to_owned()),
                    ProjectItemType::Image {
                        path,
                        width,
                        height,
                    } => format!(
                        "{} × {} px{}",
                        width,
                        height,
                        file_size(path)
                            .map(|size| format!(" · {size}"))
                            .unwrap_or_default()
                    ),
                    ProjectItemType::Video { path, duration_sec } => format!(
                        "{duration_sec:.1} 秒{}",
                        file_size(path)
                            .map(|size| format!(" · {size}"))
                            .unwrap_or_default()
                    ),
                    ProjectItemType::Audio { path, duration_sec } => {
                        let duration_sec = self
                            .waveform_cache
                            .get(path)
                            .map(|waveform| waveform.duration_seconds)
                            .unwrap_or(*duration_sec);
                        format!(
                            "{duration_sec:.1} 秒{}",
                            file_size(path)
                                .map(|size| format!(" · {size}"))
                                .unwrap_or_default()
                        )
                    }
                    ProjectItemType::Model3D { path } => file_size(path)
                        .map(|size| format!("3Dモデル · {size}"))
                        .unwrap_or_else(|| "3Dモデル".to_owned()),
                    ProjectItemType::Solid { color } => format!(
                        "色 #{:02X}{:02X}{:02X}",
                        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8
                    ),
                    ProjectItemType::Folder { .. } => "フォルダー".to_owned(),
                }
            })
            .unwrap_or_default();
        let selected_project_asset_can_move_here =
            self.selected_project_asset_id.as_deref().is_some_and(|id| {
                project_asset_can_move_to_folder(
                    &self.project_model.assets,
                    id,
                    self.asset_folder_id.as_deref(),
                )
            });
        let selected_project_asset_preview_status = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .map(|item| {
                use kagari_vfx::core::timeline::ProjectItemType;
                if self.asset_preview_worker_failed
                    && !matches!(
                        item.item_type,
                        ProjectItemType::Audio { .. }
                            | ProjectItemType::Solid { .. }
                            | ProjectItemType::Folder { .. }
                    )
                {
                    return "素材プレビュー処理が停止しました。アプリを再起動してください。".to_owned();
                }
                if self.asset_preview_failed_asset_id.as_deref() == Some(item.id.as_str()) {
                    return "プレビューを読み込めません。ファイル形式とパスを確認してください。詳細はアプリのログに記録されています。".to_owned();
                }
                let preview_state = |cache_key: String| {
                    self.asset_render_preview_cache
                        .get(&cache_key)
                        .map(Option::is_some)
                };
                match &item.item_type {
                    ProjectItemType::Image { .. } if item.is_media_missing() => {
                        "画像ファイルが見つかりません。素材を再指定してください。".to_owned()
                    }
                    ProjectItemType::Image { path, .. } => {
                        match preview_state(asset_image_preview_cache_key(&item.id, path)) {
                            Some(true) => "画像・SVGのプレビュー".to_owned(),
                            Some(false) => "画像・SVGを読み込めません。ファイル形式とパスを確認してください。".to_owned(),
                            None => "画像・SVGのプレビューを読み込み中…".to_owned(),
                        }
                    }
                    ProjectItemType::Video { .. } if item.is_media_missing() => {
                        "動画ファイルが見つかりません。素材を再指定してください。".to_owned()
                    }
                    ProjectItemType::Video { path, .. } => find_video_preview_frame(
                        &self.project_model.compositions,
                        path,
                    )
                    .map(|frame| {
                        let key = asset_image_preview_cache_key(
                            &item.id,
                            &frame.to_string_lossy(),
                        );
                        match preview_state(key) {
                            Some(true) => "動画の先頭フレームをプレビュー中。配置すると再生できます。".to_owned(),
                            Some(false) => "動画の先頭フレームを読み込めません。素材を再指定してください。".to_owned(),
                            None => "動画の先頭フレームを読み込み中…".to_owned(),
                        }
                    })
                    .unwrap_or_else(|| "動画はシーンへ配置するとコンポジションプレビューで確認できます。".to_owned()),
                    ProjectItemType::Audio { .. } if item.is_media_missing() => {
                        "音声ファイルが見つかりません。素材を再指定してください。".to_owned()
                    }
                    ProjectItemType::Audio { path, .. } => {
                        if self.waveform_cache.contains_key(path) {
                            "音声波形をプレビュー中。配置するとタイムラインでも編集できます。".to_owned()
                        } else if self.waveform_worker_failed {
                            "音声波形の読み込み処理を開始できません。アプリを再起動してください。".to_owned()
                        } else if self.waveform_failed.contains_key(path) {
                            "音声をデコードできません。ファイル形式と音声トラックを確認してください。".to_owned()
                        } else {
                            "音声波形をプレビューします。読み込み後に波形が表示されます。".to_owned()
                        }
                    }
                    ProjectItemType::Model3D { .. } if item.is_media_missing() => {
                        "3Dモデルファイルが見つかりません。素材を再指定してください。".to_owned()
                    }
                    ProjectItemType::Model3D { path } => {
                        match preview_state(model_preview_cache_key(path)) {
                            Some(true) => "3Dモデルのサムネイルを表示中。配置後はシーンで調整できます。".to_owned(),
                            Some(false) => "3Dモデルのプレビューを読み込めません。形式とパスを確認してください。詳細はアプリのログに記録されています。".to_owned(),
                            None => "3Dモデルのサムネイルを読み込み中…".to_owned(),
                        }
                    }
                    ProjectItemType::Composition { comp_idx } => {
                        let key = format!("composition:{}:{}", item.id, self.preview_revision);
                        match preview_state(key) {
                            Some(true) => "コンポジションの1フレーム目を表示中。「開く」または「配置」を選べます。".to_owned(),
                            Some(false) => "コンポジションのプレビューを作成できませんでした。".to_owned(),
                            None if self.project_model.compositions.get(*comp_idx).is_some() => "コンポジションの1フレーム目を読み込み中…".to_owned(),
                            None => "コンポジションが見つかりません。プロジェクト項目を更新してください。".to_owned(),
                        }
                    }
                    ProjectItemType::Solid { .. } => "平面素材の色プレビュー。配置後に色を調整できます。".to_owned(),
                    ProjectItemType::Folder { .. } => "フォルダーを開いて素材を選択してください。".to_owned(),
                }
            })
            .unwrap_or_else(|| "プロジェクト素材を選択するとプレビューします。".to_owned());
        self.ui.set_binding(
            "has_selected_project_asset",
            Value::Bool(!selected_project_asset_name.is_empty()),
        )?;
        self.ui.set_binding(
            "selected_project_asset_name",
            Value::String(selected_project_asset_name),
        )?;
        let selected_project_asset_is_image = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .is_some_and(|item| {
                matches!(
                    &item.item_type,
                    kagari_vfx::core::timeline::ProjectItemType::Image { .. }
                ) && !item.is_media_missing()
            });
        self.ui.set_binding(
            "selected_project_asset_is_image",
            Value::Bool(selected_project_asset_is_image),
        )?;
        self.ui.set_binding(
            "selected_project_asset_details",
            Value::String(selected_project_asset_details),
        )?;
        self.ui.set_binding(
            "selected_project_asset_can_move_here",
            Value::Bool(selected_project_asset_can_move_here),
        )?;
        self.ui.set_binding(
            "selected_project_asset_preview_status",
            Value::String(selected_project_asset_preview_status),
        )?;
        let asset_query = self.project_asset_search.trim().to_lowercase();
        let search_all_asset_folders =
            !asset_query.is_empty() && self.project_asset_search_all_folders;
        let mut project_asset_entries = self
            .project_model
            .assets
            .iter()
            .filter(|item| {
                search_all_asset_folders
                    || item.parent_folder.as_deref() == self.asset_folder_id.as_deref()
            })
            .collect::<Vec<_>>();
        let asset_by_id = self
            .project_model
            .assets
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect::<std::collections::HashMap<_, _>>();
        let mut folder_path_cache = std::collections::HashMap::new();
        let mut asset_folder_paths = std::collections::HashMap::new();
        for item in &self.project_model.assets {
            let Some(parent_folder) = item.parent_folder.as_deref() else {
                continue;
            };
            let path = project_asset_folder_path_indexed(
                Some(parent_folder),
                &asset_by_id,
                &mut folder_path_cache,
            );
            asset_folder_paths.insert(item.id.as_str(), path);
        }
        project_asset_entries.sort_by_cached_key(|item| {
            use kagari_vfx::core::timeline::ProjectItemType;
            let kind = |item: &kagari_vfx::core::timeline::ProjectItem| match &item.item_type {
                ProjectItemType::Composition { .. } => "コンポジション",
                ProjectItemType::Image { .. } => "画像",
                ProjectItemType::Model3D { .. } => "3Dモデル",
                ProjectItemType::Video { .. } => "動画",
                ProjectItemType::Audio { .. } => "音声",
                ProjectItemType::Solid { .. } => "平面",
                ProjectItemType::Folder { .. } => "フォルダー",
            };
            (
                if self.project_assets_sort_by_type {
                    kind(item)
                } else {
                    ""
                },
                item.name.to_lowercase(),
            )
        });
        let mut usage_counts = std::collections::HashMap::new();
        for composition in &self.project_model.compositions {
            for layer in &composition.layers {
                let key = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::PreComp { comp_id } => {
                        Some(ProjectAssetUsageKey::Composition(comp_id.as_str()))
                    }
                    kagari_vfx::core::timeline::LayerType::Image { path } => {
                        Some(ProjectAssetUsageKey::Image(path.as_str()))
                    }
                    kagari_vfx::core::timeline::LayerType::Video { source, .. } => {
                        Some(ProjectAssetUsageKey::Video(source.as_str()))
                    }
                    kagari_vfx::core::timeline::LayerType::Audio { path, .. } => {
                        Some(ProjectAssetUsageKey::Audio(path.as_str()))
                    }
                    kagari_vfx::core::timeline::LayerType::Model3D { path } => {
                        Some(ProjectAssetUsageKey::Model3D(path.as_str()))
                    }
                    _ => None,
                };
                if let Some(key) = key {
                    *usage_counts.entry(key).or_insert(0usize) += 1;
                }
            }
        }
        let folders_with_children = self
            .project_model
            .assets
            .iter()
            .filter_map(|item| item.parent_folder.as_deref())
            .collect::<std::collections::HashSet<_>>();
        let project_assets = project_asset_entries
            .into_iter()
            .filter(|item| {
                if asset_query.is_empty() {
                    return true;
                }
                use kagari_vfx::core::timeline::ProjectItemType;
                let kind = match &item.item_type {
                    ProjectItemType::Composition { .. } => "コンポジション",
                    ProjectItemType::Image { .. } => "画像 svg image",
                    ProjectItemType::Model3D { .. } => "3Dモデル model",
                    ProjectItemType::Video { .. } => "動画 video",
                    ProjectItemType::Audio { .. } => "音声 audio",
                    ProjectItemType::Solid { .. } => "平面 solid",
                    ProjectItemType::Folder { .. } => "フォルダー folder",
                };
                item.name.to_lowercase().contains(&asset_query)
                    || item
                        .media_path()
                        .is_some_and(|path| path.to_lowercase().contains(&asset_query))
                    || asset_folder_paths
                        .get(item.id.as_str())
                        .is_some_and(|path| path.to_lowercase().contains(&asset_query))
                    || kind.contains(&asset_query)
            })
            .map(|item| {
                use kagari_vfx::core::timeline::ProjectItemType;
                let (kind, source, placeable) = match &item.item_type {
                    ProjectItemType::Composition { .. } => ("コンポジション", String::new(), true),
                    ProjectItemType::Image { path, .. } => ("画像", path.clone(), true),
                    ProjectItemType::Model3D { path } => ("3Dモデル", path.clone(), true),
                    ProjectItemType::Video { path, .. } => ("動画", path.clone(), true),
                    ProjectItemType::Audio { path, .. } => ("音声", path.clone(), true),
                    ProjectItemType::Solid { .. } => ("平面", String::new(), true),
                    ProjectItemType::Folder { .. } => ("フォルダー", String::new(), false),
                };
                let missing = item.is_media_missing();
                let usage_key = match &item.item_type {
                    ProjectItemType::Composition { comp_idx } => self
                        .project_model
                        .compositions
                        .get(*comp_idx)
                        .map(|composition| {
                            ProjectAssetUsageKey::Composition(composition.id.as_str())
                        }),
                    ProjectItemType::Image { path, .. } => {
                        Some(ProjectAssetUsageKey::Image(path.as_str()))
                    }
                    ProjectItemType::Video { path, .. } => {
                        Some(ProjectAssetUsageKey::Video(path.as_str()))
                    }
                    ProjectItemType::Audio { path, .. } => {
                        Some(ProjectAssetUsageKey::Audio(path.as_str()))
                    }
                    ProjectItemType::Model3D { path } => {
                        Some(ProjectAssetUsageKey::Model3D(path.as_str()))
                    }
                    _ => None,
                };
                let usage_count = usage_key
                    .and_then(|key| usage_counts.get(&key))
                    .copied()
                    .unwrap_or(0);
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(item.id.clone())),
                    ("name".to_owned(), Value::String(item.name.clone())),
                    ("kind".to_owned(), Value::String(kind.to_owned())),
                    ("source".to_owned(), Value::String(source)),
                    (
                        "usage_count".to_owned(),
                        Value::String(usage_count.to_string()),
                    ),
                    (
                        "parent_path".to_owned(),
                        Value::String(
                            asset_folder_paths
                                .get(item.id.as_str())
                                .cloned()
                                .unwrap_or_default(),
                        ),
                    ),
                    ("missing".to_owned(), Value::Bool(missing)),
                    ("placeable".to_owned(), Value::Bool(placeable && !missing)),
                    (
                        "is_folder".to_owned(),
                        Value::Bool(matches!(&item.item_type, ProjectItemType::Folder { .. })),
                    ),
                    (
                        "is_composition".to_owned(),
                        Value::Bool(matches!(
                            &item.item_type,
                            ProjectItemType::Composition { .. }
                        )),
                    ),
                    (
                        "removable".to_owned(),
                        Value::Bool(match &item.item_type {
                            ProjectItemType::Composition { .. } => false,
                            ProjectItemType::Folder { .. } => {
                                !folders_with_children.contains(item.id.as_str())
                            }
                            _ => true,
                        }),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(
                            self.selected_project_asset_id.as_deref() == Some(item.id.as_str()),
                        ),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_project_assets",
            Value::Bool(!project_assets.is_empty()),
        )?;
        self.ui
            .set_binding("project_assets", Value::List(project_assets))?;
        Ok(())
    }
}
