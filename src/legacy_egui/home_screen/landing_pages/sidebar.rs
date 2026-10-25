use super::*;

pub(in crate::ui::home_screen) fn draw_landing_sidebar(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let scale = (ctx.screen_rect().width() / 1584.0).max(1.0);
    let visible = ui.max_rect();
    ui.painter()
        .rect_filled(visible, 0.0, egui::Color32::from_rgb(11, 20, 23));
    ui.painter().line_segment(
        [visible.right_top(), visible.right_bottom()],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let scroll_height = visible.height().max(680.0 * scale);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |scroll_ui| {
            scroll_ui.allocate_ui_with_layout(
                egui::vec2(scroll_ui.available_width(), scroll_height),
                egui::Layout::top_down(egui::Align::Min),
                |content_ui| draw_landing_sidebar_content(app, content_ui, ctx),
            );
        });
}

fn draw_landing_sidebar_content(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let scale = (ctx.screen_rect().width() / 1584.0).max(1.0);
    let current = match home_nav(ctx) {
        HomeNav::NewProject => HomeNav::Projects,
        nav => nav,
    };
    let side = ui.max_rect();
    let compact = side.width() < 150.0 * scale;
    ui.add_space(if compact { 16.0 * scale } else { 22.0 * scale });
    let (brand_rect, _) =
        ui.allocate_exact_size(egui::vec2(side.width(), 44.0 * scale), egui::Sense::hover());
    if app.home_banner.is_none() {
        if let Ok(img) = crate::ui::embedded_assets::open_image(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = load_logo_texture(ctx, img);
        }
    }
    let brand_painter = ui.painter().with_clip_rect(brand_rect);
    if let Some(texture) = app.home_banner.as_ref() {
        let logo_size = if compact { 26.0 * scale } else { 34.0 * scale };
        let logo_rect = egui::Rect::from_min_size(
            egui::pos2(
                if compact {
                    brand_rect.center().x - logo_size * 0.5
                } else {
                    side.left() + 27.0 * scale
                },
                brand_rect.center().y - logo_size * 0.5,
            ),
            egui::vec2(logo_size, logo_size),
        );
        brand_painter.image(
            texture.id(),
            logo_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    if !compact {
        let wordmark_y = brand_rect.center().y;
        brand_painter.text(
            egui::pos2(side.left() + 72.0 * scale, wordmark_y),
            egui::Align2::LEFT_CENTER,
            "Kagari VFX",
            egui::FontId::proportional(16.0 * scale),
            colors::TEXT_PRIMARY,
        );
    }
    ui.add_space(if compact { 16.0 * scale } else { 28.0 * scale });
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Home,
        crate::ui::icons::SVG_HOME,
        "ホーム",
        true,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Projects,
        crate::ui::icons::SVG_FOLDER,
        "プロジェクト",
        true,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Documentation,
        crate::ui::icons::SVG_DOCUMENT,
        "ドキュメント",
        false,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Settings,
        crate::ui::icons::SVG_SETTINGS,
        "設定",
        false,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Compositing,
        crate::ui::icons::SVG_POWER,
        "終了",
        false,
    );
    ui.painter().text(
        egui::pos2(side.left() + 38.0 * scale, side.bottom() - 27.0 * scale),
        egui::Align2::LEFT_CENTER,
        concat!("Kagari VFX\nv", env!("CARGO_PKG_VERSION")),
        egui::FontId::proportional(12.0 * scale),
        crate::ui::theme::colors::TEXT_SECONDARY,
    );
}
