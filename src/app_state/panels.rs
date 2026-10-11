use super::*;

impl KagariApp {
    pub(super) fn draw_startup_animation(&self, ctx: &eframe::egui::Context) {
        if !self.startup_animation.is_animating() {
            return;
        }

        let progress =
            crate::ui::panel_animation::PanelAnimation::ease(self.startup_animation.progress);
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(eframe::egui::LayerId::new(
            eframe::egui::Order::Foreground,
            eframe::egui::Id::new("kagari-startup-transition"),
        ));
        let veil_alpha = ((1.0 - progress) * 232.0).round() as u8;
        painter.rect_filled(
            screen,
            0.0,
            eframe::egui::Color32::from_rgba_unmultiplied(7, 13, 19, veil_alpha),
        );

        let logo_alpha = if progress < 0.58 {
            (progress / 0.58).clamp(0.0, 1.0)
        } else {
            ((1.0 - progress) / 0.42).clamp(0.0, 1.0)
        };
        let logo_size = (screen.width().min(screen.height()) * 0.13).clamp(64.0, 118.0)
            * (0.88 + progress * 0.12);
        let logo_rect = eframe::egui::Rect::from_center_size(
            eframe::egui::pos2(screen.center().x, screen.center().y - 18.0),
            eframe::egui::vec2(logo_size, logo_size),
        );
        if let Some(texture) = self.home_banner.as_ref() {
            painter.image(
                texture.id(),
                logo_rect,
                eframe::egui::Rect::from_min_max(
                    eframe::egui::pos2(0.0, 0.0),
                    eframe::egui::pos2(1.0, 1.0),
                ),
                eframe::egui::Color32::from_white_alpha((logo_alpha * 255.0).round() as u8),
            );
        }
        painter.text(
            eframe::egui::pos2(screen.center().x, logo_rect.bottom() + 22.0),
            eframe::egui::Align2::CENTER_TOP,
            "Kagari VFX",
            eframe::egui::FontId::proportional((logo_size * 0.22).clamp(16.0, 24.0)),
            eframe::egui::Color32::from_white_alpha((logo_alpha * 220.0).round() as u8),
        );
    }

