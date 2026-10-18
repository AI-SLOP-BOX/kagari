use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_audio_mixer(app: &mut KagariApp, ui: &mut egui::Ui) {
    let correction_before = app.audio_correction_settings();
    crate::ui::kui::section(ui, "Audio Mixer");

    let layer_count = app.history.current().active_composition().layers.len();

    if app.audio_mixer_channels.len() != layer_count {
        app.audio_mixer_channels
            .resize(layer_count, Default::default());
    }
    let channels_before = app.audio_mixer_channels.clone();
    let master_volume_before = app.playback.master_volume;

    let comp_name = app.history.current().active_composition().name.clone();
    let audio_track_count = app
        .history
        .current()
        .active_composition()
        .layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.layer_type,
                crate::core::timeline::LayerType::Audio { .. }
                    | crate::core::timeline::LayerType::Video {
                        audio_wav: Some(_),
                        ..
                    }
            )
        })
        .count();
    crate::ui::kui::secondary(
        ui,
        format!(
            "{}: {}",
            crate::ui::localization::tr("Composition"),
            comp_name
        ),
    );

    ui.add_space(crate::ui::kui::space::XS);

    // ── Toolbar: Mute All / Solo All / Reset All ──
    ui.horizontal(|ui| {
        if crate::ui::kui::button(ui, "Reset All", crate::ui::kui::ButtonKind::Quiet)
            .on_hover_text(crate::ui::localization::tr(
                "Reset all channels to their defaults",
            ))
            .clicked()
        {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.gain_db = 0.0;
                ch.pan = 0.0;
                ch.mute = false;
                ch.solo = false;
            }
        }
        let has_solo = app.audio_mixer_channels.iter().any(|c| c.solo);
        if has_solo
            && crate::ui::kui::button(ui, "Clear Solo", crate::ui::kui::ButtonKind::Quiet).clicked()
        {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.solo = false;
            }
        }
        let has_mute = app.audio_mixer_channels.iter().any(|c| c.mute);
        if has_mute
            && crate::ui::kui::button(ui, "Clear Mute", crate::ui::kui::ButtonKind::Quiet).clicked()
        {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.mute = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::ui::kui::caption(
                ui,
                crate::ui::localization::tr_format(
                    "{} audio tracks",
                    &[&audio_track_count.to_string()],
                ),
            );
        });
    });

    ui.add_space(crate::ui::kui::space::XS);

    // ── Track Strips + Output Meter + Master ──
    egui::ScrollArea::horizontal()
        .max_height(280.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let layer_info: Vec<(String, bool)> = app
                    .history
                    .current()
                    .active_composition()
                    .layers
                    .iter()
                    .map(|l| {
                        let has_audio = matches!(
                            l.layer_type,
                            crate::core::timeline::LayerType::Audio { .. }
                                | crate::core::timeline::LayerType::Video {
                                    audio_wav: Some(_),
                                    ..
                                }
                        );
                        (l.name.clone(), has_audio)
                    })
                    .collect();

                if audio_track_count == 0 {
                    crate::ui::kui::secondary(ui, "No audio tracks");
                }

                for (idx, (name, has_audio)) in layer_info.iter().enumerate() {
                    if !has_audio {
                        continue;
                    }
                    let strip_width = 116.0 + 18.0;
                    let divider_width = ui.spacing().item_spacing.x + 1.0;
                    let strip_probe = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(strip_width + divider_width, 280.0),
                    );
                    if !ui.is_rect_visible(strip_probe) {
                        ui.allocate_space(strip_probe.size());
                        continue;
                    }
                    crate::ui::kui::panel(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(116.0);

                            // ── Track header: icon + name ──
                            let type_icon = "AUD";
                            ui.horizontal(|ui| {
                                crate::ui::kui::caption(ui, type_icon);
                                crate::ui::kui::secondary(ui, format!("{:02}", idx + 1));
                            });
                            crate::ui::kui::truncated_secondary(ui, name);

                            ui.add_space(crate::ui::kui::space::XS);

                            // ── Mute / Solo / Reset ──
                            let ch = &mut app.audio_mixer_channels[idx];
                            ui.horizontal(|ui| {
                                let mute = crate::ui::kui::icon_button(
                                    ui,
                                    "M",
                                    "Mute track",
                                    ch.mute,
                                    26.0,
                                );
                                if mute.clicked() || mute.changed() {
                                    ch.mute = !ch.mute;
                                }
                                let solo = crate::ui::kui::icon_button(
                                    ui,
                                    "S",
                                    "Solo track",
                                    ch.solo,
                                    26.0,
                                );
                                if solo.clicked() || solo.changed() {
                                    ch.solo = !ch.solo;
                                }
                                let reset = crate::ui::kui::icon_button(
                                    ui,
                                    "R",
                                    "Reset track settings",
                                    false,
                                    26.0,
                                );
                                if reset.clicked() || reset.changed() {
                                    ch.gain_db = 0.0;
                                    ch.pan = 0.0;
                                }
                            });

                            ui.add_space(crate::ui::kui::space::XS);

                            // Keep the fader and its post-fader meter together as one channel strip.
                            let gain_text = if ch.gain_db <= -60.0 {
                                "−∞ dB".to_owned()
                            } else {
                                format!("{:.1} dB", ch.gain_db)
                            };
                            crate::ui::kui::readout(ui, gain_text, colors::TEXT_SECONDARY);
                            ui.horizontal(|ui| {
                                crate::ui::kui::vertical_fader(
                                    ui,
                                    &mut ch.gain_db,
                                    -60.0..=12.0,
                                    108.0,
                                );

                                let level = if !ch.mute {
                                    let (left, right) = app
                                        .audio_track_meters
                                        .get(idx)
                                        .copied()
                                        .unwrap_or((0.0, 0.0));
                                    left.max(right).clamp(0.0, 1.0)
                                } else {
                                    0.0
                                };
                                crate::ui::kui::vertical_meter(ui, "", level, 12.0, 108.0);
                            });

                            // ── Pan knob (horizontal) with L/C/R labels ──
                            let pan_label = if ch.pan < -10.0 {
                                format!("L{:.0}", ch.pan.abs())
                            } else if ch.pan > 10.0 {
                                format!("R{:.0}", ch.pan)
                            } else {
                                "C".into()
                            };
                            crate::ui::kui::slider_step_without_value(
                                ui,
                                &mut ch.pan,
                                -100.0..=100.0,
                                1.0,
                            );
                            crate::ui::kui::secondary(
                                ui,
                                format!("{} {}", crate::ui::localization::tr("Pan"), pan_label),
                            );
                        });
                    });
                    crate::ui::kui::toolbar_separator(ui);
                }

                // ── Output Meters (L/R) ──
                ui.vertical(|ui| {
                    ui.set_width(60.0);
                    crate::ui::kui::section_title(ui, "Output");
                    let (meter_h, meter_w) = (140.0, 16.0);
                    ui.horizontal(|ui| {
                        for (label, level) in [("L", app.audio_meter.0), ("R", app.audio_meter.1)] {
                            ui.vertical(|ui| {
                                crate::ui::kui::vertical_meter(ui, label, level, meter_w, meter_h);

                                crate::ui::kui::caption(ui, label);
                            });
                        }
                    });
                    // dB readout
                    let db_l = if app.audio_meter.0 > 0.0 {
                        20.0 * app.audio_meter.0.log10()
                    } else {
                        -96.0
                    };
                    let db_r = if app.audio_meter.1 > 0.0 {
                        20.0 * app.audio_meter.1.log10()
                    } else {
                        -96.0
                    };
                    crate::ui::kui::readout(
                        ui,
                        format!("{:.0} / {:.0} dB", db_l, db_r),
                        colors::TEXT_SECONDARY,
                    );
                });
                crate::ui::kui::toolbar_separator(ui);

                // ── Master Channel Strip ──
                ui.vertical(|ui| {
                    ui.set_width(80.0);
                    crate::ui::kui::section_title(ui, "Master");
                    ui.add_space(crate::ui::kui::space::XS);

                    // Master volume fader
                    crate::ui::kui::readout(
                        ui,
                        format!("{:.0}%", app.playback.master_volume * 100.0),
                        colors::TEXT_SECONDARY,
                    );
                    crate::ui::kui::vertical_fader(
                        ui,
                        &mut app.playback.master_volume,
                        0.0..=1.0,
                        108.0,
                    );

                    ui.add_space(crate::ui::kui::space::XS);

                    // dB readout
                    let master_db = if app.playback.master_volume > 0.0 {
                        20.0 * app.playback.master_volume.log10()
                    } else {
                        -96.0
                    };
                    let db_color = if master_db < -3.0 {
                        colors::TEXT_SECONDARY
                    } else if master_db < 0.0 {
                        colors::ACCENT_YELLOW
                    } else {
                        colors::ACCENT_RED
                    };
                    crate::ui::kui::readout(ui, format!("{:.1} dB", master_db), db_color);

                    ui.add_space(crate::ui::kui::space::XS);
                    crate::ui::kui::caption(ui, "Volume");
                });
            });
        });

    // ── Voice correction: the same non-destructive chain is used by preview
    // and export. Advanced controls stay collapsed for first-time users.
    ui.add_space(crate::ui::kui::space::SM);
    crate::ui::kui::separator(ui);
    crate::ui::kui::section(ui, "Voice Correction");
    ui.horizontal_wrapped(|ui| {
        crate::ui::kui::toggle(ui, &mut app.master_dsp_enabled, "Enable Correction");
        let compare_label = if app.master_dsp_bypass {
            "Apply Correction"
        } else {
            "Compare to Original"
        };
        if crate::ui::kui::button(ui, compare_label, crate::ui::kui::ButtonKind::Default)
            .on_hover_text(crate::ui::localization::tr(
                "Switch between the unprocessed mix and corrected audio",
            ))
            .clicked()
        {
            app.master_dsp_bypass = !app.master_dsp_bypass;
            app.playback.cached_audio_key = 0;
        }
        if crate::ui::kui::button(ui, "Restore Defaults", crate::ui::kui::ButtonKind::Quiet)
            .on_hover_text(crate::ui::localization::tr(
                "Restore the default subtle voice correction settings",
            ))
            .clicked()
        {
            app.apply_audio_correction_settings(
                crate::core::audio_types::AudioCorrectionSettings::default(),
            );
            app.master_dsp_bypass = false;
            app.playback.cached_audio_key = 0;
        }
    });

    ui.columns(2, |columns| {
        crate::ui::kui::section(&mut columns[0], "Basic Correction");
        let grid = crate::ui::kui::PropertyGrid::new(
            &columns[0],
            &["Auto Level", "Target Level", "Correction Amount"],
        );
        grid.row(&mut columns[0], "Auto Level", |ui| {
            crate::ui::kui::toggle(ui, &mut app.master_auto_gain, "")
        });
        grid.row(&mut columns[0], "Target Level", |ui| {
            crate::ui::kui::slider(ui, &mut app.master_target_level_db, -30.0..=-6.0, " dB")
        });
        grid.row(&mut columns[0], "Correction Amount", |ui| {
            crate::ui::kui::slider(ui, &mut app.master_wet_dry, 0.0..=1.0, " mix")
        });

        crate::ui::kui::section(&mut columns[1], "Peak Protection");
        let grid = crate::ui::kui::PropertyGrid::new(&columns[1], &["Limiter", "Ceiling"]);
        grid.row(&mut columns[1], "Limiter", |ui| {
            crate::ui::kui::toggle(ui, &mut app.master_limiter_enabled, "")
        });
        grid.row(&mut columns[1], "Ceiling", |ui| {
            crate::ui::kui::slider(ui, &mut app.master_limiter_ceiling_db, -6.0..=-0.1, " dB")
        });
    });

    crate::ui::kui::collapsing_section(
        ui,
        "audio_advanced_voice",
        "Advanced Voice Tuning",
        false,
        |ui| {
            ui.columns(2, |columns| {
                crate::ui::kui::section(&mut columns[0], "Clarity EQ");
                let grid = crate::ui::kui::PropertyGrid::new(
                    &columns[0],
                    &["High-Pass Cut", "Low-Pass Cut", "Presence", "Frequency"],
                );
                grid.row(&mut columns[0], "High-Pass Cut", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_eq_highpass, 20.0..=500.0, " Hz")
                });
                grid.row(&mut columns[0], "Low-Pass Cut", |ui| {
                    crate::ui::kui::slider(
                        ui,
                        &mut app.master_eq_lowpass,
                        2_000.0..=20_000.0,
                        " Hz",
                    )
                });
                grid.row(&mut columns[0], "Presence", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_eq_mid_gain, -12.0..=12.0, " dB")
                });
                grid.row(&mut columns[0], "Frequency", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_eq_mid_freq, 200.0..=8_000.0, " Hz")
                });

                crate::ui::kui::section(&mut columns[1], "Dynamics");
                let grid = crate::ui::kui::PropertyGrid::new(
                    &columns[1],
                    &["Threshold", "Ratio", "Attack", "Release", "Makeup Gain"],
                );
                grid.row(&mut columns[1], "Threshold", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_comp_threshold, -40.0..=0.0, " dB")
                });
                grid.row(&mut columns[1], "Ratio", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_comp_ratio, 1.0..=20.0, ":1")
                });
                grid.row(&mut columns[1], "Attack", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_comp_attack, 0.1..=50.0, " ms")
                });
                grid.row(&mut columns[1], "Release", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_comp_release, 10.0..=500.0, " ms")
                });
                grid.row(&mut columns[1], "Makeup Gain", |ui| {
                    crate::ui::kui::slider(ui, &mut app.master_comp_makeup, 0.0..=24.0, " dB")
                });
            });
        },
    );

    ui.add_space(crate::ui::kui::space::XS);
    crate::ui::kui::collapsing_section(
        ui,
        "audio_keyframe_generation",
        "Generate Keyframes from Audio",
        false,
        |ui| {
            crate::ui::kui::caption(
                ui,
                "Create animation keyframes from the volume of each frequency band.",
            );
            let mut band_sel = ui.ctx().data(|d| {
                d.get_temp::<i32>(egui::Id::new("audio_crossover_band"))
                    .unwrap_or(0)
            });
            ui.horizontal_wrapped(|ui| {
                for (value, label) in [(0, "Full Mix"), (1, "Bass"), (2, "Mids"), (3, "Treble")] {
                    if crate::ui::kui::tool_button(ui, band_sel == value, label).clicked() {
                        band_sel = value;
                    }
                }
            });
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("audio_crossover_band"), band_sel));
            if crate::ui::kui::button(
                ui,
                "Bake to Null Controller",
                crate::ui::kui::ButtonKind::Accent,
            )
            .clicked()
            {
                let source = app
                    .history
                    .current()
                    .active_composition()
                    .layers
                    .iter()
                    .find_map(|layer| match &layer.layer_type {
                        crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                        crate::core::timeline::LayerType::Video {
                            audio_wav: Some(path),
                            ..
                        } => Some(path.clone()),
                        _ => None,
                    });
                let Some(source) = source else {
                    app.toasts.error(crate::ui::localization::tr(
                        "Add an audio layer or a video layer with audio first",
                    ));
                    return;
                };
                let band = match band_sel {
                    1 => crate::core::audio_dsp::AudioExtractBand::Bass,
                    2 => crate::core::audio_dsp::AudioExtractBand::Mid,
                    3 => crate::core::audio_dsp::AudioExtractBand::Treble,
                    _ => crate::core::audio_dsp::AudioExtractBand::Master,
                };
                let mut project = app.history.current().clone();
                match crate::core::audio_to_keyframes::convert_selected_band_to_keyframes(
                    project.active_composition_mut(),
                    &source,
                    band,
                ) {
                    Ok(name) => {
                        app.commit_project(project);
                        app.toasts.info(crate::ui::localization::tr_format(
                            "Created Slider Control keyframes on {}",
                            &[&name],
                        ));
                    }
                    Err(error) => app.toasts.error(error),
                }
            }
        },
    );

    if correction_before != app.audio_correction_settings()
        || channels_before != app.audio_mixer_channels
        || (master_volume_before - app.playback.master_volume).abs() > f32::EPSILON
    {
        app.autosave.mark_dirty();
        app.playback.cached_audio_key = 0;
    }
}
