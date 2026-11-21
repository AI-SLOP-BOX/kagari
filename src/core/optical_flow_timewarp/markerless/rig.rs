use super::*;

pub fn pose_quality(pose: &MarkerlessPoseTrack, frame_index: usize) -> (usize, f32) {
    let Some(frame) = pose.frames.get(frame_index) else {
        return (0, 0.0);
    };
    let valid = frame
        .joints
        .iter()
        .filter(|point| point.iter().all(|value| value.is_finite()))
        .count();
    let confidence = if frame.joints.is_empty() {
        0.0
    } else {
        frame.confidence.clamp(0.0, 1.0)
    };
    (valid, confidence)
}

pub fn filter_pose_frames_by_quality(
    pose: &mut MarkerlessPoseTrack,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedMarkerlessPoseTrack {
    pub joint_names: Vec<String>,
    pub bones: Vec<[usize; 2]>,
    pub frames: Vec<MarkerlessPoseFrame>,
    pub bone_lengths: Vec<f32>,
}

pub fn standard_humanoid_joint_names(count: usize) -> Vec<String> {
    const NAMES: &[&str] = &[
        "hip",
        "spine",
        "chest",
        "neck",
        "head",
        "left_shoulder",
        "left_elbow",
        "left_wrist",
        "right_shoulder",
        "right_elbow",
        "right_wrist",
        "left_hip",
        "left_knee",
        "left_ankle",
        "right_hip",
        "right_knee",
        "right_ankle",
    ];
    (0..count)
        .map(|index| {
            NAMES
                .get(index)
                .map(|name| (*name).to_string())
                .unwrap_or_else(|| format!("joint_{index:02}"))
        })
        .collect()
}

pub fn standard_humanoid_bones(joint_count: usize) -> Vec<[usize; 2]> {
    const BONES: &[(usize, usize)] = &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (2, 5),
        (5, 6),
        (6, 7),
        (2, 8),
        (8, 9),
        (9, 10),
        (0, 11),
        (11, 12),
        (12, 13),
        (0, 14),
        (14, 15),
        (15, 16),
    ];
    BONES
        .iter()
        .filter(|&&(parent, child)| parent < joint_count && child < joint_count)
        .map(|&(parent, child)| [parent, child])
        .collect()
}

pub fn normalize_joint_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

pub fn name_markerless_pose_track(
    pose: &MarkerlessPoseTrack,
    joint_names: &[String],
) -> NamedMarkerlessPoseTrack {
    let names = joint_names
        .iter()
        .take(
            pose.frames
                .first()
                .map(|frame| frame.joints.len())
                .unwrap_or(0),
        )
        .map(|name| normalize_joint_name(name))
        .collect::<Vec<_>>();
    let joint_count = names.len();
    let bones = pose
        .bones
        .iter()
        .copied()
        .filter(|bone| bone[0] < joint_count && bone[1] < joint_count)
        .collect();
    NamedMarkerlessPoseTrack {
        joint_names: names,
        bones,
        frames: pose.frames.clone(),
        bone_lengths: pose
            .bone_lengths
            .iter()
            .copied()
            .take(pose.bones.len())
            .collect(),
    }
}

pub fn name_and_connect_markerless_pose_track(
    pose: &MarkerlessPoseTrack,
) -> NamedMarkerlessPoseTrack {
    let count = pose
        .frames
        .first()
        .map(|frame| frame.joints.len())
        .unwrap_or(0);
    let names = standard_humanoid_joint_names(count);
    let mut named = name_markerless_pose_track(pose, &names);
    named.bones = standard_humanoid_bones(named.joint_names.len());
    named.bone_lengths = named
        .bones
        .iter()
        .map(|bone| {
            pose.bone_lengths
                .get(
                    pose.bones
                        .iter()
                        .position(|candidate| candidate[0] == bone[0] && candidate[1] == bone[1])
                        .unwrap_or(usize::MAX),
                )
                .copied()
                .unwrap_or_else(|| {
                    pose.frames
                        .first()
                        .map(|frame| {
                            (frame.joints[bone[0]][0] - frame.joints[bone[1]][0])
                                .hypot(frame.joints[bone[0]][1] - frame.joints[bone[1]][1])
                        })
                        .unwrap_or(0.0)
                })
        })
        .collect();
    named
}

