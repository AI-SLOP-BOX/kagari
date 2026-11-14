use crate::core::timeline::{Composition, LayerType, ProjectItemType};
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

mod model_companions;
pub(super) use model_companions::{model_companion_references, rewrite_model_companions};

const MAX_MODEL_COMPANION_LINE_BYTES: u64 = 1024 * 1024;

fn read_model_line(
    reader: &mut BufReader<std::fs::File>,
    line: &mut Vec<u8>,
) -> Result<usize, String> {
    line.clear();
    let bytes_read = reader
        .take(MAX_MODEL_COMPANION_LINE_BYTES + 1)
        .read_until(b'\n', line)
        .map_err(|error| format!("Cannot read model companion text: {error}"))?;
    if bytes_read as u64 > MAX_MODEL_COMPANION_LINE_BYTES {
        return Err(format!(
            "Model companion contains a line longer than {} MiB",
            MAX_MODEL_COMPANION_LINE_BYTES / (1024 * 1024)
        ));
    }
    Ok(bytes_read)
}

fn model_line_text(line: &[u8]) -> Result<&str, String> {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    std::str::from_utf8(line).map_err(|error| format!("Model companion is not UTF-8 text: {error}"))
}

pub fn collect_production_document(
    document: &crate::core::production_document::ProductionDocument,
    destination_parent: &Path,
    source_project_dir: &Path,
) -> Result<(PathBuf, usize), String> {
    document
        .validate()
        .map_err(|error| format!("Cannot collect invalid production document: {error}"))?;
    let root_name = format!(
        "{}_collected",
        safe_component(&document.project.active_composition().name)
    );
    let bundle_dir = destination_parent.join(root_name);
    std::fs::create_dir(&bundle_dir).map_err(|error| {
        format!(
            "Cannot create collection folder '{}': {error}",
            bundle_dir.display()
        )
    })?;
    let result = (|| {
        let footage_dir = bundle_dir.join("Footage");
        std::fs::create_dir(&footage_dir)
            .map_err(|error| format!("Cannot create Footage folder: {error}"))?;

        let mut raw_paths = BTreeSet::new();
        for asset in &document.project.assets {
            if let Some(path) = asset.media_path() {
                raw_paths.insert(path.to_string());
            }
        }
        for composition in &document.project.compositions {
            collect_composition_paths(composition, &mut raw_paths);
        }
        let mut sequence_cache_dirs = BTreeSet::new();
        for composition in &document.project.compositions {
            collect_image_sequence_cache_dirs(composition, &mut sequence_cache_dirs);
        }

        let mut inspected = BTreeSet::new();
        let mut pending: Vec<String> = raw_paths.iter().cloned().collect();
        while let Some(raw_path) = pending.pop() {
            let raw = Path::new(&raw_path);
            let candidate = if raw.is_absolute() {
                raw.to_path_buf()
            } else {
                source_project_dir.join(raw)
            };
            let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
                format!(
                    "Cannot collect referenced media '{}': {error}",
                    candidate.display()
                )
            })?;
            if !canonical.is_file() {
                return Err(format!(
                    "Referenced media is not a file: {}",
                    canonical.display()
                ));
            }
            if !inspected.insert(canonical.clone()) {
                continue;
            }
            for reference in model_companion_references(&canonical)? {
                let companion = canonical
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(reference);
                let companion = std::fs::canonicalize(&companion).map_err(|error| {
                    format!(
                        "Cannot collect model dependency '{}': {error}",
                        companion.display()
                    )
                })?;
                if !companion.is_file() {
                    return Err(format!(
                        "Model dependency is not a file: {}",
                        companion.display()
                    ));
                }
                let companion = companion.to_string_lossy().into_owned();
                if raw_paths.insert(companion.clone()) {
                    pending.push(companion);
                }
            }
        }

        let mut canonical_names = HashMap::<PathBuf, String>::new();
        let mut raw_names = HashMap::<String, String>::new();
        let mut file_count = 0usize;
        for raw_path in raw_paths {
            let raw = Path::new(&raw_path);
            let candidate = if raw.is_absolute() {
                raw.to_path_buf()
            } else {
                source_project_dir.join(raw)
            };
            let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
                format!(
                    "Cannot collect referenced media '{}': {error}",
                    candidate.display()
                )
            })?;
            if !canonical.is_file() {
                return Err(format!(
                    "Referenced media is not a file: {}",
                    canonical.display()
                ));
            }
            let name = if let Some(existing) = canonical_names.get(&canonical) {
                existing.clone()
            } else {
                let original_name = canonical
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("media");
                let name = format!("{:04}_{}", file_count + 1, safe_component(original_name));
                std::fs::copy(&canonical, footage_dir.join(&name)).map_err(|error| {
                    format!(
                        "Could not copy '{}' into collection: {error}",
                        canonical.display()
                    )
                })?;
                canonical_names.insert(canonical, name.clone());
                file_count += 1;
                name
            };
            raw_names.insert(raw_path, name);
        }

        for (canonical, name) in &canonical_names {
            rewrite_model_companions(canonical, &footage_dir.join(name), &canonical_names)?;
        }
        for (index, raw_cache_dir) in sequence_cache_dirs.iter().enumerate() {
            let raw_path = Path::new(raw_cache_dir);
            let source = if raw_path.is_absolute() {
                raw_path.to_path_buf()
            } else {
                source_project_dir.join(raw_path)
            };
            let canonical = std::fs::canonicalize(&source).map_err(|error| {
                format!(
                    "Cannot collect image-sequence cache '{}': {error}",
                    source.display()
                )
            })?;
            if !canonical.is_dir() {
                return Err(format!(
                    "Image-sequence cache is not a directory: {}",
                    canonical.display()
                ));
            }
            let collected_name = format!("_sequence_cache_{:04}", index + 1);
            let collected_path = footage_dir.join(&collected_name);
            file_count += copy_directory_tree(&canonical, &collected_path)?;
            raw_names.insert(raw_cache_dir.clone(), collected_name);
        }

        let mut collected = document.clone();
        for asset in &mut collected.project_mut().assets {
            if let ProjectItemType::Image { path, .. }
            | ProjectItemType::Model3D { path }
            | ProjectItemType::Video { path, .. }
            | ProjectItemType::Audio { path, .. } = &mut asset.item_type
            {
                *path = raw_names
                    .get(path)
                    .cloned()
                    .ok_or_else(|| format!("Collected asset path is missing for '{path}'"))?;
            }
        }
        for composition in &mut collected.project_mut().compositions {
            rewrite_composition_paths(composition, &raw_names)?;
        }
        let project_path = bundle_dir.join("collected_project.json");
        collected
            .save_atomic(&project_path)
            .map_err(|error| format!("Could not save collected production document: {error}"))?;
        Ok((project_path, file_count))
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&bundle_dir);
    }
    result
}

