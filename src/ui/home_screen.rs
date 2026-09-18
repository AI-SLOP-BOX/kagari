//! Home screen: a resume-first production workspace.
//!
//! Layout: slim nav sidebar | resume + recents workspace | inspector-style
//! right panel, with a subtle background-task status strip at the bottom.
//! The filesystem browser and marketing hero are intentionally gone — this
//! screen answers "continue exactly where you left off" within seconds.
#![allow(dead_code)]
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

fn fmt_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} {}", bytes, UNITS[u])
    } else {
        format!("{:.1} {}", v, UNITS[u])
    }
}

fn fmt_age(modified: std::time::SystemTime) -> String {
    let secs = modified.elapsed().map(|d| d.as_secs()).unwrap_or(0);
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

fn disk_usage(path: &std::path::Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let total = st.f_blocks as u64 * st.f_frsize as u64;
    let free = st.f_bavail as u64 * st.f_frsize as u64;
    Some((total.saturating_sub(free), total))
}

/// One-line storage indicator for the sidebar footer / status strip.
fn storage_free_text() -> Option<String> {
    let (_, total) = disk_usage(std::path::Path::new("/"))?;
    let (used, _) = disk_usage(std::path::Path::new("/")).unwrap_or((0, total));
    Some(format!("{} free", fmt_bytes(total.saturating_sub(used))))
}

fn is_image_ext(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "exr"
    )
}

fn is_media_ext(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "bmp"
            | "tif"
            | "tiff"
            | "exr"
            | "mov"
            | "mp4"
            | "m4v"
            | "mkv"
            | "webm"
            | "av1"
            | "wav"
            | "mp3"
            | "aif"
            | "aiff"
    )
}

fn media_kind(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "mov" | "mp4" | "m4v" | "mkv" | "webm" | "av1" => "video",
        "wav" | "mp3" | "aif" | "aiff" => "audio",
        _ => "image",
    }
}

fn load_texture(
    ctx: &egui::Context,
    name: &str,
    img: image::DynamicImage,
    max_w: u32,
    max_h: u32,
) -> Option<egui::TextureHandle> {
    let thumb = img.thumbnail(max_w, max_h);
    let rgba = thumb.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

pub(crate) fn load_logo_texture(ctx: &egui::Context, img: image::DynamicImage) -> Option<egui::TextureHandle> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 { return None; }
    let mut pixels = rgba.into_raw();
    for pixel in pixels.chunks_exact_mut(4) {
        if pixel[0].max(pixel[1]).max(pixel[2]) < 18 { pixel[3] = 0; }
    }
    Some(ctx.load_texture("home:kagari-logo", egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels), egui::TextureOptions::LINEAR))
}

fn thumb_for(
    app: &mut KagariApp,
    ctx: &egui::Context,
    path: &std::path::Path,
) -> Option<egui::TextureId> {
    if let Some(h) = app.home_thumbs.get(path) {
        return Some(h.id());
    }
    if app.home_thumbs.len() > 96 {
        app.home_thumbs.clear();
    }
    let img = image::open(path).ok()?;
    let name = format!("thumb:{}", path.display());
    // Reference artwork is already small and is displayed at roughly its native
    // width; keeping the source detail avoids blur when cards grow responsively.
    let handle = load_texture(ctx, &name, img, 512, 256)?;
    let id = handle.id();
    app.home_thumbs.insert(path.to_path_buf(), handle);
    Some(id)
}

// ── Data model ────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HomeNav {
    Home,
    Compositing,
    NewProject,
    Projects,
    Recent,
    Starred,
    Templates,
    Effects,
    Render,
    Settings,
    Tutorial,
    Documentation,
}

fn home_nav(ctx: &egui::Context) -> HomeNav {
    ctx.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(egui::Id::new("home_nav"), || HomeNav::Home)
    })
}

fn set_home_nav(ctx: &egui::Context, nav: HomeNav) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("home_nav"), nav);
        d.insert_temp(egui::Id::new("home_search_reset"), true);
    });
}

/// Select one of the reference-facing pages before drawing a UI snapshot.
pub fn set_reference_page(ctx: &egui::Context, page: &str) {
    let nav = match page {
        "assets" | "projects" => HomeNav::Projects,
        "templates" => HomeNav::Templates,
        "effects" => HomeNav::Effects,
        "render" => HomeNav::Render,
        "settings" => HomeNav::Settings,
        "new-project" => HomeNav::NewProject,
        "tutorial" => HomeNav::Tutorial,
        "documentation" | "docs" => HomeNav::Documentation,
        _ => HomeNav::Home,
    };
    set_home_nav(ctx, nav);
}

#[derive(Clone)]
struct ProjectSummary {
    comp_names: Vec<String>,
    active_idx: usize,
    layers: usize,
}

fn read_project_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    if text.len() > 8_000_000 {
        return None;
    }
    serde_json::from_str(&text).ok()
}

/// Project root may be a ProductionDocument ({project: {...}}) or a raw project.
fn project_root(v: &serde_json::Value) -> &serde_json::Value {
    v.get("project").unwrap_or(v)
}

fn summarize_value(v: &serde_json::Value) -> Option<ProjectSummary> {
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
    Some(ProjectSummary {
        comp_names: names,
        active_idx,
        layers,
    })
}

fn summarize_project(path: &std::path::Path) -> Option<(usize, usize)> {
    read_project_json(path).and_then(|v| {
        summarize_value(&v).map(|s| (s.comp_names.len(), s.layers))
    })
}

#[derive(Clone)]
struct RecentProject {
    path: std::path::PathBuf,
    name: String,
    modified: Option<std::time::SystemTime>,
    summary: Option<ProjectSummary>,
}

/// Cached per (path, mtime) so the JSON parse does not repeat every frame.
fn recent_project_entries(ctx: &egui::Context) -> Vec<RecentProject> {
    let recents = crate::ui::project_io::recent_projects();
    let cache_id = egui::Id::new("home_proj_cache");
    type ProjCache = std::collections::HashMap<
        std::path::PathBuf,
        (Option<std::time::SystemTime>, Option<ProjectSummary>),
    >;
    let mut cache: ProjCache =
        ctx.data_mut(|d| d.get_temp_mut_or_default::<ProjCache>(cache_id).clone());
    let mut out = Vec::new();
    for path_str in recents.iter().take(8) {
        let path = std::path::PathBuf::from(path_str);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        let modified = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
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
    // Existing files first (newest first), missing files trail.
    out.sort_by(|a, b| match (&a.modified, &b.modified) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    ctx.data_mut(|d| d.insert_temp(cache_id, cache));
    out
}

#[derive(Clone)]
struct AssetEntry {
    path: std::path::PathBuf,
    name: String,
    kind: &'static str,
    modified: Option<std::time::SystemTime>,
    size: u64,
    is_sequence: bool,
    frames: usize,
}

/// Strip trailing frame digits (and separators) so `shot_010.0004.exr`
/// groups with its sequence siblings.
fn sequence_prefix(stem: &str) -> String {
    let no_digits = stem.trim_end_matches(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
    if no_digits.is_empty() {
        return String::new();
    }
    let has_sequence_separator = no_digits
        .chars()
        .last()
        .map(|ch| matches!(ch, '_' | '-' | '.' | ' '))
        .unwrap_or(false);
    if has_sequence_separator {
        no_digits
            .trim_end_matches(['_', '-', '.', ' '])
            .to_lowercase()
    } else {
        stem.to_lowercase()
    }
}

fn group_key(path: &std::path::Path) -> Option<(String, String, String)> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let ext = path
        .extension()?
        .to_string_lossy()
        .to_lowercase();
    let parent = path
        .parent()?
        .to_string_lossy()
        .to_string();
    Some((parent, format!("{}:{}", sequence_prefix(&stem), ext), ext))
}

/// Media files living next to recent projects, newest first. Refreshed at
/// most every 2 s so directory scans never run per frame.
fn recent_asset_entries(ctx: &egui::Context) -> Vec<AssetEntry> {
    let cache_id = egui::Id::new("home_assets_cache");
    let now = std::time::Instant::now();
    if let Some((at, items)) =
        ctx.data(|d| d.get_temp::<(std::time::Instant, Vec<AssetEntry>)>(cache_id))
    {
        if now.duration_since(at).as_secs() < 2 {
            return items;
        }
    }
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    for path_str in crate::ui::project_io::recent_projects().iter().take(6) {
        let p = std::path::PathBuf::from(path_str);
        if let Some(parent) = p.parent() {
            if parent.is_dir() && !dirs.contains(&parent.to_path_buf()) {
                dirs.push(parent.to_path_buf());
            }
        }
        if dirs.len() >= 4 {
            break;
        }
    }
    // (group_key, path, modified, size) — keep newest per group.
    let mut groups: std::collections::HashMap<
        String,
        (std::path::PathBuf, Option<std::time::SystemTime>, u64, usize),
    > = std::collections::HashMap::new();
    for dir in &dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in rd.flatten().take(400) {
            let path = entry.path();
            if !is_media_ext(&path) {
                continue;
            }
            if path
                .file_name()
                .map(|s| s.to_string_lossy().starts_with('.'))
                .unwrap_or(true)
            {
                continue;
            }
            let Some((_, key, _)) = group_key(&path) else {
                continue;
            };
            let meta = entry.metadata().ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok());
            let size = meta.map(|m| m.len()).unwrap_or(0);
            groups
                .entry(key)
                .and_modify(|(p, m, s, n)| {
                    *n += 1;
                    let newer = match (&m, &modified) {
                        (Some(a), Some(b)) => b > a,
                        (None, Some(_)) => true,
                        _ => false,
                    };
                    if newer {
                        *p = path.clone();
                        *m = modified;
                        *s = size;
                    }
                })
                .or_insert((path, modified, size, 1));
        }
    }
    let mut items: Vec<AssetEntry> = groups
        .into_values()
        .map(|(path, modified, size, frames)| {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            AssetEntry {
                kind: media_kind(&path),
                name,
                path,
                modified,
                size,
                is_sequence: frames > 1,
                frames,
            }
        })
        .collect();
    items.sort_by(|a, b| match (&a.modified, &b.modified) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    items.truncate(12);
    ctx.data_mut(|d| d.insert_temp(cache_id, (now, items.clone())));
    items
}

// ── Studio entry points ───────────────────────────────────────

fn enter_studio_new_project(app: &mut KagariApp) {
    app.history =
        crate::core::history::ProjectHistory::new(crate::core::timeline::Project::default());
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    crate::core::frame_cache::bump_version();
    app.ui_tabs.left_tab_idx = 0;
    app.ui_tabs.right_tab_idx = 30;
    app.show_home = false;
}

fn enter_studio_open_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari VFX Project", &["json"])
        .pick_file()
    {
        open_project_path(app, &path);
    }
}

fn open_project_path(app: &mut KagariApp, path: &std::path::Path) {
    if let Err(e) = crate::ui::project_io::open_project_from_path(app, path) {
        app.toasts.error(e);
    } else {
        app.ui_tabs.left_tab_idx = 0;
        app.ui_tabs.right_tab_idx = 30;
        app.show_home = false;
    }
}

fn new_composition(app: &mut KagariApp) {
    enter_studio_new_project(app);
    app.show_new_comp_dialog = true;
}

pub(crate) fn import_footage_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Footage",
            &[
                "png", "jpg", "jpeg", "webp", "bmp", "exr", "mov", "mp4", "wav", "mp3",
            ],
        )
        .pick_file()
    {
        open_selected_in_studio(app, &path);
    }
}

fn open_selected_in_studio(app: &mut KagariApp, path: &std::path::Path) {
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "json" {
        open_project_path(app, path);
        return;
    }
    // Media files: enter the studio and import (same path as drag & drop).
    enter_studio_new_project(app);
    let path_str = path.to_string_lossy().to_string();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".to_string());
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("img_{}", name),
                    name.clone(),
                    crate::core::timeline::LayerType::Image { path: path_str.clone() },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported image: {}", name));
        }
        "wav" | "mp3" => {
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let layer = crate::core::timeline::Layer::new(
                    format!("aud_{}", name),
                    format!("🔊 {}", name),
                    crate::core::timeline::LayerType::Audio {
                        path: path_str.clone(),
                        volume: crate::core::property::Animatable::new_constant(1.0),
                    },
                    comp.duration_frames,
                );
                comp.layers.push(layer);
            });
            app.toasts.info(format!("Imported audio: {}", name));
        }
        _ => {
            app.toasts.info("Entered studio — use File > Import for video");
        }
    }
}

// ── Layout ────────────────────────────────────────────────────

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let search_reset_id = egui::Id::new("home_search_reset");
    let should_reset_search = ctx.data_mut(|d| d.get_temp::<bool>(search_reset_id).unwrap_or(false));
    if should_reset_search {
        ctx.data_mut(|d| d.remove::<bool>(search_reset_id));
        app.home_search.clear();
    }
    if matches!(home_nav(ctx), HomeNav::Settings) {
        if ctx.screen_rect().width() >= 1200.0 {
            draw_settings_target(app, ctx);
        } else {
            draw_settings_target_responsive(app, ctx);
        }
        return;
    }
    if matches!(home_nav(ctx), HomeNav::Render) {
        if ctx.screen_rect().width() < 500.0 {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 19, 26)))
                .show(ctx, |ui| draw_reference_render_page(app, ui, ctx));
        } else {
            draw_export_target(app, ctx);
        }
        return;
    }
    if matches!(home_nav(ctx), HomeNav::Tutorial) {
        app.show_home = false;
        app.show_guided_tutorial = true;
        app.tutorial_step = 0;
        return;
    }
    let landing_mode = matches!(home_nav(ctx), HomeNav::Home)
        && ctx.screen_rect().width() >= 900.0;
    let sidebar_width = if landing_mode {
        293.0
    } else if ctx.screen_rect().width() < 1100.0 {
        120.0
    } else {
        136.0
    };
    let show_home_sidebar = ctx.screen_rect().width() >= 500.0;
    if landing_mode {
        egui::TopBottomPanel::top("landing_window_bar")
            .exact_height(46.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(24, 31, 37))
                    .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE)),
            )
            .show(ctx, |ui| {
                for (x, color) in [
                    (23.0, egui::Color32::from_rgb(255, 82, 78)),
                    (46.0, egui::Color32::from_rgb(255, 190, 45)),
                    (69.0, egui::Color32::from_rgb(42, 211, 86)),
                ] {
                    ui.painter().circle_filled(egui::pos2(x, 23.0), 7.0, color);
                }
                ui.painter().text(
                    egui::pos2(96.0, 23.0),
                    egui::Align2::LEFT_CENTER,
                    "Kagari VFX",
                    egui::FontId::proportional(14.0),
                    colors::TEXT_PRIMARY,
                );
            });
    }
    if show_home_sidebar && matches!(home_nav(ctx), HomeNav::Home | HomeNav::Compositing) {
        egui::SidePanel::left("home_nav")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(18, 28, 38)))
            .show(ctx, |ui| {
                if landing_mode {
                    draw_landing_sidebar(app, ui, ctx);
                } else {
                    draw_reference_nav(app, ui, ctx);
                }
            });
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 19, 26)))
        .show(ctx, |ui| {
        sync_selected_preview(app, ctx);
        match home_nav(ctx) {
            HomeNav::Home if landing_mode => draw_landing_home(app, ui, ctx),
            HomeNav::Home | HomeNav::Compositing => draw_reference_home(app, ui, ctx),
            HomeNav::NewProject => draw_reference_new_project_page(app, ui, ctx),
            HomeNav::Projects | HomeNav::Recent | HomeNav::Starred => draw_reference_project_browser_page(app, ui, ctx),
            HomeNav::Templates => draw_reference_templates_page(app, ui, ctx),
            HomeNav::Effects => draw_reference_effects_page(app, ui, ctx),
            HomeNav::Render => draw_reference_render_page(app, ui, ctx),
            HomeNav::Settings => draw_reference_settings_page(app, ui, ctx),
            HomeNav::Documentation => draw_reference_documentation_page(app, ui, ctx),
            HomeNav::Tutorial => draw_reference_home(app, ui, ctx),
        }
        });

    // Export progress needs continuous repaint; idle home stays static.
    if app.export.is_exporting {
        ctx.request_repaint();
    }
}

fn draw_settings_target(app: &mut KagariApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("settings_target_window_bar")
        .exact_height(43.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 31, 37)))
        .show(ctx, |ui| {
            for (x, color) in [(23.0, egui::Color32::from_rgb(255, 82, 78)), (46.0, egui::Color32::from_rgb(255, 190, 45)), (69.0, egui::Color32::from_rgb(42, 211, 86))] {
                ui.painter().circle_filled(egui::pos2(x, 22.0), 7.0, color);
            }
            ui.painter().text(egui::pos2(91.0, 22.0), egui::Align2::LEFT_CENTER, "Kagari VFX", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
            ui.painter().line_segment([egui::pos2(0.0, 42.0), egui::pos2(ui.max_rect().right(), 42.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(48, 59, 69)));
        });
    egui::CentralPanel::default().frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 19, 25))).show(ctx, |ui| {
        let r = ui.max_rect();
        let left = egui::Rect::from_min_max(r.min, egui::pos2(r.left() + 285.0, r.bottom()));
        let category = egui::Rect::from_min_max(egui::pos2(left.right(), r.top()), egui::pos2(left.right() + 236.0, r.bottom()));
        let content = egui::Rect::from_min_max(category.right_top(), r.right_bottom());
        ui.painter().rect_filled(left, 0.0, egui::Color32::from_rgb(15, 25, 32));
        ui.painter().rect_filled(category, 0.0, egui::Color32::from_rgb(12, 21, 27));
        ui.painter().line_segment([egui::pos2(left.right(), r.top()), egui::pos2(left.right(), r.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        ui.painter().line_segment([egui::pos2(category.right(), r.top()), egui::pos2(category.right(), r.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));

        if app.home_banner.is_none() {
            if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        if let Some(texture) = app.home_banner.as_ref() {
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(36.0, r.top() + 19.0), egui::vec2(78.0, 78.0))), |icon_ui| {
                icon_ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(78.0, 78.0))));
            });
        }
        ui.painter().text(egui::pos2(123.0, r.top() + 61.0), egui::Align2::LEFT_CENTER, "Kagari", egui::FontId::proportional(29.0), colors::TEXT_PRIMARY);
        ui.painter().text(egui::pos2(214.0, r.top() + 61.0), egui::Align2::LEFT_CENTER, "VFX", egui::FontId::proportional(29.0), egui::Color32::from_rgb(161, 174, 190));
        let nav = [(crate::ui::icons::SVG_HOME, "ホーム", 140.0), (crate::ui::icons::SVG_FOLDER, "プロジェクトを開く", 196.0), (crate::ui::icons::SVG_FILE_PLUS, "新規プロジェクト", 252.0), (crate::ui::icons::SVG_BOOK, "チュートリアル", 342.0), (crate::ui::icons::SVG_DOCUMENT, "ドキュメント", 398.0), (crate::ui::icons::SVG_SETTINGS, "設定", 498.0), (crate::ui::icons::SVG_POWER, "終了", 554.0)];
        for (icon, label, y) in nav {
            let active = label == "設定";
            if active {
                ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(17.0, r.top() + y - 5.0), egui::pos2(left.right(), r.top() + y + 47.0)), 0.0, egui::Color32::from_rgb(27, 35, 42));
                ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(17.0, r.top() + y - 5.0), egui::vec2(4.0, 52.0)), 0.0, egui::Color32::from_rgb(255, 111, 28));
            }
            crate::ui::icons::render_svg_at(ui, format!("settings-target-nav-{label}"), icon, egui::vec2(28.0, 28.0), if active { egui::Color32::from_rgb(255, 145, 50) } else { egui::Color32::from_rgb(193, 205, 218) }, egui::pos2(40.0, r.top() + y + 5.0));
            ui.painter().text(egui::pos2(87.0, r.top() + y + 19.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(16.0), if active { colors::TEXT_PRIMARY } else { egui::Color32::from_rgb(193, 205, 218) });
        }
        ui.painter().line_segment([egui::pos2(36.0, r.top() + 319.0), egui::pos2(254.0, r.top() + 319.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 62, 72)));
        ui.painter().text(egui::pos2(35.0, r.bottom() - 44.0), egui::Align2::LEFT_CENTER, "Kagari VFX   v0.1.0", egui::FontId::proportional(14.0), egui::Color32::from_rgb(157, 169, 183));

        ui.painter().text(egui::pos2(category.left() + 30.0, r.top() + 44.0), egui::Align2::LEFT_CENTER, "設定", egui::FontId::proportional(24.0), colors::TEXT_PRIMARY);
        let categories = [(crate::ui::icons::SVG_SETTINGS, "一般"), (crate::ui::icons::SVG_PALETTE, "外観"), (crate::ui::icons::SVG_GPU, "パフォーマンス"), (crate::ui::icons::SVG_KEYBOARD, "ショートカット"), (crate::ui::icons::SVG_FOLDER, "メディア"), (crate::ui::icons::SVG_AUDIO, "オーディオ"), (crate::ui::icons::SVG_LAYERS, "プラグイン"), (crate::ui::icons::SVG_FILE, "保存")];
        for (index, (icon, label)) in categories.into_iter().enumerate() {
            let y = r.top() + 84.0 + index as f32 * 53.0;
            if index == 0 { ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(category.left() + 17.0, y), egui::pos2(category.right(), y + 53.0)), 0.0, egui::Color32::from_rgb(27, 35, 42)); ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(category.left() + 17.0, y), egui::vec2(4.0, 53.0)), 0.0, egui::Color32::from_rgb(255, 111, 28)); }
            crate::ui::icons::render_svg_at(ui, format!("settings-target-category-{index}"), icon, egui::vec2(25.0, 25.0), if index == 0 { egui::Color32::from_rgb(255, 128, 30) } else { colors::TEXT_SECONDARY }, egui::pos2(category.left() + 37.0, y + 14.0));
            ui.painter().text(egui::pos2(category.left() + 82.0, y + 27.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(15.0), if index == 0 { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
        }
        draw_settings_target_content(ui, content);
    });
}

fn draw_settings_target_responsive(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let narrow = width < 1100.0;
    let left_width = if narrow { 68.0 } else { 200.0 };
    let category_width = if narrow { 68.0 } else { 176.0 };
    egui::TopBottomPanel::top("settings_target_window_bar_responsive")
        .exact_height(43.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 31, 37)))
        .show(ctx, |ui| {
            for (x, color) in [(23.0, egui::Color32::from_rgb(255, 82, 78)), (46.0, egui::Color32::from_rgb(255, 190, 45)), (69.0, egui::Color32::from_rgb(42, 211, 86))] {
                ui.painter().circle_filled(egui::pos2(x, 22.0), 7.0, color);
            }
            ui.painter().text(egui::pos2(91.0, 22.0), egui::Align2::LEFT_CENTER, "Kagari VFX", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        });
    egui::CentralPanel::default().frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 19, 25))).show(ctx, |ui| {
        let r = ui.max_rect();
        let left = egui::Rect::from_min_max(r.min, egui::pos2(r.left() + left_width, r.bottom()));
        let category = egui::Rect::from_min_max(egui::pos2(left.right(), r.top()), egui::pos2(left.right() + category_width, r.bottom()));
        let content = egui::Rect::from_min_max(category.right_top(), r.right_bottom());
        ui.painter().rect_filled(left, 0.0, egui::Color32::from_rgb(15, 25, 32));
        ui.painter().rect_filled(category, 0.0, egui::Color32::from_rgb(12, 21, 27));
        ui.painter().line_segment([egui::pos2(left.right(), r.top()), egui::pos2(left.right(), r.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        ui.painter().line_segment([egui::pos2(category.right(), r.top()), egui::pos2(category.right(), r.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        let logo_size = if narrow { 42.0 } else { 62.0 };
        if app.home_banner.is_none() {
            if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        if let Some(texture) = app.home_banner.as_ref() {
            let x = if narrow { (left_width - logo_size) * 0.5 } else { 28.0 };
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(x, r.top() + 18.0), egui::vec2(logo_size, logo_size))), |icon_ui| {
                icon_ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(logo_size, logo_size))));
            });
        }
        let nav = [(crate::ui::icons::SVG_HOME, "ホーム"), (crate::ui::icons::SVG_FOLDER, "プロジェクトを開く"), (crate::ui::icons::SVG_FILE_PLUS, "新規プロジェクト"), (crate::ui::icons::SVG_BOOK, "チュートリアル"), (crate::ui::icons::SVG_DOCUMENT, "ドキュメント"), (crate::ui::icons::SVG_SETTINGS, "設定"), (crate::ui::icons::SVG_POWER, "終了")];
        for (index, (icon, label)) in nav.into_iter().enumerate() {
            let y = r.top() + 112.0 + index as f32 * if narrow { 52.0 } else { 55.0 };
            let active = label == "設定";
            if active { ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(0.0, y - 6.0), egui::pos2(left.right(), y + 43.0)), 0.0, egui::Color32::from_rgb(27, 35, 42)); ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(0.0, y - 6.0), egui::vec2(4.0, 49.0)), 0.0, egui::Color32::from_rgb(255, 111, 28)); }
            let icon_x = if narrow { (left_width - 24.0) * 0.5 } else { 28.0 };
            crate::ui::icons::render_svg_at(ui, format!("settings-responsive-nav-{index}"), icon, egui::vec2(24.0, 24.0), if active { egui::Color32::from_rgb(255, 145, 50) } else { egui::Color32::from_rgb(193, 205, 218) }, egui::pos2(icon_x, y + 5.0));
            if !narrow { ui.painter().text(egui::pos2(72.0, y + 17.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(14.0), if active { colors::TEXT_PRIMARY } else { egui::Color32::from_rgb(193, 205, 218) }); }
        }
        if !narrow { ui.painter().text(egui::pos2(20.0, r.bottom() - 26.0), egui::Align2::LEFT_CENTER, "Kagari VFX  v0.1.0", egui::FontId::proportional(11.0), colors::TEXT_SECONDARY); }
        let categories = [(crate::ui::icons::SVG_SETTINGS, "一般"), (crate::ui::icons::SVG_PALETTE, "外観"), (crate::ui::icons::SVG_GPU, "パフォーマンス"), (crate::ui::icons::SVG_KEYBOARD, "ショートカット"), (crate::ui::icons::SVG_FOLDER, "メディア"), (crate::ui::icons::SVG_AUDIO, "オーディオ"), (crate::ui::icons::SVG_LAYERS, "プラグイン"), (crate::ui::icons::SVG_FILE, "保存")];
        if !narrow { ui.painter().text(egui::pos2(category.left() + 20.0, r.top() + 36.0), egui::Align2::LEFT_CENTER, "設定", egui::FontId::proportional(20.0), colors::TEXT_PRIMARY); }
        for (index, (icon, label)) in categories.into_iter().enumerate() {
            let y = r.top() + if narrow { 35.0 } else { 76.0 } + index as f32 * if narrow { 44.0 } else { 49.0 };
            if index == 0 { ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(category.left(), y), egui::pos2(category.right(), y + 46.0)), 0.0, egui::Color32::from_rgb(27, 35, 42)); ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(category.left(), y), egui::vec2(3.0, 46.0)), 0.0, egui::Color32::from_rgb(255, 111, 28)); }
            crate::ui::icons::render_svg_at(ui, format!("settings-responsive-category-{index}"), icon, egui::vec2(21.0, 21.0), if index == 0 { egui::Color32::from_rgb(255, 128, 30) } else { colors::TEXT_SECONDARY }, egui::pos2(if narrow { category.left() + (category_width - 21.0) * 0.5 } else { category.left() + 20.0 }, y + 12.0));
            if !narrow { ui.painter().text(egui::pos2(category.left() + 53.0, y + 23.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), if index == 0 { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }); }
        }
        if narrow {
            draw_settings_responsive_narrow_content(ui, content);
        } else {
            draw_settings_responsive_content(ui, content, false);
        }
    });
}

