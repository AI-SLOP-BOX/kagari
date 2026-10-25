use super::*;

pub(in crate::ui::home_screen) fn draw_reference_documentation_page(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let narrow = reference_narrow(ui);
    let compact = ui.available_height() < 820.0;
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, crate::ui::theme::colors::BG_PANEL_DEEP);
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(
            if narrow {
                8.0
            } else {
                reference_content_margin(ui)
            },
            0.0,
        ))
        .show(ui, |ui| {
            draw_reference_topbar(app, ui, ctx);
            ui.add_space(if narrow { 12.0 } else { 36.0 });
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("ドキュメント")
                            .size(if narrow { 22.0 } else { 30.0 })
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Kagari VFX の使い方と制作ワークフローを確認できます。",
                        )
                        .size(if narrow { 9.0 } else { 13.0 })
                        .color(colors::TEXT_SECONDARY),
                    );
                });
            });
            ui.add_space(if compact { 18.0 } else { 28.0 });
            let cards = [
                (
                    crate::ui::icons::SVG_HOME,
                    "はじめに",
                    "Kagari VFX の画面構成と基本的な考え方",
                ),
                (
                    crate::ui::icons::SVG_LAYERS,
                    "レイヤーと合成",
                    "素材を重ね、調整し、自然な合成を作る",
                ),
                (
                    crate::ui::icons::SVG_EFFECTS,
                    "エフェクト",
                    "Glow、Blur、色補正などの適用方法",
                ),
                (
                    crate::ui::icons::SVG_TRACKING,
                    "トラッキング",
                    "動く素材にマスクやレイヤーを追従させる",
                ),
                (
                    crate::ui::icons::SVG_EXPORT,
                    "書き出し",
                    "映像を目的の形式と解像度で出力する",
                ),
                (
                    crate::ui::icons::SVG_SETTINGS,
                    "設定と環境",
                    "自動保存、キャッシュ、GPU設定を管理する",
                ),
            ];
            let cols = if narrow { 1 } else { 2 };
            let gap = if narrow { 8.0 } else { 14.0 };
            let card_w =
                ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).max(160.0);
            for i in (0..cards.len()).step_by(cols) {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let index = i + col;
                        if index >= cards.len() {
                            break;
                        }
                        let (card_icon, card_title, card_desc) = cards[index];
                        documentation_card(ui, card_icon, card_title, card_desc, card_w, compact);
                        if col + 1 < cols {
                            ui.add_space(gap);
                        }
                    }
                });
                ui.add_space(gap);
            }
            ui.separator();
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("クイックリファレンス")
                    .size(if narrow { 14.0 } else { 18.0 })
                    .strong(),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for shortcut in [
                    "V  選択",
                    "Space  再生",
                    "Cmd / Ctrl + S  保存",
                    "Cmd / Ctrl + M  書き出し",
                    "Cmd / Ctrl + K  コマンドパレット",
                ] {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(shortcut)
                                .size(if narrow { 9.0 } else { 12.0 })
                                .color(colors::TEXT_SECONDARY),
                        )
                        .truncate(),
                    );
                    ui.add_space(18.0);
                }
            });
        });
}

pub(in crate::ui::home_screen) fn documentation_card(
    ui: &mut egui::Ui,
    icon: &'static str,
    title: &str,
    desc: &str,
    width: f32,
    compact: bool,
) {
    let height = if compact { 84.0 } else { 112.0 };
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(width, height),
        title,
        true,
        false,
    );
    ui.painter().rect(
        rect,
        7.0,
        crate::ui::theme::colors::BG_PANEL_BASE,
        egui::Stroke::new(
            1.0_f32,
            if response.hovered() {
                colors::ACCENT_BLUE
            } else {
                colors::BORDER_SUBTLE
            },
        ),
    );
    crate::ui::icons::render_svg_at(
        ui,
        format!("documentation-card-{title}"),
        icon,
        egui::vec2(
            if compact { 22.0 } else { 28.0 },
            if compact { 22.0 } else { 28.0 },
        ),
        colors::TEXT_PRIMARY,
        egui::pos2(rect.left() + 16.0, rect.top() + 16.0),
    );
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 58.0, rect.top() + 13.0),
            egui::pos2(rect.right() - 12.0, rect.top() + 39.0),
        ),
        egui::Label::new(
            egui::RichText::new(title)
                .size(if compact { 12.0 } else { 15.0 })
                .strong()
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    );
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 58.0, rect.top() + 42.0),
            egui::pos2(rect.right() - 12.0, rect.bottom() - 10.0),
        ),
        egui::Label::new(
            egui::RichText::new(desc)
                .size(if compact { 9.0 } else { 11.0 })
                .color(colors::TEXT_SECONDARY),
        )
        .truncate(),
    );
}
