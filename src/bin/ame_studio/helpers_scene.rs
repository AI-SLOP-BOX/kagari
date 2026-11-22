pub(crate) fn media_file_status(path: &str) -> String {
    let media_path = std::path::Path::new(path);
    let name = media_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned());
    if media_path.is_file() {
        name
    } else {
        format!("{name} · ファイルが見つかりません")
    }
}

pub(crate) fn scene_layer_kind_label(layer: &kagari_vfx::core::timeline::Layer) -> &'static str {
    use kagari_vfx::core::timeline::{LayerType, SceneObjectRef};

    match layer.scene_object.as_ref() {
        Some(SceneObjectRef::Camera { .. }) => "カメラ",
        Some(SceneObjectRef::Light { .. }) => "ライト",
        None => match &layer.layer_type {
            LayerType::Particle { .. } => "パーティクル",
            LayerType::Image { .. } => "画像",
            LayerType::Video { .. } => "動画",
            LayerType::Audio { .. } => "音声",
            LayerType::Text { .. } => "テキスト",
            LayerType::Shape { .. } => "シェイプ",
            LayerType::Solid { .. } => "平面",
            LayerType::Model3D { .. } => "3Dモデル",
            LayerType::PreComp { .. } => "プリコンポーズ",
            LayerType::Null => "ヌル",
            LayerType::AdjustmentLayer => "調整レイヤー",
        },
    }
}

pub(crate) fn scene_layer_depths(
    layers: &[kagari_vfx::core::timeline::Layer],
    layer_by_id: &std::collections::HashMap<&str, &kagari_vfx::core::timeline::Layer>,
) -> std::collections::HashMap<String, usize> {
    let mut depths = std::collections::HashMap::with_capacity(layers.len());
    for layer in layers {
        if depths.contains_key(&layer.id) {
            continue;
        }
        let mut path = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current = Some(layer);
        let mut base_depth = 0usize;
        while let Some(candidate) = current {
            if let Some(depth) = depths.get(candidate.id.as_str()) {
                base_depth = *depth;
                break;
            }
            if !visited.insert(candidate.id.as_str()) {
                break;
            }
            path.push(candidate);
            current = candidate
                .parent_id
                .as_deref()
                .and_then(|parent_id| layer_by_id.get(parent_id).copied());
        }
        for candidate in path.into_iter().rev() {
            let has_parent = candidate
                .parent_id
                .as_deref()
                .is_some_and(|parent_id| layer_by_id.contains_key(parent_id));
            base_depth = if has_parent {
                base_depth.saturating_add(1)
            } else {
                0
            };
            depths.insert(candidate.id.clone(), base_depth);
        }
    }
    depths
}

pub(crate) fn replace_project_media_path(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    old_path: &str,
    new_path: &str,
) {
    use kagari_vfx::core::timeline::LayerType;
    for composition in compositions {
        for layer in &mut composition.layers {
            match &mut layer.layer_type {
                LayerType::Image { path }
                | LayerType::Audio { path, .. }
                | LayerType::Model3D { path }
                    if path == old_path =>
                {
                    *path = new_path.to_owned();
                }
                _ => {}
            }
        }
        replace_project_media_path(&mut composition.sub_compositions, old_path, new_path);
    }
}

pub(crate) fn remove_precomp_references(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    removed_comp_id: &str,
) -> usize {
    use kagari_vfx::core::timeline::LayerType;
    let mut removed = 0;
    for composition in compositions {
        let previous_count = composition.layers.len();
        composition.layers.retain(|layer| {
            !matches!(
                &layer.layer_type,
                LayerType::PreComp { comp_id } if comp_id == removed_comp_id
            )
        });
        removed += previous_count - composition.layers.len();
        removed += remove_precomp_references(&mut composition.sub_compositions, removed_comp_id);
    }
    removed
}

