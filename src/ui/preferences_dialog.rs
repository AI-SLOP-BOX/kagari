//! Preferences dialog: performance / history / autosave / audio settings
//! with JSON persistence in $HOME/.kagari_prefs.json.
use crate::ui::kui::{self, ButtonKind};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub language: crate::ui::localization::LanguagePreference,
    pub cache_mb: usize,
    pub undo_steps: usize,
    pub autosave_secs: u64,
    pub audio_preview: bool,
    pub adaptive_preview: bool,
    #[serde(default = "default_disk_cache_gb")]
    pub disk_cache_gb: usize,
    #[serde(default)]
    pub disk_cache_directory: String,
    #[serde(default)]
    pub custom_workspaces: Vec<crate::ui::workspace_manager::SavedWorkspace>,
}

fn default_disk_cache_gb() -> usize {
    50
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            language: crate::ui::localization::LanguagePreference::System,
            cache_mb: 512,
            undo_steps: 50,
            autosave_secs: 30,
            audio_preview: true,
            adaptive_preview: true,
            disk_cache_gb: 50,
            disk_cache_directory: String::new(),
            custom_workspaces: Vec::new(),
        }
    }
}

pub(crate) fn load() -> Prefs {
    crate::ui::project_io::load_prefs_value()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub(crate) fn save(p: &Prefs) -> Result<(), String> {
    let serde_json::Value::Object(fields) = serde_json::to_value(p)
        .map_err(|error| format!("Could not serialize preferences: {error}"))?
    else {
        return Err("Preferences did not serialize to an object".to_owned());
    };
    crate::ui::project_io::update_prefs(|root| {
        root.extend(fields);
    })
}

pub(crate) fn save_workspaces(
    workspaces: &[crate::ui::workspace_manager::SavedWorkspace],
) -> Result<(), String> {
    let mut prefs = load();
    prefs.custom_workspaces = workspaces.to_vec();
    save(&prefs)
}

/// Apply stored prefs to live app state. Called once at startup.
pub fn apply_loaded(app: &mut KagariApp) {
    let p = load();
    apply(app, &p);
}

pub(crate) fn apply(app: &mut KagariApp, p: &Prefs) {
    crate::ui::localization::set_preference(p.language);
    app.frame_cache.max_memory_bytes = p.cache_mb * 1024 * 1024;
    crate::core::frame_cache::disk_cache::set_max_disk_bytes(
        p.disk_cache_gb as u64 * 1024 * 1024 * 1024,
    );
    let cache_path = if p.disk_cache_directory.trim().is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(&p.disk_cache_directory))
    };
    if let Err(error) = crate::core::frame_cache::disk_cache::set_cache_directory(cache_path) {
        app.toasts
            .error(format!("Could not set disk cache folder: {error}"));
    }
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
    match save(&defaults) {
        Ok(()) => app.toasts.info("Preferences reset to defaults"),
        Err(error) => app.toasts.error(error),
    }
}

