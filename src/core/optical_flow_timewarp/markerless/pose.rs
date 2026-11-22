use super::*;

pub fn estimate_markerless_pose(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    max_features: usize,
    feature_spacing: u32,
    block_radius: i32,
    search_radius: i32,
) -> MarkerlessPoseTrack {
    let first = frames.first().copied().unwrap_or(&[]);
    let seeds = detect_markerless_features(first, width, height, max_features, feature_spacing);
    let tracks =
        track_markerless_motion(frames, width, height, &seeds, block_radius, search_radius);
    estimate_markerless_pose_from_tracks(&seeds, &tracks, width, height)
}

pub fn estimate_markerless_pose_from_tracks(
    seeds: &[[f32; 2]],
    tracks: &[MarkerlessMotionTrack],
    width: u32,
    height: u32,
) -> MarkerlessPoseTrack {
    let assignments = assign_features_to_humanoid(seeds, width, height);
    let frame_count = tracks
        .iter()
        .map(|track| track.samples.len())
        .min()
        .unwrap_or(0);
    let mut pose_frames = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        let mut joints = vec![[f32::NAN, f32::NAN]; 17];
        let mut confidence_sum = 0.0;
        let mut confidence_count = 0;
        for (joint, assignment) in assignments.iter().enumerate() {
            if let Some(track_index) = assignment {
                if let Some(sample) = tracks
                    .get(*track_index)
                    .and_then(|track| track.samples.get(frame_index))
                {
                    joints[joint] = sample.position;
                    confidence_sum += sample.confidence;
                    confidence_count += 1;
                }
            }
        }
        let valid = joints
            .iter()
            .filter(|point| point.iter().all(|value| value.is_finite()))
            .collect::<Vec<_>>();
        let root = if valid.is_empty() {
            [0.0, 0.0]
        } else {
            [
                valid.iter().map(|point| point[0]).sum::<f32>() / valid.len() as f32,
                valid.iter().map(|point| point[1]).sum::<f32>() / valid.len() as f32,
            ]
        };
        pose_frames.push(MarkerlessPoseFrame {
            frame: tracks
                .first()
                .and_then(|track| track.samples.get(frame_index))
                .map(|sample| sample.frame)
                .unwrap_or(frame_index as u32),
            joints,
            root,
            confidence: if confidence_count == 0 {
                0.0
            } else {
                (confidence_sum / confidence_count as f32).clamp(0.0, 1.0)
            },
        });
    }
    let mut pose = MarkerlessPoseTrack {
        frames: pose_frames,
        bones: standard_humanoid_bones(17),
        bone_lengths: Vec::new(),
    };
    pose.bone_lengths = pose
        .bones
        .iter()
        .map(|bone| {
            pose.frames
                .first()
                .and_then(|frame| {
                    let a = frame.joints.get(bone[0])?;
                    let b = frame.joints.get(bone[1])?;
                    if a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()) {
                        Some((a[0] - b[0]).hypot(a[1] - b[1]))
                    } else {
                        Some(0.0)
                    }
                })
                .unwrap_or(0.0)
        })
        .collect();
    let _ = stabilize_markerless_pose_track(&mut pose, 2, 1);
    for frame in &mut pose.frames {
        let valid = frame
            .joints
            .iter()
            .filter(|point| point.iter().all(|value| value.is_finite()))
            .collect::<Vec<_>>();
        frame.root = if valid.is_empty() {
            [0.0, 0.0]
        } else {
            [
                valid.iter().map(|point| point[0]).sum::<f32>() / valid.len() as f32,
                valid.iter().map(|point| point[1]).sum::<f32>() / valid.len() as f32,
            ]
        };
    }
    pose
}

