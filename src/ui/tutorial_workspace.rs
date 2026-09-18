use crate::ui::theme::colors;
use crate::ui::icons;
use crate::KagariApp;
use eframe::egui;

const ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 106, 24);
const PANEL: egui::Color32 = egui::Color32::from_rgb(15, 24, 31);

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let compact = width < 1100.0;
    let mobile = width < 760.0;
    egui::TopBottomPanel::top("tutorial_window_bar")
        .exact_height(if mobile { 52.0 } else { 60.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(15, 23, 30)))
        .show(ctx, |ui| draw_topbar(ui, width, mobile));
    egui::TopBottomPanel::bottom("tutorial_footer")
        .exact_height(if mobile { 38.0 } else { 68.0 })
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(9, 16, 22)))
        .show(ctx, |ui| draw_footer(ui, width, mobile));
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(8, 15, 21)))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let nav_w = if mobile { 0.0 } else if compact { 230.0 } else { 340.0 };
            let right_w = if mobile || compact { 0.0 } else { 498.0 };
            if nav_w > 0.0 {
                draw_chapters(ui, rect, nav_w, compact, app);
            }
            let main = egui::Rect::from_min_max(
                egui::pos2(rect.left() + nav_w, rect.top()),
                egui::pos2(rect.right() - right_w, rect.bottom()),
            );
            draw_main(ui, ctx, main, compact, mobile, app);
            if right_w > 0.0 {
                draw_about(ui, rect, main.right(), right_w, compact);
            }
        });
}

