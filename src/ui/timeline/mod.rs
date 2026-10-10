pub mod breadcrumb;
pub mod geometry;
pub mod header;
mod keyframe_menu;
pub mod keyframe_rows;
pub mod keyframe_tick;
pub mod layers;
pub mod layout;
pub mod pending_actions;
pub mod precomp_children;
pub mod ruler;
pub mod ruler_bar;
pub mod snapping;
mod state;
pub mod utils;
mod waveform_cache;

use crate::core::timeline::{BlendMode, LabelColor, LayerType, TrackMatteMode};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use header::draw_timeline_header;
use snapping::maybe_snap_frame;
use state::{timeline_derived_data, would_create_parent_cycle, LayerBodyDragState};

const TIMELINE_THUMBNAIL_WIDTH: f32 = 54.0;
const MAX_TIMELINE_THUMBNAILS_PER_CLIP: usize = 12;
const MAX_TIMELINE_THUMBNAIL_TEXTURES: usize = 256;

fn layer_has_timeline_preview(layer_type: &LayerType) -> bool {
    matches!(
        layer_type,
        LayerType::Video { .. } | LayerType::Image { .. }
    )
}

struct TimelineThumbnailRequest {
    key: String,
    path: std::path::PathBuf,
    target_dim: u32,
}

type ThumbnailResultReceiver =
    std::sync::mpsc::Receiver<(String, Option<(usize, usize, Vec<u8>)>)>;

struct TimelineThumbnailCache {
    version: u64,
    textures: std::collections::HashMap<String, egui::TextureHandle>,
    order: std::collections::VecDeque<String>,
    requested: std::collections::HashSet<String>,
    failed: std::collections::HashSet<String>,
    request_tx: std::sync::mpsc::SyncSender<TimelineThumbnailRequest>,
    result_rx: ThumbnailResultReceiver,
}

fn timeline_layer_preview_texture(
    ctx: &egui::Context,
    composition: &crate::core::timeline::Composition,
    layer: &crate::core::timeline::Layer,
    timeline_frame: u32,
) -> Option<egui::TextureId> {
    let version = crate::core::frame_cache::current_version();
    let (path, source_frame) = match &layer.layer_type {
        LayerType::Image { path } => (std::path::PathBuf::from(path), Some(0)),
        LayerType::Video {
            frames_dir,
            frame_count,
            speed,
            ..
        } => {
            let source_position = (match &layer.posterize_time {
                Some(settings) if settings.enabled => {
                    layer.effective_render_frame(timeline_frame, composition.fps) as f32
                }
                _ => layer.remap_frame_f32(timeline_frame),
            } * speed.max(0.0))
            .max(0.0);
            let source_frame = (source_position.floor() as u32)
                .min(frame_count.saturating_sub(1));
            (
                crate::core::video_import::frame_path_in_dir(frames_dir, source_frame),
                Some(source_frame),
            )
        }
        _ => return None,
    };
    let key = format!(
        "{}:{}:{}:{}:{}",
        version,
        composition.id,
        layer.id,
        path.display(),
        source_frame.unwrap_or(timeline_frame)
    );
    let cache_id = egui::Id::new("timeline_media_thumbnail_cache_v2");
    let cache = ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with(cache_id, || {
            let (request_tx, request_rx) =
                std::sync::mpsc::sync_channel::<TimelineThumbnailRequest>(8);
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let preview_ctx = ctx.clone();
            std::thread::Builder::new()
                .name("timeline-media-thumbnails".into())
                .spawn(move || {
                    while let Ok(request) = request_rx.recv() {
                        let decoded = image::open(&request.path).ok().map(|image| {
                            let (orig_w, orig_h) = (image.width(), image.height());
                            let (target_w, target_h) = if orig_w == 0 || orig_h == 0 {
                                (request.target_dim, request.target_dim)
                            } else if orig_w >= orig_h {
                                let w = request.target_dim;
                                let h = ((request.target_dim as f32 * orig_h as f32) / orig_w as f32).round().max(1.0) as u32;
                                (w, h)
                            } else {
                                let h = request.target_dim;
                                let w = ((request.target_dim as f32 * orig_w as f32) / orig_h as f32).round().max(1.0) as u32;
                                (w, h)
                            };
                            let thumbnail = image::imageops::resize(
                                &image.to_rgba8(),
                                target_w,
                                target_h,
                                image::imageops::FilterType::Triangle,
                            );
                            (
                                target_w as usize,
                                target_h as usize,
                                thumbnail.into_raw(),
                            )
                        });
                        if result_tx.send((request.key, decoded)).is_err() {
                            break;
                        }
                        preview_ctx.request_repaint_after(std::time::Duration::from_millis(16));
                    }
                })
            .ok();
            std::sync::Arc::new(std::sync::Mutex::new(TimelineThumbnailCache {
                version,
                textures: std::collections::HashMap::new(),
                order: std::collections::VecDeque::new(),
                requested: std::collections::HashSet::new(),
                failed: std::collections::HashSet::new(),
                request_tx,
                result_rx,
            }))
        })
        .clone()
    });
    let Ok(mut cache) = cache.lock() else {
        return None;
    };
    if cache.version != version {
        cache.version = version;
        cache.textures.clear();
        cache.order.clear();
        cache.requested.clear();
        cache.failed.clear();
    }
    while let Ok((completed_key, decoded)) = cache.result_rx.try_recv() {
        cache.requested.remove(&completed_key);
        if !completed_key.starts_with(&format!("{version}:")) {
            continue;
        }
        match decoded {
            Some((width, height, pixels)) if width > 0 && height > 0 => {
                while cache.textures.len() >= MAX_TIMELINE_THUMBNAIL_TEXTURES {
                    if let Some(oldest_key) = cache.order.pop_front() {
                        cache.textures.remove(&oldest_key);
                    } else {
                        break;
                    }
                }
                let texture = ctx.load_texture(
                    format!("timeline-media-thumbnail:{completed_key}"),
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels),
                    egui::TextureOptions::LINEAR,
                );
                cache.textures.insert(completed_key.clone(), texture);
                cache.order.push_back(completed_key);
            }
            Some(_) => {}
            None => {
                cache.failed.insert(completed_key);
            }
        }
    }
    if let Some(texture) = cache.textures.get(&key) {
        return Some(texture.id());
    }
    if (cache.textures.len() + cache.requested.len()) < MAX_TIMELINE_THUMBNAIL_TEXTURES * 2
        && !cache.failed.contains(&key)
        && !cache.requested.contains(&key)
    {
        let request = TimelineThumbnailRequest {
            key: key.clone(),
            path,
            target_dim: 96,
        };
        if cache.request_tx.try_send(request).is_ok() {
            cache.requested.insert(key);
        }
    }
    None
}

struct LayerWaveformParams<'a> {
    timeline_rect: egui::Rect,
    clip_rect: egui::Rect,
    layer: &'a crate::core::timeline::Layer,
    waveform: &'a waveform_cache::WaveformData,
    start_frame: u32,
    visible_span: u32,
    fps: u32,
    color: egui::Color32,
}

