use crate::core::editor::EditorSession;
use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

const ORANGE: egui::Color32 = crate::ui::theme::colors::ACCENT_BRAND;
const BG: egui::Color32 = egui::Color32::from_rgb(8, 16, 22);
const PANEL: egui::Color32 = crate::ui::theme::colors::BG_PANEL_BASE;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 1350.0;
    let mobile = width < 760.0;
    egui::TopBottomPanel::top("effects_workspace_bar")
        .exact_height(if mobile { 52.0 } else { 60.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(14, 23, 30)))
        .show(ctx, |ui| draw_topbar(ui, width, mobile, app, ctx));
    egui::TopBottomPanel::bottom("effects_workspace_footer")
        .exact_height(if mobile { 34.0 } else { 50.0 })
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_DEEPEST))
        .show(ctx, |ui| draw_footer(ui, width, mobile));
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            // At compact widths the category rail collapses so the effect grid keeps a usable width.
            let left_w = if mobile || compact { 0.0 } else { 293.0 };
            let right_w = if mobile || compact { 0.0 } else { 430.0 };
            if left_w > 0.0 {
                draw_sidebar(ui, rect, left_w, compact, app);
            }
            let center = egui::Rect::from_min_max(
                egui::pos2(rect.left() + left_w, rect.top()),
                egui::pos2(rect.right() - right_w, rect.bottom()),
            );
            draw_center(ui, ctx, center, compact, mobile, app);
            if right_w > 0.0 {
                draw_detail(
                    ui,
                    ctx,
                    egui::Rect::from_min_max(egui::pos2(center.right(), rect.top()), rect.max),
                    compact,
                    app,
                );
            }
        });
}

fn draw_topbar(ui: &mut egui::Ui, width: f32, mobile: bool, app: &mut KagariApp, ctx: &egui::Context) {
    let p = ui.painter().clone();
    for (x, c) in [
        (23.0, (255, 82, 78)),
        (46.0, (255, 190, 45)),
        (69.0, (42, 211, 86)),
    ] {
        p.circle_filled(
            egui::pos2(x, 23.0),
            6.5,
            egui::Color32::from_rgb(c.0, c.1, c.2),
        );
    }
    p.text(
        egui::pos2(96.0, 23.0),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    if !mobile && width >= 1100.0 {
        p.line_segment(
            [egui::pos2(270.0, 12.0), egui::pos2(270.0, 47.0)],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(290.0, 23.0),
            egui::Align2::LEFT_CENTER,
            "映像に、まだ見ぬ世界を。",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        let right = width - 42.0;
        let settings =
            egui::Rect::from_min_max(egui::pos2(right - 72.0, 5.0), egui::pos2(right, 55.0));
        let new_project = egui::Rect::from_min_max(
            egui::pos2(settings.left() - 152.0, 5.0),
            egui::pos2(settings.left() - 8.0, 55.0),
        );
        let open_project = egui::Rect::from_min_max(
            egui::pos2(new_project.left() - 182.0, 5.0),
            egui::pos2(new_project.left() - 8.0, 55.0),
        );
        effects_header_action(
            ui,
            open_project,
            icons::SVG_FOLDER,
            "プロジェクトを開く",
            "effects-open-project",
            app,
            ctx,
        );
        effects_header_action(
            ui,
            new_project,
            icons::SVG_FILE_PLUS,
            "新規プロジェクト",
            "effects-new-project",
            app,
            ctx,
        );
        effects_header_action(
            ui,
            settings,
            icons::SVG_SETTINGS,
            "設定",
            "effects-settings",
            app,
            ctx,
        );
        let close = egui::Rect::from_center_size(egui::pos2(width - 18.0, 23.0), egui::vec2(28.0, 32.0));
        if ui.interact(close, egui::Id::new("effects-window-close"), egui::Sense::click()).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        p.text(close.center(), egui::Align2::CENTER_CENTER, "×", egui::FontId::proportional(18.0), colors::TEXT_MUTED);
    }
}

fn effects_header_action(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    icon: &'static str,
    label: &str,
    id: &str,
    app: &mut KagariApp,
    ctx: &egui::Context,
) {
    let response = ui.interact(rect, egui::Id::new(id), egui::Sense::click());
    if response.clicked() {
        match id {
            "effects-open-project" => crate::ui::home_screen::open_project_dialog(app),
            "effects-new-project" => crate::ui::home_screen::new_project_from_header(app),
            "effects-settings" => app.show_preferences = true,
            _ => {}
        }
        ctx.request_repaint();
    }
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.0,
            egui::Color32::from_rgba_unmultiplied(48, 61, 72, 110),
        );
    }
    icons::render_svg_at(
        ui,
        id.to_string(),
        icon,
        egui::vec2(20.0, 20.0),
        colors::TEXT_SECONDARY,
        egui::pos2(rect.left() + 8.0, rect.center().y - 10.0),
    );
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 36.0, rect.top()),
        egui::pos2(rect.right() - 4.0, rect.bottom()),
    );
    ui.put(
        text_rect,
        egui::Label::new(
            egui::RichText::new(label)
                .size(13.0)
                .color(colors::TEXT_SECONDARY),
        )
        .truncate(),
    );
}

