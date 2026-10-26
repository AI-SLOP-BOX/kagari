use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::ui::localization::{tr, tr_format};

fn rescale_track<T: Clone>(track: &mut Animatable<T>, in_frame: u32, factor: f32) {
    if let Animatable::Animated(kfs) = track {
        for kf in kfs.iter_mut() {
            let rel = kf.frame.saturating_sub(in_frame) as f32 * factor;
            kf.frame = in_frame.saturating_add(rel.round() as u32);
        }
        kfs.sort_by_key(|k| k.frame);
    }
}

pub fn draw_time_remap_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "Time Stretch & Time Remapping");
    kui::separator(ui);

    let layer_info = if let Some(idx) = app.selection.selected_layer_idx {
        let comp = app.history.current().active_composition();
        if idx < comp.layers.len() {
            Some((
                idx,
                comp.layers[idx].name.clone(),
                comp.layers[idx].in_frame,
                comp.layers[idx].out_frame,
            ))
        } else {
            None
        }
    } else {
        None
    };

    if let Some((idx, layer_name, in_frame, out_frame)) = layer_info {
        kui::secondary(ui, tr_format("Selected Layer: {}", &[&layer_name]));

        ui.add_space(kui::space::XS);
        if kui::button(ui, "Enable Time Remapping (Cmd+Alt+T)", ButtonKind::Accent)
            .on_hover_text(tr("Adds Time Remap keyframe track for speed control"))
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if idx < comp_mut.layers.len() {
                let dur = out_frame - in_frame;
                comp_mut.layers[idx].time_remap = Some(Animatable::Animated(vec![
                    Keyframe::new(in_frame, 0.0, InterpolationType::Linear),
                    Keyframe::new(out_frame, dur as f32, InterpolationType::Linear),
                ]));
                app.commit_project(temp_proj);
                app.toasts
                    .info(tr_format("Enabled Time Remapping on {}", &[&layer_name]));
            }
        }

        ui.horizontal(|ui| {
            if kui::button(ui, "Freeze Frame at Playhead", ButtonKind::Default)
                .on_hover_text(tr("Locks layer playback to the current frame"))
                .clicked()
            {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let cur_f = app.playback.current_frame.saturating_sub(in_frame) as f32;
                    comp_mut.layers[idx].time_remap = Some(Animatable::new_constant(cur_f));
                    app.commit_project(temp_proj);
                    app.toasts.info(tr_format(
                        "Froze {} at frame {}",
                        &[&layer_name, &app.playback.current_frame.to_string()],
                    ));
                }
            }
            if kui::button(ui, "⏪ Reverse Layer Time", ButtonKind::Default)
                .on_hover_text(tr("Plays the layer backwards"))
                .clicked()
            {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let dur = out_frame - in_frame;
                    comp_mut.layers[idx].time_remap = Some(Animatable::Animated(vec![
                        Keyframe::new(in_frame, dur as f32, InterpolationType::Linear),
                        Keyframe::new(out_frame, 0.0, InterpolationType::Linear),
                    ]));
                    app.commit_project(temp_proj);
                    app.toasts
                        .info(tr_format("Reversed playback on {}", &[&layer_name]));
                }
            }
        });

        // ── Loop: bake repeating time-remap keyframes over the layer duration ──
        ui.add_space(kui::space::SM);
        kui::section_title(ui, "Auto Loop (bakes time-remap keys)");
        let comp_dur = app.history.current().active_composition().duration_frames;
        ui.horizontal_wrapped(|ui| {
            for (mode, tip) in [
                (
                    crate::core::time_remap::LoopMode::Cycle,
                    tr("Repeat source forward continuously"),
                ),
                (
                    crate::core::time_remap::LoopMode::PingPong,
                    tr("Alternate forward / reverse"),
                ),
            ] {
                let button_label = if mode == crate::core::time_remap::LoopMode::Cycle {
                    "Loop Cycle"
                } else {
                    "Loop PingPong"
                };
                if kui::button(ui, button_label, ButtonKind::Default)
                    .on_hover_text(tip)
                    .clicked()
                {
                    match crate::core::time_remap::loop_keyframes(
                        in_frame, out_frame, comp_dur, mode,
                    ) {
                        Ok(kfs) => {
                            let mut temp_proj = app.history.current().clone();
                            let comp_mut = temp_proj.active_composition_mut();
                            if idx < comp_mut.layers.len() {
                                comp_mut.layers[idx].time_remap = Some(Animatable::Animated(kfs));
                                app.commit_project(temp_proj);
                                app.toasts.info(tr_format(
                                    "Baked {} loop onto {}",
                                    &[&tr(mode.label()), &layer_name],
                                ));
                            }
                        }
                        Err(error) => app.toasts.error(localized_loop_error(&error)),
                    }
                }
            }
        });

        ui.add_space(kui::space::SM);
        kui::separator(ui);

        // ── Time Stretch: scales duration AND all keyframe times around the In point ──
        let stretch_id = egui::Id::new(format!("ae_time_stretch_{}", idx));
        let mut stretch_factor: f32 = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or_insert_with(stretch_id, || 100.0));
        let stretch_label = tr("Stretch Factor:");
        let stretch_grid = kui::PropertyGrid::new(ui, &[&stretch_label]);
        if stretch_grid
            .row(ui, &stretch_label, |ui| {
                ui.add(
                    kui::drag(&mut stretch_factor)
                        .range(1.0..=1000.0)
                        .unit(" %")
                        .speed(1.0),
                )
            })
            .changed()
        {
            ui.ctx()
                .data_mut(|data| data.insert_temp(stretch_id, stretch_factor));
        }
        ui.horizontal_wrapped(|ui| {
            if kui::button(ui, "Apply Stretch", ButtonKind::Accent)
                .on_hover_text(tr(
                    "200% = half speed (duration doubles), 50% = double speed",
                ))
                .clicked()
            {
                let factor = (stretch_factor / 100.0).max(0.01);
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let layer = &mut comp_mut.layers[idx];
                    let span = (layer.out_frame - layer.in_frame).max(1);
                    let new_span = ((span as f32 * factor).round() as u32).max(1);
                    layer.out_frame = layer.in_frame + new_span;

                    let in_f = layer.in_frame;
                    rescale_track(&mut layer.transform.position, in_f, factor);
                    rescale_track(&mut layer.transform.scale, in_f, factor);
                    rescale_track(&mut layer.transform.rotation, in_f, factor);
                    rescale_track(&mut layer.transform.opacity, in_f, factor);
                    rescale_track(&mut layer.transform.anchor_point, in_f, factor);
                    if let Some(remap) = layer.time_remap.as_mut() {
                        rescale_track(remap, in_f, factor);
                    }
                    app.commit_project(temp_proj);
                    app.toasts.info(tr_format(
                        "Stretched {} to {}% ({} → {} frames)",
                        &[
                            &layer_name,
                            &format!("{stretch_factor:.0}"),
                            &span.to_string(),
                            &new_span.to_string(),
                        ],
                    ));
                }
            }
        });
        kui::caption(ui, "Keyframe times on Position / Scale / Rotation / Opacity / Anchor / Time Remap are rescaled relative to the layer In point.");

        ui.add_space(kui::space::SM);
        let blend_id = egui::Id::new("ae_frame_blending_mode");
        let mut blend_idx = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or_insert_with(blend_id, || 0));
        let blend_label = tr("Frame Blending Mode:");
        let blend_grid = kui::PropertyGrid::new(ui, &[&blend_label]);
        if blend_grid.row(ui, &blend_label, |ui| {
            kui::combo(
                ui,
                blend_id,
                &mut blend_idx,
                &[(0, "Off"), (1, "Frame Mix"), (2, "Pixel Motion")],
            )
        }) {
            ui.ctx()
                .data_mut(|data| data.insert_temp(blend_id, blend_idx));
        }

        ui.add_space(kui::space::XS);
        let mut preserve_fps = ui.ctx().data(|d| {
            d.get_temp::<bool>(egui::Id::new("ae_preserve_fps"))
                .unwrap_or(true)
        });
        if kui::toggle(
            ui,
            &mut preserve_fps,
            "Preserve Frame Rate when nested or in render queue",
        )
        .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("ae_preserve_fps"), preserve_fps));
            crate::core::frame_cache::bump_version();
        }

        ui.add_space(kui::space::SM);
        kui::separator(ui);
        kui::section_title(ui, "Audio Time-Stretch & Pitch Correction");
        let mut preserve_pitch = ui.ctx().data(|d| {
            d.get_temp::<bool>(egui::Id::new("ae_audio_preserve_pitch"))
                .unwrap_or(true)
        });
        if kui::toggle(
            ui,
            &mut preserve_pitch,
            "Preserve Audio Pitch (WSOLA Timestretch)",
        )
        .changed()
        {
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new("ae_audio_preserve_pitch"), preserve_pitch)
            });
            crate::core::frame_cache::bump_version();
        }
        let mut pitch_semitones = ui.ctx().data(|d| {
            d.get_temp::<f32>(egui::Id::new("ae_audio_pitch_shift"))
                .unwrap_or(0.0)
        });
        let pitch_label = tr("Pitch Shift:");
        let pitch_grid = kui::PropertyGrid::new(ui, &[&pitch_label]);
        if pitch_grid
            .row(ui, &pitch_label, |ui| {
                ui.add(
                    kui::drag(&mut pitch_semitones)
                        .range(-24.0..=24.0)
                        .speed(0.5)
                        .unit(" st"),
                )
            })
            .changed()
        {
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new("ae_audio_pitch_shift"), pitch_semitones)
            });
            crate::core::frame_cache::bump_version();
        }
    } else {
        kui::empty_state(
            ui,
            "Select a layer to adjust time stretch & remapping.",
            72.0,
        );
    }
}

fn localized_loop_error(error: &str) -> String {
    if error == "The selected layer starts after the composition ends." {
        return tr(error);
    }
    if let Some((count, _)) = error
        .strip_prefix("Loop would create ")
        .and_then(|message| message.split_once(" keyframes; the limit is "))
    {
        if let Some((limit, _)) = error
            .split_once(" keyframes; the limit is ")
            .and_then(|(_, rest)| rest.split_once(". Increase the source duration"))
        {
            return tr_format(
                "Loop would create {} keyframes; limit is {}. Increase source duration or shorten the composition.",
                &[count, limit],
            );
        }
    }
    tr(error)
}
