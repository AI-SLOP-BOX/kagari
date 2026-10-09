use eframe::egui;

/// Neutral editing-tool palette. Accent colors indicate interaction state; layer colors identify content.
#[allow(dead_code)]
pub mod colors {
    use eframe::egui::Color32;

    // ── Background Layers (darkest → lightest) ──
    /// Deep application background (#121418 - ultra-dense studio base).
    pub const BG_DEEPEST: Color32 = Color32::from_rgb(18, 20, 24);
    /// Standard panel surface (#1A1D24 - sleek modern panel).
    pub const BG_DARKEST: Color32 = Color32::from_rgb(26, 29, 36);
    pub const BG_DARK: Color32 = Color32::from_rgb(26, 29, 36);
    /// Raised surface for rows and controls (#222731).
    pub const BG_MID: Color32 = Color32::from_rgb(34, 39, 49);
    pub const BG_PANEL: Color32 = Color32::from_rgb(34, 39, 49);
    /// Deep input surface (#161820).
    pub const BG_SURFACE: Color32 = Color32::from_rgb(22, 24, 32);
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(40, 46, 58);
    pub const BG_PANEL_ALT: Color32 = Color32::from_rgb(26, 29, 36);
    pub const BG_PANEL_DEEP: Color32 = Color32::from_rgb(18, 20, 24);
    pub const BG_PANEL_DARK: Color32 = Color32::from_rgb(26, 29, 36);
    pub const BG_PANEL_BASE: Color32 = Color32::from_rgb(26, 29, 36);
    pub const BG_PANEL_RAISED: Color32 = Color32::from_rgb(34, 39, 49);
    pub const BG_EXTREME: Color32 = Color32::from_rgb(20, 22, 28);

    // ── Interactive States ──
    pub const BG_HOVER: Color32 = Color32::from_rgb(31, 36, 40);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(52, 34, 27);
    pub const BG_PRESSED: Color32 = Color32::from_rgb(66, 40, 29);
    pub const BG_SELECTION: Color32 = Color32::from_rgb(52, 34, 27);
    pub const BG_SELECTION_HOVER: Color32 = Color32::from_rgb(66, 44, 35);

