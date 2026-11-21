use super::*;

/// Parse SRT or LRC timed lyrics into (start_frame, end_frame, text) segments.
fn parse_timed_lyrics(content: &str, fps: f32, max_frames: u32) -> Vec<(u32, u32, String)> {
    let mut cues = Vec::new();

    // Check if LRC format: [mm:ss.xx]Text
    let mut lrc_lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(bracket_end) = trimmed.find(']') {
                let tag = &trimmed[1..bracket_end];
                let text = trimmed[bracket_end + 1..].trim();
                if let Some((min_str, sec_str)) = tag.split_once(':') {
                    if let (Ok(min), Ok(sec)) = (min_str.parse::<f32>(), sec_str.parse::<f32>()) {
                        let time_sec = min * 60.0 + sec;
                        let frame = (time_sec * fps).round() as u32;
                        if !text.is_empty() {
                            lrc_lines.push((frame, text.to_string()));
                        }
                    }
                }
            }
        }
    }

    if !lrc_lines.is_empty() {
        lrc_lines.sort_by_key(|&(f, _)| f);
        for i in 0..lrc_lines.len() {
            let start = lrc_lines[i].0;
            let end = if i + 1 < lrc_lines.len() {
                lrc_lines[i + 1].0.saturating_sub(1).max(start + 1)
            } else {
                (start + (fps * 4.0) as u32).min(max_frames)
            };
            cues.push((
                start.min(max_frames),
                end.min(max_frames),
                lrc_lines[i].1.clone(),
            ));
        }
        return cues;
    }

    // Check if SRT format:
    // 1
    // 00:00:01,000 --> 00:00:04,000
    // Lyric text
    let parse_srt_time = |t_str: &str| -> Option<f32> {
        let parts: Vec<&str> = t_str.trim().split(':').collect();
        if parts.len() == 3 {
            let hr: f32 = parts[0].parse().ok()?;
            let min: f32 = parts[1].parse().ok()?;
            let sec_comma = parts[2].replace(',', ".");
            let sec: f32 = sec_comma.parse().ok()?;
            Some(hr * 3600.0 + min * 60.0 + sec)
        } else {
            None
        }
    };

    let mut current_start: Option<f32> = None;
    let mut current_end: Option<f32> = None;
    let mut current_text = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let (Some(s), Some(e)) = (current_start, current_end) {
                let text = current_text.join(" ");
                if !text.is_empty() {
                    cues.push(((s * fps).round() as u32, (e * fps).round() as u32, text));
                }
            }
            current_start = None;
            current_end = None;
            current_text.clear();
        } else if trimmed.contains("-->") {
            let mut parts = trimmed.split("-->");
            if let (Some(s_str), Some(e_str)) = (parts.next(), parts.next()) {
                current_start = parse_srt_time(s_str);
                current_end = parse_srt_time(e_str);
            }
        } else if trimmed.parse::<u32>().is_ok() && current_start.is_none() {
            // Index number, ignore
        } else if current_start.is_some() {
            current_text.push(trimmed.to_string());
        }
    }
    if let (Some(s), Some(e)) = (current_start, current_end) {
        let text = current_text.join(" ");
        if !text.is_empty() {
            cues.push(((s * fps).round() as u32, (e * fps).round() as u32, text));
        }
    }

    cues
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_mv(
    audio_path: &str,
    lyrics_opt: Option<&str>,
    lyrics_file_opt: Option<&str>,
    output_path: &str,
    style: &str,
    width: u32,
    height: u32,
    fps: u32,
    duration_sec: Option<u32>,
    project_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎵 ═══ Kagari Autonomous Music Video Generator ═══");
    eprintln!("  Audio: {}", audio_path);
    eprintln!("  Style: {}", style);
    eprintln!("  Format: {}x{} @ {}fps", width, height, fps);

    // 1. Analyze Audio Beats & Transients
    let max_frames = duration_sec.unwrap_or(30) * fps.max(1);
    let detected_beats = kagari_vfx::core::audio_engine::detect_beat_frames(
        std::path::Path::new(audio_path),
        max_frames,
        fps as f32,
    );

    eprintln!(
        "  Analyzed Audio: Found {} transient beat drops across {} frames",
        detected_beats.len(),
        max_frames
    );

    // 2. Parse Lyrics / Timed Subtitles
    let mut timed_cues: Vec<(u32, u32, String)> = Vec::new();
    let mut raw_phrases: Vec<String> = Vec::new();

    if let Some(f_path) = lyrics_file_opt {
        if let Ok(content) = std::fs::read_to_string(f_path) {
            let is_timed = f_path.ends_with(".srt")
                || f_path.ends_with(".lrc")
                || content.contains("-->")
                || content.contains("[0");
            if is_timed {
                timed_cues = parse_timed_lyrics(&content, fps as f32, max_frames);
                eprintln!(
                    "  Parsed Timed Lyrics (SRT/LRC): {} synchronized cue points",
                    timed_cues.len()
                );
            }
            if timed_cues.is_empty() {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        raw_phrases.push(trimmed.to_string());
                    }
                }
            }
        }
    }
    if timed_cues.is_empty() && raw_phrases.is_empty() {
        if let Some(l_str) = lyrics_opt {
            for chunk in l_str.split(&['/', ';', '\n'][..]) {
                let trimmed = chunk.trim();
                if !trimmed.is_empty() {
                    raw_phrases.push(trimmed.to_string());
                }
            }
        }
    }
    if timed_cues.is_empty() && raw_phrases.is_empty() {
        raw_phrases = vec![
            "YOU ARE ABSOLUTELY RIGHT".to_string(),
            "HALLUCINATING THROUGH THE NOISE".to_string(),
            "LOOKS GOOD TO ME (LGTM)".to_string(),
            "HUMAN IN THE LOOP".to_string(),
            "I'LL TAKE IT FROM HERE".to_string(),
            "SUPERINTELLIGENCE ACTIVE".to_string(),
        ];
    }

    let total_frames = max_frames;
    let mut comp = kagari_vfx::core::timeline::Composition::new(
        "mv_composition".to_string(),
        "Autonomous Music Video".to_string(),
        width,
        height,
        fps,
        total_frames,
    );

    // 3. Stylistic Palette
    let (bg_color, neon_pri, neon_sec, _font_name) = match style.to_lowercase().as_str() {
        "analog-grunge" => (
            [0.05, 0.04, 0.04, 1.0],
            [0.95, 0.9, 0.8, 1.0],
            [0.8, 0.2, 0.2, 1.0],
            "Outfit",
        ),
        "kinetic-swiss" => (
            [0.02, 0.02, 0.02, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 0.1, 0.1, 1.0],
            "Inter",
        ),
        "deep-space" => (
            [0.01, 0.01, 0.04, 1.0],
            [0.3, 0.8, 1.0, 1.0],
            [0.7, 0.3, 1.0, 1.0],
            "Outfit",
        ),
        "hyper-pop" => (
            [0.08, 0.02, 0.08, 1.0],
            [1.0, 0.2, 0.8, 1.0],
            [0.2, 1.0, 0.9, 1.0],
            "Outfit",
        ),
        _ => (
            [0.02, 0.01, 0.03, 1.0],
            [0.0, 0.95, 1.0, 1.0],
            [1.0, 0.05, 0.6, 1.0],
            "Outfit",
        ), // Cyberpunk Neon
    };

    // Layer 1: Dark Backdrop with Beat-Synced Camera Shake and Bass Kick
    let mut bg_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_bg".to_string(),
        "Deep Background".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid { color: bg_color },
        total_frames,
    );
    bg_layer.locked = true;

    // Apply Beat-Synced Bass Kick Pulse & Camera Shake to Background
    if !detected_beats.is_empty() {
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let mut pos_keys = Vec::new();
        let mut scale_keys = Vec::new();

        // Baseline key at frame 0
        pos_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            0,
            [center_x, center_y],
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        scale_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            0,
            [1.0, 1.0],
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));

        for &beat_f in &detected_beats {
            if beat_f >= total_frames {
                continue;
            }
            let time_sec = beat_f as f32 / fps as f32;
            let shake = kagari_vfx::core::ae_effects_pack::camera_shake(time_sec, 18.0, 12.0, 42);

            // Impact pop
            pos_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
                beat_f,
                [center_x + shake[0], center_y + shake[1]],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
            scale_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
                beat_f,
                [1.04, 1.04],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));

            // Settle after 3-5 frames
            let settle_f = (beat_f + 4).min(total_frames.saturating_sub(1));
            pos_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
                settle_f,
                [center_x, center_y],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
            scale_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
                settle_f,
                [1.0, 1.0],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        }

        pos_keys.sort_by_key(|k| k.frame);
        pos_keys.dedup_by_key(|k| k.frame);
        scale_keys.sort_by_key(|k| k.frame);
        scale_keys.dedup_by_key(|k| k.frame);

        bg_layer.transform.position = kagari_vfx::core::property::Animatable::Animated(pos_keys);
        bg_layer.transform.scale = kagari_vfx::core::property::Animatable::Animated(scale_keys);
    }
    comp.layers.push(bg_layer);

    // Layer 2: Audio Track Layer
    let audio_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_soundtrack".to_string(),
        "Master Audio Track".to_string(),
        kagari_vfx::core::timeline::LayerType::Audio {
            path: audio_path.to_string(),
            volume: kagari_vfx::core::property::Animatable::new_constant(1.0),
        },
        total_frames,
    );
    comp.layers.push(audio_layer);

    // Layer 3: Audio Spectrum Waveform Visualizer
    let spec_effect = kagari_vfx::core::timeline::Effect {
        id: "mv_spectrum_fx".to_string(),
        name: "Audio Visualizer".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
            enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
            bands: kagari_vfx::core::property::Animatable::new_constant(96.0),
            start_frequency: 20.0,
            end_frequency: 20_000.0,
            opacity: kagari_vfx::core::property::Animatable::new_constant(0.7),
            color_start: neon_pri,
            color_end: neon_sec,
            position_x: kagari_vfx::core::property::Animatable::new_constant(0.05),
            position_y: kagari_vfx::core::property::Animatable::new_constant(0.08),
            width: kagari_vfx::core::property::Animatable::new_constant(0.9),
            height: kagari_vfx::core::property::Animatable::new_constant(0.25),
        },
        enabled: true,
    };
    let mut vis_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_visualizer".to_string(),
        "Beat Reactive Spectrum".to_string(),
        kagari_vfx::core::timeline::LayerType::Solid {
            color: [0.0, 0.0, 0.0, 0.0],
        },
        total_frames,
    );
    vis_layer.effects.push(spec_effect);
    comp.layers.push(vis_layer);

    // Layer 4: Particle Embers / Stardust
    let emitter = kagari_vfx::core::particle_system::ParticleEmitter {
        rate: 80.0,
        lifetime: 2.0,
        speed: 120.0,
        spread_degrees: 360.0,
        gravity: [0.0, -40.0],
        color_start: neon_pri,
        color_end: [neon_sec[0], neon_sec[1], neon_sec[2], 0.0],
        size_start: 4.0,
        size_end: 1.0,
        ..Default::default()
    };
    let mut particle_layer = kagari_vfx::core::timeline::Layer::new(
        "mv_particles".to_string(),
        "Audio Atmosphere Sparks".to_string(),
        kagari_vfx::core::timeline::LayerType::Particle { emitter },
        total_frames,
    );
    particle_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        width as f32 / 2.0,
        height as f32 / 2.0,
    ]);
    comp.layers.push(particle_layer);

    // Layer 5+: Kinetic Beat-Synchronized Typography Layers
    let phrases_data: Vec<(u32, u32, String)> = if !timed_cues.is_empty() {
        timed_cues
    } else {
        let phrase_count = raw_phrases.len().max(1);
        let frames_per_phrase = (total_frames / phrase_count as u32).max(fps);
        raw_phrases
            .into_iter()
            .enumerate()
            .map(|(i, phrase)| {
                let p_start = (i as u32 * frames_per_phrase).min(total_frames.saturating_sub(1));
                let p_end = (p_start + frames_per_phrase).min(total_frames);
                (p_start, p_end, phrase)
            })
            .collect()
    };

    let mut phrase_starts = Vec::new();

    for (i, (p_start, p_end, phrase)) in phrases_data.iter().enumerate() {
        let p_start = *p_start;
        let p_end = *p_end;
        let p_dur = p_end.saturating_sub(p_start).max(1);
        phrase_starts.push(p_start);

        let mut text_layer = kagari_vfx::core::timeline::Layer::new(
            format!("kinetic_text_{}", i + 1),
            format!("Lyric #{}: {}", i + 1, phrase),
            kagari_vfx::core::timeline::LayerType::Text {
                text: phrase.clone(),
                font_size: (height / 12).clamp(42, 110),
                color: neon_pri,
                font_family: "Outfit".to_string(),
                tracking: 10.0,
                leading: 1.1,
                align: 1, // Center
                stroke_color: [0.0, 0.0, 0.0, 0.9],
                stroke_width: 3.0,
                text_on_path: false,
            },
            total_frames,
        );
        text_layer.in_frame = p_start;
        text_layer.out_frame = p_end;

        // Position: centered with gentle float
        text_layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
            width as f32 / 2.0,
            height as f32 * 0.48,
        ]);

        // Scale Kick on Beat: quick pop zoom
        let beat_pop_end = p_start + (fps / 4).min(p_dur);
        text_layer.transform.scale = kagari_vfx::core::property::Animatable::Animated(vec![
            kagari_vfx::core::keyframe::Keyframe::new(
                p_start,
                [1.4, 1.4],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ),
            kagari_vfx::core::keyframe::Keyframe::new(
                beat_pop_end,
                [1.0, 1.0],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ),
            kagari_vfx::core::keyframe::Keyframe::new(
                p_end,
                [1.05, 1.05],
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ),
        ]);

        // Per-character Text Animator (Typewriter & Rise)
        let sweep_frames = (fps / 2).min(p_dur);
        text_layer.text_animator = Some(kagari_vfx::core::text_animator::TextAnimatorSettings {
            enabled: true,
            selector: kagari_vfx::core::text_animator::RangeSelector {
                start: 0.0,
                end: 100.0,
                offset: 0.0,
                shape: kagari_vfx::core::text_animator::SelectorShape::RampUp,
                ease_high: 25.0,
                ease_low: 25.0,
                random_order: false,
                char_start: -1,
                char_end: -1,
                offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                    kagari_vfx::core::keyframe::Keyframe::new(
                        p_start,
                        -100.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                    kagari_vfx::core::keyframe::Keyframe::new(
                        p_start + sweep_frames,
                        100.0,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                ])),
                expression: None,
            },
            position_offset: [0.0, 60.0],
            scale: [0.8, 0.8],
            opacity: 0.0,
            tracking: 12.0,
            rotation: 0.0,
            blur_amount: 18.0,
        });

        // Glow Effect on Lyrics
        text_layer.effects.push(kagari_vfx::core::timeline::Effect {
            id: format!("lyric_glow_{}", i),
            name: "Cyber Glow".to_string(),
            effect_type: kagari_vfx::core::timeline::EffectType::Glow {
                intensity: kagari_vfx::core::property::Animatable::new_constant(1.5),
                radius: kagari_vfx::core::property::Animatable::new_constant(22.0),
                threshold: kagari_vfx::core::property::Animatable::new_constant(0.4),
                color: kagari_vfx::core::property::Animatable::new_constant(neon_pri),
            },
            enabled: true,
        });

        comp.layers.push(text_layer);
    }

    // Top Master Layer: 35mm Cinema Finish & Audio Glitch / RGB Split Transitions
    let mut finish_adj = kagari_vfx::core::timeline::Layer::new(
        "mv_finishing".to_string(),
        "35mm Cinema & Lens Finishing".to_string(),
        kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
        total_frames,
    );
    finish_adj.is_adjustment_layer = true;
    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_grain".to_string(),
        name: "Film Grain".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::FilmGrain {
            intensity: kagari_vfx::core::property::Animatable::new_constant(0.14),
            grain_size: 1.5,
            color_film: true,
        },
        enabled: true,
    });

    // Dynamic Chromatic Aberration Spike / Glitch at every phrase boundary
    let mut chroma_r_keys = Vec::new();
    let mut chroma_b_keys = Vec::new();
    chroma_r_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
        0,
        2.5,
        kagari_vfx::core::keyframe::InterpolationType::Linear,
    ));
    chroma_b_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
        0,
        -2.5,
        kagari_vfx::core::keyframe::InterpolationType::Linear,
    ));

    for &start_f in &phrase_starts {
        if start_f == 0 {
            continue;
        }
        let pre_f = start_f.saturating_sub(1);
        let spike_f = start_f;
        let post_f = (start_f + 5).min(total_frames.saturating_sub(1));

        chroma_r_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            pre_f,
            2.5,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        chroma_r_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            spike_f,
            18.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        )); // Glitch burst
        chroma_r_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            post_f,
            2.5,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));

        chroma_b_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            pre_f,
            -2.5,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        chroma_b_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            spike_f,
            -18.0,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        chroma_b_keys.push(kagari_vfx::core::keyframe::Keyframe::new(
            post_f,
            -2.5,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
    }
    chroma_r_keys.sort_by_key(|k| k.frame);
    chroma_r_keys.dedup_by_key(|k| k.frame);
    chroma_b_keys.sort_by_key(|k| k.frame);
    chroma_b_keys.dedup_by_key(|k| k.frame);

    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_chroma".to_string(),
        name: "RGB Fringe & Glitch".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::ChromaticAberration {
            shift_r: kagari_vfx::core::property::Animatable::Animated(chroma_r_keys),
            shift_b: kagari_vfx::core::property::Animatable::Animated(chroma_b_keys),
            edge_falloff: kagari_vfx::core::property::Animatable::new_constant(1.0),
            iris_linked: true,
        },
        enabled: true,
    });
    finish_adj.effects.push(kagari_vfx::core::timeline::Effect {
        id: "mv_vignette".to_string(),
        name: "Lens Falloff".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::Vignette {
            intensity: kagari_vfx::core::property::Animatable::new_constant(40.0),
            roundness: kagari_vfx::core::property::Animatable::new_constant(0.0),
            feather: kagari_vfx::core::property::Animatable::new_constant(55.0),
            color: kagari_vfx::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
        },
        enabled: true,
    });
    comp.layers.push(finish_adj);

    let mut project = Project::default();
    project.compositions.push(comp);
    project.active_composition_idx = 0;

    let project_json_path = if output_path.ends_with(".json") {
        output_path.to_string()
    } else {
        "./mv_project.json".to_string()
    };

    save_cli_project(&project_json_path, &project)?;
    eprintln!("\n✅ Generated Music Video Project: {}", project_json_path);

    if !project_only && output_path.ends_with(".mp4") {
        eprintln!("🚀 Launching High-Definition MP4 Render with 48kHz Master Mix...");
        cmd_render(RenderArgs {
            project_path: project_json_path,
            comp_ref: Some("Autonomous Music Video".to_string()),
            output: output_path.to_string(),
            format: OutputFormat::Mp4,
            frames: None,
            from: 0,
            to: Some(total_frames.saturating_sub(1)),
            width: Some(width),
            height: Some(height),
            exposure: 0.0,
            lut: 0,
            lut_file: None,
            color_space: None,
            step: 1,
            threads: 0, // auto CPU cores
            skip_existing: false,
            mfr: true,
            log_format: "standard".to_string(),
            bindings: Vec::new(),
            overrides: Vec::new(),
            expressions: Vec::new(),
        })?;
        eprintln!(
            "\n🎉 Masterpiece Music Video Exported Successfully to '{}'!",
            output_path
        );
    }

    Ok(())
}