pub fn markerless_track_keyframes(
    track: &MarkerlessMotionTrack,
) -> crate::core::property::Animatable<[f32; 2]> {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    let keyframes = track
        .samples
        .iter()
        .filter(|sample| sample.position.iter().all(|value| value.is_finite()))
        .map(|sample| Keyframe::new(sample.frame, sample.position, InterpolationType::Linear))
        .collect::<Vec<_>>();
    match keyframes.as_slice() {
        [] => crate::core::property::Animatable::Constant([0.0, 0.0]),
        [keyframe] => crate::core::property::Animatable::Constant(keyframe.value),
        _ => crate::core::property::Animatable::Animated(keyframes),
    }
}

pub fn apply_markerless_track_to_tracker_point(
    tracker: &mut crate::core::timeline::TrackerPoint,
    track: &MarkerlessMotionTrack,
    minimum_confidence: f32,
) -> usize {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    let threshold = if minimum_confidence.is_finite() {
        minimum_confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let keyframes = track
        .samples
        .iter()
        .filter(|sample| {
            sample.confidence >= threshold
                && sample.confidence.is_finite()
                && sample.position.iter().all(|value| value.is_finite())
        })
        .map(|sample| Keyframe::new(sample.frame, sample.position, InterpolationType::Linear))
        .collect::<Vec<_>>();
    let count = keyframes.len();
    if count > 0 {
        tracker.position = crate::core::property::Animatable::Animated(keyframes);
    }
    count
}

pub fn smooth_markerless_track(
    track: &MarkerlessMotionTrack,
    radius: usize,
) -> MarkerlessMotionTrack {
    if track.samples.len() < 2 || radius == 0 {
        return track.clone();
    }
    let mut samples = Vec::with_capacity(track.samples.len());
    for (index, sample) in track.samples.iter().enumerate() {
        let start = index.saturating_sub(radius);
        let end = (index + radius + 1).min(track.samples.len());
        let mut weighted_position = [0.0f32; 2];
        let mut weight_sum = 0.0f32;
        let mut confidence_sum = 0.0f32;
        for neighbor in &track.samples[start..end] {
            let weight = neighbor.confidence.clamp(0.0, 1.0).max(0.01);
            if neighbor.position.iter().all(|value| value.is_finite()) {
                weighted_position[0] += neighbor.position[0] * weight;
                weighted_position[1] += neighbor.position[1] * weight;
                confidence_sum += neighbor.confidence.clamp(0.0, 1.0) * weight;
                weight_sum += weight;
            }
        }
        let position = if weight_sum > 0.0 {
            [
                weighted_position[0] / weight_sum,
                weighted_position[1] / weight_sum,
            ]
        } else {
            sample.position
        };
        samples.push(MarkerlessMotionSample {
            frame: sample.frame,
            position,
            confidence: if weight_sum > 0.0 {
                (confidence_sum / weight_sum).clamp(0.0, 1.0)
            } else {
                0.0
            },
        });
    }
    MarkerlessMotionTrack { samples }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPoseFrame {
    pub frame: u32,
    pub joints: Vec<[f32; 2]>,
    pub root: [f32; 2],
    pub confidence: f32,
}

pub fn pose_joint_velocities(pose: &MarkerlessPoseTrack, frame_index: usize) -> Vec<[f32; 2]> {
    let Some(current) = pose.frames.get(frame_index) else {
        return Vec::new();
    };
    let previous = frame_index.checked_sub(1).and_then(|i| pose.frames.get(i));
    current
        .joints
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let Some(previous_point) = previous.and_then(|frame| frame.joints.get(index)) else {
                return [0.0, 0.0];
            };
            if point.iter().all(|value| value.is_finite())
                && previous_point.iter().all(|value| value.is_finite())
            {
                [point[0] - previous_point[0], point[1] - previous_point[1]]
            } else {
                [0.0, 0.0]
            }
        })
        .collect()
}

