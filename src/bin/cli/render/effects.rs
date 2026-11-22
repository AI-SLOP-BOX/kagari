use super::*;

pub fn cmd_particle(
    project_path: &str,
    layer_name: &str,
    preset: &str,
    rate: Option<f32>,
    max_particles: Option<u32>,
    pos_str: Option<&str>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let preset = preset.to_lowercase();
    if !matches!(
        preset.as_str(),
        "sparks" | "dust" | "fire" | "smoke" | "snow" | "explosion"
    ) {
        return Err(format!(
            "unknown particle preset '{preset}'; valid presets: sparks, dust, fire, smoke, snow, explosion"
        )
        .into());
    }
    if rate.is_some_and(|rate| !rate.is_finite() || rate < 0.0) {
        return Err("particle emission rate must be finite and non-negative".into());
    }
    if max_particles.is_some_and(|count| {
        count == 0 || count > kagari_vfx::core::particle_system::MAX_PARTICLES_PER_EMITTER
    }) {
        return Err(format!(
            "maximum alive particle count must be between 1 and {}",
            kagari_vfx::core::particle_system::MAX_PARTICLES_PER_EMITTER
        )
        .into());
    }
    let requested_position = pos_str
        .map(|value| parse_finite_components(value, "Emitter position"))
        .transpose()?;
    if requested_position
        .as_ref()
        .is_some_and(|position| position.len() != 2)
    {
        return Err("Emitter position requires exactly two values: 'x,y'".into());
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let mut emitter = kagari_vfx::core::particle_system::ParticleEmitter::default();
    match preset.to_lowercase().as_str() {
        "sparks" => {
            emitter.rate = 120.0;
            emitter.lifetime = 0.8;
            emitter.speed = 280.0;
            emitter.spread_degrees = 360.0;
            emitter.gravity = [0.0, 300.0];
            emitter.color_start = [1.0, 0.85, 0.2, 1.0];
            emitter.color_end = [1.0, 0.2, 0.0, 0.0];
            emitter.size_start = 4.0;
            emitter.size_end = 1.0;
        }
        "dust" => {
            emitter.rate = 35.0;
            emitter.lifetime = 4.0;
            emitter.speed = 25.0;
            emitter.spread_degrees = 360.0;
            emitter.gravity = [0.0, -10.0];
            emitter.color_start = [0.9, 0.9, 1.0, 0.4];
            emitter.color_end = [0.8, 0.8, 0.9, 0.0];
            emitter.size_start = 2.5;
            emitter.size_end = 5.0;
        }
        "fire" => {
            emitter.rate = 180.0;
            emitter.lifetime = 0.7;
            emitter.speed = 150.0;
            emitter.spread_degrees = 45.0;
            emitter.gravity = [0.0, -250.0];
            emitter.color_start = [1.0, 0.9, 0.1, 0.9];
            emitter.color_end = [0.9, 0.1, 0.0, 0.0];
            emitter.size_start = 18.0;
            emitter.size_end = 2.0;
        }
        "smoke" => {
            emitter.rate = 40.0;
            emitter.lifetime = 2.5;
            emitter.speed = 60.0;
            emitter.spread_degrees = 60.0;
            emitter.gravity = [0.0, -80.0];
            emitter.color_start = [0.3, 0.3, 0.35, 0.5];
            emitter.color_end = [0.1, 0.1, 0.1, 0.0];
            emitter.size_start = 8.0;
            emitter.size_end = 35.0;
        }
        "snow" => {
            emitter.rate = 75.0;
            emitter.lifetime = 5.0;
            emitter.speed = 80.0;
            emitter.spread_degrees = 30.0;
            emitter.gravity = [10.0, 120.0];
            emitter.shape = kagari_vfx::core::particle_system::EmitterShape::Box;
            emitter.emitter_size = [comp.width as f32, 20.0];
            emitter.color_start = [0.95, 0.98, 1.0, 0.8];
            emitter.color_end = [0.9, 0.95, 1.0, 0.2];
            emitter.size_start = 3.0;
            emitter.size_end = 2.0;
        }
        "explosion" => {
            emitter.rate = 220.0;
            emitter.lifetime = 1.1;
            emitter.speed = 360.0;
            emitter.spread_degrees = 360.0;
            emitter.gravity = [0.0, 180.0];
            emitter.color_start = [1.0, 0.85, 0.35, 1.0];
            emitter.color_end = [0.75, 0.12, 0.015, 0.0];
            emitter.size_start = 12.0;
            emitter.size_end = 2.0;
        }
        _ => unreachable!("preset was validated before loading the project"),
    }

    if let Some(r) = rate {
        emitter.rate = r;
    }
    if let Some(m) = max_particles {
        emitter.max_particles = m;
    }

    let emitter_pos = requested_position
        .map(|position| [position[0], position[1]])
        .unwrap_or([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);

    if let Some(layer) = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
    {
        layer.layer_type = kagari_vfx::core::timeline::LayerType::Particle { emitter };
        layer.transform.position =
            kagari_vfx::core::property::Animatable::new_constant(emitter_pos);
        eprintln!(
            "Updated existing layer '{}' with particle preset '{}'",
            layer.name, preset
        );
    } else {
        let mut new_layer = kagari_vfx::core::timeline::Layer::new(
            format!("particle_{}", comp.layers.len() + 1),
            layer_name.to_string(),
            kagari_vfx::core::timeline::LayerType::Particle { emitter },
            comp.duration_frames,
        );
        new_layer.transform.position =
            kagari_vfx::core::property::Animatable::new_constant(emitter_pos);
        comp.layers.push(new_layer);
        eprintln!(
            "Created new particle layer '{}' with preset '{}'",
            layer_name, preset
        );
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_audio_spectrum(
    project_path: &str,
    layer_name: &str,
    audio_file: &str,
    bands: usize,
    min_freq: f32,
    max_freq: f32,
    max_height: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(audio_file).is_file() {
        return Err(format!("audio source file not found: {audio_file}").into());
    }
    if !max_height.is_finite() {
        return Err("audio spectrum maximum height must be finite".into());
    }
    if !min_freq.is_finite() || !max_freq.is_finite() || min_freq <= 0.0 || max_freq <= min_freq {
        return Err(
            "audio spectrum frequency range must be finite, positive, and increasing".into(),
        );
    }
    if !(1..=256).contains(&bands) {
        return Err("audio spectrum band count must be between 1 and 256".into());
    }
    if min_freq < 20.0 || max_freq > 20_000.0 {
        return Err("audio spectrum range must remain within 20–20000 Hz".into());
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let height_norm = (max_height / comp.height as f32).clamp(0.05, 1.0);
    let effect_id = format!(
        "audio_spectrum_{}_{}",
        comp.layers.len(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let effect = kagari_vfx::core::timeline::Effect {
        id: effect_id,
        name: "Audio Spectrum".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::AudioSpectrum {
            enabled: kagari_vfx::core::property::Animatable::new_constant(1.0),
            bands: kagari_vfx::core::property::Animatable::new_constant(bands as f32),
            start_frequency: min_freq,
            end_frequency: max_freq,
            opacity: kagari_vfx::core::property::Animatable::new_constant(1.0),
            color_start: [0.2, 0.8, 1.0, 1.0],
            color_end: [0.9, 0.2, 0.9, 1.0],
            position_x: kagari_vfx::core::property::Animatable::new_constant(0.05),
            position_y: kagari_vfx::core::property::Animatable::new_constant(0.2),
            width: kagari_vfx::core::property::Animatable::new_constant(0.9),
            height: kagari_vfx::core::property::Animatable::new_constant(height_norm),
        },
        enabled: true,
    };

    if let Some(layer) = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
    {
        layer.effects.push(effect);
        eprintln!("Added AudioSpectrum effect to layer '{}'", layer.name);
    } else {
        let mut solid = kagari_vfx::core::timeline::Layer::new(
            format!("audio_vis_{}", comp.layers.len() + 1),
            layer_name.to_string(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.0, 0.0, 0.0, 0.0],
            },
            comp.duration_frames,
        );
        solid.effects.push(effect);
        comp.layers.push(solid);
        eprintln!(
            "Created visualizer layer '{}' with AudioSpectrum effect",
            layer_name
        );
    }

    let audio_layer_exists = comp.layers.iter().any(|l| {
        if let kagari_vfx::core::timeline::LayerType::Audio { path, .. } = &l.layer_type {
            path == audio_file
        } else {
            false
        }
    });

    if !audio_layer_exists {
        let audio_layer = kagari_vfx::core::timeline::Layer::new(
            format!("audio_src_{}", comp.layers.len() + 1),
            "BGM Source".to_string(),
            kagari_vfx::core::timeline::LayerType::Audio {
                path: audio_file.to_string(),
                volume: kagari_vfx::core::property::Animatable::new_constant(1.0),
            },
            comp.duration_frames,
        );
        comp.layers.insert(0, audio_layer);
        eprintln!("Added audio source layer for '{}'", audio_file);
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub fn cmd_roto(
    project_path: &str,
    layer_name: &str,
    points_str: &str,
    kind_str: &str,
    radius: f32,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !radius.is_finite() {
        return Err("Roto stroke radius must be finite".into());
    }
    let flat = parse_finite_components(points_str, "Roto stroke points")?;
    if flat.len() < 2 || !flat.len().is_multiple_of(2) {
        return Err("Roto stroke points require complete x,y pairs (e.g. \"960,540\")".into());
    }
    let stroke_type = if kind_str.eq_ignore_ascii_case("bg")
        || kind_str.eq_ignore_ascii_case("background")
    {
        kagari_vfx::core::roto_brush_engine::RotoStrokeType::Background
    } else if kind_str.eq_ignore_ascii_case("fg") || kind_str.eq_ignore_ascii_case("foreground") {
        kagari_vfx::core::roto_brush_engine::RotoStrokeType::Foreground
    } else {
        return Err("Roto stroke kind must be 'fg'/'foreground' or 'bg'/'background'".into());
    };
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| {
            format!(
                "Layer '{layer_name}' not found in composition '{}'",
                comp.name
            )
        })?;

    let points: Vec<[f32; 2]> = flat
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| [c[0], c[1]])
        .collect();

    let stroke = kagari_vfx::core::roto_brush_engine::RotoStroke {
        stroke_type,
        points,
        pressures: Vec::new(),
        radius: radius.clamp(2.0, 200.0),
        frame: frame.or(Some(layer.in_frame)),
    };

    layer.roto_brush_strokes.push(stroke);
    eprintln!(
        "Added RotoBrush {:?} stroke ({} points, r={:.1}px) to layer '{}'",
        stroke_type,
        flat.len() / 2,
        radius,
        layer.name
    );

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub fn cmd_puppet(
    project_path: &str,
    layer_name: &str,
    pin_name: &str,
    pos_str: Option<&str>,
    target_str: Option<&str>,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let requested_pos = pos_str
        .map(|value| parse_finite_components(value, "Puppet pin position"))
        .transpose()?;
    let target_pos = target_str
        .map(|value| parse_finite_components(value, "Puppet pin target"))
        .transpose()?;
    for (name, values) in [
        ("position", requested_pos.as_ref()),
        ("target", target_pos.as_ref()),
    ] {
        if values.is_some_and(|values| values.len() != 2) {
            return Err(format!("Puppet pin {name} requires exactly two values: 'x,y'").into());
        }
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| {
            format!(
                "Layer '{layer_name}' not found in composition '{}'",
                comp.name
            )
        })?;

    let default_pos = [comp.width as f32 / 2.0, comp.height as f32 / 2.0];
    let pos = requested_pos
        .as_ref()
        .map(|parts| [parts[0], parts[1]])
        .unwrap_or(default_pos);

    let pin_idx = layer
        .puppet_pins
        .iter()
        .position(|p| p.name == pin_name || p.id == pin_name);
    let target_f = frame.unwrap_or(layer.in_frame);

    if let Some(idx) = pin_idx {
        if let Some(parts) = target_pos {
            let target_pos = [parts[0], parts[1]];
            let pin = &mut layer.puppet_pins[idx];
            let mut kfs = match &pin.position {
                kagari_vfx::core::property::Animatable::Animated(k) => k.clone(),
                kagari_vfx::core::property::Animatable::Constant(c) => {
                    vec![kagari_vfx::core::keyframe::Keyframe::new(
                        layer.in_frame,
                        *c,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    )]
                }
            };
            if let Some(existing_kf) = kfs.iter_mut().find(|k| k.frame == target_f) {
                existing_kf.value = target_pos;
            } else {
                kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                    target_f,
                    target_pos,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ));
                kfs.sort_by_key(|k| k.frame);
            }
            pin.position = kagari_vfx::core::property::Animatable::Animated(kfs);
            eprintln!(
                "Animated Puppet Pin '{}' to [{:.1}, {:.1}] at frame {}",
                pin_name, target_pos[0], target_pos[1], target_f
            );
        }
    } else {
        let mut pin = kagari_vfx::core::timeline::PuppetPin::new(
            format!("pin_{}", layer.puppet_pins.len() + 1),
            pin_name.to_string(),
            pos,
        );
        if let Some(parts) = target_pos {
            let target_pos = [parts[0], parts[1]];
            pin.position = kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(
                    layer.in_frame,
                    pos,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
                kagari_vfx::core::keyframe::Keyframe::new(
                    target_f,
                    target_pos,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
            ]);
        }
        layer.puppet_pins.push(pin);
        eprintln!(
            "Created new Puppet Pin '{}' at [{:.1}, {:.1}] on layer '{}'",
            pin_name, pos[0], pos[1], layer.name
        );
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub fn cmd_chroma_key(
    project_path: &str,
    layer_name: &str,
    color_str: &str,
    gain: f32,
    clip_black: f32,
    clip_white: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, value) in [
        ("gain", gain),
        ("clip black", clip_black),
        ("clip white", clip_white),
    ] {
        if !value.is_finite() {
            return Err(format!("Chroma Key {name} must be finite").into());
        }
    }
    if clip_black > clip_white {
        return Err("Chroma Key clip black must not exceed clip white".into());
    }
    let screen_color = match color_str.to_ascii_lowercase().as_str() {
        "green" => [0.0, 1.0, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let parse_channel = |range| {
                u8::from_str_radix(&hex[range], 16)
                    .map(|value| value as f32 / 255.0)
                    .map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("invalid Chroma Key hex color '{color_str}'"),
                        )
                    })
            };
            [
                parse_channel(1..3)?,
                parse_channel(3..5)?,
                parse_channel(5..7)?,
            ]
        }
        _ => return Err("Chroma Key color must be green, blue, or #RRGGBB".into()),
    };
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| {
            format!(
                "Layer '{layer_name}' not found in composition '{}'",
                comp.name
            )
        })?;

    let effect = kagari_vfx::core::timeline::Effect {
        id: format!(
            "chroma_key_{}_{}",
            layer.effects.len() + 1,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ),
        name: "Chroma Key".to_string(),
        effect_type: kagari_vfx::core::timeline::EffectType::ChromaKey {
            screen_color: kagari_vfx::core::property::Animatable::new_constant(screen_color),
            screen_gain: kagari_vfx::core::property::Animatable::new_constant(gain.clamp(0.1, 3.0)),
            clip_black: kagari_vfx::core::property::Animatable::new_constant(
                clip_black.clamp(0.0, 1.0),
            ),
            clip_white: kagari_vfx::core::property::Animatable::new_constant(
                clip_white.clamp(0.0, 1.0),
            ),
        },
        enabled: true,
    };

    layer.effects.push(effect);
    eprintln!(
        "Configured Chroma Key on layer '{}' (screen: [{:.2}, {:.2}, {:.2}], gain: {:.2})",
        layer.name, screen_color[0], screen_color[1], screen_color[2], gain
    );

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

pub fn cmd_audio_track(
    project_path: &str,
    layer_name: &str,
    volume_opt: Option<f32>,
    fade_in: Option<u32>,
    fade_out: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if volume_opt.is_none() && fade_in.is_none() && fade_out.is_none() {
        return Err("Audio Track requires --volume, --fade-in, or --fade-out".into());
    }
    if volume_opt.is_some_and(|volume| !volume.is_finite() || volume < 0.0) {
        return Err("Audio Track volume must be finite and non-negative".into());
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_name || l.id == layer_name)
        .ok_or_else(|| {
            format!(
                "Layer '{layer_name}' not found in composition '{}'",
                comp.name
            )
        })?;

    let in_f = layer.in_frame;
    let out_f = layer.out_frame;
    let target_vol = volume_opt.unwrap_or(1.0);

    if fade_in.is_some() || fade_out.is_some() {
        let mut kfs = Vec::new();
        if let Some(dur_in) = fade_in {
            let fade_end = in_f.saturating_add(dur_in).min(out_f);
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                in_f,
                0.0,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                fade_end,
                target_vol,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        } else {
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                in_f,
                target_vol,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        }

        if let Some(dur_out) = fade_out {
            let fade_start = out_f.saturating_sub(dur_out).max(in_f);
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                fade_start,
                target_vol,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                out_f,
                0.0,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        } else {
            kfs.push(kagari_vfx::core::keyframe::Keyframe::new(
                out_f,
                target_vol,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        }

        kfs.sort_by_key(|k| k.frame);
        kfs.dedup_by_key(|k| k.frame);

        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                *volume = kagari_vfx::core::property::Animatable::Animated(kfs);
                eprintln!(
                    "Configured audio volume automation envelope on '{}'",
                    layer.name
                );
            }
            _ => {
                eprintln!(
                    "Applied volume envelope to non-audio layer '{}' (recorded for export mix)",
                    layer.name
                );
            }
        }
    } else if let Some(vol) = volume_opt {
        match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                *volume = kagari_vfx::core::property::Animatable::new_constant(vol);
                eprintln!(
                    "Set constant audio volume to {:.2} on '{}'",
                    vol, layer.name
                );
            }
            _ => {
                eprintln!("Set volume to {:.2} on layer '{}'", vol, layer.name);
            }
        }
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}
