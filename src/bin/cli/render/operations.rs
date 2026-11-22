use super::*;

pub fn cmd_beats(
    file: &str,
    fps: f32,
    duration: u32,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(file);
    if !path.is_file() {
        return Err(format!("Audio file '{}' not found", file).into());
    }

    let beats = kagari_vfx::core::audio_engine::detect_beat_frames(path, duration, fps);
    if json {
        let json_str = serde_json::to_string(&beats)?;
        println!("{}", json_str);
    } else {
        println!(
            "Detected {} transient beat onsets in '{}' (fps: {}):",
            beats.len(),
            file,
            fps
        );
        for b in &beats {
            let sec = *b as f32 / fps;
            println!("  frame {:5}  ({:6.2}s)", b, sec);
        }
    }
    Ok(())
}

pub fn cmd_preview(
    project_path: &str,
    comp_ref: Option<&str>,
    output: &str,
    scale: f32,
    step: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = find_comp(&project, comp_ref)?;
    let scale = scale.clamp(0.1, 1.0);
    let preview_w = ((comp.width as f32 * scale).round() as u32).max(64);
    let preview_h = ((comp.height as f32 * scale).round() as u32).max(64);

    eprintln!(
        "Draft Preview: Rendering '{}' at {:.0}% resolution ({}x{}, step={}) → {}",
        comp.name,
        scale * 100.0,
        preview_w,
        preview_h,
        step.max(1),
        output
    );

    cmd_render(RenderArgs {
        project_path: project_path.to_string(),
        comp_ref: comp_ref.map(|s| s.to_string()),
        output: output.to_string(),
        format: OutputFormat::Mp4,
        frames: None,
        from: 0,
        to: None,
        width: Some(preview_w),
        height: Some(preview_h),
        exposure: 0.0,
        lut: 0,
        lut_file: None,
        color_space: None,
        step: step.max(1),
        threads: 0,
        skip_existing: false,
        mfr: true,
        log_format: "standard".to_string(),
        bindings: Vec::new(),
        overrides: Vec::new(),
        expressions: Vec::new(),
    })
}

pub fn cmd_bundle(project_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = std::path::Path::new(project_path);
    let json = kagari_vfx::core::project_migration::read_project_json_file(project_path)?;
    let (project, document) =
        kagari_vfx::core::project_migration::load_project_document_or_legacy(&json)?;
    let document = document
        .unwrap_or_else(|| kagari_vfx::core::production_document::ProductionDocument::new(project));
    let source_project_dir = project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let destination_parent = std::path::Path::new(output_dir);
    std::fs::create_dir_all(destination_parent)?;
    let (bundle_project, asset_count) =
        kagari_vfx::core::project_migration::collect_production_document(
            &document,
            destination_parent,
            source_project_dir,
        )
        .map_err(std::io::Error::other)?;

    eprintln!(
        "\nProject Bundle created successfully at '{}' (collected {} media files)",
        bundle_project
            .parent()
            .unwrap_or(destination_parent)
            .display(),
        asset_count
    );
    Ok(())
}

