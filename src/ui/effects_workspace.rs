use crate::core::editor::EditorSession;
use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

const ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 106, 24);
const BG: egui::Color32 = egui::Color32::from_rgb(8, 16, 22);
const PANEL: egui::Color32 = egui::Color32::from_rgb(15, 26, 34);

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 1150.0;
    let mobile = width < 760.0;
    egui::TopBottomPanel::top("effects_workspace_bar")
        .exact_height(if mobile { 52.0 } else { 60.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(14, 23, 30)))
        .show(ctx, |ui| draw_topbar(ui, width, mobile));
    egui::TopBottomPanel::bottom("effects_workspace_footer")
        .exact_height(if mobile { 34.0 } else { 50.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(9, 16, 22)))
        .show(ctx, |ui| draw_footer(ui, width, mobile));
    egui::CentralPanel::default().frame(egui::Frame::none().fill(BG)).show(ctx, |ui| {
        let rect = ui.max_rect();
        let left_w = if mobile { 0.0 } else if compact { 225.0 } else { 293.0 };
        let right_w = if mobile { 0.0 } else if compact { 300.0 } else { 548.0 };
        if left_w > 0.0 { draw_sidebar(ui, rect, left_w, compact, app); }
        let center = egui::Rect::from_min_max(egui::pos2(rect.left() + left_w, rect.top()), egui::pos2(rect.right() - right_w, rect.bottom()));
        draw_center(ui, ctx, center, compact, mobile, app);
        if right_w > 0.0 { draw_detail(ui, ctx, egui::Rect::from_min_max(egui::pos2(center.right(), rect.top()), rect.max), compact, app); }
    });
}

