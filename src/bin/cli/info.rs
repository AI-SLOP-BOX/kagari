use super::*;

pub(super) fn cmd_effects(json_output: bool) {
    let effects = vec![
        ("GaussianBlur", "blur", "Fast box blur (CPU)"),
        ("ColorTint", "color", "Color tint overlay"),
        ("DropShadow", "stylize", "Drop shadow with blur"),
        ("ChromaticAberration", "lens", "RGB channel split (CPU)"),
        ("Vignette", "lens", "Darkened edges (CPU)"),
        ("Levels", "color", "Input/output levels + gamma (CPU)"),
        ("HueSaturation", "color", "HSL adjustment (CPU)"),
        ("Glow", "stylize", "Bloom / glow effect"),
        ("MotionBlur", "blur", "Shutter-based motion blur (CPU)"),
        ("MeshWarp", "distort", "4-corner mesh warp (CPU)"),
        ("ColorGradeLUT", "color", "LUT color grading (CPU)"),
        ("ColorSpaceConvert", "color", "Color space transform (CPU)"),
        ("FilmGrain", "noise", "Film grain noise (CPU)"),
        (
            "FractalNoise",
            "noise",
            "Procedural noise (fBm/turb/ridge) (CPU)",
        ),
        ("Curves", "color", "Per-channel bezier tone curve (CPU)"),
        (
            "DisplacementMap",
            "distort",
            "Layer-based displacement warp (CPU)",
        ),
        (
            "CompoundBlur",
            "blur",
            "Variable blur with intensity map (CPU)",
        ),
        ("Minimax", "matte", "Dilate/erode matte (CPU)"),
        ("ShiftChannels", "color", "RGBA channel swap/remap (CPU)"),
        ("Twirl", "distort", "Twirl distortion (CPU)"),
        ("Bulge", "distort", "Bulge distortion (CPU)"),
        ("Posterize", "color", "Reduce color levels (CPU)"),
        ("Invert", "color", "Invert colors (CPU)"),
        ("Offset", "distort", "Pixel offset with wrap (CPU)"),
        ("DirectionalBlur", "blur", "Directional blur (CPU)"),
        ("RadialBlur", "blur", "Radial blur (CPU)"),
        ("Sharpen", "blur", "Unsharp mask (CPU)"),
        ("Threshold", "color", "Binary threshold (CPU)"),
        ("LinearWipe", "transition", "Linear wipe transition (CPU)"),
        ("SimpleChoker", "matte", "Matte choker (CPU)"),
        ("ChromaKey", "keying", "Green screen key (CPU)"),
        ("Spherize", "distort", "Sphere distortion (CPU)"),
        (
            "TurbulentDisplace",
            "distort",
            "Turbulence displacement (CPU)",
        ),
        ("Colorama", "color", "Color cycle (CPU)"),
    ];
    if json_output {
        let arr: Vec<serde_json::Value> = effects
            .iter()
            .map(|(name, cat, desc)| {
                serde_json::json!({
                    "name": name,
                    "category": cat,
                    "description": desc,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).unwrap());
    } else {
        println!("Available Effects:");
        println!("==================");
        for (name, _cat, desc) in &effects {
            println!("  {:<24} {}", name, desc);
        }
        println!("\nTotal: {} effects", effects.len());
    }
}

pub(super) fn cmd_info(project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;

    println!("Project: {} compositions", project.compositions.len());
    println!("Assets: {}", project.assets.len());
    println!();

    for (i, comp) in project.compositions.iter().enumerate() {
        println!("Composition {}: \"{}\"", i, comp.name);
        println!("  Size: {}x{}", comp.width, comp.height);
        println!("  FPS: {}", comp.fps);
        println!(
            "  Duration: {} frames ({:.2}s)",
            comp.duration_frames,
            comp.duration_frames as f32 / comp.fps as f32
        );
        println!("  Layers: {}", comp.layers.len());

        for layer in comp.layers.iter() {
            let type_name = match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Solid { .. } => "Solid",
                kagari_vfx::core::timeline::LayerType::Text { .. } => "Text",
                kagari_vfx::core::timeline::LayerType::Image { .. } => "Image",
                kagari_vfx::core::timeline::LayerType::Model3D { .. } => "Model3D",
                kagari_vfx::core::timeline::LayerType::Video { .. } => "Video",
                kagari_vfx::core::timeline::LayerType::Shape { .. } => "Shape",
                kagari_vfx::core::timeline::LayerType::Null => "Null",
                kagari_vfx::core::timeline::LayerType::PreComp { .. } => "PreComp",
                kagari_vfx::core::timeline::LayerType::Audio { .. } => "Audio",
                kagari_vfx::core::timeline::LayerType::AdjustmentLayer => "Adjustment",
                kagari_vfx::core::timeline::LayerType::Particle { .. } => "Particle",
            };
            let vis = if layer.visible { "●" } else { "○" };
            println!(
                "    {} [{}] {} ({} effects)",
                vis,
                type_name,
                layer.name,
                layer.effects.len()
            );
        }
        println!();
    }

    Ok(())
}

pub(super) fn cmd_production_info(
    project_path: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(
        project_path,
    ))
    .map_err(std::io::Error::other)?;
    let document = kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
        .map_err(|error| format!("Not a valid production document: {error}"))?;
    let clock = document.clock();
    if json_output {
        let output = production_info_value(&document);
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    println!("Production document schema: {}", document.schema_version);
    println!("Audio sample rate: {} Hz", document.audio.sample_rate);
    println!("Audio channels: {}", document.audio.channels.len());
    println!("Tempo changes: {}", document.tempo.changes.len());
    println!("Initial BPM: {:.3}", document.tempo.changes[0].bpm);
    println!("Automation bindings: {}", document.bindings.len());
    println!(
        "Beat at 1 second: {:.3}",
        clock.beat(kagari_vfx::core::unified_time::Time::new(1, 1))
    );
    for binding in &document.bindings {
        println!(
            "  {} -> {} ({} points)",
            binding.source,
            binding.target,
            binding.curve.points.len()
        );
    }
    Ok(())
}

pub(super) fn production_info_value(
    document: &kagari_vfx::core::production_document::ProductionDocument,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": document.schema_version,
        "sample_rate": document.audio.sample_rate,
        "audio_channels": document.audio.channels.len(),
        "tempo_changes": document.tempo.changes.len(),
        "initial_bpm": document.tempo.changes[0].bpm,
        "automation_bindings": document.bindings.iter().map(|binding| serde_json::json!({
            "source": binding.source,
            "target": binding.target,
            "points": binding.curve.points.len(),
        })).collect::<Vec<_>>(),
    })
}