pub(crate) fn clone_composition_asset_tree(
    project_compositions: &[kagari_vfx::core::timeline::Composition],
    source_index: usize,
    root_id: &str,
) -> Result<Vec<kagari_vfx::core::timeline::Composition>, &'static str> {
    use kagari_vfx::core::timeline::LayerType;
    fn collect_existing_ids(
        composition: &kagari_vfx::core::timeline::Composition,
        ids: &mut std::collections::HashSet<String>,
    ) {
        ids.insert(composition.id.clone());
        for child in &composition.sub_compositions {
            collect_existing_ids(child, ids);
        }
    }
    fn assign_tree_ids(
        composition: &kagari_vfx::core::timeline::Composition,
        root_id: &str,
        next: &mut usize,
        ids: &mut std::collections::HashMap<String, String>,
        reserved_ids: &mut std::collections::HashSet<String>,
    ) {
        let id = loop {
            let candidate = if *next == 0 {
                root_id.to_owned()
            } else {
                format!("{root_id}_nested_{}", *next)
            };
            *next += 1;
            if reserved_ids.insert(candidate.clone()) {
                break candidate;
            }
        };
        ids.entry(composition.id.clone()).or_insert(id);
        for child in &composition.sub_compositions {
            assign_tree_ids(child, root_id, next, ids, reserved_ids);
        }
    }
    fn collect_references(
        composition: &kagari_vfx::core::timeline::Composition,
        references: &mut Vec<String>,
    ) {
        references.extend(composition.layers.iter().filter_map(|layer| {
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                Some(comp_id.clone())
            } else {
                None
            }
        }));
        for child in &composition.sub_compositions {
            collect_references(child, references);
        }
    }
    fn apply_ids(
        composition: &mut kagari_vfx::core::timeline::Composition,
        ids: &std::collections::HashMap<String, String>,
    ) {
        if let Some(id) = ids.get(&composition.id) {
            composition.id = id.clone();
        }
        for layer in &mut composition.layers {
            if let LayerType::PreComp { comp_id } = &mut layer.layer_type {
                if let Some(id) = ids.get(comp_id) {
                    *comp_id = id.clone();
                }
            }
        }
        for child in &mut composition.sub_compositions {
            apply_ids(child, ids);
        }
    }

    let Some(source) = project_compositions.get(source_index) else {
        return Err("配置するコンポジションが見つかりません。");
    };
    let mut cloned_roots = vec![source.clone()];
    let mut reserved_ids = std::collections::HashSet::new();
    for composition in project_compositions {
        collect_existing_ids(composition, &mut reserved_ids);
    }
    let mut ids = std::collections::HashMap::new();
    let mut next_id = 0;
    assign_tree_ids(
        &cloned_roots[0],
        root_id,
        &mut next_id,
        &mut ids,
        &mut reserved_ids,
    );
    let mut next_dependency = 0;
    let mut index = 0;
    while index < cloned_roots.len() {
        let mut references = Vec::new();
        collect_references(&cloned_roots[index], &mut references);
        for reference in references {
            if ids.contains_key(&reference) {
                continue;
            }
            let Some(dependency) = project_compositions
                .iter()
                .find(|composition| composition.id == reference)
            else {
                return Err("参照先コンポジションが見つからないため配置できません。");
            };
            let dependency_id = format!("{root_id}_dependency_{next_dependency}");
            next_dependency += 1;
            assign_tree_ids(
                dependency,
                &dependency_id,
                &mut next_id,
                &mut ids,
                &mut reserved_ids,
            );
            cloned_roots.push(dependency.clone());
        }
        index += 1;
    }
    for composition in &mut cloned_roots {
        apply_ids(composition, &ids);
    }
    Ok(cloned_roots)
}

pub(crate) fn replace_project_video_references(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    old_path: &str,
    asset: &kagari_vfx::core::video_import::VideoAsset,
) {
    use kagari_vfx::core::timeline::LayerType;
    for composition in compositions {
        for layer in &mut composition.layers {
            let settings = match &layer.layer_type {
                LayerType::Video {
                    source,
                    speed,
                    audio_volume,
                    ..
                } if source == old_path => Some((*speed, audio_volume.clone())),
                _ => None,
            };
            if let Some((speed, audio_volume)) = settings {
                layer.layer_type = LayerType::Video {
                    source: asset.source_path.clone(),
                    frames_dir: asset.frames_dir.clone(),
                    frame_count: asset.frame_count,
                    audio_wav: asset.audio_wav.clone(),
                    audio_volume,
                    speed,
                };
            }
        }
        replace_project_video_references(&mut composition.sub_compositions, old_path, asset);
    }
}

pub(crate) fn scene_layer_hidden_by_collapse(
    layer_by_id: &std::collections::HashMap<&str, &kagari_vfx::core::timeline::Layer>,
    layer: &kagari_vfx::core::timeline::Layer,
    collapsed: &std::collections::HashSet<String>,
) -> bool {
    let mut parent_id = layer.parent_id.as_deref();
    let mut visited = std::collections::HashSet::new();
    while let Some(id) = parent_id {
        if collapsed.contains(id) || !visited.insert(id) {
            return collapsed.contains(id);
        }
        let Some(parent) = layer_by_id.get(id).copied() else {
            return false;
        };
        parent_id = parent.parent_id.as_deref();
    }
    false
}