fn draw_topbar(ui: &mut egui::Ui, width: f32, mobile: bool) {
    let p = ui.painter();
    for (x, c) in [(23.0, (255, 82, 78)), (46.0, (255, 190, 45)), (69.0, (42, 211, 86))] { p.circle_filled(egui::pos2(x, 23.0), 6.5, egui::Color32::from_rgb(c.0, c.1, c.2)); }
    p.text(egui::pos2(96.0, 23.0), egui::Align2::LEFT_CENTER, "Kagari VFX", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
    if !mobile && width >= 1100.0 {
        p.line_segment([egui::pos2(270.0, 12.0), egui::pos2(270.0, 47.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        p.text(egui::pos2(290.0, 23.0), egui::Align2::LEFT_CENTER, "映像に、まだ見ぬ世界を。", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        p.text(egui::pos2(width - 380.0, 23.0), egui::Align2::LEFT_CENTER, "▱  プロジェクトを開く   │   ⊞  新規プロジェクト   │   ⚙  設定", egui::FontId::proportional(13.0), colors::TEXT_SECONDARY);
        p.text(egui::pos2(width - 18.0, 23.0), egui::Align2::RIGHT_CENTER, "×", egui::FontId::proportional(18.0), colors::TEXT_MUTED);
    }
}

fn draw_sidebar(ui: &mut egui::Ui, rect: egui::Rect, width: f32, compact: bool, _app: &mut KagariApp) {
    let side = egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + width, rect.bottom()));
    {
        let p = ui.painter();
        p.rect_filled(side, 0.0, egui::Color32::from_rgb(13, 23, 30));
        p.line_segment([egui::pos2(side.right(), side.top()), egui::pos2(side.right(), side.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    }
    icons::render_svg_at(ui, "effects-heading".to_string(), icons::SVG_EFFECTS, egui::vec2(30.0, 30.0), colors::TEXT_PRIMARY, egui::pos2(side.left() + 20.0, side.top() + 29.0));
    for (i, icon) in [icons::SVG_LAYERS, icons::SVG_STAR, icons::SVG_CLOCK].into_iter().enumerate() { icons::render_svg_at(ui, format!("effects-nav-{i}"), icon, egui::vec2(22.0, 22.0), colors::TEXT_PRIMARY, egui::pos2(side.left() + 25.0, side.top() + 91.0 + i as f32 * 42.0)); }
    for i in 0..10 { let icon = if i == 7 { icons::SVG_LIGHT } else { icons::SVG_FOLDER }; icons::render_svg_at(ui, format!("effects-category-{i}"), icon, egui::vec2(20.0, 20.0), if i == 7 { ORANGE } else { colors::TEXT_PRIMARY }, egui::pos2(side.left() + 25.0, side.top() + 265.0 + i as f32 * 34.0)); }
    let p = ui.painter();
    p.text(egui::pos2(side.left() + 84.0, side.top() + 44.0), egui::Align2::LEFT_CENTER, "エフェクト", egui::FontId::proportional(if compact { 20.0 } else { 23.0 }), colors::TEXT_PRIMARY);
    let top = side.top() + 82.0;
    for (i, label) in ["すべて", "お気に入り", "最近使用"].into_iter().enumerate() {
        let y = top + i as f32 * 42.0;
        let active = i == 0;
        if active { p.rect_filled(egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y), egui::vec2(width - 20.0, 40.0)), 6.0, egui::Color32::from_rgb(28, 43, 55)); p.rect_filled(egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y), egui::vec2(4.0, 40.0)), 2.0, ORANGE); }
        p.text(egui::pos2(side.left() + 78.0, y + 20.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
    }
    p.line_segment([egui::pos2(side.left() + 20.0, top + 145.0), egui::pos2(side.right() - 20.0, top + 145.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    let video_rows = ["ぼかし", "カラー補正", "キーイング", "ノイズ＆グレイン", "スタイライズ", "ディストーション", "生成", "発光", "時間", "ユーティリティ"];
    let mut y = top + 177.0;
    for (i, label) in video_rows.into_iter().enumerate() {
            if i == 0 { p.text(egui::pos2(side.left() + 20.0, y), egui::Align2::LEFT_CENTER, "ビデオエフェクト", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY); p.text(egui::pos2(side.right() - 22.0, y), egui::Align2::RIGHT_CENTER, "⌃", egui::FontId::proportional(13.0), colors::TEXT_SECONDARY); y += 28.0; }
            let active = i == 7;
            if active { p.rect_filled(egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y - 18.0), egui::vec2(width - 20.0, 43.0)), 5.0, egui::Color32::from_rgb(50, 38, 31)); p.rect_filled(egui::Rect::from_min_size(egui::pos2(side.left() + 10.0, y - 18.0), egui::vec2(4.0, 43.0)), 2.0, ORANGE); }
            p.text(egui::pos2(side.left() + 76.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), if active { ORANGE } else { colors::TEXT_PRIMARY });
            p.text(egui::pos2(side.right() - 24.0, y), egui::Align2::RIGHT_CENTER, format!("{}", [12,18,10,9,12,11,16,14,9,10][i]), egui::FontId::proportional(11.0), colors::TEXT_SECONDARY); y += 34.0;
    }
    p.line_segment([egui::pos2(side.left() + 20.0, y - 9.0), egui::pos2(side.right() - 20.0, y - 9.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE)); y += 22.0;
    p.text(egui::pos2(side.left() + 20.0, y), egui::Align2::LEFT_CENTER, "プリセット", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY); p.text(egui::pos2(side.right() - 22.0, y), egui::Align2::RIGHT_CENTER, "⌃", egui::FontId::proportional(13.0), colors::TEXT_SECONDARY); y += 28.0;
    for (i, label) in ["ユーザープリセット", "内蔵プリセット"].into_iter().enumerate() {
            p.text(egui::pos2(side.left() + 76.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), colors::TEXT_PRIMARY);
            p.text(egui::pos2(side.right() - 24.0, y), egui::Align2::RIGHT_CENTER, if i == 0 { "5" } else { "24" }, egui::FontId::proportional(11.0), colors::TEXT_SECONDARY); y += 34.0;
    }
}

fn draw_center(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect, compact: bool, mobile: bool, app: &mut KagariApp) {
    let pad = if mobile { 16.0 } else if compact { 18.0 } else { 30.0 };
    let search = egui::Rect::from_min_size(egui::pos2(rect.left() + pad, rect.top() + 105.0), egui::vec2((rect.width() - pad * 2.0 - 176.0).max(130.0), 42.0));
    icons::render_svg_at(ui, "effects-search".to_string(), icons::SVG_SEARCH, egui::vec2(20.0, 20.0), colors::TEXT_SECONDARY, egui::pos2(search.left() + 12.0, search.top() + 11.0));
    icons::render_svg_at(ui, "effects-sort-chevron".to_string(), icons::SVG_CHEVRON_DOWN, egui::vec2(16.0, 16.0), colors::TEXT_PRIMARY, egui::pos2(search.right() + 117.0, search.top() + 13.0));
    let p = ui.painter();
    p.text(egui::pos2(rect.left() + pad, rect.top() + 43.0), egui::Align2::LEFT_CENTER, "エフェクト", egui::FontId::proportional(if mobile { 28.0 } else { 36.0 }), colors::TEXT_PRIMARY);
    p.text(egui::pos2(rect.left() + pad, rect.top() + 76.0), egui::Align2::LEFT_CENTER, "映像表現を広げる、豊富なエフェクトライブラリ", egui::FontId::proportional(15.0), colors::TEXT_SECONDARY);
    p.rect(search, 6.0, egui::Color32::from_rgb(16, 29, 38), egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
    p.text(egui::pos2(search.left() + 42.0, search.center().y), egui::Align2::LEFT_CENTER, "エフェクトを検索...", egui::FontId::proportional(14.0), colors::TEXT_SECONDARY);
    p.rect(egui::Rect::from_min_size(egui::pos2(search.right() + 18.0, search.top()), egui::vec2(125.0, 42.0)), 6.0, egui::Color32::from_rgb(16, 29, 38), egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
    p.text(egui::pos2(search.right() + 67.0, search.center().y), egui::Align2::CENTER_CENTER, "人気順", egui::FontId::proportional(13.0), colors::TEXT_PRIMARY);
    p.text(egui::pos2(rect.right() - pad - 44.0, search.center().y), egui::Align2::CENTER_CENTER, "GRID  LIST", egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
    let cards = [("Gaussian Blur", "assets/studio/assets_smoke.webp"), ("Glow", "assets/studio/assets_light_leak.webp"), ("Film Grain", "assets/studio/assets_glass_texture.webp"), ("Color Balance", "assets/studio/assets_particles.webp"), ("Vignette", "assets/studio/assets_mountain.webp"), ("Lens Flare", "assets/studio/assets_light_leak.webp"), ("Chromatic Aberration", "assets/studio/assets_city.webp"), ("Light Leak", "assets/studio/assets_light_leak.webp"), ("Halftone", "assets/studio/assets_floor_ref.webp")];
    let cols = if mobile { 1 } else { 3 };
    let gap = if compact { 14.0 } else { 20.0 };
    let card_w = ((rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32).max(130.0);
    let card_h = if mobile { 130.0 } else { 155.0 };
    for (i, (name, asset)) in cards.into_iter().enumerate() {
        let x = rect.left() + pad + (i % cols) as f32 * (card_w + gap);
        let y = rect.top() + 170.0 + (i / cols) as f32 * (card_h + 32.0);
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));
        let response = ui.interact(card, egui::Id::new(("effect-card", i)), egui::Sense::click());
        let selected = i == ctx.data_mut(|d| d.get_temp::<usize>(egui::Id::new("effect-selected")).unwrap_or(1));
        p.rect(card, 7.0, PANEL, egui::Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, if selected { ORANGE } else { colors::BORDER_SUBTLE }));
        if let Some(id) = texture(ctx, asset, &format!("effect-card-{i}")) { p.image(id, egui::Rect::from_min_size(egui::pos2(x + 2.0, y + 2.0), egui::vec2(card_w - 4.0, card_h - 40.0)), egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE); }
        p.text(egui::pos2(x, card.bottom() + 18.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(15.0), colors::TEXT_PRIMARY);
        if selected { p.text(egui::pos2(card.right() - 18.0, y + 18.0), egui::Align2::CENTER_CENTER, "★", egui::FontId::proportional(23.0), ORANGE); }
        if response.clicked() { ctx.data_mut(|d| d.insert_temp(egui::Id::new("effect-selected"), i)); apply_effect(app, name); }
    }
}

fn draw_detail(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect, compact: bool, _app: &mut KagariApp) {
    let pad = if compact { 16.0 } else { 22.0 };
    let card = egui::Rect::from_min_max(egui::pos2(rect.left() + pad, rect.top() + 18.0), egui::pos2(rect.right() - pad, rect.bottom() * 0.53));
    icons::render_svg_at(ui, "effect-detail-star".to_string(), icons::SVG_STAR, egui::vec2(25.0, 25.0), ORANGE, egui::pos2(card.right() - 37.0, card.top() + 23.0));
    let p = ui.painter();
    p.rect(card, 8.0, PANEL, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    p.text(egui::pos2(card.left() + 20.0, card.top() + 35.0), egui::Align2::LEFT_CENTER, "Glow", egui::FontId::proportional(if compact { 24.0 } else { 30.0 }), colors::TEXT_PRIMARY);
    p.text(egui::pos2(card.left() + 20.0, card.top() + 70.0), egui::Align2::LEFT_CENTER, "明るい部分にじむような発光を適用します。", egui::FontId::proportional(13.0), colors::TEXT_PRIMARY);
    for (i, tag) in ["光・発光", "スタイライズ", "よく使う"].into_iter().enumerate() { let x = card.left() + 20.0 + i as f32 * 97.0; p.rect(egui::Rect::from_min_size(egui::pos2(x, card.top() + 96.0), egui::vec2(86.0, 31.0)), 15.0, egui::Color32::from_rgb(28, 43, 55), egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE)); p.text(egui::pos2(x + 43.0, card.top() + 111.0), egui::Align2::CENTER_CENTER, tag, egui::FontId::proportional(10.0), colors::TEXT_PRIMARY); }
    let rows = [("しきい値", "0.60"), ("強さ", "5.00"), ("拡散", "1.00")];
    let slider_left = card.left() + if compact { 116.0 } else { 160.0 };
    let slider_right = card.right() - if compact { 96.0 } else { 160.0 };
    for (i, (label, value)) in rows.into_iter().enumerate() { let y = card.top() + 171.0 + i as f32 * 44.0; p.text(egui::pos2(card.left() + 20.0, y), egui::Align2::LEFT_CENTER, if i == 0 { "◉" } else { "◷" }, egui::FontId::proportional(16.0), colors::TEXT_PRIMARY); p.text(egui::pos2(card.left() + if compact { 42.0 } else { 50.0 }, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(if compact { 11.0 } else { 14.0 }), colors::TEXT_PRIMARY); p.line_segment([egui::pos2(slider_left, y), egui::pos2(slider_right, y)], egui::Stroke::new(4.0_f32, colors::BORDER_MEDIUM)); let knob_x = slider_left + (slider_right - slider_left) * (0.55 + i as f32 * 0.12); p.line_segment([egui::pos2(slider_left, y), egui::pos2(knob_x, y)], egui::Stroke::new(4.0_f32, ORANGE)); p.circle_filled(egui::pos2(knob_x, y), 6.0, ORANGE); p.rect(egui::Rect::from_min_size(egui::pos2(card.right() - if compact { 76.0 } else { 84.0 }, y - 17.0), egui::vec2(if compact { 58.0 } else { 63.0 }, 34.0)), 5.0, egui::Color32::from_rgb(22, 35, 45), egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE)); p.text(egui::pos2(card.right() - if compact { 47.0 } else { 52.0 }, y), egui::Align2::CENTER_CENTER, value, egui::FontId::proportional(if compact { 11.0 } else { 13.0 }), colors::TEXT_PRIMARY); }
    if !compact {
        p.text(egui::pos2(card.left() + 20.0, card.bottom() - 38.0), egui::Align2::LEFT_CENTER, "カラー     ▣  白", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        p.text(egui::pos2(card.right() - 20.0, card.bottom() - 38.0), egui::Align2::RIGHT_CENTER, "↻ リセット", egui::FontId::proportional(12.0), colors::TEXT_SECONDARY);
    }
    let preview = egui::Rect::from_min_max(egui::pos2(rect.left() + pad, rect.bottom() * 0.56), egui::pos2(rect.right() - pad, rect.bottom() - 18.0));
    p.rect(preview, 8.0, PANEL, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    p.text(egui::pos2(preview.left() + 18.0, preview.top() + 28.0), egui::Align2::LEFT_CENTER, "プレビュー", egui::FontId::proportional(18.0), colors::TEXT_PRIMARY);
    if let Some(id) = texture(ctx, "assets/home/continue_working_preview.webp", "effect-preview") { p.image(id, egui::Rect::from_min_max(egui::pos2(preview.left() + 12.0, preview.top() + 48.0), egui::pos2(preview.right() - 12.0, preview.bottom() - 66.0)), egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE); }
    p.text(egui::pos2(preview.left() + 18.0, preview.bottom() - 31.0), egui::Align2::LEFT_CENTER, "▶     00:00 / 00:10", egui::FontId::proportional(12.0), colors::TEXT_SECONDARY);
}

fn draw_footer(ui: &mut egui::Ui, width: f32, mobile: bool) { let p = ui.painter(); p.text(egui::pos2(28.0, if mobile { 17.0 } else { 25.0 }), egui::Align2::LEFT_CENTER, "Kagari VFX  |  映像で、まだ見ぬ世界を。", egui::FontId::proportional(11.0), colors::TEXT_SECONDARY); if !mobile { p.text(egui::pos2(width * 0.5, 25.0), egui::Align2::CENTER_CENTER, "エフェクトライブラリ", egui::FontId::proportional(19.0), colors::TEXT_PRIMARY); p.text(egui::pos2(width - 28.0, 25.0), egui::Align2::RIGHT_CENTER, "VFX  /  COMPOSITING  /  MORE POSSIBILITIES", egui::FontId::proportional(9.0), colors::TEXT_MUTED); } }

fn apply_effect(app: &mut KagariApp, name: &str) {
    let Some(idx) = app.selection.selected_layer_idx else { app.toasts.info("レイヤーを選択するとエフェクトを適用できます"); return; };
    let Some(create_fn) = crate::ui::effects_controls::get_all_effect_presets().iter().find(|p| p.name == name).map(|p| p.create_fn) else { return; };
    let mut session = EditorSession::new(&mut app.history, "Add Effect Library Effect");
    let comp = session.current_mut().active_composition_mut();
    if idx < comp.layers.len() { let effect = create_fn(comp.layers[idx].effects.len()); comp.layers[idx].effects.push(effect); session.commit(); app.toasts.info(format!("{} を適用しました", name)); }
}

fn texture(ctx: &egui::Context, path: &str, key: &str) -> Option<egui::TextureId> { let id = egui::Id::new(("effects-workspace", key)); if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) { return Some(t.id()); } let img = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).ok()?.to_rgba8(); let size = [img.width() as usize, img.height() as usize]; let handle = ctx.load_texture(key, egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR); let out = handle.id(); ctx.data_mut(|d| d.insert_temp(id, handle)); Some(out) }
