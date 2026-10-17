use crate::core::property::Animatable;
use crate::core::roto_assist::trace_contour_to_polygon;
use crate::core::roto_brush_engine::{
    generate_rotobrush_matte, RotoBrushSettings, RotoStroke as EngineRotoStroke, RotoStrokeType,
};

use crate::ui::theme::colors;
use crate::KagariApp;
use crate::ViewportMode;
use eframe::egui;

mod mask_tools;
mod painting;
mod pen_preview;
mod puppet_pin;
use mask_tools::*;
mod roto_brush;
use roto_brush::*;
mod inline_text_editor;
mod interaction;
mod navigation;
mod toolbar;
mod view_menu;
use inline_text_editor::{draw_inline_text_editor, InlineTextEditorGeometry};

#[derive(Clone, Copy)]
struct ViewportGeometry {
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: u32) {
    poll_roto_flow_bake(app, ctx);
    poll_roto_segment_job(app, ctx);
    // Only clone the project to the production document when history has actually
    // changed. The generation counter avoids a full deep clone every frame.
    let hist_gen = app.history.generation();
    if let Some(document) = app.production_document.as_mut() {
        if app.doc_sync_generation != hist_gen {
            document.project = app.history.current().clone();
            app.doc_sync_generation = hist_gen;
        }
    }
    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Composition / Layer / Footage tab strip ───────────────────────────────
        let paint_preview_required = app
            .history
            .current()
            .active_composition()
            .layers
            .iter()
            .any(|layer| !layer.paint_strokes.is_empty());
        let source_map_effect_preview_required = app
            .history
            .current()
            .active_composition()
            .layers
            .iter()
            .any(|layer| {
                layer.effects.iter().any(|effect| {
                    matches!(
                        &effect.effect_type,
                        crate::core::timeline::EffectType::DisplacementMap { .. }
                            | crate::core::timeline::EffectType::CompoundBlur { .. }
                    )
            })
            });
        let frame_blending_required = app
            .history
            .current()
            .active_composition()
            .layers
            .iter()
            .any(|layer| {
                layer.frame_blending
                    && matches!(layer.layer_type, crate::core::timeline::LayerType::Video { .. })
            });
        toolbar::draw_viewport_toolbar(app, ui);
        crate::ui::theme::draw_separator(ui);

        // ── Comp Settings Modal ──────────────────────────────────────────────
        crate::ui::comp_settings_dialog::draw_comp_settings_dialog(app, ctx);

        // ── AE Viewport Composition Top Tabs & Breadcrumbs Bar ──
        if app.history.current().active_composition().id != "comp_empty_start"
            && app.history.current().compositions.len() > 1
        {
            let active_comp_idx = app.history.current().active_composition_idx;
            let comps_count = app.history.current().compositions.len();
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), crate::ui::theme::layout::PANEL_HEADER_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("viewer_composition_tabs")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for idx in 0..comps_count {
                                    let is_active = idx == active_comp_idx;
                                    let c_name = app.history.current().compositions[idx].name.clone();
                                    if crate::ui::theme::draw_custom_tab(ui, is_active, &c_name).clicked() && idx != active_comp_idx {
                                        app.history.current_mut().active_composition_idx = idx;
                                        crate::core::frame_cache::bump_version();
                                    }
                                }
                            });
                        });
                },
            );
            crate::ui::theme::draw_separator(ui);
        }

        let size = ui.available_size();
        let bottom_transport_h = 36.0_f32;
        let canvas_h = (size.y - bottom_transport_h).max(60.0);

        // Canvas Area
        let (rect, mut viewport_response) = ui.allocate_exact_size(
            egui::vec2(size.x, canvas_h),
            egui::Sense::click_and_drag(),
        );

        // Floating Modern Tool Strip (Glassmorphism Pill directly inside viewer top-left)
        let tool_rect = {
            let tool_strip_w = 34.0_f32;
            let tool_strip_h = 240.0_f32;
            let float_margin = 12.0_f32;
            let tool_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(float_margin, float_margin),
                egui::vec2(tool_strip_w, tool_strip_h),
            );
            ui.painter().rect_filled(
                tool_rect,
                8.0,
                egui::Color32::from_rgba_premultiplied(16, 18, 24, 230),
            );
            ui.painter().rect_stroke(
                tool_rect,
                8.0,
                egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            );
            let mut tool_ui = ui.new_child(egui::UiBuilder::new().max_rect(tool_rect));
            crate::ui::toolbar::draw_viewer_tool_strip_vertical(app, &mut tool_ui);
            tool_rect
        };

        // If pointer is inside the floating tool strip, do not allow canvas viewport_response
        // to steal or trigger pan/zoom/selection clicks.
        let pointer_over_tool_strip = ui.input(|i| {
            i.pointer.hover_pos().is_some_and(|pos| tool_rect.contains(pos))
                || i.pointer.interact_pos().is_some_and(|pos| tool_rect.contains(pos))
        });
        if pointer_over_tool_strip {
            viewport_response.clicked = false;
            viewport_response.dragged = false;
        }
        viewport_response.context_menu(|ui| {
            let layer_count = app.history.current().active_composition().layers.len();
            if ui.button("Select All Layers").clicked() {
                app.selection.selected_layers = (0..layer_count).collect();
                app.selection.selected_layer_idx = (layer_count > 0).then_some(0);
                ui.close_menu();
            }
            if ui.button("Deselect Layers").clicked() {
                app.selection.selected_layers.clear();
                app.selection.selected_layer_idx = None;
                app.mask_selected_vertices = None;
                ui.close_menu();
            }
        ui.separator();
            if ui.button("Fit Composition to Viewer").clicked() {
                app.ui_tabs.viewport_mag_ratio = 0.0;
                app.playback.viewport_pan = egui::Vec2::ZERO;
                ui.close_menu();
            }
            ui.checkbox(&mut app.show_grid, "Show Grid");
            ui.checkbox(&mut app.show_guides, "Show Guides");
            ui.checkbox(&mut app.snap_to_keyframes, "Snap to Keyframes");
        });

        navigation::handle_navigation(app, ui, &viewport_response, rect);

        // Render background
        ui.painter().rect_filled(rect, 4.0, colors::BG_DEEPEST);
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );

        let active_comp_idx = app.history.current().active_composition_idx;
        let binding_snapshot = app
            .production_document
            .as_ref()
            .filter(|document| !document.bindings.is_empty())
            .and_then(|document| {
                let live_audio = crate::core::expression_engine::audio_binding_source_values();
                document.composition_for_frame_with_sources(active_comp_idx, current_frame, &live_audio)
            });
        let history_comp = app.history.current().active_composition();
        let comp = binding_snapshot.as_ref().unwrap_or(history_comp);
        let comp_fps = comp.fps.max(1);
        let comp_duration_frames = comp.duration_frames.max(1);
        let comp_is_empty = comp.layers.is_empty();
        let missing_media = comp.layers.iter().filter(|layer| layer.is_active(current_frame)).find_map(|layer| {
            let path = match &layer.layer_type {
                crate::core::timeline::LayerType::Image { path } => Some(path),
                crate::core::timeline::LayerType::Model3D { path } => Some(path),
                crate::core::timeline::LayerType::Video { source, .. } => Some(source),
                _ => None,
            }?;
            // If it's a video layer and frames_dir exists or source exists, it's not missing.
            // Use revision cache / fast check instead of uncached disk syscall
            let is_file = if matches!(layer.layer_type, crate::core::timeline::LayerType::Video { .. }) {
                std::path::Path::new(path).exists()
            } else {
                std::path::Path::new(path).is_file()
            };
            if is_file { None } else { Some(layer.name.clone()) }
        });
        let software_preview_required =
            crate::ui::viewport_rendering::software_preview_required(comp, current_frame);
        let aspect = comp.width as f32 / comp.height as f32;

        // ── One-shot bbox focus request (Shift+Z with selection) ──
        if let Some((bmin, bmax)) = app.viewport_focus_bbox.take() {
            let cw = comp.width as f32;
            let ch = comp.height as f32;
            let bw = (bmax[0] - bmin[0]).max(1.0);
            let bh = (bmax[1] - bmin[1]).max(1.0);
            // Base fit layout at mag 1.0 to obtain comp->pixel scale.
            let (_, _, bw_px, _) = crate::ui::viewport_state::compute_draw_layout_pan(rect, aspect, 1.0, egui::Vec2::ZERO);
            let s = (bw_px / cw).max(f32::EPSILON);
            let m = (((rect.width() * 0.85) / (bw * s)).min((rect.height() * 0.85) / (bh * s))).clamp(0.05, 8.0);
            let (fx, fy, fw_px, fh_px) =
                crate::ui::viewport_state::compute_draw_layout_pan(rect, aspect, m, egui::Vec2::ZERO);
            let cx = (bmin[0] + bmax[0]) * 0.5;
            let cy = (bmin[1] + bmax[1]) * 0.5;
            app.ui_tabs.viewport_mag_ratio = m;
            app.playback.viewport_pan = egui::vec2(
                rect.center().x - (fx + cx * (fw_px / cw)),
                rect.center().y - (fy + cy * (fh_px / ch)),
            );
        }

        let (origin_x, origin_y, draw_w, draw_h) =
            crate::ui::viewport_state::compute_draw_layout_pan(rect, aspect, app.ui_tabs.viewport_mag_ratio, app.playback.viewport_pan);
        let draw_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x, origin_y),
            egui::vec2(draw_w, draw_h),
        );
        app.viewer_draw_rect = Some(draw_rect);

        // ── Checkerboard transparency grid behind comp canvas (Batched single Mesh) ──
        if !comp_is_empty {
            let cell = 20.0f32;
            // Keep transparency legible without competing with the footage.
            // A bright checkerboard makes Fit mode read like a debug canvas.
            let light = egui::Color32::from_rgb(48, 59, 70);
            let dark = egui::Color32::from_rgb(32, 42, 52);
            let cols = (draw_w / cell).ceil() as i32;
            let rows = (draw_h / cell).ceil() as i32;
            let p = ui.painter();
            // Single base rectangle for light squares
            p.rect_filled(draw_rect, 0.0, light);

            let mut mesh = egui::Mesh::default();
            for r in 0..rows {
                for c in 0..cols {
                    if (r + c) % 2 != 0 {
                        let x = draw_rect.left() + c as f32 * cell;
                        let y = draw_rect.top() + r as f32 * cell;
                        let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell, cell)).intersect(draw_rect);
                        if cell_rect.is_positive() {
                            mesh.add_colored_rect(cell_rect, dark);
                        }
                    }
                }
            }
            if !mesh.is_empty() {
                p.add(mesh);
            }
        }

        let comp_w = comp.width as f32;
        let comp_h = comp.height as f32;

        let snap_frame_opt = crate::ui::viewport_state::snap_frame(ctx);
        let is_comparing = crate::ui::viewport_state::is_comparing(ctx);
        let wipe_pos = crate::ui::viewport_state::wipe_pos(ctx);

        // -- GPU Render Path --
        #[allow(unused_mut)]
        let mut rendered_gpu = false;
        #[cfg(feature = "wgpu")]
        if app.color_lut_mode != 3
            && !paint_preview_required
            && !source_map_effect_preview_required
            && !frame_blending_required
            && !software_preview_required
            && !comp_is_empty
        {
            if let Some(renderer) = &mut app.renderer {
            if let Some(wgpu_state) = &app.wgpu_state {
                let exp_id = egui::Id::new("ae_exposure_ev");
                let exposure_ev = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(exp_id, || 0.0f32));
                let lut_idx = app.color_lut_mode as usize;

                let snap_id = egui::Id::new(crate::ui::viewport_state::SNAP_FRAME_ID);
                let is_comparing_id = egui::Id::new(crate::ui::viewport_state::IS_COMPARING_ID);
                let wipe_id = egui::Id::new(crate::ui::viewport_state::WIPE_POS_ID);

                let snap_frame_opt = ctx.data_mut(|d| d.get_temp::<u32>(snap_id));
                let is_comparing = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_comparing_id, || false));
                let wipe_pos = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(wipe_id, || 0.5f32));

                // Render at display resolution: a 4K comp shown in an 800px
                // viewport renders at ~800px wide (4-16x less fill rate).
                // While playing, the adaptive factor further reduces resolution when
                // frames exceed the playback budget (AE-style auto quality drop).
                // While the user is mid-drag (transform/mask/pin handles) we cap at
                // half resolution so heavy comps stay interactive; idle frames
                // snap back to full via the drift logic below.
                let dragging_now = ui.input(|i| i.pointer.any_down())
                    && (app.viewport_drag_state.is_some()
                        || app.viewport_multi_drag.is_some()
                        || app.viewport_mask_drag_state.is_some()
                        || app.viewport_pos_kf_drag_state.is_some()
                        || app.viewport_scale_drag.is_some());
                let effective_factor = if dragging_now {
                    app.playback.adaptive_preview_factor.min(0.5)
                } else {
                    app.playback.adaptive_preview_factor
                };
                let proxy_factor = crate::core::proxy::composition_preview_scale(comp);
                let display_px = (draw_w * ctx.pixels_per_point()).ceil();
                let preview_px = ((display_px * effective_factor * proxy_factor) as u32)
                    .clamp(64, 4096);
                renderer.set_preview_max_width(Some(preview_px));

                // ── RAM preview: incremental pre-pass, a few frames per UI frame ──
                // Rendering everything up front would freeze the UI on heavy comps;
                // instead we chip away at the work area while playback runs.
                const RAM_PREPASS_FRAMES_PER_TICK: u32 = 2;
                const RAM_PREPASS_MAX_FRAMES: u32 = 300;
                const RAM_PREVIEW_BUDGET_BYTES: u64 = 256 * 1024 * 1024;
                if app.playback.is_playing && !app.was_playing_last_frame {
                    let last_frame = comp.duration_frames.saturating_sub(1);
                    let wa_in = app.playback.work_area_in.unwrap_or(0).min(last_frame);
                    let wa_out = app
                        .playback.work_area_out
                        .unwrap_or(last_frame)
                        .min(last_frame)
                        .max(wa_in);
                    if app.viewport_texture_id.is_some_and(|current| {
                        app.ram_texture_ids.iter().any(|(_, id)| *id == current)
                    }) {
                        app.viewport_texture_id = None;
                    }
                    for (_, id) in app.ram_texture_ids.drain(..) {
                        wgpu_state.renderer.write().free_texture(&id);
                    }
                    let frame_width = comp.width.min(preview_px).max(1);
                    let frame_height = ((comp.height as u64 * frame_width as u64)
                        / comp.width.max(1) as u64)
                        .clamp(1, u32::MAX as u64) as u32;
                    let bytes_per_frame = frame_width as u64 * frame_height as u64 * 8;
                    let memory_limited_frames = (RAM_PREVIEW_BUDGET_BYTES / bytes_per_frame.max(1))
                        .clamp(1, RAM_PREPASS_MAX_FRAMES as u64)
                        as u32;
                    if comp.duration_frames == 0 {
                        app.ram_prepass_cursor = 1;
                        app.ram_prepass_end = 0;
                    } else {
                        app.ram_prepass_cursor = wa_in;
                        app.ram_prepass_end = wa_in.saturating_add(
                            wa_out.saturating_sub(wa_in).min(memory_limited_frames - 1),
                        );
                    }
                }
                // During playback, check if current frame is already cached in RAM preview
                let ram_id = if app.playback.is_playing {
                    app.ram_texture_ids
                        .iter()
                        .find(|(f, _)| *f == current_frame)
                        .map(|(_, id)| *id)
                } else {
                    None
                };

                // Only run background RAM prepass if the current frame is already cached
                // (which means live GPU render is skipped and we have spare frame budget).
                // If the current frame was NOT in cache, do NOT render extra prepass frames
                // on the same UI frame, avoiding frame budget overruns that degrade playback.
                if app.playback.is_playing && ram_id.is_some() && app.ram_prepass_cursor <= app.ram_prepass_end {
                    let batch_end = app
                        .ram_prepass_cursor
                        .saturating_add(RAM_PREPASS_FRAMES_PER_TICK - 1);
                    let batch_end = batch_end.min(app.ram_prepass_end);
                    renderer.render_ram_preview_range(
                        comp,
                        app.ram_prepass_cursor,
                        batch_end,
                        exposure_ev,
                        lut_idx as u32,
                        RAM_PREPASS_MAX_FRAMES,
                    );
                    for f in app.ram_prepass_cursor..=batch_end {
                        if !app.ram_texture_ids.iter().any(|(frame, _)| *frame == f) {
                            if let Some(view) = renderer.ram_frame_view(f) {
                                let id = wgpu_state.renderer.write().register_native_texture(
                                    &wgpu_state.device,
                                    view,
                                    wgpu::FilterMode::Linear,
                                );
                                app.ram_texture_ids.push((f, id));
                            }
                        }
                    }
                    app.ram_prepass_cursor = batch_end.saturating_add(1);

                    // Prune ancient RAM preview textures (keep at most 90 frames)
                    const MAX_RAM_CACHE_ENTRIES: usize = 90;
                    if app.ram_texture_ids.len() > MAX_RAM_CACHE_ENTRIES {
                        let excess = app.ram_texture_ids.len() - MAX_RAM_CACHE_ENTRIES;
                        let evicted: Vec<_> = app.ram_texture_ids.drain(0..excess).collect();
                        for (_, id) in evicted {
                            if app.viewport_texture_id != Some(id) {
                                wgpu_state.renderer.write().free_texture(&id);
                            }
                        }
                    }
                }
                if !app.playback.is_playing && app.was_playing_last_frame {
                    // Playback stopped: free ring textures and egui ids
                    if app.viewport_texture_id.is_some_and(|current| {
                        app.ram_texture_ids.iter().any(|(_, id)| *id == current)
                    }) {
                        app.viewport_texture_id = None;
                    }
                    for (_, id) in app.ram_texture_ids.drain(..) {
                        wgpu_state.renderer.write().free_texture(&id);
                    }
                    renderer.clear_ram_preview();
                }
                app.was_playing_last_frame = app.playback.is_playing;

                let render_started = std::time::Instant::now();
                if let Some(id) = ram_id {
                    // Cached frame: swap the displayed texture, no GPU re-render
                    if app.viewport_texture_id != Some(id) {
                        if let Some(old_id) = app.viewport_texture_id {
                            if !app.ram_texture_ids.iter().any(|(_, cached)| *cached == old_id) {
                                wgpu_state.renderer.write().free_texture(&old_id);
                            }
                        }
                        app.viewport_texture_id = Some(id);
                    }
                    rendered_gpu = true;
                    app.gpu_rendered = true;
                }
                let (texture_view, recreated) = if ram_id.is_some() {
                    // Skip live rendering entirely this frame
                    (None, false)
                } else {
                    // Shadow map upload for GPU preview parity (CPU Phase 2.72)
                    if comp.lights.iter().any(|l| l.casts_shadows && l.intensity > 0.0) {
                        let map = crate::core::software_renderer::build_shadow_map(
                            comp, current_frame, comp.width.max(1), comp.height.max(1),
                        );
                        renderer.update_shadow_map(&map, comp.width.max(1), comp.height.max(1));
                    } else if renderer.shadow_active() {
                        renderer.clear_shadow_map();
                    }
                    let (view, rec) = renderer.render(comp, current_frame, exposure_ev, lut_idx as u32);
                    app.gpu_rendered = true;
                    (Some(view), rec)
                };
                let render_ms = render_started.elapsed().as_secs_f32() * 1000.0;
                app.playback.preview_render_ema_ms = if app.playback.preview_render_ema_ms <= 0.0 {
                    render_ms
                } else {
                    app.playback.preview_render_ema_ms * 0.9 + render_ms * 0.1
                };

                // Adapt quality only during playback; idle renders restore full quality
                if app.playback.is_playing {
                    let budget_ms = 1000.0 / comp.fps.max(1) as f32 * 0.8;
                    if app.viewport_fast_preview == 0 {
                        app.playback.adaptive_preview_factor = 1.0;
                    } else if app.viewport_fast_preview == 2 {
                        app.playback.adaptive_preview_factor = 0.5;
                    } else if app.playback.preview_render_ema_ms > budget_ms {
                        app.playback.adaptive_preview_factor = (app.playback.adaptive_preview_factor * 0.85).max(0.5);
                    } else if app.playback.preview_render_ema_ms < budget_ms * 0.6 {
                        app.playback.adaptive_preview_factor = (app.playback.adaptive_preview_factor * 1.1).min(1.0);
                    }
                } else {
                    app.playback.adaptive_preview_factor = 1.0;
                }
                if let Some(view) = texture_view {
                    if app.viewport_texture_id.is_none() || recreated {
                        if let Some(old_id) = app.viewport_texture_id {
                            wgpu_state.renderer.write().free_texture(&old_id);
                        }
                        let texture_id = wgpu_state.renderer.write().register_native_texture(
                            &wgpu_state.device,
                            view,
                            wgpu::FilterMode::Linear,
                        );
                        app.viewport_texture_id = Some(texture_id);
                    }
                }

                let mut snap_texture_id_val = None;
                if is_comparing {
                    if let Some(snap_frame) = snap_frame_opt {
                        let (snap_view, snap_recreated) = renderer.render_snapshot_frame(comp, snap_frame, exposure_ev, lut_idx as u32);
                        if app.viewport_snapshot_texture_id.is_none() || snap_recreated {
                            if let Some(old_id) = app.viewport_snapshot_texture_id {
                                wgpu_state.renderer.write().free_texture(&old_id);
                            }
                            let texture_id = wgpu_state.renderer.write().register_native_texture(
                                &wgpu_state.device,
                                snap_view,
                                wgpu::FilterMode::Linear,
                            );
                            app.viewport_snapshot_texture_id = Some(texture_id);
                        }
                        snap_texture_id_val = app.viewport_snapshot_texture_id;
                    }
                }

                if let (true, Some(cur_tex), Some(snap_tex)) = (is_comparing, app.viewport_texture_id, snap_texture_id_val) {
                    // Left Side: Current Frame
                    let left_rect = egui::Rect::from_min_max(
                        draw_rect.min,
                        egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.max.y),
                    );
                    let left_img = egui::Image::new(egui::load::SizedTexture::new(cur_tex, left_rect.size()))
                        .uv(egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(wipe_pos, 1.0)));
                    ui.put(left_rect, left_img);

                    // Right Side: Snapshot Frame (Compare)
                    let right_rect = egui::Rect::from_min_max(
                        egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.min.y),
                        draw_rect.max,
                    );
                    let right_img = egui::Image::new(egui::load::SizedTexture::new(snap_tex, right_rect.size()))
                        .uv(egui::Rect::from_min_max(egui::pos2(wipe_pos, 0.0), egui::pos2(1.0, 1.0)));
                    ui.put(right_rect, right_img);

                    rendered_gpu = true;
                } else if let Some(texture_id) = app.viewport_texture_id {
                    ui.put(draw_rect, egui::Image::new(egui::load::SizedTexture::new(texture_id, draw_rect.size())));
                    rendered_gpu = true;
                }
            }
            }
        }

        // -- CPU Fallback Render (2D) --
        // -- 3D Camera View --
        if app.viewport_mode == ViewportMode::Camera3D && !comp_is_empty {
            crate::ui::viewport_camera_3d::draw_camera_3d_viewport(
                ui, app, ctx, current_frame, &viewport_response, rect, comp_w, comp_h, draw_w, draw_h,
            );
        }

        if app.viewport_mode == ViewportMode::Comp2D && !comp_is_empty && !rendered_gpu && is_comparing {
            if let Some(snap_frame) = snap_frame_opt {
                let left_rect = egui::Rect::from_min_max(
                    draw_rect.min,
                    egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.max.y),
                );
                let right_rect = egui::Rect::from_min_max(
                    egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.min.y),
                    draw_rect.max,
                );
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, current_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    Some(left_rect),
                );
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, snap_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    Some(right_rect),
                );
            } else {
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, current_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    None,
                );
            }
        }

        // CPU fallback keeps the Viewer useful when a GPU texture is not
        // available, including CPU-only builds and software preview mode.
        if app.viewport_mode == ViewportMode::Comp2D && !comp_is_empty && !rendered_gpu && !is_comparing {
            let sw_started = std::time::Instant::now();
            crate::ui::viewport_canvas::draw_software_canvas(
                ui, app, current_frame, draw_rect, origin_x, origin_y,
                draw_w, draw_h, comp_w, comp_h, None,
            );
            let sw_ms = sw_started.elapsed().as_secs_f32() * 1000.0;
            app.playback.preview_render_ema_ms = if app.playback.preview_render_ema_ms <= 0.0 {
                sw_ms
            } else {
                app.playback.preview_render_ema_ms * 0.8 + sw_ms * 0.2
            };
            if app.playback.is_playing {
                let budget_ms = 1000.0 / comp_fps as f32 * 0.8;
                // If fast preview is Full (0), keep factor at 1.0. Otherwise adapt with a sensible floor (0.5 = half resolution).
                if app.viewport_fast_preview == 0 {
                    app.playback.adaptive_preview_factor = 1.0;
                } else if app.viewport_fast_preview == 2 {
                    // Draft: fixed 0.5
                    app.playback.adaptive_preview_factor = 0.5;
                } else if app.playback.preview_render_ema_ms > budget_ms {
                    app.playback.adaptive_preview_factor = (app.playback.adaptive_preview_factor * 0.85).max(0.5);
                } else if app.playback.preview_render_ema_ms < budget_ms * 0.6 {
                    app.playback.adaptive_preview_factor = (app.playback.adaptive_preview_factor * 1.1).min(1.0);
                }
            } else {
                // When paused/scrubbing, immediately restore full crisp resolution
                app.playback.adaptive_preview_factor = 1.0;
            }
            rendered_gpu = true;
        }

        // ── Viewport Overlays (Grid, Safe Guides, HUD Badges, 3D Gizmo, Snapshot Wipe) ──
        crate::ui::viewport_overlays::draw_viewport_overlays(
            ui, app, ctx, current_frame, rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h, rendered_gpu,
        );

        if app.history.current().has_pristine_start_composition() {
            let empty_rect = egui::Rect::from_center_size(
                rect.center(),
                egui::vec2(344.0, 122.0),
            );
            ui.painter().rect_filled(
                empty_rect,
                6.0,
                egui::Color32::from_rgba_premultiplied(13, 22, 31, 220),
            );
            ui.painter().rect_stroke(
                empty_rect,
                6.0,
                egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            );
            ui.painter().text(
                egui::pos2(empty_rect.center().x, empty_rect.top() + 25.0),
                egui::Align2::CENTER_CENTER,
                "プロジェクトを始めましょう",
                egui::FontId::proportional(14.0),
                colors::TEXT_SECONDARY,
            );
            let action_width = 148.0;
            let action_gap = 10.0;
            let action_y = empty_rect.top() + 54.0;
            let create_rect = egui::Rect::from_min_size(
                egui::pos2(empty_rect.center().x - action_width - action_gap * 0.5, action_y),
                egui::vec2(action_width, 38.0),
            );
            let open_rect = egui::Rect::from_min_size(
                egui::pos2(empty_rect.center().x + action_gap * 0.5, action_y),
                egui::vec2(action_width, 38.0),
            );
            if ui.put(
                create_rect,
                egui::Button::new("新規コンポジション").fill(colors::ACCENT_BRAND),
            ).clicked() {
                app.show_new_comp_dialog = true;
            }
            if ui.put(open_rect, egui::Button::new("プロジェクトを開く")).clicked() {
                crate::ui::home_screen::open_project_dialog(app);
            }
        } else if let Some(layer_name) = missing_media {
            let missing_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(280.0, 72.0));
            ui.painter().rect_filled(
                missing_rect,
                5.0,
                egui::Color32::from_rgba_premultiplied(31, 24, 22, 224),
            );
            ui.painter().rect_stroke(
                missing_rect,
                5.0,
                egui::Stroke::new(1.0_f32, colors::ACCENT_ORANGE),
            );
            ui.painter().text(
                missing_rect.center_top() + egui::vec2(0.0, 21.0),
                egui::Align2::CENTER_CENTER,
                "Missing media",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                missing_rect.center_bottom() - egui::vec2(0.0, 18.0),
                egui::Align2::CENTER_CENTER,
                format!("{} · Relink in Project", layer_name),
                egui::FontId::proportional(11.0),
                colors::TEXT_SECONDARY,
            );
        }

        // ── Rectangle tool rubber-band preview ──
        if app.active_tool == crate::ui::toolbar::ActiveTool::Rectangle {
            if let Some(start) = app.rect_drag_start {
                if let Some(current) = viewport_response.interact_pointer_pos() {
                    let r = egui::Rect::from_two_pos(start, current);
                    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.5_f32, colors::ACCENT_BLUE));
                    ui.painter().rect_filled(r, 0.0, colors::TIMELINE_SELECTION.linear_multiply(0.15));
                }
            }
        }

        painting::draw_brush_eraser(
            app, ctx, ui, current_frame, &viewport_response, origin_x, origin_y, draw_w, draw_h,
            comp_w, comp_h,
        );

        painting::draw_clone_stamp(
            app, ctx, current_frame, &viewport_response, origin_x, origin_y, draw_w, draw_h,
            comp_w, comp_h,
        );

        puppet_pin::draw(
            app,
            ui,
            current_frame,
            &viewport_response,
            ViewportGeometry { origin_x, origin_y, draw_w, draw_h, comp_w, comp_h },
        );

        // ── Motion Sketch live path recording ──
        let is_sketching_id = egui::Id::new("ae_motion_sketch_armed");
        let is_armed = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_sketching_id, || false));
        if is_armed {
            if let Some(pp) = viewport_response.interact_pointer_pos() {
                if viewport_response.drag_started() {
                    app.begin_drag("Motion Sketch");
                }
                if viewport_response.dragged() {
                    let cx = (pp.x - origin_x) / draw_w * comp_w;
                    let cy = (pp.y - origin_y) / draw_h * comp_h;
                    if let Some(idx) = app.selection.selected_layer_idx {
                        let cf = app.playback.current_frame;
                        let comp = app.history.current_mut().active_composition_mut();
                        if let Some(layer) = comp.layers.get_mut(idx) {
                            match &mut layer.transform.position {
                                crate::core::property::Animatable::Animated(kfs) => {
                                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == cf) {
                                        existing.value = [cx, cy];
                                    } else {
                                        kfs.push(crate::core::keyframe::Keyframe::new(cf, [cx, cy], crate::core::keyframe::InterpolationType::Linear));
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                                _ => {
                                    layer.transform.position = crate::core::property::Animatable::Animated(vec![
                                        crate::core::keyframe::Keyframe::new(cf, [cx, cy], crate::core::keyframe::InterpolationType::Linear),
                                    ]);
                                }
                            }
                            crate::core::frame_cache::bump_version();
                        }
                        let max_f = app.history.current().active_composition().duration_frames.saturating_sub(1);
                        if app.playback.current_frame < max_f {
                            app.playback.current_frame += 1;
                            ctx.request_repaint();
                        }
                    }
                }
                if viewport_response.drag_stopped() {
                    app.commit_drag();
                }
            }
        }

        roto_brush::draw_roto_brush_interaction(
            app, ctx, ui, current_frame, &viewport_response, origin_x, origin_y, draw_w, draw_h,
            comp_w, comp_h,
        );

        // ── Interactive Layer & Mask Drag ──────────────────────────
        let pen_commit = interaction::handle_viewport_interaction(
            app,
            ctx,
            ui,
            &viewport_response,
            interaction::InteractionContext {
                origin_x,
                origin_y,
                draw_w,
                draw_h,
                comp_w,
                comp_h,
                current_frame,
                draw_rect,
            },
        );

        pen_preview::draw(
            app,
            ui,
            &viewport_response,
            ViewportGeometry { origin_x, origin_y, draw_w, draw_h, comp_w, comp_h },
        );

        mask_tools::draw_selection_overlays(
            app,
            ui,
            &viewport_response,
            current_frame,
            ViewportGeometry { origin_x, origin_y, draw_w, draw_h, comp_w, comp_h },
        );

        // ── Pen tool commit: turn collected points into a mask on the selected layer ──
        if pen_commit && app.pen_points.len() >= 3 {
            app.begin_drag("Create Pen Mask");
            app.pen_bezier_drag = None;
            let target_layer = match app.selection.selected_layer_idx {
                Some(idx) => idx,
                None => {
                    // No layer selected: create a host solid for the mask
                    let (n, dur) = {
                        let c = app.history.current().active_composition();
                        (c.layers.len(), c.duration_frames)
                    };
                    let mut solid = crate::core::timeline::Layer::new(
                        format!("solid_{}", n),
                        format!("Mask Solid {}", n + 1),
                        crate::core::timeline::LayerType::Solid { color: [0.8, 0.8, 0.8, 1.0] },
                        dur,
                    );
                    solid.transform.position = crate::core::property::Animatable::new_constant([comp_w * 0.5, comp_h * 0.5]);
                    let comp_mut = app.history.current_mut().active_composition_mut();
                    comp_mut.layers.push(solid);
                    let new_idx = comp_mut.layers.len() - 1;
                    app.selection.selected_layer_idx = Some(new_idx);
                    new_idx
                }
            };
            let pts = std::mem::take(&mut app.pen_points);
            let tangs = std::mem::take(&mut app.pen_tangents);
            let comp_mut = app.history.current_mut().active_composition_mut();
            if let Some(layer) = comp_mut.layers.get_mut(target_layer) {
                let mask_id = layer.masks.len();
                layer.masks.push(crate::core::mask::Mask {
                    id: format!("pen_mask_{}_{}", layer.id, mask_id),
                    name: format!("Mask {}", mask_id + 1),
                    enabled: true,
                    mode: crate::core::mask::MaskMode::Add,
                    path: pen_path(pts.clone(), tangs),
                    feather: crate::core::property::Animatable::new_constant(0.0),
                    opacity: crate::core::property::Animatable::new_constant(100.0),
                    expansion: crate::core::property::Animatable::new_constant(0.0),
                    inverted: false,
                    wiggle: None,
                });
            }
            crate::core::frame_cache::bump_version();
            app.commit_drag();
            app.autosave.mark_dirty();
            app.toasts.info(format!("Pen mask created ({} points)", pts.len()));
        } else if pen_commit {
            app.pen_points.clear();
            app.pen_tangents.clear();
        }

        draw_inline_text_editor(
            app,
            ctx,
            InlineTextEditorGeometry {
                current_frame,
                origin: [origin_x, origin_y],
                viewport_size: [draw_w, draw_h],
                composition_size: [comp_w, comp_h],
            },
        );

        // ── Integrated Viewer Bottom Transport Bar (Matching Mockup) ──
        ui.add_space(2.0);
        let b_rect = ui.available_rect_before_wrap();
        let bar_h = 34.0;
        let trans_rect = egui::Rect::from_min_size(b_rect.min, egui::vec2(b_rect.width(), bar_h));
        ui.painter().rect_filled(trans_rect, 0.0, colors::BG_DARKEST);
        ui.painter().line_segment(
            [trans_rect.left_top(), trans_rect.right_top()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );

        let mut bottom_ui = ui.new_child(egui::UiBuilder::new().max_rect(trans_rect.shrink2(egui::vec2(16.0, 3.0))));
        bottom_ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;

            let fps = comp_fps;
            let timecode_str = crate::ui::timeline::ruler::format_timecode_label(current_frame, fps);

            ui.label(
                egui::RichText::new(timecode_str)
                    .size(13.0)
                    .strong()
                    .monospace()
                    .color(colors::TEXT_PRIMARY),
            );

            // Sleek orange progress bar / scrubber (matching ideal)
            let total_f = comp_duration_frames.max(1);
            let scrubber_w = 120.0_f32;
            let (scrub_rect, scrub_resp) = ui.allocate_exact_size(egui::vec2(scrubber_w, 16.0), egui::Sense::click_and_drag());
            let track_rect = egui::Rect::from_center_size(scrub_rect.center(), egui::vec2(scrubber_w, 4.0));
            ui.painter().rect_filled(track_rect, 2.0, colors::BG_SURFACE);
            let progress = (current_frame as f32 / total_f as f32).clamp(0.0, 1.0);
            let filled_rect = egui::Rect::from_min_max(
                track_rect.left_top(),
                egui::pos2(track_rect.left() + track_rect.width() * progress, track_rect.bottom()),
            );
            ui.painter().rect_filled(filled_rect, 2.0, colors::ACCENT_BRAND);
            let knob_pos = egui::pos2(track_rect.left() + track_rect.width() * progress, track_rect.center().y);
            ui.painter().circle_filled(knob_pos, 5.0, colors::ACCENT_BRAND);

            if scrub_resp.clicked() || scrub_resp.dragged() {
                if let Some(pos) = scrub_resp.interact_pointer_pos() {
                    let t = ((pos.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
                    app.playback.current_frame = (t * total_f as f32).round() as u32;
                }
            }

            ui.add_space(6.0);

            // Transport buttons: |<, <, Play/Pause, >, >|
            if ui.add(egui::Button::new(egui::RichText::new("⏮").size(12.0).color(colors::TEXT_SECONDARY)).frame(false)).clicked() {
                app.playback.current_frame = 0;
            }
            if ui.add(egui::Button::new(egui::RichText::new("◀").size(12.0).color(colors::TEXT_SECONDARY)).frame(false)).clicked() {
                app.playback.current_frame = current_frame.saturating_sub(1);
            }

            let play_label = if app.playback.is_playing { "⏸" } else { "▶" };
            let play_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new(play_label)
                        .size(15.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .frame(false),
            );
            if play_btn.clicked() {
                app.playback.is_playing = !app.playback.is_playing;
            }

            if ui.add(egui::Button::new(egui::RichText::new("▶").size(12.0).color(colors::TEXT_SECONDARY)).frame(false)).clicked() {
                app.playback.current_frame = (current_frame + 1).min(total_f);
            }
            if ui.add(egui::Button::new(egui::RichText::new("⏭").size(12.0).color(colors::TEXT_SECONDARY)).frame(false)).clicked() {
                app.playback.current_frame = total_f;
            }

            // Right side: Total Timecode + Quality combo + Fullscreen icon
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                if btn_resp.hovered() {
                    ui.painter().rect_filled(btn_rect, 3.0, colors::BG_HOVER);
                }
                crate::ui::icons::render_svg_at(
                    ui,
                    "viewport-bottom-fullscreen".to_string(),
                    crate::ui::icons::SVG_WINDOW_MAXIMIZE,
                    egui::vec2(14.0, 14.0),
                    colors::TEXT_SECONDARY,
                    btn_rect.center() - egui::vec2(7.0, 7.0),
                );
                if btn_resp.clicked() {
                    app.viewer_maximized = !app.viewer_maximized;
                }

                ui.add_space(4.0);

                egui::ComboBox::from_id_salt("viewport_transport_quality")
                    .selected_text(match app.viewport_fast_preview {
                        0 => "フル",
                        1 => "適応",
                        2 => "ドラフト",
                        _ => "ワイヤー",
                    })
                    .width(62.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.viewport_fast_preview, 0, "フル");
                        ui.selectable_value(&mut app.viewport_fast_preview, 1, "適応");
                        ui.selectable_value(&mut app.viewport_fast_preview, 2, "ドラフト");
                        ui.selectable_value(&mut app.viewport_fast_preview, 3, "ワイヤー");
                    });

                ui.add_space(8.0);

                let dur_timecode = crate::ui::timeline::ruler::format_timecode_label(total_f, fps);
                ui.label(
                    egui::RichText::new(dur_timecode)
                        .size(12.5)
                        .monospace()
                        .color(colors::TEXT_MUTED),
                );
            });
        });
    });
}

/// Inline source-text editor opened by double-clicking a text layer in the viewport.
/// Renders as an `egui::Area` positioned exactly over the text layer on the composition canvas.
#[cfg(test)]
#[path = "viewport/review_regression_tests.rs"]
mod review_regression_tests;