    /// Panel and dialog drawing extracted from [`eframe::App::update`] so the
    /// full UI can run headlessly in tests (`_frame` was unused).
    pub fn update_panels(&mut self, ctx: &eframe::egui::Context) {
        self.ui_ctx = Some(ctx.clone());
        crate::ui::project_io::poll_collect_files(self, ctx);
        crate::ui::workspace_manager::restore_pending_workspace(ctx);
        let assets_workspace = ctx.data(|data| {
            data.get_temp::<usize>(eframe::egui::Id::new("studio_active_workspace"))
                == Some(crate::ui::menu::ASSETS_WORKSPACE_INDEX)
        });
        let compact_inspector_drawer = ctx
            .data(|data| data.get_temp::<bool>(eframe::egui::Id::new("compact_inspector_drawer")))
            .unwrap_or(false);
        let scripting_console_visible = !self.show_home
            && !self.show_guided_tutorial
            && !(self.ui_tabs.left_tab_idx == 1
                && self.ui_tabs.right_panel == crate::app_state::RightPanel::Color)
            && !(assets_workspace
                && self.ui_tabs.left_tab_idx == 0
                && self.ui_tabs.right_panel == crate::app_state::RightPanel::Properties)
            && !self.viewer_maximized
            && !(ctx.screen_rect().width() < crate::ui::theme::layout::DOCKED_PANELS_MIN_WIDTH
                && !compact_inspector_drawer)
            && self.ui_tabs.right_panel == crate::app_state::RightPanel::Scripting;
        if !scripting_console_visible {
            if let Some(cancel) = self.script_console_cancel.as_ref() {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let opening_was_active = self.startup_animation.is_animating();
        self.startup_animation
            .update(ctx.input(|input| input.unstable_dt));
        if opening_was_active || self.startup_animation.is_animating() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Global drag safety net: any pointer release while a transaction is
        // open seals it, even if the owning widget missed the release event
        // (prevents permanently-open transactions from swallowing edits).
        if self.drag_active() && ctx.input(|i| i.pointer.any_released()) {
            self.commit_drag();
        }

        // One-time startup check for crash recovery snapshots.
        // Only prompt if a "dirty exit" marker exists — written on startup and
        // removed on clean shutdown. This prevents false positives when the
        // user simply closed the app normally with autosave files still present.
        if !self.recovery_checked {
            self.recovery_checked = true;
            // Load user preferences before the first autosave tick so cache,
            // history, audio preview, and saved workspaces are live from the
            // first frame. This must not share a second one-shot guard below.
            crate::ui::preferences_dialog::apply_loaded(self);
            let had_crash = find_crashed_session_marker();
            let dirty_exit_marker = dirty_exit_marker_path(std::process::id());
            let _ = std::fs::write(dirty_exit_marker, b"active");
            if had_crash && self.autosave.has_recovery() {
                self.recovery_snapshot_time = self.autosave.latest_recovery_modified().map(|t| {
                    let secs = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    format!("#{}", secs)
                });
                self.show_recovery_dialog = true;
            }
        }

        // History-backed edits can come from panels that use EditorSession or
        // ProjectHistory directly. Observe the generation centrally so every
        // committed edit participates in crash recovery.
        let history_generation = self.history.generation();
        if history_generation != self.autosave_history_generation {
            self.autosave.mark_dirty();
            self.autosave_history_generation = history_generation;
        }

        // Crash-recovery autosave: write a rotating snapshot when dirty and due
        let correction = self.audio_correction_settings();
        if let Some(document) = self.production_document.as_mut() {
            document.audio.correction = correction;
            document.audio.channels = self.audio_mixer_channels.clone();
            document.audio.master_gain = self.playback.master_volume;
        }
        let autosave_path = self
            .autosave
            .tick_async(self.history.current(), self.production_document.as_ref());
        if let Some(path) = autosave_path {
            log::info!("[Autosave] Recovery snapshot written: {:?}", path);
        }
        if let Some(error) = self.autosave.take_last_error() {
            log::warn!("[Autosave] Recovery snapshot failed; retrying in 30 seconds: {error}");
            self.toasts.warning(format!(
                "自動復旧保存に失敗しました。30秒後に再試行します: {error}"
            ));
        }

        // ── Detect play-start transition for wall-clock AV sync ──
        self.playback.sync_play_start();

        // ── Synced audio playback: follow the playhead while playing ──
        {
            let audio_enabled = self.audio_preview_enabled;
            let fps = self.history.current().active_composition().fps.max(1);

            // Live metering: mix a tiny buffer at the playhead with mixer gains
            {
                let project = self.history.current();
                let comp = project.active_composition();
                let dsp = self.master_dsp_params();
                let (mix, meter) = crate::core::audio_engine::mix_audio_sources_for_frame(
                    comp,
                    self.playback.current_frame,
                    48000,
                    64, // small buffer — metering only
                    Some(&self.audio_mixer_channels),
                    &dsp,
                );
                let db_to_lin = |db: f32| 10.0f32.powf(db / 20.0).min(1.0);
                self.audio_meter = (
                    db_to_lin(meter.peak_db_left),
                    db_to_lin(meter.peak_db_right),
                );
                self.audio_track_meters = meter.track_peaks;
                for issue in &meter.source_errors {
                    let key = format!("{}\0{}", issue.path, issue.reason);
                    if self.audio_error_notices.insert(key) {
                        self.toasts.warning(format!(
                            "音声素材を読み込めません: {} ({}): {}",
                            issue.layer, issue.path, issue.reason
                        ));
                    }
                }
                let spectrum_options = crate::core::audio_spectrum::AudioSpectrumOptions {
                    frequency_bands: 32,
                    ..Default::default()
                };
                self.audio_spectrum_bands =
                    crate::core::audio_spectrum::generate_audio_spectrum_bands(
                        &mix,
                        48000,
                        &spectrum_options,
                    );
            }

            // Mix only when audio graph or work area changes (not every frame)
            if audio_enabled && self.playback.is_playing {
                use std::hash::{Hash, Hasher};
                let project = self.history.current();
                let comp = project.active_composition();
                let wa_start = self
                    .playback
                    .work_area_in
                    .unwrap_or(0)
                    .min(comp.duration_frames);
                let wa_end = self
                    .playback
                    .work_area_out
                    .map(|frame| frame.saturating_add(1))
                    .unwrap_or(comp.duration_frames)
                    .clamp(wa_start, comp.duration_frames);
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                comp.id.hash(&mut hasher);
                self.history.generation().hash(&mut hasher);
                wa_start.hash(&mut hasher);
                wa_end.hash(&mut hasher);
                (self.master_eq_highpass.to_bits()).hash(&mut hasher);
                (self.master_eq_lowpass.to_bits()).hash(&mut hasher);
                (self.master_eq_mid_gain.to_bits()).hash(&mut hasher);
                (self.master_eq_mid_freq.to_bits()).hash(&mut hasher);
                (self.master_comp_threshold.to_bits()).hash(&mut hasher);
                (self.master_comp_ratio.to_bits()).hash(&mut hasher);
                (self.master_comp_attack.to_bits()).hash(&mut hasher);
                (self.master_comp_release.to_bits()).hash(&mut hasher);
                (self.master_comp_makeup.to_bits()).hash(&mut hasher);
                self.master_auto_gain.hash(&mut hasher);
                (self.master_target_level_db.to_bits()).hash(&mut hasher);
                self.master_limiter_enabled.hash(&mut hasher);
                (self.master_limiter_ceiling_db.to_bits()).hash(&mut hasher);
                (self.master_wet_dry.to_bits()).hash(&mut hasher);
                self.master_dsp_enabled.hash(&mut hasher);
                self.master_dsp_bypass.hash(&mut hasher);
                for ch in &self.audio_mixer_channels {
                    (ch.gain_db.to_bits()).hash(&mut hasher);
                    (ch.pan.to_bits()).hash(&mut hasher);
                    ch.mute.hash(&mut hasher);
                    ch.solo.hash(&mut hasher);
                }
                let key = hasher.finish();

                let needs_mix = self.playback.cached_audio_key != key;

                if needs_mix {
                    let dsp = self.master_dsp_params();
                    if let Some(previous_mix) = self.playback.cached_audio_path.take() {
                        let _ = std::fs::remove_file(previous_mix);
                    }
                    match crate::core::audio_engine::mix_composition_to_wav(
                        comp,
                        wa_start,
                        wa_end,
                        48000,
                        Some(&self.audio_mixer_channels),
                        &dsp,
                    ) {
                        Ok(Some(wav)) => {
                            self.playback.cached_audio_path = Some(wav);
                            self.playback.cached_audio_key = key;
                        }
                        Ok(None) => {
                            self.playback.cached_audio_key = key;
                            self.playback.cached_audio_path = None;
                        }
                        Err(error) => {
                            self.playback.cached_audio_key = key;
                            self.toasts
                                .warning(format!("Could not prepare audio preview: {error}"));
                            log::error!("Could not prepare audio preview: {error}");
                        }
                    }
                }
            }

            match (
                self.playback.is_playing,
                audio_enabled,
                self.playback.cached_audio_path.as_ref(),
                &mut self.audio_playback,
            ) {
                (true, true, Some(wav), Some(playback)) => {
                    let work_area_start = self.playback.work_area_in.unwrap_or(0);
                    let audio_offset_sec =
                        self.playback.current_frame.saturating_sub(work_area_start) as f32
                            / fps as f32;
                    if let Err(e) = playback.play(wav, audio_offset_sec) {
                        log::warn!("[Audio] playback error: {}", e);
                    }
                }
                _ => {
                    if let Some(playback) = &mut self.audio_playback {
                        if playback.is_playing() {
                            playback.pause();
                        }
                    }
                }
            }
            // Master volume applies to the sink output
            if let Some(playback) = &self.audio_playback {
                playback.set_volume(self.playback.master_volume);
            }
        }

        if let Some(rx_frame) = &self.rx_frame {
            while let Ok(frame) = rx_frame.try_recv() {
                self.playback.current_frame = frame;
            }
        }
        if let Some(rx_connection) = &self.rx_connection {
            while let Ok(conn) = rx_connection.try_recv() {
                self.connected_app = conn;
            }
        }

        let total_frames = self.history.current().active_composition().duration_frames;
        let mut current_frame = self.playback.current_frame;

        let last_frame = total_frames.saturating_sub(1);
        let wa_start = self.playback.work_area_in.unwrap_or(0).min(last_frame);
        let wa_end = self
            .playback
            .work_area_out
            .unwrap_or(last_frame)
            .min(last_frame)
            .max(wa_start);

        if self.playback.is_playing {
            let speed = self.playback_speed.abs().max(1) as u32;
            let fps = self.history.current().active_composition().fps.max(1);

            // Wall-clock based frame calculation
            if let Some(start_time) = self.playback.play_start_time {
                let elapsed = start_time.elapsed().as_secs_f64();
                let frames_per_sec = fps as f64;
                let direction = if self.playback_speed >= 0 { 1.0 } else { -1.0 };
                let raw = self.playback.play_start_frame as f64
                    + elapsed * frames_per_sec * direction * speed as f64;

                if self.playback_speed >= 0 {
                    let _frame_range = wa_start..=wa_end;
                    if raw >= wa_end as f64 {
                        if self.loop_playback {
                            let span = (wa_end - wa_start) as f64;
                            let wrapped = if span > 0.0 {
                                wa_start as f64 + ((raw - wa_start as f64) % span)
                            } else {
                                wa_start as f64
                            };
                            current_frame = wrapped as u32;
                        } else {
                            self.playback.stop_playing();
                            self.motion_sketch_active = false;
                            current_frame = wa_end;
                        }
                    } else {
                        current_frame = raw.max(wa_start as f64) as u32;
                    }
                } else {
                    if raw <= wa_start as f64 {
                        if self.loop_playback {
                            let span = (wa_end - wa_start) as f64;
                            let wrapped = if span > 0.0 {
                                wa_end as f64 - ((wa_start as f64 - raw) % span)
                            } else {
                                wa_end as f64
                            };
                            current_frame = wrapped as u32;
                        } else {
                            self.playback.stop_playing();
                            self.motion_sketch_active = false;
                            current_frame = wa_start;
                        }
                    } else {
                        current_frame = raw.min(wa_end as f64) as u32;
                    }
                }
            } else {
                // Fallback: no start time recorded yet, use simple increment
                if self.playback_speed >= 0 {
                    let next = current_frame.saturating_add(speed);
                    current_frame = if next >= wa_end {
                        if self.loop_playback {
                            wa_start
                        } else {
                            self.playback.stop_playing();
                            self.motion_sketch_active = false;
                            wa_end
                        }
                    } else {
                        next
                    };
                } else {
                    current_frame = if current_frame == wa_start {
                        if self.loop_playback {
                            wa_end.saturating_sub(1)
                        } else {
                            self.playback.stop_playing();
                            self.motion_sketch_active = false;
                            wa_start
                        }
                    } else {
                        current_frame.saturating_sub(speed)
                    };
                }
            }

            let effective_fps = (fps as f32 * speed as f32).max(1.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.0 / effective_fps));
        }

        crate::ui::shortcuts::handle_global_shortcuts(self, ctx, &mut current_frame, total_frames);
        self.draw_workspace(ctx, current_frame, total_frames);
    }
}