fn draw_sidebar(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    width: f32,
    compact: bool,
    _app: &mut KagariApp,
) {
    let side = egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + width, rect.bottom()));
    {
        let p = ui.painter();
        p.rect_filled(side, 0.0, egui::Color32::from_rgb(13, 23, 30));
        p.line_segment(
            [
                egui::pos2(side.right(), side.top()),
                egui::pos2(side.right(), side.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
    }
    icons::render_svg_at(
        ui,
        "effects-heading".to_string(),
        icons::SVG_EFFECTS,
        egui::vec2(30.0, 30.0),
        colors::TEXT_PRIMARY,
        egui::pos2(side.left() + 20.0, side.top() + 29.0),
    );
    for (i, icon) in [icons::SVG_LAYERS, icons::SVG_STAR, icons::SVG_CLOCK]
        .into_iter()
        .enumerate()
    {
        icons::render_svg_at(
            ui,
            format!("effects-nav-{i}"),
            icon,
            egui::vec2(22.0, 22.0),
            colors::TEXT_PRIMARY,
            egui::pos2(side.left() + 25.0, side.top() + 91.0 + i as f32 * 42.0),
        );
    }
    for i in 0..10 {
        let icon = if i == 7 {
            icons::SVG_LIGHT
        } else {
            icons::SVG_FOLDER
        };
        icons::render_svg_at(
            ui,
            format!("effects-category-{i}"),
            icon,
            egui::vec2(20.0, 20.0),
            if i == 7 { ORANGE } else { colors::TEXT_PRIMARY },
            egui::pos2(side.left() + 25.0, side.top() + 265.0 + i as f32 * 34.0),
        );
    }
    let p = ui.painter().clone();
    p.text(
        egui::pos2(side.left() + 84.0, side.top() + 44.0),
        egui::Align2::LEFT_CENTER,
        "エフェクト",
        egui::FontId::proportional(if compact { 20.0 } else { 23.0 }),
        colors::TEXT_PRIMARY,
    );
    let top = side.top() + 82.0;
    for (i, label) in ["すべて", "お気に入り", "最近使用"].into_iter().enumerate() {
        let y = top + i as f32 * 42.0;
        let active = i == 0;
        if active {
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y),
                    egui::vec2(width - 20.0, 40.0),
                ),
                6.0,
                crate::ui::theme::colors::BG_PANEL,
            );
            p.rect_filled(
                egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y), egui::vec2(4.0, 40.0)),
                2.0,
                ORANGE,
            );
        }
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 78.0, y + 2.0),
            egui::pos2(side.right() - 18.0, y + 38.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
    }
    p.line_segment(
        [
            egui::pos2(side.left() + 20.0, top + 145.0),
            egui::pos2(side.right() - 20.0, top + 145.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let video_rows = [
        "ぼかし",
        "カラー補正",
        "キーイング",
        "ノイズ＆グレイン",
        "スタイライズ",
        "ディストーション",
        "生成",
        "発光",
        "時間",
        "ユーティリティ",
    ];
    let mut y = top + 177.0;
    for (i, label) in video_rows.into_iter().enumerate() {
        if i == 0 {
            p.text(
                egui::pos2(side.left() + 20.0, y),
                egui::Align2::LEFT_CENTER,
                "ビデオエフェクト",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            p.text(
                egui::pos2(side.right() - 22.0, y),
                egui::Align2::RIGHT_CENTER,
                "⌃",
                egui::FontId::proportional(13.0),
                colors::TEXT_SECONDARY,
            );
            y += 28.0;
        }
        let active = i == 7;
        if active {
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y - 18.0),
                    egui::vec2(width - 20.0, 43.0),
                ),
                5.0,
                egui::Color32::from_rgb(50, 38, 31),
            );
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(side.left() + 10.0, y - 18.0),
                    egui::vec2(4.0, 43.0),
                ),
                2.0,
                ORANGE,
            );
        }
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 76.0, y - 16.0),
            egui::pos2(side.right() - 45.0, y + 16.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(egui::RichText::new(label).size(13.0).color(if active {
                ORANGE
            } else {
                colors::TEXT_PRIMARY
            }))
            .truncate(),
        );
        p.text(
            egui::pos2(side.right() - 24.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{}", [12, 18, 10, 9, 12, 11, 16, 14, 9, 10][i]),
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        y += 34.0;
    }
    p.line_segment(
        [
            egui::pos2(side.left() + 20.0, y - 9.0),
            egui::pos2(side.right() - 20.0, y - 9.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    y += 22.0;
    p.text(
        egui::pos2(side.left() + 20.0, y),
        egui::Align2::LEFT_CENTER,
        "プリセット",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(side.right() - 22.0, y),
        egui::Align2::RIGHT_CENTER,
        "⌃",
        egui::FontId::proportional(13.0),
        colors::TEXT_SECONDARY,
    );
    y += 28.0;
    for (i, label) in ["ユーザープリセット", "内蔵プリセット"]
        .into_iter()
        .enumerate()
    {
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(side.left() + 76.0, y - 16.0),
            egui::pos2(side.right() - 45.0, y + 16.0),
        );
        ui.put(
            label_rect,
            egui::Label::new(
                egui::RichText::new(label)
                    .size(13.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
        p.text(
            egui::pos2(side.right() - 24.0, y),
            egui::Align2::RIGHT_CENTER,
            if i == 0 { "5" } else { "24" },
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        y += 34.0;
    }
}

fn draw_center(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    compact: bool,
    mobile: bool,
    app: &mut KagariApp,
) {
    let pad = if mobile {
        16.0
    } else if compact {
        18.0
    } else {
        30.0
    };
    let p = ui.painter().clone();
    p.text(
        egui::pos2(rect.left() + pad, rect.top() + 43.0),
        egui::Align2::LEFT_CENTER,
        "エフェクト",
        egui::FontId::proportional(if mobile { 28.0 } else { 36.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.left() + pad, rect.top() + 76.0),
        egui::Align2::LEFT_CENTER,
        "映像表現を広げる、豊富なエフェクトライブラリ",
        egui::FontId::proportional(15.0),
        colors::TEXT_SECONDARY,
    );

    let list_view = ctx.data_mut(|d| {
        d.get_temp::<bool>(egui::Id::new("effects-list-view"))
            .unwrap_or(false)
    });
    let toolbar_top = rect.top() + 105.0;
    let toolbar_h = 42.0;
    let toggle_w = if mobile { 0.0 } else { 92.0 };
    let sort_w = if mobile { 0.0 } else { 125.0 };
    let gaps = if mobile { 0.0 } else { 28.0 };
    let search_w = (rect.width() - pad * 2.0 - sort_w - toggle_w - gaps).max(130.0);
    let search = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad, toolbar_top),
        egui::vec2(search_w, toolbar_h),
    );
    p.rect(
        search,
        6.0,
        crate::ui::theme::colors::BG_PANEL_ALT,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    p.text(
        egui::pos2(search.left() + 42.0, search.center().y),
        egui::Align2::LEFT_CENTER,
        "エフェクトを検索...",
        egui::FontId::proportional(14.0),
        colors::TEXT_SECONDARY,
    );
    icons::render_svg_at(
        ui,
        "effects-search".to_string(),
        icons::SVG_SEARCH,
        egui::vec2(20.0, 20.0),
        colors::TEXT_SECONDARY,
        egui::pos2(search.left() + 12.0, search.top() + 11.0),
    );
    if !mobile {
        let sort_rect = egui::Rect::from_min_size(
            egui::pos2(search.right() + 14.0, toolbar_top),
            egui::vec2(sort_w, toolbar_h),
        );
        p.rect(
            sort_rect,
            6.0,
            crate::ui::theme::colors::BG_PANEL_ALT,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );
        p.text(
            sort_rect.center(),
            egui::Align2::CENTER_CENTER,
            "人気順",
            egui::FontId::proportional(13.0),
            colors::TEXT_PRIMARY,
        );
        icons::render_svg_at(
            ui,
            "effects-sort-chevron".to_string(),
            icons::SVG_CHEVRON_DOWN,
            egui::vec2(16.0, 16.0),
            colors::TEXT_PRIMARY,
            egui::pos2(sort_rect.right() - 24.0, sort_rect.top() + 13.0),
        );
        let view_rect = egui::Rect::from_min_size(
            egui::pos2(sort_rect.right() + 14.0, toolbar_top),
            egui::vec2(toggle_w, toolbar_h),
        );
        p.rect(
            view_rect,
            6.0,
            crate::ui::theme::colors::BG_PANEL_ALT,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );
        let grid_rect = egui::Rect::from_min_size(view_rect.min, egui::vec2(46.0, toolbar_h));
        let list_rect = egui::Rect::from_min_size(
            egui::pos2(view_rect.left() + 46.0, view_rect.top()),
            egui::vec2(46.0, toolbar_h),
        );
        p.rect_filled(
            if list_view { list_rect } else { grid_rect },
            5.0,
            egui::Color32::from_rgb(31, 47, 59),
        );
        if ui
            .interact(
                grid_rect,
                egui::Id::new("effects-grid-view"),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("effects-list-view"), false));
        }
        if ui
            .interact(
                list_rect,
                egui::Id::new("effects-list-view"),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("effects-list-view"), true));
        }
        icons::render_svg_at(
            ui,
            "effects-grid-icon".to_string(),
            icons::SVG_GRID,
            egui::vec2(18.0, 18.0),
            colors::TEXT_PRIMARY,
            egui::pos2(grid_rect.left() + 14.0, grid_rect.top() + 12.0),
        );
        icons::render_svg_at(
            ui,
            "effects-list-icon".to_string(),
            icons::SVG_SORT,
            egui::vec2(18.0, 18.0),
            colors::TEXT_PRIMARY,
            egui::pos2(list_rect.left() + 14.0, list_rect.top() + 12.0),
        );
    }

    let effects = [
        ("Gaussian Blur", "Blur", "assets/studio/assets_smoke.webp"),
        ("Glow", "Light", "assets/studio/assets_light_leak.webp"),
        (
            "Film Grain",
            "Texture",
            "assets/studio/assets_glass_texture.webp",
        ),
        (
            "Color Balance",
            "Color",
            "assets/studio/assets_particles.webp",
        ),
        ("Vignette", "Blur", "assets/studio/assets_mountain.webp"),
        (
            "Lens Flare",
            "Light",
            "assets/studio/assets_light_leak.webp",
        ),
        (
            "Chromatic Aberration",
            "Distort",
            "assets/studio/assets_city.webp",
        ),
        (
            "Light Leak",
            "Light",
            "assets/studio/assets_light_leak.webp",
        ),
        ("Halftone", "Stylize", "assets/studio/assets_floor_ref.webp"),
    ];
    let selected_index = ctx.data_mut(|d| {
        d.get_temp::<usize>(egui::Id::new("effect-selected"))
            .unwrap_or(1)
    });
    if list_view || mobile {
        let cols = if mobile { 1 } else { 2 };
        let gap = if compact { 10.0 } else { 16.0 };
        let row_h = if mobile { 54.0 } else { 58.0 };
        let row_w =
            ((rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32).max(150.0);
        let list_top = rect.top() + if mobile { 154.0 } else { 170.0 };
        for (i, (name, category, asset)) in effects.into_iter().enumerate() {
            let x = rect.left() + pad + (i % cols) as f32 * (row_w + gap);
            let y = list_top + (i / cols) as f32 * (row_h + 4.0);
            let row = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(row_w, row_h));
            let response = ui.interact(row, egui::Id::new(("effect-row", i)), egui::Sense::click());
            let selected = i == selected_index;
            if selected || response.hovered() {
                p.rect_filled(
                    row,
                    4.0,
                    egui::Color32::from_rgba_premultiplied(34, 50, 62, 210),
                );
            }
            if selected {
                p.rect_filled(
                    egui::Rect::from_min_size(row.min, egui::vec2(3.0, row.height())),
                    1.5,
                    ORANGE,
                );
            }
            if let Some(id) = texture(ctx, asset, &format!("effect-row-{i}")) {
                p.image(
                    id,
                    egui::Rect::from_min_size(
                        egui::pos2(x + 12.0, y + 8.0),
                        egui::vec2(68.0, row_h - 16.0),
                    ),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            p.text(
                egui::pos2(x + 92.0, y + 21.0),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(if compact { 13.0 } else { 14.0 }),
                colors::TEXT_PRIMARY,
            );
            p.text(
                egui::pos2(x + 92.0, y + 41.0),
                egui::Align2::LEFT_CENTER,
                category,
                egui::FontId::proportional(11.0),
                colors::TEXT_MUTED,
            );
            p.line_segment(
                [
                    egui::pos2(x + 92.0, row.bottom()),
                    egui::pos2(row.right(), row.bottom()),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            );
            if response.clicked() {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("effect-selected"), i));
                apply_effect(app, name);
            }
        }
    } else {
        let cols = 3;
        let gap = 18.0;
        let card_w =
            ((rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32).max(150.0);
        let card_h = if compact { 120.0 } else { 185.0 };
        let thumb_h = if compact { 90.0 } else { 155.0 };
        let row_step = if compact { 132.0 } else { 205.0 };
        let grid_top = rect.top() + if compact { 170.0 } else { 194.0 };
        for (i, (name, _category, asset)) in effects.into_iter().enumerate() {
            let x = rect.left() + pad + (i % cols) as f32 * (card_w + gap);
            let y = grid_top + (i / cols) as f32 * row_step;
            let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));
            let response = ui.interact(
                card,
                egui::Id::new(("effect-card", i)),
                egui::Sense::click(),
            );
            let selected = i == selected_index;
            if response.hovered() && !selected {
                p.rect_filled(
                    card,
                    6.0,
                    egui::Color32::from_rgba_premultiplied(24, 38, 48, 150),
                );
            }
            if let Some(id) = texture(ctx, asset, &format!("effect-card-{i}")) {
                let thumb =
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, thumb_h));
                p.image(
                    id,
                    thumb,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                p.rect_stroke(
                    thumb,
                    5.0,
                    egui::Stroke::new(
                        if selected { 2.0_f32 } else { 1.0_f32 },
                        if selected {
                            ORANGE
                        } else {
                            colors::BORDER_SUBTLE
                        },
                    ),
                );
            }
            p.text(
                egui::pos2(x, y + if compact { 109.0 } else { 174.0 }),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(if compact { 13.0 } else { 16.0 }),
                colors::TEXT_PRIMARY,
            );
            if response.clicked() {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("effect-selected"), i));
                apply_effect(app, name);
            }
        }
    }
}

