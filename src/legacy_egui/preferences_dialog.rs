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
    #[serde(default = "default_recovery_slots")]
    pub recovery_slots: usize,
    pub audio_preview: bool,
    pub adaptive_preview: bool,
    #[serde(default = "default_disk_cache_gb")]
    pub disk_cache_gb: usize,
    #[serde(default)]
    pub disk_cache_directory: String,
    #[serde(default)]
    pub custom_workspaces: Vec<crate::ui::workspace_manager::SavedWorkspace>,
    #[serde(default)]
    pub shortcut_bindings:
        std::collections::HashMap<String, crate::ui::shortcuts::defs::ShortcutBinding>,
    #[serde(default = "default_startup_home")]
    pub startup_home: bool,
    #[serde(default = "default_true")]
    pub confirm_exit: bool,
    #[serde(default = "default_true")]
    pub confirm_close_project: bool,
    #[serde(default = "default_true")]
    pub restore_previous_state: bool,
    #[serde(default)]
    pub gpu_compute_enabled: Option<bool>,
}

fn default_disk_cache_gb() -> usize {
    100
}
fn default_startup_home() -> bool {
    true
}
fn default_true() -> bool {
    true
}
fn default_recovery_slots() -> usize {
    10
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            language: crate::ui::localization::LanguagePreference::System,
            cache_mb: 12 * 1024,
            undo_steps: 50,
            autosave_secs: 300,
            recovery_slots: 10,
            audio_preview: true,
            adaptive_preview: true,
            disk_cache_gb: 100,
            disk_cache_directory: String::new(),
            custom_workspaces: Vec::new(),
            shortcut_bindings: std::collections::HashMap::new(),
            startup_home: true,
            confirm_exit: true,
            confirm_close_project: true,
            restore_previous_state: true,
            gpu_compute_enabled: None,
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
    app.show_home = p.startup_home;
}

