//! Home screen: a resume-first production workspace.
//!
//! Layout: slim nav sidebar | resume + recents workspace | inspector-style
//! right panel, with a subtle background-task status strip at the bottom.
//! The filesystem browser and marketing hero are intentionally gone — this
//! screen answers "continue exactly where you left off" within seconds.
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

mod reference_pages;
use reference_pages::*;

mod export_target;
use export_target::*;

fn fmt_age(modified: std::time::SystemTime) -> String {
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

fn is_image_ext(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "exr"
    )
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

pub(crate) fn load_logo_texture(
    ctx: &egui::Context,
    img: image::DynamicImage,
) -> Option<egui::TextureHandle> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let mut pixels = rgba.into_raw();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        if pixel[0].max(pixel[1]).max(pixel[2]) < 18 {
            pixel[3] = 0;
        }
    }
    Some(ctx.load_texture(
        "home:kagari-logo",
        egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels),
        egui::TextureOptions::LINEAR,
    ))
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
    let img = crate::ui::embedded_assets::open_image(path).ok()?;
    let name = format!("thumb:{}", path.display());
    // Reference artwork is already small and is displayed at roughly its native
    // width; keeping the source detail avoids blur when cards grow responsively.
    let handle = load_texture(ctx, &name, img, 512, 256)?;
    let id = handle.id();
    app.home_thumbs.insert(path.to_path_buf(), handle);
    Some(id)
}

// ── Data model ────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HomeNav {
    Home,
    Compositing,
    Graph,
    Color,
    Audio,
    Retiming,
    NewProject,
    Projects,
    Templates,
    Effects,
    Render,
    Settings,
    Tutorial,
    Documentation,
}

fn home_nav(ctx: &egui::Context) -> HomeNav {
    ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("home_nav"), || HomeNav::Home))
}

pub(crate) fn set_home_nav(ctx: &egui::Context, nav: HomeNav) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("home_nav"), nav);
        d.insert_temp(egui::Id::new("home_search_reset"), true);
    });
}

fn target_nav_for_label(label: &str) -> Option<HomeNav> {
    match label {
        "ホーム" => Some(HomeNav::Home),
        "プロジェクトを開く" => Some(HomeNav::Projects),
        "新規プロジェクト" => Some(HomeNav::NewProject),
        "編集" => Some(HomeNav::Compositing),
        "エフェクト" => Some(HomeNav::Effects),
        "素材" => Some(HomeNav::Projects),
        "チュートリアル" => Some(HomeNav::Tutorial),
        "ドキュメント" => Some(HomeNav::Documentation),
        "書き出し" => Some(HomeNav::Render),
        "設定" => Some(HomeNav::Settings),
        _ => None,
    }
}

fn interact_target_nav(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    label: &str,
    id_scope: &str,
) {
    let Some(nav) = target_nav_for_label(label) else {
        return;
    };
    if ui
        .interact(rect, egui::Id::new((id_scope, label)), egui::Sense::click())
        .clicked()
    {
        set_home_nav(ctx, nav);
    }
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
    preview_path: Option<String>,
}

fn read_project_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = crate::core::project_migration::read_bounded_text_file(path, 8_000_000).ok()?;
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
struct RecentProject {
    path: std::path::PathBuf,
    name: String,
    modified: Option<std::time::SystemTime>,
    summary: Option<ProjectSummary>,
}

/// Cached per (path, mtime) so the JSON parse does not repeat every frame.
fn recent_project_entries(ctx: &egui::Context) -> Vec<RecentProject> {
    project_entries_for_paths(ctx, &crate::ui::project_io::recent_projects(), true)
}

fn project_entries_for_paths(
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

// ── Studio entry points ───────────────────────────────────────

fn enter_studio_new_project(app: &mut KagariApp) {
    app.replace_project_history(crate::core::timeline::Project::empty_for_new_project());
    app.mogrt_properties.clear();
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    crate::core::frame_cache::bump_version();
    app.ui_tabs.left_tab_idx = 0;
    app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
    app.show_home = false;
}

pub(crate) fn new_project_from_header(app: &mut KagariApp) {
    enter_studio_new_project(app);
}

fn enter_studio_open_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari VFX Project", &["json"])
        .pick_file()
    {
        open_project_path(app, &path);
    }
}

