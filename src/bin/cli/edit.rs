use super::*;
use kagari_vfx::core::timeline::{Composition, Layer, LayerType, Project};

pub(super) fn cmd_init(
    project_path: &str,
    name: &str,
    width: u32,
    height: u32,
    fps: u32,
    duration: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = Project {
        compositions: vec![Composition::new(
            "comp_0".to_string(),
            name.to_string(),
            width,
            height,
            fps,
            duration,
        )],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };
    // Add default background solid
    let mut bg = Layer::new(
        "bg_solid".to_string(),
        "Background".to_string(),
        LayerType::Solid {
            color: [0.05, 0.05, 0.08, 1.0],
        },
        duration,
    );
    bg.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        width as f32 * 0.5,
        height as f32 * 0.5,
    ]);
    bg.transform.scale = kagari_vfx::core::property::Animatable::new_constant([
        width as f32,
        height as f32,
    ]);
    project.compositions[0].add_layer(bg);

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Created new project: {} ({}x{} @ {}fps, {} frames)",
        project_path, width, height, fps, duration
    );
    Ok(())
}

pub(super) fn cmd_add_layer(
    project_path: &str,
    name: &str,
    comp_ref: Option<&str>,
    kind: &str,
    content: Option<&str>,
    in_frame: u32,
    out_frame: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let dur = out_frame.unwrap_or(comp.duration_frames);
    let layer_id = format!("layer_{}", comp.layers.len());

    let layer_type = match kind.to_lowercase().as_str() {
        "text" => {
            let text_val = content.unwrap_or("New Text");
            LayerType::Text {
                text: text_val.to_string(),
                font_size: 64,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Inter".to_string(),
                tracking: 0.0,
                leading: 1.2,
                align: 1, // Center
                stroke_color: [0.0, 0.0, 0.0, 0.0],
                stroke_width: 0.0,
                text_on_path: false,
            }
        }
        "image" => {
            let path_val = content.ok_or("Image layer requires --content <file_path>")?;
            LayerType::Image {
                path: path_val.to_string(),
            }
        }
        "video" => {
            let source_val = content.ok_or("Video layer requires --content <source_file>")?;
            LayerType::Video {
                source: source_val.to_string(),
                frames_dir: String::new(),
                frame_count: dur,
                audio_wav: None,
                speed: 1.0,
            }
        }
        "solid" => {
            let mut col = [0.2, 0.2, 0.2, 1.0];
            if let Some(c_str) = content {
                let parts: Vec<f32> = c_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect();
                if parts.len() >= 3 {
                    col[0] = parts[0];
                    col[1] = parts[1];
                    col[2] = parts[2];
                    if parts.len() >= 4 {
                        col[3] = parts[3];
                    }
                }
            }
            LayerType::Solid { color: col }
        }
        other => return Err(format!("Unsupported layer kind '{}'. Use text, image, solid, or video.", other).into()),
    };

    let mut layer = Layer::new(layer_id, name.to_string(), layer_type, dur);
    layer.in_frame = in_frame;
    layer.out_frame = dur;
    layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([
        comp.width as f32 * 0.5,
        comp.height as f32 * 0.5,
    ]);

    let comp_name = comp.name.clone();
    comp.add_layer(layer);

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Added layer '{}' ({}) to '{}' in {}",
        name, kind, comp_name, project_path
    );
    Ok(())
}

fn find_comp_mut<'a>(
    project: &'a mut Project,
    comp_ref: Option<&str>,
) -> Result<&'a mut Composition, Box<dyn std::error::Error>> {
    if let Some(target) = comp_ref {
        if let Ok(idx) = target.parse::<usize>() {
            return project
                .compositions
                .get_mut(idx)
                .ok_or_else(|| format!("Composition index {idx} not found").into());
        }
        return project
            .compositions
            .iter_mut()
            .find(|c| c.name == target || c.id == target)
            .ok_or_else(|| format!("Composition '{target}' not found").into());
    }
    let idx = if project.active_composition_idx < project.compositions.len() {
        project.active_composition_idx
    } else {
        0
    };
    project
        .compositions
        .get_mut(idx)
        .ok_or_else(|| "Project contains no compositions".into())
}

