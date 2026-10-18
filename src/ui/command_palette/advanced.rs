use super::PaletteCommand;

pub fn get_advanced_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand {
            name: "File: Save Project",
            category: "File",
            shortcut_hint: "Cmd+S",
            action: Box::new(|app| {
                let path = app.project_path.clone();
                if let Err(e) =
                    crate::ui::project_io::save_project_to_path(app, std::path::Path::new(&path))
                {
                    app.toasts.error(format!("Save failed: {}", e));
                }
            }),
        },
        PaletteCommand {
            name: "Composition: New Composition",
            category: "Composition",
            shortcut_hint: "Cmd+N",
            action: Box::new(|app| {
                let comp_id = app.history.current().next_composition_id("comp");
                let new_comp = crate::core::timeline::Composition::new(
                    comp_id,
                    "Composition 1".to_string(),
                    1920,
                    1080,
                    30,
                    300,
                );
                let proj = app.history.current_mut();
                proj.compositions.push(new_comp);
                proj.active_composition_idx = proj.compositions.len() - 1;
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Keyframe Assistant: Sequence Layers",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_sequence_layers = true;
            }),
        },
        PaletteCommand {
            name: "Composition: Save Frame as PNG",
            category: "Export",
            shortcut_hint: "",
            action: Box::new(|app| {
                let dir = std::env::temp_dir().join("kagari_frames");
                let _ = std::fs::create_dir_all(&dir);
                let comp = app.history.current().active_composition().clone();
                let frame = app.playback.current_frame;
                let out = dir.join(format!("{}_f{}.png", comp.name, frame));
                let pixels = crate::core::software_renderer::render_frame_to_pixels(
                    &comp,
                    frame,
                    comp.width,
                    comp.height,
                    0.0,
                    0,
                );
                match image::save_buffer(
                    &out,
                    &pixels,
                    comp.width,
                    comp.height,
                    image::ColorType::Rgba8,
                ) {
                    Ok(_) => app.toasts.info(format!("Frame saved: {}", out.display())),
                    Err(e) => app.toasts.error(format!("Save failed: {}", e)),
                }
            }),
        },
        // ── Layer Styles Commands ──
        PaletteCommand {
            name: "Layer Style: Toggle Stroke",
            category: "Layer Styles",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].style.stroke.enabled =
                            !comp.layers[idx].style.stroke.enabled;
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Layer Style: Toggle Drop Shadow",
            category: "Layer Styles",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].style.drop_shadow.enabled =
                            !comp.layers[idx].style.drop_shadow.enabled;
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Layer Style: Toggle Color Overlay",
            category: "Layer Styles",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].style.color_overlay.enabled =
                            !comp.layers[idx].style.color_overlay.enabled;
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Layer Style: Toggle Gradient Overlay",
            category: "Layer Styles",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].style.gradient_overlay.enabled =
                            !comp.layers[idx].style.gradient_overlay.enabled;
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Layer Style: Toggle Bevel & Emboss",
            category: "Layer Styles",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].style.bevel_emboss.enabled =
                            !comp.layers[idx].style.bevel_emboss.enabled;
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        // ── Easing Commands ──
        PaletteCommand {
            name: "Animation: Easy Ease",
            category: "Animation",
            shortcut_hint: "F9",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let layer = &mut comp.layers[idx];
                        let pts = crate::core::keyframe::EasePreset::Standard.control_points();
                        if let crate::core::property::Animatable::Animated(ref mut kfs) =
                            layer.transform.position
                        {
                            for kf in kfs.iter_mut() {
                                kf.interpolation =
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        incoming: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        custom_bezier: Some(pts),
                                    };
                            }
                        }
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Animation: Apply Bounce Easing",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let layer = &mut comp.layers[idx];
                        let pts = crate::core::keyframe::EasePreset::Bounce.control_points();
                        if let crate::core::property::Animatable::Animated(ref mut kfs) =
                            layer.transform.position
                        {
                            for kf in kfs.iter_mut() {
                                kf.interpolation =
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        incoming: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        custom_bezier: Some(pts),
                                    };
                            }
                        }
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Animation: Apply Elastic Easing",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let layer = &mut comp.layers[idx];
                        let pts = crate::core::keyframe::EasePreset::Elastic.control_points();
                        if let crate::core::property::Animatable::Animated(ref mut kfs) =
                            layer.transform.position
                        {
                            for kf in kfs.iter_mut() {
                                kf.interpolation =
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        incoming: crate::core::keyframe::BezierControlPoint {
                                            influence: 0.333,
                                            speed: 0.0,
                                        },
                                        custom_bezier: Some(pts),
                                    };
                            }
                        }
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        // ── Additional Layer Creation Commands ──
        PaletteCommand {
            name: "Add Layer: New Adjustment Layer",
            category: "Layer",
            shortcut_hint: "Cmd+Alt+Y",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let len = comp.layers.len();
                let dur = comp.duration_frames;
                let mut layer = crate::core::timeline::Layer::new(
                    format!("adj_{}", len),
                    format!("Adjustment Layer {}", len + 1),
                    crate::core::timeline::LayerType::Solid {
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    dur,
                );
                layer.is_adjustment_layer = true;
                comp.add_layer(layer);
                app.selection.selected_layer_idx = Some(len);
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Add Layer: New Null Object",
            category: "Layer",
            shortcut_hint: "Cmd+Alt+Shift+Y",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let len = comp.layers.len();
                let dur = comp.duration_frames;
                let layer = crate::core::timeline::Layer::new(
                    format!("null_{}", len),
                    format!("Null {}", len + 1),
                    crate::core::timeline::LayerType::Null,
                    dur,
                );
                comp.add_layer(layer);
                app.selection.selected_layer_idx = Some(len);
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Add Effect: Lightning Arc",
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
                                id: format!("lightning_{}", len),
                                name: "Lightning".to_string(),
                                effect_type: crate::core::timeline::EffectType::LightningArc {
                                    start_x: crate::core::property::Animatable::new_constant(0.2),
                                    start_y: crate::core::property::Animatable::new_constant(0.2),
                                    end_x: crate::core::property::Animatable::new_constant(0.8),
                                    end_y: crate::core::property::Animatable::new_constant(0.8),
                                    seed: crate::core::property::Animatable::new_constant(12345.0),
                                    glow: crate::core::property::Animatable::new_constant(1.0),
                                },
                                enabled: true,
                            });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Effect: Laser Beam",
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
                                id: format!("laser_{}", len),
                                name: "Laser Beam".to_string(),
                                effect_type: crate::core::timeline::EffectType::LaserBeam {
                                    start_x: crate::core::property::Animatable::new_constant(0.1),
                                    start_y: crate::core::property::Animatable::new_constant(0.5),
                                    end_x: crate::core::property::Animatable::new_constant(0.9),
                                    end_y: crate::core::property::Animatable::new_constant(0.5),
                                    progress: crate::core::property::Animatable::new_constant(0.5),
                                    length: crate::core::property::Animatable::new_constant(40.0),
                                    starting_thickness:
                                        crate::core::property::Animatable::new_constant(12.0),
                                    ending_thickness:
                                        crate::core::property::Animatable::new_constant(4.0),
                                    core_color: crate::core::property::Animatable::new_constant([
                                        1.0, 1.0, 1.0, 1.0,
                                    ]),
                                    glow_color: crate::core::property::Animatable::new_constant([
                                        1.0, 0.2, 0.1, 0.8,
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
            name: "Keyframe Assistant: Convert Multi-Band Audio",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                let mut audio_source: Option<String> = None;
                let comp = app.history.current().active_composition();
                if let Some(idx) = app.selection.selected_layer_idx {
                    if let Some(l) = comp.layers.get(idx) {
                        if let crate::core::timeline::LayerType::Audio { path, .. } = &l.layer_type
                        {
                            audio_source = Some(path.clone());
                        } else if let crate::core::timeline::LayerType::Video {
                            audio_wav: Some(w),
                            ..
                        } = &l.layer_type
                        {
                            audio_source = Some(w.clone());
                        }
                    }
                }
                if let Some(src) = audio_source {
                    let mut temp_proj = app.history.current().clone();
                    match crate::core::audio_to_keyframes::convert_multiband_audio_to_keyframes(
                        temp_proj.active_composition_mut(),
                        &src,
                        None,
                    ) {
                        Ok(name) => {
                            app.commit_project(temp_proj);
                            app.toasts
                                .info(format!("Created '{}' with Master/Bass/Mid/Treble", name));
                        }
                        Err(e) => app.toasts.error(e),
                    }
                } else {
                    app.toasts.error("Select an Audio or Video layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Tracker: Apply Motion to New Null",
            category: "Tracking",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selection.selected_layer_idx {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        let null_idx = comp.layers.len();
                        let dur = comp.duration_frames;
                        let null_layer = crate::core::timeline::Layer::new(
                            format!("null_track_{}", null_idx + 1),
                            format!("Tracker {} Null", idx + 1),
                            crate::core::timeline::LayerType::Null,
                            dur,
                        );
                        comp.add_layer(null_layer);
                        crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                            comp, idx, 0, null_idx, true, false,
                        );
                    });
                    app.toasts
                        .info("Created new Null layer with tracked motion!");
                }
            }),
        },
        PaletteCommand {
            name: "View: Switch to Audio Mixer Panel",
            category: "View",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.ui_tabs.right_panel = crate::app_state::RightPanel::Audio;
            }),
        },
        PaletteCommand {
            name: "View: Switch to Motion Tracker Panel",
            category: "View",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.ui_tabs.right_panel = crate::app_state::RightPanel::Tracker;
            }),
        },
        PaletteCommand {
            name: "View: Switch to Properties Panel",
            category: "View",
            shortcut_hint: "F3",
            action: Box::new(|app| {
                app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            }),
        },
    ]
}