pub(crate) fn open_project_dialog(app: &mut KagariApp) {
    enter_studio_open_dialog(app);
}

fn open_project_path(app: &mut KagariApp, path: &std::path::Path) {
    if let Err(e) = crate::ui::project_io::open_project_from_path(app, path) {
        app.toasts.error(e);
    }
}

fn new_composition(app: &mut KagariApp) {
    enter_studio_new_project(app);
    app.show_new_comp_dialog = true;
}

pub(crate) fn import_footage_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Footage",
            &[
                "png", "jpg", "jpeg", "webp", "bmp", "exr", "mov", "mp4", "avi", "mkv", "webm",
                "av1", "wav", "mp3",
            ],
        )
        .pick_file()
    {
        open_selected_in_studio(app, ctx, &path);
    }
}

fn open_selected_in_studio(app: &mut KagariApp, ctx: &egui::Context, path: &std::path::Path) {
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
                    crate::core::timeline::LayerType::Image {
                        path: path_str.clone(),
                    },
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
                    name.to_string(),
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
        "mov" | "mp4" | "avi" | "mkv" | "webm" | "av1" => {
            crate::ui::drop_import::start_video_import(app, ctx, path.to_path_buf(), name);
        }
        _ => {
            app.toasts
                .info("Entered studio — use File > Import for unsupported media");
        }
    }
}