fn draw_layer_waveform(
    painter: &egui::Painter,
    params: LayerWaveformParams<'_>,
) {
    let LayerWaveformParams {
        timeline_rect,
        clip_rect,
        layer,
        waveform,
        start_frame,
        visible_span,
        fps,
        color,
    } = params;

    if waveform.peaks.len() < 2
        || waveform.duration_seconds <= 0.0
        || timeline_rect.width() <= 8.0
        || clip_rect.width() <= 0.0
    {
        return;
    }
    let fps = fps.max(1) as f32;
    let amp = clip_rect.height() * 0.40;
    let mut x = clip_rect.left() + 2.0;
    while x <= clip_rect.right() - 2.0 {
        let t = (x - timeline_rect.left()) / timeline_rect.width();
        let comp_frame = start_frame as f32 + t * visible_span.max(1) as f32;
        let source_frame = match &layer.layer_type {
            LayerType::Video { speed, .. } => {
                layer.remap_frame_f32(comp_frame.round() as u32) * speed.max(0.0)
            }
            _ => comp_frame - layer.in_frame as f32,
        };
        let seconds = source_frame / fps;
        if seconds >= 0.0 && seconds < waveform.duration_seconds {
            let index =
                ((seconds / waveform.duration_seconds) * waveform.peaks.len() as f32) as usize;
            let peak = waveform.peaks[index.min(waveform.peaks.len() - 1)];
            let height = (peak * amp).clamp(1.0, amp.max(1.0));
            let middle = clip_rect.center().y;
            painter.line_segment(
                [
                    egui::pos2(x, middle - height),
                    egui::pos2(x, middle + height),
                ],
                egui::Stroke::new(1.0_f32, color),
            );
        }
        x += 2.0;
    }
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    let screen_height = ctx.screen_rect().height();
    let narrow_timeline = ctx.screen_rect().width() < 950.0;
    // Keep the timeline large enough to show several tracks without forcing the
    // user to resize the dock on every launch.
    let (timeline_default_height, timeline_min_height, viewer_min_height) = if narrow_timeline {
        (
            (screen_height * 0.30).clamp(160.0, 250.0),
            (screen_height * 0.15).clamp(80.0, 140.0),
            (screen_height * 0.35).clamp(180.0, 320.0),
        )
    } else {
        (
            (screen_height * 0.38).clamp(200.0, 340.0),
            (screen_height * 0.15).clamp(90.0, 150.0),
            (screen_height * 0.40).clamp(200.0, 420.0),
        )
    };
    let timeline_max_height =
        (screen_height * 0.94).min((screen_height - viewer_min_height).max(timeline_min_height));
    egui::TopBottomPanel::bottom("timeline_panel_editor_v12")
        .resizable(true)
        .default_height(timeline_default_height)
        .min_height(timeline_min_height)
        .max_height(timeline_max_height)
        .show(ctx, |ui| {
            if app.ui_tabs.bottom_dock_tab != 1
                && app.history.current().has_pristine_start_composition()
            {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), crate::ui::theme::layout::PANEL_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| { crate::ui::kui::title(ui, "タイムライン"); },
                );
                crate::ui::theme::draw_separator(ui);
                let empty_height = ui.available_height().max(56.0);
                crate::ui::kui::empty_state(
                    ui,
                    "コンポジションを作成するか、素材を追加するとここに表示されます",
                    empty_height,
                );
                return;
            }

            if app.ui_tabs.bottom_dock_tab == 1 {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), crate::ui::theme::layout::PANEL_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if crate::ui::theme::draw_custom_tab(ui, false, "タイムライン").clicked() {
                            app.ui_tabs.bottom_dock_tab = 0;
                        }
                        let _ = crate::ui::theme::draw_custom_tab(ui, true, "レンダーキュー");
                    },
                );
                crate::ui::theme::draw_separator(ui);
                egui::ScrollArea::vertical()
                    .id_salt("render_queue_content")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        crate::ui::render_queue::draw_render_queue_panel(app, ui);
                    });
                return;
            }

            let mut project_changed = false;
            // A full project clone is only needed while a pointer gesture can
            // mutate the timeline. Cloning merely because Graph Editor is
            // visible made an idle editor pay the snapshot cost every frame.
            let pointer_gesture = ui.input(|i| {
                i.pointer.any_pressed() || i.pointer.any_down() || i.pointer.any_released()
            });
            let pre_edit_snapshot = if !app.drag_active() && pointer_gesture {
                Some(app.history.current().clone())
            } else {
                None
            };
            let _compact_mode = ui.ctx().data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new("ae_compact_timeline"), || false)
            });
            let mut actions = pending_actions::TimelineActions::default();

            crate::ui::timeline::breadcrumb::draw_comp_breadcrumb(app, ui);

            let mut header_state = header::TimelineHeaderState {
                bottom_dock_tab: &mut app.ui_tabs.bottom_dock_tab,
                show_switches_pane: &mut app.ui_tabs.show_switches_pane,
                timeline_zoom: &mut app.timeline_zoom,
                snap_to_keyframes: &mut app.snap_to_keyframes,
                show_graph_editor: &mut app.show_graph_editor,
                layer_filter_text: &mut app.ui_tabs.layer_filter_text,
                timeline_view_start: &mut app.timeline_view_start,
                work_area_in: &mut app.playback.work_area_in,
                work_area_out: &mut app.playback.work_area_out,
                expanded_layers: &mut app.selection.expanded_layers,
                fit_to_selection: &mut app.timeline_fit_to_selection,
                fit_all: &mut app.timeline_fit_all,
            };

            // Access live project mutably without per-frame cloning
            {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if draw_timeline_header(&mut header_state, ui, comp_mut, current_frame, total_frames) {
                    project_changed = true;
                }

                // ── Fit to Selection: zoom timeline to selected layers' time range ──
                if app.timeline_fit_to_selection {
                    app.timeline_fit_to_selection = false;
                    let comp = app.history.current().active_composition();
                    let layers = if app.timeline_fit_all {
                        comp.layers.iter().enumerate().collect::<Vec<_>>()
                    } else {
                        let sel = &app.selection.selected_layers;
                        comp.layers.iter().enumerate().filter(|(i, _)| sel.contains(i)).collect::<Vec<_>>()
                    };
                    if !layers.is_empty() {
                        let mut min_f = u32::MAX;
                        let mut max_f = 0u32;
                        for &(_, layer) in &layers {
                            min_f = min_f.min(layer.in_frame);
                            max_f = max_f.max(layer.out_frame);
                        }
                        if max_f > min_f {
                            let span = (max_f - min_f).max(10);
                            app.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                            app.timeline_view_start = min_f;
                        }
                    }
                    app.timeline_fit_all = false;
                }
            }


            let timeline_history_generation = app.history.generation();
            // Rebuild while a model-edit transaction or row reorder is live,
            // because keyframe frames and layer indices can change mid-gesture.
            // Unrelated pointer activity (hover, scroll, toolbar clicks) does
            // not invalidate these derived lists and should keep using cache.
            let timeline_interacting = app.drag_active() || app.dragging_layer.is_some();
            let derived_timeline = timeline_derived_data(
                ui.ctx(),
                app.history.current().active_composition(),
                timeline_history_generation,
                timeline_interacting,
            );

            let mut automation_curve = app
                .production_document
                .as_ref()
                .and_then(|document| app.selected_automation_binding
                    .and_then(|index| document.bindings.get(index)))
                .map(|binding| binding.curve.clone());
            let temp_project = app.history.current_mut();

            // Graph mode is the animation curve workspace; dependency nodes are
            // intentionally kept out of the compositing timeline.
            if app.show_graph_editor {
                if let Some(selected_idx) = app.selection.selected_layer_idx {
                    let duration_f = temp_project.active_composition().duration_frames;
                    let fps = temp_project.active_composition().fps;
                    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(selected_idx) {
                        crate::ui::graph_editor::draw_graph_editor(
                            &mut app.selection.selected_property,
                            ui,
                            duration_f,
                            fps,
                            layer,
                            &mut project_changed,
                            &mut app.linked_tangent,
                        );
                    }
                    if let Some(curve) = automation_curve.as_mut() {
                        crate::ui::graph_editor::draw_automation_curve(ui, curve, &mut project_changed);
                    }
                } else {
                    ui.label("Select a layer to edit keyframe curves in Graph Editor");
                }
                if let (Some(curve), Some(index), Some(document)) = (
                    automation_curve,
                    app.selected_automation_binding,
                    app.production_document.as_mut(),
                ) {
                    if let Some(binding) = document.bindings.get_mut(index) {
                        binding.curve = curve;
                    }
                }
                if project_changed {
                    if let Some(snapshot) = pre_edit_snapshot {
                        app.begin_drag_with_snapshot(snapshot, "Graph Editor Edit");
                    }
                    crate::core::frame_cache::bump_version();
                    app.autosave.mark_dirty();
                }
                if !ui.input(|i| i.pointer.any_down()) {
                    app.commit_drag();
                }
                return;
            }


            // ── Pre-collect sub-comp layer data (before mutable borrow) ──
            let precomp_children = crate::ui::timeline::precomp_children::collect_cached(
                ui.ctx(),
                temp_project,
                &app.selection.expanded_layers,
                app.project_session_id,
                timeline_history_generation,
            );

            // ── Tracks Mode: Layer List & Keyframe Ruler Area ──
            let comp = temp_project.active_composition_mut();

            // ── Responsive Width Calculation ──
            let total_w = ui.available_width();
            let left_pane_width_id = egui::Id::new("timeline_left_pane_width_v2");
            let default_left_pane_w = crate::ui::theme::layout::TIMELINE_LEFT_PANE_WIDTH;
            let requested_left_pane_w = ui.ctx().data_mut(|data| {
                data.get_persisted::<f32>(left_pane_width_id)
            })
            .unwrap_or(default_left_pane_w);
            let initial_columns = layout::TimelineColumns::new(
                total_w,
                requested_left_pane_w,
                ui.spacing().item_spacing.x,
            );
            let mut left_pane_w = initial_columns.left_width;
            let max_left_pane_w = initial_columns.max_left_width;

            // ── Work Area (In / Out) Handles Bar ──
            ui.horizontal(|ui| {
                let divider_rect = egui::Rect::from_min_size(
                    egui::pos2(ui.max_rect().left() + left_pane_w - 7.0, ui.cursor().top() - 4.0),
                    egui::vec2(14.0, 26.0),
                );
                let divider_response = ui.interact(
                    divider_rect,
                    left_pane_width_id.with("divider"),
                    egui::Sense::drag(),
                );
                let divider_color = if divider_response.hovered() || divider_response.dragged() {
                    colors::BORDER_ACTIVE
                } else {
                    colors::BORDER_SUBTLE
                };
                if divider_response.hovered() || divider_response.dragged() {
                    ui.painter().rect_filled(
                        divider_rect,
                        3.0,
                        colors::BG_HOVER,
                    );
                }
                ui.painter().line_segment(
                    [divider_rect.center_top(), divider_rect.center_bottom()],
                    egui::Stroke::new(1.0_f32, divider_color),
                );
                if divider_response.hovered() || divider_response.dragged() {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                }
                if divider_response.dragged() || divider_response.drag_stopped() {
                    if let Some(pointer) = divider_response.interact_pointer_pos() {
                        left_pane_w = (pointer.x - ui.max_rect().left())
                            .clamp(initial_columns.min_left_width, max_left_pane_w);
                        ui.ctx().data_mut(|data| {
                            data.insert_persisted(left_pane_width_id, left_pane_w);
                        });
                    }
                }
                divider_response.on_hover_text("ドラッグしてレイヤー列の幅を調整");

                let columns = layout::TimelineColumns::new(
                    total_w,
                    left_pane_w,
                    ui.spacing().item_spacing.x,
                );

                ui.allocate_ui(egui::vec2(left_pane_w, layout::COLUMN_HEADER_HEIGHT), |ui| {
                    ui.set_min_width(left_pane_w);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("レイヤー").small().strong().color(colors::TEXT_SECONDARY))
                            .on_hover_text("B：作業範囲の開始点、N：終了点を現在位置に設定");

                        if ui.selectable_label(app.ui_tabs.global_shy_active, "シャイ").on_hover_text("シャイレイヤーの表示／非表示を切り替えます").clicked() {
                            app.ui_tabs.global_shy_active = !app.ui_tabs.global_shy_active;
                        }

                        let is_w_hovered = ui.rect_contains_pointer(ui.max_rect());
                        if app.playback.work_area_in.is_some() || app.playback.work_area_out.is_some() {
                            let w_in_frame = app.playback.work_area_in.unwrap_or(0);
                            let w_out_frame = app.playback.work_area_out.unwrap_or(total_frames);
                            ui.weak(format!("範囲 {}–{}f", w_in_frame, w_out_frame));
                        }

                        if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::B)) {
                            app.playback.work_area_in = Some(*current_frame);
                            log::info!("Set Work Area In point to frame {}", *current_frame);
                        }
                        if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::N)) {
                            app.playback.work_area_out = Some(*current_frame);
                            log::info!("Set Work Area Out point to frame {}", *current_frame);
                        }
                    });
                });

                let avail_w = ui.available_width().max(0.0).min(columns.time_width);
                let (ruler_rect, ruler_response) = ui.allocate_exact_size(
                    egui::vec2(avail_w, layout::COLUMN_HEADER_HEIGHT),
                    egui::Sense::click_and_drag(),
                );

                // Ruler scrub cursor feedback
                if ruler_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                ui.painter().rect_filled(ruler_rect, 0.0, colors::BG_DARK);

                let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
                let playhead_moved = ui.ctx().data_mut(|data| {
                    let id = egui::Id::new("timeline_last_playhead_frame");
                    let previous = data.get_temp::<u32>(id);
                    data.insert_temp(id, *current_frame);
                    previous.is_some_and(|frame| frame != *current_frame)
                });
                // Preserve user-panned/zoomed views while idle. Recenter only when
                // playback or a scrub actually moves the playhead outside the view.
                if playhead_moved
                    && (*current_frame < app.timeline_view_start
                    || *current_frame >= app.timeline_view_start.saturating_add(zoom_span)
                    )
                {
                    app.timeline_view_start =
                        current_frame.saturating_sub(zoom_span / 2).min(total_frames.saturating_sub(zoom_span));
                }
                let start_frame = app.timeline_view_start;

                crate::ui::timeline::ruler_bar::draw_ram_ruler(
                    &app.frame_cache,
                    comp,
                    ui,
                    total_frames,
                    ruler_rect,
                    start_frame,
                    zoom_span,
                );

                // Work area highlighted bar
                let wa_in = app.playback.work_area_in.unwrap_or(0);
                let wa_out = app.playback.work_area_out.unwrap_or(total_frames);
                let mut wa_drag_active = false;
                let has_explicit_work_area = app.playback.work_area_in.is_some()
                    || app.playback.work_area_out.is_some();
                if has_explicit_work_area
                    && wa_out > wa_in
                    && wa_out >= start_frame
                    && wa_in <= start_frame + zoom_span
                {
                    let norm_in = (wa_in.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let norm_out = (wa_out.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let wa_rect = egui::Rect::from_min_max(
                        egui::pos2(ruler_rect.left() + norm_in * ruler_rect.width(), ruler_rect.top() + 2.0),
                        egui::pos2(ruler_rect.left() + norm_out * ruler_rect.width(), ruler_rect.bottom() - 2.0),
                    );
                    ui.painter().rect_filled(wa_rect, 2.0, colors::TIMELINE_SELECTION);
                    ui.painter().rect_stroke(wa_rect, 2.0, egui::Stroke::new(1.0_f32, colors::BORDER_ACTIVE));

                    // ── Work-area edge handles: drag to set In/Out points ──
                    const WA_HW: f32 = 5.0;
                    let wa_in_handle = egui::Rect::from_min_size(
                        egui::pos2(wa_rect.left() - WA_HW * 0.5, ruler_rect.top()),
                        egui::vec2(WA_HW, ruler_rect.height()),
                    );
                    let wa_out_handle = egui::Rect::from_min_size(
                        egui::pos2(wa_rect.right() - WA_HW * 0.5, ruler_rect.top()),
                        egui::vec2(WA_HW, ruler_rect.height()),
                    );
                    let wa_in_resp = ui.interact(wa_in_handle, egui::Id::new("wa_in_handle"), egui::Sense::drag());
                    let wa_out_resp = ui.interact(wa_out_handle, egui::Id::new("wa_out_handle"), egui::Sense::drag());
                    let wa_frame_from_pointer = |resp: &egui::Response| -> Option<u32> {
                        resp.interact_pointer_pos().map(|p| {
                            let norm = ((p.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                            start_frame.saturating_add((norm * zoom_span as f32).round() as u32)
                        })
                    };
                    if wa_in_resp.dragged() {
                        wa_drag_active = true;
                        if let Some(f) = wa_frame_from_pointer(&wa_in_resp) {
                            app.playback.work_area_in = Some(f.min(app.playback.work_area_out.unwrap_or(total_frames).saturating_sub(1)));
                            crate::core::frame_cache::bump_version();
                        }
                    }
                    if wa_out_resp.dragged() {
                        wa_drag_active = true;
                        if let Some(f) = wa_frame_from_pointer(&wa_out_resp) {
                            app.playback.work_area_out = Some(f.max(app.playback.work_area_in.unwrap_or(0) + 1));
                            crate::core::frame_cache::bump_version();
                        }
                    }
                    if wa_in_resp.hovered() || wa_out_resp.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                    }
                }

                // Measure real timecode labels before choosing a tick interval so
                // the labels remain readable across zoom, panel width, and UI scale.
                let fps = comp.fps.max(1);
                let timecode_font = egui::FontId::new(
                    ui.text_style_height(&egui::TextStyle::Small) * 0.72,
                    egui::FontFamily::Monospace,
                );
                let end_frame = start_frame.saturating_add(zoom_span);
                let measured_label_width = [start_frame, end_frame]
                    .into_iter()
                    .map(|frame| {
                        let label = crate::ui::timeline::ruler::format_timecode_label(frame, fps);
                        ui.painter()
                            .layout_no_wrap(label, timecode_font.clone(), colors::TEXT_MUTED)
                            .size()
                            .x
                    })
                    .fold(0.0_f32, f32::max);
                let step = crate::ui::timeline::ruler::ruler_label_step(
                    zoom_span,
                    ruler_rect.width(),
                    measured_label_width,
                );
                let mut last_label_right = f32::NEG_INFINITY;
                for f in crate::ui::timeline::ruler::ruler_tick_frames(
                    start_frame,
                    zoom_span,
                    step,
                ) {
                    let norm = (f - start_frame) as f32 / zoom_span as f32;
                    let tick_x = ruler_rect.left() + norm * ruler_rect.width();
                    ui.painter().line_segment(
                        [egui::pos2(tick_x, ruler_rect.bottom() - 6.0), egui::pos2(tick_x, ruler_rect.bottom())],
                        egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
                    );
                    let tc_str = crate::ui::timeline::ruler::format_timecode_label(f, fps);
                    let galley = ui.painter().layout_no_wrap(
                        tc_str.clone(),
                        timecode_font.clone(),
                        colors::TEXT_MUTED,
                    );
                    let label_rect = egui::Rect::from_center_size(
                        egui::pos2(tick_x, ruler_rect.top() + 7.0),
                        galley.size(),
                    );
                    if label_rect.left() >= ruler_rect.left()
                        && label_rect.right() <= ruler_rect.right()
                        && label_rect.left() >= last_label_right + 6.0
                    {
                        ui.painter()
                            .with_clip_rect(ruler_rect)
                            .galley(label_rect.min, galley, colors::TEXT_MUTED);
                        last_label_right = label_rect.right();
                    }
                }

                // Current Playhead Line (Blue indicator)
                let playhead_norm = (*current_frame as f32 - start_frame as f32) / zoom_span as f32;
                let playhead_x = ruler_rect.left() + playhead_norm * ruler_rect.width();
                if (0.0..=1.0).contains(&playhead_norm) {
                    ui.painter().line_segment(
                        [
                            egui::pos2(playhead_x, ruler_rect.top() + 4.0),
                            egui::pos2(playhead_x, ruler_rect.bottom()),
                        ],
                        egui::Stroke::new(1.5_f32, colors::TIMELINE_PLAYHEAD),
                    );
                    // Inverted triangle handle (AE-style grab point)
                    let tri = vec![
                        egui::pos2(playhead_x - 4.0, ruler_rect.top()),
                        egui::pos2(playhead_x + 4.0, ruler_rect.top()),
                        egui::pos2(playhead_x, ruler_rect.top() + 6.0),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(tri, colors::TIMELINE_PLAYHEAD, egui::Stroke::NONE));
                }

                // ── Ctrl/Cmd+drag on ruler: rubber-band Work Area definition ──
                let cmd_mod = ui.input(|i| i.modifiers.command);
                if !wa_drag_active && cmd_mod && ruler_response.dragged() {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        let drag_id = egui::Id::new("wa_rubber_start");
                        let anchor_f = ui.ctx().data_mut(|d| {
                            let cur = d.get_temp::<u32>(drag_id);
                            if cur.is_none() { d.insert_temp(drag_id, raw_f); }
                            cur
                        });
                        if let Some(s) = anchor_f {
                            app.playback.work_area_in = Some(s.min(raw_f));
                            app.playback.work_area_out = Some(s.max(raw_f).saturating_add(1));
                        }
                    }
                }
                if ruler_response.drag_stopped() {
                    ui.ctx().data_mut(|d| d.remove_temp::<u32>(egui::Id::new("wa_rubber_start")));
                }

                if ruler_response.double_clicked() && !cmd_mod {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        comp.markers.push(crate::core::timeline::TimelineMarker {
                            frame: raw_f,
                            label: format!("Marker {}", comp.markers.len() + 1),
                            color: [0.35, 0.75, 1.0],
                        });
                        project_changed = true;
                        app.toasts.info(format!("Comp marker added at frame {}", raw_f));
                    }
                }

                let ruler_scrub_response = ui.interact(
                    ruler_rect,
                    egui::Id::new("timeline_ruler_scrub"),
                    egui::Sense::click_and_drag(),
                );
                let (pointer_pos, primary_pressed, primary_down, primary_released) = ui.input(|input| {
                    (
                        input.pointer.interact_pos().or_else(|| input.pointer.hover_pos()),
                        input.pointer.button_pressed(egui::PointerButton::Primary),
                        input.pointer.button_down(egui::PointerButton::Primary),
                        input.pointer.button_released(egui::PointerButton::Primary),
                    )
                });
                let scrub_id = egui::Id::new("timeline_ruler_scrub_active");
                if primary_pressed && pointer_pos.is_some_and(|pos| ruler_rect.contains(pos)) {
                    ui.ctx().data_mut(|data| data.insert_temp(scrub_id, true));
                }
                let scrub_active = ui.ctx().data(|data| data.get_temp::<bool>(scrub_id).unwrap_or(false));
                let should_scrub = !wa_drag_active
                    && !cmd_mod
                    && pointer_pos.is_some_and(|pos| ruler_rect.contains(pos))
                    && ((scrub_active && (primary_down || primary_released))
                        || ruler_scrub_response.clicked()
                        || ruler_scrub_response.dragged());
                if should_scrub {
                    if let Some(pos) = pointer_pos {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        *current_frame = maybe_snap_frame(
                            raw_f,
                            app.snap_to_keyframes,
                            &derived_timeline.keyframe_frames,
                        );
                    }
                }
                if primary_released || (scrub_active && !primary_down) {
                    ui.ctx().data_mut(|data| data.remove_temp::<bool>(scrub_id));
                }

                // ── Ruler context menu (AE parity) ──
                ruler_response.context_menu(|ui| {
                    if ui.button("Add Composition Marker at Playhead").clicked() {
                        comp.markers.push(crate::core::timeline::TimelineMarker {
                            frame: *current_frame,
                            label: format!("Marker {}", comp.markers.len() + 1),
                            color: [0.35, 0.75, 1.0],
                        });
                        project_changed = true;
                        app.toasts.info(format!("Comp marker at frame {}", current_frame));
                        ui.close_menu();
                    }
                    if !comp.markers.is_empty() && ui.button("Clear All Composition Markers").clicked() {
                        comp.markers.clear();
                        project_changed = true;
                        ui.close_menu();
                    }
                    if let Some(layer_idx) = app.selection.selected_layer_idx {
                        if ui.button("Split Selected Layer at Playhead").clicked() {
                            actions.pending_split_layer = Some(layer_idx);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Zoom to Work Area").clicked() {
                        let w_in = app.playback.work_area_in.unwrap_or(0);
                        let w_out = app.playback.work_area_out.unwrap_or(total_frames).max(w_in + 1);
                        let span = (w_out - w_in).max(10);
                        app.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                        app.timeline_view_start = w_in;
                        ui.close_menu();
                    }
                    if ui.button("Set Comp Duration to Work Area End").clicked() {
                        let w_out = app.playback.work_area_out.unwrap_or(total_frames);
                        if w_out > 0 && w_out <= total_frames {
                            actions.pending_duration = Some(w_out);
                            app.toasts.info(format!("Comp duration set to {} frames", w_out));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Trim Comp to Work Area").on_hover_text("Set comp duration to work area and trim layers beyond it").clicked() {
                        let w_in = app.playback.work_area_in.unwrap_or(0);
                        let w_out = app.playback.work_area_out.unwrap_or(total_frames);
                        if w_out > w_in {
                            actions.pending_duration = Some(w_out);
                            actions.pending_trim_work_area = Some((w_in, w_out));
                            app.toasts.info(format!("Trimmed to work area: {}f–{}f", w_in, w_out));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Reset Work Area to Full Comp").clicked() {
                        app.playback.work_area_in = None;
                        app.playback.work_area_out = None;
                        ui.close_menu();
                    }
                });

                // ⌨️ B / N Keyboard Shortcuts for Work Area In / Out
                if ui.input(|i| i.key_pressed(egui::Key::B)) {
                    app.playback.work_area_in = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area Start at frame {}", current_frame));
                }
                if ui.input(|i| i.key_pressed(egui::Key::N)) {
                    app.playback.work_area_out = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area End at frame {}", current_frame));
                }
                // Timeline Zoom Shortcuts (= / -), anchored on the playhead so the
                // frame under it stays visually in place while zooming.
                {
                    // Returns the new window start that keeps the playhead stationary.
                    let view_start = app.timeline_view_start;
                    let anchored_start = |old_zoom: f32, new_zoom: f32| -> u32 {
                        let old_span = (total_frames as f32 / old_zoom.max(0.01)).max(10.0) as u32;
                        let new_span = (total_frames as f32 / new_zoom.max(0.01)).max(10.0) as u32;
                        // The playhead's offset from the window's left edge scales with
                        // the span ratio, keeping the playhead stationary on screen.
                        let head_offset = current_frame.saturating_sub(view_start);
                        let new_head_offset = ((head_offset as f32) * (new_span as f32 / old_span.max(1) as f32)) as u32;
                        current_frame
                            .saturating_sub(new_head_offset)
                            .min(total_frames.saturating_sub(new_span))
                    };
                    let pointer_anchored_start =
                        |old_zoom: f32, old_start: u32, new_zoom: f32, pointer_x: f32| -> u32 {
                        let old_span =
                            (total_frames as f32 / old_zoom.max(0.01)).max(10.0) as u32;
                        let new_span = (total_frames as f32 / new_zoom.max(0.01))
                            .max(10.0) as u32;
                        let pointer_fraction = ((pointer_x - ruler_rect.left())
                            / ruler_rect.width().max(1.0))
                            .clamp(0.0, 1.0);
                        let frame_under_pointer = old_start as f32 + pointer_fraction * old_span as f32;
                        ((frame_under_pointer - pointer_fraction * new_span as f32)
                            .round()
                            .max(0.0) as u32)
                            .min(total_frames.saturating_sub(new_span))
                    };
                    if ui.input(|i| i.key_pressed(egui::Key::Equals)) && app.timeline_zoom < 20.0 {
                        let new_zoom = (app.timeline_zoom * 1.25).min(20.0);
                        app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                        app.timeline_zoom = new_zoom;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Minus)) && app.timeline_zoom > 0.1 {
                        let new_zoom = (app.timeline_zoom / 1.25).max(0.1);
                        app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                        app.timeline_zoom = new_zoom;
                    }
                    if ruler_response.hovered() {
                        let (scroll, command, pointer_x) = ui.input(|i| {
                            (
                                i.raw_scroll_delta.y,
                                i.modifiers.command,
                                i.pointer.hover_pos().map(|pos| pos.x),
                            )
                        });
                        if scroll != 0.0 {
                            if command {
                                let max_start = total_frames.saturating_sub(zoom_span);
                                let pan_frames = (-scroll * zoom_span as f32 * 0.08).round() as i64;
                                app.timeline_view_start = (app.timeline_view_start as i64 + pan_frames)
                                    .clamp(0, max_start as i64) as u32;
                            } else if let Some(pointer_x) = pointer_x {
                                let factor = if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 };
                                let new_zoom = (app.timeline_zoom * factor).clamp(0.1, 20.0);
                                if new_zoom != app.timeline_zoom {
                                    app.timeline_view_start = pointer_anchored_start(
                                        app.timeline_zoom,
                                        app.timeline_view_start,
                                        new_zoom,
                                        pointer_x,
                                    );
                                    app.timeline_zoom = new_zoom;
                                }
                            }
                        }
                    }
                }
            });

            crate::ui::theme::draw_separator(ui);

            // ── Scrollable Layer Rows & Property Tracks ──
            let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
            let start_frame = app.timeline_view_start;
            // Capture effect drag info before closure to avoid borrow conflicts
            let drag_info = app.dragging_effect.clone();
            // Collect pending effect drops to apply after the closure

            let layer_scroll_height = ui.available_height().max(32.0);
            let _scroll_resp = egui::ScrollArea::vertical().max_height(layer_scroll_height).show(ui, |ui| {
                let layers_len = comp.layers.len();
                let comp_w_f = comp.width as f32;
                let comp_h_f = comp.height.max(1) as f32;
                let derived = &derived_timeline;
                let parent_choices_ref = &derived.parent_choices;
                let layer_edges = &derived.layer_edges;
                let all_kf_frames = &derived.keyframe_frames;
                if layers_len == 0 {
                    ui.add_space(4.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("素材を追加するか、ここにドロップして編集を始めます")
                                .small()
                                .color(colors::TEXT_MUTED),
                        );
                    });
                    ui.add_space(4.0);
                }
                for i in 0..layers_len {
                    let layer_preview_textures = comp.layers.get(i).and_then(|layer| {
                        if !layer_has_timeline_preview(&layer.layer_type)
                            || (app.ui_tabs.global_shy_active && layer.is_shy)
                            || (!app.ui_tabs.layer_filter_text.is_empty()
                                && !layer
                                    .name
                                    .to_lowercase()
                                    .contains(&app.ui_tabs.layer_filter_text.to_lowercase()))
                        {
                            return None;
                        }
                        let mut layer_height = layout::LAYER_ROW_HEIGHT;
                        if app.selection.expanded_layers.contains(&i) {
                            layer_height +=
                                (5 + layer.effects.len() + layer.masks.len()) as f32
                                    * layout::PROPERTY_ROW_HEIGHT;
                        }
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), layer_height),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            return None;
                        }
                        let visible_start = layer.in_frame.max(start_frame);
                        let visible_end = layer
                            .out_frame
                            .min(start_frame.saturating_add(zoom_span));
                        let visible_span = visible_end.saturating_sub(visible_start);
                        if visible_span == 0 {
                            return None;
                        }
                        let time_canvas_width =
                            (ui.available_width() - left_pane_w).max(0.0);
                        let visible_width = visible_span as f32 / zoom_span.max(1) as f32
                            * time_canvas_width;
                        let sample_count = (visible_width / TIMELINE_THUMBNAIL_WIDTH)
                            .ceil()
                            .clamp(1.0, MAX_TIMELINE_THUMBNAILS_PER_CLIP as f32)
                            as usize;
                        let clip_span = layer.out_frame.saturating_sub(layer.in_frame).max(1) as f32;
                        let preview_start = visible_start
                            .saturating_sub(layer.in_frame) as f32
                            / clip_span;
                        let preview_end = visible_end
                            .saturating_sub(layer.in_frame) as f32
                            / clip_span;
                        Some((
                            preview_start.clamp(0.0, 1.0),
                            preview_end.clamp(0.0, 1.0),
                            (0..sample_count)
                                .map(|sample| {
                                    let sample_t = (sample as f32 + 0.5) / sample_count as f32;
                                    let frame = visible_start.saturating_add(
                                        (visible_span as f32 * sample_t).round() as u32,
                                    );
                                    timeline_layer_preview_texture(
                                        ui.ctx(),
                                        comp,
                                        layer,
                                        frame,
                                    )
                                })
                                .collect::<Vec<_>>(),
                        ))
                    });
                    // Safe index access (.get_mut(i))
                    if let Some(layer) = comp.layers.get_mut(i) {
                        if app.ui_tabs.global_shy_active && layer.is_shy {
                            continue;
                        }
                        if !app.ui_tabs.layer_filter_text.is_empty() && !layer.name.to_lowercase().contains(&app.ui_tabs.layer_filter_text.to_lowercase()) {
                            continue;
                        }

                        // Calculate exact estimated height for this layer based on expanded rows
                        let mut layer_height = layout::LAYER_ROW_HEIGHT;
                        if app.selection.expanded_layers.contains(&i) {
                            let prop_count = 5 + layer.effects.len() + layer.masks.len();
                            layer_height += prop_count as f32 * layout::PROPERTY_ROW_HEIGHT;
                        }

                        // ── Visibility Culling: skip off-screen rows ──
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), layer_height),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            // Still advance cursor by exact layer height to keep scroll extent accurate
                            ui.add_space(layer_height);
                            continue;
                        }

                        let row_columns = layout::TimelineColumns::new(
                            total_w,
                            left_pane_w,
                            ui.spacing().item_spacing.x,
                        );

                        ui.horizontal(|ui| {
                            // Denser rows: shrink button padding for the pill
                            // controls (Switches/Modes/reorder) in this row.
                            ui.style_mut().spacing.button_padding = egui::vec2(4.0, 1.0);
                            ui.allocate_ui(egui::vec2(left_pane_w, layout::LAYER_ROW_HEIGHT), |ui| {
                                ui.set_min_width(left_pane_w);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.spacing_mut().item_spacing.x = if left_pane_w < 240.0 { 2.0 } else { 4.0 };
                                ui.spacing_mut().interact_size.y = layout::LAYER_ROW_HEIGHT;
                                ui.spacing_mut().button_padding = egui::vec2(2.0, 0.0);
                                ui.horizontal(|ui| {
                                    if left_pane_w >= 240.0 {
                                        ui.label(egui::RichText::new(format!("{:02}", i + 1)).small().strong().color(colors::TEXT_SECONDARY));
                                        ui.add_space(2.0);
                                    }

                                    // ── Layer type icon ──
                                    {
let type_icon = crate::ui::icons::layer_icon(&layer.layer_type);
                                        crate::ui::kui::icon_glyph(
                                            ui,
                                            type_icon,
                                            14.0,
                                            colors::TEXT_SECONDARY,
                                        );
                                    }
                                    ui.add_space(1.0);

                                    // ── Label color chip: click cycles through AE label colors ──
                                    {
                                        let rgb = layer.label.to_rgb();
                                        let chip_color = egui::Color32::from_rgb(
                                            (rgb[0] * 255.0) as u8,
                                            (rgb[1] * 255.0) as u8,
                                            (rgb[2] * 255.0) as u8,
                                        );
                                        let (chip_rect, chip_resp) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 14.0),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(chip_rect, 2.0, chip_color);
                                        let row_selected = app.selection.selected_layers.contains(&i) || app.selection.selected_layer_idx == Some(i);
                                        if row_selected {
                                            ui.painter().rect_stroke(chip_rect, 2.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE));
                                        }
                                        if chip_resp.clicked() {
                                            let next = match layer.label {
                                                crate::core::timeline::LabelColor::None => crate::core::timeline::LabelColor::Red,
                                                crate::core::timeline::LabelColor::Red => crate::core::timeline::LabelColor::Yellow,
                                                crate::core::timeline::LabelColor::Yellow => crate::core::timeline::LabelColor::Aqua,
                                                crate::core::timeline::LabelColor::Aqua => crate::core::timeline::LabelColor::Pink,
                                                crate::core::timeline::LabelColor::Pink => crate::core::timeline::LabelColor::Lavender,
                                                crate::core::timeline::LabelColor::Lavender => crate::core::timeline::LabelColor::Peach,
                                                crate::core::timeline::LabelColor::Peach => crate::core::timeline::LabelColor::Sea,
                                                crate::core::timeline::LabelColor::Sea => crate::core::timeline::LabelColor::Blue,
                                                crate::core::timeline::LabelColor::Blue => crate::core::timeline::LabelColor::Purple,
                                                crate::core::timeline::LabelColor::Purple => crate::core::timeline::LabelColor::None,
                                            };
                                            // Direct mutation: layer is already &mut from the row loop
                                            layer.label = next;
                                            project_changed = true;
                                        }
                                        if chip_resp.secondary_clicked() {
                                            actions.pending_select_label_group = Some(layer.label);
                                        }
                                        if chip_resp.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    }
                                    ui.add_space(2.0);

                                    ui.add_space(if left_pane_w < 240.0 { 1.0 } else { 4.0 });
                                    let is_expanded = app.selection.expanded_layers.contains(&i);
                                    let arrow = if is_expanded { "⌄" } else { "›" };
                                    let (arrow_rect, arrow_response) = ui.allocate_exact_size(
                                        egui::vec2(if left_pane_w < 240.0 { 14.0 } else { 20.0 }, 22.0),
                                        egui::Sense::click(),
                                    );
                                    if arrow_response.hovered() {
                                        ui.painter().rect_filled(arrow_rect, 3.0, colors::BG_HOVER);
                                    }
                                    ui.painter().text(
                                        arrow_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        arrow,
                                        egui::FontId::proportional(15.0),
                                        if is_expanded { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED },
                                    );
                                    if arrow_response.clicked() {
                                        if is_expanded {
                                            app.selection.expanded_layers.remove(&i);
                                        } else {
                                            app.selection.expanded_layers.insert(i);
                                        }
                                    }

                                    if left_pane_w >= 240.0 {
                                    ui.scope(|ui| {
                                    let widgets = &mut ui.style_mut().visuals.widgets;
                                    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.hovered.bg_fill = colors::BG_HOVER;
                                    widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                    widgets.hovered.weak_bg_fill = colors::BG_HOVER;
                                    ui.menu_button(egui::RichText::new("色").small(), |ui| {
                                    // ── AE Layer Color Label Square Picker ──
                                    let label_rgb = layer.label.to_rgb();
                                    let label_c32 = egui::Color32::from_rgb(
                                        (label_rgb[0] * 255.0) as u8,
                                        (label_rgb[1] * 255.0) as u8,
                                        (label_rgb[2] * 255.0) as u8,
                                    );
                                    let (lbl_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(lbl_rect, 1.0, label_c32);
                                    let lbl_resp = ui.interact(
                                        lbl_rect.expand(
                                            crate::ui::theme::layout::HIT_TARGET_MIN * 0.5 - 6.0,
                                        ),
                                        ui.make_persistent_id(("layer_label_chip", layer.id.as_str())),
                                        egui::Sense::click(),
                                    );
                                    lbl_resp.context_menu(|ui| {
                                        ui.label("Label Color:");
                                        for label_choice in [
                                            LabelColor::Red, LabelColor::Yellow, LabelColor::Aqua,
                                            LabelColor::Pink, LabelColor::Lavender, LabelColor::Peach,
                                            LabelColor::Sea, LabelColor::Blue, LabelColor::Purple
                                        ] {
                                            if ui.button(format!("{:?}", label_choice)).clicked() {
                                                layer.label = label_choice;
                                                project_changed = true;
                                                ui.close_menu();
                    }
                }
                // ── Click empty area below layers → deselect all ──
                let remaining = ui.available_rect_before_wrap();
                let bg_resp = ui.interact(remaining, egui::Id::new("timeline_bg_deselect"), egui::Sense::click());
                if bg_resp.clicked() {
                    app.selection.selected_layers.clear();
                    app.selection.selected_layer_idx = None;
                }
            });

                                    if !app.ui_tabs.show_switches_pane {
                                    // Track Matte Mode Selection (AE Standard)
                                    let matte_id = ui.make_persistent_id(format!("tm_combo_{}", i));
                                    let matte_label = match layer.track_matte {
                                        crate::core::timeline::TrackMatteMode::AlphaMatte => "Alpha Matte",
                                        crate::core::timeline::TrackMatteMode::AlphaMatteInverted => "Alpha Inverted",
                                        crate::core::timeline::TrackMatteMode::LumaMatte => "Luma Matte",
                                        crate::core::timeline::TrackMatteMode::LumaMatteInverted => "Luma Inverted",
                                        crate::core::timeline::TrackMatteMode::None => "No Matte",
                                    };
                                    egui::ComboBox::from_id_salt(matte_id)
                                        .selected_text(matte_label)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::None, "No Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatte, "Alpha Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatteInverted, "Alpha Inverted").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatte, "Luma Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatteInverted, "Luma Inverted").clicked() {
                                                project_changed = true;
                                            }
                                        });

                                    // Blend Mode Selection (AE Standard)
                                    let blend_id = ui.make_persistent_id(format!("bm_combo_{}", i));
                                    let blend_label = match layer.blend_mode {
                                        crate::core::timeline::BlendMode::Normal => "Normal",
                                        crate::core::timeline::BlendMode::Multiply => "Multiply",
                                        crate::core::timeline::BlendMode::Screen => "Screen",
                                        crate::core::timeline::BlendMode::Overlay => "Overlay",
                                        crate::core::timeline::BlendMode::Add => "Add",
                                        crate::core::timeline::BlendMode::Darken => "Darken",
                                        crate::core::timeline::BlendMode::Lighten => "Lighten",
                                        crate::core::timeline::BlendMode::SoftLight => "Soft Light",
                                        crate::core::timeline::BlendMode::HardLight => "Hard Light",
                                        crate::core::timeline::BlendMode::Difference => "Difference",
                                        crate::core::timeline::BlendMode::Exclusion => "Exclusion",
                                        crate::core::timeline::BlendMode::Divide => "Divide",
                                        crate::core::timeline::BlendMode::Subtract => "Subtract",
                                            crate::core::timeline::BlendMode::ColorBurn => "Color Burn",
                                            crate::core::timeline::BlendMode::LinearBurn => "Linear Burn",
                                            crate::core::timeline::BlendMode::VividLight => "Vivid Light",
                                            crate::core::timeline::BlendMode::ColorDodge => "Color Dodge",
                                            crate::core::timeline::BlendMode::LinearDodge => "Linear Dodge",
                                            crate::core::timeline::BlendMode::Color => "Color",
                                            crate::core::timeline::BlendMode::Hue => "Hue",
                                            crate::core::timeline::BlendMode::Saturation => "Saturation",
                                            crate::core::timeline::BlendMode::Luminosity => "Luminosity",
                                            crate::core::timeline::BlendMode::StencilAlpha => "Stencil Alpha",
                                            crate::core::timeline::BlendMode::StencilLuma => "Stencil Luma",
                                            crate::core::timeline::BlendMode::SilhouetteAlpha => "Silhouette Alpha",
                                            crate::core::timeline::BlendMode::SilhouetteLuma => "Silhouette Luma",
                                            crate::core::timeline::BlendMode::Behind => "Behind",
                                            crate::core::timeline::BlendMode::AlphaAdd => "Alpha Add",
                                            crate::core::timeline::BlendMode::LinearLight => "Linear Light",
                                    };
                                    egui::ComboBox::from_id_salt(blend_id)
                                        .selected_text(blend_label)
                                        .show_ui(ui, |ui| {
                                            for (bm, name) in [
                                                (crate::core::timeline::BlendMode::Normal, "Normal"),
                                                (crate::core::timeline::BlendMode::Multiply, "Multiply"),
                                                (crate::core::timeline::BlendMode::Screen, "Screen"),
                                                (crate::core::timeline::BlendMode::Overlay, "Overlay"),
                                                (crate::core::timeline::BlendMode::Add, "Add"),
                                                (crate::core::timeline::BlendMode::Darken, "Darken"),
                                                (crate::core::timeline::BlendMode::Lighten, "Lighten"),
                                                (crate::core::timeline::BlendMode::SoftLight, "Soft Light"),
                                                (crate::core::timeline::BlendMode::HardLight, "Hard Light"),
                                                (crate::core::timeline::BlendMode::Difference, "Difference"),
                                                (crate::core::timeline::BlendMode::Exclusion, "Exclusion"),
                                                (crate::core::timeline::BlendMode::Divide, "Divide"),
                                                (crate::core::timeline::BlendMode::Subtract, "Subtract"),
                                                (crate::core::timeline::BlendMode::ColorBurn, "Color Burn"),
                                                (crate::core::timeline::BlendMode::LinearBurn, "Linear Burn"),
                                                (crate::core::timeline::BlendMode::VividLight, "Vivid Light"),
                                                (crate::core::timeline::BlendMode::ColorDodge, "Color Dodge"),
                                                (crate::core::timeline::BlendMode::LinearDodge, "Linear Dodge"),
                                                (crate::core::timeline::BlendMode::Color, "Color"),
                                                (crate::core::timeline::BlendMode::Hue, "Hue"),
                                                (crate::core::timeline::BlendMode::Saturation, "Saturation"),
                                                (crate::core::timeline::BlendMode::Luminosity, "Luminosity"),
                                                (crate::core::timeline::BlendMode::StencilAlpha, "Stencil Alpha"),
                                                (crate::core::timeline::BlendMode::StencilLuma, "Stencil Luma"),
                                                (crate::core::timeline::BlendMode::SilhouetteAlpha, "Silhouette Alpha"),
                                                (crate::core::timeline::BlendMode::SilhouetteLuma, "Silhouette Luma"),
                                                (crate::core::timeline::BlendMode::Behind, "Behind"),
                                                (crate::core::timeline::BlendMode::AlphaAdd, "Alpha Add"),
                                                (crate::core::timeline::BlendMode::LinearLight, "Linear Light"),
                                            ] {
                                                if ui.selectable_value(&mut layer.blend_mode, bm, name).clicked() {
                                                    project_changed = true;
                                                }
                                            }
                                        });
                                    }


                                    });
                                    });
                                    }
                                    let is_selected = app.selection.selected_layers.contains(&i) || app.selection.selected_layer_idx == Some(i);
                                    let name_width = layout::TimelineColumns::layer_name_width(
                                        ui.available_width(),
                                        app.ui_tabs.show_switches_pane,
                                    );
                                    let (name_rect, click_resp) = ui.allocate_exact_size(
                                        egui::vec2(name_width, layout::LAYER_ROW_HEIGHT),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if is_selected || click_resp.hovered() {
                                        let fill = if is_selected {
                                            egui::Color32::from_rgba_premultiplied(45, 81, 112, 82)
                                        } else {
                                            colors::BG_HOVER.linear_multiply(0.55)
                                        };
                                        ui.painter().rect_filled(name_rect, 3.0, fill);
                                    }
                                    if is_selected {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                name_rect.left_top(),
                                                egui::vec2(2.0, name_rect.height()),
                                            ),
                                            1.0,
                                            colors::ACCENT_SELECTION,
                                        );
                                    }
                                    ui.painter().with_clip_rect(name_rect.shrink2(egui::vec2(6.0, 0.0))).text(
                                        egui::pos2(name_rect.left() + 8.0, name_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &layer.name,
                                        egui::FontId::proportional(12.0),
                                        if is_selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY },
                                    );
                                    let click_resp = click_resp.on_hover_text(&layer.name);

                                    if app.ui_tabs.show_switches_pane {
                                        use crate::ui::custom_widgets::ae_svg_toggle;
                                        use crate::ui::icons::*;

                                        let visibility_icon = if layer.visible { SVG_EYE_OPEN } else { SVG_EYE_CLOSED };
                                        if ae_svg_toggle(ui, &mut layer.visible, visibility_icon, &format!("row_vis_{i}"), egui::vec2(15.0, 15.0), colors::TEXT_PRIMARY, "表示／非表示 (V)").clicked() {
                                            project_changed = true;
                                        }
                                        let lock_icon = if layer.locked { SVG_LOCK } else { SVG_UNLOCK };
                                        if ae_svg_toggle(ui, &mut layer.locked, lock_icon, &format!("row_lock_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "ロック (L)").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_shy, SVG_SHY, &format!("row_shy_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_CYAN, "シャイレイヤー (Shy)").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_collapsed, SVG_COLLAPSE_TRANSFORM, &format!("row_col_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_ORANGE, "トランスフォームの折りたたみ／コラプス").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_3d, SVG_3D_CUBE, &format!("row_3d_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "3Dレイヤー").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.motion_blur, SVG_MOTION_BLUR, &format!("row_mb_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "モーションブラー").clicked() {
                                            project_changed = true;
                                        }
                                    }

                                    // ── Pick Whip: clicking a layer in pick mode sets it as parent ──
                                    if click_resp.clicked() && app.pick_whip_mode {
                                        if let Some(sel) = app.selection.selected_layer_idx {
                                            if sel != i {
                                                actions.pending_pick_whip = Some((sel, i));
                                            }
                                        }
                                        app.pick_whip_mode = false;
                                        app.pick_whip_target = None;
                                    }

                                    if click_resp.double_clicked() {
                                        // Double-click a PreComp layer opens its nested composition (AE parity)
                                        // (deferred: resolved after the row loop)
                                        if let crate::core::timeline::LayerType::PreComp { comp_id } = &layer.layer_type {
                                            actions.pending_open_comp = Some(comp_id.clone());
                                        } else {
                                            app.renaming_layer = Some(i);
                                        }
                                    }
                                    if app.renaming_layer == Some(i) {
                                        let mut name_buf = layer.name.clone();
                                        let rename_resp = ui.text_edit_singleline(&mut name_buf);
                                        if rename_resp.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                            if !name_buf.is_empty() && name_buf != layer.name {
                                                layer.name = name_buf;
                                                project_changed = true;
                                            }
                                            app.renaming_layer = None;
                                        }
                                    }

                                    // ── Drag & drop reordering (live, AE-style) ──
                                    if click_resp.dragged() && app.dragging_layer.is_none() {
                                        app.dragging_layer = Some(i);
                                    }
                                    if let Some(src) = app.dragging_layer {
                                        if click_resp.hovered() && src != i {
                                            // Live swap; the dragged row follows the pointer
                                            actions.swap_request = Some((src, i));
                                            app.dragging_layer = Some(i);
                                        }
                                    }
                                    if click_resp.drag_stopped() || (app.dragging_layer.is_some() && !click_resp.dragged()) {
                                        // Drag ended — clear state (row index moved with the pointer)
                                        app.dragging_layer = None;
                                    }

                                    // ── Effect drop zone: drag from Effects & Presets library ──
                                    if let Some((ref effect_name, preset_idx)) = drag_info {
                                        if click_resp.hovered() {
                                            // Visual drop indicator: blue glow on the layer row
                                            let row_rect = click_resp.rect;
                                             ui.painter().rect_stroke(
                                                 row_rect, 4.0,
                                                 egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE)
                                             );
                                             ui.painter().rect_filled(
                                                 row_rect, 4.0,
                                                 colors::TIMELINE_SELECTION
                                             );
                                        }
                                        // Collect effect on drop (apply after loop to avoid borrow conflicts)
                                        if click_resp.drag_stopped() && click_resp.hovered() {
                                            actions.pending_effect_drops.push((i, effect_name.clone(), preset_idx));
                                            app.dragging_effect = None;
                                        }
                                    }

                                    if click_resp.clicked() {
                                        let modifiers = ui.input(|inp| inp.modifiers);
                                        if modifiers.shift || modifiers.command || modifiers.ctrl {
                                            if app.selection.selected_layers.contains(&i) {
                                                app.selection.selected_layers.remove(&i);
                                                if app.selection.selected_layer_idx == Some(i) {
                                                    app.selection.selected_layer_idx = app.selection.selected_layers.iter().next().copied();
                                                }
                                            } else {
                                                app.selection.selected_layers.insert(i);
                                                app.selection.selected_layer_idx = Some(i);
                                            }
                                        } else {
                                            app.selection.selected_layers.clear();
                                            app.selection.selected_layers.insert(i);
                                            app.selection.selected_layer_idx = Some(i);
                                        }
                                    }
                                    click_resp.context_menu(|ui| {
                                        if i > 0 && ui.button("↑ 上へ移動").clicked() {
                                            actions.swap_request = Some((i, i - 1));
                                            ui.close_menu();
                                        }
                                        if i + 1 < layers_len && ui.button("↓ 下へ移動").clicked() {
                                            actions.swap_request = Some((i, i + 1));
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Duplicate Layer (Cmd+D)").clicked() {
                                            actions.pending_dup_layer = Some(i);
                                            app.toasts.info("Duplicated selected layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("Pre-compose… (Cmd+Shift+C)").on_hover_text("Nest this layer into a new composition").clicked() {
                                            app.selection.selected_layers.clear();
                                            app.selection.selected_layers.insert(i);
                                            app.selection.selected_layer_idx = Some(i);
                                            app.show_precompose_dialog = true;
                                            ui.close_menu();
                                        }
                                        if matches!(layer.layer_type, crate::core::timeline::LayerType::Text { .. })
                                            && ui.button("Create Shapes from Text").on_hover_text("Decompose this Text layer into animatable vector Bezier Shape paths").clicked()
                                        {
                                            actions.pending_create_shapes = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("Toggle 3D Layer").on_hover_text("Switch this layer between 2D and 3D").clicked() {
                                            layer.is_3d = !layer.is_3d;
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button(if layer.is_guide_layer { "Remove Guide Layer" } else { "Guide Layer" }).on_hover_text("Guide layers appear in composition preview but are excluded from exports and pre-comps").clicked() {
                                            layer.is_guide_layer = !layer.is_guide_layer;
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("⏪ Time-Reverse Keyframes").on_hover_text("Reverse the order of all animated keyframes on this layer").clicked() {
                                            fn rev<T: Clone>(a: &mut crate::core::property::Animatable<T>, in_f: u32, out_f: u32) {
                                                if let crate::core::property::Animatable::Animated(kfs) = a {
                                                    let span = out_f.saturating_sub(in_f);
                                                    for k in kfs.iter_mut() {
                                                        k.frame = in_f + span.saturating_sub(k.frame.saturating_sub(in_f));
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                }
                                            }
                                            let (inf, outf) = (layer.in_frame, layer.out_frame);
                                            rev(&mut layer.transform.position, inf, outf);
                                            rev(&mut layer.transform.scale, inf, outf);
                                            rev(&mut layer.transform.rotation, inf, outf);
                                            rev(&mut layer.transform.opacity, inf, outf);
                                            project_changed = true;
                                            app.toasts.info("Keyframes time-reversed");
                                            ui.close_menu();
                                        }
                                        if ui.button("⏱ Enable Time Remapping (Cmd+Opt+T)").on_hover_text("Enable variable speed time remapping on this layer").clicked() {
                                            layer.enable_time_remapping();
                                            project_changed = true;
                                            app.toasts.info("Time remapping enabled for layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("Split Layer at Current Time (Cmd+Shift+D)").clicked() {
                                            actions.pending_split_layer = Some(i);
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Into ← Set In-Point to Current Time (I)").clicked() {
                                            layer.in_frame = *current_frame;
                                            if layer.in_frame >= layer.out_frame {
                                                layer.out_frame = layer.in_frame + 1;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("→ Set Out-Point to Current Time (O)").clicked() {
                                            layer.out_frame = *current_frame;
                                            if layer.out_frame <= layer.in_frame {
                                                layer.in_frame = layer.out_frame.saturating_sub(1);
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Add Layer Marker at Playhead").clicked() {
                                            actions.pending_layer_marker = Some(i);
                                            ui.close_menu();
                                        }
                                        if !layer.markers.is_empty()
                                            && ui.button("Clear Layer Markers").clicked() {
                                            actions.pending_clear_markers = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("⬚ Select All Keyframes").on_hover_text("Select every keyframe on this layer (transform, effects, pins)").clicked() {
                                            actions.pending_select_all_kfs = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("Select Label Group").on_hover_text("Select all layers with the same label color").clicked() {
                                            actions.pending_select_label_group = Some(layer.label);
                                            ui.close_menu();
                                        }
                                        // ── Animation Presets ──
                                        ui.menu_button("Animation Presets", |ui| {
                                            let cf = *current_frame;
                                            let mut last_cat = "";
                                            for name in crate::core::presets::NAMES {
                                                let cat = crate::core::presets::category_of(name);
                                                if cat != last_cat && !cat.is_empty() {
                                                    if !last_cat.is_empty() { ui.separator(); }
                                                    ui.label(egui::RichText::new(cat).strong().color(colors::TEXT_SECONDARY));
                                                    last_cat = cat;
                                                }
                                                if ui.small_button(*name).clicked() {
                                                    let ok = crate::core::presets::apply_by_name(name, layer, cf, comp_w_f, 0.0);
                                                    if ok {
                                                        project_changed = true;
                                                        app.toasts.info(format!("{} applied", name));
                                                    } else {
                                                        app.toasts.error(format!("{}: not applicable here", name));
                                                    }
                                                    ui.close_menu();
                                                    ui.close_menu();
                                                }
                                            }
                                            ui.separator();
                                            ui.label(egui::RichText::new("Motion Library (keyframe data)").strong().color(colors::TEXT_SECONDARY));
                                            for preset in crate::core::animation_presets::all_presets() {
                                                if ui.small_button(preset.name.clone()).on_hover_text(preset.description.clone()).clicked() {
                                                    let ok = crate::core::animation_presets::apply_preset_to_layer(&preset, layer, cf);
                                                    if ok {
                                                        project_changed = true;
                                                        app.toasts.info(format!("{} applied", preset.name));
                                                    } else {
                                                        app.toasts.error(format!("{}: not applicable here", preset.name));
                                                    }
                                                    ui.close_menu();
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                        if !layer.paint_strokes.is_empty()
                                            && ui.button("Clear All Paint Strokes").clicked() {
                                            layer.paint_strokes.clear();
                                            project_changed = true;
                                            app.toasts.info("All paint strokes cleared");
                                            ui.close_menu();
                                        }
                                        if ui.button("Pre-Compose Selected... (Cmd+Shift+C)").clicked() {
                                            let selected_indices: Vec<usize> = if !app.selection.selected_layers.is_empty() {
                                                let mut s: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
                                                s.sort();
                                                s
                                            } else {
                                                vec![i]
                                            };
                                            actions.pending_precomp_indices = Some(selected_indices);
                                            ui.close_menu();
                                        }
                                        if ui.button("Reset Transform").clicked() {
                                            layer.transform = crate::core::timeline::Transform2D::default();
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        // ── Audio fades (Audio layers only) ──
                                        if matches!(layer.layer_type, crate::core::timeline::LayerType::Audio { .. }) {
                                            ui.separator();
                                            const FADE_F: u32 = 10;
                                            if ui.button("Add 10-frame Audio Fade In").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let start_f = layer.in_frame;
                                                    let end_f = (start_f + FADE_F).min(layer.out_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(start_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(end_f.max(start_f + 1), base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame > end_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-in added");
                                                ui.close_menu();
                                            }
                                            if ui.button("Add 10-frame Audio Fade Out").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let end_f = layer.out_frame.saturating_sub(1);
                                                    let start_f = end_f.saturating_sub(FADE_F).max(layer.in_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(end_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(start_f, base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame < start_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-out added");
                                                ui.close_menu();
                                            }
                                        }

                                        ui.separator();
                                        let any_disabled = layer.effects.iter().any(|e| !e.enabled);
                                        let fx_label = if any_disabled { "Enable All Effects" } else { "Disable All Effects" };
                                        if ui.button(fx_label).clicked() {
                                            let target = any_disabled;
                                            for fx in layer.effects.iter_mut() {
                                                fx.enabled = target;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                    });
                                    ui.style_mut().visuals.override_text_color = None;

                                    if !app.ui_tabs.show_switches_pane {
                                    ui.scope(|ui| {
                                    let widgets = &mut ui.style_mut().visuals.widgets;
                                    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.hovered.bg_fill = colors::BG_HOVER;
                                    widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                    widgets.hovered.weak_bg_fill = colors::BG_HOVER;
                                    ui.menu_button(egui::RichText::new("モード").small(), |ui| {
                                    // ── Blend Mode Dropdown ──
                                    let bm_text = format!("{:?}", layer.blend_mode);
                                    egui::ComboBox::from_id_salt(format!("tl_blend_{}", i))
                                        .selected_text(format!("Blend: {}", bm_text))
                                        .show_ui(ui, |ui| {
                                            for bm in [
                                                BlendMode::Normal,
                                                BlendMode::Multiply,
                                                BlendMode::Screen,
                                                BlendMode::Overlay,
                                                BlendMode::Add,
                                                BlendMode::Darken,
                                                BlendMode::Lighten,
                                                BlendMode::SoftLight,
                                                BlendMode::HardLight,
                                                BlendMode::Difference,
                                                BlendMode::Exclusion,
                                                BlendMode::Divide,
                                                BlendMode::Subtract,
                                            ] {
                                                if ui.selectable_label(layer.blend_mode == bm, format!("{:?}", bm)).clicked() {
                                                    layer.blend_mode = bm;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Track Matte Dropdown ──
                                    let tm_text = match layer.track_matte {
                                        TrackMatteMode::None => "None",
                                        TrackMatteMode::AlphaMatte => "Alpha",
                                        TrackMatteMode::AlphaMatteInverted => "Alpha Inv",
                                        TrackMatteMode::LumaMatte => "Luma",
                                        TrackMatteMode::LumaMatteInverted => "Luma Inv",
                                    };
                                    egui::ComboBox::from_id_salt(format!("tl_matte_{}", i))
                                        .selected_text(format!("Matte: {}", tm_text))
                                        .show_ui(ui, |ui| {
                                            for (mode, label) in [
                                                (TrackMatteMode::None, "None"),
                                                (TrackMatteMode::AlphaMatte, "Alpha Matte"),
                                                (TrackMatteMode::AlphaMatteInverted, "Alpha Matte Inverted"),
                                                (TrackMatteMode::LumaMatte, "Luma Matte"),
                                                (TrackMatteMode::LumaMatteInverted, "Luma Matte Inverted"),
                                            ] {
                                                if ui.selectable_label(layer.track_matte == mode, label).clicked() {
                                                    layer.track_matte = mode;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Parenting Pick Whip @ & Dropdown ──
                                    let pw_active = app.pick_whip_mode;
                                    let pw_btn = ui.selectable_label(pw_active, "@").on_hover_text("Parenting Pick Whip: Click to enter pick mode, then click target layer");
                                    if pw_btn.clicked() {
                                        app.pick_whip_mode = !app.pick_whip_mode;
                                        app.pick_whip_target = None;
                                        if app.pick_whip_mode {
                                            app.toasts.info("Pick Whip active: click a layer to set as parent");
                                        }
                                    }
                                    let parent_name_resolved = layer.parent_id.as_deref()
                                        .and_then(|pid| derived.parent_indices.get(pid))
                                        .and_then(|index| parent_choices_ref.get(*index))
                                        .map(|(_, name)| name.as_str())
                                        .unwrap_or("None");
                                    let parent_prefix = if layer.is_3d { "Parent (3D)" } else { "Parent" };
                                    egui::ComboBox::from_id_salt(format!("tl_parent_{}", i))
                                        .selected_text(format!("{}: {}", parent_prefix, parent_name_resolved))
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(layer.parent_id.is_none(), "None").clicked() {
                                                layer.parent_id = None;
                                                project_changed = true;
                                            }
                                            for (p_idx, (p_id, p_name)) in parent_choices_ref.iter().enumerate() {
                                                if p_idx != i {
                                                    let is_p = layer.parent_id.as_deref() == Some(p_id);
                                                    if ui.selectable_label(is_p, p_name).clicked() {
                                                        let is_cycle = would_create_parent_cycle(
                                                            &layer.id,
                                                            p_id,
                                                            &derived.parents,
                                                        );

                                                        if is_cycle {
                                                            app.toasts.warning(format!("Cycle prevented! Cannot parent '{}' to '{}'", layer.name, p_name));
                                                        } else {
                                                            layer.parent_id = Some(p_id.clone());
                                                            project_changed = true;
                                                            app.toasts.info(format!("Parented '{}' ➔ '{}'", layer.name, p_name));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    });
                                    });
                                    }
                                });
                            });

                            // Render Layer Bar Span & Waveform + parent link line
                            let avail_w = ui.available_width().min(row_columns.time_width).max(0.0);
                            let (bar_rect, _bar_sense) = ui.allocate_exact_size(
                                egui::vec2(avail_w, layout::LAYER_ROW_HEIGHT),
                                egui::Sense::hover().union(egui::Sense::drag()),
                            );
                            // Draw parent connection if this layer has a parent
                            if let Some(pid) = &layer.parent_id {
                                if derived.parent_indices.contains_key(pid) {
                                    // Small chain icon on the left of the bar
                                     ui.painter().text(
                                         egui::pos2(bar_rect.left() + 8.0, bar_rect.top() + 4.0),
                                         egui::Align2::LEFT_TOP,
                                         "↔",
                                         egui::FontId::proportional(10.0),
                                         colors::ACCENT_PURPLE,
                                     );
                                }
                            }

                            let norm_in = (layer.in_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                            let norm_out = (layer.out_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;

                            let layer_rect = egui::Rect::from_min_max(
                                egui::pos2(bar_rect.left() + norm_in * bar_rect.width(), bar_rect.top() + 4.0),
                                egui::pos2(bar_rect.left() + norm_out * bar_rect.width(), bar_rect.bottom() - 4.0),
                            );

                            let label_fill = if app.selection.selected_layer_idx == Some(i) {
                                // Selected: label color at reduced gain, never neon.
                                let [r, g, b] = layer.label.to_rgb();
                                egui::Color32::from_rgb(
                                    (r * 170.0) as u8,
                                    (g * 170.0) as u8,
                                    (b * 170.0) as u8,
                                )
                            } else {
                                // Muted toward the panel background unless selected.
                                let [r, g, b] = layer.label.to_rgb();
                                egui::Color32::from_rgb(
                                    (r * 115.0 + 14.0) as u8,
                                    (g * 115.0 + 16.0) as u8,
                                    (b * 115.0 + 19.0) as u8,
                                )
                            };
                            let fill_c = match &layer.layer_type {
                                LayerType::Solid { color } => egui::Color32::from_rgba_unmultiplied(
                                    (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                                    (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                                    (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                                    (color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                                ),
                                _ => label_fill,
                            };

                            let has_rendered_thumbnail =
                                layer_has_timeline_preview(&layer.layer_type);
                            let bar_fill = if has_rendered_thumbnail {
                                egui::Color32::from_rgb(17, 20, 25)
                            } else {
                                fill_c
                            };
                            ui.painter().rect_filled(layer_rect, 2.0, bar_fill);
                            if has_rendered_thumbnail {
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_max(
                                        layer_rect.left_top(),
                                        egui::pos2(
                                            (layer_rect.left() + 3.0).min(layer_rect.right()),
                                            layer_rect.bottom(),
                                        ),
                                    ),
                                    1.0,
                                    fill_c,
                                );
                            }
                            let preview_painter = ui.painter().with_clip_rect(layer_rect.shrink(1.0));
                            if layer_rect.width() > 46.0 {
                                if let Some((preview_start, preview_end, textures)) = &layer_preview_textures {
                                    let preview_left = layer_rect.left() + layer_rect.width() * preview_start;
                                    let preview_right = layer_rect.left() + layer_rect.width() * preview_end;
                                    let preview_width = (preview_right - preview_left).max(0.0);
                                    let tile_width = preview_width / textures.len().max(1) as f32;
                                    for (tile_index, texture) in textures.iter().enumerate() {
                                        let x = preview_left + tile_index as f32 * tile_width;
                                        let tile_right = if tile_index + 1 == textures.len() {
                                            preview_right
                                        } else {
                                            x + tile_width + 0.5
                                        };
                                        if let Some(texture) =
                                            texture.as_ref().filter(|_| tile_width > 0.0)
                                        {
                                            let tile = egui::Rect::from_min_max(
                                                egui::pos2(x, layer_rect.top()),
                                                egui::pos2(tile_right.min(layer_rect.right()), layer_rect.bottom()),
                                            );
                                            let source_aspect = comp_w_f / comp_h_f;
                                            let tile_aspect = tile.width() / tile.height().max(1.0);
                                            let uv = if source_aspect > tile_aspect {
                                                let visible = tile_aspect / source_aspect.max(0.01);
                                                egui::Rect::from_min_max(
                                                    egui::pos2((1.0 - visible) * 0.5, 0.0),
                                                    egui::pos2(1.0 - (1.0 - visible) * 0.5, 1.0),
                                                )
                                            } else {
                                                let visible = source_aspect / tile_aspect.max(0.01);
                                                egui::Rect::from_min_max(
                                                    egui::pos2(0.0, (1.0 - visible) * 0.5),
                                                    egui::pos2(1.0, 1.0 - (1.0 - visible) * 0.5),
                                                )
                                            };
                                            preview_painter.image(*texture, tile, uv, egui::Color32::WHITE);
                                        }
                                    }
                                }
                            }
                            if layer_rect.width() > 72.0 {
                                let label = layer.name.as_str();
                                let label_galley = ui.painter().layout_no_wrap(
                                    label.to_owned(),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::WHITE,
                                );
                                let label_rect = egui::Rect::from_min_size(
                                    egui::pos2(layer_rect.left() + 2.0, layer_rect.top() + 1.0),
                                    egui::vec2(
                                        (label_galley.size().x + 10.0)
                                            .min(layer_rect.width() - 4.0),
                                        (label_galley.size().y + 4.0)
                                            .min(layer_rect.height() - 2.0)
                                            .max(1.0),
                                    ),
                                );
                                preview_painter.rect_filled(label_rect, 1.0, egui::Color32::from_black_alpha(92));
                                preview_painter.with_clip_rect(label_rect).text(
                                    label_rect.left_center() + egui::vec2(4.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    label,
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::WHITE,
                                );
                            }
                            ui.painter().rect_stroke(layer_rect, 2.0, egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY));

                            // ── Audio waveform overlay inside the bar ──
                            let audio_path: Option<String> = match &layer.layer_type {
                                crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                                crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
                                _ => None,
                            };
                            if let Some(apath) = audio_path {
                                if let Some(waveform) =
                                    waveform_cache::get(ui.ctx(), std::path::Path::new(&apath))
                                {
                                    draw_layer_waveform(
                                        &ui.painter().with_clip_rect(layer_rect),
                                        LayerWaveformParams {
                                            timeline_rect: bar_rect,
                                            clip_rect: layer_rect,
                                            layer,
                                            waveform: &waveform,
                                            start_frame,
                                            visible_span: zoom_span,
                                            fps: comp.fps,
                                            color: colors::TIMELINE_WAVEFORM,
                                        },
                                    );
                                }
                            }

                            // ── Trim handles: drag bar edges to set in/out points ──
                            fn handle_rect_of(is_in: bool, lr: &egui::Rect) -> egui::Rect {
                                const HW: f32 = 6.0;
                                if is_in {
                                    egui::Rect::from_min_size(egui::pos2(lr.left() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                } else {
                                    egui::Rect::from_min_size(egui::pos2(lr.right() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                }
                            }
                            const HANDLE_W: f32 = 6.0;
                            let in_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.left() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let out_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.right() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let in_resp = ui.interact(in_handle, egui::Id::new(("trim_in", i)), egui::Sense::click_and_drag());
                            let out_resp = ui.interact(out_handle, egui::Id::new(("trim_out", i)), egui::Sense::click_and_drag());

                            // ── Body drag: slide the whole layer (in/out preserved) ──
                            {
                                const HEADER_W: f32 = 6.0;
                                let body_rect = egui::Rect::from_min_max(
                                    egui::pos2(layer_rect.left() + HEADER_W, layer_rect.top()),
                                    egui::pos2(layer_rect.right() - HEADER_W, layer_rect.bottom()),
                                );
                                let body_resp = ui.interact(
                                    body_rect,
                                    egui::Id::new(("layer_slide", i, layer.id.as_str())),
                                    egui::Sense::drag(),
                                );
                                let body_drag_id = egui::Id::new(("layer_slide_state", layer.id.as_str()));
                                if body_resp.drag_started() {
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(body_drag_id, LayerBodyDragState::default());
                                    });
                                }
                                if body_resp.dragged() {
                                    let total_frame_offset =
                                        (body_resp.drag_delta().x / bar_rect.width() * zoom_span as f32).round() as i32;
                                    let mut drag_state = ui.ctx().data(|data| {
                                        data.get_temp::<LayerBodyDragState>(body_drag_id)
                                            .unwrap_or_default()
                                    });
                                    let delta_frames = total_frame_offset - drag_state.frame_offset;
                                    drag_state.frame_offset = total_frame_offset;
                                    // ── Vertical drag → reorder layers in stack ──
                                    let row_h = layout::LAYER_ROW_HEIGHT;
                                    let total_y = body_resp.drag_delta().y;
                                    let incremental_y = total_y - drag_state.previous_y;
                                    drag_state.previous_y = total_y;
                                    let new_accum = drag_state.y_remainder + incremental_y;
                                    let row_offset = (new_accum / row_h).trunc() as i32;
                                    if row_offset != 0 {
                                        let target = (i as i32 - row_offset).clamp(0, layers_len as i32 - 1) as usize;
                                        if target != i {
                                            actions.swap_request = Some((i, target));
                                        }
                                        drag_state.y_remainder = new_accum - row_offset as f32 * row_h;
                                    } else {
                                        drag_state.y_remainder = new_accum;
                                    }
                                    ui.ctx().data_mut(|data| data.insert_temp(body_drag_id, drag_state));
                                    if delta_frames != 0 {
                                        let alt_held = ui.input(|inp| inp.modifiers.alt);
                                        if alt_held {
                                            // ── Slip edit: bar stays, content timing shifts ──
                                            // Keyframes & markers move opposite the drag so a
                                            // different moment of the source plays at the same time.
                                            let shift = -delta_frames;
                                            for kf in layer.transform.position.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.position.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.scale.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.scale.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.rotation.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.rotation.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.opacity.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.opacity.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for m in layer.markers.iter_mut() {
                                                m.frame = (m.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            // Slip effect keyframes too
                                            use crate::core::effect_params::ParamRef;
                                            for effect in layer.effects.iter_mut() {
                                                for (_, param) in effect.effect_type.animatable_params() {
                                                    match param {
                                                        ParamRef::Scalar(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec2(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec3(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec4Color(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // Slide: move the whole bar
                                            let span = layer.out_frame - layer.in_frame;
                                            let raw_in = (layer.in_frame as i32 + delta_frames)
                                                .clamp(0, (total_frames - span) as i32)
                                                as u32;
                                            let new_in = if app.snap_to_keyframes {
                                                crate::ui::timeline::snapping::snap_to_sorted_layer_edges(
                                                    raw_in,
                                                    i,
                                                    layer_edges,
                                                )
                                            } else {
                                                raw_in
                                            };
                                            layer.in_frame = new_in;
                                            layer.out_frame = new_in + span;
                                        }
                                        project_changed = true;
                                    }
                                }
                                if body_resp.drag_stopped() {
                                    ui.ctx().data_mut(|data| data.remove::<LayerBodyDragState>(body_drag_id));
                                }
                                if body_resp.hovered() && !in_resp.hovered() && !out_resp.hovered() {
                                    let alt_held = ui.input(|inp| inp.modifiers.alt);
                                    ui.ctx().set_cursor_icon(if alt_held { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::Grab });
                                    body_resp.on_hover_text(format!(
                                        "{} — In: {} Out: {} ({:.2}s)",
                                        layer.name,
                                        layer.in_frame,
                                        layer.out_frame,
                                        layer.duration_frames() as f32 / comp.fps.max(1) as f32
                                    ));
                                }
                            }

                            for (resp, is_in) in [(&in_resp, true), (&out_resp, false)] {
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                    ui.painter().rect_filled(handle_rect_of(is_in, &layer_rect), 2.0, colors::HANDLE_HOVER_FILL.linear_multiply(180.0 / 255.0));
                                }
                                if resp.dragged() {
                                    if let Some(pos) = resp.interact_pointer_pos() {
                                        let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                                        let frame = (start_frame as f32 + norm * zoom_span as f32).round() as u32;
                                        let frame = frame.min(total_frames);
                                        if is_in {
                                            layer.in_frame = frame.min(layer.out_frame.saturating_sub(1));
                                        } else {
                                            let old_out = layer.out_frame;
                                            layer.out_frame = frame.max(layer.in_frame + 1);
                                            // ── Shift+drag Out = ripple edit: close gaps for layers below ──
                                            if ui.input(|inp| inp.modifiers.shift) && layer.out_frame != old_out {
                                                let shift = layer.out_frame as i64 - old_out as i64;
                                                actions.pending_ripple = Some((i, old_out, shift));
                                            }
                                        }
                                        project_changed = true;
                                    }
                                }
                            }

                            // ── Layer markers: small triangles on the bar (AE parity) ──
                            for marker in &layer.markers {
                                if marker.frame < start_frame || marker.frame > start_frame + zoom_span { continue; }
                                let norm = (marker.frame - start_frame) as f32 / zoom_span as f32;
                                let mx = layer_rect.left() + norm * layer_rect.width();
                                let my = layer_rect.top() + 1.0;
                                let [r, g, b] = marker.color;
                                let mcol = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                                let pts = vec![
                                    egui::pos2(mx, my + 5.0),
                                    egui::pos2(mx - 4.0, my),
                                    egui::pos2(mx + 4.0, my),
                                ];
                                ui.painter().add(egui::Shape::convex_polygon(pts, mcol, egui::Stroke::NONE));
                            }

                            // ── Real waveform for video layers with audio ──
                            if let LayerType::Video { audio_wav: Some(wav_path), .. } = &layer.layer_type {
                                if let Some(waveform) = waveform_cache::get(
                                    ui.ctx(),
                                    std::path::Path::new(wav_path),
                                ) {
                                    draw_layer_waveform(
                                        &ui.painter().with_clip_rect(layer_rect),
                                        LayerWaveformParams {
                                            timeline_rect: bar_rect,
                                            clip_rect: layer_rect,
                                            layer,
                                            waveform: &waveform,
                                            start_frame,
                                            visible_span: zoom_span,
                                            fps: comp.fps,
                                            color: colors::TIMELINE_WAVEFORM,
                                        },
                                    );
                                }
                            }

                            if let LayerType::Audio { path, .. } = &layer.layer_type {
                                if let Some(waveform) = waveform_cache::get(
                                    ui.ctx(),
                                    std::path::Path::new(path),
                                ) {
                                    draw_layer_waveform(
                                        &ui.painter().with_clip_rect(layer_rect),
                                        LayerWaveformParams {
                                            timeline_rect: bar_rect,
                                            clip_rect: layer_rect,
                                            layer,
                                            waveform: &waveform,
                                            start_frame,
                                            visible_span: zoom_span,
                                            fps: comp.fps,
                                            color: colors::TIMELINE_WAVEFORM,
                                        },
                                    );
                                }
                            }

                            let playhead_norm =
                                (*current_frame as f32 - start_frame as f32) / zoom_span as f32;
                            if (0.0..=1.0).contains(&playhead_norm) {
                                let playhead_x =
                                    bar_rect.left() + playhead_norm * bar_rect.width();
                                ui.painter()
                                    .with_clip_rect(bar_rect)
                                    .line_segment(
                                        [
                                            egui::pos2(playhead_x, bar_rect.top()),
                                            egui::pos2(playhead_x, bar_rect.bottom()),
                                        ],
                                        egui::Stroke::new(
                                            1.5_f32,
                                            colors::TIMELINE_PLAYHEAD,
                                        ),
                                    );
                            }
                        });

                        // ── Expanded waveform lane ──
                        if app.expanded_waveform_layers.contains(&i) {
                            let audio_path: Option<String> = match &layer.layer_type {
                                crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                                crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
                                _ => None,
                            };
                            if let Some(apath) = audio_path {
                                let wf_height = 48.0;
                                let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), wf_height), egui::Sense::hover());
                                let wf_rect = response.rect;
                                painter.rect_filled(wf_rect, 0.0, egui::Color32::from_rgba_premultiplied(20, 20, 25, 255));
                                if let Some(waveform) =
                                    waveform_cache::get(ui.ctx(), std::path::Path::new(&apath))
                                {
                                    let timeline_rect = egui::Rect::from_min_max(
                                        egui::pos2(wf_rect.left() + left_pane_w, wf_rect.top()),
                                        wf_rect.right_bottom(),
                                    );
                                    let start_norm =
                                        layer.in_frame.saturating_sub(start_frame) as f32
                                            / zoom_span.max(1) as f32;
                                    let end_norm =
                                        layer.out_frame.saturating_sub(start_frame) as f32
                                            / zoom_span.max(1) as f32;
                                    let clip_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            timeline_rect.left() + start_norm * timeline_rect.width(),
                                            wf_rect.top(),
                                        ),
                                        egui::pos2(
                                            timeline_rect.left() + end_norm * timeline_rect.width(),
                                            wf_rect.bottom(),
                                        ),
                                    );
                                    draw_layer_waveform(
                                        &painter.with_clip_rect(clip_rect),
                                        LayerWaveformParams {
                                            timeline_rect,
                                            clip_rect,
                                            layer,
                                            waveform: &waveform,
                                            start_frame,
                                            visible_span: zoom_span,
                                            fps: comp.fps,
                                            color: colors::TIMELINE_WAVEFORM,
                                        },
                                    );
                                }
                            }
                        }

                        // If expanded, render transform properties & effects
                        if app.selection.expanded_layers.contains(&i) {
                            crate::ui::timeline::keyframe_rows::draw_expanded_rows(
                                ui,
                                crate::ui::timeline::keyframe_rows::ExpandedRowsState {
                                    layer,
                                    layer_index: i,
                                    selected_keyframes: &mut app.selected_keyframes,
                                    selected_property: &mut app.selection.selected_property,
                                    current_frame,
                                    start_frame,
                                    zoom_span,
                                    left_pane_w,
                                    all_kf_frames,
                                    precomp_children: &precomp_children,
                                    open_comp_request: &mut actions.pending_open_comp,
                                    project_changed: &mut project_changed,
                                },
                            );
                        }
                    }
                }
            });

            crate::ui::timeline::pending_actions::apply_deferred(
                app,
                ui,
                *current_frame,
                &mut project_changed,
                &mut actions,
            );

            if project_changed {
                // Transactional undo commit: snapshot on pointer-down, single entry on release
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if is_pointer_down {
                    let edit_label = if app.show_graph_editor { "Graph Editor Edit" } else { "Timeline Edit" };
                    if let Some(snapshot) = pre_edit_snapshot {
                        app.begin_drag_with_snapshot(snapshot, edit_label);
                    } else if !app.drag_active() {
                        app.begin_drag(edit_label);
                    }
                } else {
                    app.commit_drag();
                }
                crate::core::frame_cache::bump_version();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::state::would_create_parent_cycle;
    use std::collections::HashMap;

    #[test]
    fn parent_pick_whip_rejects_descendant_and_self() {
        let parents = HashMap::from([
            ("child".to_string(), "middle".to_string()),
            ("middle".to_string(), "root".to_string()),
        ]);

        assert!(would_create_parent_cycle("root", "child", &parents));
        assert!(would_create_parent_cycle("middle", "middle", &parents));
        assert!(!would_create_parent_cycle("child", "root", &parents));
    }
}