    // ── Accent Colors (Restrained Production Palette) ──
    /// Saturated blue retained for domain graphics that specifically encode blue.
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(14, 120, 220);
    /// Compatibility token for interface accents, aligned with Kagari's warm brand color.
    pub const ACCENT_CYAN: Color32 = ACCENT_BRAND;
    /// Muted amber - timecode, warnings, keyframes.
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(213, 168, 90);
    /// Muted green - success, solo states.
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(105, 185, 157);
    /// Red - error, mute. Kept readable; errors must pop.
    pub const ACCENT_RED: Color32 = Color32::from_rgb(214, 107, 107);
    /// Warning amber. Reserve this token for warning states.
    pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 155, 0);
    pub const ACCENT_BRAND: Color32 = Color32::from_rgb(255, 107, 22);
    pub const ACCENT_BRAND_HOVER: Color32 = Color32::from_rgb(255, 145, 50);
    pub const ACCENT_SELECTION: Color32 = ACCENT_BRAND;
    pub const ACCENT_TOGGLE: Color32 = ACCENT_SELECTION;
    pub const ACCENT_WORKSPACE: Color32 = ACCENT_SELECTION;
    pub const ACCENT_PRIMARY_ACTION: Color32 = ACCENT_SELECTION;
    pub const ACCENT_WARNING: Color32 = Color32::from_rgb(213, 168, 90);
    pub const ACCENT_PROGRESS: Color32 = ACCENT_SELECTION;
    pub const ACCENT_INFORMATION: Color32 = ACCENT_SELECTION;
    pub const ACCENT_BEAT_MARKER: Color32 = ACCENT_YELLOW;
    /// Purple - expressions, advanced.
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(155, 110, 230);
    pub const ACCENT_BLUE_LIGHT: Color32 = Color32::from_rgb(90, 160, 255);
    pub const ACCENT_ACTION: Color32 = ACCENT_PRIMARY_ACTION;
    pub const ACCENT_ACTION_HOVER: Color32 = ACCENT_BRAND_HOVER;
    pub const ACCENT_ACTION_STROKE: Color32 = ACCENT_SELECTION;
    pub const METER_BAD: Color32 = Color32::from_rgb(255, 82, 78);
    pub const METER_WARNING: Color32 = Color32::from_rgb(255, 190, 45);
    pub const METER_GOOD: Color32 = Color32::from_rgb(42, 211, 86);
    pub const OVERLAY_SURFACE: Color32 = Color32::from_rgba_premultiplied(18, 22, 30, 225);
    /// Dark glassmorphism background tint with subtle translucency.
    pub const GLASS_SURFACE: Color32 = Color32::from_rgba_premultiplied(22, 25, 33, 220);
    pub const GLASS_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 28);

    // ── Borders (crisp 1px with modern cool-slate tone) ──
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 43, 54);
    pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(48, 55, 69);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(64, 73, 92);
    pub const BORDER_ACTIVE: Color32 = ACCENT_SELECTION;
    pub const BORDER_FOCUS: Color32 = BORDER_ACTIVE;

    // ── Typography Colors ──
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 228, 230);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(158, 166, 172);
    pub const TEXT_SECONDARY_BRIGHT: Color32 = Color32::from_rgb(193, 199, 203);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 120, 125);
    pub const TEXT_ACCENT: Color32 = ACCENT_SELECTION;
    pub const TEXT_ON_ACCENT: Color32 = Color32::from_rgb(255, 255, 255);

    pub const TUTORIAL_HINT_BACKGROUND: Color32 = Color32::from_rgb(25, 29, 33);
    pub const TUTORIAL_COMPLETE_BACKGROUND: Color32 = Color32::from_rgb(32, 58, 56);

    // ── Layer Label Colors (AE standard) ──
    pub const LABEL_RED: Color32 = Color32::from_rgb(255, 60, 60);
    pub const LABEL_ORANGE: Color32 = Color32::from_rgb(255, 160, 40);
    pub const LABEL_YELLOW: Color32 = Color32::from_rgb(255, 230, 50);
    pub const LABEL_GREEN: Color32 = Color32::from_rgb(80, 220, 100);
    pub const LABEL_CYAN: Color32 = Color32::from_rgb(50, 210, 255);
    pub const LABEL_BLUE: Color32 = Color32::from_rgb(50, 140, 255);
    pub const LABEL_PURPLE: Color32 = Color32::from_rgb(160, 110, 255);
    pub const LABEL_MAGENTA: Color32 = Color32::from_rgb(230, 80, 200);

    // ── Viewport Overlay Colors (desaturated; overlays must not glow) ──
    pub const GRID_LINE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 28);
    pub const MOTION_PATH: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
    pub const KEYFRAME_DOT: Color32 = Color32::from_rgb(205, 170, 95);
    pub const GUIDE_LINE: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
    pub const HUD_BG: Color32 = Color32::from_rgba_premultiplied(14, 20, 30, 220);
    pub const HUD_STROKE: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
    pub const HUD_TEXT: Color32 = Color32::from_rgb(175, 195, 215);
    pub const HUD_STATUS_TEXT: Color32 = Color32::from_rgb(165, 185, 205);
    pub const FPS_GOOD: Color32 = Color32::from_rgb(75, 155, 115);
    pub const FPS_BAD: Color32 = Color32::from_rgb(220, 110, 95);

    // ── 3D Gizmo Colors ──
    pub const GIZMO_X: Color32 = Color32::from_rgb(235, 70, 70);
    pub const GIZMO_Y: Color32 = Color32::from_rgb(60, 210, 80);
    pub const GIZMO_Z: Color32 = Color32::from_rgb(60, 140, 245);
    pub const BBOX_STROKE: Color32 = Color32::from_rgb(70, 135, 190);
    pub const HANDLE_NORMAL: Color32 = Color32::WHITE;
    pub const HANDLE_HOVER_FILL: Color32 = Color32::from_rgb(215, 185, 110);
    pub const HANDLE_HOVER_STROKE: Color32 = Color32::from_rgb(225, 110, 60);
    pub const CENTER_DOT: Color32 = Color32::from_rgb(210, 175, 85);
    pub const CENTER_HOVER_RING: Color32 = Color32::from_rgb(70, 135, 190);

    // ── Timeline Overlay Colors ──
    pub const TIMELINE_PLAYHEAD: Color32 = ACCENT_BRAND;
    pub const TIMELINE_KEYFRAME: Color32 = Color32::from_rgb(205, 165, 90);
    pub const TIMELINE_CACHE: Color32 = Color32::from_rgb(176, 105, 66);
    pub const TIMELINE_WAVEFORM: Color32 = Color32::from_rgb(176, 105, 66);
    pub const TIMELINE_SELECTION: Color32 = Color32::from_rgba_premultiplied(38, 16, 3, 38);
}