fn draw_settings_responsive_narrow_content(ui: &mut egui::Ui, content: egui::Rect) {
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("settings_responsive_narrow_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width().max(180.0));
                ui.add_space(12.0);
                ui.label(egui::RichText::new("一般").size(20.0).strong());
                ui.label(
                    egui::RichText::new("アプリケーションの基本設定")
                        .size(11.0)
                        .color(colors::TEXT_SECONDARY),
                );
                ui.add_space(10.0);
                let cards = [
                    (
                        "インターフェース",
                        [("テーマ", "ダーク"), ("言語", "日本語"), ("UIのスケール", "100%（推奨）")],
                    ),
                    (
                        "プロジェクトと保存",
                        [("自動保存の間隔", "5分"), ("保持数", "10個"), ("既定の保存先", "~/Kagari VFX/Projects")],
                    ),
                    (
                        "パフォーマンス",
                        [("GPUアクセラレーション", "オン"), ("使用するGPU", "自動選択（推奨）"), ("メモリ上限", "75%")],
                    ),
                    (
                        "メディアとタイムライン",
                        [("プロキシメディア", "自動生成"), ("プロキシ解像度", "1/2（推奨）"), ("フレームレート", "30 fps")],
                    ),
                    (
                        "書き出しの既定設定",
                        [("フォーマット", "H.264 (MP4)"), ("解像度", "1920 × 1080"), ("品質", "高品質")],
                    ),
                    (
                        "プラグイン",
                        [
                            ("プラグインの管理", "フォルダを開く"),
                            ("利用可能なプラグイン", "スキャン"),
                            ("状態", "有効"),
                        ],
                    ),
                ];
                for (title, rows) in cards {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(15, 26, 33))
                        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
                        .rounding(7.0)
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(title).size(14.0).strong());
                            ui.add_space(5.0);
                            for (label, value) in rows {
                                ui.horizontal(|ui| {
                                    let label_width = (ui.available_width() * 0.42).max(92.0);
                                    ui.add_sized(
                                        [label_width, 25.0],
                                        egui::Label::new(
                                            egui::RichText::new(label)
                                                .size(10.0)
                                                .color(colors::TEXT_SECONDARY),
                                        )
                                        .truncate(),
                                    );
                                    ui.add_sized(
                                        [ui.available_width(), 25.0],
                                        egui::Button::new(
                                            egui::RichText::new(value)
                                                .size(10.0)
                                                .color(colors::TEXT_PRIMARY),
                                        )
                                        .fill(egui::Color32::from_rgb(20, 32, 41))
                                        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
                                        .rounding(4.0),
                                    );
                                });
                                ui.add_space(4.0);
                            }
                        });
                    ui.add_space(8.0);
                }
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(15, 26, 33))
                    .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Kagari VFX v0.1.0")
                                .size(10.0)
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                ui.add_space(12.0);
            });
    });
}

fn draw_export_target(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let height = ctx.screen_rect().height();
    let narrow = width < 900.0;
    let short = height < 760.0;
    let left_width = if width >= 1200.0 { 264.0 } else if narrow { 68.0 } else { 190.0 };
    egui::TopBottomPanel::top("export_target_window_bar").exact_height(43.0).resizable(false).frame(egui::Frame::none().fill(egui::Color32::from_rgb(24,31,37))).show(ctx, |ui| {
        for (x, color) in [(23.0, egui::Color32::from_rgb(255,82,78)), (46.0, egui::Color32::from_rgb(255,190,45)), (69.0, egui::Color32::from_rgb(42,211,86))] { ui.painter().circle_filled(egui::pos2(x,22.0),7.0,color); }
        ui.painter().text(egui::pos2(91.0,22.0),egui::Align2::LEFT_CENTER,"Kagari VFX",egui::FontId::proportional(14.0),colors::TEXT_PRIMARY);
        ui.painter().text(egui::pos2(width-28.0,22.0),egui::Align2::RIGHT_CENTER,"Create. Composite. Illuminate.",egui::FontId::proportional(13.0),colors::TEXT_MUTED);
    });
    egui::CentralPanel::default().frame(egui::Frame::none().fill(egui::Color32::from_rgb(11,19,25))).show(ctx, |ui| {
        let r = ui.max_rect();
        let sidebar = egui::Rect::from_min_max(r.min,egui::pos2(r.left()+left_width,r.bottom()));
        ui.painter().rect_filled(sidebar,0.0,egui::Color32::from_rgb(15,25,32));
        ui.painter().line_segment([egui::pos2(sidebar.right(),r.top()),egui::pos2(sidebar.right(),r.bottom())],egui::Stroke::new(1.0_f32,colors::BORDER_SUBTLE));
        if app.home_banner.is_none() { if let Ok(img)=image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) { app.home_banner=load_logo_texture(ctx,img); } }
        let logo_size=if narrow {42.0}else if width>=1200.0 {76.0}else{58.0};
        if let Some(texture)=app.home_banner.as_ref() { let x=if narrow {(left_width-logo_size)*0.5}else{32.0}; ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(x,r.top()+17.0),egui::vec2(logo_size,logo_size))),|icon_ui|{icon_ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(),egui::vec2(logo_size,logo_size))));}); }
        if !narrow { ui.painter().text(egui::pos2(108.0,r.top()+59.0),egui::Align2::LEFT_CENTER,"Kagari",egui::FontId::proportional(if width>=1200.0 {27.0}else{20.0}),colors::TEXT_PRIMARY); ui.painter().text(egui::pos2(if width>=1200.0 {190.0}else{167.0},r.top()+59.0),egui::Align2::LEFT_CENTER,"VFX",egui::FontId::proportional(if width>=1200.0 {27.0}else{20.0}),egui::Color32::from_rgb(161,174,190)); }
        let nav=[(crate::ui::icons::SVG_HOME,"ホーム"),(crate::ui::icons::SVG_FOLDER,"プロジェクトを開く"),(crate::ui::icons::SVG_FILE_PLUS,"新規プロジェクト"),(crate::ui::icons::SVG_LAYERS,"編集"),(crate::ui::icons::SVG_LIGHT,"エフェクト"),(crate::ui::icons::SVG_FOLDER,"素材"),(crate::ui::icons::SVG_AUDIO,"オーディオ"),(crate::ui::icons::SVG_TOOL_TEXT,"テキスト"),(crate::ui::icons::SVG_ARROW_RIGHT,"トランジション"),(crate::ui::icons::SVG_PALETTE,"カラー"),(crate::ui::icons::SVG_EXPORT,"書き出し"),(crate::ui::icons::SVG_BOOK,"チュートリアル"),(crate::ui::icons::SVG_DOCUMENT,"ドキュメント"),(crate::ui::icons::SVG_SETTINGS,"設定"),(crate::ui::icons::SVG_POWER,"終了")];
        let start=r.top()+if width>=1200.0{113.0}else{104.0};
        for (index,(icon,label)) in nav.into_iter().enumerate() { let y=start+index as f32*if width>=1200.0{41.0}else{36.0}; let active=label=="書き出し"; if active { ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(10.0,y-5.0),egui::pos2(sidebar.right(),y+37.0)),0.0,egui::Color32::from_rgb(27,35,42)); ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(10.0,y-5.0),egui::vec2(4.0,42.0)),0.0,egui::Color32::from_rgb(255,111,28)); } let ix=if narrow{(left_width-22.0)*0.5}else{34.0}; crate::ui::icons::render_svg_at(ui,format!("export-nav-{index}"),icon,egui::vec2(22.0,22.0),if active{egui::Color32::from_rgb(255,145,50)}else{egui::Color32::from_rgb(193,205,218)},egui::pos2(ix,y+5.0)); if !narrow {ui.painter().text(egui::pos2(80.0,y+16.0),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(if width>=1200.0{14.0}else{11.0}),if active{colors::TEXT_PRIMARY}else{egui::Color32::from_rgb(193,205,218)});}}
        if !narrow && !short { ui.painter().text(egui::pos2(32.0,r.bottom()-27.0),egui::Align2::LEFT_CENTER,"Kagari VFX   v0.1.0",egui::FontId::proportional(13.0),colors::TEXT_SECONDARY); }
        let main=egui::Rect::from_min_max(egui::pos2(sidebar.right(),r.top()),r.right_bottom());
        let margin=if width>=1200.0{34.0}else{16.0}; let title_x=main.left()+margin;
        ui.painter().text(egui::pos2(title_x,r.top()+43.0),egui::Align2::LEFT_CENTER,"書き出し設定",egui::FontId::proportional(if width>=1200.0{32.0}else{23.0}),colors::TEXT_PRIMARY);
        ui.painter().text(egui::pos2(title_x,r.top()+79.0),egui::Align2::LEFT_CENTER,"あなたの作品を、世界へ。高品質な映像を書き出しましょう。",egui::FontId::proportional(if width>=1200.0{17.0}else{12.0}),colors::TEXT_SECONDARY);
        let preview_w=if width>=1200.0{703.0}else{((main.width()-margin*2.0-14.0)*0.5).max(280.0)};
        let settings_w=if width>=1200.0{main.width()-margin*2.0-preview_w-14.0}else{preview_w};
        let top=r.top()+118.0; let preview=egui::Rect::from_min_size(egui::pos2(title_x,top),egui::vec2(preview_w,if width>=1200.0{449.0}else{260.0})); let settings=egui::Rect::from_min_size(egui::pos2(if width>=1200.0{preview.right()+14.0}else{title_x+preview_w+14.0},top-34.0),egui::vec2(settings_w,if width>=1200.0{482.0}else{400.0}));
        let fill=egui::Color32::from_rgb(15,26,33); let stroke=egui::Stroke::new(1.0_f32,colors::BORDER_SUBTLE); ui.painter().rect(preview,8.0,fill,stroke); ui.painter().rect(settings,8.0,fill,stroke);
        let image_rect=egui::Rect::from_min_size(egui::pos2(preview.left()+18.0,preview.top()+15.0),egui::vec2(preview.width()-36.0,if width>=1200.0{352.0}else{155.0})); if let Some(id)=reference_texture(app,ctx,"continue_working_preview.webp"){ui.painter().image(id,image_rect,egui::Rect::from_min_max(egui::pos2(0.0,0.0),egui::pos2(1.0,1.0)),egui::Color32::WHITE);} ui.painter().rect_stroke(image_rect,0.0,stroke); ui.painter().text(egui::pos2(preview.left()+18.0,preview.bottom()-55.0),egui::Align2::LEFT_CENTER,"00:00:04:12",egui::FontId::proportional(16.0),egui::Color32::from_rgb(255,107,22)); ui.painter().line_segment([egui::pos2(preview.left()+135.0,preview.bottom()-56.0),egui::pos2(preview.right()-115.0,preview.bottom()-56.0)],egui::Stroke::new(3.0_f32,egui::Color32::from_rgb(255,107,22))); for (i,icon) in [crate::ui::icons::SVG_JUMP_BACK,crate::ui::icons::SVG_PLAY,crate::ui::icons::SVG_PAUSE,crate::ui::icons::SVG_JUMP_FORWARD].into_iter().enumerate(){ crate::ui::icons::render_svg_at(ui,format!("export-preview-transport-{i}"),icon,egui::vec2(16.0,16.0),colors::TEXT_PRIMARY,egui::pos2(preview.center().x-83.0+i as f32*44.0,preview.bottom()-35.0)); }
        ui.painter().text(egui::pos2(settings.left()+20.0,settings.top()+28.0),egui::Align2::LEFT_CENTER,"ビデオ        オーディオ        詳細設定",egui::FontId::proportional(if width>=1200.0{14.0}else{11.0}),colors::TEXT_PRIMARY); ui.painter().line_segment([egui::pos2(settings.left()+30.0,settings.top()+42.0),egui::pos2(settings.left()+127.0,settings.top()+42.0)],egui::Stroke::new(2.0_f32,egui::Color32::from_rgb(255,107,22)));
        let rows=[("形式","MP4 (H.264)"),("コーデック","H.264 (AVC)"),("解像度","1920 × 1080 (フルHD)"),("フレームレート","30 fps"),("ビットレート","高品質（20 Mbps）"),("保存先","/Users/username/Movies/Kagari VFX"),("ファイル名","Sample Project")]; let field_x=settings.left()+if width>=1200.0{214.0}else{settings.width()*0.35}; for (i,(lab,val)) in rows.into_iter().enumerate(){let yy=settings.top()+72.0+i as f32*38.0; ui.painter().text(egui::pos2(settings.left()+20.0,yy+11.0),egui::Align2::LEFT_CENTER,lab,egui::FontId::proportional(if width>=1200.0{14.0}else{10.0}),colors::TEXT_SECONDARY); ui.painter().rect(egui::Rect::from_min_size(egui::pos2(field_x,yy-5.0),egui::vec2(settings.right()-field_x-20.0,32.0)),5.0,egui::Color32::from_rgb(20,32,41),stroke); ui.painter().text(egui::pos2(field_x+12.0,yy+11.0),egui::Align2::LEFT_CENTER,val,egui::FontId::proportional(if width>=1200.0{13.0}else{9.0}),colors::TEXT_PRIMARY);}
        ui.painter().rect(egui::Rect::from_min_size(egui::pos2(settings.left()+20.0,settings.bottom()-60.0),egui::vec2(settings.width()-40.0,47.0)),6.0,egui::Color32::from_rgb(255,103,24),egui::Stroke::NONE); crate::ui::icons::render_svg_at(ui,"export-start-icon".to_string(),crate::ui::icons::SVG_EXPORT,egui::vec2(18.0,18.0),egui::Color32::WHITE,egui::pos2(settings.center().x-84.0,settings.bottom()-45.0)); ui.painter().text(egui::pos2(settings.center().x+10.0,settings.bottom()-36.0),egui::Align2::CENTER_CENTER,"書き出し開始",egui::FontId::proportional(if width>=1200.0{15.0}else{11.0}),egui::Color32::WHITE);
        let queue_y=if width>=1200.0{r.top()+582.0}else{settings.bottom()+14.0}; let queue=egui::Rect::from_min_size(egui::pos2(title_x,queue_y),egui::vec2(main.width()-margin*2.0,(height-queue_y-r.top()-34.0).max(150.0))); let queue_title_y=if short{22.0}else{31.0}; let queue_header_y=if short{45.0}else{75.0}; let queue_row_start=if short{70.0}else{113.0}; let queue_row_step=if short{29.0}else{47.0}; ui.painter().rect(queue,8.0,fill,stroke); ui.painter().text(egui::pos2(queue.left()+20.0,queue.top()+queue_title_y),egui::Align2::LEFT_CENTER,"レンダーキュー（3）",egui::FontId::proportional(if width>=1200.0{17.0}else{13.0}),colors::TEXT_PRIMARY); let cols=["#","プロジェクト名","形式","解像度","フレームレート","状態","進行状況","完了時の動作"]; for (i,c) in cols.into_iter().enumerate(){ui.painter().text(egui::pos2(queue.left()+20.0+i as f32*(queue.width()/8.0),queue.top()+queue_header_y),egui::Align2::LEFT_CENTER,c,egui::FontId::proportional(if width>=1200.0{11.0}else{8.0}),colors::TEXT_MUTED);} for (i,(name,status)) in [("Sample Project","レンダリング中"),("City Overview","待機中"),("Particles Shot","完了")].into_iter().enumerate(){let yy=queue.top()+queue_row_start+i as f32*queue_row_step; ui.painter().line_segment([egui::pos2(queue.left()+18.0,yy+22.0),egui::pos2(queue.right()-18.0,yy+22.0)],egui::Stroke::new(1.0_f32,colors::BORDER_SUBTLE)); ui.painter().text(egui::pos2(queue.left()+20.0,yy),egui::Align2::LEFT_CENTER,format!("{}",i+1),egui::FontId::proportional(11.0),colors::TEXT_SECONDARY); ui.painter().text(egui::pos2(queue.left()+queue.width()*0.10,yy),egui::Align2::LEFT_CENTER,name,egui::FontId::proportional(11.0),colors::TEXT_PRIMARY); ui.painter().text(egui::pos2(queue.left()+queue.width()*0.50,yy),egui::Align2::LEFT_CENTER,status,egui::FontId::proportional(11.0),if i==0{colors::ACCENT_CYAN}else if i==2{colors::ACCENT_GREEN}else{colors::TEXT_SECONDARY});}
    });
}

fn draw_settings_responsive_content(ui: &mut egui::Ui, content: egui::Rect, narrow: bool) {
    let p = ui.painter();
    let white = colors::TEXT_PRIMARY;
    let muted = colors::TEXT_SECONDARY;
    let border = colors::BORDER_SUBTLE;
    p.text(egui::pos2(content.left() + 22.0, content.top() + 35.0), egui::Align2::LEFT_CENTER, "一般", egui::FontId::proportional(if narrow { 19.0 } else { 22.0 }), white);
    p.text(egui::pos2(content.left() + 22.0, content.top() + 63.0), egui::Align2::LEFT_CENTER, "アプリケーションの基本設定", egui::FontId::proportional(12.0), muted);
    let gap = 10.0;
    let card_w = ((content.width() - 32.0 - gap) * 0.5).max(180.0);
    let card_h = if narrow { 154.0 } else { 170.0 };
    let x = [content.left() + 16.0, content.left() + 16.0 + card_w + gap];
    let titles = ["インターフェース", "プロジェクトと保存", "パフォーマンス", "メディアとタイムライン", "書き出しの既定設定", "プラグイン"];
    for (index, title) in titles.into_iter().enumerate() {
        let col = index % 2;
        let row = index / 2;
        let y = content.top() + 91.0 + row as f32 * (card_h + 10.0);
        let rect = egui::Rect::from_min_size(egui::pos2(x[col], y), egui::vec2(card_w, card_h));
        p.rect(rect, 7.0, egui::Color32::from_rgb(15, 26, 33), egui::Stroke::new(1.0_f32, border));
        p.text(egui::pos2(rect.left() + 16.0, rect.top() + 23.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(14.0), white);
        for row_index in 0..3 {
            let yy = rect.top() + 52.0 + row_index as f32 * 34.0;
            p.text(egui::pos2(rect.left() + 16.0, yy + 10.0), egui::Align2::LEFT_CENTER, ["テーマ", "言語", "UIのスケール"][row_index], egui::FontId::proportional(11.0), muted);
            let field_left = rect.left() + card_w * 0.46;
            p.rect(egui::Rect::from_min_size(egui::pos2(field_left, yy - 1.0), egui::vec2((rect.right() - field_left - 14.0).max(72.0), 24.0)), 4.0, egui::Color32::from_rgb(20, 32, 41), egui::Stroke::new(1.0_f32, border));
            p.text(egui::pos2(field_left + 8.0, yy + 11.0), egui::Align2::LEFT_CENTER, ["ダーク", "日本語", "100%（推奨）"][row_index], egui::FontId::proportional(10.0), white);
        }
    }
    let system = egui::Rect::from_min_size(egui::pos2(content.left() + 16.0, content.bottom() - 48.0), egui::vec2(content.width() - 32.0, 36.0));
    p.rect(system, 6.0, egui::Color32::from_rgb(15, 26, 33), egui::Stroke::new(1.0_f32, border));
    p.text(system.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, if narrow { "Kagari VFX v0.1.0" } else { "システム情報   Kagari VFX v0.1.0   │   macOS 14.0   │   Metal 対応" }, egui::FontId::proportional(10.0), muted);
}

fn draw_settings_target_content(ui: &mut egui::Ui, content: egui::Rect) {
    let p = ui.painter();
    let border = egui::Color32::from_rgb(43, 60, 76);
    let card_fill = egui::Color32::from_rgb(15, 26, 33);
    let white = colors::TEXT_PRIMARY;
    let muted = colors::TEXT_SECONDARY;
    p.text(egui::pos2(content.left() + 30.0, content.top() + 47.0), egui::Align2::LEFT_CENTER, "一般", egui::FontId::proportional(25.0), white);
    p.text(egui::pos2(content.left() + 30.0, content.top() + 79.0), egui::Align2::LEFT_CENTER, "アプリケーションの基本設定を管理します。", egui::FontId::proportional(15.0), muted);
    p.text(egui::pos2(content.right() - 265.0, content.top() + 47.0), egui::Align2::LEFT_CENTER, "Create. Composite. Illuminate.", egui::FontId::proportional(14.0), colors::TEXT_MUTED);
    let column_gap = 12.0;
    let column_width = ((content.width() - 60.0 - column_gap) * 0.5).max(360.0);
    let right_column = content.left() + 30.0 + column_width + column_gap;
    let cards = [
        (content.left() + 30.0, content.top() + 103.0, column_width, 250.0, "インターフェース"),
        (right_column, content.top() + 103.0, column_width, 250.0, "プロジェクトと保存"),
        (content.left() + 30.0, content.top() + 365.0, column_width, 257.0, "パフォーマンス"),
        (right_column, content.top() + 365.0, column_width, 257.0, "メディアとタイムライン"),
        (content.left() + 30.0, content.top() + 633.0, column_width, 184.0, "書き出しの既定設定"),
        (right_column, content.top() + 633.0, column_width, 184.0, "プラグイン"),
    ];
    for (x, y, w, h, title) in cards { p.rect(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h)), 9.0, card_fill, egui::Stroke::new(1.0_f32, border)); p.text(egui::pos2(x + 21.0, y + 27.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(17.0), white); }
    let field = |p: &egui::Painter, x: f32, y: f32, w: f32, value: &str| { p.rect(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, 34.0)), 6.0, egui::Color32::from_rgb(20, 32, 41), egui::Stroke::new(1.0_f32, border)); p.text(egui::pos2(x + 13.0, y + 17.0), egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(13.0), white); p.text(egui::pos2(x + w - 15.0, y + 17.0), egui::Align2::CENTER_CENTER, "⌄", egui::FontId::proportional(16.0), muted); };
    let label = |p: &egui::Painter, x: f32, y: f32, value: &str| p.text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(14.0), muted);
    label(p, cards[0].0 + 21.0, cards[0].1 + 68.0, "テーマ"); field(p, cards[0].0 + 190.0, cards[0].1 + 51.0, cards[0].2 - 211.0, "システム          ダーク          ライト");
    label(p, cards[0].0 + 21.0, cards[0].1 + 108.0, "アクセントカラー"); for (i, color) in [egui::Color32::from_rgb(255,107,22), egui::Color32::from_rgb(61,156,236), egui::Color32::from_rgb(145,93,239), egui::Color32::from_rgb(218,64,181), egui::Color32::from_rgb(242,72,69), egui::Color32::from_rgb(245,177,27), egui::Color32::from_rgb(32,196,73)].into_iter().enumerate() { p.circle_filled(egui::pos2(cards[0].0 + 204.0 + i as f32 * 38.0, cards[0].1 + 109.0), 10.0, color); }
    label(p, cards[0].0 + 21.0, cards[0].1 + 154.0, "言語"); field(p, cards[0].0 + 190.0, cards[0].1 + 137.0, cards[0].2 - 211.0, "日本語"); label(p, cards[0].0 + 21.0, cards[0].1 + 198.0, "UIのスケール"); field(p, cards[0].0 + 190.0, cards[0].1 + 181.0, cards[0].2 - 211.0, "100%（推奨）");
    label(p, cards[1].0 + 21.0, cards[1].1 + 68.0, "自動保存の間隔"); field(p, cards[1].0 + 261.0, cards[1].1 + 51.0, cards[1].2 - 282.0, "5分"); label(p, cards[1].0 + 21.0, cards[1].1 + 108.0, "自動保存ファイルの保持数"); field(p, cards[1].0 + 261.0, cards[1].1 + 91.0, cards[1].2 - 282.0, "10個"); label(p, cards[1].0 + 21.0, cards[1].1 + 152.0, "起動時に前回のプロジェクトを開く"); label(p, cards[1].0 + 21.0, cards[1].1 + 196.0, "プロジェクトのバックアップを作成"); label(p, cards[1].0 + 21.0, cards[1].1 + 240.0, "既定の保存先"); field(p, cards[1].0 + 221.0, cards[1].1 + 223.0, cards[1].2 - 242.0, "~/Kagari VFX/Projects");
    let switch_pill = |p: &egui::Painter, x: f32, y: f32| { p.rect(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(48.0, 27.0)), 14.0, egui::Color32::from_rgb(255, 104, 25), egui::Stroke::NONE); p.circle_filled(egui::pos2(x + 36.0, y + 13.5), 10.0, egui::Color32::WHITE); };
    switch_pill(p, cards[1].0 + cards[1].2 - 69.0, cards[1].1 + 118.0);
    switch_pill(p, cards[1].0 + cards[1].2 - 69.0, cards[1].1 + 162.0);
    label(p, cards[2].0 + 21.0, cards[2].1 + 68.0, "GPUアクセラレーション"); label(p, cards[2].0 + 21.0, cards[2].1 + 108.0, "使用するGPU"); field(p, cards[2].0 + 190.0, cards[2].1 + 91.0, cards[2].2 - 211.0, "自動選択（推奨）"); label(p, cards[2].0 + 21.0, cards[2].1 + 152.0, "メモリの使用上限"); let meter_right = cards[2].0 + cards[2].2 - 66.0; p.line_segment([egui::pos2(cards[2].0 + 190.0, cards[2].1 + 140.0), egui::pos2(meter_right - 18.0, cards[2].1 + 140.0)], egui::Stroke::new(5.0_f32, egui::Color32::from_rgb(255,107,22))); p.circle_filled(egui::pos2(cards[2].0 + 190.0 + (cards[2].2 - 274.0) * 0.65, cards[2].1 + 140.0), 8.0, white); label(p, meter_right, cards[2].1 + 140.0, "75%"); label(p, cards[2].0 + 21.0, cards[2].1 + 194.0, "キャッシュフォルダ"); field(p, cards[2].0 + 164.0, cards[2].1 + 177.0, cards[2].2 - 185.0, "~/Library/Caches/Kagari VFX");
    label(p, cards[3].0 + 21.0, cards[3].1 + 68.0, "プロキシメディアを自動生成"); label(p, cards[3].0 + 21.0, cards[3].1 + 108.0, "プロキシ解像度"); field(p, cards[3].0 + 290.0, cards[3].1 + 91.0, cards[3].2 - 311.0, "1/2（推奨）"); label(p, cards[3].0 + 21.0, cards[3].1 + 152.0, "既定のフレームレート"); field(p, cards[3].0 + 290.0, cards[3].1 + 135.0, cards[3].2 - 311.0, "30 fps"); label(p, cards[3].0 + 21.0, cards[3].1 + 196.0, "オーディオの波形を生成"); label(p, cards[3].0 + 21.0, cards[3].1 + 240.0, "スクロール動作"); field(p, cards[3].0 + 290.0, cards[3].1 + 223.0, cards[3].2 - 311.0, "スムーズスクロール");
    switch_pill(p, cards[3].0 + cards[3].2 - 69.0, cards[3].1 + 45.0);
    switch_pill(p, cards[3].0 + cards[3].2 - 69.0, cards[3].1 + 177.0);
    label(p, cards[4].0 + 21.0, cards[4].1 + 68.0, "フォーマット"); field(p, cards[4].0 + 168.0, cards[4].1 + 51.0, cards[4].2 - 189.0, "H.264 (MP4)"); label(p, cards[4].0 + 21.0, cards[4].1 + 101.0, "解像度"); field(p, cards[4].0 + 168.0, cards[4].1 + 84.0, cards[4].2 - 189.0, "1920 × 1080（フルHD）"); label(p, cards[4].0 + 21.0, cards[4].1 + 134.0, "フレームレート"); field(p, cards[4].0 + 168.0, cards[4].1 + 117.0, cards[4].2 - 189.0, "30 fps"); label(p, cards[4].0 + 21.0, cards[4].1 + 167.0, "品質"); field(p, cards[4].0 + 168.0, cards[4].1 + 150.0, cards[4].2 - 189.0, "高品質");
    label(p, cards[5].0 + 21.0, cards[5].1 + 68.0, "プラグインの管理"); field(p, cards[5].0 + 285.0, cards[5].1 + 51.0, cards[5].2 - 306.0, "フォルダを開く"); label(p, cards[5].0 + 21.0, cards[5].1 + 108.0, "利用可能なプラグイン"); field(p, cards[5].0 + 285.0, cards[5].1 + 91.0, cards[5].2 - 306.0, "⟳  スキャン");
    let system = egui::Rect::from_min_size(egui::pos2(content.left() + 30.0, content.bottom() - 67.0), egui::vec2(content.width() - 60.0, 50.0)); p.rect(system, 9.0, egui::Color32::from_rgb(15,26,33), egui::Stroke::new(1.0_f32, border)); p.text(egui::pos2(system.left()+21.0, system.center().y), egui::Align2::LEFT_CENTER, "システム情報", egui::FontId::proportional(14.0), white); p.text(egui::pos2(system.left()+126.0, system.center().y), egui::Align2::LEFT_CENTER, "Kagari VFX v0.1.0   │   macOS 14.0   │   Apple M2 Pro   │   メモリ 32 GB   │   Metal 対応", egui::FontId::proportional(12.0), muted); p.rect(egui::Rect::from_min_size(egui::pos2(system.right()-178.0, system.top()+9.0), egui::vec2(157.0, 32.0)), 6.0, egui::Color32::from_rgb(20,32,41), egui::Stroke::new(1.0_f32, border)); p.text(egui::pos2(system.right()-99.0, system.center().y), egui::Align2::CENTER_CENTER, "診断情報をコピー", egui::FontId::proportional(12.0), white);
}

fn sync_selected_preview(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(path) = app.home_selected.clone() else {
        return;
    };
    if !is_image_ext(&path) {
        return;
    }
    let stale = app
        .home_preview
        .as_ref()
        .map(|(cached_path, _)| cached_path != &path)
        .unwrap_or(true);
    if stale {
        app.home_preview = image::open(&path)
            .ok()
            .and_then(|img| load_texture(ctx, &format!("preview:{}", path.display()), img, 560, 320))
            .map(|handle| (path, handle));
    }
}

