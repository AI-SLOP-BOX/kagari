use super::*;

#[cfg(feature = "gui")]
impl eframe::App for KagariApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.repaint_context = Some(ctx.clone());
        if let Some(receiver) = self.motion_sketch_bake_rx.take() {
            match receiver.try_recv() {
                Ok(bake) => {
                    if bake.project_session_id == self.project_session_id
                        && bake.history_generation == self.history.generation()
                    {
                        let keyframe_count = bake.keyframes.len();
                        let mut applied = false;
                        self.modify_project(|project| {
                            if let Some(composition) = project
                                .compositions
                                .iter_mut()
                                .find(|composition| composition.id == bake.composition_id)
                            {
                                if let Some(layer) = composition
                                    .layers
                                    .iter_mut()
                                    .find(|layer| layer.id == bake.layer_id)
                                {
                                    layer.transform.position =
                                        crate::core::property::Animatable::Animated(bake.keyframes);
                                    applied = true;
                                }
                            }
                        });
                        if applied {
                            self.toasts.info(format!(
                                "Motion Sketch: {} keyframes (from {} raw)",
                                keyframe_count, bake.raw_sample_count
                            ));
                        } else {
                            self.toasts.warning(
                                "Motion Sketchの対象レイヤーが見つからず、結果を適用しませんでした。",
                            );
                        }
                    } else {
                        self.toasts.warning(
                            "ベイク中にプロジェクトが変更されたため、Motion Sketchの結果を破棄しました。",
                        );
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.toasts
                        .error("Motion Sketchのベイク処理に失敗しました。");
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.motion_sketch_bake_rx = Some(receiver);
                    ctx.request_repaint_after(std::time::Duration::from_millis(16));
                }
            }
        }
        crate::ui::project_io::poll_pending_project_load(self, ctx);
        crate::ui::project_io::poll_color_lut_restore(self, ctx);
        // Re-assert the dark AE theme every frame (cheap, and guards against
        // eframe's system-theme following resetting visuals)
        crate::ui::theme::configure_ae_theme(ctx);
        crate::ui::tracker_panel::poll_camera_solve(self, ctx);
        crate::ui::tracker_panel::poll_quad_track(self, ctx);

        self.update_panels(ctx);
        crate::ui::project_io::draw_project_load_modal(self, ctx);
        crate::ui::drop_import::draw_import_progress(self, ctx);
    }

    fn on_exit(&mut self) {
        if let Some(cancel) = self.script_console_cancel.as_ref() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut autosave_succeeded = true;
        if let Some(path) = self.autosave.wait_for_pending() {
            log::info!(
                "[Autosave] Recovery snapshot written before exit: {:?}",
                path
            );
        }
        if self.autosave.has_unsaved_snapshot() {
            let project = self.history.current();
            let correction = self.audio_correction_settings();
            let save_result = if let Some(document) = self.production_document.as_mut() {
                document.audio.correction = correction;
                document.audio.channels = self.audio_mixer_channels.clone();
                document.audio.master_gain = self.playback.master_volume;
                self.autosave.save_now_production(project, document)
            } else {
                self.autosave.save_now(project)
            };
            match save_result {
                Ok(path) => log::info!("[Autosave] Latest recovery snapshot written: {:?}", path),
                Err(error) => {
                    autosave_succeeded = false;
                    log::error!("[Autosave] Could not flush recovery snapshot on exit: {error}");
                }
            }
        }
        if autosave_succeeded {
            let _ = std::fs::remove_file(dirty_exit_marker_path(std::process::id()));
        }
        // Join any remaining background threads so detached threads don't
        // outlive the process (recovery load, project open, LUT restore, ...).
        for handle in self.background_threads.drain(..) {
            let _ = handle.join();
        }
        if let Some(ref flag) = self.export_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(ref flag) = self.tracker_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(ref flag) = self.quad_track_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        log::info!("Cleaned up background thread lifecycle flags on exit");
    }
}
