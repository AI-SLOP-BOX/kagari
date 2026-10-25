use crate::core::timeline::Composition;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn render_gpu_preview(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    comp: &Composition,
    current_frame: u32,
    draw_rect: egui::Rect,
    draw_w: f32,
    is_comparing: bool,
    snap_frame_opt: Option<u32>,
    wipe_pos: f32,
) -> bool {
    #[allow(unused_mut)]
    let mut rendered_gpu = false;

    #[cfg(feature = "wgpu")]
    {
        if let Some(renderer) = &mut app.renderer {
            if let Some(wgpu_state) = &app.wgpu_state {
                let exp_id = egui::Id::new("ae_exposure_ev");
                let exposure_ev =
                    ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(exp_id, || 0.0f32));
                let lut_idx = app.color_lut_mode as usize;

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
                let preview_px =
                    ((display_px * effective_factor * proxy_factor) as u32).clamp(64, 4096);
                renderer.set_preview_max_width(Some(preview_px));

                // ── RAM preview: incremental pre-pass ──
                const RAM_PREPASS_FRAMES_PER_TICK: u32 = 2;
                const RAM_PREPASS_MAX_FRAMES: u32 = 300;
                const RAM_PREVIEW_BUDGET_BYTES: u64 = 256 * 1024 * 1024;
                if app.playback.is_playing && !app.was_playing_last_frame {
                    let last_frame = comp.duration_frames.saturating_sub(1);
                    let wa_in = app.playback.work_area_in.unwrap_or(0).min(last_frame);
                    let wa_out = app
                        .playback
                        .work_area_out
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

                let ram_id = if app.playback.is_playing {
                    app.ram_texture_ids
                        .iter()
                        .find(|(f, _)| *f == current_frame)
                        .map(|(_, id)| *id)
                } else {
                    None
                };

                if app.playback.is_playing
                    && ram_id.is_some()
                    && app.ram_prepass_cursor <= app.ram_prepass_end
                {
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
                    if app.viewport_texture_id != Some(id) {
                        if let Some(old_id) = app.viewport_texture_id {
                            if !app
                                .ram_texture_ids
                                .iter()
                                .any(|(_, cached)| *cached == old_id)
                            {
                                wgpu_state.renderer.write().free_texture(&old_id);
                            }
                        }
                        app.viewport_texture_id = Some(id);
                    }
                    rendered_gpu = true;
                    app.gpu_rendered = true;
                }
                let (texture_view, recreated) = if ram_id.is_some() {
                    (None, false)
                } else {
                    if comp
                        .lights
                        .iter()
                        .any(|l| l.casts_shadows && l.intensity > 0.0)
                    {
                        let map = crate::core::software_renderer::build_shadow_map(
                            comp,
                            current_frame,
                            comp.width.max(1),
                            comp.height.max(1),
                        );
                        renderer.update_shadow_map(&map, comp.width.max(1), comp.height.max(1));
                    } else if renderer.shadow_active() {
                        renderer.clear_shadow_map();
                    }
                    let (view, rec) =
                        renderer.render(comp, current_frame, exposure_ev, lut_idx as u32);
                    app.gpu_rendered = true;
                    (Some(view), rec)
                };
                let render_ms = render_started.elapsed().as_secs_f32() * 1000.0;
                app.playback.preview_render_ema_ms = if app.playback.preview_render_ema_ms <= 0.0 {
                    render_ms
                } else {
                    app.playback.preview_render_ema_ms * 0.9 + render_ms * 0.1
                };

                if app.playback.is_playing {
                    let budget_ms = 1000.0 / comp.fps.max(1) as f32 * 0.8;
                    if app.viewport_fast_preview == 0
                        || app.viewport_fast_preview == 3
                        || (app.viewport_fast_preview == 1 && !app.adaptive_preview_enabled)
                    {
                        app.playback.adaptive_preview_factor = 1.0;
                    } else if app.viewport_fast_preview == 2 {
                        app.playback.adaptive_preview_factor = 0.5;
                    } else if app.viewport_fast_preview == 1
                        && app.playback.preview_render_ema_ms > budget_ms
                    {
                        app.playback.adaptive_preview_factor =
                            (app.playback.adaptive_preview_factor * 0.85).max(0.5);
                    } else if app.viewport_fast_preview == 1
                        && app.playback.preview_render_ema_ms < budget_ms * 0.6
                    {
                        app.playback.adaptive_preview_factor =
                            (app.playback.adaptive_preview_factor * 1.1).min(1.0);
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
                        let (snap_view, snap_recreated) = renderer.render_snapshot_frame(
                            comp,
                            snap_frame,
                            exposure_ev,
                            lut_idx as u32,
                        );
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

                if let (true, Some(cur_tex), Some(snap_tex)) =
                    (is_comparing, app.viewport_texture_id, snap_texture_id_val)
                {
                    let left_rect = egui::Rect::from_min_max(
                        draw_rect.min,
                        egui::pos2(
                            draw_rect.min.x + draw_rect.width() * wipe_pos,
                            draw_rect.max.y,
                        ),
                    );
                    let left_img =
                        egui::Image::new(egui::load::SizedTexture::new(cur_tex, left_rect.size()))
                            .uv(egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(wipe_pos, 1.0),
                            ));
                    ui.put(left_rect, left_img);

                    let right_rect = egui::Rect::from_min_max(
                        egui::pos2(
                            draw_rect.min.x + draw_rect.width() * wipe_pos,
                            draw_rect.min.y,
                        ),
                        draw_rect.max,
                    );
                    let right_img = egui::Image::new(egui::load::SizedTexture::new(
                        snap_tex,
                        right_rect.size(),
                    ))
                    .uv(egui::Rect::from_min_max(
                        egui::pos2(wipe_pos, 0.0),
                        egui::pos2(1.0, 1.0),
                    ));
                    ui.put(right_rect, right_img);

                    rendered_gpu = true;
                } else if let Some(texture_id) = app.viewport_texture_id {
                    ui.put(
                        draw_rect,
                        egui::Image::new(egui::load::SizedTexture::new(
                            texture_id,
                            draw_rect.size(),
                        )),
                    );
                    rendered_gpu = true;
                }
            }
        }
    }

    let _ = (colors::BG_DARKEST, &app.color_lut_mode);
    rendered_gpu
}
