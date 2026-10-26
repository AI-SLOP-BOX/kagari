use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeType};
use eframe::egui;
use std::collections::HashSet;

fn draw_timeline_header_row(
    ui: &mut egui::Ui,
    id: &'static str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        egui::vec2(width, 28.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            egui::ScrollArea::horizontal()
                .id_salt(id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().interact_size.y = 22.0;
                    ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                    ui.horizontal(add_contents);
                });
        },
    );
    crate::ui::theme::draw_separator(ui);
}

pub struct TimelineHeaderState<'a> {
    pub bottom_dock_tab: &'a mut usize,
    pub show_switches_pane: &'a mut bool,
    pub timeline_zoom: &'a mut f32,
    pub snap_to_keyframes: &'a mut bool,
    pub show_graph_editor: &'a mut bool,
    pub layer_filter_text: &'a mut String,
    pub timeline_view_start: &'a mut u32,
    pub work_area_in: &'a mut Option<u32>,
    pub work_area_out: &'a mut Option<u32>,
    pub expanded_layers: &'a mut HashSet<usize>,
    pub fit_to_selection: &'a mut bool,
    pub fit_all: &'a mut bool,
}

pub fn draw_transport_controls(
    ui: &mut egui::Ui,
    current_frame: &mut u32,
    total_frames: u32,
    fps: u32,
    is_playing: &mut bool,
) {
    ui.spacing_mut().item_spacing.x = 5.0;
    let tc_popup_id = ui.make_persistent_id("ae_tc_goto_popup");
    let mut show_popup = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(tc_popup_id, || false));
    let buffer_id = ui.make_persistent_id("ae_tc_goto_buf");
    let mut input = ui.ctx().data_mut(|data| {
        data.get_temp_mut_or_insert_with(buffer_id, || current_frame.to_string())
            .clone()
    });
    let mut goto_frame = None;
    let fps = fps.max(1);
    let seconds = *current_frame / fps;
    let timecode = format!(
        "{:02}:{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        *current_frame % fps,
    );

    // AE-style Scannable / Scrubbable Timecode indicator
    let tc_text = timecode;
    let (tc_rect, tc_resp) =
        ui.allocate_exact_size(egui::vec2(100.0, 22.0), egui::Sense::click_and_drag());

    if tc_resp.hovered() || tc_resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    if tc_resp.dragged() {
        let speed_mult = if ui.input(|i| i.modifiers.shift) {
            10.0
        } else if ui.input(|i| i.modifiers.alt) {
            0.1
        } else {
            1.0
        };
        let delta = tc_resp.drag_delta().x * speed_mult * 0.5;
        let accum_id = ui.make_persistent_id("ae_tc_scrub_accum");
        let mut accum = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or_insert_with(accum_id, || 0.0_f32));
        accum += delta;
        if accum.abs() >= 1.0 {
            let frames_delta = accum.trunc() as i32;
            accum -= frames_delta as f32;
            let next_f =
                (*current_frame as i32 + frames_delta).clamp(0, total_frames as i32) as u32;
            *current_frame = next_f;
        }
        ui.ctx().data_mut(|d| d.insert_temp(accum_id, accum));
    } else {
        let accum_id = ui.make_persistent_id("ae_tc_scrub_accum");
        ui.ctx().data_mut(|d| d.remove_temp::<f32>(accum_id));
    }

    let bg_color = if tc_resp.dragged() {
        crate::ui::theme::colors::BG_ACTIVE
    } else if tc_resp.hovered() {
        crate::ui::theme::colors::BG_HOVER
    } else {
        crate::ui::theme::colors::BG_SURFACE
    };
    ui.painter().rect_filled(tc_rect, 3.0, bg_color);
    ui.painter().rect_stroke(
        tc_rect,
        3.0,
        egui::Stroke::new(
            1.0_f32,
            if tc_resp.dragged() {
                crate::ui::theme::colors::ACCENT_BRAND
            } else {
                crate::ui::theme::colors::BORDER_SUBTLE
            },
        ),
    );

    let text_col = if tc_resp.dragged() {
        crate::ui::theme::colors::ACCENT_BRAND_HOVER
    } else if tc_resp.hovered() {
        crate::ui::theme::colors::ACCENT_BRAND
    } else {
        crate::ui::theme::colors::ACCENT_YELLOW
    };

    ui.painter().text(
        tc_rect.center(),
        egui::Align2::CENTER_CENTER,
        tc_text,
        egui::FontId::monospace(12.0),
        text_col,
    );

    // A click opens frame entry; a drag scrubs without opening it.
    if tc_resp.clicked() {
        show_popup = true;
        input = current_frame.to_string();
    }
    tc_resp.on_hover_text(
        "ドラッグ: フレームスクラブ (Shift: 10倍速, Alt: 精密)\nクリック: フレーム番号を指定",
    );

    ui.add_sized(
        egui::vec2(120.0, 20.0),
        egui::Slider::new(current_frame, 0..=total_frames.max(1)).show_value(false),
    );
    let buttons = [
        (
            "transport-first-frame",
            crate::ui::icons::SVG_STEP_BACK,
            "Go to First Frame (Home)",
            0,
        ),
        (
            "transport-previous-frame",
            crate::ui::icons::SVG_STEP_BACK,
            "Previous Frame (PageUp / Left)",
            1,
        ),
        (
            "transport-play-pause",
            if *is_playing {
                crate::ui::icons::SVG_PAUSE
            } else {
                crate::ui::icons::SVG_PLAY
            },
            "Play / Pause RAM Preview (Spacebar)",
            2,
        ),
        (
            "transport-next-frame",
            crate::ui::icons::SVG_STEP_FORWARD,
            "Next Frame (PageDown / Right)",
            3,
        ),
        (
            "transport-last-frame",
            crate::ui::icons::SVG_STEP_FORWARD,
            "Go to Last Frame (End)",
            4,
        ),
    ];
    for (id, icon, tooltip, action) in buttons {
        if crate::ui::kui::svg_icon_button(ui, id, icon, tooltip, false, 24.0).clicked() {
            match action {
                0 => *current_frame = 0,
                1 => *current_frame = current_frame.saturating_sub(1),
                2 => *is_playing = !*is_playing,
                3 => *current_frame = (*current_frame + 1).min(total_frames),
                _ => *current_frame = total_frames,
            }
        }
    }
    if show_popup {
        egui::Area::new(egui::Id::new("ae_tc_goto"))
            .fixed_pos(tc_rect.left_bottom() + egui::vec2(0.0, 4.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                crate::ui::kui::group(ui, "Go to Frame", |ui| {
                    ui.set_width(160.0);
                    let response = crate::ui::kui::text_field_with_hint(ui, &mut input, "Frame #");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::ui::kui::button(ui, "Go", crate::ui::kui::ButtonKind::Accent)
                            .clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            goto_frame = input
                                .trim()
                                .parse::<u32>()
                                .ok()
                                .map(|frame| frame.min(total_frames));
                            show_popup = false;
                        }
                        if crate::ui::kui::button(ui, "Cancel", crate::ui::kui::ButtonKind::Quiet)
                            .clicked()
                        {
                            show_popup = false;
                        }
                    });
                });
            });
    }
    if let Some(frame) = goto_frame {
        *current_frame = frame;
    }
    ui.ctx().data_mut(|data| {
        data.insert_temp(tc_popup_id, show_popup);
        data.insert_temp(buffer_id, input);
    });
}

