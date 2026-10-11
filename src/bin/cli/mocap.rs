pub(super) fn parse_point(value: &str) -> Result<[f32; 2], String> {
    let mut parts = value.split(',');
    let x = parts
        .next()
        .ok_or("point must be x,y")?
        .parse::<f32>()
        .map_err(|_| "invalid x")?;
    let y = parts
        .next()
        .ok_or("point must be x,y")?
        .parse::<f32>()
        .map_err(|_| "invalid y")?;
    if parts.next().is_some() || !x.is_finite() || !y.is_finite() {
        return Err("point must contain two finite numbers".into());
    }
    Ok([x, y])
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cmd_mocap(
    directory: &str,
    points: &[[f32; 2]],
    output: &str,
    block_radius: i32,
    search_radius: i32,
    min_confidence: f32,
    pose: bool,
    fps: f32,
    max_features: usize,
    feature_spacing: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::optical_flow_timewarp::{
        detect_markerless_features, estimate_markerless_pose_from_tracks,
        filter_pose_frames_by_quality, markerless_pose_to_bvh, markerless_pose_to_csv,
        markerless_pose_to_json, name_and_connect_markerless_pose_track,
        track_markerless_motion_stream,
    };
    let entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    let mut numbered_paths = entries
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()) == Some("png")
                && path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("frame_"))
        })
        .map(|path| {
            let frame_number = path
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix("frame_"))
                .and_then(|number| number.parse::<u64>().ok())
                .ok_or_else(|| {
                    format!(
                        "Mocap frame filename must use frame_<number>.png: {}",
                        path.display()
                    )
                })?;
            Ok((frame_number, path))
        })
        .collect::<Result<Vec<_>, String>>()?;
    numbered_paths.sort_by_key(|(frame_number, _)| *frame_number);
    if numbered_paths.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("Mocap frame sequence contains duplicate frame numbers".into());
    }
    let paths = numbered_paths
        .into_iter()
        .map(|(_, path)| path)
        .collect::<Vec<_>>();
    if paths.is_empty() || (!pose && points.is_empty()) {
        return Err(if pose {
            "frames are required"
        } else {
            "frames and at least one --point are required"
        }
        .into());
    }
    let (first_frame, width, height) = load_mocap_frame(&paths[0], None)?;
    let seed_points = if pose {
        detect_markerless_features(&first_frame, width, height, max_features, feature_spacing)
    } else {
        points.to_vec()
    };
    let later_frames = paths.into_iter().skip(1).map(move |path| {
        load_mocap_frame(&path, Some((width, height)))
            .map(|(pixels, _, _)| pixels)
            .map_err(|error| error.to_string())
    });
    let frames = std::iter::once(Ok(first_frame)).chain(later_frames);
    let tracks = track_markerless_motion_stream(
        frames,
        width,
        height,
        &seed_points,
        block_radius,
        search_radius,
    )
    .map_err(std::io::Error::other)?;
    if pose {
        let mut estimated =
            estimate_markerless_pose_from_tracks(&seed_points, &tracks, width, height);
        let _ = filter_pose_frames_by_quality(&mut estimated, 1, min_confidence);
        let named = name_and_connect_markerless_pose_track(&estimated);
        let serialized = match std::path::Path::new(output)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json")
        {
            "bvh" => markerless_pose_to_bvh(
                &named,
                if fps.is_finite() && fps > 0.0 {
                    1.0 / fps
                } else {
                    1.0 / 30.0
                },
            ),
            "csv" => markerless_pose_to_csv(&named),
            _ => {
                let pose_value: serde_json::Value =
                    serde_json::from_str(&markerless_pose_to_json(&estimated)?)?;
                serde_json::to_string_pretty(&serde_json::json!({
                    "width": width,
                    "height": height,
                    "fps": if fps.is_finite() && fps > 0.0 { fps } else { 30.0 },
                    "max_features": max_features,
                    "feature_spacing": feature_spacing,
                    "pose": pose_value,
                }))?
            }
        };
        std::fs::write(output, serialized)?;
        println!("wrote pose track to {}", output);
        return Ok(());
    }
    let json_tracks = tracks
        .iter()
        .map(|track| {
            let samples = track
                .samples
                .iter()
                .filter(|sample| {
                    sample.confidence.is_finite() && sample.confidence >= min_confidence
                })
                .map(|sample| {
                    serde_json::json!({
                        "frame": sample.frame,
                        "x": sample.position[0],
                        "y": sample.position[1],
                        "confidence": sample.confidence,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({ "samples": samples })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "width": width, "height": height, "tracks": json_tracks
        }))?,
    )?;
    println!("wrote {} track(s) to {}", tracks.len(), output);
    Ok(())
}

fn load_mocap_frame(
    path: &std::path::Path,
    expected_dimensions: Option<(u32, u32)>,
) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    const MAX_DIMENSION: u32 = 16_384;
    let max_flow_pixels = kagari_vfx::core::optical_flow_timewarp::MAX_DENSE_FLOW_PIXELS as u64;
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut decoder = png::Decoder::new(&mut reader);
    let info = decoder
        .read_header_info()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let (width, height) = (info.width, info.height);
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(format!(
            "Frame {:?} dimensions {}x{} are outside the supported range (maximum {}x{})",
            path, width, height, MAX_DIMENSION, MAX_DIMENSION
        )
        .into());
    }
    if u64::from(width) * u64::from(height) > max_flow_pixels {
        return Err(format!(
            "Frame {:?} is {}x{}; markerless flow supports at most {} pixels",
            path, width, height, max_flow_pixels
        )
        .into());
    }
    if expected_dimensions.is_some_and(|expected| expected != (width, height)) {
        return Err("all mocap frames must have identical dimensions".into());
    }
    let pixels = image::open(path)?.to_rgba8().into_raw();
    Ok((pixels, width, height))
}