pub(crate) fn apply(app: &mut KagariApp, p: &Prefs) {
    crate::ui::localization::set_preference(p.language);
    crate::ui::shortcuts::defs::set_overrides(&p.shortcut_bindings);
    app.confirm_exit = p.confirm_exit;
    app.confirm_close_project = p.confirm_close_project;
    app.restore_previous_state = p.restore_previous_state;
    if let Some(enabled) = p.gpu_compute_enabled {
        if app.history.current().use_gpu_compute != enabled {
            app.modify_project(|project| project.use_gpu_compute = enabled);
        }
    }
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
    app.autosave.set_max_slots(p.recovery_slots);
    app.audio_preview_enabled = p.audio_preview;
    app.adaptive_preview_enabled = p.adaptive_preview;
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
    let mut open_comp_settings = false;
    let id = egui::Id::new("ae_prefs_draft");
    let dialog_title = crate::ui::localization::tr("Preferences");
    let screen_size = ctx.screen_rect().size();
    let dialog_size = egui::vec2(
        (screen_size.x - 24.0).clamp(320.0, 900.0),
        (screen_size.y - 24.0).clamp(280.0, 560.0),
    );
    crate::ui::modal::dialog(dialog_title)
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(dialog_size)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let mut p = ctx.data_mut(|d| d.get_temp::<Prefs>(id).unwrap_or_else(load));
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum PrefCategory {
                General,
                Projects,
                Cache,
                Performance,
                KeyboardShortcuts,
                UiAppearance,
                Plugins,
                License,
            }

            impl PrefCategory {
                const ALL: &'static [Self] = &[
                    Self::General,
                    Self::Projects,
                    Self::Cache,
                    Self::Performance,
                    Self::KeyboardShortcuts,
                    Self::UiAppearance,
                    Self::Plugins,
                    Self::License,
                ];

                fn title(self) -> &'static str {
                    match self {
                        Self::General => "General",
                        Self::Performance => "Performance",
                        Self::Cache => "Media & Cache",
                        Self::UiAppearance => "Interface",
                        Self::Projects => "Projects",
                        Self::KeyboardShortcuts => "Keyboard Shortcuts",
                        Self::Plugins => "Plugins",
                        Self::License => "License",
                    }
                }

                fn nav_title(self) -> &'static str {
                    match self {
                        Self::General => "一般",
                        Self::Performance => "パフォーマンス",
                        Self::Cache => "メディアとキャッシュ",
                        Self::UiAppearance => "インターフェース",
                        Self::Projects => "プロジェクト",
                        Self::KeyboardShortcuts => "ショートカット",
                        Self::Plugins => "プラグイン",
                        Self::License => "ライセンス",
                    }
                }

                fn icon(self) -> &'static str {
                    const SVG_DATABASE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="8.5" ry="3"/><path d="M3.5 5v14c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3V5M3.5 12c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3"/></svg>"#;
                    const SVG_BOLT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2 4 14h7l-1 8 10-13h-7z"/></svg>"#;
                    const SVG_MONITOR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="14" rx="1.5"/><path d="M8 21h8m-4-4v4"/></svg>"#;
                    const SVG_PUZZLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h5a3 3 0 1 1 6 0h5v5a3 3 0 1 0 0 6v5h-5a3 3 0 1 0-6 0H4v-5a3 3 0 1 0 0-6z"/></svg>"#;
                    match self {
                        Self::General => crate::ui::icons::SVG_SETTINGS,
                        Self::Performance => SVG_BOLT,
                        Self::Cache => SVG_DATABASE,
                        Self::Projects => crate::ui::icons::SVG_FOLDER,
                        Self::UiAppearance => SVG_MONITOR,
                        Self::KeyboardShortcuts => crate::ui::icons::SVG_KEYBOARD,
                        Self::Plugins => SVG_PUZZLE,
                        Self::License => crate::ui::icons::SVG_LOCK,
                    }
                }
            }

            let category_id = egui::Id::new("ae_prefs_category_v3");
            let mut current_category = ctx
                .data_mut(|d| d.get_temp::<usize>(category_id))
                .and_then(|idx| PrefCategory::ALL.get(idx).copied())
                .unwrap_or(PrefCategory::General);

            ui.vertical(|ui| {
                // Preference controls are information dense; keep their labels
                // comfortably readable at normal desktop scaling.
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::proportional(15.0),
                );
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Button,
                    egui::FontId::proportional(15.0),
                );
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Small,
                    egui::FontId::proportional(13.0),
                );
                egui::Frame::none()
                    .fill(colors::BG_PANEL_DEEP)
                    .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let gear = egui::Image::from_bytes(
                                "bytes://preferences-header-settings.svg",
                                crate::ui::icons::SVG_SETTINGS.as_bytes(),
                            ).fit_to_exact_size(egui::vec2(20.0, 20.0));
                            ui.add(gear);
                            ui.label(egui::RichText::new("設定")
                                .size(17.0).strong().color(colors::TEXT_PRIMARY));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if kui::icon_button(ui, "×", "Close", false, 30.0).clicked() {
                                    keep_open = false;
                                }
                            });
                        });
                    });
                ui.add_space(10.0);

                let content_height = (ui.available_height() - 58.0).max(180.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), content_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.vertical(|ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                    // Reserve enough horizontal room for the longest Japanese
                                    // category label (メディアとキャッシュ) without clipping.
                                    let narrow_preferences = ui.available_width() < 700.0;
                                    let icon_only_preferences = ui.available_width() < 520.0;
                                    let nav_width = if icon_only_preferences {
                                        54.0
                                    } else if narrow_preferences {
                                        170.0
                                    } else {
                                        220.0
                                    };
                                    ui.set_width(nav_width);
                                    for &cat in PrefCategory::ALL {
                                        let selected = current_category == cat;
                                        let label = cat.nav_title();
                                        let row_size = egui::vec2(nav_width, 44.0);
                                        let (rect, response) = ui.allocate_exact_size(row_size, egui::Sense::click());
                                        if selected {
                                            ui.painter().rect_filled(
                                                rect,
                                                egui::Rounding::same(13.0),
                                                egui::Color32::from_rgb(29, 32, 38),
                                            );
                                            // Keep the active rail inside the rounded selection surface.
                                            let rail = egui::Rect::from_min_size(
                                                rect.left_top() + egui::vec2(0.0, 7.0),
                                                egui::vec2(4.0, rect.height() - 14.0),
                                            );
                                            ui.painter().rect_filled(rail, egui::Rounding::same(2.0), colors::ACCENT_BRAND);
                                        }

                                        let icon_color = if selected { "#ff6b1a" } else { "#aeb7bd" };
                                        // Match the reference's solid database glyph instead of tinting
                                        // an outlined placeholder; other categories use the existing SVG set.
                                        let database_icon = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><ellipse cx="12" cy="5" rx="8.5" ry="3" fill="#ff6b1a"/><path d="M3.5 5v14c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3V5" fill="#ff6b1a"/><path d="M3.5 12c0 1.7 3.8 3 8.5 3s8.5-1.3 8.5-3" fill="none" stroke="#111419" stroke-width="1.5"/></svg>"##;
                                        let icon_svg = if selected && cat == PrefCategory::Cache {
                                            database_icon.to_owned()
                                        } else {
                                            cat.icon()
                                                .replace("stroke=\"white\"", &format!("stroke=\"{icon_color}\""))
                                                .replace("fill=\"white\"", &format!("fill=\"{icon_color}\""))
                                        };
                                        let icon_rect = egui::Rect::from_center_size(
                                            egui::pos2(
                                                if icon_only_preferences {
                                                    rect.center().x
                                                } else if narrow_preferences {
                                                    rect.left() + 27.0
                                                } else {
                                                    rect.left() + 43.0
                                                },
                                                rect.center().y,
                                            ),
                                            egui::vec2(22.0, 22.0),
                                        );
                                        let image = egui::Image::from_bytes(
                                            format!("bytes://preferences-category-{}-{}.svg", egui::Id::new(cat.title()).value(), selected),
                                            icon_svg.into_bytes(),
                                        ).fit_to_exact_size(icon_rect.size());
                                        ui.put(icon_rect, image);
                                        let label_color = if selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY };
                                        if !icon_only_preferences {
                                            ui.painter().text(
                                                egui::pos2(
                                                    rect.left()
                                                        + if narrow_preferences { 50.0 } else { 67.0 },
                                                    rect.center().y,
                                                ),
                                                egui::Align2::LEFT_CENTER,
                                                label,
                                                egui::FontId::proportional(
                                                    if narrow_preferences { 12.5 } else { 14.0 },
                                                ),
                                                label_color,
                                            );
                                        }
                                        response.clone().on_hover_text(label);
                                        if response.clicked() {
                                            current_category = cat;
                                        }
                                    }
                                });
                            });

                        ui.add_space(10.0);
                        let (divider, _) = ui.allocate_exact_size(
                            egui::vec2(1.0, ui.available_height()),
                            egui::Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [divider.center_top(), divider.center_bottom()],
                            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                        );
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            let content_width = (ui.available_width() - 10.0).max(240.0);
                            ui.set_width(content_width);
                            let heading = crate::ui::localization::tr(current_category.title());
                            crate::ui::kui::title(ui, &heading);
                            ui.add_space(3.0);
                            kui::caption(ui, match current_category {
                                PrefCategory::General => "アプリケーションの基本動作と起動時の設定です。",
                                PrefCategory::Cache => "メディア保存先、復旧データ、描画リソースを管理します。",
                                PrefCategory::Performance => "プレビュー品質、メモリ使用量、取り消し履歴を調整します。",
                                PrefCategory::UiAppearance => "画面表示と操作インターフェースを設定します。",
                                PrefCategory::Projects => "プロジェクトと作業色空間を管理します。",
                                PrefCategory::License => "アプリケーションのバージョンとライセンス情報です。",
                                PrefCategory::KeyboardShortcuts => "キーボード操作を確認・変更します。",
                                PrefCategory::Plugins => "プラグインの対応状況を確認します。",
                            });
                            ui.add_space(2.0);
                            egui::ScrollArea::vertical()
                                .id_salt("preferences_category_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if current_category == PrefCategory::Cache {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                    }
                                    match current_category {
                                    PrefCategory::General => {
                                        let options = [
                                                (crate::ui::localization::LanguagePreference::System, "System"),
                                                (crate::ui::localization::LanguagePreference::English, "English"),
                                                (crate::ui::localization::LanguagePreference::Japanese, "日本語"),
                                            ];
                                        settings_card(ui, "", |ui| {
                                            preference_property_row(ui, "インターフェース言語", "メニューやダイアログに表示する言語を選択します。", |ui| {
                                                kui::combo(ui, "ui_language", &mut p.language, &options)
                                            });
                                            preference_property_row(ui, "テーマ", "アプリケーション全体の配色を設定します。", |ui| {
                                                let mut theme = String::from("ダーク");
                                                kui::combo(ui, "prefs_theme_selector", &mut theme, &[(String::from("ダーク"), "ダーク")]);
                                            });
                                            preference_property_row(ui, "起動時に開く画面", "起動後に最初に表示する画面を選択します。", |ui| {
                                                let mut destination = usize::from(!p.startup_home);
                                                if kui::combo(ui, "prefs_startup_destination", &mut destination, &[(0, "Home"), (1, "Studio")]) {
                                                    p.startup_home = destination == 0;
                                                }
                                            });
                                            let mut autosave_options = vec![
                                                (60_u64, "1 minute"),
                                                (180_u64, "3 minutes"),
                                                (300_u64, "5 minutes"),
                                                (600_u64, "10 minutes"),
                                                (900_u64, "15 minutes"),
                                                (1800_u64, "30 minutes"),
                                            ];
                                            if !autosave_options.iter().any(|(secs, _)| *secs == p.autosave_secs) {
                                                autosave_options.push((p.autosave_secs, "Custom"));
                                            }
                                            preference_property_row(ui, "自動保存間隔", "作業中のプロジェクトを自動保存する間隔です。", |ui| {
                                                kui::combo(ui, "prefs_general_autosave_interval", &mut p.autosave_secs, &autosave_options)
                                            });
                                            ui.add_space(8.0);
                                            preference_toggle_row(ui, "プロジェクト読込時に前回の状態を復元", "最後に開いていたコンポジションとパネルを復元します。", &mut p.restore_previous_state);
                                            preference_toggle_row(ui, "プロジェクトを閉じる前に確認", "未保存の変更がある場合に確認を表示します。", &mut p.confirm_close_project);
                                            preference_toggle_row(ui, "アプリケーション終了前に確認する", "終了時にバックグラウンド処理を確認します。", &mut p.confirm_exit);
                                        });
                                    }
                                    PrefCategory::Performance => {
                                        settings_card(ui, "Preview and Memory", |ui| {
                                            kui::property(ui, &crate::ui::localization::tr("Memory limit:"), |ui| {
                                                let mut memory_limit_gb = (p.cache_mb / 1024).clamp(1, 64);
                                                if kui::scalar_slider(ui, &mut memory_limit_gb, 1..=64, " GB", None, false).changed() {
                                                    p.cache_mb = memory_limit_gb * 1024;
                                                }
                                            });
                                            ui.add_space(8.0);
                                            kui::toggle(ui, &mut p.adaptive_preview,
                                                &crate::ui::localization::tr("Adaptive preview quality (auto-reduce while playing)"))
                                                .on_hover_text(crate::ui::localization::tr(
                                                    "When off, preview always renders at full resolution"));
                                            let mut gpu_compute = p.gpu_compute_enabled.unwrap_or_else(|| app.history.current().use_gpu_compute);
                                            if kui::toggle(ui, &mut gpu_compute, &crate::ui::localization::tr("Use GPU acceleration for supported effects")).changed() {
                                                p.gpu_compute_enabled = Some(gpu_compute);
                                            }
                                            kui::caption(ui, &crate::ui::localization::tr("GPU effects use the active adapter when available."));
                                            ui.add_space(12.0);
                                            ui.horizontal(|ui| {
                                                status_chip(ui, "Preview renderer", if app.gpu_rendered { "GPU" } else { "Software" });
                                                status_chip(ui, "Current RAM cache", &format!("{} / {} MB",
                                                    app.frame_cache.current_memory_bytes / 1024 / 1024,
                                                    p.cache_mb));
                                            });
                                        });
                                        ui.add_space(10.0);
                                        settings_card(ui, "Undo History", |ui| {
                                            kui::property(ui, &crate::ui::localization::tr("Undo steps:"), |ui| {
                                                kui::scalar_slider(ui, &mut p.undo_steps, 10..=500, "", None, true)
                                            });
                                            ui.add_space(4.0);
                                            kui::caption(ui, format!(
                                                "{} {} MB",
                                                crate::ui::localization::tr("Undo history storage:"),
                                                app.history.retained_bytes() / 1024 / 1024
                                            ));
                                        });
                                    }
                                    PrefCategory::Cache => {
                                        let compact_controls = ui.available_width() < 700.0;
                                        settings_card(ui, "Disk Cache", |ui| {
                                            let usage = crate::core::frame_cache::disk_cache::current_disk_usage();
                                            let cap = (p.disk_cache_gb as u64).saturating_mul(1024 * 1024 * 1024).max(1);
                                            ui.horizontal(|ui| {
                                                ui.add_sized([84.0, 30.0], egui::Label::new(crate::ui::localization::tr("Location")));
                                                let path = if p.disk_cache_directory.is_empty() {
                                                    crate::core::frame_cache::disk_cache::cache_directory().display().to_string()
                                                } else { p.disk_cache_directory.clone() };
                                                egui::Frame::none()
                                                    .fill(colors::BG_MID)
                                                    .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
                                                    .rounding(egui::Rounding::same(5.0))
                                                    .inner_margin(egui::Margin::symmetric(9.0, 5.0))
                                                    .show(ui, |ui| {
                                                        ui.set_width((ui.available_width() - 126.0).max(120.0));
                                                        ui.add(egui::Label::new(egui::RichText::new(path).size(12.0).color(colors::TEXT_SECONDARY)).truncate());
                                                    });
                                                if kui::button(ui, "参照...", ButtonKind::Default).clicked() {
                                                    let initial = if p.disk_cache_directory.is_empty() {
                                                        crate::core::frame_cache::disk_cache::cache_directory()
                                                    } else {
                                                        std::path::PathBuf::from(&p.disk_cache_directory)
                                                    };
                                                    if let Some(path) = rfd::FileDialog::new().set_directory(initial).pick_folder() {
                                                        p.disk_cache_directory = path.to_string_lossy().into_owned();
                                                    }
                                                }
                                            });
                                            ui.add_space(2.0);
                                            ui.horizontal(|ui| {
                                                ui.label("使用量");
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.label(format!("{} / {} GB", format_bytes(usage), p.disk_cache_gb));
                                                });
                                            });
                                            ui.add(egui::ProgressBar::new((usage as f32 / cap as f32).clamp(0.0, 1.0))
                                                .desired_height(7.0)
                                                .fill(colors::ACCENT_BRAND));
                                            ui.add_space(5.0);
                                            if compact_controls {
                                                ui.horizontal(|ui| {
                                                    ui.label("最大サイズ:");
                                                    ui.add_sized([76.0, 30.0], egui::DragValue::new(&mut p.disk_cache_gb).range(10..=500).speed(1.0));
                                                    let mut disk_cache_unit = String::from("GB");
                                                    ui.allocate_ui_with_layout(
                                                        egui::vec2(70.0, 30.0),
                                                        egui::Layout::left_to_right(egui::Align::Center),
                                                        |ui| kui::combo(ui, "prefs_disk_cache_unit", &mut disk_cache_unit, &[(String::from("GB"), "GB")]),
                                                    );
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if kui::button(ui, "消去...", ButtonKind::Default).clicked() {
                                                            crate::core::frame_cache::disk_cache::clear_all();
                                                            crate::core::frame_cache::bump_version();
                                                            app.toasts.info(crate::ui::localization::tr("Disk Cache emptied (0 bytes)"));
                                                        }
                                                    });
                                                });
                                            } else {
                                                ui.horizontal(|ui| {
                                                    ui.add_sized([175.0, 30.0], egui::Label::new("最大サイズ:"));
                                                    ui.add_sized([100.0, 30.0], egui::DragValue::new(&mut p.disk_cache_gb).range(10..=500).speed(1.0));
                                                    let mut disk_cache_unit = String::from("GB");
                                                    ui.allocate_ui_with_layout(
                                                        egui::vec2(100.0, 30.0),
                                                        egui::Layout::left_to_right(egui::Align::Center),
                                                        |ui| kui::combo(ui, "prefs_disk_cache_unit", &mut disk_cache_unit, &[(String::from("GB"), "GB")]),
                                                    );
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if kui::button(ui, "消去...", ButtonKind::Default).clicked() {
                                                            crate::core::frame_cache::disk_cache::clear_all();
                                                            crate::core::frame_cache::bump_version();
                                                            app.toasts.info(crate::ui::localization::tr("Disk Cache emptied (0 bytes)"));
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                        ui.add_space(4.0);
                                        settings_card(ui, "Recovery and Auto-save", |ui| {
                                            let mut interval_options = vec![
                                                (60_u64, "1 minute"),
                                                (180_u64, "3 minutes"),
                                                (300_u64, "5 minutes"),
                                                (600_u64, "10 minutes"),
                                                (900_u64, "15 minutes"),
                                                (1800_u64, "30 minutes"),
                                            ];
                                            if !interval_options.iter().any(|(secs, _)| *secs == p.autosave_secs) {
                                                interval_options.push((p.autosave_secs, "Custom"));
                                            }
                                            ui.horizontal(|ui| {
                                                ui.add_sized([if compact_controls { 125.0 } else { 175.0 }, 26.0], egui::Label::new("自動保存間隔"));
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(if compact_controls { 150.0 } else { 230.0 }, 26.0),
                                                    egui::Layout::left_to_right(egui::Align::Center),
                                                    |ui| { kui::combo(ui, "prefs_autosave_interval", &mut p.autosave_secs, &interval_options); },
                                                );
                                                if !compact_controls {
                                                    kui::caption(ui, "変更を指定間隔で保存します。");
                                                }
                                            });
                                            if compact_controls {
                                                kui::caption(ui, "変更を指定間隔で保存します。");
                                            }
                                            ui.horizontal(|ui| {
                                                ui.add_sized([if compact_controls { 125.0 } else { 175.0 }, 26.0], egui::Label::new("復旧スナップショット"));
                                                ui.add_sized([if compact_controls { 90.0 } else { 110.0 }, 28.0], egui::DragValue::new(&mut p.recovery_slots)
                                                    .range(1..=crate::core::autosave::MAX_RECOVERY_SLOTS).speed(1.0));
                                                ui.label("個");
                                                if !compact_controls {
                                                    kui::caption(ui, "再起動後も保持します。");
                                                }
                                            });
                                            if compact_controls {
                                                kui::caption(ui, "再起動後も保持します。");
                                            }
                                        });
                                        ui.add_space(4.0);
                                        settings_card(ui, "Performance", |ui| {
                                            kui::property(ui, "GPUデバイス:", |ui| {
                                                ui.label(if app.gpu_rendered { "使用可能なGPU" } else { "ソフトウェア描画" });
                                            });
                                            let mut gpu_compute = p.gpu_compute_enabled.unwrap_or_else(|| app.history.current().use_gpu_compute);
                                            ui.horizontal(|ui| {
                                                ui.label("ハードウェアアクセラレーション");
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if kui::switch(ui, &mut gpu_compute, "").changed() {
                                                        p.gpu_compute_enabled = Some(gpu_compute);
                                                    }
                                                });
                                            });
                                            let mut memory_limit_gb = (p.cache_mb / 1024).clamp(1, 64);
                                            let compact_memory = ui.available_width() < 480.0;
                                            if compact_memory {
                                                ui.label("メモリ上限:");
                                            }
                                            ui.horizontal(|ui| {
                                                if !compact_memory {
                                                    ui.add_sized([150.0, 28.0], egui::Label::new("メモリ上限:"));
                                                }
                                                let value_width = if compact_memory { 68.0 } else { 90.0 };
                                                let previous_slider_width = ui.spacing().slider_width;
                                                ui.spacing_mut().slider_width = if compact_memory {
                                                    (ui.available_width() - value_width - 35.0).max(72.0)
                                                } else {
                                                    320.0
                                                };
                                                let slider = ui.add(egui::Slider::new(&mut memory_limit_gb, 1..=64).show_value(false));
                                                ui.spacing_mut().slider_width = previous_slider_width;
                                                let value = ui.add_sized(
                                                    [value_width, 30.0],
                                                    egui::DragValue::new(&mut memory_limit_gb).range(1..=64).speed(1.0),
                                                );
                                                ui.label("GB");
                                                if slider.changed() || value.changed() {
                                                    p.cache_mb = memory_limit_gb * 1024;
                                                }
                                            });
                                        });
                                    }
                                    PrefCategory::UiAppearance => {
                                        settings_card(ui, "Interface", |ui| {
                                            ui.horizontal(|ui| {
                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 72.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 6.0, colors::BG_DEEPEST);
                                                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(30.0, 72.0)), 6.0, colors::BG_DARKEST);
                                                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(38.0, 12.0), egui::vec2(70.0, 7.0)), 2.0, colors::ACCENT_BRAND);
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("Kagari Dark").strong());
                                                    kui::caption(ui, "The current studio theme");
                                                    kui::caption(ui, "Warm orange selection · high-contrast panels");
                                                });
                                            });
                                        });
                                        settings_card(ui, "Audio Preview", |ui| {
                                            kui::toggle(ui, &mut p.audio_preview, "再生中に音声をプレビュー");
                                            kui::caption(ui, "Audio output follows the active system device.");
                                        });
                                    }
                                    PrefCategory::Projects => {
                                        settings_card(ui, "Custom Workspaces", |ui| {
                                            crate::ui::workspace_manager::draw_workspace_manager(app, ui);
                                        });
                                        let composition = app.history.current().active_composition();
                                        let color_space = format!("{:?}", composition.working_color_space);
                                        let bit_depth = format!("{:?}", composition.bit_depth);
                                        settings_card(ui, "Working Color Space", |ui| {
                                            kui::caption(ui, "Color space and bit depth are configured per composition.");
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                status_chip(ui, "Current composition", &color_space);
                                                status_chip(ui, "Bit depth", &bit_depth);
                                            });
                                            ui.add_space(10.0);
                                            if kui::button(ui, &crate::ui::localization::tr("Open Composition Settings"), ButtonKind::Default).clicked() {
                                                open_comp_settings = true;
                                                keep_open = false;
                                            }
                                        });
                                    }
                                    PrefCategory::KeyboardShortcuts => {
                                        settings_card(ui, "Keyboard Shortcuts", |ui| {
                                            kui::caption(ui, "Browse the commands and gestures available in the current workspace.");
                                            ui.add_space(10.0);
                                            if kui::button(ui, &crate::ui::localization::tr("Keyboard Shortcuts Reference..."), ButtonKind::Default).clicked() {
                                                app.show_shortcuts_dialog = true;
                                            }
                                        });
                                    }
                                    PrefCategory::Plugins => {
                                        settings_card(ui, "Plugin Support", |ui| {
                                            ui.horizontal(|ui| {
                                                status_chip(ui, "OpenFX", "Host supported");
                                                status_chip(ui, "AEGP", "Host bridge");
                                            });
                                            ui.add_space(8.0);
                                            kui::caption(ui, "/Library/OFX/Plugins and the standard After Effects plug-in folders are scanned at launch.");
                                        });
                                    }
                                    PrefCategory::License => {
                                        settings_card(ui, "Kagari VFX", |ui| {
                                            ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).strong());
                                            ui.add_space(6.0);
                                            kui::caption(ui, "Licensed under the MIT License or Apache License 2.0.");
                                            kui::caption(ui, "Third-party component licenses are listed in THIRD-PARTY-NOTICES.");
                                        });
                                    }
                                    }
                                });
                        });
                    },
                );

                ui.add_space(8.0);
                kui::separator(ui);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if kui::button(ui, &crate::ui::localization::tr("Apply"), ButtonKind::Accent).clicked() {
                            match save(&p) {
                                Ok(()) => {
                                    apply(app, &p);
                                    app.toasts.info(crate::ui::localization::tr("Preferences saved"));
                                    keep_open = false;
                                }
                                Err(error) => app.toasts.error(error),
                            }
                        }
                        if kui::button(ui, &crate::ui::localization::tr("Cancel"), ButtonKind::Default).clicked() {
                            keep_open = false;
                        }
                    });
                });
            });

            ctx.data_mut(|d| d.insert_temp(id, p));
            let cat_idx = PrefCategory::ALL.iter().position(|c| *c == current_category).unwrap_or(0);
            ctx.data_mut(|d| d.insert_temp(category_id, cat_idx));
        });

    app.show_preferences = open && keep_open;
    if !app.show_preferences {
        ctx.data_mut(|data| data.remove_temp::<Prefs>(id));
    }
    if open_comp_settings && !app.show_preferences {
        app.show_comp_settings = true;
    }
}