fn draw_detail(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    compact: bool,
    _app: &mut KagariApp,
) {
    let pad = if compact { 16.0 } else { 22.0 };
    let card = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.top() + 18.0),
        egui::pos2(rect.right() - pad, rect.bottom() * 0.53),
    );
    let p = ui.painter().clone();
    p.rect(
        card,
        8.0,
        PANEL,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    icons::render_svg_at(
        ui,
        "effect-detail-star".to_string(),
        icons::SVG_STAR,
        egui::vec2(25.0, 25.0),
        ORANGE,
        egui::pos2(card.right() - 37.0, card.top() + 23.0),
    );
    p.text(
        egui::pos2(card.left() + 20.0, card.top() + 35.0),
        egui::Align2::LEFT_CENTER,
        "Glow",
        egui::FontId::proportional(if compact { 24.0 } else { 30.0 }),
        colors::TEXT_PRIMARY,
    );
    if compact {
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 65.0),
            egui::Align2::LEFT_CENTER,
            "明るい部分に",
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 81.0),
            egui::Align2::LEFT_CENTER,
            "にじむような発光を適用。",
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
    } else {
        p.text(
            egui::pos2(card.left() + 20.0, card.top() + 70.0),
            egui::Align2::LEFT_CENTER,
            "明るい部分にじむような発光を適用します。",
            egui::FontId::proportional(13.0),
            colors::TEXT_PRIMARY,
        );
    }
    let tags_top = card.top() + if compact { 104.0 } else { 96.0 };
    let tag_gap = 6.0;
    let tag_width = ((card.width() - 40.0 - tag_gap * 2.0) / 3.0).max(52.0);
    for (i, tag) in ["光・発光", "スタイライズ", "よく使う"]
        .into_iter()
        .enumerate()
    {
        let x = card.left() + 20.0 + i as f32 * (tag_width + tag_gap);
        let tag_rect =
            egui::Rect::from_min_size(egui::pos2(x, tags_top), egui::vec2(tag_width, 31.0));
        p.rect(
            tag_rect,
            15.0,
            crate::ui::theme::colors::BG_PANEL,
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            tag_rect.center(),
            egui::Align2::CENTER_CENTER,
            tag,
            egui::FontId::proportional(if compact { 9.0 } else { 10.0 }),
            colors::TEXT_PRIMARY,
        );
    }
    let rows = [("しきい値", "0.60"), ("強さ", "5.00"), ("拡散", "1.00")];
    let slider_left = card.left() + if compact { 116.0 } else { 160.0 };
    let slider_right = card.right() - if compact { 96.0 } else { 84.0 };
    for (i, (label, value)) in rows.into_iter().enumerate() {
        let y = card.top() + 171.0 + i as f32 * 44.0;
        p.text(
            egui::pos2(card.left() + 20.0, y),
            egui::Align2::LEFT_CENTER,
            if i == 0 { "◉" } else { "◷" },
            egui::FontId::proportional(16.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.left() + if compact { 42.0 } else { 50.0 }, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(if compact { 11.0 } else { 14.0 }),
            colors::TEXT_PRIMARY,
        );
        p.line_segment(
            [egui::pos2(slider_left, y), egui::pos2(slider_right, y)],
            egui::Stroke::new(4.0_f32, colors::BORDER_MEDIUM),
        );
        let knob_x = slider_left + (slider_right - slider_left) * (0.55 + i as f32 * 0.12);
        p.line_segment(
            [egui::pos2(slider_left, y), egui::pos2(knob_x, y)],
            egui::Stroke::new(4.0_f32, ORANGE),
        );
        p.circle_filled(egui::pos2(knob_x, y), 6.0, ORANGE);
        p.rect(
            egui::Rect::from_min_size(
                egui::pos2(card.right() - if compact { 76.0 } else { 84.0 }, y - 17.0),
                egui::vec2(if compact { 58.0 } else { 63.0 }, 34.0),
            ),
            5.0,
            egui::Color32::from_rgb(22, 35, 45),
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(card.right() - if compact { 47.0 } else { 52.0 }, y),
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::proportional(if compact { 11.0 } else { 13.0 }),
            colors::TEXT_PRIMARY,
        );
    }
    if !compact {
        p.text(
            egui::pos2(card.left() + 20.0, card.bottom() - 38.0),
            egui::Align2::LEFT_CENTER,
            "カラー     ▣  白",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.right() - 20.0, card.bottom() - 38.0),
            egui::Align2::RIGHT_CENTER,
            "↻ リセット",
            egui::FontId::proportional(12.0),
            colors::TEXT_SECONDARY,
        );
    }
    let preview = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.bottom() * 0.56),
        egui::pos2(rect.right() - pad, rect.bottom() - 18.0),
    );
    p.rect(
        preview,
        8.0,
        PANEL,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(preview.left() + 18.0, preview.top() + 28.0),
        egui::Align2::LEFT_CENTER,
        "プレビュー",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    if let Some(id) = texture(
        ctx,
        "assets/home/continue_working_preview.webp",
        "effect-preview",
    ) {
        p.image(
            id,
            egui::Rect::from_min_max(
                egui::pos2(preview.left() + 12.0, preview.top() + 48.0),
                egui::pos2(preview.right() - 12.0, preview.bottom() - 66.0),
            ),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    crate::ui::icons::render_svg_at(
        ui,
        "effects-preview-play".to_string(),
        crate::ui::icons::SVG_PLAY,
        egui::vec2(14.0, 14.0),
        colors::TEXT_PRIMARY,
        egui::pos2(preview.left() + 12.0, preview.bottom() - 38.0),
    );
    p.text(
        egui::pos2(preview.left() + 34.0, preview.bottom() - 31.0),
        egui::Align2::LEFT_CENTER,
        "00:00 / 00:10",
        egui::FontId::proportional(12.0),
        colors::TEXT_SECONDARY,
    );
}

fn draw_footer(ui: &mut egui::Ui, width: f32, mobile: bool) {
    let p = ui.painter();
    p.text(
        egui::pos2(28.0, if mobile { 17.0 } else { 25.0 }),
        egui::Align2::LEFT_CENTER,
        "Kagari VFX  |  映像で、まだ見ぬ世界を。",
        egui::FontId::proportional(11.0),
        colors::TEXT_SECONDARY,
    );
    if !mobile {
        p.text(
            egui::pos2(width * 0.5, 25.0),
            egui::Align2::CENTER_CENTER,
            "エフェクトライブラリ",
            egui::FontId::proportional(19.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(width - 28.0, 25.0),
            egui::Align2::RIGHT_CENTER,
            "VFX  /  COMPOSITING  /  MORE POSSIBILITIES",
            egui::FontId::proportional(9.0),
            colors::TEXT_MUTED,
        );
    }
}

fn apply_effect(app: &mut KagariApp, name: &str) {
    let Some(idx) = app.selection.selected_layer_idx else {
        app.toasts
            .info("レイヤーを選択するとエフェクトを適用できます");
        return;
    };
    let Some(create_fn) = crate::ui::effects_controls::get_all_effect_presets()
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.create_fn)
    else {
        return;
    };
    let mut session = EditorSession::new(&mut app.history, "Add Effect Library Effect");
    let comp = session.current_mut().active_composition_mut();
    if idx < comp.layers.len() {
        let effect = create_fn(comp.layers[idx].effects.len());
        comp.layers[idx].effects.push(effect);
        session.commit();
        app.toasts.info(format!("{} を適用しました", name));
    }
}

fn texture(ctx: &egui::Context, path: &str, key: &str) -> Option<egui::TextureId> {
    let id = egui::Id::new(("effects-workspace", key));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t.id());
    }
    let img = crate::ui::embedded_assets::open_image(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path))
        .ok()?
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let handle = ctx.load_texture(
        key,
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let out = handle.id();
    ctx.data_mut(|d| d.insert_temp(id, handle));
    Some(out)
}