fn safe_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "Project".to_string()
    } else {
        sanitized
    }
}

fn collect_composition_paths(composition: &Composition, paths: &mut BTreeSet<String>) {
    for layer in &composition.layers {
        match &layer.layer_type {
            LayerType::Image { path }
            | LayerType::Model3D { path }
            | LayerType::Audio { path, .. } => {
                paths.insert(path.clone());
            }
            LayerType::Video { source, .. } => {
                paths.insert(source.clone());
            }
            _ => {}
        }
    }
    for child in &composition.sub_compositions {
        collect_composition_paths(child, paths);
    }
}

fn collect_image_sequence_cache_dirs(composition: &Composition, paths: &mut BTreeSet<String>) {
    for layer in &composition.layers {
        if let LayerType::Video {
            source, frames_dir, ..
        } = &layer.layer_type
        {
            if !frames_dir.is_empty()
                && crate::core::video_import::is_image_sequence_cache(frames_dir, source)
            {
                paths.insert(frames_dir.clone());
            }
        }
    }
    for child in &composition.sub_compositions {
        collect_image_sequence_cache_dirs(child, paths);
    }
}

fn copy_directory_tree(source: &Path, destination: &Path) -> Result<usize, String> {
    std::fs::create_dir(destination).map_err(|error| {
        format!(
            "Cannot create collected media directory '{}': {error}",
            destination.display()
        )
    })?;
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];
    let mut file_count = 0usize;
    while let Some((current_source, current_destination)) = pending.pop() {
        let entries = std::fs::read_dir(&current_source).map_err(|error| {
            format!(
                "Cannot read media directory '{}': {error}",
                current_source.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Cannot enumerate media directory '{}': {error}",
                    current_source.display()
                )
            })?;
            let entry_path = entry.path();
            let entry_type = entry.file_type().map_err(|error| {
                format!(
                    "Cannot inspect media item '{}': {error}",
                    entry_path.display()
                )
            })?;
            let target = current_destination.join(entry.file_name());
            if entry_type.is_dir() {
                std::fs::create_dir(&target).map_err(|error| {
                    format!(
                        "Cannot create collected media directory '{}': {error}",
                        target.display()
                    )
                })?;
                pending.push((entry_path, target));
            } else if entry_type.is_file() {
                std::fs::copy(&entry_path, &target).map_err(|error| {
                    format!(
                        "Cannot collect media file '{}': {error}",
                        entry_path.display()
                    )
                })?;
                file_count = file_count.saturating_add(1);
            } else {
                return Err(format!(
                    "Unsupported item in media directory: {}",
                    entry_path.display()
                ));
            }
        }
    }
    Ok(file_count)
}

fn rewrite_composition_paths(
    composition: &mut Composition,
    paths: &HashMap<String, String>,
) -> Result<(), String> {
    for layer in &mut composition.layers {
        let rewrite = |path: &mut String| -> Result<(), String> {
            *path = paths
                .get(path)
                .cloned()
                .ok_or_else(|| format!("Collected media path is missing for '{path}'"))?;
            Ok(())
        };
        match &mut layer.layer_type {
            LayerType::Image { path }
            | LayerType::Model3D { path }
            | LayerType::Audio { path, .. } => rewrite(path)?,
            LayerType::Video {
                source,
                frames_dir,
                audio_wav,
                ..
            } => {
                let is_image_sequence =
                    crate::core::video_import::is_image_sequence_cache(frames_dir, source);
                rewrite(source)?;
                if is_image_sequence {
                    rewrite(frames_dir)?;
                } else {
                    *frames_dir = "cache_pending".to_string();
                    *audio_wav = None;
                }
            }
            _ => {}
        }
    }
    for child in &mut composition.sub_compositions {
        rewrite_composition_paths(child, paths)?;
    }
    Ok(())
}