fn draw_reference_topbar(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let narrow = reference_narrow(ui) || reference_medium(ui);
    let ultra_narrow = narrow && ui.available_width() < 270.0;
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(if ultra_narrow { 4.0 } else if narrow { 12.0 } else { 0.0 }, 0.0))
        .show(ui, |ui| {
    ui.spacing_mut().item_spacing.x = if ultra_narrow { 3.0 } else if narrow { 5.0 } else { 8.0 };
    let nav_items = [
        (HomeNav::Home, "Home"),
        (HomeNav::Compositing, "Compositing"),
        (HomeNav::Effects, "Effects"),
        (HomeNav::Projects, "Assets"),
        (HomeNav::Render, "Render"),
    ];
    let active_top_nav = match home_nav(ctx) {
        HomeNav::NewProject => HomeNav::Projects,
        HomeNav::Settings => HomeNav::Effects,
        nav => nav,
    };
    let draw_brand = |ui: &mut egui::Ui, app: &mut KagariApp, ctx: &egui::Context| {
        if app.home_banner.is_none() {
            if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        if let Some(texture) = app.home_banner.as_ref() {
            let logo_size = if ultra_narrow { 14.0 } else if narrow { 16.0 } else { 20.0 };
            ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(logo_size, logo_size))));
        }
        ui.label(egui::RichText::new("Kagari VFX").size(if ultra_narrow { 7.5 } else if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_PRIMARY));
    };
    let draw_account_actions = |ui: &mut egui::Ui| {
        let profile_size = if narrow { 16.0 } else { 24.0 };
        let settings_size = if narrow { 15.0 } else { 18.0 };
        crate::ui::icons::render_svg_bytes(ui, "reference-profile", crate::ui::icons::SVG_PROFILE, egui::vec2(profile_size, profile_size), egui::Color32::from_rgb(172, 188, 211));
        ui.add_space(if narrow { 6.0 } else { 10.0 });
        if crate::ui::icons::render_svg_bytes(ui, "reference-settings", crate::ui::icons::SVG_SETTINGS, egui::vec2(settings_size, settings_size), colors::TEXT_SECONDARY).clicked() {
            set_home_nav(ctx, HomeNav::Settings);
        }
    };
    let draw_window_actions = |ui: &mut egui::Ui| {
        for (index, icon) in [
            crate::ui::icons::SVG_WINDOW_MINIMIZE,
            crate::ui::icons::SVG_WINDOW_MAXIMIZE,
            crate::ui::icons::SVG_WINDOW_CLOSE,
        ]
        .into_iter()
        .enumerate()
        {
            crate::ui::icons::render_svg_bytes(
                ui,
                &format!("reference-window-action-{index}"),
                icon,
                egui::vec2(if ultra_narrow { 9.0 } else { 11.0 }, if ultra_narrow { 9.0 } else { 11.0 }),
                colors::TEXT_SECONDARY,
            );
            if index < 2 {
                ui.add_space(if ultra_narrow { 2.0 } else { 4.0 });
            }
        }
    };

    if narrow {
        ui.horizontal(|ui| {
            draw_brand(ui, app, ctx);
            ui.add_space(if ultra_narrow { 5.0 } else { 8.0 });
            for (nav, label) in nav_items {
                let active = active_top_nav == nav;
                let response = ui.add(
                    egui::Label::new(
                        egui::RichText::new(label)
                            .size(if ultra_narrow { 6.0 } else { 7.0 })
                            .color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }),
                    )
                    .sense(egui::Sense::click()),
                );
                if active {
                    ui.painter().line_segment(
                        [
                            egui::pos2(response.rect.left(), response.rect.bottom() - 1.0),
                            egui::pos2(response.rect.right(), response.rect.bottom() - 1.0),
                        ],
                        egui::Stroke::new(1.0_f32, colors::ACCENT_BLUE),
                    );
                }
                if response.clicked() {
                    set_home_nav(ctx, nav);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                draw_window_actions(ui);
            });
        });
    } else {
        ui.horizontal(|ui| {
            draw_brand(ui, app, ctx);
            ui.add_space(28.0);
            for (nav, label) in nav_items {
            let active = active_top_nav == nav;
            let response = crate::ui::theme::draw_custom_tab(ui, active, label);
            if response.clicked() { set_home_nav(ctx, nav); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                draw_account_actions(ui);
            });
        });
    }
        });
}

fn draw_reference_new_project_narrow(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let top = ui.cursor().min;
    let left = egui::Rect::from_min_size(top, egui::vec2(100.0, 216.0));
    let right = egui::Rect::from_min_size(egui::pos2(top.x + 101.0, top.y), egui::vec2(214.0, 216.0));
    let fill = egui::Color32::from_rgb(16, 27, 38);
    let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    ui.painter().rect(left, 4.0, fill, stroke);
    ui.painter().rect(right, 4.0, fill, stroke);
    for (index, (icon, label)) in [(crate::ui::icons::SVG_FILE, "New Project"), (crate::ui::icons::SVG_OPEN_FOLDER, "Open Project"), (crate::ui::icons::SVG_LAYERS, "Templates"), (crate::ui::icons::SVG_HELP, "Learning")].into_iter().enumerate() {
        let y = left.top() + 8.0 + index as f32 * 20.0;
        if index == 0 { ui.painter().rect(egui::Rect::from_min_size(egui::pos2(left.left() + 5.0, y - 1.0), egui::vec2(88.0, 19.0)), 3.0, egui::Color32::from_rgb(24, 62, 102), egui::Stroke::NONE); }
        let color = if index == 0 { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY };
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 10.0, y + 2.0), egui::vec2(12.0, 12.0))), |icon_ui| {
            crate::ui::icons::render_svg_bytes(icon_ui, &format!("new-project-nav-{index}"), icon, egui::vec2(12.0, 12.0), color);
        });
        ui.painter().text(egui::pos2(left.left() + 25.0, y + 8.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(6.5), color);
    }
    if let Some(id) = reference_texture(app, ctx, "new_project_moon.webp") {
        ui.put(egui::Rect::from_min_size(egui::pos2(left.left() + 5.0, left.top() + 96.0), egui::vec2(90.0, 115.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(90.0, 115.0))).fit_to_exact_size(egui::vec2(90.0, 115.0)));
    }
    for (index, text) in ["Create.", "Composite.", "Imagine."] .into_iter().enumerate() {
        ui.painter().text(egui::pos2(left.left() + 8.0, left.top() + 158.0 + index as f32 * 11.0), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(7.0), colors::TEXT_PRIMARY);
    }
    ui.painter().text(egui::pos2(right.left() + 9.0, right.top() + 13.0), egui::Align2::LEFT_CENTER, "Create a New Project", egui::FontId::proportional(9.0), colors::TEXT_PRIMARY);
    let fields = [("Project Name", "Untitled Project"), ("Location", "/Users/redacted-user/Projects"), ("Frame Rate", "24 fps"), ("Resolution", "3440 × 2160 (4K UHD)"), ("Color Space", "Rec.709")];
    for (index, (label, value)) in fields.into_iter().enumerate() {
        let y = right.top() + 27.0 + index as f32 * 27.0;
        ui.painter().text(egui::pos2(right.left() + 9.0, y + 4.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(6.0), colors::TEXT_SECONDARY);
        let field = egui::Rect::from_min_size(egui::pos2(right.left() + 9.0, y + 10.0), egui::vec2(188.0, 18.0));
        ui.painter().rect(field, 3.0, egui::Color32::from_rgb(18, 31, 43), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
        ui.painter().text(egui::pos2(field.left() + 6.0, field.center().y), egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(6.0), colors::TEXT_PRIMARY);
        if index > 1 { ui.painter().text(egui::pos2(field.right() - 7.0, field.center().y), egui::Align2::CENTER_CENTER, "⌄", egui::FontId::proportional(7.0), colors::TEXT_MUTED); }
    }
    ui.painter().text(egui::pos2(right.left() + 9.0, right.bottom() - 41.0), egui::Align2::LEFT_CENTER, "⌄ Advanced Settings", egui::FontId::proportional(6.0), colors::TEXT_SECONDARY);
    let cancel = egui::Rect::from_min_size(egui::pos2(right.right() - 113.0, right.bottom() - 26.0), egui::vec2(35.0, 19.0));
    ui.painter().rect(cancel, 3.0, egui::Color32::from_rgb(19, 31, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
    ui.painter().text(cancel.center(), egui::Align2::CENTER_CENTER, "Cancel", egui::FontId::proportional(6.0), colors::TEXT_PRIMARY);
    let create = egui::Rect::from_min_size(egui::pos2(right.right() - 73.0, right.bottom() - 26.0), egui::vec2(57.0, 19.0));
    ui.painter().rect(create, 3.0, colors::ACCENT_BLUE, egui::Stroke::NONE);
    ui.painter().text(create.center(), egui::Align2::CENTER_CENTER, "Create Project", egui::FontId::proportional(6.0), egui::Color32::WHITE);
    ui.painter().line_segment([egui::pos2(left.left() + 8.0, left.bottom() - 12.0), egui::pos2(left.left() + 22.0, left.bottom() - 12.0)], egui::Stroke::new(1.0_f32, colors::ACCENT_CYAN));
    ui.allocate_space(egui::vec2(315.0, 216.0));
}

fn draw_reference_new_project_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
        let narrow = reference_narrow(ui);
        let medium = reference_medium(ui);
        if narrow || medium {
            draw_reference_new_project_narrow(app, ui, ctx);
            return;
        }
        let stacked = (!narrow && ui.available_width() < 720.0) || (narrow && ui.available_width() < 300.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 6.0 } else { 10.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 84.0 } else if medium { 180.0 } else if compact { 170.0 } else { 220.0 });
                let menu_id = egui::Id::new("reference_new_project_menu");
                let mut menu = ctx.data_mut(|d| d.get_temp::<usize>(menu_id)).unwrap_or(0);
                for (index, (icon, label)) in [
                    (crate::ui::icons::SVG_FILE, "New Project"),
                    (crate::ui::icons::SVG_OPEN_FOLDER, "Open Project"),
                    (crate::ui::icons::SVG_LAYERS, "Templates"),
                    (crate::ui::icons::SVG_HELP, "Learning"),
                ].into_iter().enumerate() {
                    let active = menu == index;
                    let row_width = ui.available_width();
                    let row_height = if narrow { 20.0 } else { 30.0 };
                    let clicked = ui.allocate_ui_with_layout(
                        egui::vec2(row_width, row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            egui::Frame::none()
                                .fill(if active { egui::Color32::from_rgb(24, 62, 102) } else { egui::Color32::TRANSPARENT })
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                .show(ui, |ui| {
                                    ui.set_width(row_width - 8.0);
                                    ui.horizontal(|ui| {
                                        let icon_size = if narrow { 13.0 } else { 16.0 };
                                        crate::ui::icons::render_svg_bytes(ui, label, icon, egui::vec2(icon_size, icon_size), if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
                                        ui.add_space(5.0);
                                        ui.label(egui::RichText::new(label).size(if narrow { 9.0 } else { 12.0 }).color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                        },
                    ).inner.clicked();
                    if clicked {
                        menu = index;
                        match index {
                            1 => enter_studio_open_dialog(app),
                            2 => set_home_nav(ctx, HomeNav::Templates),
                            3 => app.show_shortcuts_dialog = true,
                            _ => {}
                        }
                    }
                }
                ctx.data_mut(|d| d.insert_temp(menu_id, menu));
                ui.add_space(if narrow { 8.0 } else { 12.0 });
                if let Some(id) = reference_texture(app, ctx, "recent_project_rift.webp") {
                    let image_size = egui::vec2(ui.available_width(), if narrow { 118.0 } else if compact { 150.0 } else { 230.0 });
                    let image_top = ui.cursor().min;
                    reference_cover_image(ui, id, image_size, 2.64);
                    let text_size = if narrow { 8.0 } else { 14.0 };
                    for (index, line) in ["Create.", "Composite.", "Imagine."].into_iter().enumerate() {
                        ui.painter().text(
                            image_top + egui::vec2(4.0, image_size.y - 34.0 + index as f32 * (text_size + 2.0)),
                            egui::Align2::LEFT_TOP,
                            line,
                            egui::FontId::proportional(text_size),
                            colors::TEXT_PRIMARY,
                        );
                    }
                    let line_y = image_top.y + image_size.y - 8.0;
                    ui.painter().line_segment(
                        [egui::pos2(image_top.x + 4.0, line_y), egui::pos2(image_top.x + if narrow { 20.0 } else { 38.0 }, line_y)],
                        egui::Stroke::new(1.0_f32, colors::ACCENT_BLUE),
                    );
                }
            });
            if stacked {
                ui.end_row();
                ui.add_space(9.0);
            } else {
                ui.add_space(if narrow { 4.0 } else { 14.0 });
            }
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 8.0 } else { 16.0 })).show(ui, |ui| {
                let narrow_width = if stacked {
                    (ui.available_width() - 16.0).max(176.0)
                } else {
                    (ui.available_width() - 12.0).clamp(190.0, 199.0)
                };
                ui.set_width(if narrow { narrow_width } else { (ui.available_width() - 14.0).max(if medium || compact { 280.0 } else { 420.0 }) });
                ui.label(egui::RichText::new("Create a New Project").size(if narrow { 11.0 } else { 17.0 }).strong());
                ui.add_space(if narrow { 5.0 } else { 10.0 });
                ui.separator();
                let name_id = egui::Id::new("reference_new_project_name");
                let location_id = egui::Id::new("reference_new_project_location");
                let mut name = ctx.data_mut(|d| d.get_temp::<String>(name_id)).unwrap_or_else(|| "Untitled Project".to_string());
                let mut location = ctx.data_mut(|d| d.get_temp::<String>(location_id)).unwrap_or_else(|| "/Users/redacted-user/Projects".to_string());
                ui.add_space(if narrow { 4.0 } else { 8.0 });
                ui.label(egui::RichText::new("Project Name").size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                ui.add(egui::TextEdit::singleline(&mut name).desired_width(ui.available_width()));
                ui.add_space(if narrow { 4.0 } else { 8.0 });
                ui.label(egui::RichText::new("Location").size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                ui.horizontal(|ui| { ui.add(egui::TextEdit::singleline(&mut location).desired_width(ui.available_width() - if narrow { 24.0 } else { 30.0 })); reference_icon_button(ui, "new-project-location", crate::ui::icons::SVG_FOLDER, if narrow { 18.0 } else { 24.0 }); });
                ui.add_space(if narrow { 4.0 } else { 8.0 });
                for (label, value) in [("Frame Rate", "24 fps"), ("Resolution", "3440 × 2160 (4K UHD)"), ("Color Space", "Rec.709")] {
                    ui.horizontal(|ui| { ui.label(egui::RichText::new(label).size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { reference_select_button(ui, value, if narrow { ui.available_width().min(154.0) } else { 210.0 }, narrow); }); });
                    ui.add_space(if narrow { 2.0 } else { 5.0 });
                }
                ui.collapsing("Advanced Settings", |ui| { ui.label(egui::RichText::new("Project defaults and cache options").small().color(colors::TEXT_MUTED)); });
                ui.add_space(if narrow { 6.0 } else { 12.0 });
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("Cancel").size(if narrow { 8.0 } else { 14.0 })).min_size(egui::vec2(if narrow { 42.0 } else { 76.0 }, if narrow { 22.0 } else { 28.0 }))).clicked() { set_home_nav(ui.ctx(), HomeNav::Home); }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.add(egui::Button::new(egui::RichText::new("Create Project").size(if narrow { 8.0 } else { 14.0 }).color(egui::Color32::WHITE)).fill(colors::ACCENT_BLUE).min_size(egui::vec2(if narrow { 68.0 } else { 112.0 }, if narrow { 22.0 } else { 28.0 })).rounding(5.0)).clicked() { enter_studio_new_project(app); } });
                });
                ctx.data_mut(|d| { d.insert_temp(name_id, name); d.insert_temp(location_id, location); });
            });
        });
    });
}

fn draw_reference_assets_narrow(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let top = ui.cursor().min;
    let panel_y = top.y;
    let panel_h = 228.0;
    let left = egui::Rect::from_min_size(egui::pos2(top.x, panel_y), egui::vec2(77.0, panel_h));
    let center = egui::Rect::from_min_size(egui::pos2(left.right() + 5.0, panel_y), egui::vec2(151.0, panel_h));
    let detail = egui::Rect::from_min_size(egui::pos2(center.right() + 5.0, panel_y), egui::vec2(90.0, panel_h));
    let panel_fill = egui::Color32::from_rgb(16, 27, 38);
    let panel_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    for rect in [left, detail] {
        ui.painter().rect(rect, 4.0, panel_fill, panel_stroke);
    }
    ui.painter().rect(center, 4.0, egui::Color32::from_rgb(11, 19, 26), panel_stroke);
    ui.painter().text(egui::pos2(left.left() + 6.0, left.top() + 11.0), egui::Align2::LEFT_CENTER, "Project", egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
    let search = egui::Rect::from_min_size(egui::pos2(left.left() + 7.0, left.top() + 27.0), egui::vec2(63.0, 18.0));
    ui.painter().rect(search, 3.0, egui::Color32::from_rgb(12, 21, 30), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 58, 74)));
    ui.painter().text(egui::pos2(search.left() + 5.0, search.center().y), egui::Align2::LEFT_CENTER, "⌕  Search", egui::FontId::proportional(6.5), colors::TEXT_SECONDARY);
    for (index, (label, folder, expanded)) in [("Footage", true, true), ("solids", false, false), ("city", false, false), ("elements", false, false), ("textures", false, false), ("Comps", true, false), ("Solids", true, false), ("Audio", true, false), ("Renders", true, false)].into_iter().enumerate() {
        let y = left.top() + 44.0 + index as f32 * 17.0;
        if folder {
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 7.0, y + 2.0), egui::vec2(7.0, 7.0))), |icon_ui| {
                crate::ui::icons::render_svg_bytes(icon_ui, &format!("assets-tree-chevron-{index}"), if expanded { crate::ui::icons::SVG_CHEVRON_DOWN } else { crate::ui::icons::SVG_CHEVRON_RIGHT }, egui::vec2(7.0, 7.0), colors::TEXT_SECONDARY);
            });
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 16.0, y + 1.0), egui::vec2(10.0, 10.0))), |icon_ui| {
                crate::ui::icons::render_svg_bytes(icon_ui, &format!("assets-tree-folder-{index}"), crate::ui::icons::SVG_FOLDER, egui::vec2(10.0, 10.0), colors::TEXT_SECONDARY);
            });
            ui.painter().text(egui::pos2(left.left() + 29.0, y + 7.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(7.0), colors::TEXT_SECONDARY);
        } else {
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 16.0, y + 3.0), egui::vec2(8.0, 8.0))), |icon_ui| {
                crate::ui::icons::render_svg_bytes(icon_ui, &format!("assets-tree-file-{index}"), crate::ui::icons::SVG_FILE, egui::vec2(8.0, 8.0), colors::TEXT_SECONDARY);
            });
            ui.painter().text(egui::pos2(left.left() + 27.0, y + 7.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(7.0), colors::TEXT_SECONDARY);
        }
    }
    ui.put(egui::Rect::from_min_size(egui::pos2(center.left() + 5.0, center.top() + 5.0), egui::vec2(48.0, 20.0)), egui::Button::new(egui::RichText::new("All Types").size(7.0)));
    ui.put(egui::Rect::from_min_size(egui::pos2(center.left() + 56.0, center.top() + 5.0), egui::vec2(43.0, 20.0)), egui::Button::new(egui::RichText::new("Date").size(7.0)));
    let icon_y = center.top() + 15.0;
    let icon_color = colors::TEXT_SECONDARY;
    for (x, kind) in [(center.right() - 36.0, 0_u8), (center.right() - 24.0, 1_u8), (center.right() - 12.0, 2_u8)] {
        let icon = egui::pos2(x, icon_y);
        if kind == 0 {
            for dx in [-3.0_f32, 1.0] { for dy in [-3.0_f32, 1.0] { ui.painter().rect(egui::Rect::from_center_size(icon + egui::vec2(dx, dy), egui::vec2(2.0, 2.0)), 0.5, icon_color, egui::Stroke::NONE); } }
        } else if kind == 1 {
            ui.painter().line_segment([icon + egui::vec2(-3.0, -3.0), icon + egui::vec2(3.0, -3.0)], egui::Stroke::new(1.0_f32, icon_color));
            ui.painter().line_segment([icon + egui::vec2(-2.0, 0.0), icon + egui::vec2(3.0, 0.0)], egui::Stroke::new(1.0_f32, icon_color));
            ui.painter().line_segment([icon + egui::vec2(-1.0, 3.0), icon + egui::vec2(3.0, 3.0)], egui::Stroke::new(1.0_f32, icon_color));
        } else {
            ui.painter().rect_stroke(egui::Rect::from_center_size(icon, egui::vec2(7.0, 7.0)), 1.0, egui::Stroke::new(1.0_f32, icon_color));
            ui.painter().line_segment([icon + egui::vec2(-2.0, 0.0), icon + egui::vec2(2.0, 0.0)], egui::Stroke::new(1.0_f32, icon_color));
        }
    }
    let assets = [
        ("recent_project_citadel.webp", "city_01.mp4"),
        ("recent_project_rift.webp", "mountain_bg.jpg"),
        ("continue_working_preview.webp", "clouds.mov"),
        ("recent_project_atlas.webp", "stroke_01.mov"),
        ("recent_project_eclipse.webp", "light_leak.mov"),
        ("recent_project_rift.webp", "glass_texture.png"),
        ("logo.png", "logo.png"),
        ("recent_project_atlas.webp", "particles.mov"),
        ("recent_project_rift.webp", "four_red.jpeg"),
    ];
    for (index, (asset, name)) in assets.into_iter().enumerate() {
        let col = index % 3;
        let row = index / 3;
        let card = egui::Rect::from_min_size(
            egui::pos2(center.left() + 5.0 + col as f32 * 48.0, center.top() + 34.0 + row as f32 * 64.0),
            egui::vec2(44.0, 58.0),
        );
        ui.painter().rect(card, 4.0, egui::Color32::from_rgb(22, 32, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
        if name == "logo.png" {
            ui.put(card.shrink2(egui::vec2(2.0, 2.0)), egui::Image::new(egui::load::SizedTexture::new(reference_texture(app, ctx, asset).unwrap_or_default(), egui::vec2(40.0, 34.0))));
        } else if let Some(id) = reference_texture(app, ctx, asset) {
            ui.put(egui::Rect::from_min_size(card.min + egui::vec2(2.0, 2.0), egui::vec2(40.0, 34.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(40.0, 34.0))).fit_to_exact_size(egui::vec2(40.0, 34.0)));
        }
        ui.painter().text(egui::pos2(card.left() + 2.0, card.bottom() - 12.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(5.0), colors::TEXT_PRIMARY);
    }
    ui.painter().text(egui::pos2(detail.left() + 5.0, detail.top() + 11.0), egui::Align2::LEFT_CENTER, "city_01.mp4", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    if let Some(id) = reference_texture(app, ctx, "recent_project_citadel.webp") {
        ui.put(egui::Rect::from_min_size(egui::pos2(detail.left() + 5.0, detail.top() + 21.0), egui::vec2(80.0, 43.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(80.0, 43.0))).fit_to_exact_size(egui::vec2(80.0, 43.0)));
    }
    ui.painter().text(egui::pos2(detail.left() + 5.0, detail.top() + 76.0), egui::Align2::LEFT_CENTER, "Metadata", egui::FontId::proportional(7.0), colors::TEXT_PRIMARY);
    for (index, (label, value)) in [("Frame Rate", "24 fps"), ("Frame Size", "3840 × 2160"), ("Duration", "00:12"), ("Color Space", "Rec.709"), ("Codec", "H.264"), ("Audio", "48 kHz")].into_iter().enumerate() {
        let y = detail.top() + 91.0 + index as f32 * 14.0;
        ui.painter().text(egui::pos2(detail.left() + 5.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(5.3), colors::TEXT_MUTED);
        ui.painter().text(egui::pos2(detail.right() - 5.0, y), egui::Align2::RIGHT_CENTER, value, egui::FontId::proportional(5.3), colors::TEXT_SECONDARY);
    }
    ui.allocate_space(egui::vec2(egui::Rect::from_min_size(top, egui::vec2(322.0, panel_h)).width(), panel_h));
}

fn draw_reference_project_browser_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    let narrow = reference_narrow(ui);
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(if narrow { 8.0 } else { reference_content_margin(ui) }, 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if narrow { 8.0 } else if compact { 22.0 } else { 42.0 });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("プロジェクト").size(if narrow { 22.0 } else { 30.0 }).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new("＋ 新規プロジェクト").size(if narrow { 9.0 } else { 12.0 })).fill(colors::ACCENT_ORANGE).rounding(5.0)).clicked() {
                    enter_studio_new_project(app);
                    app.show_new_comp_dialog = true;
                }
                ui.add_space(8.0);
                if ui.add(egui::Button::new(egui::RichText::new("▱ プロジェクトを開く").size(if narrow { 9.0 } else { 12.0 })).rounding(5.0)).clicked() {
                    enter_studio_open_dialog(app);
                }
            });
        });
        ui.add_space(8.0);
        ui.label(egui::RichText::new("保存済みのプロジェクトを管理し、前回の作業を続けます。").size(if narrow { 9.0 } else { 13.0 }).color(colors::TEXT_SECONDARY));
        ui.add_space(if compact { 12.0 } else { 18.0 });
        reference_search_field(ui, &mut app.home_search, "プロジェクトを検索...", ui.available_width().min(480.0), if narrow { 26.0 } else { 34.0 });
        ui.add_space(if compact { 12.0 } else { 22.0 });
        let query = app.home_search.trim().to_ascii_lowercase();
        let projects = recent_project_entries(ctx).into_iter().filter(|project| query.is_empty() || project.name.to_ascii_lowercase().contains(&query)).collect::<Vec<_>>();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("最近のプロジェクト  ({})", projects.len())).size(if narrow { 13.0 } else { 18.0 }).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                reference_select_button(ui, "更新日時", if narrow { 76.0 } else { 108.0 }, narrow);
                ui.add_space(6.0);
                reference_select_button(ui, "すべて", if narrow { 58.0 } else { 82.0 }, narrow);
            });
        });
        ui.add_space(10.0);
        if projects.is_empty() {
            let empty_height = if compact { 190.0 } else { 280.0 };
            fixed_card_with_inset(ui, egui::vec2(ui.available_width(), empty_height), egui::Color32::TRANSPARENT, egui::Color32::from_rgb(54, 70, 83), 7.0, 18.0, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(if compact { 28.0 } else { 52.0 });
                    crate::ui::icons::render_svg_bytes(ui, "projects-empty-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(44.0, 44.0), colors::TEXT_SECONDARY);
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new(if query.is_empty() { "まだプロジェクトはありません" } else { "一致するプロジェクトはありません" }).size(if narrow { 14.0 } else { 19.0 }).strong());
                    ui.label(egui::RichText::new(if query.is_empty() { "新規プロジェクトを作成して、はじめましょう。" } else { "検索条件を変更してください。" }).size(if narrow { 10.0 } else { 13.0 }).color(colors::TEXT_SECONDARY));
                    if query.is_empty() {
                        ui.add_space(14.0);
                        if reference_cta_button(ui, "新規プロジェクト", compact).clicked() { enter_studio_new_project(app); app.show_new_comp_dialog = true; }
                    }
                });
            });
        } else {
            let columns = if narrow { 1.0 } else if ui.available_width() < 820.0 { 2.0 } else { 3.0 };
            let gap = if compact { 10.0 } else { 16.0 };
            let card_width = ((ui.available_width() - gap * (columns - 1.0)) / columns).max(170.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                for (index, project) in projects.iter().enumerate() {
                    let card = fixed_card_with_inset(ui, egui::vec2(card_width, if compact { 154.0 } else { 204.0 }), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 6.0, 0.0, |ui| {
                        if let Some(id) = reference_texture(app, ctx, ["recent_project_eclipse.webp", "recent_project_citadel.webp", "recent_project_rift.webp", "recent_project_atlas.webp"][index % 4]) { reference_cover_image(ui, id, egui::vec2(card_width, if compact { 72.0 } else { 112.0 }), 2.64); }
                        ui.add_space(10.0);
                        ui.horizontal(|ui| { ui.add_space(12.0); ui.label(egui::RichText::new(&project.name).size(if compact { 13.0 } else { 16.0 }).strong()); });
                        let comp_name = project.summary.as_ref().and_then(|summary| summary.comp_names.get(summary.active_idx)).map(String::as_str).unwrap_or("Project");
                        ui.horizontal(|ui| { ui.add_space(12.0); ui.label(egui::RichText::new(comp_name).size(11.0).color(colors::TEXT_SECONDARY)); });
                        let age = project.modified.map(fmt_age).unwrap_or_else(|| "Not found".to_string());
                        ui.horizontal(|ui| { ui.add_space(12.0); ui.label(egui::RichText::new(age).size(11.0).color(colors::TEXT_MUTED)); });
                    });
                    if card.clicked() { open_project_path(app, &project.path); }
                    if index % columns as usize != columns as usize - 1 { ui.add_space(gap); }
                }
            });
        }
    });
}