pub fn repair_markerless_pose_track(pose: &mut MarkerlessPoseTrack, max_gap: usize) -> usize {
    if pose.frames.is_empty() || max_gap == 0 {
        return 0;
    }
    let joint_count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .min()
        .unwrap_or(0);
    let mut repaired = 0;
    for joint in 0..joint_count {
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
            let missing_frames = if start == 0 || end >= pose.frames.len() {
                usize::MAX
            } else {
                pose.frames[end]
                    .frame
                    .saturating_sub(pose.frames[start - 1].frame)
                    .saturating_sub(1) as usize
            };
            if start == 0 || end >= pose.frames.len() || missing_frames > max_gap {
                continue;
            }
            let before = pose.frames[start - 1].joints[joint];
            let after = pose.frames[end].joints[joint];
            let before_frame = pose.frames[start - 1].frame;
            let after_frame = pose.frames[end].frame;
            if after_frame <= before_frame {
                continue;
            }
            let span = (after_frame.saturating_sub(before_frame)) as f32;
            for offset in 0..(end - start) {
                let current_frame = pose.frames[start + offset].frame;
                if current_frame <= before_frame || current_frame >= after_frame {
                    continue;
                }
                let t = if span > 0.0 {
                    current_frame.saturating_sub(before_frame) as f32 / span
                } else {
                    0.0
                };
                pose.frames[start + offset].joints[joint] = [
                    before[0] + (after[0] - before[0]) * t,
                    before[1] + (after[1] - before[1]) * t,
                ];
                pose.frames[start + offset].confidence =
                    (pose.frames[start + offset].confidence * 0.5).clamp(0.0, 1.0);
                repaired += 1;
            }
        }
    }
    repaired
}

pub fn stabilize_markerless_pose_track(
    pose: &mut MarkerlessPoseTrack,
    max_gap: usize,
    radius: usize,
) -> usize {
    let repaired = repair_markerless_pose_track(pose, max_gap);
    if pose.frames.is_empty() || radius == 0 {
        return repaired;
    }
    let joint_count = pose
        .frames
        .iter()
        .map(|frame| frame.joints.len())
        .min()
        .unwrap_or(0);
    let original = pose
        .frames
        .iter()
        .map(|frame| frame.joints.clone())
        .collect::<Vec<_>>();
    let mut changed = repaired;
    for frame_index in 0..pose.frames.len() {
        let start = frame_index.saturating_sub(radius);
        let end = (frame_index + radius + 1).min(pose.frames.len());
        for joint in 0..joint_count {
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
            ];
            if pose.frames[frame_index].joints[joint] != smoothed {
                pose.frames[frame_index].joints[joint] = smoothed;
                pose.frames[frame_index].confidence =
                    (pose.frames[frame_index].confidence * 0.98).clamp(0.0, 1.0);
                changed += 1;
            }
        }
    }
    changed
}

pub fn reject_markerless_motion_outliers(
    track: &mut MarkerlessMotionTrack,
    max_step: f32,
) -> usize {
    if track.samples.len() < 2 || !max_step.is_finite() || max_step <= 0.0 {
        return 0;
    }
    let mut changed = 0;
    for index in 1..track.samples.len() {
        let previous = track.samples[index - 1].position;
        let current = track.samples[index].position;
        let distance = (current[0] - previous[0]).hypot(current[1] - previous[1]);
        if !distance.is_finite() || distance > max_step {
            track.samples[index].position = if distance.is_finite() {
                let scale = max_step / distance;
                [
                    previous[0] + (current[0] - previous[0]) * scale,
                    previous[1] + (current[1] - previous[1]) * scale,
                ]
            } else {
                previous
            };
            track.samples[index].confidence =
                (track.samples[index].confidence * 0.25).clamp(0.0, 1.0);
            changed += 1;
        }
    }
    changed
}

pub fn markerless_pose_to_csv(pose: &NamedMarkerlessPoseTrack) -> String {
    let mut output = String::from("frame,root_x,root_y,confidence");
    for name in &pose.joint_names {
        output.push_str(&format!(",{}_x,{}_y", name, name));
    }
    output.push('\n');
    for frame in &pose.frames {
        output.push_str(&format!(
            "{},{},{},{}",
            frame.frame, frame.root[0], frame.root[1], frame.confidence
        ));
        for joint in &frame.joints {
            output.push_str(&format!(",{},{}", joint[0], joint[1]));
        }
        output.push('\n');
    }
    output
}

