use super::*;

impl TrackerEngine {
    pub fn apply_pose_as_tracker_points(
        layer: &mut Layer,
        pose: &crate::core::optical_flow_timewarp::MarkerlessPoseTrack,
        minimum_confidence: f32,
    ) -> usize {
        if pose.frames.is_empty() {
            return 0;
        }
        let threshold = if minimum_confidence.is_finite() {
            minimum_confidence.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let names = crate::core::optical_flow_timewarp::standard_humanoid_joint_names(
            pose.frames
                .first()
                .map(|frame| frame.joints.len())
                .unwrap_or(0),
        );
        let mut active_ids = std::collections::HashSet::new();
        let mut created = 0;
        for (joint, name) in names.into_iter().enumerate() {
            let keyframes = pose
                .frames
                .iter()
                .filter_map(|frame| {
                    let point = *frame.joints.get(joint)?;
                    (frame.confidence >= threshold && point.iter().all(|value| value.is_finite()))
                        .then(|| Keyframe::new(frame.frame, point, InterpolationType::Linear))
                })
                .collect::<Vec<_>>();
            if keyframes.is_empty() {
                continue;
            }
            let initial = keyframes[0].value;
            let id = format!("pose_{}", name);
            active_ids.insert(id.clone());
            let position = if keyframes.len() == 1 {
                Animatable::Constant(initial)
            } else {
                Animatable::Animated(keyframes)
            };
            if let Some(tracker) = layer.trackers.iter_mut().find(|tracker| tracker.id == id) {
                tracker.position = position;
            } else {
                let mut tracker = crate::core::timeline::TrackerPoint::new(
                    id,
                    format!("Pose {}", name.replace('_', " ")),
                    initial,
                );
                tracker.position = position;
                layer.trackers.push(tracker);
            }
            created += 1;
        }
        if created > 0 {
            layer.trackers.retain(|tracker| {
                !tracker.id.starts_with("pose_") || active_ids.contains(&tracker.id)
            });
        }
        created
    }

    pub fn apply_pose3d_as_tracker_points(
        layer: &mut Layer,
        pose: &crate::core::optical_flow_timewarp::MarkerlessPose3DTrack,
        camera: crate::core::optical_flow_timewarp::PoseCameraModel,
        rotation_degrees: [f32; 3],
        minimum_confidence: f32,
    ) -> usize {
        if pose.frames.is_empty() {
            return 0;
        }
        let threshold = if minimum_confidence.is_finite() {
            minimum_confidence.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let names = crate::core::optical_flow_timewarp::standard_humanoid_joint_names(
            pose.frames
                .iter()
                .map(|frame| frame.joints.len())
                .max()
                .unwrap_or(0),
        );
        let mut active_ids = std::collections::HashSet::new();
        let mut created = 0;
        for (joint, name) in names.into_iter().enumerate() {
            let keyframes = pose
                .frames
                .iter()
                .filter_map(|frame| {
                    let point = *frame.joints.get(joint)?;
                    let position =
                        crate::core::optical_flow_timewarp::project_pose3d_point_with_rotation(
                            point,
                            camera,
                            rotation_degrees,
                        )?;
                    (frame.confidence >= threshold)
                        .then(|| Keyframe::new(frame.frame, position, InterpolationType::Linear))
                })
                .collect::<Vec<_>>();
            if keyframes.is_empty() {
                continue;
            }
            let id = format!("pose3d_{}", name);
            active_ids.insert(id.clone());
            let initial = keyframes[0].value;
            let position = if keyframes.len() == 1 {
                Animatable::Constant(initial)
            } else {
                Animatable::Animated(keyframes)
            };
            if let Some(tracker) = layer.trackers.iter_mut().find(|tracker| tracker.id == id) {
                tracker.position = position;
            } else {
                let mut tracker = crate::core::timeline::TrackerPoint::new(
                    id,
                    format!("Pose 3D {}", name.replace('_', " ")),
                    initial,
                );
                tracker.position = position;
                layer.trackers.push(tracker);
            }
            created += 1;
        }
        if created > 0 {
            layer.trackers.retain(|tracker| {
                !tracker.id.starts_with("pose3d_") || active_ids.contains(&tracker.id)
            });
        }
        created
    }

    pub fn estimate_markerless_pose(
        layer: &Layer,
        start_frame: u32,
        end_frame: u32,
        max_features: usize,
        feature_spacing: u32,
        block_radius: i32,
        search_radius: i32,
    ) -> Option<crate::core::optical_flow_timewarp::MarkerlessPoseTrack> {
        if end_frame < start_frame {
            return None;
        }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let (current, _next, w, h) = Self::load_tracker_frames(layer, frame)?;
            width = w as u32;
            height = h as u32;
            frames.push(current);
        }
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        Some(
            crate::core::optical_flow_timewarp::estimate_markerless_pose(
                &refs,
                width,
                height,
                max_features,
                feature_spacing,
                block_radius,
                search_radius,
            ),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn analyze_markerless_tracks(
        layer: &mut Layer,
        start_frame: u32,
        end_frame: u32,
        max_features: usize,
        feature_spacing: u32,
        block_radius: i32,
        search_radius: i32,
        minimum_confidence: f32,
    ) -> usize {
        if end_frame < start_frame {
            return 0;
        }
        if layer.trackers.is_empty() {
            let Some((first, _, width, height)) = Self::load_tracker_frames(layer, start_frame)
            else {
                return 0;
            };
            let seeds = crate::core::optical_flow_timewarp::detect_markerless_features(
                &first,
                width as u32,
                height as u32,
                max_features,
                feature_spacing,
            );
            for (index, position) in seeds.into_iter().enumerate() {
                layer
                    .trackers
                    .push(crate::core::timeline::TrackerPoint::new(
                        format!("mocap_{index}"),
                        format!("Mocap Feature {}", index + 1),
                        position,
                    ));
            }
        }
        if layer.trackers.is_empty() {
            return 0;
        }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let Some((current, next, w, h)) = Self::load_tracker_frames(layer, frame) else {
                return 0;
            };
            width = w as u32;
            height = h as u32;
            frames.push(current);
            if frame == end_frame {
                frames.push(next);
            }
        }
        let seeds = layer
            .trackers
            .iter()
            .map(|tracker| tracker.position.evaluate(start_frame))
            .collect::<Vec<_>>();
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let tracks = crate::core::optical_flow_timewarp::track_markerless_motion(
            &refs,
            width,
            height,
            &seeds,
            block_radius,
            search_radius,
        );
        let mut total = 0;
        for (tracker, track) in layer.trackers.iter_mut().zip(tracks.iter()) {
            total += crate::core::optical_flow_timewarp::apply_markerless_track_to_tracker_point(
                tracker,
                track,
                minimum_confidence,
            );
        }
        total
    }

    pub fn analyze_markerless_track(
        layer: &mut Layer,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
        block_radius: i32,
        search_radius: i32,
        minimum_confidence: f32,
    ) -> usize {
        if tracker_idx >= layer.trackers.len() || end_frame < start_frame {
            return 0;
        }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let Some((current, next, w, h)) = Self::load_tracker_frames(layer, frame) else {
                return 0;
            };
            width = w as u32;
            height = h as u32;
            frames.push(current);
            if frame == end_frame {
                frames.push(next);
            }
        }
        let seed = layer.trackers[tracker_idx].position.evaluate(start_frame);
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let tracks = crate::core::optical_flow_timewarp::track_markerless_motion(
            &refs,
            width,
            height,
            &[seed],
            block_radius,
            search_radius,
        );
        let Some(track) = tracks.into_iter().next() else {
            return 0;
        };
        crate::core::optical_flow_timewarp::apply_markerless_track_to_tracker_point(
            &mut layer.trackers[tracker_idx],
            &track,
            minimum_confidence,
        )
    }
}