fn draw_reference_projects_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    let bg = egui::Color32::from_rgb(11, 19, 26);
    ui.painter().rect_filled(ui.max_rect(), 0.0, bg);
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0))
        .show(ui, |ui| {
            draw_reference_topbar(app, ui, ctx);
            ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
            let layout_width = ui.available_width();
            let narrow = layout_width < 500.0;
            let medium = !narrow && layout_width < 1200.0;
            if narrow || medium {
                draw_reference_assets_narrow(app, ui, ctx);
                return;
            }
            if narrow {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui(egui::vec2(77.0, 228.0), |ui| { egui::Frame::none()
                        .fill(egui::Color32::from_rgb(16, 27, 38))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67)))
                        .rounding(4.0)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.set_width(69.0);
                            ui.label(egui::RichText::new("Project").size(10.0));
                            ui.label(egui::RichText::new("⌕").size(11.0).color(colors::TEXT_SECONDARY));
                            ui.add_space(3.0);
                            for label in ["▾ Ftg", "  city", "  Comp", "  Aud"] {
                                ui.label(egui::RichText::new(label).size(7.0).color(colors::TEXT_SECONDARY));
                            }
                        }); });
                    ui.add_space(5.0);
                    ui.allocate_ui(egui::vec2(153.0, 228.0), |ui| { ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            reference_select_button(ui, "All Types", 48.0, true);
                            reference_select_button(ui, "Date", 38.0, true);
                        });
                        ui.add_space(5.0);
                        let assets = [
                            ("recent_project_citadel.webp", "city_01"),
                            ("recent_project_rift.webp", "mountain"),
                            ("continue_working_preview.webp", "clouds"),
                            ("recent_project_atlas.webp", "stroke_01"),
                            ("recent_project_eclipse.webp", "light_leak"),
                            ("recent_project_rift.webp", "glass"),
                        ];
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                            for (asset, name) in assets {
                                fixed_card_with_inset(ui, egui::vec2(47.0, 49.0), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 4.0, 0.0, |ui| {
                                    if let Some(id) = reference_texture(app, ctx, asset) {
                                        reference_cover_image(ui, id, egui::vec2(47.0, 30.0), 1.5);
                                    }
                                    ui.label(egui::RichText::new(name).size(6.0));
                                });
                            }
                        });
                    }); });
                    ui.add_space(5.0);
                    ui.allocate_ui(egui::vec2(74.0, 228.0), |ui| { egui::Frame::none()
                        .fill(egui::Color32::from_rgb(16, 27, 38))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67)))
                        .rounding(4.0)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.set_width(66.0);
                            ui.label(egui::RichText::new("city_01").size(8.0));
                            if let Some(id) = reference_texture(app, ctx, "recent_project_citadel.webp") {
                                reference_cover_image(ui, id, egui::vec2(64.0, 42.0), 1.5);
                            }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Metadata").size(7.0));
                            ui.label(egui::RichText::new("3840×2160").size(6.0).color(colors::TEXT_SECONDARY));
                        }); });
                });
                return;
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let left_panel_width = if narrow { 77.0 } else if medium { 150.0 } else { 214.0 };
                ui.allocate_ui(egui::vec2(left_panel_width, if narrow { 228.0 } else { ui.available_height() }), |ui| {
                ui.set_min_width(left_panel_width);
                ui.set_max_width(left_panel_width);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(16, 27, 38))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67)))
                    .rounding(4.0)
                        .inner_margin(egui::Margin::same(if narrow { 6.0 } else { 10.0 }))
                    .show(ui, |ui| {
                        ui.set_width(if narrow { 65.0 } else if medium { 150.0 } else { 214.0 });
                        ui.set_min_height(if narrow { 228.0 } else { 0.0 });
                        ui.label(egui::RichText::new("Project").size(13.0));
                        ui.add_space(6.0);
                        let search_width = ui.available_width();
                        reference_asset_search_field(ui, &mut app.home_search, "Search...", search_width, if narrow { 22.0 } else { 28.0 });
                        ui.add_space(6.0);
                        let folder_id = egui::Id::new("reference_asset_folder");
                        let mut selected_folder = ctx.data_mut(|d| d.get_temp::<usize>(folder_id)).unwrap_or(1);
                        for (index, (label, expanded, has_children)) in [("Footage", true, true), ("solids", false, false), ("city", false, false), ("elements", false, false), ("textures", false, false), ("Comps", false, true), ("Solids", false, true), ("Audio", false, true), ("Renders", false, true)].into_iter().enumerate() {
                            let active = selected_folder == index;
                            let fill = if active { egui::Color32::from_rgb(24, 62, 102) } else { egui::Color32::TRANSPARENT };
                            let row_width = ui.available_width();
                            let clicked = ui.allocate_ui_with_layout(
                                egui::vec2(row_width, if narrow { 21.0 } else { 26.0 }),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    egui::Frame::none()
                                        .fill(fill)
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.set_width((row_width - 8.0).max(1.0));
                                            ui.horizontal(|ui| {
                                                ui.add_space(if index == 0 { 0.0 } else { if narrow { 5.0 } else { 12.0 } });
                                                if has_children {
                                                    crate::ui::icons::render_svg_bytes(ui, &format!("asset-tree-chevron-{index}"), if expanded { crate::ui::icons::SVG_CHEVRON_DOWN } else { crate::ui::icons::SVG_CHEVRON_RIGHT }, egui::vec2(if narrow { 7.0 } else { 9.0 }, if narrow { 7.0 } else { 9.0 }), colors::TEXT_SECONDARY);
                                                } else {
                                                    ui.add_space(if narrow { 7.0 } else { 9.0 });
                                                }
                                                crate::ui::icons::render_svg_bytes(ui, &format!("asset-tree-folder-{index}"), crate::ui::icons::SVG_FOLDER, egui::vec2(if narrow { 11.0 } else { 14.0 }, if narrow { 11.0 } else { 14.0 }), if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
                                                ui.label(egui::RichText::new(label).size(if narrow { 7.0 } else { 12.0 }).color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }));
                                            });
                                        })
                                        .response
                                        .interact(egui::Sense::click())
                                },
                            ).inner.clicked();
                            if clicked {
                                selected_folder = index;
                            }
                        }
                        ctx.data_mut(|d| d.insert_temp(folder_id, selected_folder));
                    });
                });
                ui.add_space(if narrow { 5.0 } else { 12.0 });
                let center_panel_width = if narrow { 153.0 } else { ui.available_width() };
                ui.allocate_ui(egui::vec2(center_panel_width, ui.available_height()), |ui| {
                ui.set_min_width(center_panel_width);
                ui.set_max_width(center_panel_width);
                ui.vertical(|ui| {
                    if narrow {
                        ui.set_width(153.0);
                    }
                    ui.horizontal(|ui| {
                        let filter_id = egui::Id::new("reference_asset_filter");
                        let mut selected_filter = ctx.data_mut(|d| d.get_temp::<usize>(filter_id)).unwrap_or(0);
                        for (index, label, width) in [(0, "All Types", if narrow { 48.0 } else { 92.0 }), (1, "Date Modified", if narrow { 60.0 } else { 112.0 })] {
                            if reference_select_button(ui, label, width, narrow).clicked() { selected_filter = index; }
                        }
                        for (index, icon) in [(2, crate::ui::icons::SVG_GRID), (3, crate::ui::icons::SVG_SORT)] {
                            if reference_icon_button(ui, &format!("asset-filter-{index}"), icon, 24.0).clicked() { selected_filter = index; }
                        }
                        ctx.data_mut(|d| d.insert_temp(filter_id, selected_filter));
                    });
                    ui.add_space(if narrow { 6.0 } else { 8.0 });
                    let grid_width = ui.available_width();
                    let columns = if narrow { 3 } else if grid_width < 660.0 { 2 } else { 3 };
                    let gap = if narrow { 7.0 } else { 10.0 };
                    let card_width = ((grid_width - gap * (columns as f32 - 1.0)) / columns as f32).max(if narrow { 40.0 } else if medium { 112.0 } else { 140.0 });
                    let selection_id = egui::Id::new("reference_asset_selection");
                    let mut selected_asset = ctx.data_mut(|d| d.get_temp::<String>(selection_id)).unwrap_or_else(|| "city_01.mp4".to_string());
                    let asset_cards = [
                        ("recent_project_citadel.webp", "city_01.mp4", "00:12"),
                        ("recent_project_rift.webp", "mountain_bg.jpg", "00:08"),
                        ("continue_working_preview.webp", "clouds.mov", "00:09"),
                        ("recent_project_atlas.webp", "stroke_01.mov", "00:04"),
                        ("recent_project_eclipse.webp", "light_leak.mov", "00:06"),
                        ("recent_project_rift.webp", "glass_texture.png", "4096 × 4096"),
                        ("logo.png", "logo.png", "1024 × 1024"),
                        ("recent_project_atlas.webp", "particles.mov", "00:05"),
                        ("recent_project_rift.webp", "four_red.jpeg", "00:04"),
                    ];
                    let query = app.home_search.trim().to_ascii_lowercase();
                    let visible_asset_count = asset_cards.iter().filter(|(_, name, kind)| {
                        query.is_empty() || format!("{name} {kind}").to_ascii_lowercase().contains(&query)
                    }).count();
                    let mut rendered_assets = 0_usize;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (asset, name, kind) in asset_cards {
                            if !query.is_empty() && !format!("{name} {kind}").to_ascii_lowercase().contains(&query) {
                                continue;
                            }
                            rendered_assets += 1;
                            let response = fixed_card_with_inset(ui, egui::vec2(card_width, if narrow { 60.0 } else if compact { 124.0 } else { 152.0 }), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 6.0, if narrow { 0.0 } else { 14.0 }, |ui| {
                                let image_size = egui::vec2(if narrow { card_width } else { (card_width - 28.0).max(12.0) }, if narrow { 34.0 } else if compact { 74.0 } else { 96.0 });
                                if name == "logo.png" {
                                    crate::ui::icons::render_svg_bytes(ui, "asset-logo-preview", crate::ui::icons::SVG_ASSET_LOGO, image_size, egui::Color32::WHITE);
                                } else if let Some(id) = reference_texture(app, ctx, asset) {
                                    reference_cover_image(ui, id, image_size, 2.64);
                                }
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    if narrow { ui.add_space(4.0); }
                                    ui.label(egui::RichText::new(name).size(if narrow { 8.0 } else { 12.0 }));
                                });
                                ui.horizontal(|ui| {
                                    if narrow { ui.add_space(4.0); }
                                    ui.label(egui::RichText::new(kind).small().color(colors::TEXT_SECONDARY));
                                });
                            });
                            if selected_asset == name {
                                ui.painter().rect_stroke(response.rect, 6.0, egui::Stroke::new(1.0_f32, colors::ACCENT_BLUE));
                            }
                            if response.clicked() {
                                selected_asset = name.to_string();
                            }
                            if rendered_assets < visible_asset_count {
                                ui.add_space(gap);
                            }
                        }
                    });
                    ctx.data_mut(|d| d.insert_temp(selection_id, selected_asset.clone()));
                    if visible_asset_count == 0 {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No assets found").size(14.0).color(colors::TEXT_PRIMARY));
                            ui.label(egui::RichText::new("Try a different search or folder.").small().color(colors::TEXT_MUTED));
                        });
                    }
                });
                });
                ui.add_space(if narrow { 5.0 } else { 12.0 });
                let detail_panel_width = if narrow { 74.0 } else if medium { 154.0 } else { 188.0 };
                ui.allocate_ui(egui::vec2(detail_panel_width, if narrow { 228.0 } else { ui.available_height() }), |ui| {
                ui.set_min_width(detail_panel_width);
                ui.set_max_width(detail_panel_width);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(16, 27, 38))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67)))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::same(if narrow { 4.0 } else { 12.0 }))
                    .show(ui, |ui| {
                        ui.set_width(if narrow { 66.0 } else if medium { 154.0 } else { 188.0 });
                        ui.set_min_height(if narrow { 228.0 } else { 0.0 });
                        let selected_name = selected_asset_name(ctx);
                        ui.label(egui::RichText::new(&selected_name).size(if narrow { 9.0 } else { 13.0 }));
                        ui.add_space(if narrow { 4.0 } else { 8.0 });
                        let preview_width = if narrow { ui.available_width().max(1.0) } else if medium { 130.0 } else { 164.0 };
                        let preview_size = egui::vec2(preview_width, if narrow { 70.0 } else if medium { 80.0 } else { 94.0 });
                        if selected_name == "logo.png" {
                            crate::ui::icons::render_svg_bytes(ui, "asset-logo-detail-preview", crate::ui::icons::SVG_ASSET_LOGO, preview_size, egui::Color32::WHITE);
                        } else if let Some(id) = reference_texture(app, ctx, asset_thumbnail_for_name(&selected_name)) {
                            reference_cover_image(ui, id, preview_size, 319.0 / 121.0);
                        }
                        ui.add_space(if narrow { 4.0 } else { 8.0 });
                        ui.label(egui::RichText::new("Metadata").size(if narrow { 8.0 } else { 12.0 }));
                        for (label, value) in asset_metadata_for_name(&selected_name) {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(*label).size(if narrow { 7.0 } else { 10.5 }).color(colors::TEXT_SECONDARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(*value).size(if narrow { 7.0 } else { 10.5 })); });
                            });
                            ui.add_space(if narrow { 1.0 } else { 3.0 });
                        }
                    });
                });
            });
        });
}

fn draw_reference_documentation_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let narrow = reference_narrow(ui);
    let compact = ui.available_height() < 820.0;
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(if narrow { 8.0 } else { reference_content_margin(ui) }, 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if narrow { 12.0 } else { 36.0 });
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("ドキュメント").size(if narrow { 22.0 } else { 30.0 }).strong());
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Kagari VFX の使い方と制作ワークフローを確認できます。").size(if narrow { 9.0 } else { 13.0 }).color(colors::TEXT_SECONDARY));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if reference_cta_button(ui, "チュートリアルを開始", compact).clicked() {
                    app.show_home = false;
                    app.show_guided_tutorial = true;
                    app.tutorial_step = 0;
                }
            });
        });
        ui.add_space(if compact { 18.0 } else { 28.0 });
        let cards = [
            (crate::ui::icons::SVG_HOME, "はじめに", "Kagari VFX の画面構成と基本的な考え方"),
            (crate::ui::icons::SVG_LAYERS, "レイヤーと合成", "素材を重ね、調整し、自然な合成を作る"),
            (crate::ui::icons::SVG_LIGHT, "エフェクト", "Glow、Blur、色補正などの適用方法"),
            (crate::ui::icons::SVG_TRACKING, "トラッキング", "動く素材にマスクやレイヤーを追従させる"),
            (crate::ui::icons::SVG_EXPORT, "書き出し", "映像を目的の形式と解像度で出力する"),
            (crate::ui::icons::SVG_SETTINGS, "設定と環境", "自動保存、キャッシュ、GPU設定を管理する"),
        ];
        let cols = if narrow { 1 } else { 2 };
        let gap = if narrow { 8.0 } else { 14.0 };
        let card_w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).max(160.0);
        for i in (0..cards.len()).step_by(cols) {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let index = i + col;
                    if index >= cards.len() { break; }
                    let (card_icon, card_title, card_desc) = cards[index];
                    documentation_card(ui, card_icon, card_title, card_desc, card_w, compact);
                    if col + 1 < cols { ui.add_space(gap); }
                }
            });
            ui.add_space(gap);
        }
        ui.separator();
        ui.add_space(12.0);
        ui.label(egui::RichText::new("クイックリファレンス").size(if narrow { 14.0 } else { 18.0 }).strong());
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for shortcut in ["V  選択", "Space  再生", "Cmd / Ctrl + S  保存", "Cmd / Ctrl + M  書き出し", "Cmd / Ctrl + K  コマンドパレット"] {
                ui.add(egui::Label::new(egui::RichText::new(shortcut).size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY)).truncate());
                ui.add_space(18.0);
            }
        });
    });
}

fn documentation_card(ui: &mut egui::Ui, icon: &'static str, title: &str, desc: &str, width: f32, compact: bool) {
    let height = if compact { 84.0 } else { 112.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    ui.painter().rect(rect, 7.0, egui::Color32::from_rgb(15, 26, 34), egui::Stroke::new(1.0_f32, if response.hovered() { colors::ACCENT_BLUE } else { colors::BORDER_SUBTLE }));
    crate::ui::icons::render_svg_at(ui, format!("documentation-card-{title}"), icon, egui::vec2(if compact { 22.0 } else { 28.0 }, if compact { 22.0 } else { 28.0 }), colors::TEXT_PRIMARY, egui::pos2(rect.left() + 16.0, rect.top() + 16.0));
    ui.put(egui::Rect::from_min_max(egui::pos2(rect.left() + 58.0, rect.top() + 13.0), egui::pos2(rect.right() - 12.0, rect.top() + 39.0)), egui::Label::new(egui::RichText::new(title).size(if compact { 12.0 } else { 15.0 }).strong().color(colors::TEXT_PRIMARY)).truncate());
    ui.put(egui::Rect::from_min_max(egui::pos2(rect.left() + 58.0, rect.top() + 42.0), egui::pos2(rect.right() - 12.0, rect.bottom() - 10.0)), egui::Label::new(egui::RichText::new(desc).size(if compact { 9.0 } else { 11.0 }).color(colors::TEXT_SECONDARY)).truncate());
}

fn draw_reference_effects_narrow(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let top = ui.cursor().min;
    let left = egui::Rect::from_min_size(top, egui::vec2(77.0, 248.0));
    let center = egui::Rect::from_min_size(egui::pos2(left.right() + 2.0, top.y), egui::vec2(143.0, 248.0));
    let right = egui::Rect::from_min_size(egui::pos2(center.right() + 2.0, top.y), egui::vec2(105.0, 248.0));
    let fill = egui::Color32::from_rgb(16, 27, 38);
    let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    for rect in [left, center, right] { ui.painter().rect(rect, 4.0, fill, stroke); }
    ui.painter().text(egui::pos2(left.left() + 6.0, left.top() + 11.0), egui::Align2::LEFT_CENTER, "Effects", egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
    let search = egui::Rect::from_min_size(egui::pos2(left.left() + 6.0, left.top() + 27.0), egui::vec2(65.0, 20.0));
    ui.painter().rect(search, 3.0, egui::Color32::from_rgb(12, 21, 30), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 58, 74)));
    ui.painter().text(egui::pos2(search.left() + 6.0, search.center().y), egui::Align2::LEFT_CENTER, "⌕  Search", egui::FontId::proportional(6.5), colors::TEXT_SECONDARY);
    for (index, (icon, label)) in [(crate::ui::icons::SVG_STAR, "Favorites"), (crate::ui::icons::SVG_CLOCK, "Recent"), (crate::ui::icons::SVG_PALETTE, "Color"), (crate::ui::icons::SVG_LIGHT, "Distort"), (crate::ui::icons::SVG_KEYING, "Keying"), (crate::ui::icons::SVG_CLEAN_PLATE, "Blur"), (crate::ui::icons::SVG_PARTICLES, "Generate"), (crate::ui::icons::SVG_TEMPLATE_TEXT, "Stylize"), (crate::ui::icons::SVG_ARROW_RIGHT, "Transition"), (crate::ui::icons::SVG_LAYERS, "Utility")].into_iter().enumerate() {
        let y = left.top() + 53.0 + index as f32 * 18.0;
        let active = index == 2;
        if active { ui.painter().rect(egui::Rect::from_min_size(egui::pos2(left.left() + 5.0, y - 1.0), egui::vec2(67.0, 17.0)), 3.0, egui::Color32::from_rgb(24, 62, 102), egui::Stroke::NONE); }
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 8.0, y + 3.0), egui::vec2(9.0, 9.0))), |icon_ui| {
            crate::ui::icons::render_svg_bytes(icon_ui, &format!("effects-category-{index}"), icon, egui::vec2(9.0, 9.0), if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
        });
        ui.painter().text(egui::pos2(left.left() + 21.0, y + 7.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(6.5), if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
    }
    ui.painter().text(egui::pos2(center.left() + 6.0, center.top() + 12.0), egui::Align2::LEFT_CENTER, "Lumetri Color", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    for (offset, label, width) in [(31.0, "Fit", 27.0), (61.0, "Full", 31.0)] {
        let button = egui::Rect::from_min_size(egui::pos2(center.left() + offset, center.top() + 5.0), egui::vec2(width, 20.0));
        ui.painter().rect(button, 3.0, egui::Color32::from_rgb(19, 31, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
        ui.painter().text(button.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(6.0), colors::TEXT_PRIMARY);
    }
    if let Some(id) = reference_texture(app, ctx, "recent_project_rift.webp") {
        ui.put(egui::Rect::from_min_size(egui::pos2(center.left() + 2.0, center.top() + 44.0), egui::vec2(135.0, 170.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(135.0, 170.0))).fit_to_exact_size(egui::vec2(135.0, 170.0)));
    }
    ui.painter().text(egui::pos2(center.left() + 6.0, center.top() + 214.0), egui::Align2::LEFT_CENTER, "00:00:00:00", egui::FontId::proportional(6.5), colors::ACCENT_CYAN);
    ui.painter().text(egui::pos2(right.left() + 6.0, right.top() + 11.0), egui::Align2::LEFT_CENTER, "Recent Controls", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    ui.painter().text(egui::pos2(right.left() + 6.0, right.top() + 35.0), egui::Align2::LEFT_CENTER, "Lumetri Color", egui::FontId::proportional(6.5), colors::TEXT_SECONDARY);
    ui.painter().text(egui::pos2(right.right() - 6.0, right.top() + 35.0), egui::Align2::RIGHT_CENTER, "Reset", egui::FontId::proportional(5.5), colors::ACCENT_CYAN);
    for (index, (label, value)) in [("Basic Correction", ""), ("Input LUT", "None"), ("Exposure", "+0.0"), ("Contrast", "+0.0"), ("Highlights", "+0.0"), ("Shadows", "+0.0"), ("Whites", "+0.0"), ("Blacks", "+0.0"), ("Creative", ""), ("Curves", ""), ("Color Wheels & Match", ""), ("HSL Secondary", ""), ("Vignette", "")].into_iter().enumerate() {
        let y = right.top() + 58.0 + index as f32 * 14.0;
        ui.painter().text(egui::pos2(right.left() + 6.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(5.7), colors::TEXT_SECONDARY);
        if value.is_empty() {
            ui.painter().text(egui::pos2(right.right() - 7.0, y), egui::Align2::RIGHT_CENTER, "›", egui::FontId::proportional(8.0), colors::TEXT_MUTED);
        } else {
            ui.painter().text(egui::pos2(right.right() - 6.0, y), egui::Align2::RIGHT_CENTER, value, egui::FontId::proportional(5.7), colors::TEXT_SECONDARY);
        }
    }
    ui.allocate_space(egui::vec2(322.0, 248.0));
}

fn draw_reference_effects_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
        let narrow = reference_narrow(ui);
        let medium = reference_medium(ui);
        if narrow || medium {
            draw_reference_effects_narrow(app, ui, ctx);
            return;
        }
        let effect_names = ["Lumetri Color", "Brightness & Contrast", "Hue/Saturation", "Curves", "Levels", "Color Balance"];
        let selected_effect_name = ctx
            .data_mut(|d| d.get_temp::<usize>(egui::Id::new("reference_effect_selection")))
            .and_then(|index| effect_names.get(index).copied())
            .unwrap_or(effect_names[0]);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 6.0 } else { 10.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 65.0 } else if medium { 150.0 } else { 214.0 });
                ui.label(egui::RichText::new("Effects").size(13.0));
                ui.add_space(6.0);
                let search_width = ui.available_width();
                reference_search_field(ui, &mut app.home_search, "Search effects...", search_width, if narrow { 22.0 } else { 28.0 });
                ui.add_space(6.0);
                let category_id = egui::Id::new("reference_effect_category");
                let mut selected_category = ctx.data_mut(|d| d.get_temp::<usize>(category_id)).unwrap_or(2);
                for (index, label) in ["Favorites", "Recent", "Color", "Distort", "Keying", "Blur & Sharpen", "Generate", "Stylize", "Transition", "Utility"].into_iter().enumerate() {
                    let active = selected_category == index;
                    let row_width = ui.available_width().min(if narrow { ui.available_width() } else { 154.0 });
                    let row_height = if narrow { 20.0 } else { 25.0 };
                    let category_icon = match index {
                        0 => crate::ui::icons::SVG_STAR,
                        1 => crate::ui::icons::SVG_CLOCK,
                        2 => crate::ui::icons::SVG_PALETTE,
                        _ => crate::ui::icons::SVG_CHEVRON_RIGHT,
                    };
                    let clicked = ui.allocate_ui_with_layout(
                        egui::vec2(row_width, row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            egui::Frame::none()
                                .fill(if active { egui::Color32::from_rgb(24, 62, 102) } else { egui::Color32::TRANSPARENT })
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                .show(ui, |ui| {
                                    ui.set_width(row_width - 8.0);
                                    ui.horizontal(|ui| {
                                        crate::ui::icons::render_svg_bytes(ui, &format!("effect-category-{index}"), category_icon, egui::vec2(if narrow { 8.0 } else { 12.0 }, if narrow { 8.0 } else { 12.0 }), if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
                                        ui.add_space(3.0);
                                        ui.label(egui::RichText::new(label).size(if narrow { 7.0 } else { 12.0 }).color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                        },
                    ).inner.clicked();
                    if clicked {
                        selected_category = index;
                    }
                    if index == 2 && selected_category == 2 {
                        let effect_id = egui::Id::new("reference_effect_selection");
                        let mut selected_effect = ctx.data_mut(|d| d.get_temp::<usize>(effect_id)).unwrap_or(0);
                        for (sub_index, sub_label) in ["Lumetri Color", "Brightness & Contrast", "Hue/Saturation", "Curves", "Levels", "Color Balance"].into_iter().enumerate() {
                            let sub_active = selected_effect == sub_index;
                            if ui.add_sized([if narrow { ui.available_width() } else { 146.0 }, if narrow { 18.0 } else { 23.0 }], egui::Button::new(egui::RichText::new(format!("   {sub_label}")).size(if narrow { 6.0 } else { 11.0 }).color(if sub_active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY })).fill(if sub_active { egui::Color32::from_rgb(22, 52, 84) } else { egui::Color32::TRANSPARENT }).rounding(3.0)).clicked() {
                                selected_effect = sub_index;
                            }
                        }
                        ctx.data_mut(|d| d.insert_temp(effect_id, selected_effect));
                    }
                }
                ctx.data_mut(|d| d.insert_temp(category_id, selected_category));
            });
            ui.add_space(if narrow { 4.0 } else { 12.0 });
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 3.0 } else { 12.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 132.0 } else if medium { (ui.available_width() * 0.62).max(260.0) } else { (ui.available_width() * 0.62).max(360.0) });
                ui.label(egui::RichText::new(selected_effect_name).size(if narrow { 10.0 } else { 16.0 }));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    reference_select_button(ui, "Fit", if narrow { 30.0 } else { 54.0 }, narrow);
                    ui.add_space(if narrow { 3.0 } else { 6.0 });
                    reference_select_button(ui, "Full", if narrow { 34.0 } else { 62.0 }, narrow);
                    ui.add_space(if narrow { 3.0 } else { 8.0 });
                    for (index, icon) in [crate::ui::icons::SVG_GRID, crate::ui::icons::SVG_SORT].into_iter().enumerate() {
                        crate::ui::icons::render_svg_bytes(ui, &format!("effect-view-tool-{index}"), icon, egui::vec2(if narrow { 14.0 } else { 18.0 }, if narrow { 14.0 } else { 18.0 }), colors::TEXT_SECONDARY);
                        ui.add_space(if narrow { 3.0 } else { 6.0 });
                    }
                });
                ui.add_space(8.0);
                if let Some(id) = reference_texture(app, ctx, "recent_project_rift.webp") {
                    let image_size = egui::vec2(ui.available_width(), if narrow || compact { 170.0 } else { 280.0 });
                    reference_cover_image(ui, id, image_size, 2.64);
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new("00:00:00:00").small().color(colors::ACCENT_CYAN));
                ui.horizontal(|ui| {
                    for (index, icon) in [crate::ui::icons::SVG_STEP_BACK, crate::ui::icons::SVG_PLAY, crate::ui::icons::SVG_STEP_FORWARD].into_iter().enumerate() {
                        crate::ui::icons::render_svg_bytes(ui, &format!("effect-transport-{index}"), icon, egui::vec2(if narrow { 12.0 } else { 16.0 }, if narrow { 12.0 } else { 16.0 }), colors::TEXT_SECONDARY);
                        ui.add_space(if narrow { 3.0 } else { 6.0 });
                    }
                });
            });
            ui.add_space(if narrow { 4.0 } else { 12.0 });
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                ui.set_width(if narrow { 72.0 } else if medium { 180.0 } else { 230.0 });
                ui.label(egui::RichText::new("Recent Controls").size(if narrow { 9.0 } else { 14.0 }));
                ui.add_space(if narrow { 3.0 } else { 8.0 });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(selected_effect_name).size(if narrow { 7.0 } else { 12.0 }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Reset").size(if narrow { 7.0 } else { 11.0 }).color(colors::ACCENT_CYAN));
                    });
                });
                ui.add_space(if narrow { 3.0 } else { 6.0 });
                ui.separator();
                ui.add_space(if narrow { 3.0 } else { 6.0 });
                ui.horizontal(|ui| {
                    crate::ui::icons::render_svg_bytes(ui, "effect-basic-chevron", crate::ui::icons::SVG_CHEVRON_DOWN, egui::vec2(if narrow { 8.0 } else { 12.0 }, if narrow { 8.0 } else { 12.0 }), colors::TEXT_SECONDARY);
                    ui.label(egui::RichText::new("Basic Correction").size(if narrow { 7.0 } else { 12.0 }).strong());
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Input LUT").size(if narrow { 7.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        reference_select_button(ui, "None", if narrow { 38.0 } else { 94.0 }, narrow);
                    });
                });
                ui.add_space(if narrow { 3.0 } else { 6.0 });
                for (label, default) in [("Exposure", 0.0_f32), ("Contrast", 0.0), ("Highlights", -23.3), ("Shadows", 0.2), ("Whites", 0.0), ("Blacks", -0.2)] {
                    let value_id = egui::Id::new(("reference_effect_value", label));
                    let mut value = ctx.data_mut(|d| d.get_temp::<f32>(value_id)).unwrap_or(default);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(label).size(if narrow { 7.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_sized([if narrow { 48.0 } else { 86.0 }, if narrow { 18.0 } else { 22.0 }], egui::DragValue::new(&mut value).speed(0.1).fixed_decimals(1));
                        });
                    });
                    ctx.data_mut(|d| d.insert_temp(value_id, value));
                    ui.add_space(if narrow { 2.0 } else { 5.0 });
                }
                for (index, label) in ["Creative", "Curves", "Color Wheels & Match", "HSL Secondary", "Vignette"].into_iter().enumerate() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        crate::ui::icons::render_svg_bytes(ui, &format!("effect-section-chevron-{index}"), crate::ui::icons::SVG_CHEVRON_RIGHT, egui::vec2(if narrow { 8.0 } else { 12.0 }, if narrow { 8.0 } else { 12.0 }), colors::TEXT_SECONDARY);
                        ui.label(egui::RichText::new(label).size(if narrow { 7.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                    });
                }
            });
        });
    });
}