/// Layout & Spacing Constants for Pro Density
#[allow(dead_code)]
pub mod layout {
    pub const HIT_TARGET_MIN: f32 = 24.0;
    pub const HIT_TARGET_COMFORT: f32 = 28.0;
    pub const FIELD_HEIGHT: f32 = 26.0;
    pub const FIELD_RADIUS: f32 = 6.0;
    pub const BUTTON_RADIUS: f32 = 6.0;
    pub const SURFACE_RADIUS: f32 = 8.0;
    pub const PROPERTY_LABEL_WIDTH: f32 = 136.0;
    pub const PROPERTY_CONTROL_MIN_WIDTH: f32 = 130.0;
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;
    pub const TOOLBAR_HEIGHT: f32 = 32.0;
    pub const PANEL_HEADER_HEIGHT: f32 = 32.0;
    pub const DOCKED_PANELS_MIN_WIDTH: f32 = 840.0;
    pub const TIMELINE_LEFT_PANE_WIDTH: f32 = 190.0;
    pub const BOTTOM_TIMELINE_HEIGHT: f32 = 280.0;
    pub const STATUS_BAR_HEIGHT: f32 = 20.0;

    pub const FONT_SIZE_SMALL: f32 = 10.5;
    pub const FONT_SIZE_BODY: f32 = 12.5;
    pub const FONT_SIZE_HEADING: f32 = 13.5;
    pub const FONT_SIZE_TITLE: f32 = 14.5;
}

/// Preserve the bundled font metrics so mixed Japanese and Latin runs share one baseline.
fn japanese_ui_font(data: Vec<u8>) -> egui::FontData {
    egui::FontData::from_owned(data)
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Load system fonts for professional appearance
    #[cfg(target_os = "macos")]
    {
        let sf_pro_paths = [
            "/System/Library/Fonts/SFNS.ttf",
            "/Library/Fonts/SF-Pro-Display-Regular.otf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFCompact.ttf",
        ];
        let menlo_paths = [
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Menlo-Regular.ttc",
        ];
        let mut jp_font_paths = [
            "/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc",
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/LINESeedJP_OTF_Rg.otf",
            "/Library/Fonts/LINESeedJP_OTF_Rg.ttf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        if let Some(home) = std::env::var_os("HOME") {
            let user_fonts = std::path::PathBuf::from(home).join("Library/Fonts");
            jp_font_paths.extend(
                [
                    "LINESeedJP_OTF_Rg.otf",
                    "LINESeedJP_OTF_Rg.ttf",
                    "NotoSansCJKjp-Regular.otf",
                    "NotoSansJP-Regular.ttf",
                ]
                .map(|name| user_fonts.join(name)),
            );
        }
        let inter_paths = [
            "/Library/Fonts/Inter.ttf",
            "/System/Library/Fonts/Supplemental/Inter.ttf",
            "/Library/Fonts/Inter-Regular.ttf",
        ];

        // Try to load Inter for Latin UI (primary)
        let mut loaded_inter = false;
        for path in &inter_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Inter".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "Inter".to_string());
                loaded_inter = true;
                break;
            }
        }
        // Fallback to SF Pro if Inter not available
        if !loaded_inter {
            for path in &sf_pro_paths {
                if let Ok(data) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("SFPro".to_string(), egui::FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "SFPro".to_string());
                    loaded_inter = true;
                    break;
                }
            }
        }
        if !loaded_inter {
            log::info!("Using egui default proportional font (system fonts not found)");
        }

        // Load monospace font (Menlo)
        let mut loaded_mono = false;
        for path in &menlo_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Menlo".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Menlo".to_string());
                loaded_mono = true;
                break;
            }
        }
        if !loaded_mono {
            log::info!("Using egui default monospace font (Menlo not found)");
        }

        // Load Japanese font (Hiragino / LINE Seed JP fallback)
        for path in &jp_font_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LineSeedJP".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("LineSeedJP".to_string());
                break;
            }
        }
    }

    // Load Linux system fonts (DejaVu Sans, Ubuntu, Noto Sans, Liberation)
    #[cfg(target_os = "linux")]
    {
        let linux_prop_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];
        let mut linux_jp_paths = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
            "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.ttf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        let jp_font_names = [
            "NotoSansCJK-Regular.ttc",
            "NotoSansCJKjp-Regular.otf",
            "NotoSansJP-Regular.ttf",
            "LINESeedJP_OTF_Rg.otf",
        ];
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            let user_fonts = std::path::PathBuf::from(data_home).join("fonts");
            linux_jp_paths.extend(jp_font_names.map(|name| user_fonts.join(name)));
        }
        if let Some(home) = std::env::var_os("HOME") {
            let home = std::path::PathBuf::from(home);
            for font_dir in [home.join(".local/share/fonts"), home.join(".fonts")] {
                linux_jp_paths.extend(jp_font_names.map(|name| font_dir.join(name)));
            }
        }
        let linux_mono_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ];

        for path in &linux_prop_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LinuxSystemFont".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "LinuxSystemFont".to_string());
                break;
            }
        }
        for path in &linux_mono_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LinuxMonoFont".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "LinuxMonoFont".to_string());
                break;
            }
        }
        for path in &linux_jp_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "LinuxJapaneseFont".to_string(),
                    egui::FontData::from_owned(data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("LinuxJapaneseFont".to_string());
                break;
            }
        }
    }

    // Load Windows system fonts (Segoe UI, Consolas)
    #[cfg(target_os = "windows")]
    {
        let win_prop_paths = [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        let win_mono_paths = ["C:\\Windows\\Fonts\\consola.ttf"];
        let mut win_jp_paths = [
            "C:\\Windows\\Fonts\\YuGothR.ttc",
            "C:\\Windows\\Fonts\\YuGothM.ttc",
            "C:\\Windows\\Fonts\\msgothic.ttc",
            "C:\\Windows\\Fonts\\NotoSansCJKjp-Regular.otf",
            "C:\\Windows\\Fonts\\NotoSansJP-Regular.otf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        if let Some(user_font_dir) = std::env::var_os("LOCALAPPDATA") {
            let user_font_dir = std::path::PathBuf::from(user_font_dir)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts");
            win_jp_paths.extend([
                user_font_dir.join("NotoSansCJKjp-Regular.otf"),
                user_font_dir.join("NotoSansJP-Regular.otf"),
            ]);
        }

        for path in &win_prop_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("SegoeUI".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "SegoeUI".to_string());
                break;
            }
        }
        for path in &win_mono_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Consolas".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Consolas".to_string());
                break;
            }
        }
        for path in &win_jp_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "WindowsJapaneseFont".to_string(),
                    egui::FontData::from_owned(data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("WindowsJapaneseFont".to_string());
                break;
            }
        }
    }

    // Keep the product UI visually consistent even when the host OS does not
    // provide Inter. The bundled variable font contains the requested UI weights.
    if let Some(data) = crate::ui::embedded_assets::bytes("assets/fonts/Inter.ttf") {
        fonts.font_data.insert(
            "KagariInter".to_string(),
            egui::FontData::from_owned(data.to_vec()),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "KagariInter".to_string());
    }

    if let Some(data) = crate::ui::embedded_assets::bytes("assets/fonts/NotoSansJP-Regular.otf") {
        fonts.font_data.insert(
            "KagariJapanese".to_string(),
            japanese_ui_font(data.to_vec()),
        );
        let proportional = fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default();
        proportional.insert(0, "KagariJapanese".to_string());
    }

    // Phosphor icon glyphs (used across panels for crisp vector icons)
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    ctx.set_fonts(fonts);

    ctx.style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(layout::FONT_SIZE_SMALL, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(layout::FONT_SIZE_HEADING, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Monospace),
        );
    });
}

