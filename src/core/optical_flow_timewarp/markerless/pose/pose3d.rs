use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPose3DFrame {
    pub frame: u32,
    pub joints: Vec<[f32; 3]>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPose3DTrack {
    pub frames: Vec<MarkerlessPose3DFrame>,
    pub bones: Vec<[usize; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PoseCameraModel {
    pub focal_length: f32,
    pub principal_point: [f32; 2],
    pub position: [f32; 3],
}

pub fn project_pose3d_point(point: [f32; 3], camera: PoseCameraModel) -> Option<[f32; 2]> {
    project_pose3d_point_with_rotation(point, camera, [0.0, 0.0, 0.0])
}

pub fn project_pose3d_point_with_rotation(
    point: [f32; 3],
    camera: PoseCameraModel,
    rotation_degrees: [f32; 3],
) -> Option<[f32; 2]> {
    if !point.iter().all(|value| value.is_finite())
        || !camera.focal_length.is_finite()
        || camera.focal_length <= 0.0
        || !camera.principal_point.iter().all(|value| value.is_finite())
        || !camera.position.iter().all(|value| value.is_finite())
        || !rotation_degrees.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let mut relative = [
        point[0] - camera.position[0],
        point[1] - camera.position[1],
        point[2] - camera.position[2],
    ];
    let radians = rotation_degrees.map(f32::to_radians);
    let (sin_x, cos_x) = radians[0].sin_cos();
    let (sin_y, cos_y) = radians[1].sin_cos();
    let (sin_z, cos_z) = radians[2].sin_cos();
    let y = relative[1] * cos_x - relative[2] * sin_x;
    let z = relative[1] * sin_x + relative[2] * cos_x;
    relative[1] = y;
    relative[2] = z;
    let x = relative[0] * cos_y + relative[2] * sin_y;
    let z = -relative[0] * sin_y + relative[2] * cos_y;
    relative[0] = x;
    relative[2] = z;
    let x = relative[0] * cos_z - relative[1] * sin_z;
    let y = relative[0] * sin_z + relative[1] * cos_z;
    relative[0] = x;
    relative[1] = y;
    let depth = relative[2];
    if !depth.is_finite() || depth <= f32::EPSILON {
        return None;
    }
    let projected = [
        camera.principal_point[0] + camera.focal_length * relative[0] / depth,
        camera.principal_point[1] + camera.focal_length * relative[1] / depth,
    ];
    projected
        .iter()
        .all(|value| value.is_finite())
        .then_some(projected)
}

pub fn project_pose3d_track_to_motion_tracks(
    pose: &MarkerlessPose3DTrack,
    camera: PoseCameraModel,
    rotation_degrees: [f32; 3],
) -> Vec<MarkerlessMotionTrack> {
    let joint_count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .max()
        .unwrap_or(0);
    (0..joint_count)
        .map(|joint| MarkerlessMotionTrack {
            samples: pose
                .frames
                .iter()
                .filter_map(|frame| {
                    let point = *frame.joints.get(joint)?;
                    let position =
                        project_pose3d_point_with_rotation(point, camera, rotation_degrees)?;
                    Some(MarkerlessMotionSample {
                        frame: frame.frame,
                        position,
                        confidence: frame.confidence,
                    })
                })
                .collect(),
        })
        .collect()
}

pub fn markerless_pose3d_to_csv(pose: &MarkerlessPose3DTrack) -> String {
    let count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .max()
        .unwrap_or(0);
    let names = standard_humanoid_joint_names(count);
    let mut output = String::from("frame,confidence");
    for name in &names {
        output.push_str(&format!(",{}_x,{}_y,{}_z", name, name, name));
    }
    output.push('\n');
    for frame in &pose.frames {
        output.push_str(&format!("{},{}", frame.frame, frame.confidence));
        for joint in 0..count {
            let point = frame.joints.get(joint).copied().unwrap_or([f32::NAN; 3]);
            output.push_str(&format!(",{},{},{}", point[0], point[1], point[2]));
        }
        output.push('\n');
    }
    output
}

pub fn markerless_pose3d_to_json(
    pose: &MarkerlessPose3DTrack,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pose)
}

pub fn stabilize_markerless_pose3d_track(
    pose: &mut MarkerlessPose3DTrack,
    max_gap: usize,
    radius: usize,
) -> usize {
    if pose.frames.is_empty() || max_gap == 0 {
        return 0;
    }
    let count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .min()
        .unwrap_or(0);
    let mut changed = 0;
    for joint in 0..count {
        let mut index = 0;
        while index < pose.frames.len() {
            if pose.frames[index].joints[joint]
                .iter()
                .all(|value| value.is_finite())
            {
                index += 1;
                continue;
            }
            let start = index;
            while index < pose.frames.len()
                && !pose.frames[index].joints[joint]
                    .iter()
                    .all(|value| value.is_finite())
            {
                index += 1;
            }
            let end = index;
            if start == 0 || end >= pose.frames.len() {
                continue;
            }
            let gap = pose.frames[end]
                .frame
                .saturating_sub(pose.frames[start - 1].frame)
                .saturating_sub(1) as usize;
            if gap > max_gap || pose.frames[end].frame <= pose.frames[start - 1].frame {
                continue;
            }
            let before = pose.frames[start - 1].joints[joint];
            let after = pose.frames[end].joints[joint];
            let before_frame = pose.frames[start - 1].frame;
            let span = (pose.frames[end].frame - before_frame) as f32;
            for item in &mut pose.frames[start..end] {
                let t = (item.frame - before_frame) as f32 / span;
                item.joints[joint] = [
                    before[0] + (after[0] - before[0]) * t,
                    before[1] + (after[1] - before[1]) * t,
                    before[2] + (after[2] - before[2]) * t,
                ];
                item.confidence = (item.confidence * 0.5).clamp(0.0, 1.0);
                changed += 1;
            }
        }
    }
    if radius == 0 {
        return changed;
    }
    let original = pose
        .frames
        .iter()
        .map(|frame| frame.joints.clone())
        .collect::<Vec<_>>();
    for frame_index in 0..pose.frames.len() {
        let start = frame_index.saturating_sub(radius);
        let end = (frame_index + radius + 1).min(pose.frames.len());
        for joint in 0..count {
            let values = (start..end)
                .filter_map(|index| {
                    let point = *original[index].get(joint)?;
                    point.iter().all(|value| value.is_finite()).then_some(point)
                })
                .collect::<Vec<_>>();
            if values.is_empty() {
                continue;
            }
            let smoothed = [
                values.iter().map(|point| point[0]).sum::<f32>() / values.len() as f32,
                values.iter().map(|point| point[1]).sum::<f32>() / values.len() as f32,
                values.iter().map(|point| point[2]).sum::<f32>() / values.len() as f32,
            ];
            if pose.frames[frame_index].joints[joint] != smoothed {
                pose.frames[frame_index].joints[joint] = smoothed;
                changed += 1;
            }
        }
    }
    changed
}

pub fn pose3d_quality(pose: &MarkerlessPose3DTrack, frame_index: usize) -> (usize, f32) {
    let Some(frame) = pose.frames.get(frame_index) else {
        return (0, 0.0);
    };
    let valid = frame
        .joints
        .iter()
        .filter(|point| point.iter().all(|value| value.is_finite()))
        .count();
    (
        valid,
        if frame.confidence.is_finite() {
            frame.confidence.clamp(0.0, 1.0)
        } else {
            0.0
        },
    )
}

pub fn filter_pose3d_frames_by_quality(
    pose: &mut MarkerlessPose3DTrack,
    minimum_joints: usize,
    minimum_confidence: f32,
) -> usize {
    let threshold = if minimum_confidence.is_finite() {
        minimum_confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let before = pose.frames.len();
    pose.frames.retain(|frame| {
        frame
            .joints
            .iter()
            .filter(|point| point.iter().all(|value| value.is_finite()))
            .count()
            >= minimum_joints
            && frame.confidence.is_finite()
            && frame.confidence >= threshold
    });
    before - pose.frames.len()
}

pub fn normalize_pose3d_bones(pose: &mut MarkerlessPose3DTrack) -> usize {
    let joint_count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .max()
        .unwrap_or(0);
    let before = pose.bones.len();
    pose.bones
        .retain(|bone| bone[0] < joint_count && bone[1] < joint_count && bone[0] != bone[1]);
    pose.bones.sort_unstable();
    pose.bones.dedup();
    before - pose.bones.len()
}

pub trait Pose3DInferenceBackend {
    fn infer_joints_3d(&mut self, rgb: &[f32], width: u32, height: u32) -> Vec<([f32; 3], f32)>;
}

pub fn estimate_pose_3d_with_backend<B: Pose3DInferenceBackend>(
    backend: &mut B,
    frames: &[&[u8]],
    width: u32,
    height: u32,
) -> MarkerlessPose3DTrack {
    let normalized = frames
        .iter()
        .map(|frame| rgba_to_normalized_rgb(frame, width, height))
        .collect::<Option<Vec<_>>>();
    let Some(normalized) = normalized else {
        return MarkerlessPose3DTrack {
            frames: Vec::new(),
            bones: Vec::new(),
        };
    };
    let mut outputs = Vec::with_capacity(normalized.len());
    for (frame_index, rgb) in normalized.iter().enumerate() {
        let joints = backend
            .infer_joints_3d(rgb, width, height)
            .into_iter()
            .take(17)
            .map(|(point, confidence)| {
                if point.iter().all(|value| value.is_finite()) && confidence.is_finite() {
                    (point, confidence.clamp(0.0, 1.0))
                } else {
                    ([f32::NAN; 3], 0.0)
                }
            })
            .collect::<Vec<_>>();
        let confidence = if joints.is_empty() {
            0.0
        } else {
            joints.iter().map(|(_, value)| *value).sum::<f32>() / joints.len() as f32
        };
        outputs.push(MarkerlessPose3DFrame {
            frame: frame_index as u32,
            joints: joints.into_iter().map(|(point, _)| point).collect(),
            confidence,
        });
    }
    let count = outputs
        .iter()
        .map(|frame| frame.joints.len())
        .max()
        .unwrap_or(0);
    for frame in &mut outputs {
        frame.joints.resize(count, [f32::NAN; 3]);
    }
    let mut pose = MarkerlessPose3DTrack {
        frames: outputs,
        bones: standard_humanoid_bones(count),
    };
    let _ = stabilize_markerless_pose3d_track(&mut pose, 2, 1);
    pose
}

pub fn estimate_pose_with_normalized_backend<B: NormalizedPoseInferenceBackend>(
    backend: &mut B,
    frames: &[&[u8]],
    width: u32,
    height: u32,
) -> MarkerlessPoseTrack {
    let normalized = frames
        .iter()
        .map(|frame| rgba_to_normalized_rgb(frame, width, height))
        .collect::<Option<Vec<_>>>();
    let Some(normalized) = normalized else {
        return MarkerlessPoseTrack {
            frames: Vec::new(),
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
    };
    let mut raw = Vec::with_capacity(normalized.len());
    for rgb in &normalized {
        raw.push(backend.infer_normalized_rgb(rgb, width, height));
    }
    let raw_refs = raw.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let mut pose_backend = RawPoseResultsBackend {
        results: &raw_refs,
        index: 0,
    };
    estimate_pose_with_backend(&mut pose_backend, frames, width, height)
}

struct RawPoseResultsBackend<'a> {
    results: &'a [&'a [([f32; 2], f32)]],
    index: usize,
}

impl PoseInferenceBackend for RawPoseResultsBackend<'_> {
    fn infer_joints(
        &mut self,
        _frame_rgba: &[u8],
        _width: u32,
        _height: u32,
    ) -> Vec<([f32; 2], f32)> {
        let result = self.results.get(self.index).copied().unwrap_or(&[]);
        self.index = self.index.saturating_add(1);
        result.to_vec()
    }
}

pub fn estimate_pose_with_backend<B: PoseInferenceBackend>(
    backend: &mut B,
    frames: &[&[u8]],
    width: u32,
    height: u32,
) -> MarkerlessPoseTrack {
    let Some(_first) = frames.first().copied() else {
        return MarkerlessPoseTrack {
            frames: Vec::new(),
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
    };
    let Some(byte_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return MarkerlessPoseTrack {
            frames: Vec::new(),
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
    };
    if width == 0 || height == 0 || frames.iter().any(|frame| frame.len() != byte_len) {
        return MarkerlessPoseTrack {
            frames: Vec::new(),
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
    }
    let mut all_joints = Vec::with_capacity(frames.len());
    for frame in frames {
        let joints = backend
            .infer_joints(frame, width, height)
            .into_iter()
            .take(17)
            .map(|(point, confidence)| {
                if point.iter().all(|value| value.is_finite()) && confidence.is_finite() {
                    (point, confidence.clamp(0.0, 1.0))
                } else {
                    ([f32::NAN, f32::NAN], 0.0)
                }
            })
            .collect::<Vec<_>>();
        all_joints.push(joints);
    }
    let count = all_joints.iter().map(Vec::len).max().unwrap_or(0);
    let mut pose_frames = Vec::with_capacity(all_joints.len());
    for (frame_index, joints) in all_joints.iter().enumerate() {
        let joints = (0..count)
            .map(|index| {
                joints
                    .get(index)
                    .map(|(point, _)| *point)
                    .unwrap_or([f32::NAN, f32::NAN])
            })
            .collect::<Vec<_>>();
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
        let confidence = if count == 0 {
            0.0
        } else {
            (0..count)
                .map(|index| {
                    all_joints[frame_index]
                        .get(index)
                        .map(|(_, confidence)| *confidence)
                        .unwrap_or(0.0)
                })
                .sum::<f32>()
                / count as f32
        };
        pose_frames.push(MarkerlessPoseFrame {
            frame: frame_index as u32,
            joints,
            root,
            confidence,
        });
    }
    let mut pose = MarkerlessPoseTrack {
        frames: pose_frames,
        bones: standard_humanoid_bones(count),
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
                    if a.iter().all(|value| value.is_finite())
                        && b.iter().all(|value| value.is_finite())
                    {
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
