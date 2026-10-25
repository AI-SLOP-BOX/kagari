use super::{colors, fonts, layout};
use eframe::egui;

/// Apply comprehensive AE dark theme to egui.
pub fn configure_ae_theme(ctx: &egui::Context) {
    let configured_id = egui::Id::new("kagari_ae_theme_configured");
    if ctx.data(|data| data.get_temp::<bool>(configured_id).unwrap_or(false)) {
        return;
    }

    ctx.set_theme(egui::Theme::Dark);
    fonts::configure_fonts(ctx);

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
    ctx.data_mut(|data| data.insert_temp(configured_id, true));
}