/// Apply comprehensive AE dark theme to egui.
pub fn configure_ae_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    configure_fonts(ctx);

    let mut visuals = egui::Visuals::dark();

    // ── Background fills ──
    visuals.panel_fill = colors::BG_DARKEST;
    visuals.window_fill = colors::BG_DARK;
    visuals.faint_bg_color = colors::BG_DEEPEST;
    visuals.extreme_bg_color = colors::BG_EXTREME;

    // ── Selection (calm dark brand; bright pills read as toy UI) ──
    visuals.selection.bg_fill = colors::BG_SELECTION;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION);

    // ── Widget states ──
    // Noninteractive (labels, static text)
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, colors::TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.weak_bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(5.0);

    // Inactive (buttons, sliders at rest)
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.bg_fill = colors::BG_MID;
    visuals.widgets.inactive.weak_bg_fill = colors::BG_MID;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM);
    visuals.widgets.inactive.rounding = egui::Rounding::same(5.0);

    // Hovered
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_STRONG);
    visuals.widgets.hovered.rounding = egui::Rounding::same(5.0);

    // Active (pressed)
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.active.bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.weak_bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_FOCUS);
    visuals.widgets.active.rounding = egui::Rounding::same(5.0);

    // Open (expanded menus, popups)
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.open.bg_fill = colors::BG_PANEL;
    visuals.widgets.open.weak_bg_fill = colors::BG_PANEL;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_STRONG);
    visuals.widgets.open.rounding = egui::Rounding::same(5.0);

    // ── Window & Popup Shadows (smooth drop shadow for dark glass feel) ──
    visuals.window_rounding = egui::Rounding::same(8.0);
    visuals.window_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(160),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(140),
    };
    visuals.menu_rounding = egui::Rounding::same(6.0);

    // ── Warning/Error colors ──
    visuals.warn_fg_color = colors::ACCENT_WARNING;
    visuals.error_fg_color = colors::ACCENT_RED;

    // ── Resize handle styling ──
    visuals.resize_corner_size = 6.0;

    ctx.set_visuals(visuals);

    // ── Typography & Spacing ──
    ctx.style_mut(|style| {
        // Tighter spacing for pro density
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.interact_size = egui::vec2(layout::HIT_TARGET_MIN, layout::HIT_TARGET_MIN);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.indent = 12.0;
        style.spacing.scroll.bar_width = 5.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.bar_outer_margin = 1.0;
        style.spacing.menu_margin = egui::Margin::symmetric(6.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(6.0);

        // Slightly smaller default text style
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(layout::FONT_SIZE_SMALL, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(layout::FONT_SIZE_HEADING, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Monospace),
        );
    });
}