pub(super) fn cmd_animate(
    project_path: &str,
    layer_ref: &str,
    comp_ref: Option<&str>,
    property: &str,
    keyframes_raw: &[String],
    ease_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType, Keyframe};
    use kagari_vfx::core::property::Animatable;

    if keyframes_raw.is_empty() {
        return Err("No keyframes provided. Use --keyframe frame=value".into());
    }

    let interp = match ease_name.to_lowercase().as_str() {
        "linear" => InterpolationType::Linear,
        "hold" => InterpolationType::Hold,
        "ease-in" => InterpolationType::Bezier {
            outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: BezierControlPoint { influence: 1.0, speed: 0.0 },
            custom_bezier: Some([0.42, 0.0, 1.0, 1.0]),
        },
        "ease-out" => InterpolationType::Bezier {
            outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
            incoming: BezierControlPoint { influence: 0.667, speed: 0.0 },
            custom_bezier: Some([0.0, 0.0, 0.58, 1.0]),
        },
        _ => InterpolationType::Bezier {
            // Default ease-in-out (AE Easy Ease)
            outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
            custom_bezier: Some([0.42, 0.0, 0.58, 1.0]),
        },
    };

    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| format!("Layer '{layer_ref}' not found in composition '{}'", comp.name))?;

    match property.to_lowercase().as_str() {
        "position" => {
            let mut kfs = Vec::new();
            for raw in keyframes_raw {
                let (f_str, v_str) = raw.split_once('=').ok_or("Keyframe must use frame=x,y")?;
                let frame: u32 = f_str.trim().parse().map_err(|_| "Invalid frame number")?;
                let coords: Vec<f32> = v_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect();
                if coords.len() < 2 {
                    return Err(format!("Position keyframe requires x,y coords: '{raw}'").into());
                }
                kfs.push(Keyframe::new(frame, [coords[0], coords[1]], interp));
            }
            kfs.sort_by_key(|k| k.frame);
            layer.transform.position = Animatable::Animated(kfs);
        }
        "scale" => {
            let mut kfs = Vec::new();
            for raw in keyframes_raw {
                let (f_str, v_str) = raw.split_once('=').ok_or("Keyframe must use frame=x,y or frame=uniform")?;
                let frame: u32 = f_str.trim().parse().map_err(|_| "Invalid frame number")?;
                let coords: Vec<f32> = v_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect();
                let scale = if coords.len() == 1 {
                    [coords[0], coords[0]]
                } else if coords.len() >= 2 {
                    [coords[0], coords[1]]
                } else {
                    return Err(format!("Invalid scale keyframe: '{raw}'").into());
                };
                kfs.push(Keyframe::new(frame, scale, interp));
            }
            kfs.sort_by_key(|k| k.frame);
            layer.transform.scale = Animatable::Animated(kfs);
        }
        "opacity" => {
            let mut kfs = Vec::new();
            for raw in keyframes_raw {
                let (f_str, v_str) = raw.split_once('=').ok_or("Keyframe must use frame=opacity")?;
                let frame: u32 = f_str.trim().parse().map_err(|_| "Invalid frame number")?;
                let val: f32 = v_str.trim().parse().map_err(|_| "Invalid opacity value")?;
                kfs.push(Keyframe::new(frame, val, interp));
            }
            kfs.sort_by_key(|k| k.frame);
            layer.transform.opacity = Animatable::Animated(kfs);
        }
        "rotation" => {
            let mut kfs = Vec::new();
            for raw in keyframes_raw {
                let (f_str, v_str) = raw.split_once('=').ok_or("Keyframe must use frame=degrees")?;
                let frame: u32 = f_str.trim().parse().map_err(|_| "Invalid frame number")?;
                let val: f32 = v_str.trim().parse().map_err(|_| "Invalid rotation angle")?;
                kfs.push(Keyframe::new(frame, val, interp));
            }
            kfs.sort_by_key(|k| k.frame);
            layer.transform.rotation = Animatable::Animated(kfs);
        }
        other => {
            return Err(format!(
                "Unsupported animated property '{}'. Use position, scale, opacity, or rotation.",
                other
            )
            .into())
        }
    }

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Animated '{}.{}' with {} keyframes (easing: {}) in {}",
        layer_ref,
        property,
        keyframes_raw.len(),
        ease_name,
        project_path
    );
    Ok(())
}

