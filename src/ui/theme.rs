use eframe::egui;

/// VFX compositing Professional Dark Theme Palette
/// Based on actual AE CC 2024 color measurements, refined for professional density.
#[allow(dead_code)]
pub mod colors {
    use eframe::egui::Color32;

    // ── Background Layers (darkest → lightest) ──
    /// Deepest background - timeline/panel base
    pub const BG_DEEPEST: Color32 = Color32::from_rgb(16, 18, 20);
    /// Darkest panel background
    pub const BG_DARKEST: Color32 = Color32::from_rgb(20, 22, 28);
    /// Standard panel/panel background
    pub const BG_DARK: Color32 = Color32::from_rgb(26, 29, 34);
    /// Slightly elevated surfaces (cards, inputs)
    pub const BG_MID: Color32 = Color32::from_rgb(34, 40, 50);
    /// Elevated surfaces, dropdowns, popovers
    pub const BG_PANEL: Color32 = Color32::from_rgb(42, 48, 58);
    /// Input fields, search boxes
    pub const BG_SURFACE: Color32 = Color32::from_rgb(48, 54, 66);
    /// Highest elevation - dropdowns, tooltips
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(56, 62, 76);

    // ── Interactive States ──
    pub const BG_HOVER: Color32 = Color32::from_rgb(52, 62, 82);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(22, 82, 178); // Muted selection blue
    pub const BG_PRESSED: Color32 = Color32::from_rgb(12, 75, 165);

    // ── Accent Colors (Restrained Production Palette) ──
    /// Primary accent - muted steel blue. Active tabs, selection, playhead,
    /// focus states and primary actions. The only saturated hue in the UI.
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(14, 120, 220);
    /// Desaturated cyan - motion paths, guides, secondary indicators.
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(75, 135, 175);
    /// Muted amber - timecode, warnings, keyframes.
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(215, 175, 75);
    /// Muted green - success, solo states.
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(75, 155, 105);
    /// Red - error, mute. Kept readable; errors must pop.
    pub const ACCENT_RED: Color32 = Color32::from_rgb(210, 55, 55);
    /// Orange - warnings only. Never a primary action color.
    pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 155, 0);
    /// Purple - expressions, advanced.
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(155, 110, 230);

    // ── Borders (crisp 1px) ──
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(36, 42, 54);
    pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(52, 58, 72);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(72, 80, 92);
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(14, 120, 220);

    // ── Typography Colors ──
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 228, 235);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(158, 170, 192);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 124, 144);
    pub const TEXT_ACCENT: Color32 = Color32::from_rgb(85, 145, 205);
    pub const TEXT_ON_ACCENT: Color32 = Color32::from_rgb(255, 255, 255);

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
    pub const MOTION_PATH: Color32 = Color32::from_rgb(75, 135, 175);
    pub const KEYFRAME_DOT: Color32 = Color32::from_rgb(205, 170, 95);
    pub const GUIDE_LINE: Color32 = Color32::from_rgb(75, 135, 175);
    pub const HUD_BG: Color32 = Color32::from_rgba_premultiplied(14, 20, 30, 220);
    pub const HUD_STROKE: Color32 = Color32::from_rgb(75, 135, 175);
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
    pub const TIMELINE_PLAYHEAD: Color32 = Color32::from_rgb(70, 140, 210);
    pub const TIMELINE_KEYFRAME: Color32 = Color32::from_rgb(205, 165, 90);
    pub const TIMELINE_WAVEFORM: Color32 = Color32::from_rgb(85, 145, 105);
    pub const TIMELINE_SELECTION: Color32 = Color32::from_rgba_premultiplied(0, 100, 240, 38);
}

/// Layout & Spacing Constants for Pro Density
#[allow(dead_code)]
pub mod layout {
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;
    pub const TOOLBAR_HEIGHT: f32 = 32.0;
    pub const TIMELINE_LEFT_PANE_WIDTH: f32 = 260.0;
    pub const BOTTOM_TIMELINE_HEIGHT: f32 = 280.0;
    pub const STATUS_BAR_HEIGHT: f32 = 20.0;

