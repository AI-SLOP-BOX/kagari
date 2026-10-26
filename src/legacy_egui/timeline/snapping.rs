pub fn maybe_snap_frame(frame: u32, snap: bool, keyframes: &[u32]) -> u32 {
    if !snap {
        return frame;
    }
    let index = keyframes.partition_point(|keyframe| *keyframe < frame);
    let before = index.checked_sub(1).and_then(|i| keyframes.get(i)).copied();
    let after = keyframes.get(index).copied();
    match (before, after) {
        (Some(before), Some(after)) => {
            let before_distance = frame.abs_diff(before);
            let after_distance = frame.abs_diff(after);
            if before_distance <= 3 && before_distance <= after_distance {
                before
            } else if after_distance <= 3 {
                after
            } else {
                frame
            }
        }
        (Some(before), None) if frame.abs_diff(before) <= 3 => before,
        (None, Some(after)) if frame.abs_diff(after) <= 3 => after,
        _ => frame,
    }
}

pub fn snap_to_sorted_layer_edges(
    frame: u32,
    exclude_idx: usize,
    sorted_edges: &[(u32, usize)],
) -> u32 {
    const THRESHOLD: u32 = 5;
    let split = sorted_edges.partition_point(|(edge, _)| *edge < frame);
    let mut best: Option<(u32, u32)> = None;

    let mut consider = |edge, layer_idx| {
        let distance = frame.abs_diff(edge);
        if layer_idx != exclude_idx
            && best.is_none_or(|(best_distance, _)| distance < best_distance)
        {
            best = Some((distance, edge));
        }
    };
    for &(edge, layer_idx) in sorted_edges[..split].iter().rev() {
        if frame.abs_diff(edge) > THRESHOLD {
            break;
        }
        consider(edge, layer_idx);
    }
    for &(edge, layer_idx) in &sorted_edges[split..] {
        if frame.abs_diff(edge) > THRESHOLD {
            break;
        }
        consider(edge, layer_idx);
    }
    best.map_or(frame, |(_, edge)| edge)
}

pub fn snap_to_layer_edges(
    frame: u32,
    exclude_idx: usize,
    comp: &crate::core::timeline::Composition,
) -> u32 {
    let threshold = 5i32;
    let mut best = frame;
    let mut best_dist = threshold + 1;
    for (i, layer) in comp.layers.iter().enumerate() {
        if i == exclude_idx {
            continue;
        }
        for edge in [layer.in_frame, layer.out_frame] {
            let dist = (frame as i32 - edge as i32).abs();
            if dist < best_dist {
                best_dist = dist;
                best = edge;
            }
        }
    }
    best
}

pub fn collect_all_kf_frames(comp: &crate::core::timeline::Composition) -> Vec<u32> {
    let mut frames = Vec::new();
    for layer in &comp.layers {
        if let Some(kfs) = layer.transform.position.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.scale.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.rotation.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.opacity.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        for pin in &layer.puppet_pins {
            if let Some(kfs) = pin.position.keyframes() {
                for kf in kfs {
                    frames.push(kf.frame);
                }
            }
        }
    }
    frames.sort();
    frames.dedup();
    frames
}