pub fn cmd_remap(
    project_path: &str,
    layer_name: &str,
    stretch: Option<f32>,
    reverse: bool,
    freeze: Option<u32>,
    frame_blending: bool,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    if let Some(factor) = stretch {
        if !factor.is_finite() || factor <= 0.001 {
            return Err("Time stretch factor must be finite and greater than 0.001.".into());
        }
        let requested_end = u64::from(layer.in_frame).saturating_add(
            (f64::from(layer.duration_frames()) * f64::from(factor)).round() as u64,
        );
        if requested_end > u64::from(comp.duration_frames) {
            return Err(format!(
                "Time stretch would move layer '{}' beyond composition '{}'; extend the composition or use a shorter stretch factor.",
                layer.name, comp.name
            )
            .into());
        }
        layer.time_stretch(factor);
        eprintln!(
            "Applied time stretch factor {:.2}x to layer '{}'",
            factor, layer.name
        );
    }
    if reverse {
        layer.time_reverse();
        eprintln!("Applied time-reverse playback to layer '{}'", layer.name);
    }
    if let Some(freeze_frame) = freeze {
        layer.freeze_at(freeze_frame);
        eprintln!(
            "Applied freeze frame at source frame {} to layer '{}'",
            freeze_frame, layer.name
        );
    }
    if frame_blending {
        layer.frame_blending = true;
        eprintln!("Enabled adjacent frame blending on layer '{}'", layer.name);
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_track(
    project_path: &str,
    layer_name: &str,
    tracker_idx: usize,
    pos_str: Option<&str>,
    search_size: f32,
    apply_to: Option<&str>,
    apply_pos: bool,
    apply_rot: bool,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let requested_position = pos_str
        .map(super::mocap::parse_point)
        .transpose()
        .map_err(std::io::Error::other)?;
    if !search_size.is_finite() {
        return Err("search size must be a finite number".into());
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let (source_layer_idx, in_frame, out_frame) = {
        let (idx, layer) = comp
            .layers
            .iter_mut()
            .enumerate()
            .find(|(_, l)| l.name == layer_name || l.id == layer_name)
            .ok_or_else(|| {
                format!(
                    "Source layer '{layer_name}' not found in composition '{}'",
                    comp.name
                )
            })?;

        if layer.trackers.len() <= tracker_idx {
            let initial_pos =
                requested_position.unwrap_or([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
            let mut tracker = kagari_vfx::core::timeline::TrackerPoint::new(
                format!("tracker_{}", tracker_idx + 1),
                format!("Tracker {}", tracker_idx + 1),
                initial_pos,
            );
            tracker.search_size = search_size.clamp(4.0, 128.0);
            layer.trackers.push(tracker);
        } else if let Some(position) = requested_position {
            layer.trackers[tracker_idx].position =
                kagari_vfx::core::property::Animatable::new_constant(position);
        }
        (idx, layer.in_frame, layer.out_frame)
    };

    eprintln!(
        "Analyzing motion tracking on '{}' (tracker #{}, frames {}..={})...",
        layer_name, tracker_idx, in_frame, out_frame
    );
    kagari_vfx::core::tracker_engine::TrackerEngine::analyze_track(
        comp,
        source_layer_idx,
        tracker_idx,
        in_frame,
        out_frame,
    );

    if let Some(target_name) = apply_to {
        let target_layer_idx = comp
            .layers
            .iter()
            .position(|l| l.name == target_name || l.id == target_name)
            .ok_or_else(|| {
                format!(
                    "Target layer '{target_name}' not found in composition '{}'",
                    comp.name
                )
            })?;

        kagari_vfx::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
            comp,
            source_layer_idx,
            tracker_idx,
            target_layer_idx,
            apply_pos,
            apply_rot,
        );
        eprintln!(
            "Baked and applied motion tracking data to target layer '{}' (pos: {}, rot: {})",
            target_name, apply_pos, apply_rot
        );
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_text_anim(
    project_path: &str,
    layer_name: &str,
    preset: &str,
    duration: u32,
    start_frame: Option<u32>,
    shape_str: &str,
    blur: f32,
    tracking: f32,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let shape = match shape_str.to_lowercase().replace('-', "_").as_str() {
        "square" => kagari_vfx::core::text_animator::SelectorShape::Square,
        "ramp_up" | "rampup" => kagari_vfx::core::text_animator::SelectorShape::RampUp,
        "ramp_down" | "rampdown" => kagari_vfx::core::text_animator::SelectorShape::RampDown,
        "triangle" => kagari_vfx::core::text_animator::SelectorShape::Triangle,
        "round" => kagari_vfx::core::text_animator::SelectorShape::Round,
        "smooth" => kagari_vfx::core::text_animator::SelectorShape::Smooth,
        "wobble" => kagari_vfx::core::text_animator::SelectorShape::Wobble,
        "random" => kagari_vfx::core::text_animator::SelectorShape::Random,
        _ => kagari_vfx::core::text_animator::SelectorShape::RampUp,
    };

    let start_f = start_frame.unwrap_or(layer.in_frame);
    let end_f = start_f.saturating_add(duration.max(1));

    let mut anim = kagari_vfx::core::text_animator::TextAnimatorSettings {
        enabled: true,
        selector: kagari_vfx::core::text_animator::RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape,
            ease_high: 20.0,
            ease_low: 20.0,
            random_order: false,
            char_start: -1,
            char_end: -1,
            offset_anim: Some(kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(
                    start_f,
                    -100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
                kagari_vfx::core::keyframe::Keyframe::new(
                    end_f,
                    100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
            ])),
            expression: None,
        },
        position_offset: [0.0, 0.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking,
        rotation: 0.0,
        blur_amount: blur,
    };

    match preset.to_lowercase().as_str() {
        "typewriter" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Square;
            anim.position_offset = [0.0, 0.0];
            anim.opacity = 0.0;
        }
        "cascade" | "rise" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::RampUp;
            anim.position_offset = [0.0, 50.0];
            anim.opacity = 0.0;
            anim.scale = [0.8, 0.8];
        }
        "blur-in" | "blur_in" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Smooth;
            anim.blur_amount = if blur > 0.0 { blur } else { 25.0 };
            anim.opacity = 0.0;
        }
        "tracking" | "expand" => {
            anim.tracking = if tracking != 0.0 { tracking } else { 15.0 };
            anim.opacity = 0.0;
        }
        "wave" => {
            anim.selector.shape = kagari_vfx::core::text_animator::SelectorShape::Wobble;
            anim.position_offset = [0.0, -30.0];
        }
        _ => {}
    }

    layer.text_animator = Some(anim);
    eprintln!(
        "Configured text animator preset '{}' on layer '{}' (frames {}..={})",
        preset, layer.name, start_f, end_f
    );

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_camera(
    project_path: &str,
    pos_str: Option<&str>,
    rot_str: Option<&str>,
    fov: Option<f32>,
    focus_dist: Option<f32>,
    aperture: Option<f32>,
    dof: bool,
    max_blur: Option<f32>,
    iris_sides: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let position = pos_str
        .map(|value| parse_finite_components(value, "Camera position"))
        .transpose()?;
    let rotation = rot_str
        .map(|value| parse_finite_components(value, "Camera rotation"))
        .transpose()?;
    for (name, value) in [
        ("FOV", fov),
        ("focus distance", focus_dist),
        ("aperture", aperture),
        ("maximum blur", max_blur),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(format!("camera {name} must be finite").into());
        }
    }
    for (name, values) in [
        ("position", position.as_ref()),
        ("rotation", rotation.as_ref()),
    ] {
        if values.is_some_and(|values| values.len() != 3) {
            return Err(format!("camera {name} requires exactly three values").into());
        }
    }
    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;
    let cam = comp.resolve_camera_mut();

    if let Some(parts) = position {
        cam.transform.position =
            kagari_vfx::core::property::Animatable::new_constant([parts[0], parts[1], parts[2]]);
        eprintln!(
            "Set camera position: [{:.1}, {:.1}, {:.1}]",
            parts[0], parts[1], parts[2]
        );
    }
    if let Some(parts) = rotation {
        cam.transform.rotation =
            kagari_vfx::core::property::Animatable::new_constant([parts[0], parts[1], parts[2]]);
        eprintln!(
            "Set camera rotation: [{:.1}°, {:.1}°, {:.1}°]",
            parts[0], parts[1], parts[2]
        );
    }
    if let Some(f) = fov {
        cam.fov_degrees = f.clamp(5.0, 160.0);
        eprintln!("Set camera FOV: {:.1}°", cam.fov_degrees);
    }
    if let Some(fd) = focus_dist {
        cam.focus_distance = fd.max(10.0);
        eprintln!("Set camera focus distance: {:.1}", cam.focus_distance);
    }
    if let Some(ap) = aperture {
        cam.aperture = ap.max(0.5);
        eprintln!("Set camera aperture: f/{:.1}", cam.aperture);
    }
    if dof {
        cam.dof_enabled = true;
        eprintln!("Enabled cinematic Depth of Field (bokeh)");
    }
    if let Some(mb) = max_blur {
        cam.dof_max_blur = mb.clamp(1.0, 64.0);
        eprintln!("Set camera max bokeh blur: {:.1}px", cam.dof_max_blur);
    }
    if let Some(sides) = iris_sides {
        cam.dof_iris_sides = sides;
        eprintln!("Set camera iris blade count: {}", sides);
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_shape_anim(
    project_path: &str,
    layer_name: &str,
    trim_start: Option<f32>,
    trim_end: Option<f32>,
    trim_duration: Option<u32>,
    repeater_copies: Option<u32>,
    repeater_offset: Option<&str>,
    repeater_rotation: Option<f32>,
    repeater_scale: Option<&str>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, value) in [
        ("trim start", trim_start),
        ("trim end", trim_end),
        ("repeater rotation", repeater_rotation),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(format!("{name} must be finite").into());
        }
    }
    let repeater_position = repeater_offset
        .map(|value| parse_finite_components(value, "Repeater position offset"))
        .transpose()?;
    let repeater_scale_values = repeater_scale
        .map(|value| parse_finite_components(value, "Repeater scale offset"))
        .transpose()?;
    for (name, values) in [
        ("position offset", repeater_position.as_ref()),
        ("scale offset", repeater_scale_values.as_ref()),
    ] {
        if values.is_some_and(|values| values.len() != 2) {
            return Err(format!("Repeater {name} requires exactly two values").into());
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

    if trim_start.is_some() || trim_end.is_some() || trim_duration.is_some() {
        let mut trim = layer.trim_paths.take().unwrap_or_default();
        if let Some(dur) = trim_duration {
            let start_f = layer.in_frame;
            let end_f = start_f.saturating_add(dur.max(1));
            trim.start =
                kagari_vfx::core::property::Animatable::new_constant(trim_start.unwrap_or(0.0));
            trim.end = kagari_vfx::core::property::Animatable::Animated(vec![
                kagari_vfx::core::keyframe::Keyframe::new(
                    start_f,
                    0.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
                kagari_vfx::core::keyframe::Keyframe::new(
                    end_f,
                    100.0,
                    kagari_vfx::core::keyframe::InterpolationType::Linear,
                ),
            ]);
            eprintln!(
                "Configured animated trim paths sweep on '{}' (frames {}..={})",
                layer.name, start_f, end_f
            );
        } else {
            if let Some(s) = trim_start {
                trim.start =
                    kagari_vfx::core::property::Animatable::new_constant(s.clamp(0.0, 100.0));
            }
            if let Some(e) = trim_end {
                trim.end =
                    kagari_vfx::core::property::Animatable::new_constant(e.clamp(0.0, 100.0));
            }
            eprintln!("Configured static trim paths on '{}'", layer.name);
        }
        layer.trim_paths = Some(trim);
    }

    if repeater_copies.is_some()
        || repeater_offset.is_some()
        || repeater_rotation.is_some()
        || repeater_scale.is_some()
    {
        let mut rep = layer.shape_repeater.take().unwrap_or_default();
        if let Some(copies) = repeater_copies {
            rep.copies = copies.max(1);
            eprintln!("Set shape repeater copies: {}", rep.copies);
        }
        if let Some(parts) = repeater_position {
            rep.position_offset = [parts[0], parts[1]];
            eprintln!(
                "Set shape repeater position offset: [{:.1}, {:.1}]",
                parts[0], parts[1]
            );
        }
        if let Some(rot) = repeater_rotation {
            rep.rotation_offset_deg = rot;
            eprintln!("Set shape repeater rotation offset: {:.1}°", rot);
        }
        if let Some(parts) = repeater_scale_values {
            rep.scale_offset = [parts[0], parts[1]];
            eprintln!(
                "Set shape repeater scale offset: [{:.2}, {:.2}]",
                parts[0], parts[1]
            );
        }
        layer.shape_repeater = Some(rep);
    }

    save_cli_project(project_path, &project)?;
    eprintln!("Updated project file: {}", project_path);
    Ok(())
}