fn draw_reference_render_narrow(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let top = ui.cursor().min;
    let queue = egui::Rect::from_min_size(top, egui::vec2(294.0, 143.0));
    let settings = egui::Rect::from_min_size(egui::pos2(top.x, top.y + 147.0), egui::vec2(163.0, 108.0));
    let progress = egui::Rect::from_min_size(egui::pos2(top.x + 167.0, top.y + 147.0), egui::vec2(126.0, 108.0));
    let fill = egui::Color32::from_rgb(16, 27, 38);
    let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    for rect in [queue, settings, progress] { ui.painter().rect(rect, 4.0, fill, stroke); }
    ui.painter().text(egui::pos2(queue.left() + 7.0, queue.top() + 12.0), egui::Align2::LEFT_CENTER, "Render Queue", egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
    let add = egui::Rect::from_min_size(egui::pos2(queue.right() - 126.0, queue.top() + 5.0), egui::vec2(58.0, 19.0));
    ui.painter().rect(add, 3.0, egui::Color32::from_rgb(19, 31, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
    ui.painter().text(add.center(), egui::Align2::CENTER_CENTER, "+ Add to Queue", egui::FontId::proportional(6.0), colors::TEXT_PRIMARY);
    let start = egui::Rect::from_min_size(egui::pos2(queue.right() - 55.0, queue.top() + 5.0), egui::vec2(50.0, 19.0));
    ui.painter().rect(start, 3.0, colors::ACCENT_BLUE, egui::Stroke::NONE);
    ui.painter().text(start.center(), egui::Align2::CENTER_CENTER, "Start Render", egui::FontId::proportional(6.0), egui::Color32::WHITE);
    let header_y = queue.top() + 29.0;
    for (x, label) in [(queue.left() + 28.0, "Composition"), (queue.left() + 108.0, "Settings"), (queue.left() + 201.0, "Output"), (queue.right() - 37.0, "Status")] {
        ui.painter().text(egui::pos2(x, header_y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(5.0), colors::TEXT_MUTED);
    }
    ui.painter().line_segment([egui::pos2(queue.left() + 7.0, queue.top() + 31.0), egui::pos2(queue.right() - 7.0, queue.top() + 31.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(30, 45, 58)));
    for (index, (asset, name, detail)) in [("recent_project_citadel.webp", "main_comp", "H.264 3840×2160 / 24 fps"), ("recent_project_rift.webp", "teaser", "ProRes 422 1920×1080 / 24 fps"), ("recent_project_atlas.webp", "social_vertical", "H.264 1080×1920 / 30 fps")].into_iter().enumerate() {
        let y = queue.top() + 35.0 + index as f32 * 32.0;
        ui.put(egui::Rect::from_min_size(egui::pos2(queue.left() + 8.0, y), egui::vec2(10.0, 22.0)), egui::Label::new(egui::RichText::new(format!("{}", index + 1)).size(6.0).color(colors::TEXT_MUTED)));
        if let Some(id) = reference_texture(app, ctx, asset) { ui.put(egui::Rect::from_min_size(egui::pos2(queue.left() + 28.0, y), egui::vec2(31.0, 21.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(31.0, 21.0))).fit_to_exact_size(egui::vec2(31.0, 21.0))); }
        ui.put(egui::Rect::from_min_size(egui::pos2(queue.left() + 65.0, y), egui::vec2(57.0, 20.0)), egui::Label::new(egui::RichText::new(name).size(7.0)));
        ui.put(egui::Rect::from_min_size(egui::pos2(queue.left() + 124.0, y), egui::vec2(72.0, 20.0)), egui::Label::new(egui::RichText::new(detail).size(5.0).color(colors::TEXT_SECONDARY)));
        ui.painter().text(egui::pos2(queue.left() + 201.0, y + 10.0), egui::Align2::LEFT_CENTER, format!("/renders/{}.mp4", name), egui::FontId::proportional(4.5), colors::TEXT_SECONDARY);
        ui.put(egui::Rect::from_min_size(egui::pos2(queue.right() - 43.0, y), egui::vec2(36.0, 20.0)), egui::Label::new(egui::RichText::new("Queued").size(6.0).color(colors::ACCENT_CYAN)));
    }
    ui.painter().text(egui::pos2(settings.left() + 7.0, settings.top() + 12.0), egui::Align2::LEFT_CENTER, "Render Settings", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    for (index, (label, value)) in [("Format", "H.264"), ("Resolution", "3840 × 2160 (UHD)"), ("Frame Rate", "24 fps"), ("Output Path", "/renders/")].into_iter().enumerate() {
        let y = settings.top() + 31.0 + index as f32 * 18.0;
        ui.painter().text(egui::pos2(settings.left() + 7.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(6.0), colors::TEXT_SECONDARY);
        let field = egui::Rect::from_min_size(egui::pos2(settings.left() + 53.0, y - 7.0), egui::vec2(101.0, 14.0));
        ui.painter().rect(field, 2.0, egui::Color32::from_rgb(20, 32, 44), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(39, 55, 70)));
        ui.painter().text(egui::pos2(field.left() + 5.0, field.center().y), egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(5.5), colors::TEXT_PRIMARY);
        if index < 3 { ui.painter().text(egui::pos2(field.right() - 6.0, field.center().y), egui::Align2::CENTER_CENTER, "⌄", egui::FontId::proportional(7.0), colors::TEXT_MUTED); }
    }
    ui.painter().text(egui::pos2(settings.left() + 7.0, settings.bottom() - 12.0), egui::Align2::LEFT_CENTER, "☑ Open output folder when complete", egui::FontId::proportional(6.0), colors::TEXT_SECONDARY);
    ui.painter().text(egui::pos2(progress.left() + 7.0, progress.top() + 12.0), egui::Align2::LEFT_CENTER, "Progress", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    let ring_center = egui::pos2(progress.center().x, progress.top() + 56.0);
    ui.painter().circle_stroke(ring_center, 22.0, egui::Stroke::new(4.0_f32, egui::Color32::from_rgb(39, 58, 77)));
    ui.painter().text(ring_center, egui::Align2::CENTER_CENTER, "0%", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
    ui.painter().text(egui::pos2(progress.center().x, progress.bottom() - 12.0), egui::Align2::CENTER_CENTER, "No render in progress", egui::FontId::proportional(6.0), colors::TEXT_MUTED);
    ui.allocate_space(egui::vec2(294.0, 260.0));
}

fn draw_reference_render_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
        let narrow = reference_narrow(ui);
        let medium = reference_medium(ui);
        if narrow || medium {
            draw_reference_render_narrow(app, ui, ctx);
            return;
        }
        let selected_id = egui::Id::new("reference_render_selected");
        let mut selected = ctx.data_mut(|d| d.get_temp::<usize>(selected_id)).unwrap_or(0);
        let started_id = egui::Id::new("reference_render_started");
        let mut started = app.export.is_exporting;
        egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(10.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Render Queue").size(if narrow { 12.0 } else { 24.0 }).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new(if started { "Rendering..." } else { "✦ Start Render" }).size(if narrow { 8.0 } else { 12.0 }).color(egui::Color32::WHITE)).fill(colors::ACCENT_BLUE).min_size(egui::vec2(if narrow { 54.0 } else { 0.0 }, if narrow { 20.0 } else { 0.0 })).rounding(5.0)).clicked() && !app.export.is_exporting {
                        let comp_name = app.history.current().active_composition().name.clone();
                        crate::ui::export_dialog::start_comp_export(app, ctx, &comp_name);
                        started = app.export.is_exporting;
                    }
                    ui.add_space(if narrow { 4.0 } else { 8.0 });
                    if ui.add(egui::Button::new(egui::RichText::new("+ Add to Queue").size(if narrow { 8.0 } else { 12.0 })).min_size(egui::vec2(if narrow { 58.0 } else { 0.0 }, if narrow { 20.0 } else { 0.0 })).rounding(5.0)).clicked() {
                        let comp_name = app.history.current().active_composition().name.clone();
                        if !app.render_queue_items.contains(&comp_name) {
                            app.render_queue_items.push(comp_name.clone());
                            app.render_item_status.insert(comp_name, crate::app_state::QueueItemStatus::Queued);
                            app.toasts.info("Composition added to render queue");
                        }
                    }
                });
            });
            ctx.data_mut(|d| d.insert_temp(started_id, started));
            ui.add_space(if narrow { 3.0 } else { 8.0 });
            ui.separator();
            ui.horizontal(|ui| {
                if narrow {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui(egui::vec2(12.0, 18.0), |ui| { ui.label(egui::RichText::new("#").size(8.0).color(colors::TEXT_MUTED)); });
                    ui.add_space(4.0);
                    ui.allocate_exact_size(egui::vec2(28.0, 18.0), egui::Sense::hover());
                    ui.add_space(5.0);
                    ui.allocate_ui(egui::vec2(48.0, 18.0), |ui| { ui.label(egui::RichText::new("Composition").size(8.0).color(colors::TEXT_MUTED)); });
                    ui.add_space(5.0);
                    ui.allocate_ui(egui::vec2(58.0, 18.0), |ui| { ui.label(egui::RichText::new("Settings").size(8.0).color(colors::TEXT_MUTED)); });
                    ui.add_space(5.0);
                    ui.allocate_ui(egui::vec2(54.0, 18.0), |ui| { ui.label(egui::RichText::new("Output").size(8.0).color(colors::TEXT_MUTED)); });
                    ui.add_space(4.0);
                    ui.allocate_ui(egui::vec2(30.0, 18.0), |ui| { ui.label(egui::RichText::new("Status").size(8.0).color(colors::TEXT_MUTED)); });
                } else {
                    ui.label(egui::RichText::new("#").size(10.0).color(colors::TEXT_MUTED));
                    ui.add_space(18.0);
                    ui.label(egui::RichText::new("Composition").size(10.0).color(colors::TEXT_MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Status").size(10.0).color(colors::TEXT_MUTED));
                        ui.add_space(34.0);
                        ui.label(egui::RichText::new("Output").size(10.0).color(colors::TEXT_MUTED));
                        ui.add_space(34.0);
                        ui.label(egui::RichText::new("Settings").size(10.0).color(colors::TEXT_MUTED));
                    });
                }
            });
            ui.add_space(4.0);
            ui.separator();
            let queue_names = if app.render_queue_items.is_empty() {
                vec![app.history.current().active_composition().name.clone()]
            } else {
                app.render_queue_items.clone()
            };
            for (row_index, name) in queue_names.iter().enumerate() {
                let index = (row_index + 1).to_string();
                let asset = ["recent_project_citadel.webp", "recent_project_rift.webp", "recent_project_atlas.webp"][row_index % 3];
                let settings = format!("{} · {} fps", match app.export_format_preset { 1 => "ProRes 422", 2 => "PNG Sequence", _ => "H.264" }, app.export.export_fps);
                let output = if app.export.export_output_path.is_empty() { "output.mp4".to_string() } else { app.export.export_output_path.clone() };
                let status = if app.export.is_exporting && app.export.export_comp_name.as_deref() == Some(name.as_str()) {
                    "Rendering"
                } else {
                    match app.render_item_status.get(name) {
                        Some(crate::app_state::QueueItemStatus::Done) => "Done",
                        Some(crate::app_state::QueueItemStatus::Failed) => "Failed",
                        Some(crate::app_state::QueueItemStatus::Rendering) => "Rendering",
                        _ => "Queued",
                    }
                };
                ui.horizontal(|ui| {
                    let row_font = if narrow { 7.0 } else { 12.0 };
                    if narrow {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.allocate_ui(egui::vec2(12.0, 30.0), |ui| {
                            ui.vertical_centered(|ui| { ui.label(egui::RichText::new(index.as_str()).size(row_font).color(colors::TEXT_MUTED)); });
                        });
                        ui.add_space(4.0);
                        if let Some(id) = reference_texture(app, ctx, asset) {
                            reference_cover_image(ui, id, egui::vec2(28.0, 18.0), 2.64);
                        } else {
                            ui.allocate_exact_size(egui::vec2(28.0, 18.0), egui::Sense::hover());
                        }
                        ui.add_space(5.0);
                        if ui.allocate_ui(egui::vec2(48.0, 30.0), |ui| {
                            ui.vertical_centered(|ui| { ui.add(egui::Label::new(egui::RichText::new(name).size(8.0)).sense(egui::Sense::click())) }).inner
                        }).inner.clicked() {
                            selected = row_index;
                        }
                        ui.add_space(5.0);
                        let mut settings_parts = settings.split('·').map(str::trim);
                        ui.allocate_ui(egui::vec2(58.0, 30.0), |ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(settings_parts.next().unwrap_or(&settings)).size(row_font).color(colors::TEXT_SECONDARY));
                                if let Some(detail) = settings_parts.next() {
                                    ui.label(egui::RichText::new(detail).size(6.0).color(colors::TEXT_SECONDARY));
                                }
                            });
                        });
                        ui.add_space(5.0);
                        ui.allocate_ui(egui::vec2(54.0, 30.0), |ui| {
                            ui.vertical_centered(|ui| { ui.label(egui::RichText::new(output).size(6.0).color(colors::TEXT_SECONDARY)); });
                        });
                        ui.add_space(4.0);
                        let row_status = if started && selected == row_index { "Rendering" } else { status };
                        ui.allocate_ui(egui::vec2(30.0, 30.0), |ui| {
                            ui.vertical_centered(|ui| { ui.label(egui::RichText::new(row_status).size(6.0).color(colors::ACCENT_CYAN)); });
                        });
                        ui.add_space(3.0);
                        crate::ui::icons::render_svg_bytes(ui, &format!("render-row-more-{index}"), crate::ui::icons::SVG_MORE, egui::vec2(10.0, 10.0), colors::TEXT_MUTED);
                    } else {
                        ui.label(egui::RichText::new(index).size(row_font).color(colors::TEXT_MUTED));
                        ui.add_space(6.0);
                        if let Some(id) = reference_texture(app, ctx, asset) {
                            reference_cover_image(ui, id, egui::vec2(42.0, 26.0), 2.64);
                        }
                        ui.add_space(6.0);
                        if ui.add(egui::Label::new(egui::RichText::new(name).size(13.0)).sense(egui::Sense::click())).clicked() {
                            selected = row_index;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let row_status = if started && selected == row_index { "Rendering" } else { status };
                            ui.label(egui::RichText::new(row_status).size(row_font).color(colors::ACCENT_CYAN));
                            ui.add_space(24.0);
                            ui.label(egui::RichText::new(output).size(row_font).color(colors::TEXT_SECONDARY));
                            ui.add_space(24.0);
                            ui.label(egui::RichText::new(settings).size(row_font).color(colors::TEXT_SECONDARY));
                        });
                    }
                });
                if row_index + 1 < queue_names.len() { ui.separator(); }
            }
        });
        ctx.data_mut(|d| d.insert_temp(selected_id, selected));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 6.0 } else { 12.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 141.0 } else if medium { (ui.available_width() * 0.54).max(300.0) } else { (ui.available_width() * 0.54).max(360.0) });
                ui.label(egui::RichText::new("Render Settings").size(if narrow { 10.0 } else { 14.0 }));
                ui.add_space(if narrow { 4.0 } else { 10.0 });
                for (label, default, options) in [("Format", "H.264", ["H.264", "ProRes 422", "PNG Sequence"]), ("Resolution", "3840 × 2160 (4K UHD)", ["3840 × 2160 (4K UHD)", "1920 × 1080 (Full HD)", "1080 × 1920 (Vertical)"]), ("Frame Rate", "24 fps", ["24 fps", "30 fps", "60 fps"])] {
                    let id = egui::Id::new(("reference_render_setting", label));
                    let mut selected_value = ctx.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| default.to_string());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(label).size(if narrow { 8.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt(id).selected_text(&selected_value).width(if narrow { 66.0 } else { 220.0 }).show_ui(ui, |ui| {
                                for option in options {
                                    ui.selectable_value(&mut selected_value, option.to_string(), option);
                                }
                            });
                        });
                    });
                    match label {
                        "Format" => {
                            app.export_format_preset = match selected_value.as_str() {
                                "ProRes 422" => 1,
                                "PNG Sequence" => 2,
                                _ => 0,
                            };
                            app.export_codec_idx = if app.export_format_preset == 1 { 1 } else if app.export_format_preset == 2 { 3 } else { 0 };
                        }
                        "Resolution" => {
                            app.export_resolution_scale = if selected_value.contains("1920") { 1 } else if selected_value.contains("1080 × 1920") { 2 } else { 0 };
                            let scale = match app.export_resolution_scale { 1 => 0.5, 2 => 0.5, _ => 1.0 };
                            ctx.data_mut(|d| d.insert_temp(egui::Id::new("ae_export_res_scale"), scale));
                        }
                        "Frame Rate" => {
                            app.export.export_fps = selected_value.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(24);
                        }
                        _ => {}
                    }
                    ctx.data_mut(|d| d.insert_temp(id, selected_value));
                    ui.add_space(if narrow { 1.0 } else { 5.0 });
                }
                let path_id = egui::Id::new("reference_render_output_path");
                let mut output_path = ctx.data_mut(|d| d.get_temp::<String>(path_id)).unwrap_or_else(|| "/renders/".to_string());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Output Path").size(if narrow { 8.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_sized([if narrow { 38.0 } else { 190.0 }, if narrow { 18.0 } else { 24.0 }], egui::TextEdit::singleline(&mut output_path));
                        reference_icon_button(ui, "render-output-folder", crate::ui::icons::SVG_FOLDER, if narrow { 18.0 } else { 24.0 });
                    });
                });
                app.export.export_output_path = output_path.clone();
                ctx.data_mut(|d| d.insert_temp(path_id, output_path));
                let open_folder_id = egui::Id::new("reference_render_open_folder");
                let mut open_folder = ctx.data_mut(|d| d.get_temp::<bool>(open_folder_id)).unwrap_or(true);
                ui.checkbox(&mut open_folder, egui::RichText::new("Open output folder when complete").size(if narrow { 8.0 } else { 11.0 }));
                ctx.data_mut(|d| d.insert_temp(open_folder_id, open_folder));
            });
            ui.add_space(if narrow { 4.0 } else { 12.0 });
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                ui.set_width(if narrow { 101.0 } else if medium { 170.0 } else { 220.0 });
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Progress").size(if narrow { 9.0 } else { 14.0 }));
                    ui.add_space(if narrow { 3.0 } else { 16.0 });
                    let progress = app.export.export_progress.clamp(0.0, 1.0);
                    let ring_size = if narrow { 62.0 } else { 96.0 };
                    let (ring_rect, _) = ui.allocate_exact_size(egui::vec2(ring_size, ring_size), egui::Sense::hover());
                    let center = ring_rect.center();
                    let radius = ring_size * 0.34;
                    ui.painter().circle_stroke(center, radius, egui::Stroke::new(4.0_f32, egui::Color32::from_rgb(39, 58, 77)));
                    if progress > 0.0 {
                        let start_angle = -std::f32::consts::FRAC_PI_2;
                        let end_angle = start_angle + std::f32::consts::TAU * progress;
                        let segments = 24;
                        for index in 0..segments {
                            let a0 = start_angle + (end_angle - start_angle) * index as f32 / segments as f32;
                            let a1 = start_angle + (end_angle - start_angle) * (index + 1) as f32 / segments as f32;
                            ui.painter().line_segment([center + egui::Vec2::angled(a0) * radius, center + egui::Vec2::angled(a1) * radius], egui::Stroke::new(4.0_f32, colors::ACCENT_BLUE));
                        }
                    }
                    ui.painter().text(center, egui::Align2::CENTER_CENTER, format!("{}%", (progress * 100.0) as u32), egui::FontId::proportional(if narrow { 10.0 } else { 12.0 }), colors::TEXT_PRIMARY);
                    ui.add_space(if narrow { 2.0 } else { 6.0 });
                    ui.label(egui::RichText::new(if app.export.is_exporting { format!("Rendering {}", app.export.export_comp_name.as_deref().unwrap_or("composition")) } else if app.export.export_status.is_some() { "Render finished".to_string() } else { "No render in progress".to_string() }).small().color(colors::TEXT_MUTED));
                });
            });
        });
    });
}

fn draw_reference_templates_narrow(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let top = ui.cursor().min;
    let panel = egui::Rect::from_min_size(top, egui::vec2(314.0, 209.0));
    let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    ui.painter().rect(panel, 4.0, egui::Color32::from_rgb(16, 27, 38), stroke);
    ui.painter().text(egui::pos2(panel.left() + 5.0, panel.top() + 13.0), egui::Align2::LEFT_CENTER, "Project Templates", egui::FontId::proportional(9.0), colors::TEXT_PRIMARY);
    ui.painter().text(egui::pos2(panel.left() + 6.0, panel.top() + 37.0), egui::Align2::LEFT_CENTER, "⌕  Search templates...", egui::FontId::proportional(6.0), colors::TEXT_SECONDARY);
    for (index, label) in ["All", "Film & Video", "Motion Graphics", "VFX", "Social Media"].into_iter().enumerate() {
        let widths = [25.0, 45.0, 54.0, 25.0, 49.0];
        let x = panel.left() + 106.0 + widths[..index].iter().sum::<f32>() + index as f32 * 4.0;
        ui.put(egui::Rect::from_min_size(egui::pos2(x, panel.top() + 27.0), egui::vec2(widths[index], 20.0)), egui::Button::new(egui::RichText::new(label).size(5.5)).fill(if index == 0 { colors::ACCENT_BLUE } else { egui::Color32::TRANSPARENT }));
    }
    let cards = [
        ("templates/cinematic_title.webp", "Cinematic Title", "Film & Video"),
        ("templates/logo_reveal.webp", "Logo Reveal", "Motion Graphics"),
        ("templates/hud_interface.webp", "HUD Interface", "VFX"),
        ("templates/product_showcase.webp", "Product Showcase", "Motion Graphics"),
        ("templates/vfx_breakdown.webp", "VFX Breakdown", "VFX"),
        ("templates/social_media_reel.webp", "Social Media Reel", "Social Media"),
        ("templates/text_animation.webp", "Text Animation", "Motion Graphics"),
        ("templates/clean_plate.webp", "Clean Plate", "VFX"),
    ];
    for (index, (asset, title, category)) in cards.into_iter().enumerate() {
        let card = egui::Rect::from_min_size(egui::pos2(panel.left() + 6.0 + (index % 4) as f32 * 77.0, panel.top() + 52.0 + (index / 4) as f32 * 76.0), egui::vec2(64.0, 70.0));
        ui.painter().rect(card, 4.0, egui::Color32::from_rgb(22, 32, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
        if let Some(id) = reference_texture(app, ctx, asset) { ui.put(egui::Rect::from_min_size(egui::pos2(card.left() + 3.0, card.top() + 3.0), egui::vec2(58.0, 36.0)), egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(58.0, 36.0))).fit_to_exact_size(egui::vec2(58.0, 36.0))); }
        ui.put(egui::Rect::from_min_size(egui::pos2(card.left() + 3.0, card.top() + 42.0), egui::vec2(58.0, 13.0)), egui::Label::new(egui::RichText::new(title).size(5.8).strong()));
        ui.put(egui::Rect::from_min_size(egui::pos2(card.left() + 3.0, card.top() + 55.0), egui::vec2(58.0, 11.0)), egui::Label::new(egui::RichText::new(category).size(5.2).color(colors::TEXT_SECONDARY)));
    }
    ui.allocate_space(egui::vec2(314.0, 209.0));
}

fn draw_reference_templates_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0))
        .show(ui, |ui| {
            draw_reference_topbar(app, ui, ctx);
            ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
            let narrow = reference_narrow(ui);
            if narrow {
                draw_reference_templates_narrow(app, ui, ctx);
                return;
            }
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(16, 27, 38))
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67)))
                .rounding(4.0)
                .inner_margin(egui::Margin::same(if narrow { 3.0 } else { 14.0 }))
                .show(ui, |ui| {
            ui.label(egui::RichText::new("Project Templates").size(if narrow { 10.0 } else { 18.0 }).strong());
            ui.add_space(if narrow { 5.0 } else { 8.0 });
            ui.spacing_mut().item_spacing = egui::vec2(if narrow { 3.0 } else { 8.0 }, 0.0);
            ui.horizontal_wrapped(|ui| {
                let search_width = if narrow { 92.0 } else { 220.0 };
                reference_search_field(ui, &mut app.home_search, "Search templates...", search_width, if narrow { 20.0 } else { 28.0 });
                ui.add_space(6.0);
                let filter_id = egui::Id::new("reference_template_filter");
                let mut selected_filter = ctx.data_mut(|d| d.get_temp::<usize>(filter_id)).unwrap_or(0);
                for (index, label) in ["All", "Film & Video", "Motion Graphics", "VFX", "Social Media"].into_iter().enumerate() {
                    let active = selected_filter == index;
                    let filter_width = if narrow { [22.0, 42.0, 52.0, 22.0, 42.0][index] } else { 0.0 };
                    let filter = egui::Button::new(egui::RichText::new(label).size(if narrow { 7.0 } else { 11.0 }).color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }))
                        .fill(if active { colors::ACCENT_BLUE } else { egui::Color32::TRANSPARENT })
                        .rounding(4.0);
                    let clicked = if narrow {
                        ui.add_sized([filter_width, 20.0], filter).clicked()
                    } else {
                        ui.add(filter).clicked()
                    };
                    if clicked {
                        selected_filter = index;
                    }
                }
                ctx.data_mut(|d| d.insert_temp(filter_id, selected_filter));
            });
            ui.add_space(12.0);
            let selected_filter = ctx.data_mut(|d| d.get_temp::<usize>(egui::Id::new("reference_template_filter")).unwrap_or(0));
            let search_query = app.home_search.clone();
            draw_reference_template_catalog(app, ctx, ui, compact, selected_filter, &search_query);
                });
        });
}

