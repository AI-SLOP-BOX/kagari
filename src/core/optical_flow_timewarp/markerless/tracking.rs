use super::*;

pub fn detect_markerless_features(
    frame_rgba: &[u8],
    width: u32,
    height: u32,
    max_features: usize,
    min_spacing: u32,
) -> Vec<[f32; 2]> {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if width < 5 || height < 5 || frame_rgba.len() != size || max_features == 0 {
        return Vec::new();
    }
    let luma = |x: u32, y: u32| -> f32 {
        let i = (y as usize * width as usize + x as usize) * 4;
        0.299 * frame_rgba[i] as f32
            + 0.587 * frame_rgba[i + 1] as f32
            + 0.114 * frame_rgba[i + 2] as f32
    };
    let mut candidates = Vec::new();
    for y in 2..height - 2 {
        for x in 2..width - 2 {
            let gx = luma(x + 1, y) - luma(x - 1, y);
            let gy = luma(x, y + 1) - luma(x, y - 1);
            let score = gx * gx + gy * gy;
            let is_local_max = (-1..=1).all(|dy| {
                (-1..=1).all(|dx| {
                    if dx == 0 && dy == 0 {
                        return true;
                    }
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    let ngx = luma(nx + 1, ny) - luma(nx - 1, ny);
                    let ngy = luma(nx, ny + 1) - luma(nx, ny - 1);
                    score >= ngx * ngx + ngy * ngy
                })
            });
            if score.is_finite() && score > 1.0 && is_local_max {
                candidates.push((score, [x as f32, y as f32]));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let spacing = min_spacing.max(1) as f32;
    let mut selected = Vec::with_capacity(max_features.min(candidates.len()));
    for (_, point) in candidates {
        if selected
            .iter()
            .all(|other: &[f32; 2]| (point[0] - other[0]).hypot(point[1] - other[1]) >= spacing)
        {
            selected.push(point);
            if selected.len() == max_features {
                break;
            }
        }
    }
    selected
}

pub fn redetect_markerless_feature(
    frame_rgba: &[u8],
    width: u32,
    height: u32,
    predicted: [f32; 2],
    search_radius: u32,
) -> Option<[f32; 2]> {
    let candidates = detect_markerless_features(frame_rgba, width, height, 128, 2);
    let radius = search_radius.max(1) as f32;
    candidates
        .into_iter()
        .filter(|point| (point[0] - predicted[0]).hypot(point[1] - predicted[1]) <= radius)
        .min_by(|a, b| {
            (a[0] - predicted[0])
                .hypot(a[1] - predicted[1])
                .total_cmp(&(b[0] - predicted[0]).hypot(b[1] - predicted[1]))
        })
}

pub fn assign_features_to_humanoid(
    features: &[[f32; 2]],
    width: u32,
    height: u32,
) -> Vec<Option<usize>> {
    assign_features_to_humanoid_with_confidence(features, width, height, 1.0)
        .into_iter()
        .map(|value| value.map(|(index, _)| index))
        .collect()
}

pub fn assign_features_to_humanoid_with_confidence(
    features: &[[f32; 2]],
    width: u32,
    height: u32,
    minimum_confidence: f32,
) -> Vec<Option<(usize, f32)>> {
    if features.is_empty() || width == 0 || height == 0 {
        return vec![None; 17];
    }
    let minimum_confidence = if minimum_confidence.is_finite() {
        minimum_confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let valid = features
        .iter()
        .filter(|point| point.iter().all(|value| value.is_finite()))
        .collect::<Vec<_>>();
    if valid.is_empty() {
        return vec![None; 17];
    }
    let min_x = valid
        .iter()
        .map(|point| point[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = valid
        .iter()
        .map(|point| point[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = valid
        .iter()
        .map(|point| point[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = valid
        .iter()
        .map(|point| point[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let canonical = [
        (0.50, 0.62),
        (0.50, 0.48),
        (0.50, 0.38),
        (0.50, 0.28),
        (0.50, 0.16),
        (0.38, 0.40),
        (0.30, 0.52),
        (0.24, 0.65),
        (0.62, 0.40),
        (0.70, 0.52),
        (0.76, 0.65),
        (0.44, 0.64),
        (0.42, 0.82),
        (0.40, 0.98),
        (0.56, 0.64),
        (0.58, 0.82),
        (0.60, 0.98),
    ];
    let mut used = vec![false; features.len()];
    canonical
        .iter()
        .map(|&(x, y)| {
            features
                .iter()
                .enumerate()
                .filter(|(index, point)| {
                    !used[*index] && point[0].is_finite() && point[1].is_finite()
                })
                .map(|(index, point)| {
                    let da =
                        ((point[0] - min_x) / span_x - x).hypot((point[1] - min_y) / span_y - y);
                    (index, point, da)
                })
                .min_by(|(_, _, da), (_, _, db)| da.total_cmp(db))
                .and_then(|(index, _, distance)| {
                    let confidence = (1.0 - distance * 2.0).clamp(0.0, 1.0);
                    if confidence < minimum_confidence {
                        None
                    } else {
                        used[index] = true;
                        Some((index, confidence))
                    }
                })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessMotionSample {
    pub frame: u32,
    pub position: [f32; 2],
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessMotionTrack {
    pub samples: Vec<MarkerlessMotionSample>,
}

pub fn track_markerless_motion(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    seed_points: &[[f32; 2]],
    block_radius: i32,
    search_radius: i32,
) -> Vec<MarkerlessMotionTrack> {
    let Some(byte_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if width == 0 || height == 0 || frames.is_empty() || frames.iter().any(|f| f.len() != byte_len)
    {
        return Vec::new();
    }

    let (mut positions, mut tracks) = initialize_markerless_tracks(width, height, seed_points);
    if frames.len() == 1 || tracks.is_empty() {
        return tracks;
    }

    for frame in 1..frames.len() {
        advance_markerless_tracks(
            frames[frame - 1],
            frames[frame],
            width,
            height,
            frame as u32,
            block_radius,
            search_radius,
            &mut positions,
            &mut tracks,
        );
    }
    for track in &mut tracks {
        let _ = reject_markerless_motion_outliers(track, search_radius.max(1) as f32);
    }
    tracks
}

pub fn track_markerless_motion_stream<I>(
    frames: I,
    width: u32,
    height: u32,
    seed_points: &[[f32; 2]],
    block_radius: i32,
    search_radius: i32,
) -> Result<Vec<MarkerlessMotionTrack>, String>
where
    I: IntoIterator<Item = Result<Vec<u8>, String>>,
{
    let byte_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "frame dimensions overflow RGBA storage".to_string())?;
    if width == 0 || height == 0 {
        return Err("frame dimensions must be non-zero".to_string());
    }
    let mut frames = frames.into_iter();
    let mut previous = frames
        .next()
        .ok_or_else(|| "at least one frame is required".to_string())??;
    if previous.len() != byte_len {
        return Err("frame pixel data does not match its dimensions".to_string());
    }
    let (mut positions, mut tracks) = initialize_markerless_tracks(width, height, seed_points);

    for (index, current) in frames.enumerate() {
        let current = current?;
        if current.len() != byte_len {
            return Err(format!(
                "frame {} pixel data does not match {}x{} dimensions",
                index + 1,
                width,
                height
            ));
        }
        if !tracks.is_empty() {
            advance_markerless_tracks(
                &previous,
                &current,
                width,
                height,
                index.saturating_add(1) as u32,
                block_radius,
                search_radius,
                &mut positions,
                &mut tracks,
            );
        }
        previous = current;
    }
    for track in &mut tracks {
        let _ = reject_markerless_motion_outliers(track, search_radius.max(1) as f32);
    }
    Ok(tracks)
}

fn initialize_markerless_tracks(
    width: u32,
    height: u32,
    seed_points: &[[f32; 2]],
) -> (Vec<[f32; 2]>, Vec<MarkerlessMotionTrack>) {
    let mut positions = Vec::with_capacity(seed_points.len());
    let mut tracks = Vec::with_capacity(seed_points.len());
    for &point in seed_points {
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        let position = [
            point[0].clamp(0.0, width.saturating_sub(1) as f32),
            point[1].clamp(0.0, height.saturating_sub(1) as f32),
        ];
        positions.push(position);
        tracks.push(MarkerlessMotionTrack {
            samples: vec![MarkerlessMotionSample {
                frame: 0,
                position,
                confidence: 1.0,
            }],
        });
    }
    (positions, tracks)
}

#[allow(clippy::too_many_arguments)]
fn advance_markerless_tracks(
    previous: &[u8],
    current: &[u8],
    width: u32,
    height: u32,
    frame_index: u32,
    block_radius: i32,
    search_radius: i32,
    positions: &mut [[f32; 2]],
    tracks: &mut [MarkerlessMotionTrack],
) {
    let flow = compute_dense_optical_flow(
        previous,
        current,
        width,
        height,
        block_radius,
        search_radius,
    );
    let reverse_flow = compute_dense_optical_flow(
        current,
        previous,
        width,
        height,
        block_radius,
        search_radius,
    );
    for (index, position) in positions.iter_mut().enumerate() {
        let x = position[0]
            .round()
            .clamp(0.0, width.saturating_sub(1) as f32) as u32;
        let y = position[1]
            .round()
            .clamp(0.0, height.saturating_sub(1) as f32) as u32;
        let vector = flow.get(x, y);
        let magnitude = vector[0].hypot(vector[1]);
        let predicted = [
            (position[0] + vector[0]).clamp(0.0, width.saturating_sub(1) as f32),
            (position[1] + vector[1]).clamp(0.0, height.saturating_sub(1) as f32),
        ];
        let reverse = reverse_flow.get(predicted[0].round() as u32, predicted[1].round() as u32);
        let consistency = (vector[0] + reverse[0]).hypot(vector[1] + reverse[1]);
        let confidence = if magnitude.is_finite() && consistency.is_finite() {
            let motion_score = 1.0 - magnitude / (search_radius.max(1) as f32 + 1.0);
            let consistency_score = 1.0 - consistency / (search_radius.max(1) as f32 + 1.0);
            motion_score.min(consistency_score).clamp(0.0, 1.0)
        } else {
            0.0
        };
        *position = if confidence < 0.15 {
            redetect_markerless_feature(
                current,
                width,
                height,
                predicted,
                search_radius.max(1) as u32,
            )
            .unwrap_or(predicted)
        } else {
            predicted
        };
        if let Some(track) = tracks.get_mut(index) {
            track.samples.push(MarkerlessMotionSample {
                frame: frame_index,
                position: *position,
                confidence,
            });
        }
    }
}

pub fn track_markerless_auto(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    max_features: usize,
    feature_spacing: u32,
    block_radius: i32,
    search_radius: i32,
) -> Vec<MarkerlessMotionTrack> {
    let Some(first) = frames.first() else {
        return Vec::new();
    };
    let seeds = detect_markerless_features(first, width, height, max_features, feature_spacing);
    track_markerless_motion(frames, width, height, &seeds, block_radius, search_radius)
}
