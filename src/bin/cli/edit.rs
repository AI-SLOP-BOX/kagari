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
