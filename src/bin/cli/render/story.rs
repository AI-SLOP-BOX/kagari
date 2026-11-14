use super::*;

#[allow(clippy::too_many_arguments)]
pub fn cmd_story(
    project_path: &str,
    title: &str,
    subtitle: &str,
    audio: Option<&str>,
    footage: Option<&str>,
    width: u32,
    height: u32,
    fps: u32,
    duration_sec: u32,
    theme: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = if std::path::Path::new(project_path).is_file() {
        load_project(project_path).unwrap_or_else(|_| Project::default())
    } else {
        Project::default()
    };

    let total_frames = fps.max(1) * duration_sec.max(1);
    let mut comp = kagari_vfx::core::timeline::Composition::new(
        "main_scene".to_string(),
        title.to_string(),
        width,
        height,
        fps,
        total_frames,
    );

    // 1. Dark Atmosphere Background Solid
    let bg_color = match theme.to_lowercase().as_str() {
        "cyberpunk" => [0.02, 0.01, 0.05, 1.0],
        "anime" => [0.08, 0.06, 0.12, 1.0],
        "corporate" => [0.05, 0.08, 0.12, 1.0],
        "minimal" => [0.92, 0.92, 0.94, 1.0],
        _ => [0.02, 0.02, 0.03, 1.0], // Cinematic dark
    };
    let mut bg_layer = kagari_vfx::core::timeline::Layer::new(
        "bg_solid".to_string(),
        "Background Solid".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid { color: bg_color },
        total_frames,
    );
    bg_layer.locked = true;
    comp.layers.push(bg_layer);

    // 2. Footage Layer (if provided)
    if let Some(foot) = footage {
        let is_video = foot.ends_with(".mp4") || foot.ends_with(".mov") || foot.ends_with(".webm");
        let l_type = if is_video {
            kagari_vfx::core::timeline::LayerType::Video {
                source: foot.to_string(),
                frames_dir: foot.to_string(),
                frame_count: total_frames,
                audio_wav: None,
                audio_volume: kagari_vfx::core::property::Animatable::new_constant(0.0),
                speed: 1.0,
            }
        } else {
            kagari_vfx::core::timeline::LayerType::Image {
                path: foot.to_string(),
            }
        };
        let mut foot_layer = kagari_vfx::core::timeline::Layer::new(
            "footage_main".to_string(),
            "Main Visual Footage".to_string(),
            l_type,
            total_frames,
        );
        foot_layer.transform.scale =
            kagari_vfx::core::property::Animatable::new_constant([1.0, 1.0]);
        // Subtle slow cinematic push-in zoom
        foot_layer.transform.scale = kagari_vfx::core::property::Animatable::Animated(vec![
            kagari_vfx::core::keyframe::Keyframe::new(
                0,
                [1.0, 1.0],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ),
            kagari_vfx::core::keyframe::Keyframe::new(
                total_frames,
                [1.08, 1.08],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ),
        ]);
        comp.layers.push(foot_layer);
    }

    // 3. Audio / BGM Layer (if provided)
    if let Some(aud) = audio {
        let audio_layer = kagari_vfx::core::timeline::Layer::new(
            "bgm_soundtrack".to_string(),
            "Soundtrack Audio".to_string(),
            kagari_vfx::core::timeline::LayerType::Audio {
                path: aud.to_string(),
                volume: kagari_vfx::core::property::Animatable::Animated(vec![
                    kagari_vfx::core::keyframe::Keyframe::new(
                        0,
                        0.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                    kagari_vfx::core::keyframe::Keyframe::new(
                        fps,
                        1.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                    kagari_vfx::core::keyframe::Keyframe::new(
                        total_frames.saturating_sub(fps * 2),
                        1.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                    kagari_vfx::core::keyframe::Keyframe::new(
                        total_frames,
                        0.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                ]),
            },
            total_frames,
        );
        comp.layers.push(audio_layer);

        // Add matching audio spectrum
        let spec_effect = kagari_vfx::core::timeline::Effect {
            id: "director_spectrum".to_string(),
            name: "Audio Visualizer".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
                enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
                bands: kagari_vfx::core::property::Animatable::new_constant(80.0),
                start_frequency: 20.0,
                end_frequency: 20_000.0,
                opacity: kagari_vfx::core::property::Animatable::new_constant(0.65),
                color_start: [0.3, 0.8, 1.0, 0.8],
                color_end: [1.0, 0.3, 0.7, 0.8],
                position_x: kagari_vfx::core::property::Animatable::new_constant(0.08),
                position_y: kagari_vfx::core::property::Animatable::new_constant(0.12),
                width: kagari_vfx::core::property::Animatable::new_constant(0.84),
                height: kagari_vfx::core::property::Animatable::new_constant(0.18),
            },
            enabled: true,
        };
        let mut vis_solid = kagari_vfx::core::timeline::Layer::new(
            "visualizer_solid".to_string(),
            "Rhythm Waveform".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 0.0, 0.0, 0.0],
            },
            total_frames,
        );
        vis_solid.effects.push(spec_effect);
        comp.layers.push(vis_solid);
    }

    // 4. Atmospheric Particle Embers/Dust
    let emitter = kagari_vfx::core::particle_system::ParticleEmitter {
        rate: 40.0,
        lifetime: 3.5,
        speed: 35.0,
        spread_degrees: 360.0,
        gravity: [0.0, -15.0],
        color_start: [1.0, 0.85, 0.4, 0.7],
        color_end: [1.0, 0.3, 0.1, 0.0],
        size_start: 3.5,
        size_end: 1.0,
        ..Default::default()
    };
    let mut particle_layer = kagari_vfx::core::timeline::Layer::new(
        "atmosphere_particles".to_string(),
        "Ambient Particles".to_string(),
        kagari_vfx::core::timeline::LayerType::Particle { emitter },
        total_frames,
    );
    particle_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        width as f32 / 2.0,
        height as f32 * 0.7,
    ]);
    comp.layers.push(particle_layer);

    // 5. Kinetic Title Typography with Text Animator
    let mut title_layer = kagari_vfx::core::timeline::Layer::new(
        "main_title".to_string(),
        "Scene Title".to_string(),
        kagari_vfx::core::timeline::LayerType::Text {
            text: title.to_string(),
            font_size: (height / 14).max(36),
            color: [1.0, 1.0, 1.0, 1.0],
            font_family: "Outfit".to_string(),
            tracking: 12.0,
            leading: 1.2,
            align: 1, // Center
            stroke_color: [0.0, 0.0, 0.0, 0.8],
            stroke_width: 2.0,
            text_on_path: false,
        },
        total_frames,
    );
    title_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        width as f32 / 2.0,
        height as f32 * 0.44,
    ]);
    // Cascade typewriter animator
    let title_sweep_end = (fps * 2).min(total_frames);
    title_layer.text_animator = Some(kagari_vfx::core::text_animator::TextAnimatorSettings {
        enabled: true,
        selector: kagari_vfx::core::text_animator::RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: kagari_vfx::core::text_animator::SelectorShape::RampUp,
            ease_high: 30.0,
            ease_low: 30.0,
            random_order: false,
            char_start: -1,
            char_end: -1,
            offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(
                    0,
                    -100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
                kagari_vfx::core::keyframe::Keyframe::new(
                    title_sweep_end,
                    100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
            ])),
            expression: None,
        },
        position_offset: [0.0, 40.0],
        scale: [0.85, 0.85],
        opacity: 0.0,
        tracking: 10.0,
        rotation: 0.0,
        blur_amount: 15.0,
    });
    comp.layers.push(title_layer);

    // 6. Subtitle / Narrative Logline
    let mut sub_layer = kagari_vfx::core::timeline::Layer::new(
        "subtitle_logline".to_string(),
        "Subtitle Narrative".to_string(),
        kagari_vfx::core::timeline::LayerType::Text {
            text: subtitle.to_string(),
            font_size: (height / 32).max(20),
            color: [0.85, 0.88, 0.95, 0.9],
            font_family: "Inter".to_string(),
            tracking: 8.0,
            leading: 1.2,
            align: 1, // Center
            stroke_color: [0.0, 0.0, 0.0, 0.6],
            stroke_width: 1.0,
            text_on_path: false,
        },
        total_frames,
    );
    sub_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        width as f32 / 2.0,
        height as f32 * 0.54,
    ]);
    sub_layer.transform.opacity = kagari_vfx::core::property::Animatable::Animated(vec![
        kagari_vfx::core::keyframe::Keyframe::new(
            0,
            0.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            fps,
            0.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            fps * 2,
            1.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            total_frames.saturating_sub(fps),
            1.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
        kagari_vfx::core::keyframe::Keyframe::new(
            total_frames,
            0.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ),
    ]);
    comp.layers.push(sub_layer);

    // 7. Master Cinematic Finishing Layer (Adjustment Layer)
    let mut finish_layer = kagari_vfx::core::timeline::Layer::new(
        "cinematic_grade".to_string(),
        "Master Cinematic Grade".to_string(),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        total_frames,
    );
    finish_layer.is_adjustment_layer = true;
    finish_layer
        .effects
        .push(kagari_vfx::core::timeline::Effect {
            id: "film_grain".to_string(),
            name: "35mm Film Grain".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
                intensity: kagari_vfx::core::property::Animatable::new_constant(0.12),
                grain_size: 1.5,
                color_film: true,
            },
            enabled: true,
        });
    finish_layer
        .effects
        .push(kagari_vfx::core::timeline::Effect {
            id: "vignette".to_string(),
            name: "Lens Vignette".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
                intensity: kagari_vfx::core::property::Animatable::new_constant(35.0),
                roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
                feather: kagari_vfx::core::property::Animatable::new_constant(45.0),
                color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
            enabled: true,
        });
    finish_layer
        .effects
        .push(kagari_vfx::core::timeline::Effect {
            id: "chromatic".to_string(),
            name: "Chromatic Aberration".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
                shift_r: kagari_vfx::core::property::Animatable::new_constant(2.0),
                shift_b: kagari_vfx::core::property::Animatable::new_constant(-2.0),
                edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
                iris_linked: true,
            },
            enabled: true,
        });
    comp.layers.push(finish_layer);

    let comp_idx = project.compositions.len();
    project.compositions.push(comp);
    project.active_composition_idx = comp_idx;

    save_cli_project(project_path, &project)?;

    eprintln!(
        "\n🎬 Directed & Composed Scene '{}' successfully!\n  Resolution: {}x{} @ {}fps, {} seconds ({} frames)\n  Theme: {}\n  Layers created: 7 layers (Solid, Footage, Audio, Spectrum, Particles, Title, Grade)",
        title, width, height, fps, duration_sec, total_frames, theme
    );
    Ok(())
}