fn draw_reference_settings_narrow(ui: &mut egui::Ui) {
    let top = ui.cursor().min;
    let left = egui::Rect::from_min_size(top, egui::vec2(97.0, 219.0));
    let content = egui::Rect::from_min_size(egui::pos2(top.x + 101.0, top.y), egui::vec2(193.0, 219.0));
    let fill = egui::Color32::from_rgb(16, 27, 38);
    let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67));
    ui.painter().rect(left, 4.0, fill, stroke);
    ui.painter().rect(content, 4.0, fill, stroke);
    let nav = [(crate::ui::icons::SVG_SETTINGS, "General"), (crate::ui::icons::SVG_SORT, "Performance"), (crate::ui::icons::SVG_FOLDER, "Cache"), (crate::ui::icons::SVG_PALETTE, "Color Management"), (crate::ui::icons::SVG_CLOCK, "Auto-save"), (crate::ui::icons::SVG_LAYERS, "UI Appearance"), (crate::ui::icons::SVG_KEYBOARD, "Keyboard Shortcuts"), (crate::ui::icons::SVG_HELP, "Plugins")];
    for (index, (icon, label)) in nav.into_iter().enumerate() {
        let y = left.top() + 5.0 + index as f32 * 20.0;
        if index == 0 { ui.painter().rect(egui::Rect::from_min_size(egui::pos2(left.left() + 5.0, y - 1.0), egui::vec2(87.0, 19.0)), 3.0, egui::Color32::from_rgb(24, 62, 102), egui::Stroke::NONE); }
        let color = if index == 0 { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY };
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(left.left() + 10.0, y + 2.0), egui::vec2(12.0, 12.0))), |icon_ui| {
            crate::ui::icons::render_svg_bytes(icon_ui, &format!("settings-nav-{index}"), icon, egui::vec2(12.0, 12.0), color);
        });
        ui.painter().text(egui::pos2(left.left() + 25.0, y + 8.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(6.5), color);
    }
    let text = |pos: egui::Pos2, value: &str, size: f32, color: egui::Color32| {
        ui.painter().text(pos, egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(size), color);
    };
    let field = |rect: egui::Rect, value: &str| {
        ui.painter().rect(rect, 3.0, egui::Color32::from_rgb(18, 31, 43), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, value, egui::FontId::proportional(6.0), colors::TEXT_PRIMARY);
    };
    text(egui::pos2(content.left() + 8.0, content.top() + 14.0), "General", 10.0, colors::TEXT_PRIMARY);
    text(egui::pos2(content.left() + 8.0, content.top() + 39.0), "Language", 6.0, colors::TEXT_SECONDARY);
    field(egui::Rect::from_min_size(egui::pos2(content.left() + 62.0, content.top() + 28.0), egui::vec2(120.0, 18.0)), "English        ⌄");
    for (index, label) in ["Check for updates automatically", "Send anonymous usage data"].into_iter().enumerate() {
        let y = content.top() + 59.0 + index as f32 * 16.0;
        let checked = index == 0;
        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(content.left() + 8.0, y - 4.0), egui::vec2(8.0, 8.0)), 1.0, if checked { colors::ACCENT_BLUE } else { egui::Color32::from_rgb(39, 53, 67) });
        text(egui::pos2(content.left() + 21.0, y), label, 6.0, colors::TEXT_SECONDARY);
    }
    text(egui::pos2(content.left() + 8.0, content.top() + 108.0), "Project Defaults", 7.0, colors::TEXT_PRIMARY);
    for (index, (label, value)) in [("Default frame rate", "24 fps"), ("Default resolution", "1920 × 1080 (Full HD)"), ("Default color space", "Rec.709")].into_iter().enumerate() {
        let y = content.top() + 127.0 + index as f32 * 17.0;
        text(egui::pos2(content.left() + 8.0, y), label, 6.0, colors::TEXT_SECONDARY);
        field(egui::Rect::from_min_size(egui::pos2(content.left() + 62.0, y - 8.0), egui::vec2(120.0, 16.0)), value);
    }
    text(egui::pos2(content.left() + 8.0, content.top() + 180.0), "File Handling", 7.0, colors::TEXT_PRIMARY);
    for (index, label) in ["Remember last project on startup", "Show startup screen"].into_iter().enumerate() {
        let y = content.top() + 198.0 + index as f32 * 15.0;
        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(content.left() + 8.0, y - 4.0), egui::vec2(8.0, 8.0)), 1.0, colors::ACCENT_BLUE);
        text(egui::pos2(content.left() + 21.0, y), label, 6.0, colors::TEXT_SECONDARY);
    }
    let reset = egui::Rect::from_min_size(egui::pos2(content.right() - 72.0, content.bottom() - 25.0), egui::vec2(65.0, 18.0));
    ui.painter().rect(reset, 3.0, egui::Color32::from_rgb(19, 31, 42), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 60, 76)));
    ui.painter().text(reset.center(), egui::Align2::CENTER_CENTER, "Reset to Defaults", egui::FontId::proportional(5.5), colors::TEXT_PRIMARY);
    ui.allocate_space(egui::vec2(294.0, 219.0));
}

fn draw_reference_settings_page(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact = ui.available_height() < 900.0;
    let narrow = reference_narrow(ui);
    let settings_margin = if narrow { 8.0 } else { reference_content_margin(ui) };
    let prefs_id = egui::Id::new("reference_settings_prefs");
    let initial_prefs = ctx.data_mut(|d| d.get_temp::<crate::ui::preferences_dialog::Prefs>(prefs_id).unwrap_or_else(crate::ui::preferences_dialog::load));
    let mut prefs = initial_prefs.clone();
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(11, 19, 26));
    egui::Frame::none().inner_margin(egui::Margin::symmetric(settings_margin, 0.0)).show(ui, |ui| {
        draw_reference_topbar(app, ui, ctx);
        ui.add_space(if reference_narrow(ui) { 8.0 } else if compact { 22.0 } else { 42.0 });
        let medium = reference_medium(ui);
        if narrow || medium {
            draw_reference_settings_narrow(ui);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
        egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 2.0 } else { 8.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 88.0 } else if medium { 150.0 } else { 174.0 });
                ui.label(egui::RichText::new("SETTINGS").size(10.0).color(colors::TEXT_MUTED));
                ui.add_space(6.0);
                let section_id = egui::Id::new("reference_settings_section");
                let mut selected = ctx.data_mut(|d| d.get_temp::<usize>(section_id)).unwrap_or(0).min(7);
                for (index, (icon, label)) in [
                    (crate::ui::icons::SVG_SETTINGS, "General"),
                    (crate::ui::icons::SVG_GPU, "Performance"),
                    (crate::ui::icons::SVG_FOLDER, "Cache"),
                    (crate::ui::icons::SVG_PALETTE, "Color Management"),
                    (crate::ui::icons::SVG_CLOCK, "Auto-save"),
                    (crate::ui::icons::SVG_SCREEN, "UI Appearance"),
                    (crate::ui::icons::SVG_KEYBOARD, "Keyboard Shortcuts"),
                    (crate::ui::icons::SVG_LAYERS, "Plugins"),
                ].into_iter().enumerate() {
                    let icon_size = if narrow { 13.0 } else { 15.0 };
                    let row_height = if narrow { 21.0 } else { 27.0 };
                    let row_width = ui.available_width();
                    let clicked = ui.allocate_ui_with_layout(
                        egui::vec2(row_width, row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            egui::Frame::none()
                                .fill(if selected == index { egui::Color32::from_rgb(24, 62, 102) } else { egui::Color32::TRANSPARENT })
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                .show(ui, |ui| {
                                    ui.set_width(row_width - 8.0);
                                    ui.horizontal(|ui| {
                                        crate::ui::icons::render_svg_bytes(ui, label, icon, egui::vec2(icon_size, icon_size), if selected == index { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
                                        ui.add_space(5.0);
                                        ui.label(egui::RichText::new(label).size(if narrow { 7.0 } else { 11.0 }).color(if selected == index { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                        },
                    ).inner.clicked();
                    if clicked {
                        selected = index;
                    }
                }
                ctx.data_mut(|d| d.insert_temp(section_id, selected));
            });
            ui.add_space(if narrow { 6.0 } else { 12.0 });
            let selected_section = ctx.data_mut(|d| d.get_temp::<usize>(egui::Id::new("reference_settings_section")).unwrap_or(0).min(7));
            let section_names = ["General", "Performance", "Cache", "Color Management", "Auto-save", "UI Appearance", "Keyboard Shortcuts", "Plugins"];
            let section_descriptions = [
                "Application language, startup behavior, and project defaults.",
                "Preview quality, playback performance, and GPU preferences.",
                "Disk cache location, size limits, and cleanup behavior.",
                "Color profiles, display transforms, and working space.",
                "Automatic project save intervals and recovery behavior.",
                "Interface density, theme, and panel appearance.",
                "Keyboard bindings and shortcut customization.",
                "Installed extensions and plugin management.",
            ];
            egui::Frame::none().fill(egui::Color32::from_rgb(16, 27, 38)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(37, 52, 67))).rounding(4.0).inner_margin(egui::Margin::same(if narrow { 2.0 } else { 16.0 })).show(ui, |ui| {
                ui.set_width(if narrow { 204.0 } else { (ui.available_width() - 12.0).max(if medium { 320.0 } else { 420.0 }) });
                ui.label(egui::RichText::new(section_names[selected_section]).size(if narrow { 12.0 } else { 16.0 }).strong());
                if !narrow {
                    ui.label(egui::RichText::new(section_descriptions[selected_section]).small().color(colors::TEXT_MUTED));
                }
                if selected_section != 0 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(12.0);
                    match selected_section {
                        1 => {
                            ui.label(egui::RichText::new("Preview Quality").size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                            ui.checkbox(&mut prefs.adaptive_preview, "Adaptive preview quality");
                            ui.label(egui::RichText::new(format!("Frame cache budget: {} MB", prefs.cache_mb)).size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                            ui.add(egui::Slider::new(&mut prefs.cache_mb, 128..=2048).step_by(64.0).suffix(" MB"));
                        }
                        2 => {
                            ui.label(egui::RichText::new("Disk Cache").size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                            ui.label(egui::RichText::new(format!("Maximum disk cache: {} GB", prefs.disk_cache_gb)).size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                            ui.add(egui::Slider::new(&mut prefs.disk_cache_gb, 10..=500).suffix(" GB"));
                        }
                        4 => {
                            ui.label(egui::RichText::new("Automatic Save").size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                            ui.label(egui::RichText::new(format!("Save interval: {} seconds", prefs.autosave_secs)).size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                            ui.add(egui::Slider::new(&mut prefs.autosave_secs, 5..=600).suffix(" s"));
                        }
                        5 => {
                            ui.label(egui::RichText::new("Interface").size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY));
                            ui.label(egui::RichText::new("Dark theme").size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_PRIMARY));
                            ui.label(egui::RichText::new("Kagari VFX uses the dark production workspace.").small().color(colors::TEXT_MUTED));
                        }
                        _ => {
                            for (label, value) in [("Enabled", "On"), ("Quality", "Balanced"), ("Location", "Default"), ("Limit", "Automatic")] {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(label).size(12.0).color(colors::TEXT_SECONDARY));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        reference_select_button(ui, value, if narrow { 118.0 } else { 220.0 }, narrow);
                                    });
                                });
                                ui.add_space(7.0);
                            }
                        }
                    }
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Changes are saved automatically").small().color(colors::TEXT_MUTED));
                } else {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Language").size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY));
                ui.horizontal(|ui| { reference_select_button(ui, "English", ui.available_width() - 8.0, narrow); });
                let updates_id = egui::Id::new("reference_settings_updates");
                let anonymous_id = egui::Id::new("reference_settings_anonymous");
                let startup_id = egui::Id::new("reference_settings_startup");
                let splash_id = egui::Id::new("reference_settings_splash");
                let mut updates = ctx.data_mut(|d| d.get_temp::<bool>(updates_id)).unwrap_or(true);
                let mut anonymous = ctx.data_mut(|d| d.get_temp::<bool>(anonymous_id)).unwrap_or(false);
                let mut startup = ctx.data_mut(|d| d.get_temp::<bool>(startup_id)).unwrap_or(true);
                let mut splash = ctx.data_mut(|d| d.get_temp::<bool>(splash_id)).unwrap_or(true);
                ui.checkbox(&mut updates, egui::RichText::new("Check for updates automatically").size(if narrow { 8.0 } else { 11.0 }));
                ui.checkbox(&mut anonymous, egui::RichText::new("Send anonymous usage data").size(if narrow { 8.0 } else { 11.0 }));
                ui.add_space(if narrow { 5.0 } else { 12.0 });
                ui.label(egui::RichText::new("Project Defaults").size(if narrow { 9.0 } else { 12.0 }).color(colors::TEXT_PRIMARY));
                ui.add_space(if narrow { 3.0 } else { 6.0 });
                for (label, value) in [("Default frame rate", "24 fps"), ("Default resolution", "1920 × 1080 (Full HD)"), ("Default color space", "Rec.709")] {
                    ui.horizontal(|ui| { ui.label(egui::RichText::new(label).size(if narrow { 8.0 } else { 11.0 }).color(colors::TEXT_SECONDARY)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { reference_select_button(ui, value, if narrow { 118.0 } else { 220.0 }, narrow); }); });
                    ui.add_space(if narrow { 2.0 } else { 5.0 });
                }
                ui.add_space(6.0);
                ui.checkbox(&mut startup, egui::RichText::new("Remember last project on startup").size(if narrow { 8.0 } else { 11.0 }));
                ui.checkbox(&mut splash, egui::RichText::new("Show startup screen").size(if narrow { 8.0 } else { 11.0 }));
                ui.add_space(if narrow { 4.0 } else { 10.0 });
                ui.horizontal(|ui| { ui.label(egui::RichText::new("Settings are saved automatically").small().color(colors::TEXT_MUTED)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.add(egui::Button::new("Reset to Defaults").rounding(4.0)).clicked() { crate::ui::preferences_dialog::reset_to_defaults(app); } }); });
                ctx.data_mut(|d| { d.insert_temp(updates_id, updates); d.insert_temp(anonymous_id, anonymous); d.insert_temp(startup_id, startup); d.insert_temp(splash_id, splash); });
                }
            });
        });
    });
    if prefs != initial_prefs {
        crate::ui::preferences_dialog::apply(app, &prefs);
        crate::ui::preferences_dialog::save(&prefs);
    }
    ctx.data_mut(|d| d.insert_temp(prefs_id, prefs));
}

fn draw_reference_template_catalog(app: &mut KagariApp, ctx: &egui::Context, ui: &mut egui::Ui, compact: bool, selected_filter: usize, query: &str) {
    let narrow = reference_narrow(ui);
    let gap = if narrow { 6.0 } else { 10.0 };
    let columns = 4.0;
    let width = ((ui.available_width() - gap * (columns - 1.0)) / columns).max(if narrow { 54.0 } else { 130.0 });
    let height = if narrow { 68.0 } else if compact { 124.0 } else { 154.0 };
    let icon_height = if narrow { 36.0 } else if compact { 58.0 } else { 78.0 };
    let cards = [
        ("templates/cinematic_title.webp", "Cinematic Title", "Film & Video"),
        ("templates/logo_reveal.webp", "Logo Reveal", "Motion Graphics"),
        ("templates/hud_interface.webp", "HUD Interface", "VFX"),
        ("templates/product_showcase.webp", "Product Showcase", "Motion Graphics"),
        ("templates/vfx_breakdown.webp", "VFX Breakdown", "VFX"),
        ("templates/social_media_reel.webp", "Social Media Reel", "Social Media"),
        ("templates/text_animation.webp", "Text Animation", "Motion Graphics"),
        ("templates/clean_plate.webp", "Clean Plate", "VFX"),
    ];
    ui.spacing_mut().item_spacing = egui::vec2(0.0, if narrow { 12.0 } else { 10.0 });
    let normalized_query = query.trim().to_ascii_lowercase();
    let matches_filter = |title: &str, category: &str| {
        let category_index = match category {
            "Film & Video" => 1,
            "Motion Graphics" => 2,
            "VFX" => 3,
            "Social Media" => 4,
            _ => 0,
        };
        (normalized_query.is_empty() || format!("{title} {category}").to_ascii_lowercase().contains(&normalized_query))
            && (selected_filter == 0 || selected_filter == category_index)
    };
    let visible_template_count = cards.iter().filter(|(_, title, category)| matches_filter(title, category)).count();
    let mut rendered_templates = 0_usize;
    ui.horizontal_wrapped(|ui| {
        for (asset, title, category) in cards {
            if !matches_filter(title, category) {
                continue;
            }
            rendered_templates += 1;
            fixed_card_with_inset(ui, egui::vec2(width, height), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 5.0, if narrow { 2.0 } else { 14.0 }, |ui| {
                ui.vertical(|ui| {
                    if narrow {
                        ui.spacing_mut().item_spacing.y = 0.0;
                    }
                    ui.horizontal_centered(|ui| {
                        let image_size = egui::vec2((width - if narrow { 4.0 } else { 28.0 }).max(10.0), icon_height);
                        if let Some(id) = reference_texture(app, ctx, asset) {
                            reference_cover_image(ui, id, image_size, 70.0 / 40.0);
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(title).size(if narrow { 7.0 } else { 12.0 }).strong());
                    ui.label(egui::RichText::new(category).size(if narrow { 7.0 } else { 10.5 }).color(colors::TEXT_SECONDARY));
                });
            });
            if rendered_templates < visible_template_count {
                ui.add_space(gap);
            }
        }
    });
    if visible_template_count == 0 {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("No templates found").size(14.0).color(colors::TEXT_PRIMARY));
            ui.label(egui::RichText::new("Try another category or search term.").small().color(colors::TEXT_MUTED));
        });
    }
}

fn draw_reference_nav(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let compact_brand = ui.available_width() < 130.0;
    ui.add_space(if compact_brand { 22.0 } else { 20.0 });
    if app.home_banner.is_none() {
        if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
            app.home_banner = load_logo_texture(ctx, img);
        }
    }
    ui.horizontal(|ui| {
        ui.add_space(if compact_brand { 18.0 } else { 22.0 });
        if let Some(texture) = app.home_banner.as_ref() {
            let logo_size = if compact_brand { 56.0 } else { 68.0 };
            ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(logo_size, logo_size))));
        }
    });
    ui.add_space(-5.0);
    if compact_brand {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Kagari VFX").size(16.0).color(colors::TEXT_PRIMARY));
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.label(egui::RichText::new("Kagari VFX").size(17.0).color(colors::TEXT_PRIMARY));
        });
    }
    ui.add_space(if compact_brand { 31.0 } else { 37.0 });
    let current = home_nav(ctx);
    let compact_sidebar = ui.available_width() < 130.0;
    for (nav, icon, label) in [(HomeNav::Home, crate::ui::icons::SVG_HOME, "Home"), (HomeNav::Projects, crate::ui::icons::SVG_FOLDER, "Projects"), (HomeNav::Templates, crate::ui::icons::SVG_LAYERS, "Templates")] {
        let active = current == nav;
        let (row_rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), egui::Sense::click());
        if active {
            ui.painter().rect_filled(row_rect.shrink2(egui::vec2(0.0, 1.0)), 6.0, egui::Color32::from_rgb(24, 62, 102));
        }
        let icon_size = if compact_sidebar { 18.0 } else { 20.0 };
        crate::ui::icons::render_svg_at(
            ui,
            format!("reference-sidebar-{label}"),
            icon,
            egui::vec2(icon_size, icon_size),
            if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY },
            egui::pos2(row_rect.left() + if compact_sidebar { 12.0 } else { 18.0 }, row_rect.center().y - icon_size * 0.5),
        );
        let label_left = row_rect.left() + if compact_sidebar { 40.0 } else { 54.0 };
        ui.put(
            egui::Rect::from_min_max(egui::pos2(label_left, row_rect.top()), egui::pos2(row_rect.right() - 10.0, row_rect.bottom())),
            egui::Label::new(egui::RichText::new(label).size(13.0).color(if active { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY })).truncate(),
        );
        if response.clicked() { set_home_nav(ctx, nav); }
        ui.add_space(3.0);
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        let footer_bottom_space = if compact_brand {
            (ui.available_height() * 0.038).clamp(14.0, 39.0)
        } else {
            (ui.available_height() * 0.038 + 12.0).clamp(14.0, 51.0)
        };
        let footer_item_gap = (ui.available_height() * 0.007).clamp(6.0, 10.0);
        let footer_divider_gap = (ui.available_height() * 0.025).clamp(14.0, 26.0);
        ui.add_space(footer_bottom_space);
        let help_clicked = ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 28.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_space(23.0);
                crate::ui::icons::render_svg_bytes(ui, "home-help", crate::ui::icons::SVG_HELP, egui::vec2(28.0, 28.0), colors::TEXT_SECONDARY);
            },
        ).response.interact(egui::Sense::click()).clicked();
        if help_clicked {
            app.show_shortcuts_dialog = true;
        }
        ui.add_space(footer_item_gap);
        let settings_clicked = ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 28.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_space(23.0);
                crate::ui::icons::render_svg_bytes(ui, "home-settings", crate::ui::icons::SVG_SETTINGS, egui::vec2(28.0, 28.0), colors::TEXT_SECONDARY);
            },
        ).response.interact(egui::Sense::click()).clicked();
        if settings_clicked {
            set_home_nav(ctx, HomeNav::Settings);
        }
        ui.add_space(footer_divider_gap);
        ui.horizontal(|ui| {
            ui.add_space(21.0);
            let line_width = ui.available_width().clamp(1.0, 95.0);
            let (line_rect, _) = ui.allocate_exact_size(egui::vec2(line_width, 1.0), egui::Sense::hover());
            ui.painter().line_segment([line_rect.left_top(), line_rect.right_top()], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(47, 63, 78)));
        });
        ui.add_space(10.0);
    });

}

fn home_asset_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/home")
        .join(name)
}

fn reference_texture(app: &mut KagariApp, ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
    if name == "logo.png" {
        if app.home_banner.is_none() {
            if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        return app.home_banner.as_ref().map(egui::TextureHandle::id);
    }
    let path = if let Some(asset) = name.strip_prefix("templates/") {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/templates").join(asset)
    } else {
        home_asset_path(name)
    };
    thumb_for(app, ctx, &path)
}

fn selected_asset_name(ctx: &egui::Context) -> String {
    ctx.data_mut(|d| d.get_temp::<String>(egui::Id::new("reference_asset_selection")))
        .unwrap_or_else(|| "city_01.mp4".to_string())
}

fn asset_thumbnail_for_name(name: &str) -> &'static str {
    match name {
        "city_01.mp4" => "recent_project_citadel.webp",
        "mountain_bg.jpg" | "glass_texture.png" | "four_red.jpeg" => "recent_project_rift.webp",
        "clouds.mov" => "continue_working_preview.webp",
        "stroke_01.mov" | "particles.mov" => "recent_project_atlas.webp",
        "logo.png" => "logo.png",
        "light_leak.mov" => "recent_project_eclipse.webp",
        _ => "recent_project_citadel.webp",
    }
}

fn asset_metadata_for_name(name: &str) -> &'static [(&'static str, &'static str)] {
    match name {
        "mountain_bg.jpg" => &[("Frame Size", "3840 × 2160"), ("Format", "JPEG"), ("Color", "Rec.709"), ("File Size", "18 MB")],
        "clouds.mov" => &[("Frame Rate", "24 fps"), ("Frame Size", "3840 × 2160"), ("Duration", "00:09"), ("Codec", "ProRes 422"), ("Audio", "48 kHz")],
        "glass_texture.png" => &[("Frame Size", "4096 × 4096"), ("Format", "PNG"), ("Color", "RGBA"), ("File Size", "12 MB")],
        "logo.png" => &[("Frame Size", "1024 × 1024"), ("Format", "PNG"), ("Color", "RGBA"), ("File Size", "2 MB")],
        "four_red.jpeg" => &[("Frame Size", "3840 × 2160"), ("Format", "JPEG"), ("Color", "Rec.709"), ("File Size", "24 MB")],
        "stroke_01.mov" | "light_leak.mov" | "particles.mov" => &[("Frame Rate", "24 fps"), ("Frame Size", "1920 × 1080"), ("Duration", "00:05"), ("Codec", "H.264"), ("File Size", "86 MB")],
        _ => &[("Frame Rate", "24 fps"), ("Frame Size", "3840 × 2160"), ("Duration", "00:12"), ("Codec", "H.264"), ("File Size", "428 MB"), ("Audio", "48 kHz")],
    }
}

fn reference_narrow(ui: &egui::Ui) -> bool {
    ui.available_width() < 380.0
}

fn reference_medium(ui: &egui::Ui) -> bool {
    let width = ui.available_width();
    (380.0..760.0).contains(&width)
}

fn reference_content_margin(ui: &egui::Ui) -> f32 {
    // The inner content is narrower by both side margins; keep the breakpoint
    // aligned with reference_narrow after the margin is applied.
    if ui.available_width() < 442.0 { 4.0 } else { 31.0 }
}

fn ref_card(ui: &mut egui::Ui, fill: egui::Color32, stroke: egui::Color32, radius: f32, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let frame = egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0_f32, stroke))
        .rounding(radius)
        .inner_margin(egui::Margin::same(14.0));
    frame.show(ui, add).response
}

fn fixed_card(ui: &mut egui::Ui, size: egui::Vec2, fill: egui::Color32, stroke: egui::Color32, radius: f32, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    fixed_card_with_inset(ui, size, fill, stroke, radius, 20.0, add)
}

fn fixed_card_with_inset(ui: &mut egui::Ui, size: egui::Vec2, fill: egui::Color32, stroke: egui::Color32, radius: f32, inset: f32, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(rect, radius, egui::Stroke::new(1.0_f32, if response.hovered() { colors::ACCENT_BLUE } else { stroke }));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(inset)).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(rect.shrink(1.0));
    add(&mut child);
    response
}

fn reference_action_button(
    ui: &mut egui::Ui,
    icon: &'static str,
    title: &str,
    subtitle: &str,
    accent: bool,
    height: f32,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let fill = if accent { egui::Color32::from_rgb(24, 92, 180) } else { egui::Color32::from_rgb(28, 39, 51) };
    ui.painter().rect_filled(rect, 6.0, fill);
    let stroke = if accent { colors::ACCENT_BLUE } else { egui::Color32::from_rgb(45, 62, 79) };
    ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(1.0_f32, stroke));
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(12.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
    inner.spacing_mut().item_spacing.x = 10.0;
    let icon_size = if height >= 80.0 { 32.0 } else { 24.0 };
    crate::ui::icons::render_svg_bytes(&mut inner, title, icon, egui::vec2(icon_size, icon_size), egui::Color32::from_rgb(220, 230, 244));
    inner.vertical(|ui| {
        ui.label(egui::RichText::new(title).size(if height >= 80.0 { 14.0 } else { 11.0 }).strong());
        if !subtitle.is_empty() {
            ui.label(egui::RichText::new(subtitle).size(if height >= 80.0 { 12.0 } else { 9.0 }).color(colors::TEXT_SECONDARY));
        }
    });
    inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(ui, &format!("{title}-arrow"), crate::ui::icons::SVG_ARROW_RIGHT, egui::vec2(15.0, 15.0), egui::Color32::from_rgb(190, 204, 226));
    });
    response
}

fn reference_cta_button(ui: &mut egui::Ui, title: &str, compact: bool) -> egui::Response {
    let width = ui.available_width();
    let height = if compact { 24.0 } else { 43.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    ui.painter().rect_filled(rect, 6.0, egui::Color32::from_rgb(22, 118, 239));
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(10.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
    inner.label(egui::RichText::new(title).size(if compact { 12.0 } else { 14.0 }).strong());
    inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(ui, "cta-arrow", crate::ui::icons::SVG_ARROW_RIGHT, egui::vec2(17.0, 17.0), egui::Color32::WHITE);
    });
    response
}

fn reference_view_all_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.allocate_ui_with_layout(egui::vec2(82.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        let clicked = ui.scope(|ui| {
            ui.spacing_mut().button_padding.x = 4.0;
            ui.add(egui::Button::new(egui::RichText::new(label).color(colors::TEXT_SECONDARY)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE).rounding(4.0))
        }).inner.clicked();
        ui.add_space(2.0);
        let arrow_clicked = crate::ui::icons::render_svg_bytes(ui, "view-all-arrow", crate::ui::icons::SVG_ARROW_RIGHT, egui::vec2(15.0, 15.0), colors::TEXT_SECONDARY)
            .interact(egui::Sense::click())
            .clicked();
        clicked || arrow_clicked
    }).inner
}

fn reference_icon_button(ui: &mut egui::Ui, id: &str, icon: &'static str, size: f32) -> egui::Response {
    let inner = ui.allocate_ui(egui::vec2(size, size), |ui| {
        crate::ui::icons::render_svg_bytes(ui, id, icon, egui::vec2(size - 7.0, size - 7.0), colors::TEXT_SECONDARY)
    });
    inner.inner.interact(egui::Sense::click())
}

fn reference_search_field(ui: &mut egui::Ui, value: &mut String, hint: &str, width: f32, height: f32) {
    let font_size = if width < 120.0 { 7.0 } else if width < 260.0 { 11.0 } else { 12.0 };
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(12, 21, 30))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 58, 74)))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
        .show(ui, |ui| {
            let inner_width = (width - 12.0).max(1.0);
            ui.set_width(inner_width);
            ui.set_min_height(height);
            ui.horizontal(|ui| {
                crate::ui::icons::render_svg_bytes(ui, "reference-search", crate::ui::icons::SVG_SEARCH, egui::vec2(13.0, 13.0), colors::TEXT_SECONDARY);
                ui.add(egui::TextEdit::singleline(value).hint_text(hint).font(egui::FontId::proportional(font_size)).frame(false).desired_width((inner_width - 13.0).max(1.0)));
            });
        });
}

