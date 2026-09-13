//! Home landing screen: project browser, recent projects, templates,
//! storage overview, and a live preview pane. Mirrors the reference layout:
//! nav sidebar | hero + file browser + templates | preview + metadata.
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
    let secs = modified
        .elapsed()
        .map(|d| d.as_secs())
        .unwrap_or(0);
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

fn volumes() -> Vec<(String, std::path::PathBuf)> {
    let mut out = vec![("System".to_string(), std::path::PathBuf::from("/"))];
    if let Ok(home) = std::env::var("HOME") {
        out.push(("Home".to_string(), std::path::PathBuf::from(home)));
    }
    if let Ok(rd) = std::fs::read_dir("/Volumes") {
        let mut names: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        names.sort();
        for p in names.into_iter().take(4) {
            let label = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Volume".to_string());
            out.push((label, p));
        }
    }
    out
}

fn is_image_ext(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str(),
        "png" | "jpg" | "jpeg"
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

fn thumb_for(app: &mut KagariApp, ctx: &egui::Context, path: &std::path::Path) -> Option<egui::TextureId> {
    if let Some(h) = app.home_thumbs.get(path) {
        return Some(h.id());
    }
    if app.home_thumbs.len() > 96 {
        app.home_thumbs.clear();
    }
    let img = image::open(path).ok()?;
    let name = format!("thumb:{}", path.display());
    let handle = load_texture(ctx, &name, img, 192, 128)?;
    let id = handle.id();
    app.home_thumbs.insert(path.to_path_buf(), handle);
    Some(id)
}

fn banner_texture(app: &mut KagariApp, ctx: &egui::Context) -> Option<egui::TextureId> {
    if let Some(h) = &app.home_banner {
        return Some(h.id());
    }
    let bytes = include_bytes!("../../assets/kagari_logo.png");
    let img = image::load_from_memory(bytes).ok()?;
    let handle = load_texture(ctx, "home_banner", img, 560, 200)?;
    let id = handle.id();
    app.home_banner = Some(handle);
    Some(id)
}

fn enter_studio_new_project(app: &mut KagariApp) {
    app.history =
        crate::core::history::ProjectHistory::new(crate::core::timeline::Project::default());
    app.selection.selected_layer_idx = None;
    app.selection.selected_layers.clear();
    crate::core::frame_cache::bump_version();
    app.show_home = false;
}

fn enter_studio_open_dialog(app: &mut KagariApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Kagari VFX Project", &["json"])
        .pick_file()
    {
        if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
            app.toasts.error(e);
        } else {
            app.show_home = false;
        }
    }
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    // ── Workspace tab strip (Home active; others enter the studio) ──
    egui::TopBottomPanel::top("home_tabs")
        .frame(
            egui::Frame::none()
                .fill(colors::BG_DARKEST)
                .inner_margin(egui::Margin::symmetric(10.0, 2.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                crate::ui::theme::draw_custom_tab(ui, true, "Home");
                ui.add_space(12.0);
                for tab in ["Compositing", "Effects", "Animation", "Color"] {
                    if crate::ui::theme::draw_custom_tab(ui, false, tab).clicked() {
                        app.show_home = false;
                    }
                }
            });
        });

    egui::SidePanel::left("home_nav")
        .resizable(false)
        .default_width(230.0)
        .show(ctx, |ui| {
            draw_nav(app, ui, ctx);
        });

    egui::SidePanel::right("home_preview")
        .resizable(false)
        .default_width(300.0)
        .show(ctx, |ui| {
            draw_preview(app, ui, ctx);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        draw_center(app, ui, ctx);
    });
}