pub fn pose_root_acceleration(pose: &MarkerlessPoseTrack, frame_index: usize) -> [f32; 2] {
    if frame_index < 2 || frame_index >= pose.frames.len() {
        return [0.0, 0.0];
    }
    let a = pose.frames[frame_index - 2].root;
    let b = pose.frames[frame_index - 1].root;
    let c = pose.frames[frame_index].root;
    if [a, b, c].iter().flatten().all(|value| value.is_finite()) {
        [c[0] - 2.0 * b[0] + a[0], c[1] - 2.0 * b[1] + a[1]]
    } else {
        [0.0, 0.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MarkerlessMotionEventKind {
    Peak,
    Stop,
    DirectionChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessMotionEvent {
    pub frame: u32,
    pub joint: usize,
    pub kind: MarkerlessMotionEventKind,
    pub magnitude: f32,
}

pub fn detect_markerless_motion_events(
    pose: &MarkerlessPoseTrack,
    speed_threshold: f32,
    acceleration_threshold: f32,
) -> Vec<MarkerlessMotionEvent> {
    let speed_threshold = if speed_threshold.is_finite() && speed_threshold >= 0.0 {
        speed_threshold
    } else {
        0.0
    };
    let acceleration_threshold =
        if acceleration_threshold.is_finite() && acceleration_threshold >= 0.0 {
            acceleration_threshold
        } else {
            0.0
        };
    let mut events = Vec::new();
    for frame_index in 1..pose.frames.len() {
        let velocities = pose_joint_velocities(pose, frame_index);
        for (joint, velocity) in velocities.iter().enumerate() {
            let speed = velocity[0].hypot(velocity[1]);
            if speed >= speed_threshold && speed.is_finite() {
                events.push(MarkerlessMotionEvent {
                    frame: pose.frames[frame_index].frame,
                    joint,
                    kind: MarkerlessMotionEventKind::Peak,
                    magnitude: speed,
                });
            }
            if frame_index > 1 {
                let previous = pose_joint_velocities(pose, frame_index - 1)
                    .get(joint)
                    .copied()
                    .unwrap_or([0.0, 0.0]);
                let acceleration = [velocity[0] - previous[0], velocity[1] - previous[1]];
                let magnitude = acceleration[0].hypot(acceleration[1]);
                if magnitude >= acceleration_threshold && magnitude.is_finite() {
                    let dot = velocity[0] * previous[0] + velocity[1] * previous[1];
                    let kind = if dot < 0.0 {
                        MarkerlessMotionEventKind::DirectionChange
                    } else {
                        MarkerlessMotionEventKind::Peak
                    };
                    events.push(MarkerlessMotionEvent {
                        frame: pose.frames[frame_index].frame,
                        joint,
                        kind,
                        magnitude,
                    });
                }
            }
            if speed <= speed_threshold && frame_index > 1 {
                events.push(MarkerlessMotionEvent {
                    frame: pose.frames[frame_index].frame,
                    joint,
                    kind: MarkerlessMotionEventKind::Stop,
                    magnitude: speed,
                });
            }
        }
    }
    events
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPoseTrack {
    pub frames: Vec<MarkerlessPoseFrame>,
    pub bones: Vec<[usize; 2]>,
    pub bone_lengths: Vec<f32>,
}

pub trait PoseInferenceBackend {
    fn infer_joints(&mut self, frame_rgba: &[u8], width: u32, height: u32) -> Vec<([f32; 2], f32)>;
}

pub fn rgba_to_normalized_rgb(frame_rgba: &[u8], width: u32, height: u32) -> Option<Vec<f32>> {
    let byte_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || frame_rgba.len() != byte_len {
        return None;
    }
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for pixel in frame_rgba.as_chunks::<4>().0 {
        rgb.extend([
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        ]);
    }
    Some(rgb)
}

pub trait NormalizedPoseInferenceBackend {
    fn infer_normalized_rgb(
        &mut self,
        rgb: &[f32],
        width: u32,
        height: u32,
    ) -> Vec<([f32; 2], f32)>;
}

mod pose3d;
pub use pose3d::*;
