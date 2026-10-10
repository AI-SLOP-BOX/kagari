use super::character_tab::draw_character_tab;
use super::effect_controls::draw_effect_controls;
use super::presets_tab::draw_effects_presets_tab;
use crate::core::editor::EditorSession;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32) {
    let dt = ctx.input(|i| i.stable_dt);
    app.effects_animation.update(dt);

    let reference_demo = ctx.screen_rect().width() >= 1200.0;
    let narrow_right_dock =
        ctx.screen_rect().width() < crate::ui::theme::layout::DOCKED_PANELS_MIN_WIDTH;
    let compact_inspector_drawer = ctx
        .data(|data| data.get_temp::<bool>(egui::Id::new("compact_inspector_drawer")))
        .unwrap_or(false);
    if narrow_right_dock && !compact_inspector_drawer {
        return;
    }
    let screen_width = ctx.screen_rect().width();
    let right_width = if reference_demo {
        (screen_width * 0.235).clamp(300.0, 456.0)
    } else if narrow_right_dock {
        286.0
    } else {
        (screen_width * 0.20).clamp(190.0, 300.0)
    };
    let max_width = if reference_demo {
        359.0
    } else if ctx.screen_rect().width() >= 1200.0 {
        360.0
    } else if narrow_right_dock {
        300.0
    } else {
        (ctx.screen_rect().width() * 0.23).max(220.0)
    };

    egui::SidePanel::right("right_panel_editor_v2")
        .resizable(true)
        .default_width(right_width)
        .min_width(if reference_demo {
            (screen_width * 0.18).clamp(260.0, 350.0)
        } else if narrow_right_dock {
            250.0
        } else {
            (screen_width * 0.18).clamp(170.0, 230.0)
        })
        .max_width(if reference_demo {
            (screen_width * 0.28).clamp(360.0, 544.0)
        } else if narrow_right_dock {
            300.0
        } else {
            max_width.max((screen_width * 0.22).clamp(210.0, 300.0))
        })
        .frame(if reference_demo {
            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DARKEST)
                .inner_margin(egui::Margin::symmetric(21.0, 0.0))
        } else {
            egui::Frame::default().inner_margin(egui::Margin::symmetric(8.0, 0.0))
        })
        .show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(
                    ui.available_width(),
                    crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
                ),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let labels = [
                        (crate::app_state::RightPanel::Properties, "プロパティ"),
                        (crate::app_state::RightPanel::Effects, "エフェクト"),
                        (crate::app_state::RightPanel::Color, "カラー"),
                    ];
                    for (panel, label) in labels {
                        if crate::ui::theme::draw_custom_tab(
                            ui,
                            app.ui_tabs.right_panel == panel,
                            label,
                        )
                        .clicked()
                        {
                            app.ui_tabs.right_panel = panel;
                        }
                    }
                    ui.menu_button("…", |ui| {
                        let more_panels = [
                            (
                                crate::app_state::RightPanel::Tracker,
                                "トラッカー (Tracker)",
                            ),
                            (crate::app_state::RightPanel::Audio, "オーディオ (Audio)"),
                            (crate::app_state::RightPanel::Character, "文字 (Character)"),
                            (crate::app_state::RightPanel::Paragraph, "段落 (Paragraph)"),
                            (crate::app_state::RightPanel::Align, "整列 (Align)"),
                            (
                                crate::app_state::RightPanel::RenderPresets,
                                "書き出しプリセット",
                            ),
                            (
                                crate::app_state::RightPanel::LayerStyles,
                                "レイヤースタイル",
                            ),
                            (
                                crate::app_state::RightPanel::EssentialGraphics,
                                "エッセンシャルグラフィックス",
                            ),
                            (
                                crate::app_state::RightPanel::ContentAwareFill,
                                "コンテンツに応じた塗りつぶし",
                            ),
                            (crate::app_state::RightPanel::Info, "情報 (Info)"),
                        ];
                        for (panel, title) in more_panels {
                            if ui
                                .selectable_label(app.ui_tabs.right_panel == panel, title)
                                .clicked()
                            {
                                app.ui_tabs.right_panel = panel;
                                ui.close_menu();
                            }
                        }
                    });
                },
            );
            crate::ui::theme::draw_separator(ui);

            egui::ScrollArea::vertical()
                .id_salt("panel_content")
                .show(ui, |ui| {
                    let mut next_frame = None;
                    let mut current_frame_reset = None;
                    let edit_input = ui.input(|input| {
                        input.pointer.any_pressed()
                            || input.pointer.any_down()
                            || input.pointer.any_released()
                            || input.events.iter().any(|event| {
                                matches!(
                                    event,
                                    egui::Event::Text(_)
                                        | egui::Event::Key { pressed: true, .. }
                                        | egui::Event::MouseWheel { .. }
                                )
                            })
                    });
                    let pre_edit_snapshot = if !app.drag_active() && edit_input {
                        Some(app.history.current().clone())
                    } else {
                        None
                    };
                    let history_generation_before = app.history.generation();

                    // Track whether a continuous interaction (slider drag) occurred.
                    // Discrete actions are committed immediately via EditorSession.
                    let mut slider_changed = false;

                    if app.ui_tabs.right_panel == crate::app_state::RightPanel::Tracker {
                        crate::ui::tracker_panel::draw_tracker_panel(app, ui, *current_frame);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Transport {
                        let total_frames =
                            app.history.current().active_composition().duration_frames;
                        crate::ui::transport_panel::draw_transport_panel(
                            app,
                            ui,
                            current_frame,
                            total_frames,
                        );
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Properties {
                        draw_effect_controls(
                            app,
                            ui,
                            *current_frame,
                            &mut next_frame,
                            &mut slider_changed,
                        );
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Paint {
                        crate::ui::paint_panel::draw_paint_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Markers {
                        crate::ui::marker_panel::draw_marker_panel(app, ui, *current_frame);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Audio {
                        crate::ui::audio_meter::draw_content(app, ui);
                        ui.add_space(8.0);
                        crate::ui::audio_panel::draw_audio_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::TimeRemap {
                        crate::ui::time_remap_panel::draw_time_remap_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Masks {
                        crate::ui::mask_panel::draw_mask_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Expressions {
                        crate::ui::expression_panel::draw_expression_panel(app, ui);
                    } else if app.ui_tabs.right_panel
                        == crate::app_state::RightPanel::EssentialGraphics
                    {
                        crate::ui::essential_graphics::draw_essential_graphics(app, ui);
                    } else if app.ui_tabs.right_panel
                        == crate::app_state::RightPanel::ContentAwareFill
                    {
                        crate::ui::content_aware_fill::draw_content_aware_fill(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Metadata {
                        crate::ui::metadata_panel::draw_metadata_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Scripting {
                        crate::ui::scripting_console::draw_scripting_console(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Workspaces {
                        crate::ui::workspace_manager::draw_workspace_manager(app, ui);
                    } else if app.ui_tabs.right_panel
                        == crate::app_state::RightPanel::ColorManagement
                    {
                        crate::ui::color_management::draw_color_management(app, ui);
                    } else if app.ui_tabs.right_panel
                        == crate::app_state::RightPanel::FlowchartInspector
                    {
                        crate::ui::flowchart_inspector::draw_flowchart_inspector(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::CameraViews {
                        crate::ui::camera_views::draw_camera_views(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Color {
                        crate::ui::lumetri_color::draw_lumetri_color(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Libraries {
                        crate::ui::cc_libraries::draw_cc_libraries(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Fonts {
                        crate::ui::font_picker::draw_font_picker(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::RenderPresets
                    {
                        crate::ui::render_presets::draw_render_presets(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::AudioMixer {
                        crate::ui::audio_mixer::draw_audio_mixer(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::SpeedGraph {
                        let cf = app.playback.current_frame;
                        crate::ui::speed_graph_options::draw_speed_graph_options(app, ui, cf);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::CameraLight {
                        crate::ui::camera_light_options::draw_camera_light_options(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::LayerStyles {
                        crate::ui::layer_styles::draw_layer_styles(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Character {
                        draw_character_tab(app, ui, *current_frame);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Paragraph {
                        crate::ui::paragraph_panel::draw_paragraph_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Align {
                        crate::ui::align_panel::draw_align_panel(app, ui);
                    } else if app.ui_tabs.right_panel == crate::app_state::RightPanel::Info {
                        ui.heading("Info");
                        ui.separator();
                        if let Some(idx) = app.selection.selected_layer_idx {
                            let comp = app.history.current().active_composition();
                            if idx < comp.layers.len() {
                                let layer = &comp.layers[idx];
                                ui.label(format!("Layer: {}", layer.name));
                                ui.label(format!("ID: {}", layer.id));
                                let pos = layer.transform.position.evaluate(*current_frame);
                                let scale = layer.transform.scale.evaluate(*current_frame);
                                let rot = layer.transform.rotation.evaluate(*current_frame);
                                let op = layer.transform.opacity.evaluate(*current_frame);
                                ui.weak(format!("Position: ({:.1}, {:.1})", pos[0], pos[1]));
                                ui.weak(format!("Scale: ({:.1}%, {:.1}%)", scale[0], scale[1]));
                                ui.weak(format!("Rotation: {:.1}°", rot));
                                ui.weak(format!("Opacity: {:.1}%", op));
                            }
                        } else {
                            ui.weak("レイヤーが選択されていません");
                        }
                    } else {
                        draw_effects_presets_tab(
                            app,
                            ui,
                            *current_frame,
                            &mut next_frame,
                            &mut slider_changed,
                        );
                    }

                    ui.separator();
                    ui.collapsing("外部エディター連携", |ui| {
                        ui.add_space(4.0);

                        if let Some(app_name) = &app.connected_app {
                            ui.horizontal(|ui| {
                                ui.label("状態:");
                                ui.colored_label(
                                    colors::ACCENT_GREEN,
                                    format!("接続中: {}", app_name),
                                );
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("状態:");
                                ui.colored_label(colors::ACCENT_RED, "待機中: 127.0.0.1:9000");
                            });
                        }
                        ui.add_space(8.0);

                        ui.label("OTIOファイルのパス:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut app.otio_path);
                        });

                        ui.horizontal(|ui| {
                            if ui.button("OTIOを読み込む").clicked() {
                                if let Ok(json_str) =
                                    crate::core::project_migration::read_bounded_text_file(
                                        std::path::Path::new(&app.otio_path),
                                        crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
                                    )
                                {
                                    if let Ok(otio_timeline) =
                                        crate::core::integration::OtioTimeline::from_json(&json_str)
                                    {
                                        match otio_timeline.to_composition() {
                                            Ok(new_comp) => {
                                                let mut session = EditorSession::new(
                                                    &mut app.history,
                                                    "Import OTIO",
                                                );
                                                let comp =
                                                    session.current_mut().active_composition_mut();
                                                comp.name = new_comp.name;
                                                comp.width = new_comp.width;
                                                comp.height = new_comp.height;
                                                comp.fps = new_comp.fps;
                                                comp.duration_frames = new_comp.duration_frames;
                                                comp.layers = new_comp.layers;
                                                session.commit();
                                                current_frame_reset = Some(0);
                                                log::info!(
                                                    "Successfully imported OTIO composition"
                                                );
                                            }
                                            Err(error) => {
                                                log::error!(
                                                    "Failed to convert OTIO timeline: {error}"
                                                );
                                            }
                                        }
                                    } else {
                                        log::error!("Failed to parse OTIO JSON");
                                    }
                                } else {
                                    log::error!(
                                        "Failed to read OTIO file from path: {}",
                                        app.otio_path
                                    );
                                }
                            }
                            if ui.button("Export OTIO").clicked() {
                                let active_comp = app.history.current().active_composition();
                                let otio_timeline =
                                    crate::core::integration::OtioTimeline::from_composition(
                                        active_comp,
                                    );
                                if let Ok(json_str) = otio_timeline.to_json() {
                                    if std::fs::write(&app.otio_path, json_str).is_ok() {
                                        log::info!(
                                            "Successfully exported OTIO composition to: {}",
                                            app.otio_path
                                        );
                                    } else {
                                        log::error!(
                                            "Failed to write OTIO file to path: {}",
                                            app.otio_path
                                        );
                                    }
                                }
                            }
                        });
                    });

                    // Continuous interaction commit: slider drags and other live parameter
                    // edits go through the drag transaction API (begin on drag start,
                    // commit on drag end). This produces exactly one undo entry per drag
                    // regardless of how many intermediate value updates occur.
                    if slider_changed && app.history.generation() == history_generation_before {
                        let is_pointer_down = ui.input(|i| i.pointer.any_down());
                        if is_pointer_down {
                            if !app.drag_active() {
                                if let Some(snapshot) = pre_edit_snapshot {
                                    app.begin_drag_with_snapshot(snapshot, "Effect Parameter Edit");
                                }
                            }
                        } else if app.drag_active() {
                            app.commit_drag();
                        }
                    }
                    if let Some(nf) = next_frame {
                        *current_frame = nf;
                    }
                    if let Some(cf) = current_frame_reset {
                        app.playback.current_frame = cf;
                        *current_frame = cf;
                    }

                    crate::ui::effects_controls::draw_particle_emitter_controls(app, ui);
                });
        });
}