fn draw_nav(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(6.0);
    nav_row(ui, ctx, true, "🏠", "Home", || {});
    nav_row(ui, ctx, false, "📁", "Projects", || {
        app.home_dir = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        app.home_thumbs.clear();
    });
    nav_row(ui, ctx, false, "🖼", "Assets", || {});
    nav_row(ui, ctx, false, "🧩", "Templates", || {});
    ui.add_space(8.0);
    ui.separator();

    ui.label(
        egui::RichText::new("Quick Access")
            .small()
            .strong()
            .color(colors::TEXT_MUTED),
    );
    let mut goto: Option<std::path::PathBuf> = None;
    for (icon, label, dir) in quick_access_dirs() {
        let path = std::path::PathBuf::from(dir);
        if !path.is_dir() {
            continue;
        }
        if ui
            .selectable_label(app.home_dir == path, format!("{} {}", icon, label))
            .clicked()
        {
            goto = Some(path);
        }
    }
    if let Some(dir) = goto {
        app.home_dir = dir;
        app.home_selected = None;
        app.home_thumbs.clear();
        app.home_preview = None;
    }

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Recent Projects")
                .small()
                .strong()
                .color(colors::TEXT_MUTED),
        );
    });
    let recent = crate::ui::project_io::recent_projects();
    if recent.is_empty() {
        ui.weak("No recent projects yet.");
    }
    let mut open_recent: Option<std::path::PathBuf> = None;
    for path_str in recent.iter().take(6) {
        let path = std::path::PathBuf::from(path_str);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        let exists = path.is_file();
        let resp = ui.selectable_label(
            false,
            egui::RichText::new(format!("{}{}", name, if exists { "" } else { " (missing)" }))
                .small()
                .color(if exists {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_MUTED
                }),
        );
        if exists && resp.clicked() {
            open_recent = Some(path);
        }
        if !exists {
            resp.on_hover_text(path_str.clone());
        }
    }
    if let Some(path) = open_recent {
        if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
            app.toasts.error(e);
        } else {
            app.show_home = false;
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(
        egui::RichText::new("Storage Locations")
            .small()
            .strong()
            .color(colors::TEXT_MUTED),
    );
    for (label, path) in volumes() {
        if let Some((used, total)) = disk_usage(&path) {
            let frac = if total > 0 {
                used as f32 / total as f32
            } else {
                0.0
            };
            ui.label(
                egui::RichText::new(format!("{}  {} / {}", label, fmt_bytes(used), fmt_bytes(total)))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_width(ui.available_width())
                    .fill(egui::Color32::from_rgb(70, 135, 190)),
            );
        }
    }
    let _ = ctx;
}

fn nav_row(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    active: bool,
    icon: &str,
    label: &str,
    mut on_click: impl FnMut(),
) {
    let resp = ui.selectable_label(
        active,
        egui::RichText::new(format!("{}  {}", icon, label))
            .strong()
            .color(if active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            }),
    );
    if resp.clicked() {
        on_click();
    }
}

fn quick_access_dirs() -> Vec<(&'static str, &'static str, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    vec![
        ("🖥", "Desktop", format!("{}/Desktop", home)),
        ("📄", "Documents", format!("{}/Documents", home)),
        ("🎬", "Movies", format!("{}/Movies", home)),
        ("🏠", "Home", home),
        ("/", "Root", "/".to_string()),
    ]
}

fn draw_center(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical()
        .id_salt("home_center")
        .show(ui, |ui| {
            draw_hero(app, ui, ctx);
            ui.add_space(10.0);
            draw_browser(app, ui, ctx);
            ui.add_space(10.0);
            draw_templates(app, ui, ctx);
        });
}

fn draw_hero(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(18.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .rounding(egui::Rounding::same(8.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("BRING WORLDS TO LIFE")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
                ui.heading(
                    egui::RichText::new("Kagari VFX")
                        .size(30.0)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
                ui.label(
                    egui::RichText::new("Tools for Artists. Built for Bigger Worlds.")
                        .color(colors::TEXT_SECONDARY),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if crate::ui::custom_widgets::ae_button_accent(ui, "📁  New Project")
                        .clicked()
                    {
                        enter_studio_new_project(app);
                    }
                    if crate::ui::custom_widgets::ae_button(ui, "Open Project")
                        .clicked()
                    {
                        enter_studio_open_dialog(app);
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(id) = banner_texture(app, ctx) {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        id,
                        egui::vec2(300.0, 108.0),
                    )));
                }
            });
        });
    });
}

#[derive(Clone)]
struct DirEntry {
    path: std::path::PathBuf,
    is_dir: bool,
    size: u64,
}

fn list_dir(dir: &std::path::Path, query: &str) -> (Vec<DirEntry>, Vec<DirEntry>) {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (dirs, files);
    };
    let q = query.to_lowercase();
    for entry in rd.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.starts_with('.') {
            continue;
        }
        if !q.is_empty() && !name.to_lowercase().contains(&q) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let rec = DirEntry {
            path,
            is_dir: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
            size,
        };
        if rec.is_dir {
            dirs.push(rec);
        } else {
            files.push(rec);
        }
    }
    dirs.sort_by(|a, b| a.path.cmp(&b.path));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    (dirs, files)
}

