use super::*;

impl KagariApp {
    pub fn ensure_panel_edit_snapshot(&mut self, panel: &str, ui: &eframe::egui::Ui) {
        self.ensure_panel_edit_snapshot_for_rect(panel, ui.ctx(), ui.max_rect());
    }

    pub fn ensure_panel_edit_snapshot_for_rect(
        &mut self,
        panel: &str,
        ctx: &eframe::egui::Context,
        panel_rect: eframe::egui::Rect,
    ) {
        if self.drag_active() {
            return;
        }
        let keyboard_input_in_panel = ctx.wants_keyboard_input()
            && ctx
                .memory(|memory| memory.focused())
                .and_then(|id| ctx.read_response(id))
                .is_some_and(|response| panel_rect.intersects(response.rect));
        let edit_input = ctx.input(|input| {
            input.events.iter().any(|event| match event {
                eframe::egui::Event::PointerButton { pos, pressed, .. } => {
                    *pressed && panel_rect.contains(*pos)
                }
                eframe::egui::Event::Key { pressed, .. } => *pressed && keyboard_input_in_panel,
                eframe::egui::Event::Text(_)
                | eframe::egui::Event::Paste(_)
                | eframe::egui::Event::Cut => keyboard_input_in_panel,
                eframe::egui::Event::MouseWheel { .. } => input
                    .pointer
                    .hover_pos()
                    .is_some_and(|position| panel_rect.contains(position)),
                _ => false,
            })
        });
        if !edit_input {
            return;
        }
        let session = self.project_session_id;
        let generation = self.history.generation();
        let current = self.panel_edit_snapshot.as_ref().is_some_and(
            |(owner, snapshot_session, snapshot_generation, _)| {
                owner == panel && *snapshot_session == session && *snapshot_generation == generation
            },
        );
        if !current {
            self.panel_edit_snapshot = Some((
                panel.to_owned(),
                session,
                generation,
                self.history.current().clone(),
            ));
        }
    }

    pub fn take_panel_edit_snapshot(&mut self, panel: &str) -> Option<Project> {
        let (owner, session, generation, project) = self.panel_edit_snapshot.take()?;
        (owner == panel
            && session == self.project_session_id
            && generation == self.history.generation())
        .then_some(project)
    }

    pub fn replace_project_history(&mut self, project: Project) {
        self.history = crate::core::history::ProjectHistory::new(project);
        self.project_session_id = next_project_session_id();
        self.panel_edit_snapshot = None;
        self.production_document = None;
        self.doc_sync_generation = 0;
        self.color_lut_path = None;
        self.color_lut_mode = 0;
        self.pending_color_lut_load = None;
        crate::core::ocio_color::set_active_lut(None);
        self.clear_automation_history();
        if let Some(playback) = self.audio_playback.as_mut() {
            playback.pause();
        }
        if let Some(path) = self.playback.cached_audio_path.take() {
            let _ = std::fs::remove_file(path);
        }
        self.playback.cached_audio_key = 0;
    }

    pub fn persist_color_lut_path(&mut self, path: String) {
        self.color_lut_path = Some(path.clone());
        self.color_lut_mode = 3;
        if self.production_document.is_none() {
            self.production_document =
                Some(crate::core::production_document::ProductionDocument::new(
                    self.history.current().clone(),
                ));
            self.doc_sync_generation = self.history.generation();
        }
        if let Some(document) = self.production_document.as_mut() {
            document.color_lut_path = Some(path);
        }
        self.autosave.mark_dirty();
    }

    pub fn has_blocking_dialog(&self) -> bool {
        self.pending_project_load.is_some()
            || self.export.show_export_dialog
            || self.show_recovery_dialog
            || self.show_comp_settings
            || self.show_shortcuts_dialog
            || self.show_precompose_dialog
            || self.show_command_palette
            || self.show_welcome
            || self.show_new_comp_dialog
            || self.show_preferences
            || self.show_exit_confirmation
            || self.show_close_project_confirmation
    }

    pub fn master_dsp_params(&self) -> crate::core::audio_engine::MasterDspParams {
        if !self.master_dsp_enabled || self.master_dsp_bypass {
            return crate::core::audio_engine::MasterDspParams::bypass();
        }
        crate::core::audio_engine::MasterDspParams {
            eq_highpass: self.master_eq_highpass,
            eq_lowpass: self.master_eq_lowpass,
            eq_mid_gain: self.master_eq_mid_gain,
            eq_mid_freq: self.master_eq_mid_freq,
            comp_threshold: self.master_comp_threshold,
            comp_ratio: self.master_comp_ratio,
            comp_attack: self.master_comp_attack,
            comp_release: self.master_comp_release,
            comp_makeup: self.master_comp_makeup,
            auto_gain_enabled: self.master_auto_gain,
            auto_gain_target_db: self.master_target_level_db,
            limiter_enabled: self.master_limiter_enabled,
            limiter_ceiling_db: self.master_limiter_ceiling_db,
            wet_dry: self.master_wet_dry,
        }
    }

