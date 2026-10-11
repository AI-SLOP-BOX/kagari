use eframe::egui;

pub(super) fn fmt_age(modified: std::time::SystemTime) -> String {
    let secs = modified.elapsed().map(|d| d.as_secs()).unwrap_or(0);
    if crate::ui::localization::current_language() == crate::ui::localization::UiLanguage::Japanese
    {
        return if secs < 60 {
            "たった今".to_string()
        } else if secs < 3600 {
            format!("{}分前", secs / 60)
        } else if secs < 86400 {
            format!("{}時間前", secs / 3600)
        } else if secs < 86400 * 30 {
            format!("{}日前", secs / 86400)
        } else {
            format!("{}か月前", secs / (86400 * 30))
        };
    }
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} min ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else if secs < 86400 * 30 {
        format!("{} days ago", secs / 86400)
    } else {
        format!("{} months ago", secs / (86400 * 30))
    }
}

pub(super) fn is_image_ext(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "exr"
    )
}

#[derive(Clone)]
pub(super) struct ProjectSummary {
    pub(super) comp_names: Vec<String>,
    pub(super) active_idx: usize,
    pub(super) layers: usize,
    pub(super) preview_path: Option<String>,
}

pub(super) fn read_project_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = crate::core::project_migration::read_bounded_text_file(path, 8_000_000).ok()?;
    serde_json::from_str(&text).ok()
}

/// Project root may be a ProductionDocument ({project: {...}}) or a raw project.
pub(super) fn project_root(v: &serde_json::Value) -> &serde_json::Value {
    v.get("project").unwrap_or(v)
}

pub(super) fn summarize_value(v: &serde_json::Value) -> Option<ProjectSummary> {
    let root = project_root(v);
    let comps = root.get("compositions")?.as_array()?;
    let names: Vec<String> = comps
        .iter()
        .filter_map(|c| c.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    let layers: usize = comps
        .iter()
        .filter_map(|c| c.get("layers")?.as_array().map(|l| l.len()))
        .sum();
    let active_idx = root
        .get("active_composition_idx")
        .and_then(|i| i.as_u64())
        .unwrap_or(0) as usize;
    let preview_path = comps
        .get(active_idx)
        .and_then(|comp| comp.get("layers"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|layer| layer.get("visible").and_then(serde_json::Value::as_bool) != Some(false))
        .find_map(|layer| {
            let in_frame = layer
                .get("in_frame")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32;
            let layer_type = layer.get("layer_type")?.as_object()?;
            if let Some(image) = layer_type.get("Image") {
                let path = image.get("path")?.as_str()?;
                return is_image_ext(std::path::Path::new(path)).then(|| path.to_owned());
            }
            let video = layer_type.get("Video")?;
            let frames_dir = video.get("frames_dir")?.as_str()?;
            let source_frame = ((in_frame as f64
                * video
                    .get("speed")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0)
                    .max(0.0))
            .floor() as u32)
                .min(
                    video
                        .get("frame_count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1)
                        .saturating_sub(1) as u32,
                );
            let frame_path = crate::core::video_import::frame_path_in_dir(frames_dir, source_frame);
            frame_path
                .is_file()
                .then(|| frame_path.to_string_lossy().into_owned())
        });
    Some(ProjectSummary {
        comp_names: names,
        active_idx,
        layers,
        preview_path,
    })
}

#[derive(Clone)]
pub(super) struct RecentProject {
    pub(super) path: std::path::PathBuf,
    pub(super) name: String,
    pub(super) modified: Option<std::time::SystemTime>,
    pub(super) summary: Option<ProjectSummary>,
}

/// Cached per (path, mtime) so the JSON parse does not repeat every frame.
pub(super) fn recent_project_entries(ctx: &egui::Context) -> Vec<RecentProject> {
    project_entries_for_paths(ctx, &crate::ui::project_io::recent_projects(), true)
}

pub(super) fn project_entries_for_paths(
    ctx: &egui::Context,
    paths: &[String],
    sort_by_modified: bool,
) -> Vec<RecentProject> {
    let cache_id = egui::Id::new("home_proj_cache");
    type ProjCache = std::collections::HashMap<
        std::path::PathBuf,
        (Option<std::time::SystemTime>, Option<ProjectSummary>),
    >;
    let mut cache: ProjCache =
        ctx.data_mut(|d| d.get_temp_mut_or_default::<ProjCache>(cache_id).clone());
    let mut out = Vec::new();
    for path_str in paths {
        let path = crate::ui::project_io::path_from_key(path_str);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        let summary = if path.is_file() {
            match cache.get(&path) {
                Some((cached_mtime, cached)) if cached_mtime == &modified => cached.clone(),
                _ => {
                    let parsed = read_project_json(&path).and_then(|v| summarize_value(&v));
                    cache.insert(path.clone(), (modified, parsed.clone()));
                    parsed
                }
            }
        } else {
            None
        };
        out.push(RecentProject {
            path,
            name,
            modified,
            summary,
        });
    }
    if sort_by_modified {
        // Existing files first (newest first), missing files trail.
        out.sort_by(|a, b| match (&a.modified, &b.modified) {
            (Some(x), Some(y)) => y.cmp(x),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }
    ctx.data_mut(|d| d.insert_temp(cache_id, cache));
    out
}