pub fn draw_exit_confirmation_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_exit_confirmation {
        return;
    }
    let mut open = true;
    let mut quit = false;
    crate::ui::modal::dialog("Quit Kagari VFX?")
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(420.0, 180.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Quit Kagari VFX?");
            ui.add_space(8.0);
            ui.label(
                "Background rendering and recovery saves will stop when the application closes.",
            );
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if kui::button(ui, "Cancel", ButtonKind::Default).clicked() {
                    app.show_exit_confirmation = false;
                }
                if kui::button(ui, "Quit", ButtonKind::Accent).clicked() {
                    quit = true;
                }
            });
        });
    if !open {
        app.show_exit_confirmation = false;
    }
    if quit {
        app.show_exit_confirmation = false;
        app.exit_confirmed = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

pub fn request_close_project(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_home && app.confirm_close_project {
        app.show_close_project_confirmation = true;
        return;
    }
    return_to_home(app, ctx);
}

pub fn draw_close_project_confirmation_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_close_project_confirmation {
        return;
    }
    let mut open = true;
    let mut close_project = false;
    crate::ui::modal::dialog(crate::ui::localization::tr("Close Project?"))
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(460.0, 200.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading(crate::ui::localization::tr("Close Project?"));
            ui.add_space(8.0);
            ui.label(crate::ui::localization::tr(
                "Return to Home and leave the current project?",
            ));
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if kui::button(
                    ui,
                    &crate::ui::localization::tr("Cancel"),
                    ButtonKind::Default,
                )
                .clicked()
                {
                    app.show_close_project_confirmation = false;
                }
                if kui::button(
                    ui,
                    &crate::ui::localization::tr("Close Project"),
                    ButtonKind::Accent,
                )
                .clicked()
                {
                    close_project = true;
                }
            });
        });
    if !open {
        app.show_close_project_confirmation = false;
    }
    if close_project {
        app.show_close_project_confirmation = false;
        return_to_home(app, ctx);
    }
}