/// Helper: Render section headers with a crisp left accent bar, icon, and high-contrast typography.
#[allow(dead_code)]
pub fn draw_section_header(ui: &mut egui::Ui, title: &str, icon: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 1.0, colors::ACCENT_WORKSPACE);
        ui.add_space(4.0);
        if let Some(svg) = crate::ui::icons::svg_for_icon_label(icon) {
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            crate::ui::icons::render_svg_at(
                ui,
                format!("section-icon-{title}"),
                svg,
                icon_rect.size(),
                colors::TEXT_SECONDARY,
                icon_rect.min,
            );
            ui.add_space(3.0);
        } else if !icon.is_empty() {
            ui.label(
                egui::RichText::new(icon)
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(3.0);
        }
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

pub fn draw_section_header_svg(ui: &mut egui::Ui, title: &str, icon: &'static str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 1.0, colors::ACCENT_WORKSPACE);
        ui.add_space(4.0);
        crate::ui::icons::render_svg_bytes(
            ui,
            title,
            icon,
            egui::vec2(14.0, 14.0),
            colors::TEXT_PRIMARY,
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// Helper: Draw a calm pro tab: plain text with a thin blue underline when
/// selected. Deliberately no filled background (avoids heavy pill tabs).
pub fn draw_custom_tab(ui: &mut egui::Ui, selected: bool, title: &str) -> egui::Response {
    let text = egui::RichText::new(title).small().color(if selected {
        colors::TEXT_PRIMARY
    } else {
        colors::TEXT_SECONDARY
    });

    let response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
    if selected {
        let rect = response.rect;
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0),
            egui::pos2(rect.right() - 2.0, rect.bottom()),
        );
        ui.painter().rect_filled(line_rect, 1.0, colors::ACCENT_SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(
            response.rect.expand2(egui::vec2(4.0, 2.0)),
            4.0,
            colors::BG_HOVER.linear_multiply(0.6),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Helper: Draw formatted property label, value, and unit (`px`, `%`, `dB`, `f`).
#[allow(dead_code)]
pub fn draw_prop_value(ui: &mut egui::Ui, label: &str, val_str: &str, unit: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !unit.is_empty() {
                ui.label(egui::RichText::new(unit).small().color(colors::TEXT_MUTED));
            }
            ui.label(
                egui::RichText::new(val_str)
                    .small()
                    .strong()
                    .color(colors::TEXT_ACCENT),
            );
        });
    });
}

/// Helper: Draw a crisp 1px horizontal separator line.
#[allow(dead_code)]
pub fn draw_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.add_space(1.0);
}

/// Helper: Draw a layer label color chip.
#[allow(dead_code)]
pub fn draw_label_chip(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
    let (hit_rect, response) = ui.allocate_exact_size(
        egui::vec2(layout::HIT_TARGET_MIN, layout::HIT_TARGET_MIN),
        egui::Sense::click(),
    );
    let rect = egui::Rect::from_center_size(hit_rect.center(), egui::vec2(12.0, 12.0));
    ui.painter().rect_filled(rect, 2.0, color);
    response
}

/// Helper: Create a consistent AE-style panel frame.
#[allow(dead_code)]
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
}

/// Helper: Create a consistent AE-style side panel frame.
#[allow(dead_code)]
pub fn side_panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARKEST)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
}

/// Custom DragValue with AE-style modifier keys:
/// - Normal drag: 1x speed
/// - Alt+drag: 0.1x speed (fine control)
/// - Shift+drag: 10x speed (fast scrub)
#[allow(dead_code)]
pub fn ae_drag_value(value: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(value).speed(1.0)
}