pub(crate) fn scene_layer_children_by_parent<'a>(
    layers: &'a [kagari_vfx::core::timeline::Layer],
) -> std::collections::HashMap<&'a str, Vec<&'a str>> {
    let mut children_by_parent = std::collections::HashMap::<&'a str, Vec<&'a str>>::new();
    for layer in layers {
        if let Some(parent_id) = layer.parent_id.as_deref() {
            children_by_parent
                .entry(parent_id)
                .or_default()
                .push(layer.id.as_str());
        }
    }
    children_by_parent
}

pub(crate) fn scene_layer_hidden_ids(
    children_by_parent: &std::collections::HashMap<&str, Vec<&str>>,
    collapsed: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let mut hidden = std::collections::HashSet::new();
    let mut visited = std::collections::HashSet::new();
    let mut pending: Vec<&str> = collapsed.iter().map(String::as_str).collect();
    while let Some(parent_id) = pending.pop() {
        if !visited.insert(parent_id) {
            continue;
        }
        if let Some(children) = children_by_parent.get(parent_id) {
            for child_id in children {
                hidden.insert((*child_id).to_owned());
                pending.push(*child_id);
            }
        }
    }
    hidden
}

pub(crate) fn scene_search_visible_layer_ids(
    layers: &[kagari_vfx::core::timeline::Layer],
    layer_by_id: &std::collections::HashMap<&str, &kagari_vfx::core::timeline::Layer>,
    query: &str,
) -> std::collections::HashSet<String> {
    if query.is_empty() {
        return layers.iter().map(|layer| layer.id.clone()).collect();
    }

    let mut visible = std::collections::HashSet::new();
    let mut pending = Vec::new();
    for layer in layers
        .iter()
        .filter(|layer| scene_layer_matches_query(layer, query))
    {
        if visible.insert(layer.id.clone()) {
            pending.push(layer);
        }
    }
    while let Some(layer) = pending.pop() {
        let Some(parent_id) = layer.parent_id.as_deref() else {
            continue;
        };
        let Some(parent) = layer_by_id.get(parent_id).copied() else {
            continue;
        };
        if visible.insert(parent.id.clone()) {
            pending.push(parent);
        }
    }
    visible
}

pub(crate) fn scene_layer_matches_query(
    layer: &kagari_vfx::core::timeline::Layer,
    query: &str,
) -> bool {
    if layer.name.to_lowercase().contains(query) {
        return true;
    }
    let matches_text = |value: &str| value.to_lowercase().contains(query);
    let matches_kind_or_source = match layer.scene_object.as_ref() {
        Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { .. }) => matches_text("カメラ"),
        Some(kagari_vfx::core::timeline::SceneObjectRef::Light { .. }) => matches_text("ライト"),
        None => match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Solid { .. } => matches_text("平面"),
            kagari_vfx::core::timeline::LayerType::Image { path } => {
                matches_text("画像") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::Model3D { path } => {
                matches_text("3dモデル") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                source, frames_dir, ..
            } => matches_text("動画") || matches_text(source) || matches_text(frames_dir),
            kagari_vfx::core::timeline::LayerType::Text {
                text, font_family, ..
            } => matches_text("テキスト") || matches_text(text) || matches_text(font_family),
            kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => {
                matches_text("シェイプ") || matches_text(&format!("{shape_type:?}").to_lowercase())
            }
            kagari_vfx::core::timeline::LayerType::Null => matches_text("ヌル"),
            kagari_vfx::core::timeline::LayerType::PreComp { comp_id } => {
                matches_text("プリコンポーズ") || matches_text(comp_id)
            }
            kagari_vfx::core::timeline::LayerType::Audio { path, .. } => {
                matches_text("音声") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer => matches_text("調整レイヤー"),
            kagari_vfx::core::timeline::LayerType::Particle { .. } => matches_text("パーティクル"),
        },
    };
    matches_kind_or_source
        || (layer.locked && matches_text("ロック"))
        || (layer.visible && matches_text("表示"))
        || (!layer.visible && matches_text("非表示"))
        || (layer.solo && matches_text("ソロ"))
        || (layer.is_shy && matches_text("シャイ"))
        || (layer.motion_blur && matches_text("モーションブラー"))
}
