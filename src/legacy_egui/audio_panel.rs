use crate::ui::kui;
use crate::ui::localization::{tr, tr_format};
use crate::KagariApp;
use eframe::egui;

pub fn draw_audio_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::group(ui, "Audio Levels & Panning", |ui| {
        let grid = kui::PropertyGrid::new(ui, &["Master Audio Level", "L/R Pan"]);
        grid.row(ui, "Master Audio Level", |ui| {
            kui::parameter_slider(ui, &mut app.playback.master_volume, 0.0..=2.0, "×");
        });

        let master_volume = app.playback.master_volume;
        let db = if master_volume > 0.001 {
            20.0 * master_volume.log10()
        } else {
            -60.0
        };
        kui::caption(
            ui,
            format!(
                "{} dB",
                tr_format("Master Level: {}", &[&format!("{db:.1}")])
            ),
        );
        let left_peak = ((db + 60.0) / 72.0).clamp(0.0, 1.0);
        let right_db = db - 0.5;
        kui::level_meter(ui, "L", left_peak, &format!("{db:.1} dB"));
        kui::level_meter(
            ui,
            "R",
            ((right_db + 60.0) / 72.0).clamp(0.0, 1.0),
            &format!("{right_db:.1} dB"),
        );
    });

    let layer_info = app.selection.selected_layer_idx.and_then(|index| {
        app.history
            .current()
            .active_composition()
            .layers
            .get(index)
            .map(|layer| (index, layer.name.clone()))
    });

    let Some((index, layer_name)) = layer_info else {
        kui::caption(
            ui,
            tr("Select a layer to adjust audio pan and waveform settings."),
        );
        return;
    };

    let selected_label = tr_format("Selected Layer: {}", &[&layer_name]);
    kui::group(ui, "Layer Audio", |ui| {
        kui::secondary(ui, selected_label);
        let channel_count = app.history.current().active_composition().layers.len();
        app.audio_mixer_channels
            .resize(channel_count, Default::default());
        let grid = kui::PropertyGrid::new(ui, &["L/R Pan"]);
        grid.row(ui, "L/R Pan", |ui| {
            if let Some(channel) = app.audio_mixer_channels.get_mut(index) {
                if kui::parameter_slider(ui, &mut channel.pan, -100.0..=100.0, "%").changed() {
                    app.autosave.mark_dirty();
                    app.playback.cached_audio_key = 0;
                }
            }
        });

        let mut waveform_on = app.expanded_waveform_layers.contains(&index);
        if kui::switch(ui, &mut waveform_on, "Show Layer Audio Waveform (L)").changed() {
            if waveform_on {
                app.expanded_waveform_layers.insert(index);
            } else {
                app.expanded_waveform_layers.remove(&index);
            }
        }
    });

    kui::group(ui, "Audio-Reactive Tools", |ui| {
        ui.horizontal_wrapped(|ui| {
            if kui::button(ui, "Bind Scale to Bass", kui::ButtonKind::Accent)
                .on_hover_text(tr("Increase Scale from the analyzed bass band"))
                .clicked()
            {
                let mut project = app.history.current().clone();
                let composition = project.active_composition_mut();
                if index < composition.layers.len() {
                    composition.layers[index].transform.scale_expression =
                        Some(crate::core::timeline::Expression::Raw(
                            "[value[0] + audioAmplitude(\"bass\") * 15.0, value[1] + audioAmplitude(\"bass\") * 15.0]".into(),
                        ));
                    app.commit_project(project);
                    app.toasts.info(tr_format("Scale now responds to bass on {}", &[&layer_name]));
                }
            }
            if kui::button(ui, "Add Audio Spectrum", kui::ButtonKind::Default)
                .on_hover_text(tr("Add a spectrum overlay driven by the composition audio"))
                .clicked()
            {
                let mut project = app.history.current().clone();
                let composition = project.active_composition_mut();
                if index < composition.layers.len() {
                    let effect_index = composition.layers[index].effects.len();
                    composition.layers[index].effects.push(crate::core::timeline::Effect {
                        id: format!("audio_spectrum_{effect_index}"),
                        name: "Audio Spectrum".to_owned(),
                        effect_type: crate::core::timeline::EffectType::AudioSpectrum {
                            enabled: crate::core::property::Animatable::new_constant(1.0),
                            bands: crate::core::property::Animatable::new_constant(5.0),
                            opacity: crate::core::property::Animatable::new_constant(1.0),
                            color_start: [0.0, 0.8, 1.0, 1.0],
                            color_end: [0.8, 0.2, 1.0, 1.0],
                            position_x: crate::core::property::Animatable::new_constant(0.05),
                            position_y: crate::core::property::Animatable::new_constant(0.05),
                            width: crate::core::property::Animatable::new_constant(0.9),
                            height: crate::core::property::Animatable::new_constant(0.25),
                        },
                        enabled: true,
                    });
                    app.commit_project(project);
                    app.toasts.info(tr_format("Added Audio Spectrum to {}", &[&layer_name]));
                }
            }
        });
    });

    kui::group(ui, "Audio Keyframe Assistant", |ui| {
        kui::caption(ui, tr("Bake audio amplitude waveform into Slider Control keyframes (Both / Left / Right Channels)"));
        if kui::button(ui, "Convert Audio to Keyframes", kui::ButtonKind::Accent)
            .on_hover_text(tr("Bake audio amplitude waveform into Slider Control keyframes (Both / Left / Right Channels)"))
            .clicked()
        {
            let source = app
                .history
                .current()
                .active_composition()
                .layers
                .get(index)
                .and_then(|layer| match &layer.layer_type {
                    crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                    crate::core::timeline::LayerType::Video { audio_wav: Some(path), .. } => {
                        Some(path.clone())
                    }
                    _ => None,
                });
            let Some(source) = source else {
                app.toasts.error(tr("Selected layer has no readable audio source"));
                return;
            };
            let mut project = app.history.current().clone();
            match crate::core::audio_to_keyframes::convert_audio_to_keyframes(
                project.active_composition_mut(),
                &source,
            ) {
                Ok(name) => {
                    app.commit_project(project);
                    app.toasts.info(tr_format(
                    "Created {} with baked Slider Control keyframes",
                        &[&name],
                    ));
                }
                Err(error) => app.toasts.error(error),
            }
        }
    });
}