fn return_to_home(app: &mut KagariApp, ctx: &egui::Context) {
    app.show_home = true;
    crate::ui::home_screen::set_home_nav(ctx, crate::ui::home_screen::HomeNav::Home);
}

fn settings_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let card_width = ui.available_width();
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(14, 21, 24))
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(35, 44, 50),
        ))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 2.0))
        .show(ui, |ui| {
            ui.set_width((card_width - 20.0).max(0.0));
            ui.vertical(|ui| {
                if !title.is_empty() {
                    crate::ui::kui::section(ui, &crate::ui::localization::tr(title));
                    ui.add_space(4.0);
                }
                add_contents(ui)
            })
            .inner
        })
        .inner
}

fn preference_toggle_row(ui: &mut egui::Ui, label: &str, description: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        let label_width = (ui.available_width() - 60.0).max(100.0);
        ui.allocate_ui_with_layout(
            egui::vec2(label_width, 36.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(label))
                        .size(14.0)
                        .color(colors::TEXT_PRIMARY),
                );
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(description))
                        .size(10.5)
                        .color(colors::TEXT_SECONDARY),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kui::switch(ui, value, "");
        });
    });
    ui.add_space(1.0);
    kui::separator(ui);
    ui.add_space(1.0);
}

fn preference_property_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    description: &str,
    control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let result = ui
        .horizontal(|ui| {
            let label_width = (ui.available_width() - 250.0).max(180.0);
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, 38.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(
                        egui::RichText::new(crate::ui::localization::tr(label))
                            .size(14.0)
                            .color(colors::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(crate::ui::localization::tr(description))
                            .size(10.5)
                            .color(colors::TEXT_SECONDARY),
                    );
                },
            );
            let field_width = 240.0_f32.min(ui.available_width());
            ui.add_space((ui.available_width() - field_width).max(0.0));
            ui.allocate_ui_with_layout(
                egui::vec2(field_width, 38.0),
                egui::Layout::left_to_right(egui::Align::Center),
                control,
            )
            .inner
        })
        .inner;
    ui.add_space(1.0);
    kui::separator(ui);
    ui.add_space(1.0);
    result
}

fn status_chip(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::none()
        .fill(colors::BG_MID)
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                kui::caption(ui, label);
                ui.label(
                    egui::RichText::new(value)
                        .strong()
                        .color(colors::TEXT_PRIMARY),
                );
            });
        });
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}