// ── Layout ────────────────────────────────────────────────────

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let search_reset_id = egui::Id::new("home_search_reset");
    let should_reset_search =
        ctx.data_mut(|d| d.get_temp::<bool>(search_reset_id).unwrap_or(false));
    if should_reset_search {
        ctx.data_mut(|d| d.remove::<bool>(search_reset_id));
        app.home_search.clear();
    }
    if matches!(home_nav(ctx), HomeNav::Settings) {
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("ae_prefs_category"), 0usize));
        app.show_preferences = true;
        set_home_nav(ctx, HomeNav::Home);
    }
    if matches!(home_nav(ctx), HomeNav::Render) {
        draw_export_target(app, ctx);
        return;
    }
    if matches!(home_nav(ctx), HomeNav::Tutorial) {
        app.show_home = false;
        app.show_guided_tutorial = true;
        app.tutorial_step = 0;
        return;
    }
    // Keep one Home implementation at every window size. Breakpoints may
    // compact its layout, but must not swap the page's content or state.
    let landing_mode = matches!(home_nav(ctx), HomeNav::Home);
    let viewport_width = ctx.screen_rect().width();
    let pixel_scale = ctx
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let home_scale = pixel_scale * (viewport_width / (1584.0 * pixel_scale)).max(1.0);
    let sidebar_width = if landing_mode {
        if viewport_width < 820.0 * home_scale {
            72.0 * home_scale
        } else if viewport_width < 1120.0 * home_scale {
            220.0 * home_scale
        } else {
            viewport_width * 0.1805
        }
    } else if ctx.screen_rect().width() < 1100.0 {
        120.0
    } else {
        136.0
    };
    let show_home_sidebar = viewport_width >= 300.0 * home_scale;
    if show_home_sidebar
        && matches!(
            home_nav(ctx),
            HomeNav::Home
                | HomeNav::Compositing
                | HomeNav::Projects
                | HomeNav::NewProject
                | HomeNav::Documentation
        )
    {
        egui::SidePanel::left("home_nav")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 20, 23)))
            .show(ctx, |ui| {
                if !matches!(home_nav(ctx), HomeNav::Compositing) {
                    draw_landing_sidebar(app, ui, ctx);
                } else {
                    draw_reference_nav(app, ui, ctx);
                }
            });
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_PANEL_DEEP))
        .show(ctx, |ui| {
            sync_selected_preview(app, ctx);
            match home_nav(ctx) {
                HomeNav::Home if landing_mode => draw_landing_home(app, ui, ctx),
                HomeNav::Home
                | HomeNav::Compositing
                | HomeNav::Graph
                | HomeNav::Color
                | HomeNav::Audio
                | HomeNav::Retiming => draw_reference_home(app, ui, ctx),
                HomeNav::NewProject => draw_reference_new_project_page(app, ui, ctx),
                HomeNav::Projects => draw_reference_project_browser_page(app, ui, ctx),
                HomeNav::Templates => draw_reference_templates_page(app, ui, ctx),
                HomeNav::Effects => draw_reference_effects_page(app, ui, ctx),
                HomeNav::Render | HomeNav::Settings => draw_reference_home(app, ui, ctx),
                HomeNav::Documentation => draw_reference_documentation_page(app, ui, ctx),
                HomeNav::Tutorial => draw_reference_home(app, ui, ctx),
            }
        });

    // Export progress needs continuous repaint; idle home stays static.
    if app.export.is_exporting {
        ctx.request_repaint();
    }
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
        app.home_preview = crate::ui::embedded_assets::open_image(&path)
            .ok()
            .and_then(|img| {
                load_texture(ctx, &format!("preview:{}", path.display()), img, 560, 320)
            })
            .map(|handle| (path, handle));
    }
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
        let mut app = KagariApp {
            show_home: true,
            ..Default::default()
        };
        draw_home(&mut app);
        // Nothing selected: inspector falls back to Quick Actions.
        assert!(app.home_selected.is_none());
        assert!(app.home_preview.is_none());
    }

    #[test]
    fn reference_pages_render_at_narrow_reference_size() {
        let mut app = KagariApp {
            show_home: true,
            ..Default::default()
        };
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
        let mut app = KagariApp {
            show_home: true,
            ..Default::default()
        };
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
        let mut app = KagariApp {
            show_home: true,
            ..Default::default()
        };
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
        let v = read_project_json(&path).expect("must read");
        let detail = summarize_value(&v).expect("must summarize");
        assert!(!detail.comp_names.is_empty() && detail.layers >= 1);
        assert!(read_project_json(std::path::Path::new("/nonexistent.json")).is_none());
    }

    #[test]
    fn render_queue_action_deduplicates_active_composition() {
        let mut app = KagariApp::default();
        assert!(queue_active_composition(&mut app));
        assert!(!queue_active_composition(&mut app));
        assert_eq!(app.render_queue_items.len(), 1);
        assert_eq!(app.render_item_status.len(), 1);
    }

    #[test]
    fn narrow_effects_category_click_updates_real_selection_state() {
        let mut app = KagariApp::default();
        let ctx = egui::Context::default();
        set_home_nav(&ctx, HomeNav::Effects);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(338.0, 296.0));

        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        for y in (90..190).step_by(2) {
            let point = egui::pos2(20.0, y as f32);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events: vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
            if ctx.data(|data| {
                data.get_temp::<usize>(egui::Id::new("reference-effect-narrow-category"))
            }) == Some(3)
            {
                break;
            }
        }

        assert_eq!(
            ctx.data(
                |data| data.get_temp::<usize>(egui::Id::new("reference-effect-narrow-category"))
            ),
            Some(3)
        );
    }

    #[test]
    fn target_sidebar_navigation_changes_page_state() {
        assert_eq!(target_nav_for_label("ホーム"), Some(HomeNav::Home));
        assert_eq!(target_nav_for_label("エフェクト"), Some(HomeNav::Effects));
        assert_eq!(target_nav_for_label("終了"), None);

        let mut app = KagariApp::default();
        let ctx = egui::Context::default();
        set_home_nav(&ctx, HomeNav::Render);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 720.0));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        let point = egui::pos2(34.0, 160.0);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        assert_eq!(home_nav(&ctx), HomeNav::Home);
    }

    #[test]
    fn settings_navigation_opens_shared_preferences_at_any_window_size() {
        for size in [egui::vec2(1536.0, 900.0), egui::vec2(900.0, 640.0)] {
            let mut app = KagariApp::default();
            let ctx = egui::Context::default();
            set_home_nav(&ctx, HomeNav::Settings);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );

            assert!(app.show_preferences);
            assert_eq!(home_nav(&ctx), HomeNav::Home);
            assert_eq!(
                ctx.data(|data| data.get_temp::<usize>(egui::Id::new("ae_prefs_category"))),
                Some(0),
                "Settings navigation should open the shared General preferences at any size"
            );
        }
    }
}
