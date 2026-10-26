use super::PaletteCommand;

pub fn get_general_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand {
            name: "Add Effect: Gaussian Blur",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let mut added = false;
                    app.modify_project(|project| {
                        let comp = project.active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx]
                                .effects
                                .push(crate::core::timeline::Effect {
                                    id: format!("blur_{}", len),
                                    name: "Gaussian Blur".to_string(),
                                    effect_type: crate::core::timeline::EffectType::GaussianBlur {
                                        blur_radius:
                                            crate::core::property::Animatable::new_constant(10.0),
                                    },
                                    enabled: true,
                                });
                            added = true;
                        }
                    });
                    if added {
                        app.toasts.info("Added Gaussian Blur");
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Effect: Glow / Bloom",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx]
                            .effects
                            .push(crate::core::timeline::Effect {
                                id: format!("glow_{}", len),
                                name: "Glow / Bloom".to_string(),
                                effect_type: crate::core::timeline::EffectType::Glow {
                                    threshold: crate::core::property::Animatable::new_constant(0.7),
                                    radius: crate::core::property::Animatable::new_constant(15.0),
                                    intensity: crate::core::property::Animatable::new_constant(1.5),
                                    color: crate::core::property::Animatable::new_constant([
                                        1.0, 1.0, 1.0, 1.0,
                                    ]),
                                },
                                enabled: true,
                            });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Effect: Color Tint",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx]
                            .effects
                            .push(crate::core::timeline::Effect {
                                id: format!("tint_{}", len),
                                name: "Color Tint".to_string(),
                                effect_type: crate::core::timeline::EffectType::ColorTint {
                                    color: crate::core::property::Animatable::new_constant([
                                        1.0, 0.2, 0.4, 1.0,
                                    ]),
                                    intensity: crate::core::property::Animatable::new_constant(1.0),
                                },
                                enabled: true,
                            });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Layer: New Solid Layer",
            category: "Layer",
            shortcut_hint: "Cmd+Y",
            action: Box::new(|app| {
                crate::ui::commands::layer::create_new_layer(
                    app,
                    crate::ui::commands::layer::NewLayerKind::Solid,
                );
            }),
        },
        PaletteCommand {
            name: "Add Layer: New Text Layer",
            category: "Layer",
            shortcut_hint: "Cmd+Alt+Shift+T",
            action: Box::new(|app| {
                crate::ui::commands::layer::create_new_layer(
                    app,
                    crate::ui::commands::layer::NewLayerKind::Text,
                );
            }),
        },
        PaletteCommand {
            name: "Layer: Reset Transform",
            category: "Transform",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    app.modify_project(|project| {
                        if let Some(layer) = project.active_composition_mut().layers.get_mut(idx) {
                            layer.transform = crate::core::timeline::Transform2D::default();
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Composition: Settings",
            category: "Composition",
            shortcut_hint: "Cmd+Shift+E",
            action: Box::new(|app| {
                app.comp_settings_draft = Some(app.history.current().clone());
                app.show_comp_settings = true;
            }),
        },
        PaletteCommand {
            name: "Export: Render Video (Async)",
            category: "Export",
            shortcut_hint: "Cmd+M",
            action: Box::new(|app| {
                app.export.show_export_dialog = true;
            }),
        },
        PaletteCommand {
            name: "View: Toggle Composition Grid",
            category: "Viewport",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_grid = !app.show_grid;
            }),
        },
        PaletteCommand {
            name: "View: Toggle Safe Guides",
            category: "Viewport",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_guides = !app.show_guides;
            }),
        },
        PaletteCommand {
            name: "Keyframe Assistant: Convert Audio to Keyframes",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let keyframes = crate::core::audio_engine::convert_audio_to_keyframes(comp);

                let dur = comp.duration_frames;
                let mut null_layer = crate::core::timeline::Layer::new_null(
                    comp.next_layer_id("audio_amp"),
                    "Audio Amplitude".to_string(),
                    dur,
                );

                // Populate animated slider keyframes
                let mut pos_kf = Vec::new();
                for kf in keyframes {
                    pos_kf.push(crate::core::keyframe::Keyframe {
                        frame: kf.frame,
                        value: [kf.both_amp, kf.both_amp],
                        interpolation: crate::core::keyframe::InterpolationType::Linear,
                    });
                }
                null_layer.transform.position =
                    crate::core::property::Animatable::new_animated(pos_kf);

                comp.add_layer(null_layer);
                app.selection.selected_layer_idx = Some(comp.layers.len() - 1);
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Keyframe Assistant: Rove Across Time (Position)",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            if let Some(kfs) = l.transform.position.keyframes_mut() {
                                if kfs.len() >= 3 {
                                    // AE roving: interior keyframes slide along time
                                    // so velocity stays constant across the path.
                                    let rove: Vec<usize> = (1..kfs.len() - 1).collect();
                                    crate::core::spatial_keyframe::smooth_keyframe_velocity(
                                        kfs, &rove,
                                    );
                                }
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Enable Time Remapping",
            category: "Layer",
            shortcut_hint: "Cmd+Opt+T",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    app.modify_project(|project| {
                        let comp = project.active_composition_mut();
                        if let Some(layer) = comp.layers.get_mut(idx) {
                            layer.enable_time_remapping();
                        }
                    });
                    app.toasts.info("Time Remapping enabled");
                } else {
                    app.toasts.info("Select a layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Stabilize Motion (from Track)",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let mut baked_count = 0usize;
                    app.modify_project(|p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            baked_count = crate::core::stabilizer::stabilize_layer_smoothed(l, 2);
                        }
                    });
                    if baked_count > 0 {
                        app.toasts.info(format!(
                            "Stabilized: {} position keyframes baked",
                            baked_count
                        ));
                    } else {
                        app.toasts
                            .error("Layer has no tracked data — run the Tracker first");
                    }
                } else {
                    app.toasts.info("Select a layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Center in Comp",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let dims = {
                        let c = app.history.current().active_composition();
                        (c.width as f32 / 2.0, c.height as f32 / 2.0)
                    };
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            l.transform.position =
                                crate::core::property::Animatable::new_constant([dims.0, dims.1]);
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let dims = {
                        let c = app.history.current().active_composition();
                        (c.width as f32, c.height as f32)
                    };
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[0] > 1.0 && bs[1] > 1.0 {
                                let s = (dims.0 / bs[0]).max(dims.1 / bs[1]) * 100.0;
                                l.transform.scale =
                                    crate::core::property::Animatable::new_constant([s, s]);
                                l.transform.position =
                                    crate::core::property::Animatable::new_constant([
                                        dims.0 / 2.0,
                                        dims.1 / 2.0,
                                    ]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Flip Horizontal",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let cf = app.playback.current_frame;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let s = l.transform.scale.evaluate(cf);
                            l.transform.scale =
                                crate::core::property::Animatable::new_constant([-s[0], s[1]]);
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Export: PNG Image Sequence",
            category: "File",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.export_codec_idx = 3;
                app.toasts
                    .info("Codec set to PNG Sequence — open Export to render");
            }),
        },
        PaletteCommand {
            name: "Edit: Undo History Panel",
            category: "Edit",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_history_panel = !app.show_history_panel;
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp Width",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let cw = app.history.current().active_composition().width as f32;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[0] > 1.0 {
                                let s = cw / bs[0] * 100.0;
                                l.transform.scale =
                                    crate::core::property::Animatable::new_constant([s, s]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp Height",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let ch = app.history.current().active_composition().height as f32;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[1] > 1.0 {
                                let s = ch / bs[1] * 100.0;
                                l.transform.scale =
                                    crate::core::property::Animatable::new_constant([s, s]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Tool: Puppet Pin",
            category: "Tools",
            shortcut_hint: "Cmd+P",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::PuppetPin;
                app.toasts
                    .info("Puppet Pin tool — click viewport to place pins");
            }),
        },
        PaletteCommand {
            name: "Layer: Add Puppet Pin at Center",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    let center = app
                        .history
                        .current()
                        .active_composition()
                        .layers
                        .get(idx)
                        .map(|l| l.transform.position.evaluate(app.playback.current_frame))
                        .unwrap_or([0.0, 0.0]);
                    let proj = app.history.current_mut().active_composition_mut();
                    if let Some(l) = proj.layers.get_mut(idx) {
                        let n = l.puppet_pins.len() + 1;
                        l.puppet_pins.push(crate::core::timeline::PuppetPin::new(
                            format!("pin_{}", n),
                            format!("Pin {}", n),
                            center,
                        ));
                        app.toasts.info(format!("Puppet pin {} added", n));
                    }
                } else {
                    app.toasts.info("Select a layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Tool: Brush",
            category: "Tools",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::Brush;
                app.toasts
                    .info("Brush — drag in viewport to paint on the selected layer");
            }),
        },
        PaletteCommand {
            name: "Tool: Eraser (removes strokes)",
            category: "Tools",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::Eraser;
            }),
        },
        PaletteCommand {
            name: "Import: Blender Camera Track",
            category: "File",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Camera Track JSON", &["json"])
                    .pick_file()
                {
                    match crate::core::project_migration::read_bounded_text_file(
                        &path,
                        32 * 1024 * 1024,
                    )
                    .and_then(|s| crate::core::camera_track::BlenderCamTrack::parse(&s))
                    {
                        Ok(track) => {
                            let mut baked = 0usize;
                            app.modify_project(|p| {
                                baked = track.apply_to_comp(p.active_composition_mut(), true);
                            });
                            app.toasts
                                .info(format!("Camera track baked: {} kfs", baked));
                        }
                        Err(e) => app.toasts.error(e),
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Import: Subtitles (.srt/.vtt)",
            category: "File",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Subtitles", &["srt", "vtt"])
                    .pick_file()
                {
                    match crate::core::project_migration::read_bounded_text_file(
                        &path,
                        8 * 1024 * 1024,
                    )
                    .map(|s| {
                        crate::core::subtitles::parse_srt(
                            &s,
                            app.history.current().active_composition().fps,
                        )
                    }) {
                        Ok(cues) if !cues.is_empty() => {
                            let (cw, ch) = {
                                let cc = app.history.current().active_composition();
                                (cc.width as f32, cc.height as f32)
                            };
                            let layers = crate::core::subtitles::cues_to_layers(&cues, cw, ch, 48);
                            let n = layers.len();
                            let proj = app.history.current_mut();
                            let comp = proj.active_composition_mut();
                            for l in layers {
                                comp.add_layer(l);
                            }
                            crate::core::frame_cache::bump_version();
                            app.toasts.info(format!("{} caption layers created", n));
                        }
                        Ok(_) => app.toasts.error("No cues found"),
                        Err(e) => app.toasts.error(e),
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Export: Captions (.srt)",
            category: "File",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Subtitles", &["srt"])
                    .set_file_name("captions.srt")
                    .save_file()
                {
                    let proj = app.history.current();
                    let comp = proj.active_composition();
                    let srt = crate::core::subtitles::layers_to_srt(&comp.layers, comp.fps);
                    if srt.is_empty() {
                        app.toasts.error("No caption layers found");
                    } else if let Err(e) = std::fs::write(&path, &srt) {
                        app.toasts.error(format!("Write failed: {}", e));
                    } else {
                        app.toasts.info("Captions exported");
                    }
                }
            }),
        },
    ]
}