fn reference_asset_search_field(ui: &mut egui::Ui, value: &mut String, hint: &str, width: f32, height: f32) {
    let narrow = width < 120.0;
    let font_size = if width < 120.0 { 7.0 } else if width < 260.0 { 11.0 } else { 12.0 };
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(12, 21, 30))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 58, 74)))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
        .show(ui, |ui| {
            let inner_width = (width - 12.0).max(1.0);
            ui.set_width(inner_width);
            ui.set_min_height(height);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = if narrow { 2.0 } else { 8.0 };
                let icon_size = if narrow { 9.0 } else { 13.0 };
                let filter_size = if narrow { 7.0 } else { 9.0 };
                crate::ui::icons::render_svg_bytes(ui, "reference-asset-search", crate::ui::icons::SVG_SEARCH, egui::vec2(icon_size, icon_size), colors::TEXT_SECONDARY);
                ui.add(egui::TextEdit::singleline(value).hint_text(hint).font(egui::FontId::proportional(font_size)).frame(false).desired_width((inner_width - icon_size - filter_size - if narrow { 6.0 } else { 16.0 }).max(1.0)));
                crate::ui::icons::render_svg_bytes(ui, "reference-asset-search-filter", crate::ui::icons::SVG_CHEVRON_DOWN, egui::vec2(filter_size, filter_size), colors::TEXT_SECONDARY);
            });
        });
}

fn reference_select_button(ui: &mut egui::Ui, label: &str, width: f32, compact: bool) -> egui::Response {
    let height = if compact { 20.0 } else { 24.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(24, 35, 47));
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 58, 74)));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(6.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.label(egui::RichText::new(label).size(if compact { 8.5 } else { 11.0 }).color(colors::TEXT_PRIMARY));
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(ui, "reference-select-chevron", crate::ui::icons::SVG_CHEVRON_DOWN, egui::vec2(11.0, 11.0), colors::TEXT_SECONDARY);
    });
    response
}

fn reference_cover_image(ui: &mut egui::Ui, id: egui::TextureId, size: egui::Vec2, source_aspect: f32) {
    let target_aspect = size.x / size.y.max(1.0);
    let (left, right, top, bottom) = if target_aspect < source_aspect {
        let visible_u = (target_aspect / source_aspect).min(1.0);
        let left = (1.0 - visible_u) * 0.5;
        (left, left + visible_u, 0.0, 1.0)
    } else {
        let visible_v = (source_aspect / target_aspect).min(1.0);
        let top = (1.0 - visible_v) * 0.5;
        (0.0, 1.0, top, top + visible_v)
    };
    ui.add(
        egui::Image::new(egui::load::SizedTexture::new(id, size))
            .uv(egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom)))
            .fit_to_exact_size(size)
            .rounding(egui::Rounding::same(5.0))
            .maintain_aspect_ratio(false),
    );
}

fn reference_contain_image(ui: &mut egui::Ui, id: egui::TextureId, size: egui::Vec2, source_aspect: f32) {
    let target_aspect = size.x / size.y.max(1.0);
    let draw_size = if target_aspect > source_aspect {
        egui::vec2(size.y * source_aspect, size.y)
    } else {
        egui::vec2(size.x, size.x / source_aspect.max(0.01))
    };
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 5.0, egui::Color32::from_rgb(7, 12, 18));
    let image_rect = egui::Rect::from_center_size(rect.center(), draw_size);
    ui.put(
        image_rect,
        egui::Image::new(egui::load::SizedTexture::new(id, draw_size))
            .fit_to_exact_size(draw_size)
            .rounding(egui::Rounding::same(5.0)),
    );
}

fn draw_reference_home(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let bg = egui::Color32::from_rgb(11, 19, 26);
    let compact = ui.available_height() < 900.0;
    let top_space = if compact { 28.0 } else { 40.0 };
    let mobile = ui.available_width() < 520.0;
    let base_first_row_height = if mobile { 360.0 } else if compact { 200.0 } else { 266.0 };
    let preview_height = if compact { 130.0 } else { 191.0 };
    ui.painter().rect_filled(ui.max_rect(), 0.0, bg);
    let home_margin = if ui.available_width() >= 900.0 {
        egui::Margin { left: 35.0, right: 29.0, top: 0.0, bottom: 0.0 }
    } else {
        egui::Margin::symmetric(reference_content_margin(ui), 0.0)
    };
    egui::Frame::none().inner_margin(home_margin).show(ui, |ui| {
    ui.add_space(if compact { 12.0 } else { 24.0 });
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 46.0),
            egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
        let profile_size = if mobile { 26.0 } else if compact { 30.0 } else { 34.0 };
        crate::ui::icons::render_svg_bytes(ui, "home-profile", crate::ui::icons::SVG_PROFILE, egui::vec2(profile_size, profile_size), egui::Color32::WHITE);
        ui.add_space(if compact { 12.0 } else { 35.0 });
        let desired_search_width: f32 = if mobile { 220.0 } else if compact { 300.0 } else { 418.0 };
        let search_width = desired_search_width.min((ui.available_width() - 52.0).max(1.0));
        ui.allocate_ui(egui::vec2(search_width, 46.0), |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(14, 23, 32))
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(49, 67, 84)))
                .rounding(7.0)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let search_icon_size = if mobile { 15.0 } else { 18.0 };
                        let search_font_size = if mobile { 11.0 } else { 14.0 };
                        crate::ui::icons::render_svg_bytes(ui, "home-search", crate::ui::icons::SVG_SEARCH, egui::vec2(search_icon_size, search_icon_size), egui::Color32::from_rgb(172, 188, 211));
                        ui.add(egui::TextEdit::singleline(&mut app.home_search).hint_text("Search projects, templates...").font(egui::FontId::proportional(search_font_size)).frame(false).desired_width((search_width - if mobile { 48.0 } else { 54.0 }).max(1.0)));
                    });
                });
        });
        },
    );
    ui.add_space(top_space);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(0.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Pick up where you left off.").size(if mobile { 20.0 } else { 28.0 }).strong().color(egui::Color32::from_rgb(188, 201, 232)));
        });
        ui.add_space(10.0);
        let row_width = ui.available_width();
        let stacked = row_width < 920.0;
        let first_row_height = if stacked && row_width < 560.0 {
            if compact { 300.0 } else { 382.0 }
        } else {
            base_first_row_height
        };
        let first_row_gap = if compact { 12.0 } else { 18.0 };
        let continue_width = if stacked {
            row_width
        } else {
            (row_width * 0.571).min(row_width - first_row_gap - 330.0)
        };
        let start_width = if stacked {
            row_width
        } else {
            (row_width - continue_width - first_row_gap).max(330.0)
        };
        let draw_home_first_row = |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let width = continue_width;
            fixed_card(ui, egui::vec2(width, first_row_height), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 8.0, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Continue Working").size(20.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        crate::ui::icons::render_svg_bytes(ui, "continue-more", crate::ui::icons::SVG_MORE, egui::vec2(18.0, 18.0), egui::Color32::from_rgb(170, 188, 216));
                    });
                });
                ui.add_space(if compact { 8.0 } else { 12.0 });
                draw_reference_continue_body(app, ui, ctx, compact, width < 560.0, preview_height);
            });
            if stacked {
                ui.end_row();
                ui.add_space(10.0);
            } else {
                ui.add_space(first_row_gap);
            }
            let start_height = if stacked && row_width < 560.0 { 214.0 } else { first_row_height };
            fixed_card(ui, egui::vec2(start_width, start_height), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 8.0, |ui| {
                ui.label(egui::RichText::new("Start New").size(18.0).strong());
                ui.add_space(12.0);
                let action_height: f32 = if compact { 60.0 } else { 92.0 };
                if ui.available_width() < 260.0 {
                    for (icon, title, subtitle, action) in [
                        (crate::ui::icons::SVG_FILE, "New Project", "Start from scratch", 0_u8),
                        (crate::ui::icons::SVG_OPEN_FOLDER, "Open Project", "Browse files", 1_u8),
                        (crate::ui::icons::SVG_COMPOSITION, "New Composition", "Create a comp", 2_u8),
                        (crate::ui::icons::SVG_IMPORT, "Import Footage", "Add media", 3_u8),
                    ] {
                        let clicked = reference_action_button(ui, icon, title, subtitle, action == 0, action_height.min(62.0)).clicked();
                        if clicked {
                            match action {
                                0 => set_home_nav(ui.ctx(), HomeNav::NewProject),
                                1 => enter_studio_open_dialog(app),
                                2 => new_composition(app),
                                _ => import_footage_dialog(app),
                            }
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    let action_gap = 14.0;
                    ui.horizontal(|ui| {
                        let button_width = ((ui.available_width() - action_gap) / 2.0).max(72.0);
                        if ui.allocate_ui(egui::vec2(button_width, action_height), |ui| reference_action_button(ui, crate::ui::icons::SVG_FILE, "New Project", "Start from scratch", true, action_height)).inner.clicked() { set_home_nav(ui.ctx(), HomeNav::NewProject); }
                        ui.add_space(action_gap);
                        if ui.allocate_ui(egui::vec2(button_width, action_height), |ui| reference_action_button(ui, crate::ui::icons::SVG_OPEN_FOLDER, "Open Project", "Browse files", false, action_height)).inner.clicked() { enter_studio_open_dialog(app); }
                    });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let button_width = ((ui.available_width() - action_gap) / 2.0).max(72.0);
                        if ui.allocate_ui(egui::vec2(button_width, action_height), |ui| reference_action_button(ui, crate::ui::icons::SVG_COMPOSITION, "New Composition", "Create a comp", false, action_height)).inner.clicked() { new_composition(app); }
                        ui.add_space(action_gap);
                        if ui.allocate_ui(egui::vec2(button_width, action_height), |ui| reference_action_button(ui, crate::ui::icons::SVG_IMPORT, "Import Footage", "Add media", false, action_height)).inner.clicked() { import_footage_dialog(app); }
                    });
                }
            });
        };
        if stacked {
            ui.horizontal_wrapped(draw_home_first_row);
        } else {
            ui.horizontal(draw_home_first_row);
        }
        ui.add_space(if compact { 10.0 } else { 23.0 });
        draw_reference_project_row(app, ui, ctx, compact);
        ui.add_space(if compact { 10.0 } else { 24.0 });
        draw_reference_templates(ui, compact, true);
        ui.add_space(if compact { 10.0 } else { 20.0 });
    });
    });
}

fn draw_reference_continue_body(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    compact: bool,
    vertical: bool,
    preview_height: f32,
) {
    if vertical {
        let height = (ui.available_width() / 1.82).clamp(76.0, preview_height);
        draw_reference_continue_preview(app, ctx, ui, height);
        ui.add_space(8.0);
        draw_reference_continue_details(app, ctx, ui, compact);
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let preview_width = ui.available_width() * if compact { 0.54 } else { 0.567 };
            ui.allocate_ui(egui::vec2(preview_width, preview_height), |ui| {
                draw_reference_continue_preview(app, ctx, ui, preview_height);
            });
            ui.add_space(if compact { 12.0 } else { 24.0 });
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width().max(1.0), preview_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| draw_reference_continue_details(app, ctx, ui, compact),
            );
        });
    }
}

fn draw_reference_continue_preview(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    height: f32,
) {
    if let Some(id) = reference_texture(app, ctx, "continue_working_preview.webp") {
        let image_width = ui.available_width().max(1.0);
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(54, 72, 91)))
            .rounding(5.0)
            .show(ui, |ui| {
                reference_cover_image(ui, id, egui::vec2(image_width, height), 406.0 / 189.0);
            });
    }
}

fn draw_reference_continue_details(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    compact: bool,
) {
    ui.label(egui::RichText::new("Eclipse").size(if compact { 18.0 } else { 20.0 }).strong());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "comp-meta",
            crate::ui::icons::SVG_COMP,
            egui::vec2(16.0, 16.0),
            egui::Color32::from_rgb(172, 189, 216),
        );
        ui.label(egui::RichText::new("SH_0140 / portal_final_v18").size(if compact { 11.0 } else { 14.0 }).color(colors::TEXT_SECONDARY));
    });
    ui.horizontal_wrapped(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "resolution-meta",
            crate::ui::icons::SVG_RESOLUTION,
            egui::vec2(16.0, 16.0),
            egui::Color32::from_rgb(172, 189, 216),
        );
        ui.label(egui::RichText::new("3840 × 2160").size(if compact { 11.0 } else { 14.0 }).color(colors::TEXT_SECONDARY));
        ui.add_space(if compact { 4.0 } else { 12.0 });
        crate::ui::icons::render_svg_bytes(
            ui,
            "frame-meta",
            crate::ui::icons::SVG_FRAME,
            egui::vec2(16.0, 16.0),
            egui::Color32::from_rgb(172, 189, 216),
        );
        ui.label(egui::RichText::new("1001 – 1148").size(if compact { 11.0 } else { 14.0 }).color(colors::TEXT_SECONDARY));
    });
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "clock-meta",
            crate::ui::icons::SVG_CLOCK,
            egui::vec2(16.0, 16.0),
            egui::Color32::from_rgb(172, 189, 216),
        );
        ui.label(egui::RichText::new("Edited 18 minutes ago").size(if compact { 11.0 } else { 14.0 }).color(colors::TEXT_SECONDARY));
    });
    ui.add_space(if compact { 4.0 } else { 13.0 });
    if reference_cta_button(ui, "Resume Composition", compact).clicked() {
        if let Some(top) = recent_project_entries(ctx).into_iter().find(|e| e.path.is_file()) {
            open_project_path(app, &top.path);
        }
    }
}

fn landing_nav_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    current: HomeNav,
    nav: HomeNav,
    icon: &'static str,
    label: &str,
    active_orange: bool,
) {
    let active = current == nav;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), egui::Sense::click());
    if active {
        let active_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 17.0, rect.top()), rect.max);
        ui.painter().rect_filled(active_rect, 0.0, egui::Color32::from_rgb(27, 35, 42));
        ui.painter().rect_filled(
            egui::Rect::from_min_size(egui::pos2(rect.left() + 17.0, rect.top()), egui::vec2(4.0, rect.height())),
            0.0,
            if active_orange {
                egui::Color32::from_rgb(255, 111, 28)
            } else {
                egui::Color32::from_rgb(48, 141, 255)
            },
        );
    }
    crate::ui::icons::render_svg_at(
        ui,
        format!("landing-nav-{label}"),
        icon,
        egui::vec2(24.0, 24.0),
        if active {
            if active_orange { egui::Color32::from_rgb(255, 145, 50) } else { colors::TEXT_PRIMARY }
        } else {
            egui::Color32::from_rgb(193, 205, 218)
        },
        egui::pos2(rect.left() + 44.0, rect.center().y - 12.0),
    );
    let label_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 92.0, rect.top()),
        egui::pos2(rect.right() - 18.0, rect.bottom()),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(label_rect), |label_ui| {
        label_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |label_ui| {
            label_ui.add(
                egui::Label::new(
                    egui::RichText::new(label)
                        .size(16.0)
                        .color(if active { colors::TEXT_PRIMARY } else { egui::Color32::from_rgb(193, 205, 218) }),
                )
                .truncate(),
            );
        });
    });
    if response.clicked() {
        if label == "終了" {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            set_home_nav(ctx, nav);
        }
    }
}

fn draw_landing_sidebar(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let current = home_nav(ctx);
    ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(17, 25, 31));
    ui.add_space(17.0);
    ui.horizontal(|ui| {
        ui.add_space(29.0);
        if app.home_banner.is_none() {
            if let Ok(img) = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp")) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        if let Some(texture) = app.home_banner.as_ref() {
            ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::vec2(94.0, 94.0))));
        }
        ui.add_space(2.0);
        ui.label(egui::RichText::new("Kagari").size(29.0).strong().color(colors::TEXT_PRIMARY));
        ui.label(egui::RichText::new("VFX").size(29.0).color(egui::Color32::from_rgb(161, 174, 190)));
    });
    ui.add_space(33.0);
    landing_nav_row(ui, ctx, current, HomeNav::Home, crate::ui::icons::SVG_HOME, "ホーム", true);
    landing_nav_row(ui, ctx, current, HomeNav::Projects, crate::ui::icons::SVG_FOLDER, "プロジェクトを開く", false);
    landing_nav_row(ui, ctx, current, HomeNav::NewProject, crate::ui::icons::SVG_FILE_PLUS, "新規プロジェクト", false);
    ui.add_space(58.0);
    ui.painter().line_segment(
        [egui::pos2(39.0, ui.cursor().top()), egui::pos2(ui.available_width() - 40.0, ui.cursor().top())],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 62, 72)),
    );
    ui.add_space(26.0);
    landing_nav_row(ui, ctx, current, HomeNav::Tutorial, crate::ui::icons::SVG_BOOK, "チュートリアル", false);
    landing_nav_row(ui, ctx, current, HomeNav::Documentation, crate::ui::icons::SVG_DOCUMENT, "ドキュメント", false);
    let side = ui.max_rect();
    let short_sidebar = side.height() < 760.0;
    let settings_rect = egui::Rect::from_min_size(
        egui::pos2(side.left(), side.bottom() - if short_sidebar { 126.0 } else { 180.0 }),
        egui::vec2(side.width(), 56.0),
    );
    let exit_rect = egui::Rect::from_min_size(
        egui::pos2(side.left(), side.bottom() - if short_sidebar { 70.0 } else { 124.0 }),
        egui::vec2(side.width(), 56.0),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(settings_rect), |settings_ui| {
        landing_nav_row(settings_ui, ctx, current, HomeNav::Settings, crate::ui::icons::SVG_SETTINGS, "設定", false);
    });
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(exit_rect), |exit_ui| {
        landing_nav_row(exit_ui, ctx, current, HomeNav::Compositing, crate::ui::icons::SVG_POWER, "終了", false);
    });
    if !short_sidebar {
        ui.painter().text(
            egui::pos2(side.left() + 38.0, side.bottom() - 31.0),
            egui::Align2::LEFT_CENTER,
            "Kagari VFX   v0.1.0",
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(157, 169, 183),
        );
    }
}

fn landing_button_rect(ui: &mut egui::Ui, rect: egui::Rect, label: &str, icon: &'static str, accent: bool) -> egui::Response {
    let response = ui.interact(
        rect,
        egui::Id::new(("landing-button", label, rect.min.x as i32, rect.min.y as i32)),
        egui::Sense::click(),
    );
    ui.painter().rect(
        rect,
        8.0,
        if accent { egui::Color32::from_rgb(255, 103, 24) } else { egui::Color32::from_rgb(25, 33, 40) },
        egui::Stroke::new(1.0_f32, if accent { egui::Color32::from_rgb(255, 144, 58) } else { egui::Color32::from_rgb(54, 68, 80) }),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
        egui::pos2(rect.left() + 44.0, rect.center().y - 14.0), egui::vec2(28.0, 28.0),
    )), |icon_ui| {
        crate::ui::icons::render_svg_bytes(icon_ui, &format!("landing-button-{label}"), icon, egui::vec2(28.0, 28.0), egui::Color32::WHITE);
    });
    ui.painter().text(egui::pos2(rect.left() + 91.0, rect.center().y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(16.0), colors::TEXT_PRIMARY);
    response
}

fn draw_landing_hero(ui: &mut egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(12, 19, 24));
    let curve = |start: egui::Pos2, c1: egui::Pos2, c2: egui::Pos2, end: egui::Pos2| {
        (0..=48).map(move |index| {
            let t = index as f32 / 48.0;
            let u = 1.0 - t;
            egui::pos2(
                u * u * u * start.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * end.x,
                u * u * u * start.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * end.y,
            )
        }).collect::<Vec<_>>()
    };
    let start = egui::pos2(rect.left() + rect.width() * 0.59, rect.bottom() + 15.0);
    let end = egui::pos2(rect.right() + 20.0, rect.top() + 80.0);
    for (delta, width, color) in [
        (0.0_f32, 20.0_f32, egui::Color32::from_rgb(255, 79, 22)),
        (52.0_f32, 8.0_f32, egui::Color32::from_rgb(181, 53, 22)),
        (104.0_f32, 3.0_f32, egui::Color32::from_rgb(123, 43, 25)),
    ] {
        let points = curve(
            egui::pos2(start.x - delta, start.y),
            egui::pos2(rect.left() + rect.width() * 0.64 - delta, rect.top() + 132.0),
            egui::pos2(rect.left() + rect.width() * 0.78 - delta, rect.top() + 43.0),
            egui::pos2(end.x - delta * 0.16, end.y),
        );
        painter.add(egui::Shape::line(points, egui::Stroke::new(width, color.linear_multiply(0.72))));
    }
    painter.text(egui::pos2(rect.right() - 2.0, rect.top() + 39.0), egui::Align2::RIGHT_CENTER, "Create. Composite. Illuminate.", egui::FontId::proportional(14.0), egui::Color32::from_rgb(159, 168, 181));
}

fn draw_landing_dashed_rect(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0_f32, color);
    let draw_axis = |start: egui::Pos2, end: egui::Pos2| {
        let length = start.distance(end).max(1.0);
        let direction = (end - start) / length;
        let mut offset = 0.0;
        while offset < length {
            let dash_end = (offset + 6.0).min(length);
            painter.line_segment([start + direction * offset, start + direction * dash_end], stroke);
            offset += 10.0;
        }
    };
    draw_axis(rect.left_top(), rect.right_top());
    draw_axis(rect.right_top(), rect.right_bottom());
    draw_axis(rect.right_bottom(), rect.left_bottom());
    draw_axis(rect.left_bottom(), rect.left_top());
}

fn draw_landing_home(app: &mut KagariApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    let content = ui.max_rect();
    ui.painter().rect_filled(content, 0.0, egui::Color32::from_rgb(12, 19, 24));
    let mobile = content.width() < 1080.0;
    let margin = if mobile { 28.0 } else { 40.0 };
    let right_margin = if mobile { 28.0 } else { 44.0 };
    let hero = egui::Rect::from_min_size(
        egui::pos2(content.left() + margin, content.top() + 16.0),
        egui::vec2((content.width() - margin - right_margin).max(1.0), if mobile { 321.0 } else { 334.0 }),
    );
    draw_landing_hero(ui, hero);
    ui.painter().text(egui::pos2(hero.left(), hero.top() + 158.0), egui::Align2::LEFT_CENTER, "映像に、想像を重ねる。", egui::FontId::proportional(if mobile { 29.0 } else { 43.0 }), colors::TEXT_PRIMARY);
    ui.painter().text(egui::pos2(hero.left(), hero.top() + 211.0), egui::Align2::LEFT_CENTER, "ノードでも、レイヤーでも、自在に。あなたのビジョンを、より遠くへ。", egui::FontId::proportional(if mobile { 13.0 } else { 18.0 }), egui::Color32::from_rgb(168, 180, 193));
    let button_width = if mobile { 220.0 } else { 266.0 };
    let button_top = hero.top() + 254.0;
    let first_button = egui::Rect::from_min_size(egui::pos2(hero.left(), button_top), egui::vec2(button_width, 62.0));
    let second_button = egui::Rect::from_min_size(egui::pos2(hero.left() + button_width + 23.0, button_top), egui::vec2(button_width, 62.0));
    if landing_button_rect(ui, first_button, "新規プロジェクト", crate::ui::icons::SVG_FILE_PLUS, true).clicked() {
        enter_studio_new_project(app);
        app.show_new_comp_dialog = true;
    }
    if landing_button_rect(ui, second_button, "プロジェクトを開く", crate::ui::icons::SVG_OPEN_FOLDER, false).clicked() {
        enter_studio_open_dialog(app);
    }

    let section_top = hero.bottom() + 23.0;
    ui.painter().line_segment([egui::pos2(content.left() + margin, section_top), egui::pos2(content.right() - right_margin, section_top)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(48, 61, 71)));
    ui.painter().text(egui::pos2(content.left() + margin, section_top + 36.0), egui::Align2::LEFT_CENTER, "最近のプロジェクト", egui::FontId::proportional(21.0), colors::TEXT_PRIMARY);
    let compact_mobile = mobile && content.height() < 760.0;
    let recent_height = if compact_mobile { 196.0 } else if mobile { 188.0 } else { 232.0 };
    let recent = egui::Rect::from_min_size(egui::pos2(content.left() + margin, section_top + 53.0), egui::vec2(content.width() - margin - right_margin, recent_height));
    ui.painter().rect_filled(recent, 7.0, egui::Color32::from_rgb(14, 22, 28));
    draw_landing_dashed_rect(ui.painter(), recent, egui::Color32::from_rgb(58, 72, 81));
    let empty_center = recent.center().x;
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
        egui::pos2(empty_center - 22.0, recent.top() + if compact_mobile { 22.0 } else { 33.0 }),
        egui::vec2(44.0, 44.0),
    )), |empty_ui| {
        crate::ui::icons::render_svg_bytes(empty_ui, "landing-empty-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(44.0, 44.0), egui::Color32::from_rgb(161, 176, 194));
    });
    ui.painter().text(
        egui::pos2(empty_center, recent.top() + if compact_mobile { 94.0 } else { 108.0 }),
        egui::Align2::CENTER_CENTER,
        "まだプロジェクトはありません",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::pos2(empty_center, recent.top() + if compact_mobile { 119.0 } else { 140.0 }),
        egui::Align2::CENTER_CENTER,
        "新規プロジェクトを作成して、はじめましょう。",
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(163, 175, 187),
    );
    let empty_button = egui::Rect::from_min_size(
        egui::pos2(recent.center().x - 114.0, if compact_mobile { recent.bottom() - 62.0 } else { recent.center().y + 52.0 }),
        egui::vec2(228.0, 62.0),
    );
    if landing_button_rect(ui, empty_button, "新規プロジェクト", crate::ui::icons::SVG_FILE_PLUS, true).clicked() {
        enter_studio_new_project(app);
        app.show_new_comp_dialog = true;
    }
    let lower_top = recent.bottom() + if compact_mobile { 24.0 } else { 45.0 };
    let lower_width = content.width() - margin - right_margin;
    let col_gap = 44.0;
    let left_width = if mobile { lower_width } else { (lower_width - col_gap) * 0.567 };
    ui.painter().text(egui::pos2(content.left() + margin, lower_top), egui::Align2::LEFT_CENTER, "ニュース", egui::FontId::proportional(19.0), colors::TEXT_PRIMARY);
    let news = egui::Rect::from_min_size(egui::pos2(content.left() + margin, lower_top + 17.0), egui::vec2(left_width, 164.0));
    ui.painter().rect(news, 8.0, egui::Color32::from_rgb(18, 27, 33), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 75)));
    for (index, (label, date)) in [("Kagari VFX 0.1.0 リリース", "2025/09/01"), ("はじめに：基本操作ガイド", "2025/08/20"), ("サンプルプロジェクトを追加", "2025/08/10")].into_iter().enumerate() {
        let y = news.top() + 29.0 + index as f32 * 49.0;
        ui.painter().circle_filled(egui::pos2(news.left() + 28.0, y), 7.0, if index == 0 { egui::Color32::from_rgb(255, 136, 33) } else { egui::Color32::from_rgb(145, 160, 174) });
        ui.painter().text(egui::pos2(news.left() + 53.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        ui.painter().text(egui::pos2(news.right() - 31.0, y), egui::Align2::RIGHT_CENTER, date, egui::FontId::proportional(13.0), egui::Color32::from_rgb(157, 170, 183));
        if index < 2 { ui.painter().line_segment([egui::pos2(news.left() + 22.0, y + 22.0), egui::pos2(news.right() - 22.0, y + 22.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 53, 63))); }
    }
    if !mobile {
        let resources_left = news.right() + col_gap;
        ui.painter().text(egui::pos2(resources_left, lower_top), egui::Align2::LEFT_CENTER, "リソース", egui::FontId::proportional(19.0), colors::TEXT_PRIMARY);
        let resources = egui::Rect::from_min_size(egui::pos2(resources_left, lower_top + 17.0), egui::vec2(lower_width - left_width - col_gap, 164.0));
        ui.painter().rect(resources, 8.0, egui::Color32::from_rgb(18, 27, 33), egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 75)));
        for (index, (icon, label)) in [(crate::ui::icons::SVG_BOOK, "公式ドキュメント"), (crate::ui::icons::SVG_PLAY_CIRCLE, "チュートリアルを見る"), (crate::ui::icons::SVG_CHAT, "コミュニティ")].into_iter().enumerate() {
            let y = resources.top() + 29.0 + index as f32 * 49.0;
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(resources.left() + 19.0, y - 13.0), egui::vec2(27.0, 27.0))), |icon_ui| { crate::ui::icons::render_svg_bytes(icon_ui, &format!("landing-resource-{index}"), icon, egui::vec2(27.0, 27.0), egui::Color32::from_rgb(200, 212, 224)); });
            ui.painter().text(egui::pos2(resources.left() + 72.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
            ui.painter().text(egui::pos2(resources.right() - 24.0, y), egui::Align2::CENTER_CENTER, "↗", egui::FontId::proportional(22.0), egui::Color32::from_rgb(189, 202, 214));
            if index < 2 { ui.painter().line_segment([egui::pos2(resources.left() + 19.0, y + 22.0), egui::pos2(resources.right() - 19.0, y + 22.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 53, 63))); }
        }
    }
}