pub(super) fn cmd_add_effect(
    project_path: &str,
    layer_ref: &str,
    comp_ref: Option<&str>,
    kind: &str,
    param: Option<f32>,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::property::Animatable;
    use kagari_vfx::core::timeline::{Effect, EffectType};

    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| format!("Layer '{layer_ref}' not found in composition '{}'", comp.name))?;

    let effect_id = format!("eff_{}_{}", layer.effects.len(), kind);
    let val = param.unwrap_or(10.0);

    let (name, effect_type) = match kind.to_lowercase().replace(['-', '_'], "").as_str() {
        "blur" | "gaussianblur" => (
            "Gaussian Blur".to_string(),
            EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(val),
            },
        ),
        "glow" => (
            "Glow".to_string(),
            EffectType::Glow {
                threshold: Animatable::new_constant(0.6),
                radius: Animatable::new_constant(val.max(1.0)),
                intensity: Animatable::new_constant(1.5),
                color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
        ),
        "dropshadow" | "shadow" => (
            "Drop Shadow".to_string(),
            EffectType::DropShadow {
                color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                opacity: Animatable::new_constant(75.0),
                direction: Animatable::new_constant(135.0),
                distance: Animatable::new_constant(val),
                softness: Animatable::new_constant(10.0),
            },
        ),
        "vignette" => (
            "Vignette".to_string(),
            EffectType::Vignette {
                intensity: Animatable::new_constant(val),
                roundness: Animatable::new_constant(50.0),
                feather: Animatable::new_constant(50.0),
                color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
        ),
        "tint" | "colortint" => (
            "Color Tint".to_string(),
            EffectType::ColorTint {
                color: Animatable::new_constant([1.0, 0.8, 0.4, 1.0]),
                intensity: Animatable::new_constant(val / 100.0),
            },
        ),
        "chromaticaberration" | "aberration" => (
            "Chromatic Aberration".to_string(),
            EffectType::ChromaticAberration {
                shift_r: Animatable::new_constant(val),
                shift_b: Animatable::new_constant(-val),
                edge_falloff: Animatable::new_constant(1.0),
                iris_linked: true,
            },
        ),
        other => {
            return Err(format!("Unknown effect kind '{}'. Valid: blur, glow, shadow, vignette, tint, chromatic_aberration", other).into());
        }
    };

    let effect = Effect {
        id: effect_id,
        name,
        effect_type,
        enabled: true,
    };
    let effect_name = effect.name.clone();
    let layer_name = layer.name.clone();
    layer.effects.push(effect);

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Added effect '{}' ({}) to layer '{}' in {}",
        effect_name,
        kind,
        layer_name,
        project_path
    );
    Ok(())
}

pub(super) fn cmd_trim(
    project_path: &str,
    layer_ref: &str,
    comp_ref: Option<&str>,
    in_frame: Option<u32>,
    out_frame: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| format!("Layer '{layer_ref}' not found in composition '{}'", comp.name))?;

    if let Some(inf) = in_frame {
        layer.in_frame = inf;
    }
    if let Some(outf) = out_frame {
        layer.out_frame = outf;
    }

    let layer_name = layer.name.clone();
    let in_f = layer.in_frame;
    let out_f = layer.out_frame;

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Trimmed layer '{}' span to {}..={} in {}",
        layer_name, in_f, out_f, project_path
    );
    Ok(())
}

pub(super) fn cmd_transition(
    project_path: &str,
    layer_ref: &str,
    comp_ref: Option<&str>,
    kind: &str,
    duration: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType, Keyframe};
    use kagari_vfx::core::property::Animatable;

    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let comp_w = comp.width as f32;
    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| format!("Layer '{layer_ref}' not found in composition '{}'", comp.name))?;

    let dur = duration.max(1);
    let ease = InterpolationType::Bezier {
        outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
        incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
        custom_bezier: Some([0.42, 0.0, 0.58, 1.0]),
    };

    match kind.to_lowercase().replace(['-', '_'], "").as_str() {
        "fadein" => {
            let start = layer.in_frame;
            let end = (start + dur).min(layer.out_frame);
            let kfs = vec![
                Keyframe::new(start, 0.0, ease),
                Keyframe::new(end, 100.0, ease),
            ];
            layer.transform.opacity = Animatable::Animated(kfs);
        }
        "fadeout" => {
            let end = layer.out_frame;
            let start = end.saturating_sub(dur).max(layer.in_frame);
            let kfs = vec![
                Keyframe::new(start, 100.0, ease),
                Keyframe::new(end, 0.0, ease),
            ];
            layer.transform.opacity = Animatable::Animated(kfs);
        }
        "zoomout" => {
            let end = layer.out_frame;
            let start = end.saturating_sub(dur).max(layer.in_frame);
            let base_s = layer.transform.scale.evaluate(start);
            let target_s = [base_s[0] * 1.8, base_s[1] * 1.8];
            layer.transform.scale = Animatable::Animated(vec![
                Keyframe::new(start, base_s, ease),
                Keyframe::new(end, target_s, ease),
            ]);
            layer.transform.opacity = Animatable::Animated(vec![
                Keyframe::new(start, 100.0, ease),
                Keyframe::new(end, 0.0, ease),
            ]);
        }
        "slideout" => {
            let end = layer.out_frame;
            let start = end.saturating_sub(dur).max(layer.in_frame);
            let base_p = layer.transform.position.evaluate(start);
            let target_p = [base_p[0] + comp_w * 0.6, base_p[1]];
            layer.transform.position = Animatable::Animated(vec![
                Keyframe::new(start, base_p, ease),
                Keyframe::new(end, target_p, ease),
            ]);
            layer.transform.opacity = Animatable::Animated(vec![
                Keyframe::new(start, 100.0, ease),
                Keyframe::new(end, 0.0, ease),
            ]);
        }
        other => {
            return Err(format!("Unknown transition '{}'. Valid options: fade-in, fade-out, zoom-out, slide-out", other).into());
        }
    }

    let layer_name = layer.name.clone();
    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Applied transition '{}' ({} frames) to layer '{}' in {}",
        kind, dur, layer_name, project_path
    );
    Ok(())
}

