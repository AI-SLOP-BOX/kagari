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
    let dialog_title = crate::ui::localization::tr("Preferences");
    crate::ui::modal::dialog(dialog_title)
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(760.0, 560.0))
        .min_size(egui::vec2(620.0, 440.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let id = egui::Id::new("ae_prefs_draft");
            let mut p = ctx.data_mut(|d| d.get_temp::<Prefs>(id).unwrap_or_else(load));
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum PrefCategory {
                General,
                Performance,
                Cache,
                AutoSave,
                UiAppearance,
                Audio,
                ColorManagement,
                KeyboardShortcuts,
                Plugins,
            }

            impl PrefCategory {
                const ALL: &'static [PrefCategory] = &[
                    PrefCategory::General,
                    PrefCategory::Performance,
                    PrefCategory::Cache,
                    PrefCategory::AutoSave,
                    PrefCategory::UiAppearance,
                    PrefCategory::Audio,
                    PrefCategory::ColorManagement,
                    PrefCategory::KeyboardShortcuts,
                    PrefCategory::Plugins,
                ];

                fn title(&self) -> &'static str {
                    match self {
                        PrefCategory::General => "General",
                        PrefCategory::Performance => "Performance",
                        PrefCategory::Cache => "Cache",
                        PrefCategory::AutoSave => "Auto-save",
                        PrefCategory::UiAppearance => "UI Appearance",
                        PrefCategory::Audio => "Audio",
                        PrefCategory::ColorManagement => "Color Management",
                        PrefCategory::KeyboardShortcuts => "Keyboard Shortcuts",
                        PrefCategory::Plugins => "Plugins",
                    }
                }
            }

            let category_id = egui::Id::new("ae_prefs_category_v2");
            let mut current_category = ctx
                .data_mut(|d| d.get_temp::<usize>(category_id))
                .and_then(|idx| PrefCategory::ALL.get(idx).copied())
                .unwrap_or(PrefCategory::General);

            ui.horizontal_top(|ui| {
                // Left Column: Category Sidebar
                egui::Frame::none()
                    .fill(colors::BG_DARKEST)
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(170.0);
                        ui.set_min_height(380.0);
                        crate::ui::kui::caption(ui, crate::ui::localization::tr("Settings"));
                        ui.add_space(8.0);
                        for &cat in PrefCategory::ALL {
                            let tr_label = crate::ui::localization::tr(cat.title());
                            let response = crate::ui::kui::selectable_row(
                                ui,
                                current_category == cat,
                                &tr_label,
                                30.0,
                            );
                            if response.clicked() {
                                current_category = cat;
                            }
                        }
                    });

                ui.add_space(8.0);

                // Vertical Divider
                let (div_r, _) = ui.allocate_exact_size(egui::vec2(1.0, 420.0), egui::Sense::hover());
                ui.painter().line_segment(
                    [div_r.center_top(), div_r.center_bottom()],
                    egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                );

                ui.add_space(12.0);

                // Right Column: Settings Content
                ui.vertical(|ui| {
                    let content_w = (ui.available_width() - 8.0).max(340.0);
                    ui.set_width(content_w);
                    ui.add_space(2.0);
                    let cat_title = crate::ui::localization::tr(current_category.title());
                    crate::ui::kui::title(ui, &cat_title);
                    ui.add_space(12.0);

                    match current_category {
                        PrefCategory::General => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Language"));
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
                                    "日本語",
                                ),
                            ];
                            let language_grid =
                                crate::ui::kui::PropertyGrid::new(ui, &[&crate::ui::localization::tr("Display language")]);
                            language_grid.row(ui, &crate::ui::localization::tr("Display language"), |ui| {
                                let changed = crate::ui::kui::combo(ui, "ui_language", &mut p.language, &language_options);
                                if changed {
                                    crate::ui::localization::set_preference(p.language);
                                    ui.ctx().request_repaint();
                                }
                                changed
                            });

                            ui.add_space(12.0);
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Undo History"));
                            crate::ui::kui::property(ui, &crate::ui::localization::tr("Undo steps:"), |ui| {
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
                        }

                        PrefCategory::Performance => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Performance"));
                            crate::ui::kui::property(ui, &crate::ui::localization::tr("Frame cache budget:"), |ui| {
                                kui::scalar_slider(
                                    ui,
                                    &mut p.cache_mb,
                                    128..=2048,
                                    " MB",
                                    Some(64.0),
                                    false,
                                )
                            });
                            ui.add_space(6.0);
                            crate::ui::kui::toggle(
                                ui,
                                &mut p.adaptive_preview,
                                &crate::ui::localization::tr("Adaptive preview quality (auto-reduce while playing)"),
                            )
                            .on_hover_text(crate::ui::localization::tr(
                                "When off, preview always renders at full resolution",
                            ));
                        }

                        PrefCategory::Cache => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Cache"));
                            crate::ui::kui::property(ui, &crate::ui::localization::tr("Maximum Disk Cache Size:"), |ui| {
                                kui::scalar_slider(ui, &mut p.disk_cache_gb, 10..=500, " GB", None, false)
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if kui::button(ui, &crate::ui::localization::tr("Choose Cache Folder..."), ButtonKind::Default)
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
                            });
                            ui.add_space(8.0);
                            if kui::button(ui, &crate::ui::localization::tr("Empty Disk Cache..."), ButtonKind::Default)
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
                        }

                        PrefCategory::AutoSave => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Auto-save"));
                            crate::ui::kui::property(ui, &crate::ui::localization::tr("Interval:"), |ui| {
                                kui::scalar_slider(ui, &mut p.autosave_secs, 5..=600, " s", None, false)
                            });
                        }

                        PrefCategory::UiAppearance => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("UI Appearance"));
                            ui.label(
                                egui::RichText::new("Dark Studio (After Effects Pro Dark)")
                                    .size(13.0)
                                    .color(colors::TEXT_PRIMARY),
                            );
                            ui.label(
                                egui::RichText::new("Kagari VFX の標準テーマです。最適なコントラストと視認性を提供します。")
                                    .size(11.5)
                                    .color(colors::TEXT_MUTED),
                            );
                        }

                        PrefCategory::Audio => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Audio"));
                            crate::ui::kui::toggle(
                                ui,
                                &mut p.audio_preview,
                                &crate::ui::localization::tr("Preview audio during playback"),
                            );
                        }

                        PrefCategory::ColorManagement => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Color Management"));
                            ui.label(
                                egui::RichText::new("Working Color Space: Rec.709 / sRGB (Standard Gamma 2.4)")
                                    .size(13.0)
                                    .color(colors::TEXT_PRIMARY),
                            );
                            ui.label(
                                egui::RichText::new("OCIO / ACES および 32-bit Float リニアレンダリング パイプラインが有効です。")
                                    .size(11.5)
                                    .color(colors::TEXT_MUTED),
                            );
                        }

                        PrefCategory::KeyboardShortcuts => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Keyboard Shortcuts"));
                            ui.label(
                                egui::RichText::new("After Effects 互換キーバインド")
                                    .size(13.0)
                                    .color(colors::TEXT_PRIMARY),
                            );
                            ui.add_space(4.0);
                            if kui::button(ui, &crate::ui::localization::tr("Keyboard Shortcuts Reference..."), ButtonKind::Default).clicked() {
                                app.show_shortcuts_dialog = true;
                            }
                        }

                        PrefCategory::Plugins => {
                            crate::ui::kui::section(ui, &crate::ui::localization::tr("Plugins"));
                            ui.label(
                                egui::RichText::new("After Effects AEGP / OpenFX Plugin Host")
                                    .size(13.0)
                                    .color(colors::TEXT_PRIMARY),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("/Library/OFX/Plugins および AEプラグイン ディレクトリを自動スキャンします。")
                                    .size(11.5)
                                    .color(colors::TEXT_MUTED),
                            );
                        }
                    }

                    ui.add_space(16.0);
                    kui::separator(ui);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if kui::button(ui, &crate::ui::localization::tr("Save"), ButtonKind::Accent).clicked() {
                            apply(app, &p);
                            match save(&p) {
                                Ok(()) => app
                                    .toasts
                                    .info(crate::ui::localization::tr("Preferences saved")),
                                Err(error) => app.toasts.error(error),
                            }
                            keep_open = false;
                        }
                        if kui::button(ui, &crate::ui::localization::tr("Cancel"), ButtonKind::Default).clicked() {
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
            let cat_idx = PrefCategory::ALL
                .iter()
                .position(|c| *c == current_category)
                .unwrap_or(0);
            ctx.data_mut(|d| d.insert_temp(category_id, cat_idx));
        });

    app.show_preferences = open && keep_open;
}