fn draw_view_mode(
    ui: &mut egui::Ui,
    label: &'static str,
    selected: bool,
    tooltip: &'static str,
) -> egui::Response {
    let label = crate::ui::localization::tr(label);
    let response = ui.add(
        egui::Label::new(egui::RichText::new(label).small().color(if selected {
            crate::ui::theme::colors::TEXT_PRIMARY
        } else {
            crate::ui::theme::colors::TEXT_SECONDARY
        }))
        .sense(egui::Sense::click()),
    );
    if selected {
        ui.painter().line_segment(
            [
                egui::pos2(response.rect.left(), response.rect.bottom()),
                egui::pos2(response.rect.right(), response.rect.bottom()),
            ],
            egui::Stroke::new(1.5_f32, crate::ui::theme::colors::ACCENT_ACTION),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(crate::ui::localization::tr(tooltip))
}

pub fn draw_timeline_header(
    state: &mut TimelineHeaderState,
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: &mut u32,
    total_frames: u32,
) -> bool {
    let mut project_changed = false;

    draw_timeline_header_row(ui, "timeline_view_row", |ui| {
        if draw_view_mode(
            ui,
            "タイムライン",
            !*state.show_graph_editor,
            "レイヤーの時間軸",
        )
        .clicked()
        {
            *state.show_graph_editor = false;
        }
        if draw_view_mode(
            ui,
            "グラフ",
            *state.show_graph_editor,
            "グラフエディター／速度カーブ",
        )
        .clicked()
        {
            *state.show_graph_editor = true;
        }
        if draw_view_mode(
            ui,
            "レンダーキュー",
            *state.bottom_dock_tab == 1,
            "レンダーキューを表示",
        )
        .clicked()
        {
            *state.bottom_dock_tab = 1;
        }
        crate::ui::kui::toolbar_separator(ui);
        crate::ui::kui::caption(ui, "ズーム");
        ui.add(
            crate::ui::kui::drag(state.timeline_zoom)
                .speed(0.05)
                .range(0.1..=20.0)
                .width(88.0)
                .unit("×"),
        )
        .on_hover_text("Timeline magnification");
        crate::ui::kui::menu_button(ui, "表示", |ui| {
            if crate::ui::kui::menu_item(ui, "全体を表示")
                .on_hover_text("作業範囲、または全尺に合わせます")
                .clicked()
            {
                let w_in = *state.work_area_in;
                let w_out = *state.work_area_out;
                let span = match (w_in, w_out) {
                    (Some(wi), Some(wo)) => (wo - wi).max(10),
                    (Some(wi), None) => (total_frames - wi).max(10),
                    (None, Some(wo)) => wo.max(10),
                    (None, None) => total_frames,
                };
                *state.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                *state.timeline_view_start = w_in.unwrap_or(0);
            }
            if crate::ui::kui::menu_item(ui, "選択レイヤー")
                .on_hover_text("選択レイヤーの時間範囲に合わせます")
                .clicked()
            {
                *state.fit_to_selection = true;
            }
            if crate::ui::kui::menu_item(ui, "全レイヤー")
                .on_hover_text("全レイヤーの時間範囲に合わせます")
                .clicked()
            {
                *state.fit_to_selection = true;
                *state.fit_all = true;
            }
            if crate::ui::kui::menu_item(ui, "作業範囲を解除")
                .on_hover_text("作業範囲のイン／アウトを解除します")
                .clicked()
            {
                *state.work_area_in = None;
                *state.work_area_out = None;
            }
            if crate::ui::kui::menu_item(ui, "すべて展開")
                .on_hover_text("全レイヤーのプロパティを展開します")
                .clicked()
            {
                for i in 0..comp.layers.len() {
                    state.expanded_layers.insert(i);
                }
            }
            if crate::ui::kui::menu_item(ui, "すべて折りたたむ")
                .on_hover_text("全レイヤーのプロパティを折りたたみます")
                .clicked()
            {
                state.expanded_layers.clear();
            }
        });
        crate::ui::kui::menu_button(ui, "⋯", |ui| {
            let snap_label = if *state.snap_to_keyframes {
                "✓ スナップを有効化"
            } else {
                "　スナップを有効化"
            };
            if crate::ui::kui::menu_item(ui, snap_label)
                .on_hover_text("キーフレームとマーカーへの吸着を切り替えます（Shift+S）")
                .clicked()
            {
                *state.snap_to_keyframes = !*state.snap_to_keyframes;
            }

            crate::ui::kui::separator(ui);
            if crate::ui::kui::menu_item(
                ui,
                if *state.show_switches_pane {
                    "✓ スイッチを表示"
                } else {
                    "　スイッチを表示"
                },
            )
            .clicked()
            {
                *state.show_switches_pane = true;
            }
            if crate::ui::kui::menu_item(
                ui,
                if !*state.show_switches_pane {
                    "✓ モードを表示"
                } else {
                    "　モードを表示"
                },
            )
            .clicked()
            {
                *state.show_switches_pane = false;
            }

            crate::ui::kui::separator(ui);
            let depth = comp.bit_depth;
            let depth_label = format!("色深度: {}", depth.label());
            if crate::ui::kui::menu_item(ui, &depth_label)
                .on_hover_text("色深度を 8 / 16 / 32 bpc で切り替えます")
                .clicked()
            {
                comp.bit_depth = comp.bit_depth.next();
                crate::core::frame_cache::bump_version();
                project_changed = true;
            }

            crate::ui::kui::separator(ui);
            crate::ui::kui::search_field_sized(
                ui,
                state.layer_filter_text,
                "レイヤーを検索…",
                180.0,
            );
        })
        .response
        .on_hover_text("タイムライン表示とレイヤー操作");
        if ui.input(|input| input.key_pressed(egui::Key::F4)) {
            *state.show_switches_pane = !*state.show_switches_pane;
        }

        crate::ui::kui::menu_button(ui, "+", |ui| {
            if crate::ui::kui::menu_item(ui, "+ ソリッド").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Solid {}", comp.layers.len());
                let mut layer = Layer::new(
                    id,
                    name,
                    LayerType::Solid {
                        color: [0.3, 0.5, 0.7, 1.0],
                    },
                    total_frames,
                );
                layer.transform.position =
                    Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ テキスト").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Text {}", comp.layers.len());
                let mut layer = Layer::new(
                    id,
                    name,
                    LayerType::new_text("New Text", 48, [1.0, 1.0, 1.0, 1.0]),
                    total_frames,
                );

                layer.transform.position =
                    Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ 調整レイヤー").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Adjustment Layer {}", comp.layers.len());
                let layer = Layer::new_adjustment(id, name, total_frames);
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }

            if crate::ui::kui::menu_item(ui, "+ マーカー (M)").clicked() {
                let m_idx = comp.markers.len() + 1;
                comp.markers.push(crate::core::timeline::TimelineMarker {
                    frame: *current_frame,
                    label: format!("Marker {}", m_idx),
                    color: [1.0, 0.6, 0.1],
                });
                project_changed = true;
                ui.close_menu();
            }
            if !comp.markers.is_empty()
                && crate::ui::kui::menu_item(ui, "YouTubeチャプターをコピー")
                    .on_hover_text("マーカーからYouTube用のチャプター時刻をコピー")
                    .clicked()
            {
                let fps = (comp.fps as f32).max(1.0);
                let mut sorted = comp.markers.clone();
                sorted.sort_by_key(|m| m.frame);
                let mut lines = Vec::new();
                for m in sorted {
                    let total_secs = (m.frame as f32 / fps).floor() as u32;
                    let mins = total_secs / 60;
                    let secs = total_secs % 60;
                    lines.push(format!("{:02}:{:02} {}", mins, secs, m.label));
                }
                let output = lines.join("\n");
                ui.output_mut(|o| o.copied_text = output);
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ カメラ")
                .on_hover_text("コンポジションに3Dカメラを追加")
                .clicked()
            {
                let camera = crate::core::timeline::Camera3D {
                    id: comp.next_camera_id(),
                    name: format!("Camera {}", comp.cameras.len() + 1),
                    ..Default::default()
                };
                let camera_id = camera.id.clone();
                let camera_transform = camera.transform.clone();
                comp.cameras.push(camera);
                let camera_idx = comp.cameras.len().saturating_sub(1);
                comp.set_active_camera(Some(camera_idx));
                let mut layer = Layer::new(
                    comp.next_layer_id("camera"),
                    format!("Camera {}", comp.cameras.len()),
                    LayerType::Null,
                    total_frames,
                );
                layer.is_3d = true;
                layer.transform_3d = camera_transform;
                layer.scene_object =
                    Some(crate::core::timeline::SceneObjectRef::Camera { id: camera_id });
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ ライト")
                .on_hover_text("コンポジションにポイントライトを追加")
                .clicked()
            {
                let light = crate::core::timeline::Light3D {
                    id: comp.next_light_id(),
                    name: format!("Light {}", comp.lights.len() + 1),
                    ..Default::default()
                };
                let light_id = light.id.clone();
                let light_position = light.position.clone();
                comp.lights.push(light);
                let mut layer = Layer::new(
                    comp.next_layer_id("light"),
                    format!("Light {}", comp.lights.len()),
                    LayerType::Null,
                    total_frames,
                );
                layer.is_3d = true;
                layer.transform_3d.position = light_position;
                layer.scene_object =
                    Some(crate::core::timeline::SceneObjectRef::Light { id: light_id });
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ シェイプ").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Shape {}", comp.layers.len());
                let mut layer = Layer::new(
                    id,
                    name,
                    LayerType::Shape {
                        shape_type: ShapeType::Star {
                            points: Animatable::new_constant(5.0),
                            inner_radius: Animatable::new_constant(40.0),
                            outer_radius: Animatable::new_constant(100.0),
                        },
                        color: [0.9, 0.4, 0.2, 1.0],
                        stroke_color: [0.0, 0.0, 0.0, 1.0],
                        stroke_width: 0.0,
                        fill_type: Default::default(),
                        extrusion_depth: 0.0,
                        bevel_depth: 0.0,
                    },
                    total_frames,
                );
                layer.transform.position =
                    Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ オーディオ").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Audio {}", comp.layers.len());
                let mut layer = Layer::new(
                    id,
                    name,
                    LayerType::Audio {
                        path: "audio_track_01.wav".to_string(),
                        volume: Animatable::new_constant(1.0),
                    },
                    total_frames,
                );
                layer.label = crate::core::timeline::LabelColor::Aqua;
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ パーティクル").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Particles {}", comp.layers.len());
                let mut layer = Layer::new(
                    id,
                    name,
                    LayerType::Particle {
                        emitter: crate::core::particle_system::ParticleEmitter::default(),
                    },
                    total_frames,
                );
                layer.transform.position =
                    Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 * 0.75]);
                layer.label = crate::core::timeline::LabelColor::Yellow;
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
            if crate::ui::kui::menu_item(ui, "+ ヌル").clicked() {
                let id = comp.next_layer_id("layer");
                let name = format!("Null {}", comp.layers.len());
                let layer = Layer::new_null(id, name, total_frames);
                comp.add_layer(layer);
                project_changed = true;
                ui.close_menu();
            }
        })
        .response
        .on_hover_text("レイヤーを追加");
    });

    project_changed
}