pub(super) fn doctor_report_value() -> serde_json::Value {
    let ofx_count: usize = kagari_vfx::core::openfx_bridge::ofx_search_paths()
        .iter()
        .map(|p| kagari_vfx::core::openfx_bridge::discover_ofx_plugins(p).len())
        .sum();
    let gpu_enabled = cfg!(any(feature = "wgpu", feature = "cli-gpu"));
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "gpu_compiled": gpu_enabled,
        "openfx_plugins_detected": ofx_count,
        "subsystems": {
            "aces_color_management": true,
            "deep_compositing": true,
            "compositor_node_graph": true,
            "color_wheel_3way": true,
            "curves_monotone_lut": true,
            "color_scopes": true,
            "pro_keyer": true,
            "sapphire_optical_fx": gpu_enabled,
            "saber_beam_fx": gpu_enabled,
            "mograph_cloner": true,
            "shadow_map_pcf": gpu_enabled,
            "extruded_mesh_3d": true,
            "sfm_camera_tracker": true,
            "optical_flow_gpu": gpu_enabled,
            "temporal_cleanplate": true,
            "roto_edge_refine": true,
            "film_grain_pro": true,
            "roi_scanline": true,
            "premult_guard": true,
            "auto_proxy_lru_cache": true,
            "audio_wav_mixdown": true,
        }
    })
}

pub(super) fn cmd_doctor(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let val = doctor_report_value();
    if json {
        println!("{}", serde_json::to_string_pretty(&val)?);
    } else {
        println!(
            "Kagari Doctor: OS={}/{} GPU={} OpenFX={}",
            val["os"], val["arch"], val["gpu_compiled"], val["openfx_plugins_detected"]
        );
    }
    Ok(())
}

pub(super) fn presets_report_value() -> serde_json::Value {
    let timeline_presets: Vec<serde_json::Value> = kagari_vfx::core::presets::NAMES
        .iter()
        .map(|&name| {
            serde_json::json!({
                "name": name,
                "category": kagari_vfx::core::presets::category_of(name),
            })
        })
        .collect();
    let composer_presets: Vec<serde_json::Value> =
        kagari_vfx::core::motion_composer::COMPOSER_BUNDLES
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "name": p.display_name,
                    "category": format!("{:?}", p.category),
                })
            })
            .collect();
    serde_json::json!({
        "timeline_presets": timeline_presets,
        "composer_bundles": composer_presets,
    })
}

pub(super) fn cmd_presets(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let val = presets_report_value();
    if json {
        println!("{}", serde_json::to_string_pretty(&val)?);
    } else {
        let tl_n = val["timeline_presets"].as_array().map_or(0, Vec::len);
        let ac_n = val["composer_bundles"].as_array().map_or(0, Vec::len);
        println!("Kagari Presets: {tl_n} timeline presets, {ac_n} composer bundles");
    }
    Ok(())
}
