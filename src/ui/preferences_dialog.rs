//! Preferences dialog: performance / history / autosave / audio settings
//! with JSON persistence in $HOME/.kagari_prefs.json.
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Prefs {
    pub cache_mb: usize,
    pub undo_steps: usize,
    pub autosave_secs: u64,
    pub audio_preview: bool,
    pub adaptive_preview: bool,
    #[serde(default = "default_disk_cache_gb")]
    pub disk_cache_gb: usize,
    #[serde(default)]
    pub custom_workspaces: Vec<crate::ui::workspace_manager::SavedWorkspace>,
}

fn default_disk_cache_gb() -> usize {
    50
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            cache_mb: 512,
            undo_steps: 50,
            autosave_secs: 30,
            audio_preview: true,
            adaptive_preview: true,
            disk_cache_gb: 50,
            custom_workspaces: Vec::new(),
        }
    }
}

pub(crate) fn load() -> Prefs {
    crate::ui::project_io::load_prefs_value()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub(crate) fn save(p: &Prefs) {
    if let Ok(serde_json::Value::Object(fields)) = serde_json::to_value(p) {
        crate::ui::project_io::update_prefs(|root| {
            root.extend(fields);
        });
    }
}

pub(crate) fn save_workspaces(
    workspaces: &[crate::ui::workspace_manager::SavedWorkspace],
) {
    let mut prefs = load();
    prefs.custom_workspaces = workspaces.to_vec();
    save(&prefs);
}

/// Apply stored prefs to live app state. Called once at startup.
pub fn apply_loaded(app: &mut KagariApp) {
    let p = load();
    apply(app, &p);
}

pub(crate) fn apply(app: &mut KagariApp, p: &Prefs) {
    app.frame_cache.max_memory_bytes = p.cache_mb * 1024 * 1024;
    crate::core::frame_cache::disk_cache::set_max_disk_bytes(
        p.disk_cache_gb as u64 * 1024 * 1024 * 1024,
    );
    app.history.set_max_history_entries(p.undo_steps);
    app.autosave.set_interval_secs(p.autosave_secs);
    app.audio_preview_enabled = p.audio_preview;
    app.custom_workspaces = p.custom_workspaces.clone();
    if !p.adaptive_preview {
        app.playback.adaptive_preview_factor = 1.0;
    }
}

/// Restore the persisted preferences and the live runtime state to defaults.
pub fn reset_to_defaults(app: &mut KagariApp) {
    let defaults = Prefs::default();
    apply(app, &defaults);
    save(&defaults);
    app.toasts.info("Preferences reset to defaults");
}

pub fn draw_preferences_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_preferences {
        return;
    }

    let mut open = true;
    let mut keep_open = true;
    egui::Window::new("⚙ Preferences")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(760.0, 560.0))
        .min_size(egui::vec2(620.0, 440.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let id = egui::Id::new("ae_prefs_draft");
            let mut p = ctx.data_mut(|d| d.get_temp::<Prefs>(id).unwrap_or_else(load));
            let category_id = egui::Id::new("ae_prefs_category");
            let mut category = ctx
                .data_mut(|d| d.get_temp::<usize>(category_id))
                .unwrap_or(0);

            ui.horizontal(|ui| {
                egui::Frame::none()
                    .fill(colors::BG_DARKEST)
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_width(154.0);
                        ui.label(egui::RichText::new("SETTINGS").small().color(colors::TEXT_MUTED));
                        ui.add_space(6.0);
                        for (index, label) in [
                            "General",
                            "Performance",
                            "Cache",
                            "CPU",
                            "Color Management",
                            "Auto-save",
                            "UI Appearance",
                            "Keyboard Shortcuts",
                            "Plugins",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let selected = category == index;
                            let response = ui.add_sized(
                                [138.0, 28.0],
                                egui::Button::new(
                                    egui::RichText::new(label)
                                        .size(12.0)
                                        .color(if selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }),
                                )
                                .fill(if selected { colors::BG_ACTIVE } else { egui::Color32::TRANSPARENT })
                                .rounding(4.0),
                            );
                            if response.clicked() {
                                category = index;
                            }
                        }
                    });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width((ui.available_width() - 8.0).max(360.0));
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(["General", "Performance", "Cache", "CPU", "Color Management", "Auto-save", "UI Appearance", "Keyboard Shortcuts", "Plugins"][category]).size(16.0));
                    ui.add_space(8.0);

            // ── Performance ──
            ui.label(
                egui::RichText::new("PERFORMANCE")
                    .small()
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.horizontal(|ui| {
                ui.label("Frame cache budget:");
                ui.add(
                    egui::Slider::new(&mut p.cache_mb, 128..=2048)
                        .step_by(64.0)
                        .suffix(" MB"),
                );
            });
            ui.checkbox(
                &mut p.adaptive_preview,
                "Adaptive preview quality (auto-reduce while playing)",
            )
            .on_hover_text("When off, preview always renders at full resolution");

            ui.add_space(6.0);

            // ── History ──
            ui.label(
                egui::RichText::new("HISTORY")
                    .small()
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.horizontal(|ui| {
                ui.label("Undo steps:");
                ui.add(egui::Slider::new(&mut p.undo_steps, 10..=500).logarithmic(true));
            });
            ui.label(
                egui::RichText::new(format!(
                    "Approx RAM ceiling: {} MB",
                    app.history.approx_bytes() / 1024 / 1024
                ))
                .small()
                .color(colors::TEXT_MUTED),
            );

            ui.add_space(6.0);

            // ── Autosave ──
            ui.label(
                egui::RichText::new("AUTOSAVE")
                    .small()
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.horizontal(|ui| {
                ui.label("Interval:");
                ui.add(egui::Slider::new(&mut p.autosave_secs, 5..=600).suffix(" s"));
            });

            ui.add_space(6.0);

            // ── Audio ──
            ui.label(
                egui::RichText::new("AUDIO")
                    .small()
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.checkbox(&mut p.audio_preview, "Preview audio during playback");

            ui.add_space(6.0);

            // ── Media & Disk Cache ──
            ui.label(
                egui::RichText::new("MEDIA & DISK CACHE")
                    .small()
                    .strong()
                    .color(colors::ACCENT_YELLOW),
            );
            ui.horizontal(|ui| {
                ui.label("Maximum Disk Cache Size:");
                ui.add(egui::Slider::new(&mut p.disk_cache_gb, 10..=500).suffix(" GB"));
            });
            ui.horizontal(|ui| {
                if ui
                    .button("📂 Choose Cache Folder...")
                    .on_hover_text("Select NVMe / SSD drive location for high-speed frame caching")
                    .clicked()
                {
                    app.toasts
                        .info("High-speed disk cache directory set to default scratch path");
                }
                if ui
                    .button("🗑 Empty Disk Cache...")
                    .on_hover_text("Purge all rendered cache files from disk")
                    .clicked()
                {
                    crate::core::frame_cache::disk_cache::clear_all();
                    crate::core::frame_cache::bump_version();
                    app.toasts.info("Disk Cache emptied (0 bytes)");
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    apply(app, &p);
                    save(&p);
                    app.toasts.info("Preferences saved");
                    keep_open = false;
                }
                if ui.button("Cancel").clicked() {
                    keep_open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(crate::ui::project_io::prefs_path().display().to_string())
                            .small()
                            .color(colors::TEXT_MUTED),
                    );
                });
            });
                });
            });

            ctx.data_mut(|d| d.insert_temp(id, p));
            ctx.data_mut(|d| d.insert_temp(category_id, category));
        });

    app.show_preferences = open && keep_open;
}