fn draw_reference_project_row(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context, compact: bool) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Recent Projects").size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if reference_view_all_button(ui, "View All") {
                set_home_nav(ctx, HomeNav::Projects);
            }
        });
    });
    ui.add_space(if compact { 7.0 } else { 10.0 });
    let projects = recent_project_entries(ctx);
    if projects.is_empty() {
        let empty_height = if compact { 154.0 } else { 228.0 };
        let empty = fixed_card_with_inset(
            ui,
            egui::vec2(ui.available_width(), empty_height),
            egui::Color32::TRANSPARENT,
            egui::Color32::from_rgb(54, 70, 83),
            7.0,
            if compact { 12.0 } else { 22.0 },
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(if compact { 20.0 } else { 34.0 });
                    crate::ui::icons::render_svg_bytes(ui, "empty-project-folder", crate::ui::icons::SVG_FOLDER, egui::vec2(if compact { 30.0 } else { 42.0 }, if compact { 30.0 } else { 42.0 }), colors::TEXT_SECONDARY);
                    ui.add_space(if compact { 8.0 } else { 12.0 });
                    ui.label(egui::RichText::new("まだプロジェクトはありません").size(if compact { 14.0 } else { 19.0 }).strong());
                    ui.label(egui::RichText::new("新規プロジェクトを作成して、はじめましょう。").size(if compact { 10.0 } else { 14.0 }).color(colors::TEXT_SECONDARY));
                    ui.add_space(if compact { 10.0 } else { 16.0 });
                    if reference_cta_button(ui, "新規プロジェクト", compact).clicked() {
                        enter_studio_new_project(app);
                        app.show_new_comp_dialog = true;
                    }
                });
            },
        );
        let _ = empty;
        return;
    }
    let available_width = ui.available_width();
    let columns = if available_width < 680.0 { 1.0 } else if available_width < 1040.0 { 2.0 } else { 4.0 };
    let card_gap = if columns == 1.0 { 0.0 } else if compact { 12.0 } else { 18.0 };
    let card_width = ((available_width - card_gap * (columns - 1.0)) / columns).max(140.0);
    let card_height = if compact { 150.0 } else { 215.0 };
    let image_height = if compact { 74.0 } else { 124.0 };
    let project_inset = if compact { 10.0 } else { 20.0 };
    let thumbnails = ["recent_project_eclipse.webp", "recent_project_citadel.webp", "recent_project_rift.webp", "recent_project_atlas.webp"];
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (card_index, project) in projects.iter().take(4).enumerate() {
            let asset = thumbnails[card_index % thumbnails.len()];
            let title = project.name.as_str();
            let meta = project.summary.as_ref().and_then(|summary| summary.comp_names.get(summary.active_idx)).map(String::as_str).unwrap_or("Project");
            let age = project.modified.map(fmt_age).unwrap_or_else(|| "Not found".to_string());
            let response = fixed_card_with_inset(ui, egui::vec2(card_width, card_height), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 5.0, 0.0, |ui| {
                if let Some(id) = reference_texture(app, ctx, asset) {
                    reference_cover_image(ui, id, egui::vec2(card_width, image_height), 316.0 / 121.0);
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(project_inset);
                    ui.label(egui::RichText::new(title).size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(project_inset);
                        crate::ui::icons::render_svg_bytes(ui, &format!("{title}-more"), crate::ui::icons::SVG_MORE, egui::vec2(15.0, 15.0), colors::TEXT_SECONDARY);
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(project_inset);
                    ui.label(egui::RichText::new(meta).size(12.0).color(colors::TEXT_SECONDARY));
                });
                ui.horizontal(|ui| {
                    ui.add_space(project_inset);
                    ui.label(egui::RichText::new(&age).size(12.0).color(colors::TEXT_MUTED));
                });
            });
            if response.clicked() {
                open_project_path(app, &project.path);
            }
            if card_index < 3 {
                ui.add_space(card_gap);
            }
        }
    });
}

fn draw_reference_templates(ui: &mut egui::Ui, compact: bool, show_header: bool) {
    let narrow = reference_narrow(ui);
    if show_header {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Templates").size(18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if reference_view_all_button(ui, "View All") {
                    set_home_nav(ui.ctx(), HomeNav::Templates);
                }
            });
        });
        ui.add_space(if compact { 7.0 } else { 9.0 });
    }
    let available_width = ui.available_width();
    let columns = if available_width < 560.0 { 2.0 } else if available_width < 980.0 { 3.0 } else { 6.0 };
    let template_gap = if narrow { 6.0 } else if compact { 4.0 } else { 15.0 };
    let width_gap = if columns == 6.0 { 15.0 } else { template_gap };
    let card_width = (((available_width - width_gap * (columns - 1.0)) / columns)
        - if columns == 6.0 { 3.0 } else { 0.0 })
        .max(72.0);
    let card_height = if narrow { 76.0 } else if compact { 116.0 } else { 166.0 };
    let illustration_height = if narrow { 34.0 } else if compact { 38.0 } else { 78.0 };
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.horizontal_wrapped(|ui| {
        for (index, (icon, title, desc)) in [(crate::ui::icons::SVG_KEYING, "Keying Setup", "Green screen workflow"), (crate::ui::icons::SVG_TRACKING, "Tracking Setup", "Planar / 3D tracking"), (crate::ui::icons::SVG_CLEAN_PLATE, "Clean Plate Setup", "Remove objects"), (crate::ui::icons::SVG_PARTICLES, "Particle Composite", "Sparks, smoke, fire"), (crate::ui::icons::SVG_TITLE, "Title Animation", "Motion graphics"), (crate::ui::icons::SVG_SCREEN, "Screen Replacement", "Compositing workflow")].into_iter().enumerate() {
            let response = fixed_card_with_inset(ui, egui::vec2(card_width, card_height), egui::Color32::from_rgb(22, 32, 42), egui::Color32::from_rgb(43, 60, 76), 5.0, if narrow { 6.0 } else if compact { 10.0 } else { 18.0 }, |ui| {
                ui.vertical(|ui| {
                    let illustration_width = illustration_height * 80.0 / 48.0;
                    ui.horizontal(|ui| {
                        ui.add_space(((ui.available_width() - illustration_width) * 0.5).max(0.0));
                        crate::ui::icons::render_svg_bytes(ui, title, icon, egui::vec2(illustration_width, illustration_height), egui::Color32::WHITE);
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(if narrow { 0.0 } else { 6.0 });
                        ui.label(egui::RichText::new(title).size(if narrow { 10.0 } else if compact { 12.0 } else { 16.0 }).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(if narrow { 0.0 } else { 6.0 });
                        ui.label(egui::RichText::new(desc).size(if narrow { 8.0 } else if compact { 10.0 } else { 13.0 }).color(colors::TEXT_SECONDARY));
                    });
                });
            });
            if show_header && response.clicked() {
                set_home_nav(ui.ctx(), HomeNav::Templates);
            }
            if index < 5 {
                ui.add_space(template_gap);
            }
        }
    });
}

fn draw_nav(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(6.0);
    let current = home_nav(ctx);
    let mut next = current;
    for (nav, icon, label) in [
        (HomeNav::Home, crate::ui::icons::SVG_HOME, "Home"),
        (HomeNav::Projects, crate::ui::icons::SVG_FOLDER, "Projects"),
        (HomeNav::Recent, crate::ui::icons::SVG_CLOCK, "Recent"),
        (HomeNav::Starred, crate::ui::icons::SVG_SCREEN, "Starred"),
        (HomeNav::Templates, crate::ui::icons::SVG_LAYERS, "Templates"),
    ] {
        let resp = ui.horizontal(|ui| {
            crate::ui::icons::render_svg_bytes(ui, label, icon, egui::vec2(15.0, 15.0), if current == nav { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
            ui.selectable_label(
                current == nav,
                egui::RichText::new(label)
                    .small()
                    .color(if current == nav { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }),
            )
        });
        if resp.inner.clicked() {
            next = nav;
        }
    }
    if next != current {
        set_home_nav(ctx, next);
        app.home_selected = None;
        app.home_preview = None;
    }
    ui.add_space(4.0);
    ui.separator();
    // Quiet secondary storage indicator (full management lives in Settings).
    if let Some(text) = storage_free_text() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("💾 {}", text))
                .small()
                .color(colors::TEXT_MUTED),
        )
        .on_hover_text("Storage details live in Settings > Cache");
    }
    let _ = app;
}

fn draw_home_workspace(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical()
        .id_salt("home_center")
        .show(ui, |ui| {
            draw_resume(app, ui, ctx);
            ui.add_space(8.0);
            draw_recent_projects(app, ui, ctx, 5);
            ui.add_space(8.0);
            draw_recent_assets(app, ui, ctx);
        });
}

// ── 1. Continue Working ───────────────────────────────────────

fn draw_resume(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    crate::ui::theme::draw_section_header(ui, "Continue Working", "▶");
    let entries = recent_project_entries(ctx);
    let Some(top) = entries.iter().find(|e| e.path.is_file()) else {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("No recent work yet — start something.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::ui::custom_widgets::ae_button(ui, "Open Project").clicked() {
                    enter_studio_open_dialog(app);
                }
                if crate::ui::custom_widgets::ae_button_accent(ui, "＋ New Project").clicked()
                {
                    enter_studio_new_project(app);
                }
            });
        });
        return;
    };
    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .rounding(egui::Rounding::same(4.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Compact project tile.
            let (tile, _) =
                ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
            ui.painter().rect_filled(tile, 3.0, colors::BG_SURFACE);
            ui.painter().rect_stroke(
                tile,
                3.0,
                egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
            );
            ui.painter().text(
                tile.center(),
                egui::Align2::CENTER_CENTER,
                "🎬",
                egui::FontId::proportional(20.0),
                colors::TEXT_SECONDARY,
            );
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&top.name)
                        .size(14.0)
                        .color(colors::TEXT_PRIMARY),
                );
                let comp_line = match &top.summary {
                    Some(s) => {
                        let active = s
                            .comp_names
                            .get(s.active_idx.min(s.comp_names.len().saturating_sub(1)))
                            .cloned()
                            .unwrap_or_default();
                        if s.comp_names.len() > 1 {
                            format!(
                                "{}  ·  {} comps · {} layers",
                                active,
                                s.comp_names.len(),
                                s.layers
                            )
                        } else if active.is_empty() {
                            format!("{} layers", s.layers)
                        } else {
                            format!("{}  ·  {} layers", active, s.layers)
                        }
                    }
                    None => "Project".to_string(),
                };
                ui.label(
                    egui::RichText::new(comp_line)
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
                let sub = match top.modified {
                    Some(m) => format!(
                        "Edited {}  ·  {}",
                        fmt_age(m),
                        top.path.display()
                    ),
                    None => top.path.display().to_string(),
                };
                ui.label(
                    egui::RichText::new(sub).small().color(colors::TEXT_MUTED),
                );
            });
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if crate::ui::custom_widgets::ae_button_accent(ui, "▶ Resume").clicked() {
                        open_project_path(app, &top.path.clone());
                    }
                },
            );
        });
    });
}

// ── 2. Recent Projects ────────────────────────────────────────

fn draw_recent_projects(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    limit: usize,
) {
    ui.horizontal(|ui| {
        crate::ui::theme::draw_section_header(ui, "Recent Projects", "");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("All")
                .on_hover_text("Show all projects")
                .clicked()
            {
                set_home_nav(ctx, HomeNav::Projects);
            }
        });
    });
    draw_project_rows(app, ui, ctx, recent_project_entries(ctx), limit, false);
}

fn draw_projects_view(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    starred_only: Option<bool>,
) {
    egui::ScrollArea::vertical()
        .id_salt("home_projects")
        .show(ui, |ui| {
            if starred_only == Some(true) {
                crate::ui::theme::draw_section_header(ui, "Starred", "☆");
            } else {
                crate::ui::theme::draw_section_header(ui, "Projects", "");
            }
            let entries = recent_project_entries(ctx);
            draw_project_rows(app, ui, ctx, entries, usize::MAX, starred_only == Some(true));
        });
}

fn draw_project_rows(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    entries: Vec<RecentProject>,
    limit: usize,
    starred_only: bool,
) {
    let starred = crate::ui::project_io::starred_projects();
    let is_starred = |p: &std::path::Path| {
        let s = p.to_string_lossy().to_string();
        starred.iter().any(|q| q == &s)
    };
    let mut shown = 0;
    let mut open: Option<std::path::PathBuf> = None;
    for entry in entries.iter() {
        if starred_only && !is_starred(&entry.path) {
            continue;
        }
        if shown >= limit {
            break;
        }
        shown += 1;
        let exists = entry.path.is_file();
        let selected = app.home_selected.as_deref() == Some(entry.path.as_path());
        let row_h = 30.0;
        let probe = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_width(), row_h),
        );
        if !ui.is_rect_visible(probe) {
            ui.add_space(row_h);
            continue;
        }
        ui.horizontal(|ui| {
            let star_label = if is_starred(&entry.path) { "★" } else { "☆" };
            if ui
                .small_button(star_label)
                .on_hover_text("Toggle starred")
                .clicked()
            {
                crate::ui::project_io::toggle_starred(&entry.path);
            }
            let name = if exists {
                entry.name.clone()
            } else {
                format!("{} (missing)", entry.name)
            };
            let resp = ui.selectable_label(
                selected,
                egui::RichText::new(name).small().color(if exists {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_MUTED
                }),
            );
            if exists && resp.clicked() {
                app.home_selected = Some(entry.path.clone());
                app.home_preview = None;
            }
            if exists && resp.double_clicked() {
                open = Some(entry.path.clone());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let age = entry
                    .modified
                    .map(fmt_age)
                    .unwrap_or_else(|| "—".to_string());
                ui.label(egui::RichText::new(age).small().color(colors::TEXT_MUTED));
                if let Some(s) = &entry.summary {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} comps · {} layers",
                            s.comp_names.len(),
                            s.layers
                        ))
                        .small()
                        .color(colors::TEXT_MUTED),
                    );
                }
            });
        });
        if selected && exists {
            ui.indent("home_proj_path", |ui| {
                ui.label(
                    egui::RichText::new(entry.path.display().to_string())
                        .small()
                        .color(colors::TEXT_MUTED),
                );
            });
        }
        ui.separator();
    }
    if shown == 0 {
        ui.label(
            egui::RichText::new(if starred_only {
                "Nothing starred yet — hover a project and press ☆."
            } else {
                "No projects yet."
            })
            .small()
            .color(colors::TEXT_MUTED),
        );
    }
    if let Some(path) = open {
        open_project_path(app, &path);
    }
    let _ = ctx;
}

// ── 3. Recent Assets ──────────────────────────────────────────

fn draw_recent_assets(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    crate::ui::theme::draw_section_header(ui, "Recent Assets", "🖼");
    let items = recent_asset_entries(ctx);
    if items.is_empty() {
        ui.label(
            egui::RichText::new("No footage or images next to recent projects yet.")
                .small()
                .color(colors::TEXT_MUTED),
        );
        return;
    }
    const TILE_W: f32 = 118.0;
    const TILE_H: f32 = 92.0;
    let mut pending_open: Option<std::path::PathBuf> = None;
    let per_row = ((ui.available_width() / (TILE_W + 8.0)).floor() as usize).max(1);
    for chunk in items.chunks(per_row) {
        ui.horizontal(|ui| {
            for item in chunk {
                draw_asset_tile(app, ui, ctx, item, TILE_W, TILE_H, &mut pending_open);
            }
        });
    }
    if let Some(path) = pending_open {
        open_selected_in_studio(app, &path);
    }
}

fn draw_asset_tile(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    item: &AssetEntry,
    w: f32,
    h: f32,
    pending_open: &mut Option<std::path::PathBuf>,
) {
    let probe = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(w, h));
    if !ui.is_rect_visible(probe) {
        ui.add_space(h);
        return;
    }
    let selected = app.home_selected.as_deref() == Some(item.path.as_path());
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        3.0,
        if selected {
            colors::BG_HOVER
        } else {
            colors::BG_DARK
        },
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(
            1.0_f32,
            if selected {
                colors::BORDER_ACTIVE
            } else {
                colors::BORDER_SUBTLE
            },
        ),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(5.0))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    if item.kind == "image" {
        if let Some(id) = thumb_for(app, ctx, &item.path) {
            child.add(egui::Image::new(egui::load::SizedTexture::new(
                id,
                egui::vec2(88.0, 48.0),
            )));
        } else {
            child.label(egui::RichText::new("🖼").size(22.0));
        }
    } else {
        child.label(egui::RichText::new(if item.kind == "video" {
            "🎬"
        } else {
            "🔊"
        })
        .size(22.0));
    }
    child.add(
        egui::Label::new(
            egui::RichText::new(&item.name)
                .small()
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    );
    let mut meta = if item.is_sequence {
        format!("{} frames", item.frames)
    } else {
        fmt_bytes(item.size)
    };
    if let Some(m) = item.modified {
        meta.push_str(&format!(" · {}", fmt_age(m)));
    }
    child.label(egui::RichText::new(meta).small().color(colors::TEXT_MUTED));
    if resp.clicked() {
        app.home_selected = Some(item.path.clone());
        app.home_preview = None;
    }
    if resp.double_clicked() {
        *pending_open = Some(item.path.clone());
    }
}

fn draw_recent_view(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical()
        .id_salt("home_recent")
        .show(ui, |ui| {
            crate::ui::theme::draw_section_header_svg(ui, "Recent", crate::ui::icons::SVG_CLOCK);
            draw_project_rows(app, ui, ctx, recent_project_entries(ctx), usize::MAX, false);
            ui.add_space(8.0);
            draw_recent_assets(app, ui, ctx);
        });
}

// ── Templates (compact) ───────────────────────────────────────

fn draw_templates_view(app: &mut KagariApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("home_templates")
        .show(ui, |ui| {
            crate::ui::theme::draw_section_header_svg(ui, "Templates", crate::ui::icons::SVG_LAYERS);
            for (icon, title, desc, run) in [
                (crate::ui::icons::SVG_FILE, "Blank Project", "Start from scratch", 0u8),
                (crate::ui::icons::SVG_PLAY, "Demo Scene", "Animated showcase", 1u8),
                (crate::ui::icons::SVG_CAMERA, "Cinematic VFX", "Film & trailer work", 2u8),
            ] {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::click());
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                ui.painter().rect_filled(rect, 3.0, colors::BG_DARK);
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink(8.0))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                crate::ui::icons::render_svg_bytes(&mut child, title, icon, egui::vec2(20.0, 20.0), colors::TEXT_PRIMARY);
                child.vertical(|ui| {
                    ui.label(egui::RichText::new(title).small().color(colors::TEXT_PRIMARY));
                    ui.label(egui::RichText::new(desc).small().color(colors::TEXT_MUTED));
                });
                if resp.clicked() {
                    match run {
                        0 => enter_studio_new_project(app),
                        _ => {
                            crate::ui::demo_scene::build(app);
                            app.show_home = false;
                        }
                    }
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.add_space(4.0);
            }
        });
}

// ── 7. Right inspector panel ──────────────────────────────────

fn draw_inspector(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical()
        .id_salt("home_inspector")
        .show(ui, |ui| {
            ui.add_space(4.0);
            if app.home_selected.clone().is_some() {
                draw_selection_inspector(app, ui, ctx);
            } else {
                draw_quick_actions(app, ui);
                ui.add_space(6.0);
                ui.separator();
                draw_task_mini(app, ui);
                ui.add_space(6.0);
                ui.separator();
                draw_session(app, ui);
            }
        });
}

fn draw_quick_actions(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::theme::draw_section_header(ui, "Quick Actions", "⚡");
    if crate::ui::custom_widgets::ae_button_accent(ui, "＋  New Project")
        .clicked()
    {
        enter_studio_new_project(app);
    }
    ui.add_space(2.0);
    for (label, tip) in [
        ("📂  Open Project", "Open a .json project file"),
        ("🎬  New Composition", "New project, then composition settings"),
        ("🎞  Import Footage", "Import media into a new project"),
    ] {
        if crate::ui::custom_widgets::ae_button(ui, label)
            .on_hover_text(tip)
            .clicked()
        {
            match label {
                "📂  Open Project" => enter_studio_open_dialog(app),
                "🎬  New Composition" => new_composition(app),
                _ => import_footage_dialog(app),
            }
        }
        ui.add_space(2.0);
    }
}

fn draw_selection_inspector(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(path) = app.home_selected.clone() else {
        return;
    };
    crate::ui::theme::draw_section_header(ui, "Details", "ℹ");
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_uppercase())
        .unwrap_or_default();

    // Preview.
    let preview_id = if is_image_ext(&path) {
        let stale = app
            .home_preview
            .as_ref()
            .map(|(p, _)| p != &path)
            .unwrap_or(true);
        if stale {
            app.home_preview = image::open(&path)
                .ok()
                .and_then(|img| {
                    load_texture(ctx, &format!("preview:{}", path.display()), img, 560, 320)
                })
                .map(|h| (path.clone(), h));
        }
        app.home_preview.as_ref().map(|(_, h)| h.id())
    } else {
        None
    };
    if let Some(id) = preview_id {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(
            id,
            egui::vec2(ui.available_width(), 150.0),
        )));
    } else {
        ui.label(
            egui::RichText::new(if ext == "JSON" { "🎬" } else { "🖼" }).size(36.0),
        );
    }

    ui.label(egui::RichText::new(&name).size(13.0).color(colors::TEXT_PRIMARY));
    ui.separator();
    egui::Grid::new("home_meta")
        .num_columns(2)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            let mut row = |k: &str, v: String| {
                ui.label(egui::RichText::new(k).small().color(colors::TEXT_MUTED));
                ui.label(egui::RichText::new(v).small().color(colors::TEXT_PRIMARY));
                ui.end_row();
            };
            row("Type", ext.clone());
            if let Ok(meta) = std::fs::metadata(&path) {
                row("Size", fmt_bytes(meta.len()));
                if let Ok(m) = meta.modified() {
                    row("Modified", fmt_age(m));
                }
            }
            if is_image_ext(&path) {
                if let Ok((w, h)) = image::image_dimensions(&path) {
                    row("Dimensions", format!("{} × {}", w, h));
                }
            }
            if ext == "JSON" {
                if let Some(s) = read_project_json(&path).and_then(|v| summarize_value(&v)) {
                    row("Compositions", format!("{}", s.comp_names.len()));
                    row("Layers", format!("{}", s.layers));
                    for (i, c) in s.comp_names.iter().take(6).enumerate() {
                        row(
                            if i == 0 { "Shots" } else { "" },
                            if i == s.active_idx.min(s.comp_names.len().saturating_sub(1)) {
                                format!("{} ●", c)
                            } else {
                                c.clone()
                            },
                        );
                    }
                }
            }
            row("Location", path.display().to_string());
        });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button_accent(
            ui,
            if ext == "JSON" { "▶ Open" } else { "⤵ Import" },
        )
        .clicked()
        {
            open_selected_in_studio(app, &path);
        }
        if ui.small_button("✕ Clear").clicked() {
            app.home_selected = None;
            app.home_preview = None;
        }
    });
}

fn draw_session(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("Session")
            .small()
            .color(colors::TEXT_MUTED),
    );
    let ffmpeg = crate::core::video_import::ffmpeg_available();
    let gpu = app.renderer.is_some();
    for (name, state, good) in [
        ("K-Render", "Built-in", true),
        (
            "FFmpeg",
            if ffmpeg { "Available" } else { "Not found" },
            ffmpeg,
        ),
        ("GPU Preview", if gpu { "Active" } else { "CPU fallback" }, gpu),
        ("Autosave", "On", true),
    ] {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("●")
                    .small()
                    .color(if good {
                        colors::ACCENT_GREEN
                    } else {
                        colors::ACCENT_ORANGE
                    }),
            );
            ui.label(egui::RichText::new(name).small().color(colors::TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(state).small().color(colors::TEXT_MUTED));
            });
        });
    }
    if let Some(text) = storage_free_text() {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(format!("💾 {}", text))
                .small()
                .color(colors::TEXT_MUTED),
        );
    }
}

// ── 5. Render / background tasks ──────────────────────────────

fn task_summary(app: &KagariApp) -> (String, egui::Color32, bool) {
    use crate::app_state::QueueItemStatus;
    if app.export.is_exporting {
        let pct = (app.export.export_progress.clamp(0.0, 1.0) * 100.0).round() as u32;
        return (
            format!("Rendering… {}%", pct),
            colors::ACCENT_GREEN,
            true,
        );
    }
    if app.render_queue_items.is_empty() {
        return (
            "No background tasks".to_string(),
            colors::TEXT_MUTED,
            false,
        );
    }
    let (mut done, mut rendering, mut failed, mut queued) = (0, 0, 0, 0);
    for name in &app.render_queue_items {
        match app
            .render_item_status
            .get(name)
            .copied()
            .unwrap_or(QueueItemStatus::Queued)
        {
            QueueItemStatus::Done => done += 1,
            QueueItemStatus::Rendering => rendering += 1,
            QueueItemStatus::Failed => failed += 1,
            QueueItemStatus::Queued => queued += 1,
        }
    }
    if failed > 0 {
        (
            format!(
                "Render queue: {} done · {} rendering · {} queued · {} failed",
                done, rendering, queued, failed
            ),
            colors::ACCENT_RED,
            true,
        )
    } else if rendering > 0 || queued > 0 {
        (
            format!(
                "Render queue: {} done · {} rendering · {} queued",
                done, rendering, queued
            ),
            colors::ACCENT_GREEN,
            true,
        )
    } else {
        (
            format!("Render queue: {} done", done),
            colors::TEXT_MUTED,
            false,
        )
    }
}

fn draw_task_mini(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("Background Tasks")
            .small()
            .color(colors::TEXT_MUTED),
    );
    let (text, color, _) = task_summary(app);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("●").small().color(color));
        ui.label(egui::RichText::new(text).small().color(colors::TEXT_PRIMARY));
    });
    if app.export.is_exporting {
        ui.add(egui::ProgressBar::new(
            app.export.export_progress.clamp(0.0, 1.0),
        ));
    }
}

fn draw_status_strip(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let (text, color, active) = task_summary(app);
        ui.label(egui::RichText::new("●").small().color(color));
        ui.label(
            egui::RichText::new(text)
                .small()
                .color(if active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_MUTED
                }),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(free) = storage_free_text() {
                ui.label(
                    egui::RichText::new(format!("💾 {}", free))
                        .small()
                        .color(colors::TEXT_MUTED),
                );
            }
            ui.label(
                egui::RichText::new("Autosave On")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw_home(app: &mut KagariApp) {
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                draw(app, ctx);
            },
        );
    }

    #[test]
    fn home_renders_resume_workspace_without_selection() {
        let mut app = KagariApp::default();
        app.show_home = true;
        draw_home(&mut app);
        // Nothing selected: inspector falls back to Quick Actions.
        assert!(app.home_selected.is_none());
        assert!(app.home_preview.is_none());
    }

    #[test]
    fn reference_pages_render_at_narrow_reference_size() {
        let mut app = KagariApp::default();
        app.show_home = true;
        let ctx = egui::Context::default();
        for nav in [
            HomeNav::Home,
            HomeNav::Projects,
            HomeNav::Effects,
            HomeNav::Render,
            HomeNav::NewProject,
            HomeNav::Templates,
            HomeNav::Settings,
            HomeNav::Documentation,
        ] {
            set_home_nav(&ctx, nav);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(338.0, 296.0),
                    )),
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
        }
    }

    #[test]
    fn tutorial_navigation_enters_guided_workspace() {
        let mut app = KagariApp::default();
        app.show_home = true;
        let ctx = egui::Context::default();
        set_home_nav(&ctx, HomeNav::Tutorial);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        assert!(!app.show_home);
        assert!(app.show_guided_tutorial);
    }

    #[test]
    fn home_selects_image_and_populates_preview() {
        let mut app = KagariApp::default();
        app.show_home = true;
        // Repo-bundled artwork: resolves when tests run from the package root.
        let logo = std::path::PathBuf::from("assets/kagari_logo.webp");
        if !logo.is_file() {
            return;
        }
        app.home_selected = Some(logo.clone());
        draw_home(&mut app);
        let (path, _) = app.home_preview.as_ref().expect("image preview must load");
        assert_eq!(path, &logo);
    }

    #[test]
    fn home_summarizes_project_files() {
        let path = std::path::PathBuf::from("test_project.json");
        if !path.is_file() {
            return;
        }
        let (comps, layers) = summarize_project(&path).expect("must parse");
        assert!(comps >= 1 && layers >= 1);
        let v = read_project_json(&path).expect("must read");
        let detail = summarize_value(&v).expect("must summarize");
        assert_eq!(detail.comp_names.len(), comps);
        assert_eq!(detail.layers, layers);
        assert!(summarize_project(std::path::Path::new("/nonexistent.json")).is_none());
    }

    #[test]
    fn home_sequence_prefix_groups_frames() {
        assert_eq!(sequence_prefix("shot_010.0004"), "shot_010");
        assert_eq!(sequence_prefix("portal_final_v18"), "portal_final_v18");
        assert_eq!(sequence_prefix("0004"), "");
    }

    #[test]
    fn home_task_summary_is_quiet_when_idle() {
        let app = KagariApp::default();
        let (text, _, active) = task_summary(&app);
        assert!(!active);
        assert!(text.contains("No background tasks"));
    }
}