fn draw_browser(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.heading("Project Browser");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.home_search)
                    .hint_text("🔍 Search")
                    .desired_width(200.0),
            );
        });
    });

    // Breadcrumbs
    ui.horizontal_wrapped(|ui| {
        let mut prefix = std::path::PathBuf::new();
        let mut crumbs: Vec<(String, std::path::PathBuf)> = Vec::new();
        for comp in app.home_dir.components() {
            prefix.push(comp.as_os_str());
            let label = if crumbs.is_empty() {
                prefix.display().to_string()
            } else {
                comp.as_os_str().to_string_lossy().to_string()
            };
            crumbs.push((label, prefix.clone()));
        }
        let mut goto: Option<std::path::PathBuf> = None;
        for (i, (label, path)) in crumbs.iter().enumerate() {
            if i > 0 {
                ui.label(egui::RichText::new("›").color(colors::TEXT_MUTED));
            }
            if ui.small_button(label).clicked() {
                goto = Some(path.clone());
            }
        }
        if let Some(dir) = goto {
            app.home_dir = dir;
            app.home_selected = None;
            app.home_thumbs.clear();
            app.home_preview = None;
        }
    });
    ui.add_space(4.0);

    let (dirs, files) = list_dir(&app.home_dir, &app.home_search.clone());
    if dirs.is_empty() && files.is_empty() {
        ui.weak("Empty folder.");
        return;
    }

    // Fixed-size cards with visibility probes: off-screen cards cost nothing.
    const CARD_W: f32 = 148.0;
    const CARD_H: f32 = 118.0;
    let mut pending_dir: Option<std::path::PathBuf> = None;
    let per_row = ((ui.available_width() / (CARD_W + 8.0)).floor() as usize).max(1);
    let mut row: Vec<DirEntry> = Vec::with_capacity(per_row);
    let mut flush = |ui: &mut egui::Ui, row: &mut Vec<DirEntry>| {
        ui.horizontal(|ui| {
            for entry in row.drain(..) {
                draw_card(app, ui, ctx, &entry, &mut pending_dir, CARD_W, CARD_H);
            }
        });
    };
    for entry in dirs.into_iter().chain(files) {
        row.push(entry);
        if row.len() >= per_row {
            flush(ui, &mut row);
        }
    }
    if !row.is_empty() {
        flush(ui, &mut row);
    }
    if let Some(dir) = pending_dir {
        app.home_dir = dir;
        app.home_selected = None;
        app.home_thumbs.clear();
        app.home_preview = None;
    }
}

fn draw_card(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    entry: &DirEntry,
    pending_dir: &mut Option<std::path::PathBuf>,
    w: f32,
    h: f32,
) {
    let probe = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(w, h));
    if !ui.is_rect_visible(probe) {
        ui.add_space(h);
        return;
    }
    let name = entry
        .path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let selected = app.home_selected.as_deref() == Some(entry.path.as_path());
    // Fixed-size card: allocate the rect first so content can never stretch it.
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        6.0,
        if selected {
            colors::BG_HOVER
        } else {
            colors::BG_DARK
        },
    );
    ui.painter().rect_stroke(
        rect,
        6.0,
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
            .max_rect(rect.shrink(6.0))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    if entry.is_dir {
        child.label(egui::RichText::new("📁").size(30.0));
    } else if is_image_ext(&entry.path) {
        if let Some(id) = thumb_for(app, ctx, &entry.path) {
            child.add(egui::Image::new(egui::load::SizedTexture::new(
                id,
                egui::vec2(96.0, 56.0),
            )));
        } else {
            child.label(egui::RichText::new("🖼").size(30.0));
        }
    } else {
        child.label(egui::RichText::new("📄").size(30.0));
    }
    child.add(
        egui::Label::new(egui::RichText::new(&name).small().color(colors::TEXT_PRIMARY))
            .truncate(),
    );
    let meta = if entry.is_dir {
        "Folder".to_string()
    } else {
        fmt_bytes(entry.size)
    };
    child.label(egui::RichText::new(meta).small().color(colors::TEXT_MUTED));
    if resp.clicked() {
        app.home_selected = Some(entry.path.clone());
        app.home_preview = None;
    }
    if resp.double_clicked() && entry.is_dir {
        *pending_dir = Some(entry.path.clone());
    }
}

fn draw_templates(app: &mut KagariApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.heading("Project Templates");
    });
    ui.horizontal(|ui| {
        template_card(
            ui,
            "➕",
            "Blank Project",
            "Start from scratch",
            || {
                enter_studio_new_project(app);
            },
        );
        template_card(
            ui,
            "🎬",
            "Demo Scene",
            "Animated showcase",
            || {
                crate::ui::demo_scene::build(app);
                app.show_home = false;
            },
        );
        template_card(ui, "🎞", "Cinematic VFX", "Film & trailer work", || {
            crate::ui::demo_scene::build(app);
            app.show_home = false;
        });
    });
}