    pub fn audio_correction_settings(&self) -> crate::core::audio_types::AudioCorrectionSettings {
        crate::core::audio_types::AudioCorrectionSettings {
            enabled: self.master_dsp_enabled,
            auto_gain: self.master_auto_gain,
            target_level_db: self.master_target_level_db,
            highpass_hz: self.master_eq_highpass,
            lowpass_hz: self.master_eq_lowpass,
            presence_gain_db: self.master_eq_mid_gain,
            presence_freq_hz: self.master_eq_mid_freq,
            compressor_threshold_db: self.master_comp_threshold,
            compressor_ratio: self.master_comp_ratio,
            compressor_attack_ms: self.master_comp_attack,
            compressor_release_ms: self.master_comp_release,
            compressor_makeup_db: self.master_comp_makeup,
            limiter_enabled: self.master_limiter_enabled,
            limiter_ceiling_db: self.master_limiter_ceiling_db,
            wet_dry: self.master_wet_dry,
        }
    }

    pub fn apply_audio_correction_settings(
        &mut self,
        settings: crate::core::audio_types::AudioCorrectionSettings,
    ) {
        self.master_dsp_enabled = settings.enabled;
        self.master_dsp_bypass = false;
        self.master_auto_gain = settings.auto_gain;
        self.master_target_level_db = settings.target_level_db;
        self.master_eq_highpass = settings.highpass_hz;
        self.master_eq_lowpass = settings.lowpass_hz;
        self.master_eq_mid_gain = settings.presence_gain_db;
        self.master_eq_mid_freq = settings.presence_freq_hz;
        self.master_comp_threshold = settings.compressor_threshold_db;
        self.master_comp_ratio = settings.compressor_ratio;
        self.master_comp_attack = settings.compressor_attack_ms;
        self.master_comp_release = settings.compressor_release_ms;
        self.master_comp_makeup = settings.compressor_makeup_db;
        self.master_limiter_enabled = settings.limiter_enabled;
        self.master_limiter_ceiling_db = settings.limiter_ceiling_db;
        self.master_wet_dry = settings.wet_dry;
    }

    /// Commit a project snapshot and keep history, render caches, and crash
    /// recovery in sync. Returns whether the snapshot changed the project.
    pub fn commit_project(&mut self, project: Project) -> bool {
        let generation_before = self.history.generation();
        self.history.commit(project);
        let changed = self.history.generation() != generation_before;
        if changed {
            crate::core::frame_cache::bump_version();
            self.autosave.mark_dirty();
            self.autosave_history_generation = self.history.generation();
            self.frame_cache.collect_garbage();
        }
        changed
    }

    pub fn modify_project(&mut self, f: impl FnOnce(&mut Project)) {
        let mut next_project = self.history.current().clone();
        f(&mut next_project);
        self.commit_project(next_project);
    }

    pub fn begin_drag(&mut self, label: &'static str) {
        if self.drag_tx.is_some() {
            // Unbalanced re-begin: seal the previous gesture as its own undo
            // entry instead of letting one transaction swallow multiple
            // gestures (which would produce a single giant undo step).
            self.commit_drag();
        }
        self.drag_tx = Some(DragTransaction::new(self.history.current().clone(), label));
    }

    pub fn begin_drag_with_snapshot(&mut self, snapshot: Project, label: &'static str) {
        if self.drag_tx.is_some() {
            self.commit_drag();
        }
        self.drag_tx = Some(DragTransaction::new(snapshot, label));
    }

    /// Whether a drag transaction is currently open.
    pub fn drag_active(&self) -> bool {
        self.drag_tx.is_some()
    }

    pub fn commit_drag(&mut self) {
        if let Some(tx) = self.drag_tx.take() {
            let current = self.history.current().clone();
            // No-op: pre-edit snapshot equals current state (nothing changed)
            let unchanged = match (
                serde_json::to_vec(&current),
                serde_json::to_vec(&tx.snapshot),
            ) {
                (Ok(current), Ok(snapshot)) => current == snapshot,
                _ => false,
            };
            if unchanged {
                return;
            }
            // Restore pre-edit state at current index, push post-edit as new entry.
            // This is needed because current_mut() already mutated the stack entry
            // in place, so commit_action's comparison would always see equality.
            self.history
                .commit_drag_action(tx.snapshot, current, tx.label);
            crate::core::frame_cache::bump_version();
            self.autosave.mark_dirty();
            self.autosave_history_generation = self.history.generation();
            self.frame_cache.collect_garbage();
        }
    }

    pub fn record_automation_edit(
        &mut self,
        before: Vec<crate::core::automation_binding::AutomationBinding>,
    ) -> bool {
        if self
            .production_document
            .as_ref()
            .is_some_and(|document| document.bindings != before)
        {
            self.automation_undo.push(before);
            self.automation_redo.clear();
            crate::core::frame_cache::bump_version();
            return true;
        }
        false
    }

    pub fn clear_automation_history(&mut self) {
        self.automation_undo.clear();
        self.automation_redo.clear();
        self.selected_automation_binding = None;
    }

    pub fn undo_automation_edit(&mut self) -> bool {
        let Some(document) = self.production_document.as_mut() else {
            return false;
        };
        let Some(previous) = self.automation_undo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut document.bindings, previous);
        self.automation_redo.push(current);
        crate::core::frame_cache::bump_version();
        true
    }

    pub fn redo_automation_edit(&mut self) -> bool {
        let Some(document) = self.production_document.as_mut() else {
            return false;
        };
        let Some(next) = self.automation_redo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut document.bindings, next);
        self.automation_undo.push(current);
        crate::core::frame_cache::bump_version();
        true
    }

    pub fn cancel_drag(&mut self) {
        if let Some(tx) = self.drag_tx.take() {
            self.history.restore_current_without_history(tx.snapshot);
            crate::core::frame_cache::bump_version();
        }
    }
}