pub fn cmd_critique(
    project_path: &str,
    comp_ref: Option<&str>,
    sample_step: u32,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = find_comp(&project, comp_ref)?;

    if comp.duration_frames == 0 {
        return Err(format!("Composition '{}' has no frames", comp.name).into());
    }

    let step = sample_step.max(1);
    let sample_frames: Vec<u32> = (0..comp.duration_frames).step_by(step as usize).collect();

    let mut shadow_clip_sum = 0.0f32;
    let mut highlight_clip_sum = 0.0f32;
    let mut contrast_sum = 0.0f32;
    let mut evaluated_frames = 0;

    for &f in &sample_frames {
        let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels(
            comp,
            f,
            comp.width.min(640),
            comp.height.min(360),
            0.0,
            0,
        );
        let n_pixels = pixels.len() / 4;
        if n_pixels == 0 {
            continue;
        }

        let mut lum_min = 255u8;
        let mut lum_max = 0u8;
        let mut shadows = 0;
        let mut highlights = 0;

        for chunk in pixels.as_chunks::<4>().0 {
            let lum =
                ((chunk[0] as u32 * 77 + chunk[1] as u32 * 150 + chunk[2] as u32 * 29) >> 8) as u8;
            if lum < lum_min {
                lum_min = lum;
            }
            if lum > lum_max {
                lum_max = lum;
            }
            if lum < 5 {
                shadows += 1;
            }
            if lum > 250 {
                highlights += 1;
            }
        }

        let shadow_ratio = (shadows as f32 / n_pixels as f32) * 100.0;
        let highlight_ratio = (highlights as f32 / n_pixels as f32) * 100.0;
        let dynamic_range = (lum_max as f32 - lum_min as f32) / 255.0;

        shadow_clip_sum += shadow_ratio;
        highlight_clip_sum += highlight_ratio;
        contrast_sum += dynamic_range;
        evaluated_frames += 1;
    }

    let avg_shadow_clip = if evaluated_frames > 0 {
        shadow_clip_sum / evaluated_frames as f32
    } else {
        0.0
    };
    let avg_highlight_clip = if evaluated_frames > 0 {
        highlight_clip_sum / evaluated_frames as f32
    } else {
        0.0
    };
    let avg_dynamic_range = if evaluated_frames > 0 {
        contrast_sum / evaluated_frames as f32
    } else {
        0.0
    };

    // Audio & sync analysis
    let has_audio = comp.layers.iter().any(|l| {
        matches!(
            l.layer_type,
            kagari_vfx::core::timeline::LayerType::Audio { .. }
        )
    });
    let has_particles = comp.layers.iter().any(|l| {
        matches!(
            l.layer_type,
            kagari_vfx::core::timeline::LayerType::Particle { .. }
        )
    });
    let has_camera = comp.cameras.iter().any(|c| c.active) || comp.active_camera.active;
    let has_lut_or_grade = comp.layers.iter().any(|l| {
        l.is_adjustment_layer
            || l.effects
                .iter()
                .any(|e| e.name.contains("Film") || e.name.contains("Grade"))
    });

    // Scoring out of 100
    let visual_contrast_score = (avg_dynamic_range * 100.0f32).clamp(0.0f32, 100.0f32);
    let exposure_balance_score =
        (100.0f32 - (avg_shadow_clip + avg_highlight_clip) * 2.0f32).clamp(0.0f32, 100.0f32);
    let depth_add: f32 = (if has_camera { 25.0f32 } else { 0.0f32 })
        + (if has_particles { 25.0f32 } else { 0.0f32 })
        + (if has_lut_or_grade { 20.0f32 } else { 0.0f32 });
    let composition_depth_score = (30.0f32 + depth_add).clamp(0.0f32, 100.0f32);
    let audio_sync_score = if has_audio { 95.0f32 } else { 40.0f32 };
    let overall_score = (visual_contrast_score * 0.25
        + exposure_balance_score * 0.25
        + composition_depth_score * 0.3
        + audio_sync_score * 0.2)
        .round();

    let mut suggestions = Vec::new();
    if avg_shadow_clip > 15.0 {
        suggestions.push("High shadow clipping detected. Consider adding a Lift (+0.03) or lowering contrast to prevent crushed blacks.");
    }
    if avg_highlight_clip > 10.0 {
        suggestions.push("Highlight clipping detected. Add a soft knee or lower exposure (-0.3 EV) for filmic roll-off.");
    }
    if !has_camera {
        suggestions.push("No 3D camera motion found. Run 'kagari camera --pos ... --dof' to add cinematic 2.5D parallax depth.");
    }
    if !has_particles {
        suggestions.push("Scene lacks atmospheric texture. Run 'kagari particle --preset dust' to add organic ambient dust/embers.");
    }
    if !has_lut_or_grade {
        suggestions.push("No color grading applied. Run 'kagari cinematic --preset 35mm' to infuse physical film grain and lens character.");
    }

    if json {
        let report = serde_json::json!({
            "composition": comp.name,
            "overall_score": overall_score,
            "metrics": {
                "visual_contrast": visual_contrast_score,
                "exposure_balance": exposure_balance_score,
                "depth_and_camera": composition_depth_score,
                "audio_integration": audio_sync_score,
            },
            "stats": {
                "avg_shadow_clipping_percent": avg_shadow_clip,
                "avg_highlight_clipping_percent": avg_highlight_clip,
                "evaluated_sample_frames": evaluated_frames,
            },
            "actionable_tuning_suggestions": suggestions,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        eprintln!("\n🔍 ═══ Autonomous Vision & Rhythm Critique Report ═══");
        eprintln!("  Composition: {}", comp.name);
        eprintln!("  Overall Production Score: {} / 100", overall_score);
        eprintln!("  ├── Visual Contrast:      {:.1}%", visual_contrast_score);
        eprintln!("  ├── Exposure Balance:     {:.1}%", exposure_balance_score);
        eprintln!(
            "  ├── Depth & Cinematography: {:.1}%",
            composition_depth_score
        );
        eprintln!("  └── Audio Synchronization: {:.1}%", audio_sync_score);
        eprintln!("\n💡 Actionable Director Suggestions for Claude/Opus 5.5 Beatdown:");
        for (i, s) in suggestions.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, s);
        }
        eprintln!("═════════════════════════════════════════════════════════\n");
    }

    Ok(())
}