fn template_card(ui: &mut egui::Ui, icon: &str, title: &str, desc: &str, mut on_open: impl FnMut()) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(202.0, 98.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 6.0, colors::BG_DARK);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(10.0))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    child.label(egui::RichText::new(icon).size(24.0));
    child.label(egui::RichText::new(title).strong().color(colors::TEXT_PRIMARY));
    child.label(egui::RichText::new(desc).small().color(colors::TEXT_MUTED));
    if resp.clicked() {
        on_open();
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

fn draw_preview(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Preview")
            .strong()
            .color(colors::TEXT_PRIMARY),
    );
    ui.separator();
    let Some(path) = app.home_selected.clone() else {
        ui.weak("Select a file to preview.");
        draw_tools(app, ui);
        return;
    };
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Image preview (loaded once per selection).
    let preview_id = if is_image_ext(&path) {
        let stale = app
            .home_preview
            .as_ref()
            .map(|(p, _)| p != &path)
            .unwrap_or(true);
        if stale {
            app.home_preview = image::open(&path)
                .ok()
                .and_then(|img| load_texture(ctx, &format!("preview:{}", path.display()), img, 560, 320))
                .map(|h| (path.clone(), h));
        }
        app.home_preview.as_ref().map(|(_, h)| h.id())
    } else {
        None
    };
    if let Some(id) = preview_id {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(
            id,
            egui::vec2(ui.available_width(), 170.0),
        )));
    } else {
        ui.label(egui::RichText::new("🖼").size(44.0));
    }

    ui.label(egui::RichText::new(&name).strong().size(14.0));
    ui.separator();

    // Metadata rows (all live filesystem / project data).
    egui::Grid::new("home_meta")
        .num_columns(2)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            let mut row = |k: &str, v: String| {
                ui.label(egui::RichText::new(k).small().color(colors::TEXT_MUTED));
                ui.label(egui::RichText::new(v).small().color(colors::TEXT_PRIMARY));
                ui.end_row();
            };
            let ext = path
                .extension()
                .map(|s| s.to_string_lossy().to_uppercase())
                .unwrap_or_default();
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
                if let Some((comps, layers)) = summarize_project(&path) {
                    row("Compositions", format!("{}", comps));
                    row("Layers", format!("{}", layers));
                }
            }
            row("Location", path.display().to_string());
        });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("📂 Open in Studio").clicked() {
            open_selected_in_studio(app, &path);
        }
    });

    ui.add_space(8.0);
    ui.separator();
    draw_tools(app, ui);
}

fn summarize_project(path: &std::path::Path) -> Option<(usize, usize)> {
    let text = std::fs::read_to_string(path).ok()?;
    if text.len() > 8_000_000 {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let comps = v.get("compositions")?.as_array()?;
    let layers: usize = comps
        .iter()
        .filter_map(|c| c.get("layers")?.as_array().map(|l| l.len()))
        .sum();
    Some((comps.len(), layers))
}

fn open_selected_in_studio(app: &mut KagariApp, path: &std::path::Path) {
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "json" {
        if let Err(e) = crate::ui::project_io::open_project_from_path(app, path) {
            app.toasts.error(e);
        } else {
            app.show_home = false;
        }
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

fn draw_tools(app: &mut KagariApp, ui: &mut egui::Ui) {    ui.label(
        egui::RichText::new("Tools & Integrations")
            .small()
            .strong()
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
    fn home_renders_banner_and_tools_without_selection() {
        let mut app = KagariApp::default();
        app.show_home = true;
        app.home_dir = std::env::temp_dir();
        draw_home(&mut app);
        // Banner art decoded and cached; no thumbnails needed for empty tmp.
        assert!(app.home_banner.is_some());
        assert!(app.home_preview.is_none());
    }

    #[test]
    fn home_selects_image_and_populates_preview() {
        let mut app = KagariApp::default();
        app.show_home = true;
        // Repo-bundled artwork: resolves when tests run from the package root.
        let logo = std::path::PathBuf::from("assets/kagari_logo.png");
        if !logo.is_file() {
            return;
        }
        app.home_dir = std::path::PathBuf::from("assets");
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
        assert!(summarize_project(std::path::Path::new("/nonexistent.json")).is_none());
    }

    #[test]
    fn home_disk_usage_reports_this_filesystem() {
        let (used, total) = disk_usage(std::path::Path::new("/")).expect("statvfs must work");
        assert!(total > 0 && used <= total);
    }
}
