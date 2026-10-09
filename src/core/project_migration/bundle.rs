use crate::core::timeline::{Composition, LayerType, ProjectItemType};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

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

fn model_companion_references(path: &Path) -> Result<Vec<String>, String> {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "obj" && extension != "mtl" {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "Cannot inspect model companion file '{}': {error}",
            path.display()
        )
    })?;
    let mut references = Vec::new();
    for line in text.lines() {
        let content = line.split('#').next().unwrap_or_default().trim();
        let mut parts = content.split_whitespace();
        let Some(tag) = parts.next() else { continue };
        if extension == "obj" && tag == "mtllib" {
            let rest = content[tag.len()..].trim();
            if let Some(quoted) = rest
                .strip_prefix('"')
                .and_then(|value| value.split_once('"'))
            {
                references.push(quoted.0.to_string());
            } else {
                references.extend(rest.split_whitespace().map(str::to_string));
            }
        } else if extension == "mtl"
            && (tag.to_ascii_lowercase().starts_with("map_")
                || matches!(
                    tag.to_ascii_lowercase().as_str(),
                    "bump" | "norm" | "disp" | "decal" | "refl"
                ))
        {
            let rest = content[tag.len()..].trim();
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    references.push(rest[start + 1..start + 1 + end].to_string());
                }
            } else if let Some(filename) = rest.split_whitespace().last() {
                references.push(filename.to_string());
            }
        }
    }
    Ok(references)
}

fn rewrite_model_companions(
    source: &Path,
    destination: &Path,
    canonical_names: &HashMap<PathBuf, String>,
) -> Result<(), String> {
    let extension = source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "obj" && extension != "mtl" {
        return Ok(());
    }
    let source_text = std::fs::read_to_string(source).map_err(|error| {
        format!("Cannot rewrite model file '{}': {error}", source.display())
    })?;
    let mut rewritten = String::with_capacity(source_text.len());
    for line in source_text.lines() {
        let content = line.split('#').next().unwrap_or_default();
        let trimmed = content.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let mut parts = trimmed.split_whitespace();
        let Some(tag) = parts.next() else {
            rewritten.push_str(line);
            rewritten.push('\n');
            continue;
        };
        if extension == "obj" && tag == "mtllib" {
            let rest = trimmed[tag.len()..].trim();
            let references = if let Some(start) = rest.find('"') {
                rest[start + 1..]
                    .find('"')
                    .map(|end| vec![rest[start + 1..start + 1 + end].to_string()])
                    .unwrap_or_default()
            } else {
                rest.split_whitespace().map(str::to_string).collect()
            };
            let mut names = Vec::with_capacity(references.len());
            for reference in references {
                let canonical = std::fs::canonicalize(
                    source
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&reference),
                )
                .map_err(|error| {
                    format!("Cannot resolve OBJ material '{reference}': {error}")
                })?;
                names.push(
                    canonical_names
                        .get(&canonical)
                        .ok_or_else(|| {
                            format!("Collected OBJ material is missing for '{reference}'")
                        })?
                        .clone(),
                );
            }
            rewritten.push_str(indent);
            rewritten.push_str("mtllib");
            for name in names {
                rewritten.push(' ');
                rewritten.push_str(&name);
            }
            rewritten.push('\n');
            continue;
        }
        let is_mtl_map = extension == "mtl"
            && (tag.to_ascii_lowercase().starts_with("map_")
                || matches!(
                    tag.to_ascii_lowercase().as_str(),
                    "bump" | "norm" | "disp" | "decal" | "refl"
                ));
        if is_mtl_map {
            let rest_start = indent.len() + tag.len();
            let rest = line[rest_start..].trim_start();
            let quote_start = rest.find('"');
            let (reference, line_start, line_end) = if let Some(start) = quote_start {
                if let Some(end) = rest[start + 1..].find('"') {
                    (
                        rest[start + 1..start + 1 + end].to_string(),
                        rest_start + (line[rest_start..].len() - rest.len()) + start + 1,
                        rest_start + (line[rest_start..].len() - rest.len()) + start + 1 + end,
                    )
                } else {
                    rewritten.push_str(line);
                    rewritten.push('\n');
                    continue;
                }
            } else if let Some(filename) = rest.split_whitespace().last() {
                let start = content
                    .rfind(filename)
                    .unwrap_or(content.len() - filename.len());
                (filename.to_string(), start, start + filename.len())
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
                continue;
            };
            let canonical = std::fs::canonicalize(
                source
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&reference),
            )
            .map_err(|error| format!("Cannot resolve MTL texture '{reference}': {error}"))?;
            let name = canonical_names
                .get(&canonical)
                .ok_or_else(|| format!("Collected MTL texture is missing for '{reference}'"))?;
            rewritten.push_str(&line[..line_start]);
            rewritten.push_str(name);
            rewritten.push_str(&line[line_end..]);
            rewritten.push('\n');
        } else {
            rewritten.push_str(line);
            rewritten.push('\n');
        }
    }
    std::fs::write(destination, rewritten).map_err(|error| {
        format!(
            "Cannot save collected model file '{}': {error}",
            destination.display()
        )
    })
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
                rewrite(source)?;
                *frames_dir = "cache_pending".to_string();
                *audio_wav = None;
            }
            _ => {}
        }
    }
    for child in &mut composition.sub_compositions {
        rewrite_composition_paths(child, paths)?;
    }
    Ok(())
}
