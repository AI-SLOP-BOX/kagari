use super::*;

impl TrackerEngine {
    /// Track a feature forward from `current_frame` to `current_frame + 1`.
    /// Generates keyframe data for the tracker point.
    pub fn track_next_frame(
        layer: &Layer,
        fps: u32,
        tracker_idx: usize,
        current_frame: u32,
    ) -> Option<[f32; 2]> {
        // Real image-based tracking when pixel sources are available
        if let Some((curr_img, next_img, img_w, img_h)) =
            Self::load_tracker_frames(layer, current_frame)
        {
            return Self::track_next_frame_pixels(
                layer,
                tracker_idx,
                current_frame,
                &curr_img,
                &next_img,
                img_w,
                img_h,
            );
        }
        // Fallback: transform-velocity extrapolation (Null/Shape/etc.)
        Self::track_next_frame_synthetic(layer, fps, tracker_idx, current_frame)
    }

    /// Loads the RGBA buffers for `frame` and `frame + 1` from the layer's media.
    /// Video layers use their extracted sequence; image layers reuse the same file.
    pub(super) fn load_tracker_frames(
        layer: &Layer,
        frame: u32,
    ) -> Option<(Vec<u8>, Vec<u8>, usize, usize)> {
        use crate::core::image_cache::with_image_cache;

        let (dir_a, dir_b): (Option<String>, Option<String>) = match &layer.layer_type {
            crate::core::timeline::LayerType::Video {
                frames_dir,
                frame_count,
                ..
            } => {
                let seq_a = frame.min(frame_count.saturating_sub(1));
                let seq_b = (frame + 1).min(frame_count.saturating_sub(1));
                (
                    Some(
                        crate::core::video_import::frame_path_in_dir(frames_dir, seq_a)
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    Some(
                        crate::core::video_import::frame_path_in_dir(frames_dir, seq_b)
                            .to_string_lossy()
                            .into_owned(),
                    ),
                )
            }
            crate::core::timeline::LayerType::Image { path } => {
                (Some(path.clone()), Some(path.clone()))
            }
            _ => (None, None),
        };

        with_image_cache(|cache| {
            let a = cache.load_image(&dir_a?)?;
            let w = a.width as usize;
            let h = a.height as usize;
            let buf_a = a.pixels.clone();
            let b = cache.load_image(&dir_b?)?;
            if b.width as usize != w || b.height as usize != h {
                return None;
            }
            Some((buf_a, b.pixels.clone(), w, h))
        })
    }

    /// Real SAD block matching over decoded frames, with subpixel refinement.
    pub fn track_next_frame_pixels(
        layer: &Layer,
        tracker_idx: usize,
        current_frame: u32,
        curr_img: &[u8],
        next_img: &[u8],
        img_w: usize,
        img_h: usize,
    ) -> Option<[f32; 2]> {
        if tracker_idx >= layer.trackers.len() {
            return None;
        }
        let tracker = &layer.trackers[tracker_idx];
        let pos = tracker.position.evaluate(current_frame);

        let feature = (tracker.feature_size.max(8.0) as usize) & !1; // even
        let half = feature / 2;
        let cx = pos[0] as i32;
        let cy = pos[1] as i32;
        if (cx - half as i32) < 0 || (cy - half as i32) < 0 {
            return Some(pos);
        }

        // Reference patch: prefer the tracker's persistent template when one
        // of matching size is stored (appearance-locked tracking), otherwise
        // slice the current frame around the position as before.
        let template_len = feature * feature * 4;
        let ref_patch: Vec<u8> = match tracker
            .reference_pattern
            .as_deref()
            .filter(|t| t.len() == template_len)
        {
            // Stored patterns are normalised 0..1 floats — quantise to u8.
            Some(stored) => stored
                .iter()
                .map(|&f| (f.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect(),
            None => {
                let mut patch = vec![0u8; template_len];
                for py in 0..feature {
                    for px in 0..feature {
                        let sx = cx - half as i32 + px as i32;
                        let sy = cy - half as i32 + py as i32;
                        if sx < 0 || sy < 0 || sx >= img_w as i32 || sy >= img_h as i32 {
                            continue;
                        }
                        let src = ((sy * img_w as i32 + sx) * 4) as usize;
                        let dst = (py * feature + px) * 4;
                        patch[dst..dst + 4].copy_from_slice(&curr_img[src..src + 4]);
                    }
                }
                patch
            }
        };

        let search_radius = (tracker.search_size as i32).clamp(2, 64);
        let [bx, by] = compute_sad_match(
            &ref_patch,
            feature,
            feature,
            next_img,
            img_w,
            img_h,
            cx,
            cy,
            search_radius,
        );

        // Subpixel refinement along each axis
        let sad_at = |x: i32, y: i32| -> f32 {
            let mut shifted = [bx, by];
            shifted[0] = x as f32;
            shifted[1] = y as f32;
            let _ = shifted;
            // Recompute SAD at integer offsets around best
            let mut total = 0.0f32;
            for py in 0..feature {
                for px in 0..feature {
                    let tx = x - half as i32 + px as i32;
                    let ty = y - half as i32 + py as i32;
                    if tx < 0 || ty < 0 || tx >= img_w as i32 || ty >= img_h as i32 {
                        continue;
                    }
                    let ref_idx = (py * feature + px) * 4;
                    let tgt_idx = ((ty * img_w as i32 + tx) * 4) as usize;
                    if ref_idx + 3 < ref_patch.len() && tgt_idx + 3 < next_img.len() {
                        total += (ref_patch[ref_idx] as f32 - next_img[tgt_idx] as f32).abs()
                            + (ref_patch[ref_idx + 1] as f32 - next_img[tgt_idx + 1] as f32).abs();
                    }
                }
            }
            total
        };
        let [dx_sub, dy_sub] = subpixel_refine(&sad_at, bx as i32, by as i32);

        // New position = original anchor + (matched center - original center) + subpixel
        let moved_x = bx - cx as f32 + dx_sub;
        let moved_y = by - cy as f32 + dy_sub;
        Some([pos[0] + moved_x, pos[1] + moved_y])
    }

    /// Fallback tracking for layers without pixel data (velocity extrapolation).
    pub fn track_next_frame_synthetic(
        layer: &Layer,
        fps: u32,
        tracker_idx: usize,
        current_frame: u32,
    ) -> Option<[f32; 2]> {
        if tracker_idx >= layer.trackers.len() {
            return None;
        }
        let tracker = &layer.trackers[tracker_idx];
        let current_pos = tracker.position.evaluate(current_frame);
        let pos_t0 = layer.transform.eval_position(current_frame, fps);
        let pos_t1 = layer.transform.eval_position(current_frame + 1, fps);
        Some([
            current_pos[0] + (pos_t1[0] - pos_t0[0]),
            current_pos[1] + (pos_t1[1] - pos_t0[1]),
        ])
    }

    /// Run tracking analysis over a range of frames.
    pub fn analyze_track(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
    ) {
        Self::analyze_track_cancellable(comp, layer_idx, tracker_idx, start_frame, end_frame, None);
    }

    /// Run tracking analysis over a range of frames with optional cancellation flag.
    pub fn analyze_track_cancellable(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
        cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) {
        if layer_idx >= comp.layers.len() {
            return;
        }

        let fps = comp.fps;
        let mut current_positions = Vec::new();

        {
            let layer = &comp.layers[layer_idx];
            if tracker_idx >= layer.trackers.len() {
                return;
            }
            let tracker = &layer.trackers[tracker_idx];
            let start_pos = tracker.position.evaluate(start_frame);
            current_positions.push((start_frame, start_pos));

            for f in (start_frame + 1)..=end_frame {
                if let Some(ref flag) = cancel_flag {
                    if flag.load(std::sync::atomic::Ordering::Relaxed) {
                        log::info!("Motion tracking canceled via thread lifecycle guard");
                        return;
                    }
                }
                if let Some(next_pos) = Self::track_next_frame(layer, fps, tracker_idx, f - 1) {
                    current_positions.push((f, next_pos));
                }
            }
        }

        let tracker = &mut comp.layers[layer_idx].trackers[tracker_idx];
        let mut keyframes = Vec::new();

        for (f, pos) in current_positions {
            keyframes.push(Keyframe::new(f, pos, InterpolationType::Linear));
        }

        tracker.position = Animatable::Animated(keyframes);
    }

    /// Bake and apply motion tracker keyframe data to a target layer's position or rotation.
    #[allow(dead_code)]
    pub fn apply_tracker_to_target(
        comp: &mut Composition,
        source_layer_idx: usize,
        tracker_idx: usize,
        target_layer_idx: usize,
        apply_position: bool,
        apply_rotation: bool,
    ) {
        if source_layer_idx >= comp.layers.len() || target_layer_idx >= comp.layers.len() {
            return;
        }

        let tracker_kfs = match comp.layers[source_layer_idx].trackers.get(tracker_idx) {
            Some(t) => match &t.position {
                Animatable::Animated(kfs) => kfs.clone(),
                Animatable::Constant(pos) => {
                    vec![Keyframe::new(0, *pos, InterpolationType::Linear)]
                }
            },
            None => return,
        };

        if apply_position {
            comp.layers[target_layer_idx].transform.position =
                Animatable::Animated(tracker_kfs.clone());
        }

        if apply_rotation && tracker_kfs.len() > 1 {
            let mut rot_kfs = Vec::new();
            for i in 0..(tracker_kfs.len() - 1) {
                let p1 = tracker_kfs[i].value;
                let p2 = tracker_kfs[i + 1].value;
                let angle_rad = (p2[1] - p1[1]).atan2(p2[0] - p1[0]);
                let angle_deg = angle_rad.to_degrees();
                rot_kfs.push(Keyframe::new(
                    tracker_kfs[i].frame,
                    angle_deg,
                    InterpolationType::Linear,
                ));
            }
            comp.layers[target_layer_idx].transform.rotation = Animatable::Animated(rot_kfs);
        }
    }

    /// Apply motion tracking data as reverse stabilization (camera shake compensation) to anchor point / position.
    #[allow(dead_code)]
    pub fn apply_stabilize_to_layer(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        stabilize_position: bool,
        stabilize_rotation: bool,
    ) {
        Self::apply_stabilize_tracker_to_target(
            comp,
            layer_idx,
            tracker_idx,
            layer_idx,
            stabilize_position,
            stabilize_rotation,
        );
    }

    /// Apply reverse stabilization from a tracker on one layer to another layer.
    /// This is the target-aware form used by the tracker panel's Stabilize action.
    pub fn apply_stabilize_tracker_to_target(
        comp: &mut Composition,
        source_layer_idx: usize,
        tracker_idx: usize,
        target_layer_idx: usize,
        stabilize_position: bool,
        stabilize_rotation: bool,
    ) {
        if source_layer_idx >= comp.layers.len() || target_layer_idx >= comp.layers.len() {
            return;
        }

        let tracker_kfs = match comp.layers[source_layer_idx].trackers.get(tracker_idx) {
            Some(t) => match &t.position {
                Animatable::Animated(kfs) => kfs.clone(),
                Animatable::Constant(pos) => {
                    vec![Keyframe::new(0, *pos, InterpolationType::Linear)]
                }
            },
            None => return,
        };

        if tracker_kfs.is_empty() {
            return;
        }

        let base_pos = tracker_kfs[0].value;
        let init_pos = comp.layers[target_layer_idx].transform.position.evaluate(0);

        if stabilize_position {
            let mut stab_pos_kfs = Vec::new();
            for kf in &tracker_kfs {
                // Invert drift: pos(t) = init_pos - (track(t) - base_pos)
                let dx = kf.value[0] - base_pos[0];
                let dy = kf.value[1] - base_pos[1];
                stab_pos_kfs.push(Keyframe::new(
                    kf.frame,
                    [init_pos[0] - dx, init_pos[1] - dy],
                    InterpolationType::Linear,
                ));
            }
            comp.layers[target_layer_idx].transform.position = Animatable::Animated(stab_pos_kfs);
        }

        if stabilize_rotation && tracker_kfs.len() > 1 {
            let mut stab_rot_kfs = Vec::new();
            let base_angle = (tracker_kfs[1].value[1] - tracker_kfs[0].value[1])
                .atan2(tracker_kfs[1].value[0] - tracker_kfs[0].value[0])
                .to_degrees();
            let init_rot = comp.layers[target_layer_idx].transform.rotation.evaluate(0);

            for i in 0..(tracker_kfs.len() - 1) {
                let p1 = tracker_kfs[i].value;
                let p2 = tracker_kfs[i + 1].value;
                let cur_angle = (p2[1] - p1[1]).atan2(p2[0] - p1[0]).to_degrees();
                let delta_angle = cur_angle - base_angle;
                stab_rot_kfs.push(Keyframe::new(
                    tracker_kfs[i].frame,
                    init_rot - delta_angle,
                    InterpolationType::Linear,
                ));
            }
            comp.layers[target_layer_idx].transform.rotation = Animatable::Animated(stab_rot_kfs);
        }
    }
}