pub fn cmd_cinematic(
    project_path: &str,
    preset: &str,
    grain: f32,
    letterbox: bool,
    aberration: f32,
    vignette: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let mut adj = kagari_vfx::core::timeline::Layer::new(
        format!("cinematic_finishing_{}", comp.layers.len() + 1),
        format!("Cinematic Finishing ({})", preset),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        comp.duration_frames,
    );
    adj.is_adjustment_layer = true;

    // 1. Film Grain
    if grain > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("grain_{}", comp.layers.len()),
            name: "35mm Grain".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
                intensity: kagari_vfx::core::property::Animatable::new_constant(
                    grain.clamp(0.01, 1.0),
                ),
                grain_size: 1.6,
                color_film: true,
            },
            enabled: true,
        });
    }

    // 2. Lens Chromatic Aberration
    if aberration > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("chromatic_{}", comp.layers.len()),
            name: "Lens Aberration".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
                shift_r: kagari_vfx::core::property::Animatable::new_constant(
                    aberration.clamp(0.5, 20.0),
                ),
                shift_b: kagari_vfx::core::property::Animatable::new_constant(
                    -aberration.clamp(0.5, 20.0),
                ),
                edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
                iris_linked: true,
            },
            enabled: true,
        });
    }

    // 3. Vignette
    if vignette > 0.0 {
        adj.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("vignette_{}", comp.layers.len()),
            name: "Lens Falloff".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
                intensity: kagari_vfx::core::property::Animatable::new_constant(
                    (vignette * 100.0).clamp(5.0, 95.0),
                ),
                roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
                feather: kagari_vfx::core::property::Animatable::new_constant(50.0),
                color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
            enabled: true,
        });
    }

    // 4. Anamorphic 2.39:1 Letterbox Bars
    if letterbox {
        let bar_fraction = 0.125f32; // Standard 16:9 to 2.39:1 letterbox
        let bar_h = (comp.height as f32 * bar_fraction).round() as u32;
        let mut top_bar = kagari_vfx::core::timeline::Layer::new(
            "letterbox_top".to_string(),
            "Cinema Letterbox Top".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 0.0, 0.0, 1.0],
            },
            comp.duration_frames,
        );
        top_bar.transform.anchor_point =
            kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, 0.0]);
        top_bar.transform.position =
            kagari_vfx::core::property::Animatable::new_constant([comp.width as f32 / 2.0, 0.0]);
        top_bar.transform.scale = kagari_vfx::core::property::Animatable::new_constant([
            1.0,
            bar_h as f32 / comp.height as f32,
        ]);
        top_bar.locked = true;

        let mut btm_bar = kagari_vfx::core::timeline::Layer::new(
            "letterbox_btm".to_string(),
            "Cinema Letterbox Bottom".to_string(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 0.0, 0.0, 1.0],
            },
            comp.duration_frames,
        );
        btm_bar.transform.anchor_point = kagari_vfx::core::property::Animatable::new_constant([
            comp.width as f32 / 2.0,
            comp.height as f32,
        ]);
        btm_bar.transform.position = kagari_vfx::core::property::Animatable::new_constant([
            comp.width as f32 / 2.0,
            comp.height as f32,
        ]);
        btm_bar.transform.scale = kagari_vfx::core::property::Animatable::new_constant([
            1.0,
            bar_h as f32 / comp.height as f32,
        ]);
        btm_bar.locked = true;

        comp.layers.push(top_bar);
        comp.layers.push(btm_bar);
    }

    comp.layers.push(adj);
    eprintln!(
        "Injected Cinematic Finishing Preset '{}' (grain: {:.2}, letterbox: {}, aberration: {:.1}px, vignette: {:.2})",
        preset, grain, letterbox, aberration, vignette
    );

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}