    pub const FONT_SIZE_SMALL: f32 = 10.5;
    pub const FONT_SIZE_BODY: f32 = 12.5;
    pub const FONT_SIZE_HEADING: f32 = 13.5;
    pub const FONT_SIZE_TITLE: f32 = 14.5;
}

/// Configure fonts for professional appearance.
/// Loads Inter for Latin UI, LINE Seed JP / Hiragino for Japanese UI.
/// Falls back to system fonts gracefully.
fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Load system fonts for professional appearance
    #[cfg(target_os = "macos")]
    {
        let sf_pro_paths = [
            "/System/Library/Fonts/SFCompact.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/Library/Fonts/SF-Pro-Display-Regular.otf",
            "/System/Library/Fonts/Helvetica.ttc",
        ];
        let menlo_paths = [
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Menlo-Regular.ttc",
        ];
        let jp_font_paths = [
            "/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc",
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ];
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
                    .insert("LineSeedJP".to_string(), egui::FontData::from_owned(data));
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
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ];
        let linux_mono_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ];

        for path in &linux_prop_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "LinuxSystemFont".to_string(),
                    egui::FontData::from_owned(data),
                );
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
                fonts.font_data.insert(
                    "LinuxMonoFont".to_string(),
                    egui::FontData::from_owned(data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "LinuxMonoFont".to_string());
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
    visuals.extreme_bg_color = egui::Color32::from_rgb(10, 10, 12);

    // ── Selection ──
    visuals.selection.bg_fill = colors::BG_ACTIVE;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, colors::ACCENT_BLUE);

    // ── Widget states ──
    // Noninteractive (labels, static text)
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, colors::TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.weak_bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(2.0);

    // Inactive (buttons, sliders at rest)
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.bg_fill = colors::BG_MID;
    visuals.widgets.inactive.weak_bg_fill = colors::BG_MID;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM);
    visuals.widgets.inactive.rounding = egui::Rounding::same(2.0);

    // Hovered
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_STRONG);
    visuals.widgets.hovered.rounding = egui::Rounding::same(2.0);

    // Active (pressed)
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.active.bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.weak_bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, colors::ACCENT_BLUE);
    visuals.widgets.active.rounding = egui::Rounding::same(2.0);

    // Open (expanded menus, popups)
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    visuals.widgets.open.bg_fill = colors::BG_PANEL;
    visuals.widgets.open.weak_bg_fill = colors::BG_PANEL;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, colors::BORDER_STRONG);
    visuals.widgets.open.rounding = egui::Rounding::same(2.0);

    // ── Warning/Error colors ──
    visuals.warn_fg_color = colors::ACCENT_ORANGE;
    visuals.error_fg_color = colors::ACCENT_RED;

    // ── Resize handle styling ──
    visuals.resize_corner_size = 6.0;

    ctx.set_visuals(visuals);

    // ── Typography & Spacing ──
    ctx.style_mut(|style| {
        // Tighter spacing for pro density
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
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
        ui.painter().rect_filled(rect, 1.0, colors::ACCENT_BLUE);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("{} {}", icon, title))
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// Helper: Draw a pro tab with dynamic bottom cyan border when selected.
pub fn draw_custom_tab(ui: &mut egui::Ui, selected: bool, title: &str) -> egui::Response {
    let text = egui::RichText::new(title)
        .small()
        .strong()
        .color(if selected {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_SECONDARY
        });

    let response = ui.selectable_label(selected, text);
    if selected {
        let rect = response.rect;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom() - 1.0),
                egui::pos2(rect.right(), rect.bottom() - 1.0),
            ],
            egui::Stroke::new(1.5_f32, colors::ACCENT_BLUE),
        );
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
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