pub fn draw_preferences_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_preferences {
        return;
    }

    let mut open = true;
    let mut keep_open = true;
    crate::ui::modal::dialog("Preferences")
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
                        crate::ui::kui::caption(ui, "SETTINGS");
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
                            let response =
                                crate::ui::kui::selectable_row(ui, category == index, label, 28.0);
                            if response.clicked() {
                                category = index;
                            }
                        }
                    });
                crate::ui::kui::separator(ui);
                ui.vertical(|ui| {
                    ui.set_min_width((ui.available_width() - 8.0).max(360.0));
                    ui.add_space(2.0);
                    crate::ui::kui::title(
                        ui,
                        [
                            "General",
                            "Performance",
                            "Cache",
                            "CPU",
                            "Color Management",
                            "Auto-save",
                            "UI Appearance",
                            "Keyboard Shortcuts",
                            "Plugins",
                        ][category],
                    );
                    ui.add_space(8.0);

                    crate::ui::kui::section(ui, "Language");
                    let language_options = [
                        (
                            crate::ui::localization::LanguagePreference::System,
                            "System",
                        ),
                        (
                            crate::ui::localization::LanguagePreference::English,
                            "English",
                        ),
                        (
                            crate::ui::localization::LanguagePreference::Japanese,
                            "Japanese",
                        ),
                    ];
                    let language_grid =
                        crate::ui::kui::PropertyGrid::new(ui, &["Display language"]);
                    language_grid.row(ui, "Display language", |ui| {
                        crate::ui::kui::combo(ui, "ui_language", &mut p.language, &language_options)
                    });

                    // ── Performance ──
                    crate::ui::kui::section(ui, "PERFORMANCE");
                    crate::ui::kui::property(ui, "Frame cache budget:", |ui| {
                        kui::scalar_slider(
                            ui,
                            &mut p.cache_mb,
                            128..=2048,
                            " MB",
                            Some(64.0),
                            false,
                        )
                    });
                    crate::ui::kui::toggle(
                        ui,
                        &mut p.adaptive_preview,
                        "Adaptive preview quality (auto-reduce while playing)",
                    )
                    .on_hover_text(crate::ui::localization::tr(
                        "When off, preview always renders at full resolution",
                    ));

                    ui.add_space(6.0);

                    // ── History ──
                    crate::ui::kui::section(ui, "HISTORY");
                    crate::ui::kui::property(ui, "Undo steps:", |ui| {
                        kui::scalar_slider(ui, &mut p.undo_steps, 10..=500, "", None, true)
                    });
                    crate::ui::kui::caption(
                        ui,
                        format!(
                            "{} {} MB",
                            crate::ui::localization::tr("Undo history storage:"),
                            app.history.retained_bytes() / 1024 / 1024
                        ),
                    );

                    ui.add_space(6.0);

                    // ── Autosave ──
                    crate::ui::kui::section(ui, "AUTOSAVE");
                    crate::ui::kui::property(ui, "Interval:", |ui| {
                        kui::scalar_slider(ui, &mut p.autosave_secs, 5..=600, " s", None, false)
                    });

                    ui.add_space(6.0);

                    // ── Audio ──
                    crate::ui::kui::section(ui, "AUDIO");
                    crate::ui::kui::toggle(
                        ui,
                        &mut p.audio_preview,
                        "Preview audio during playback",
                    );

                    ui.add_space(6.0);

                    // ── Media & Disk Cache ──
                    crate::ui::kui::section(ui, "MEDIA & DISK CACHE");
                    crate::ui::kui::property(ui, "Maximum Disk Cache Size:", |ui| {
                        kui::scalar_slider(ui, &mut p.disk_cache_gb, 10..=500, " GB", None, false)
                    });
                    ui.horizontal(|ui| {
                        if kui::button(ui, "Choose Cache Folder...", ButtonKind::Default)
                            .on_hover_text(crate::ui::localization::tr(
                                "Select NVMe / SSD drive location for high-speed frame caching",
                            ))
                            .clicked()
                        {
                            let initial = if p.disk_cache_directory.is_empty() {
                                crate::core::frame_cache::disk_cache::cache_directory()
                            } else {
                                std::path::PathBuf::from(&p.disk_cache_directory)
                            };
                            if let Some(path) =
                                rfd::FileDialog::new().set_directory(initial).pick_folder()
                            {
                                p.disk_cache_directory = path.to_string_lossy().into_owned();
                                app.toasts.info(crate::ui::localization::tr(
                                    "Disk cache folder selected. Save preferences to apply it.",
                                ));
                            }
                        }
                        kui::secondary(
                            ui,
                            if p.disk_cache_directory.is_empty() {
                                crate::core::frame_cache::disk_cache::cache_directory()
                                    .display()
                                    .to_string()
                            } else {
                                p.disk_cache_directory.clone()
                            },
                        );
                        if kui::button(ui, "Empty Disk Cache...", ButtonKind::Default)
                            .on_hover_text(crate::ui::localization::tr(
                                "Purge all rendered cache files from disk",
                            ))
                            .clicked()
                        {
                            crate::core::frame_cache::disk_cache::clear_all();
                            crate::core::frame_cache::bump_version();
                            app.toasts
                                .info(crate::ui::localization::tr("Disk Cache emptied (0 bytes)"));
                        }
                    });

                    ui.add_space(8.0);
                    kui::separator(ui);
                    ui.horizontal(|ui| {
                        if kui::button(ui, "Save", ButtonKind::Accent).clicked() {
                            apply(app, &p);
                            match save(&p) {
                                Ok(()) => app
                                    .toasts
                                    .info(crate::ui::localization::tr("Preferences saved")),
                                Err(error) => app.toasts.error(error),
                            }
                            keep_open = false;
                        }
                        if kui::button(ui, "Cancel", ButtonKind::Default).clicked() {
                            keep_open = false;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(
                                    crate::ui::project_io::prefs_path().display().to_string(),
                                )
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