fn draw_topbar(ui: &mut egui::Ui, width: f32, mobile: bool) {
    if !mobile && width >= 1100.0 {
        let right = width - 18.0;
        let settings = egui::Rect::from_min_max(egui::pos2(right - 72.0, 5.0), egui::pos2(right, 55.0));
        let new_project = egui::Rect::from_min_max(egui::pos2(settings.left() - 152.0, 5.0), egui::pos2(settings.left() - 8.0, 55.0));
        let open_project = egui::Rect::from_min_max(egui::pos2(new_project.left() - 182.0, 5.0), egui::pos2(new_project.left() - 8.0, 55.0));
        tutorial_header_action(ui, open_project, icons::SVG_FOLDER, "プロジェクトを開く", "tutorial-open-project");
        tutorial_header_action(ui, new_project, icons::SVG_FILE_PLUS, "新規プロジェクト", "tutorial-new-project");
        tutorial_header_action(ui, settings, icons::SVG_SETTINGS, "設定", "tutorial-settings");
    }
    let p = ui.painter();
    for (x, c) in [(23.0, (255, 82, 78)), (46.0, (255, 190, 45)), (69.0, (42, 211, 86))] {
        p.circle_filled(egui::pos2(x, 24.0), 6.5, egui::Color32::from_rgb(c.0, c.1, c.2));
    }
    p.text(egui::pos2(96.0, 24.0), egui::Align2::LEFT_CENTER, "Kagari VFX", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
    if !mobile && width >= 1100.0 {
        p.line_segment([egui::pos2(264.0, 14.0), egui::pos2(264.0, 45.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        p.text(egui::pos2(291.0, 24.0), egui::Align2::LEFT_CENTER, "映像に、まだ見ぬ世界を。", egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        p.text(egui::pos2(width - 18.0, 24.0), egui::Align2::RIGHT_CENTER, "×", egui::FontId::proportional(18.0), colors::TEXT_MUTED);
    }
}

fn tutorial_header_action(ui: &mut egui::Ui, rect: egui::Rect, icon: &'static str, label: &str, id: &str) {
    let response = ui.interact(rect, egui::Id::new(id), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, egui::Color32::from_rgba_unmultiplied(48, 61, 72, 110));
    }
    let icon_size = egui::vec2(20.0, 20.0);
    icons::render_svg_at(ui, id.to_string(), icon, icon_size, colors::TEXT_SECONDARY, egui::pos2(rect.left() + 8.0, rect.center().y - 10.0));
    let text_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 36.0, rect.top()), egui::pos2(rect.right() - 4.0, rect.bottom()));
    ui.put(text_rect, egui::Label::new(egui::RichText::new(label).size(13.0).color(colors::TEXT_SECONDARY)).truncate());
}

fn draw_chapters(ui: &mut egui::Ui, rect: egui::Rect, width: f32, compact: bool, app: &mut KagariApp) {
    {
        let p = ui.painter();
        p.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + width, rect.bottom())), 0.0, egui::Color32::from_rgb(13, 22, 29));
        p.line_segment([egui::pos2(rect.left() + width, rect.top()), egui::pos2(rect.left() + width, rect.bottom())], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    }
    icons::render_svg_at(ui, "tutorial-chapter-heading".to_string(), icons::SVG_LAYERS, egui::vec2(55.0, 55.0), colors::TEXT_PRIMARY, egui::pos2(rect.left() + 30.0, rect.top() + 20.0));
    let chapter_icons = [icons::SVG_HOME, icons::SVG_FILE, icons::SVG_LAYERS, icons::SVG_MARKER, icons::SVG_EFFECTS, icons::SVG_PALETTE, icons::SVG_FILE_PLUS, icons::SVG_BOOK];
    for (i, icon) in chapter_icons.into_iter().enumerate() { icons::render_svg_at(ui, format!("tutorial-chapter-{i}"), icon, egui::vec2(30.0, 30.0), if i == 0 { ORANGE } else { colors::TEXT_PRIMARY }, egui::pos2(rect.left() + 28.0, rect.top() + 97.0 + i as f32 * if compact { 61.0 } else { 78.0 })); }
    let p = ui.painter().clone();
    let logo_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 36.0, rect.top() + 20.0), egui::vec2(55.0, 55.0));
    if let Some(id) = texture(ctx_for(ui), "assets/kagari_logo.webp", "tutorial-logo") {
        p.image(id, logo_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
    p.text(egui::pos2(rect.left() + 105.0, rect.top() + 48.0), egui::Align2::LEFT_CENTER, "Kagari", egui::FontId::proportional(27.0), colors::TEXT_PRIMARY);
    p.text(egui::pos2(rect.left() + 185.0, rect.top() + 48.0), egui::Align2::LEFT_CENTER, "VFX", egui::FontId::proportional(27.0), colors::TEXT_SECONDARY);
    let chapters = [
        ("はじめに", "Kagari VFX の紹介"), ("基本操作", "画面の見方・操作方法"),
        ("レイヤー", "合成の基本"), ("マスクとトラック", "マスク・モーショントラッキング"),
        ("エフェクト", "VFX エフェクトの使い方"), ("カラーグレーディング", "色調補正・ルック開発"),
        ("書き出し", "レンダリング・書き出し設定"), ("チュートリアル", "すべてのチュートリアル"),
    ];
    for (i, (title, sub)) in chapters.into_iter().enumerate() {
        let item_h = if compact { 57.0 } else { 72.0 };
        let y = rect.top() + 25.0 + i as f32 * if compact { 61.0 } else { 78.0 } + 72.0;
        let item = egui::Rect::from_min_size(egui::pos2(rect.left() + 13.0, y), egui::vec2(width - 25.0, item_h));
        let response = ui.interact(item, egui::Id::new(("tutorial-chapter", i)), egui::Sense::click());
        if i == 0 {
            p.rect_filled(item, 7.0, egui::Color32::from_rgb(29, 32, 36));
            p.rect_filled(egui::Rect::from_min_size(item.min, egui::vec2(5.0, item.height())), 3.0, ORANGE);
        } else if response.hovered() {
            p.rect_filled(item, 7.0, egui::Color32::from_rgb(23, 31, 38));
        }
        let label_left = item.left() + 62.0;
        let label_right = item.right() - 12.0;
        ui.put(
            egui::Rect::from_min_max(egui::pos2(label_left, item.top() + 3.0), egui::pos2(label_right, item.top() + item_h * 0.53)),
            egui::Label::new(egui::RichText::new(title).size(if compact { 13.0 } else { 16.0 }).color(colors::TEXT_PRIMARY)).truncate(),
        );
        ui.put(
            egui::Rect::from_min_max(egui::pos2(label_left, item.top() + item_h * 0.49), egui::pos2(label_right, item.bottom() - 2.0)),
            egui::Label::new(egui::RichText::new(sub).size(if compact { 9.0 } else { 12.0 }).color(colors::TEXT_SECONDARY)).truncate(),
        );
        if response.clicked() { app.tutorial_step = i; }
    }
    if compact { return; }
    let thumb = egui::Rect::from_min_max(egui::pos2(rect.left() + 14.0, rect.bottom() - 133.0), egui::pos2(rect.left() + width - 13.0, rect.bottom()));
    if let Some(id) = texture(ctx_for(ui), "assets/studio/studio_city_reference.webp", "tutorial-nav-thumb") { p.image(id, thumb, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::from_rgba_unmultiplied(255,255,255,150)); }
    p.text(egui::pos2(thumb.left() + 28.0, thumb.bottom() - 40.0), egui::Align2::LEFT_CENTER, "C R E A T E    C O M P O S I T E", egui::FontId::proportional(8.0), colors::TEXT_PRIMARY);
}

fn draw_main(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect, compact: bool, mobile: bool, app: &mut KagariApp) {
    let p = ui.painter();
    let pad = if mobile { 18.0 } else if compact { 24.0 } else { 34.0 };
    p.text(egui::pos2(rect.left() + pad, rect.top() + 75.0), egui::Align2::LEFT_CENTER, "T U T O R I A L", egui::FontId::proportional(11.0), colors::TEXT_SECONDARY);
    p.text(egui::pos2(rect.left() + pad, rect.top() + 119.0), egui::Align2::LEFT_CENTER, "はじめに", egui::FontId::proportional(if mobile { 32.0 } else { 42.0 }), colors::TEXT_PRIMARY);
    p.text(egui::pos2(rect.left() + pad, rect.top() + 157.0), egui::Align2::LEFT_CENTER, "Kagari VFX で広がる、映像表現の可能性", egui::FontId::proportional(if mobile { 16.0 } else { 21.0 }), colors::TEXT_SECONDARY);
    if !mobile { p.text(egui::pos2(rect.right() - 30.0, rect.top() + 76.0), egui::Align2::RIGHT_CENTER, "V I S U A L   E F F E C T S\nF O R   A   B R I G H T E R   T O M O R R O W", egui::FontId::proportional(8.0), colors::TEXT_SECONDARY); }
    let hero_top = rect.top() + 190.0;
    let hero_h = if mobile { 190.0 } else if compact { 220.0 } else { 310.0 };
    let hero = egui::Rect::from_min_max(egui::pos2(rect.left() + pad, hero_top), egui::pos2(rect.right() - pad, hero_top + hero_h));
    if let Some(id) = texture(ctx, "assets/studio/studio_city_reference.webp", "tutorial-hero") { p.image(id, hero, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE); }
    p.rect_stroke(hero, 8.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
    p.circle_stroke(hero.center(), hero.height().min(hero.width()) * 0.16, egui::Stroke::new(2.0_f32, egui::Color32::WHITE));
    p.text(egui::pos2(hero.left() + 22.0, hero.bottom() - 38.0), egui::Align2::LEFT_CENTER, "Kagari VFX で、\n想像を超えた映像をつくる", egui::FontId::proportional(14.0), egui::Color32::WHITE);
    p.text(egui::pos2(hero.right() - 18.0, hero.bottom() - 22.0), egui::Align2::RIGHT_CENTER, "03:24", egui::FontId::proportional(13.0), egui::Color32::WHITE);
    let _ = p;
    icons::render_svg_at(ui, "tutorial-hero-play".to_string(), icons::SVG_PLAY, egui::vec2(42.0, 42.0), egui::Color32::WHITE, egui::pos2(hero.center().x - 21.0, hero.center().y - 21.0));
    let timeline_top = hero.bottom() + 18.0;
    let timeline_h = if mobile { 112.0 } else if compact { 120.0 } else { 190.0 };
    draw_timeline(ui, egui::Rect::from_min_max(egui::pos2(rect.left() + pad, timeline_top), egui::pos2(rect.right() - pad, timeline_top + timeline_h)), mobile);
    let p = ui.painter();
    let button = egui::Rect::from_min_size(egui::pos2(rect.center().x - 194.0, timeline_top + timeline_h + 22.0), egui::vec2(388.0, 62.0));
    let response = ui.interact(button, egui::Id::new("tutorial-play"), egui::Sense::click());
    p.rect_filled(button, 10.0, if response.hovered() { egui::Color32::from_rgb(255, 125, 32) } else { ORANGE });
    p.text(egui::pos2(button.left() + 82.0, button.center().y), egui::Align2::LEFT_CENTER, "チュートリアルを再生", egui::FontId::proportional(if mobile { 18.0 } else { 21.0 }), egui::Color32::WHITE);
    p.text(egui::pos2(rect.center().x, button.bottom() + 30.0), egui::Align2::CENTER_CENTER, "Kagari VFX の基本とワークフローを動画で学びましょう", egui::FontId::proportional(13.0), colors::TEXT_SECONDARY);
    let _ = p;
    icons::render_svg_at(ui, "tutorial-button-play".to_string(), icons::SVG_PLAY, egui::vec2(22.0, 22.0), egui::Color32::WHITE, egui::pos2(button.left() + 48.0, button.center().y - 11.0));
    if response.clicked() { crate::ui::tutorial::restart(app); }
}

fn draw_timeline(ui: &mut egui::Ui, rect: egui::Rect, mobile: bool) {
    let p = ui.painter();
    p.rect(rect, 7.0, PANEL, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    p.text(egui::pos2(rect.left() + 16.0, rect.top() + 20.0), egui::Align2::LEFT_CENTER, "➜  レイヤー", egui::FontId::proportional(13.0), colors::TEXT_PRIMARY);
    let ruler_x = rect.left() + if mobile { 110.0 } else { 375.0 };
    for i in 0..6 { p.text(egui::pos2(ruler_x + i as f32 * 65.0, rect.top() + 24.0), egui::Align2::CENTER_CENTER, format!("00:{:02}", i * 2), egui::FontId::proportional(9.0), colors::TEXT_SECONDARY); }
    let labels = ["グロー", "パーティクル", "宇宙_背景", "調整レイヤー", "ベース映像"];
    let lane_colors = [egui::Color32::from_rgb(98, 73, 186), egui::Color32::from_rgb(52, 153, 123), egui::Color32::from_rgb(107, 87, 188), egui::Color32::from_rgb(181, 139, 57), egui::Color32::from_rgb(51, 117, 195)];
    for (i, label) in labels.into_iter().enumerate() {
        let y = rect.top() + 47.0 + i as f32 * (rect.height().min(190.0) / 6.0);
        p.line_segment([egui::pos2(rect.left() + 8.0, y + 15.0), egui::pos2(rect.right() - 8.0, y + 15.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        p.text(egui::pos2(rect.left() + 22.0, y), egui::Align2::LEFT_CENTER, "◉", egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
        p.text(egui::pos2(rect.left() + 57.0, y), egui::Align2::LEFT_CENTER, format!("{}   {}", 5 - i, label), egui::FontId::proportional(10.0), colors::TEXT_PRIMARY);
        let bar_x = if mobile { rect.left() + 112.0 } else { rect.left() + rect.width() * 0.52 };
        let bar = egui::Rect::from_min_size(egui::pos2(bar_x - i as f32 * 40.0, y - 9.0), egui::vec2((rect.width() * 0.33 + i as f32 * 22.0).min(rect.right() - bar_x), 18.0));
        p.rect_filled(bar, 3.0, lane_colors[i]);
    }
}

fn draw_about(ui: &mut egui::Ui, rect: egui::Rect, left: f32, _width: f32, compact: bool) {
    let p = ui.painter();
    let panel = egui::Rect::from_min_max(egui::pos2(left + 20.0, rect.top() + 28.0), egui::pos2(rect.right() - 20.0, rect.bottom() - 18.0));
    p.rect(panel, 10.0, PANEL, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    let x = panel.left() + if compact { 26.0 } else { 32.0 };
    p.text(egui::pos2(x, panel.top() + 54.0), egui::Align2::LEFT_CENTER, "Kagari VFX とは", egui::FontId::proportional(if compact { 25.0 } else { 32.0 }), colors::TEXT_PRIMARY);
    p.line_segment([egui::pos2(x, panel.top() + 87.0), egui::pos2(x + 55.0, panel.top() + 87.0)], egui::Stroke::new(2.0_f32, ORANGE));
    let desc = "Kagari VFX は、映像制作のための\nレイヤーベース VFX・コンポジットソフトです。\n直感的な操作と高品質なエフェクトで、\n映画のような映像表現を、すべてのクリエイターに。";
    p.text(egui::pos2(x, panel.top() + 125.0), egui::Align2::LEFT_TOP, desc, egui::FontId::proportional(if compact { 12.0 } else { 15.0 }), colors::TEXT_PRIMARY);
    p.line_segment([egui::pos2(x, panel.top() + 225.0), egui::pos2(panel.right() - 32.0, panel.top() + 225.0)], egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
    p.text(egui::pos2(x, panel.top() + 258.0), egui::Align2::LEFT_CENTER, "このチュートリアルで学べること", egui::FontId::proportional(if compact { 16.0 } else { 19.0 }), colors::TEXT_PRIMARY);
    let items = [("インターフェースの概要", "作業画面の構成と各パネルの役割"), ("レイヤーの基本", "レイヤーの管理と合成の仕組み"), ("マスクとトラッキング", "マスク作成とモーショントラッキングの使い方"), ("エフェクトの適用", "豊富なVFXエフェクトで映像を強化"), ("カラーグレーディング", "色調補正とシネマティックなルックの作り方"), ("レンダリングと書き出し", "最終的な映像の出力設定"), ("実践テクニック", "プロのワークフローから学ぶ応用テクニック")];
    for (i, (title, sub)) in items.into_iter().enumerate() {
        let y = panel.top() + 310.0 + i as f32 * if compact { 58.0 } else { 61.0 };
        p.circle_stroke(egui::pos2(x + 19.0, y), 19.0, egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY));
        p.text(egui::pos2(x + 19.0, y), egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(14.0), colors::TEXT_PRIMARY);
        p.text(egui::pos2(x + 62.0, y - 8.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(if compact { 12.0 } else { 15.0 }), colors::TEXT_PRIMARY);
        p.text(egui::pos2(x + 62.0, y + 14.0), egui::Align2::LEFT_CENTER, sub, egui::FontId::proportional(if compact { 10.0 } else { 12.0 }), colors::TEXT_SECONDARY);
    }
    p.text(egui::pos2(panel.right() - 30.0, panel.bottom() - 28.0), egui::Align2::RIGHT_CENTER, "Create\nWithout Limits.", egui::FontId::proportional(15.0), colors::TEXT_SECONDARY);
}

fn draw_footer(ui: &mut egui::Ui, width: f32, mobile: bool) {
    let p = ui.painter();
    if mobile { p.text(egui::pos2(18.0, 19.0), egui::Align2::LEFT_CENTER, "Kagari VFX  |  チュートリアル", egui::FontId::proportional(10.0), colors::TEXT_SECONDARY); return; }
    p.text(egui::pos2(38.0, 34.0), egui::Align2::LEFT_CENTER, "Kagari VFX  |  映像で、まだ見ぬ世界を。", egui::FontId::proportional(11.0), colors::TEXT_SECONDARY);
    p.line_segment([egui::pos2(width * 0.325, 34.0), egui::pos2(width * 0.39, 34.0)], egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY));
    p.text(egui::pos2(width * 0.5, 34.0), egui::Align2::CENTER_CENTER, "チュートリアル画面", egui::FontId::proportional(25.0), colors::TEXT_PRIMARY);
    p.line_segment([egui::pos2(width * 0.61, 34.0), egui::pos2(width * 0.675, 34.0)], egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY));
    p.text(egui::pos2(width - 38.0, 34.0), egui::Align2::RIGHT_CENTER, "VFX  /  COMPOSITING  /  MORE POSSIBILITIES", egui::FontId::proportional(9.0), colors::TEXT_MUTED);
}

fn ctx_for(ui: &egui::Ui) -> &egui::Context { ui.ctx() }

fn texture(ctx: &egui::Context, path: &str, key: &str) -> Option<egui::TextureId> {
    let id = egui::Id::new(("tutorial-texture", key));
    if let Some(handle) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) { return Some(handle.id()); }
    let image = image::open(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let handle = ctx.load_texture(key, egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()), egui::TextureOptions::LINEAR);
    let out = handle.id();
    ctx.data_mut(|d| d.insert_temp(id, handle));
    Some(out)
}
