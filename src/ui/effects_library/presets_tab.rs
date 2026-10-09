use crate::core::editor::EditorSession;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_effects_presets_tab(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    next_frame: &mut Option<u32>,
    slider_changed: &mut bool,
) {
    let presets = crate::ui::effects_controls::get_all_effect_presets();
    let search_q = app.ui_tabs.effects_search_query.to_lowercase();

    fn category_of(name: &str) -> &'static str {
        if name.contains("Key")
            || name.contains("Matte")
            || name.contains("Choker")
            || name.contains("Choke")
            || name.contains("Minimax")
            || name.contains("Alpha")
        {
            "キーイング・マット"
        } else if name.contains("Blur")
            || name.contains("Sharpen")
            || name.contains("Median")
            || name.contains("Tilt")
        {
            "ぼかし・シャープ"
        } else if name.contains("Tint")
            || name.contains("Hue")
            || name.contains("LUT")
            || name.contains("Levels")
            || name.contains("Log Space")
            || name.contains("Balance")
            || name.contains("Vibrance")
            || name.contains("HSL")
            || name.contains("Curve")
            || name.contains("Channel")
            || name.contains("Colorama")
            || name.contains("Color Space")
            || name.contains("Gradient")
        {
            "カラー補正"
        } else if name.contains("Warp")
            || name.contains("Bulge")
            || name.contains("Twirl")
            || name.contains("Offset")
            || name.contains("Distort")
            || name.contains("Ripple")
            || name.contains("Spherize")
            || name.contains("Displace")
        {
            "変形"
        } else if name.contains("Shadow")
            || name.contains("Glow")
            || name.contains("Grain")
            || name.contains("Vignette")
            || name.contains("Aberration")
            || name.contains("Scanline")
            || name.contains("CRT")
            || name.contains("Posterize")
            || name.contains("Invert")
            || name.contains("Threshold")
            || name.contains("Light Sweep")
            || name.contains("Night")
            || name.contains("Vision")
            || name.contains("Halftone")
            || name.contains("Solarize")
            || name.contains("Pixel")
            || name.contains("Emboss")
        {
            "スタイライズ"
        } else if name.contains("Noise")
            || name.contains("Fractal")
            || name.contains("Star")
            || name.contains("Lightning")
            || name.contains("Fire")
            || name.contains("Reflection")
            || name.contains("Perlin")
        {
            "生成・シミュレーション"
        } else if name.contains("Wipe") {
            "トランジション"
        } else {
            "その他"
        }
    }

    let layer_idx = app.selection.selected_layer_idx;

    if layer_idx.is_none() {
        ui.colored_label(
            colors::ACCENT_ORANGE,
            "レイヤーを作成すると、エフェクトを適用できます。",
        );
    }

    egui::CollapsingHeader::new("エフェクト一覧")
        .id_salt(("effects_browser", !search_q.is_empty()))
        .default_open(true)
        .show(ui, |ui| {
        let categories = [
            "ぼかし・シャープ",
            "カラー補正",
            "変形",
            "スタイライズ",
            "キーイング・マット",
            "生成・シミュレーション",
            "トランジション",
            "その他",
        ];
        for cat in categories {
            let matching: Vec<_> = presets
                .iter()
                .filter(|p| category_of(p.name) == cat)
                .filter(|p| {
                    search_q.is_empty()
                        || p.search_key.contains(&search_q)
                        || p.name.to_lowercase().contains(&search_q)
                })
                .collect();
            if matching.is_empty() {
                continue;
            }
            egui::CollapsingHeader::new(format!("{} ({})", cat, matching.len()))
                .id_salt(("effect_category", cat, !search_q.is_empty()))
                .default_open(!search_q.is_empty())
                .show(ui, |ui| {
                for (pi, p) in matching.iter().enumerate() {
                    let preset_idx = presets
                        .iter()
                        .position(|pp| std::ptr::eq(pp, *p))
                        .unwrap_or(pi);
                    let row_width = ui.available_width();
                    let resp = ui.add_sized(
                        [row_width, 22.0],
                        egui::Button::new(
                            egui::RichText::new(format!("{}  {}", "•", p.button_label))
                                .small()
                                .color(colors::TEXT_PRIMARY),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE)
                        .rounding(egui::Rounding::same(2.0)),
                    );
                    if resp.hovered() {
                        ui.painter().rect_filled(resp.rect, 2.0, colors::BG_HOVER);
                    }

                    if resp.drag_started() {
                        app.dragging_effect = Some((p.name.to_string(), preset_idx));
                    }
                    if resp.drag_stopped() {
                        app.dragging_effect = None;
                    }

                    // Click to apply effect — discrete action via EditorSession
                    if resp.clicked() {
                        if let Some(idx) = layer_idx {
                            let mut session = EditorSession::new(&mut app.history, "Apply Effect");
                            let comp = session.current_mut().active_composition_mut();
                            if idx < comp.layers.len() {
                                let effect = (p.create_fn)(comp.layers[idx].effects.len());
                                comp.layers[idx].effects.push(effect);
                                session.commit();
                            }
                        } else {
                            app.toasts.info("Select a layer first, or drag the preset onto the timeline");
                        }
                    }

                    if resp.dragged() {
                        ui.painter()
                            .rect_filled(resp.rect, 2.0, colors::TIMELINE_SELECTION);
                    }
                    resp.on_hover_text("クリックで適用・タイムラインへドラッグで追加");
                }
                });
        }
        });

    if layer_idx.is_some() {
        ui.label("選択レイヤーにエフェクトを追加:");
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(0.0, 4.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("モーションVFXプリセット")
                        .strong()
                        .color(colors::ACCENT_CYAN),
                );
                ui.small("プリセットをすばやく追加:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.ui_tabs.effects_search_query)
                            .hint_text("例: サイバーパンク・ネオングロー"),
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button("Cyberpunk").clicked() {
                        if let Some(idx) = layer_idx {
                            let mut session =
                                EditorSession::new(&mut app.history, "Add Cyberpunk Neon");
                            let comp = session.current_mut().active_composition_mut();
                            if idx < comp.layers.len() {
                                let len = comp.layers[idx].effects.len();
                                comp.layers[idx]
                                .effects
                                .push(crate::core::timeline::Effect {
                                    id: format!("ai_glow_{}", len),
                                    name: "Cyberpunk Neon".to_string(),
                                    effect_type: crate::core::timeline::EffectType::Glow {
                                        threshold: crate::core::property::Animatable::new_constant(
                                            0.2,
                                        ),
                                        radius: crate::core::property::Animatable::new_constant(
                                            30.0,
                                        ),
                                        intensity: crate::core::property::Animatable::new_constant(
                                            3.0,
                                        ),
                                        color: crate::core::property::Animatable::new_constant([
                                            0.0, 0.9, 1.0, 1.0,
                                        ]),
                                    },
                                    enabled: true,
                                });
                                session.commit();
                            }
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                    }
                    if ui.button("Motion Burn").clicked() {
                        if let Some(idx) = layer_idx {
                            let mut session =
                                EditorSession::new(&mut app.history, "Add Motion Burn");
                            let comp = session.current_mut().active_composition_mut();
                            if idx < comp.layers.len() {
                                let len = comp.layers[idx].effects.len();
                                comp.layers[idx]
                                .effects
                                .push(crate::core::timeline::Effect {
                                    id: format!("ai_burn_{}", len),
                                    name: "Motion Burn".to_string(),
                                    effect_type: crate::core::timeline::EffectType::Glow {
                                        threshold: crate::core::property::Animatable::new_constant(
                                            0.1,
                                        ),
                                        radius: crate::core::property::Animatable::new_constant(
                                            45.0,
                                        ),
                                        intensity: crate::core::property::Animatable::new_constant(
                                            4.0,
                                        ),
                                        color: crate::core::property::Animatable::new_constant([
                                            1.0, 0.3, 0.0, 1.0,
                                        ]),
                                    },
                                    enabled: true,
                                });
                                session.commit();
                            }
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                    }
                });
            });
    }
    ui.add_space(4.0);

    // Applied effects list
    ui.separator();
    if let Some(idx) = layer_idx {
        let effect_count = app
            .history
            .current()
            .active_composition()
            .layers
            .get(idx)
            .map_or(0, |l| l.effects.len());
        if effect_count > 0 {
            ui.label("Applied Effects:");

            let mut effect_to_remove: Option<usize> = None;
            let mut effect_to_swap: Option<(usize, usize)> = None;
            let mut effect_toggle: Option<(usize, bool)> = None;

            // Render with current_mut for combo boxes that need &mut access.
            // Use block scope so mutable borrow is released before EditorSession.
            {
                let temp_project = app.history.current_mut();
                let comp = temp_project.active_composition_mut();

                if idx < comp.layers.len() {
                    let layer = &mut comp.layers[idx];
                    let effects_count = layer.effects.len();

                    for (e_idx, effect) in layer.effects.iter_mut().enumerate() {
                        let fx_persistent_id =
                            ui.make_persistent_id(format!("ae_fx_item_{}_{}", effect.id, e_idx));
                        ui.push_id(fx_persistent_id, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("[X]")
                                    .on_hover_text("Delete Effect")
                                    .clicked()
                                {
                                    effect_to_remove = Some(e_idx);
                                }
                                if e_idx > 0
                                    && ui.small_button("▲").on_hover_text("Move Up").clicked()
                                {
                                    effect_to_swap = Some((e_idx, e_idx - 1));
                                }
                                if e_idx + 1 < effects_count
                                    && ui.small_button("▼").on_hover_text("Move Down").clicked()
                                {
                                    effect_to_swap = Some((e_idx, e_idx + 1));
                                }
                                let fx_label = if effect.enabled { "[fx]" } else { "[fx off]" };
                                if ui
                                    .selectable_label(effect.enabled, fx_label)
                                    .on_hover_text("Toggle Effect Bypass (ON/OFF)")
                                    .clicked()
                                {
                                    effect.enabled = !effect.enabled;
                                    effect_toggle = Some((e_idx, effect.enabled));
                                }
                            });
                            ui.collapsing(&effect.name, |ui| {
                                if let crate::core::timeline::EffectType::LensFlare {
                                    link_to_light,
                                    ..
                                } = &mut effect.effect_type
                                {
                                    let light_names: Vec<String> =
                                        comp.lights.iter().map(|l| l.name.clone()).collect();
                                    let current = link_to_light
                                        .clone()
                                        .unwrap_or_else(|| "(manual position)".into());
                                    egui::ComboBox::from_id_salt((
                                        "flare_link2",
                                        effect.id.as_str(),
                                    ))
                                    .selected_text(current)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                link_to_light.is_none(),
                                                "(manual position)",
                                            )
                                            .clicked()
                                        {
                                            *link_to_light = None;
                                        }
                                        for ln in &light_names {
                                            if ui
                                                .selectable_label(
                                                    link_to_light.as_deref() == Some(ln.as_str()),
                                                    ln,
                                                )
                                                .clicked()
                                            {
                                                *link_to_light = Some(ln.clone());
                                            }
                                        }
                                    });
                                }
                                crate::ui::effects_controls::draw_effect_type_ui(
                                    &mut effect.effect_type,
                                    &effect.id,
                                    ui,
                                    current_frame,
                                    slider_changed,
                                    next_frame,
                                );
                            });
                        });
                    }
                }
            } // <-- mutable borrow released here

            // Apply discrete actions via EditorSession
            if let Some(r_idx) = effect_to_remove {
                let mut session = EditorSession::new(&mut app.history, "Delete Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    comp.layers[idx].effects.remove(r_idx);
                }
                session.commit();
            }
            if let Some((a, b)) = effect_to_swap {
                let mut session = EditorSession::new(&mut app.history, "Reorder Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    comp.layers[idx].effects.swap(a, b);
                }
                session.commit();
            }
            if let Some((e_idx, new_enabled)) = effect_toggle {
                let mut session = EditorSession::new(&mut app.history, "Toggle Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() && e_idx < comp.layers[idx].effects.len() {
                    comp.layers[idx].effects[e_idx].enabled = new_enabled;
                }
                session.commit();
            }
        }
    }
}