pub fn markerless_pose_to_bvh(pose: &NamedMarkerlessPoseTrack, frame_time: f32) -> String {
    if pose.joint_names.is_empty() || pose.frames.is_empty() {
        return String::new();
    }
    let frame_time = if frame_time.is_finite() && frame_time > 0.0 {
        frame_time
    } else {
        1.0 / 30.0
    };
    let mut children = vec![Vec::<usize>::new(); pose.joint_names.len()];
    for bone in &pose.bones {
        if bone[0] < children.len() && bone[1] < children.len() && bone[0] != bone[1] {
            children[bone[0]].push(bone[1]);
        }
    }
    let first = &pose.frames[0];
    let mut parent = vec![None; pose.joint_names.len()];
    for bone in &pose.bones {
        if bone[1] < parent.len() && parent[bone[1]].is_none() {
            parent[bone[1]] = Some(bone[0]);
        }
    }
    let root = parent.iter().position(Option::is_none).unwrap_or(0);
    let mut output = String::from("HIERARCHY\n");
    struct HierarchyWriter<'a> {
        names: &'a [String],
        children: &'a [Vec<usize>],
        frame: &'a MarkerlessPoseFrame,
        visited: Vec<bool>,
    }

    impl HierarchyWriter<'_> {
        fn write_joint(
            &mut self,
            out: &mut String,
            index: usize,
            parent: Option<usize>,
            indent: usize,
            is_root: bool,
        ) {
            if index >= self.names.len() || self.visited[index] {
                return;
            }
            self.visited[index] = true;
            let pad = "  ".repeat(indent);
            out.push_str(&format!(
                "{}{} {}\n{}{{\n",
                pad,
                if is_root { "ROOT" } else { "JOINT" },
                self.names[index],
                pad
            ));
            let point = self.frame.joints.get(index).copied().unwrap_or([0.0, 0.0]);
            let parent_point = parent
                .and_then(|parent_index| self.frame.joints.get(parent_index).copied())
                .unwrap_or([0.0, 0.0]);
            let (ox, oy) = if is_root {
                (0.0, 0.0)
            } else {
                (point[0] - parent_point[0], point[1] - parent_point[1])
            };
            out.push_str(&format!("{}  OFFSET {:.6} {:.6} 0.000000\n", pad, ox, oy));
            if is_root {
                out.push_str(&format!(
                    "{}  CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation\n",
                    pad
                ));
            } else {
                out.push_str(&format!(
                    "{}  CHANNELS 3 Zrotation Xrotation Yrotation\n",
                    pad
                ));
            }
            let child_count = self.children[index].len();
            for child_index in 0..child_count {
                let child = self.children[index][child_index];
                self.write_joint(out, child, Some(index), indent + 1, false);
            }
            if self.children[index].is_empty() {
                out.push_str(&format!(
                    "{}  End Site\n{}  {{\n{}    OFFSET 0.000000 0.000000 0.000000\n{}  }}\n",
                    pad, pad, pad, pad
                ));
            }
            out.push_str(&format!("{}}}\n", pad));
        }
    }

    let mut writer = HierarchyWriter {
        names: &pose.joint_names,
        children: &children,
        frame: first,
        visited: vec![false; pose.joint_names.len()],
    };
    writer.write_joint(&mut output, root, None, 0, true);
    output.push_str(&format!(
        "MOTION\nFrames: {}\nFrame Time: {:.9}\n",
        pose.frames.len(),
        frame_time
    ));
    let initial = &first.joints;
    for (frame_index, frame) in pose.frames.iter().enumerate() {
        let root_point = frame.joints.get(root).copied().unwrap_or([0.0, 0.0]);
        output.push_str(&format!(
            "{:.6} {:.6} 0.000000",
            root_point[0], root_point[1]
        ));
        for (index, parent_index) in parent.iter().enumerate().take(pose.joint_names.len()) {
            if index == root {
                continue;
            }
            let angle = parent_index
                .and_then(|p| {
                    let a = initial.get(index)?;
                    let b = initial.get(p)?;
                    let ia = frame.joints.get(index)?;
                    let ib = frame.joints.get(p)?;
                    Some(
                        ((ia[1] - ib[1]).atan2(ia[0] - ib[0]) - (a[1] - b[1]).atan2(a[0] - b[0]))
                            .to_degrees(),
                    )
                })
                .unwrap_or(0.0);
            output.push_str(&format!(" {:.6} 0.000000 0.000000", angle));
        }
        if frame_index + 1 < pose.frames.len() {
            output.push('\n');
        }
    }
    output
}

pub fn build_markerless_pose_track(
    joint_tracks: &[MarkerlessMotionTrack],
    bones: &[(usize, usize)],
) -> MarkerlessPoseTrack {
    if joint_tracks.is_empty() {
        return MarkerlessPoseTrack {
            frames: Vec::new(),
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
    }
    let frame_count = joint_tracks
        .iter()
        .map(|track| track.samples.len())
        .min()
        .unwrap_or(0);
    let mut frames = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        let joints = joint_tracks
            .iter()
            .map(|track| track.samples[frame_index].position)
            .collect::<Vec<_>>();
        let root = joints.iter().fold([0.0, 0.0], |sum, point| {
            [sum[0] + point[0], sum[1] + point[1]]
        });
        let root = [root[0] / joints.len() as f32, root[1] / joints.len() as f32];
        let confidence = joint_tracks
            .iter()
            .map(|track| track.samples[frame_index].confidence)
            .sum::<f32>()
            / joint_tracks.len() as f32;
        frames.push(MarkerlessPoseFrame {
            frame: joint_tracks[0].samples[frame_index].frame,
            joints,
            root,
            confidence: confidence.clamp(0.0, 1.0),
        });
    }
    let bone_lengths = bones
        .iter()
        .map(|&(a, b)| {
            let Some(first) = frames.first() else {
                return 0.0;
            };
            match (first.joints.get(a), first.joints.get(b)) {
                (Some(a), Some(b))
                    if a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()) =>
                {
                    (a[0] - b[0]).hypot(a[1] - b[1])
                }
                _ => 0.0,
            }
        })
        .collect();
    MarkerlessPoseTrack {
        frames,
        bones: bones.iter().map(|&(a, b)| [a, b]).collect(),
        bone_lengths,
    }
}

pub fn markerless_pose_to_json(pose: &MarkerlessPoseTrack) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pose)
}
