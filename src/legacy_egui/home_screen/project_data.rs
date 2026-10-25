use crate::KagariApp;
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

#[derive(Clone, Debug, Default)]
pub struct ProjectSummary {
    pub comp_names: Vec<String>,
    pub active_idx: usize,
    pub layers: usize,
    pub preview_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub total_frames: u32,
    pub fps: u32,
}

pub(super) fn read_project_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = crate::core::project_migration::read_bounded_text_file(path, 8_000_000).ok()?;
    serde_json::from_str(&text).ok()
}

/// Project root may be a ProductionDocument, a versioned project file, or a raw project.
pub(super) fn project_root(v: &serde_json::Value) -> &serde_json::Value {
    v.get("project")
        .or_else(|| v.get("project_data"))
        .unwrap_or(v)
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
    let active_comp = comps.get(active_idx);
    let width = active_comp
        .and_then(|c| c.get("width"))
        .and_then(|w| w.as_u64())
        .unwrap_or(1920) as u32;
    let height = active_comp
        .and_then(|c| c.get("height"))
        .and_then(|h| h.as_u64())
        .unwrap_or(1080) as u32;
    let total_frames = active_comp
        .and_then(|c| c.get("total_frames"))
        .and_then(|f| f.as_u64())
        .unwrap_or(300) as u32;
    let fps = active_comp
        .and_then(|c| c.get("fps"))
        .and_then(|f| f.as_u64())
        .unwrap_or(30) as u32;

    let preview_path = active_comp
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
        width,
        height,
        total_frames,
        fps,
    })
}

#[derive(Clone, Debug)]
pub struct RecentProject {
    pub path: std::path::PathBuf,
    pub name: String,
    pub modified: Option<std::time::SystemTime>,
    pub summary: Option<ProjectSummary>,
}

/// Query recent project entries. Uses app cached list if available.
pub(crate) fn recent_project_entries(app: &KagariApp, _ctx: &egui::Context) -> Vec<RecentProject> {
    // Recent project discovery can touch cloud-backed files. Keep the frame
    // path read-only and let the startup worker populate this cache instead.
    app.home_recent_projects.clone()
}

/// Synchronous scan for background worker thread or fallback.
pub fn scan_recent_projects_sync(paths: &[String], sort_by_modified: bool) -> Vec<RecentProject> {
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
            read_project_json(&path).and_then(|v| summarize_value(&v))
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
        out.sort_by(|a, b| match (&a.modified, &b.modified) {
            (Some(x), Some(y)) => y.cmp(x),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }
    out
}

/// Spawns an asynchronous background thread to read and summarize recent projects,
/// avoiding UI thread blocking during frame rendering.
pub fn trigger_recent_projects_scan(app: &mut KagariApp, ctx: &egui::Context) {
    let paths = crate::ui::project_io::recent_projects();
    // Seed the UI immediately with the persisted order. The full metadata and
    // project summary scan can be slow for cloud-backed or missing paths; a
    // slow worker must not leave Home and Projects looking empty meanwhile.
    app.home_recent_projects = paths
        .iter()
        .map(|path_str| {
            let path = crate::ui::project_io::path_from_key(path_str);
            let name = path
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            RecentProject {
                path,
                name,
                modified: None,
                summary: None,
            }
        })
        .collect();
    let (tx, rx) = std::sync::mpsc::channel();
    app.home_recent_rx = Some(rx);
    let repaint_ctx = ctx.clone();
    std::thread::spawn(move || {
        // `paths` is already ordered by last opened time (newest first).
        // File modification time is not a reliable proxy for recent usage.
        let results = scan_recent_projects_sync(&paths, false);
        let _ = tx.send(results);
        repaint_ctx.request_repaint();
    });
}

/// Polls for completed background recent projects scan.
pub fn poll_recent_projects(app: &mut KagariApp, ctx: &egui::Context) {
    if let Some(rx) = app.home_recent_rx.take() {
        match rx.try_recv() {
            Ok(results) => {
                app.home_recent_projects = results;
                ctx.request_repaint();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                app.home_recent_rx = Some(rx);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }
}