pub(super) fn cmd_parent(
    project_path: &str,
    child_ref: &str,
    parent_ref: &str,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let child_idx = comp
        .layers
        .iter()
        .position(|l| l.name == child_ref || l.id == child_ref)
        .ok_or_else(|| format!("Child layer '{child_ref}' not found in composition '{}'", comp.name))?;

    let child_name = comp.layers[child_idx].name.clone();

    if parent_ref.eq_ignore_ascii_case("none") {
        comp.layers[child_idx].parent_id = None;
        let json = serde_json::to_string_pretty(&project)?;
        std::fs::write(project_path, json)?;
        eprintln!("Unparented layer '{}' in {}", child_name, project_path);
        return Ok(());
    }

    let parent_layer = comp
        .layers
        .iter()
        .find(|l| l.name == parent_ref || l.id == parent_ref)
        .ok_or_else(|| format!("Parent layer '{parent_ref}' not found in composition '{}'", comp.name))?;

    let parent_id = parent_layer.id.clone();
    let parent_name = parent_layer.name.clone();

    if parent_id == comp.layers[child_idx].id {
        return Err("A layer cannot be parented to itself".into());
    }

    comp.layers[child_idx].parent_id = Some(parent_id);
    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Parented layer '{}' → '{}' in {}",
        child_name, parent_name, project_path
    );
    Ok(())
}

pub(super) fn cmd_matte(
    project_path: &str,
    layer_ref: &str,
    mode_str: &str,
    source_ref: Option<&str>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::timeline::TrackMatteMode;

    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let layer_idx = comp
        .layers
        .iter()
        .position(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| format!("Layer '{layer_ref}' not found in composition '{}'", comp.name))?;

    let mode = match mode_str.to_lowercase().replace(['-', '_'], "").as_str() {
        "none" => TrackMatteMode::None,
        "alpha" | "alphamatte" => TrackMatteMode::AlphaMatte,
        "alphainv" | "alphamatteinverted" | "alphainverted" => TrackMatteMode::AlphaMatteInverted,
        "luma" | "lumamatte" => TrackMatteMode::LumaMatte,
        "lumainv" | "lumamatteinverted" | "lumainverted" => TrackMatteMode::LumaMatteInverted,
        other => return Err(format!("Unknown matte mode '{other}'. Valid: alpha, alpha-inv, luma, luma-inv, none").into()),
    };

    let source_id = if let Some(s) = source_ref {
        let src_layer = comp
            .layers
            .iter()
            .find(|l| l.name == s || l.id == s)
            .ok_or_else(|| format!("Source matte layer '{s}' not found in composition '{}'", comp.name))?;
        Some(src_layer.id.clone())
    } else {
        None
    };

    let layer_name = comp.layers[layer_idx].name.clone();
    comp.layers[layer_idx].track_matte = mode;
    comp.layers[layer_idx].track_matte_layer_id = source_id.clone();

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(project_path, json)?;
    eprintln!(
        "Set Track Matte on '{}': mode={:?}, source={:?} in {}",
        layer_name, mode, source_id, project_path
    );
    Ok(())
}
