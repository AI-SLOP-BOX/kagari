use super::*;

const MAX_ROTO_STROKES_PER_LAYER: usize = 10_000;
const MAX_ROTO_POINTS_PER_LAYER: usize = 1_000_000;

pub(super) fn metadata_text_is_safe(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && !value.chars().any(char::is_control)
}

pub(super) fn validate_precomp_references(
    composition: &crate::core::timeline::Composition,
    composition_ids: &HashSet<String>,
) -> Result<(), String> {
    for layer in &composition.layers {
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if !metadata_text_is_safe(comp_id)
                || comp_id.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                || !composition_ids.contains(comp_id)
            {
                return Err(format!("precomp references missing composition: {comp_id}"));
            }
        }
    }
    for nested in &composition.sub_compositions {
        validate_precomp_references(nested, composition_ids)?;
    }
    Ok(())
}

pub(super) fn validate_camera(camera: &crate::core::timeline::Camera3D) -> Result<(), String> {
    if !metadata_text_is_safe(&camera.name)
        || camera.name.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
        || !camera.fov_degrees.is_finite()
        || !(0.0..180.0).contains(&camera.fov_degrees)
        || !camera.focus_distance.is_finite()
        || camera.focus_distance <= 0.0
        || !camera.aperture.is_finite()
        || camera.aperture < 0.0
        || !camera.dof_max_blur.is_finite()
        || camera.dof_max_blur < 0.0
        || camera
            .dof_max_blur_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
        || (camera.dof_iris_sides != 0 && !(3..=32).contains(&camera.dof_iris_sides))
    {
        return Err("camera settings are invalid".into());
    }
    if !vector3_animation_is_finite(&camera.transform.position)
        || !vector3_animation_is_finite(&camera.transform.rotation)
        || !vector3_animation_is_finite(&camera.transform.scale)
        || camera
            .fov_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
        || camera
            .focus_distance_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
        || camera
            .aperture_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
        || camera
            .dof_max_blur_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
        || camera
            .dof_enabled_animation
            .as_ref()
            .is_some_and(|track| !scalar_animation_is_finite(track))
    {
        return Err("camera animation values must be finite".into());
    }
    Ok(())
}

pub(super) fn mask_maximum_vertex_count(mask: &crate::core::mask::Mask) -> usize {
    match &mask.path.vertices {
        crate::core::property::Animatable::Constant(vertices) => vertices.len(),
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .map(|keyframe| keyframe.value.len())
            .max()
            .unwrap_or(0),
    }
}

pub(super) fn validate_mask(mask: &crate::core::mask::Mask) -> Result<(), String> {
    let validate_vertices =
        |vertices: &[[f32; 2]]| vertices.iter().flatten().all(|value| value.is_finite());
    let maximum_vertex_count = mask_maximum_vertex_count(mask);
    if maximum_vertex_count > crate::core::mask::MAX_MASK_PATH_VERTICES {
        return Err(format!(
            "mask path exceeds the {} vertex limit",
            crate::core::mask::MAX_MASK_PATH_VERTICES
        ));
    }
    match &mask.path.vertices {
        crate::core::property::Animatable::Constant(vertices) => {
            if !validate_vertices(vertices) {
                return Err("mask vertex coordinates must be finite".into());
            }
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            if !keyframes_are_strictly_ordered(keyframes)
                || keyframes
                    .iter()
                    .any(|keyframe| !validate_vertices(&keyframe.value))
            {
                return Err("mask path keyframes must be ordered and finite".into());
            }
        }
    }
    if let Some(tangents) = &mask.path.tangents {
        if tangents
            .iter()
            .flat_map(|(incoming, outgoing)| incoming.iter().chain(outgoing.iter()))
            .any(|value| !value.is_finite())
        {
            return Err("mask tangent coordinates must be finite".into());
        }
    }
    for value in [&mask.feather, &mask.expansion] {
        if !scalar_animation_is_finite(value) {
            return Err("mask animation values must be finite".into());
        }
    }
    if !scalar_animation_is_percentage(&mask.opacity) {
        return Err("mask opacity must be within 0..100".into());
    }
    Ok(())
}

pub(super) fn validate_roto_strokes(
    strokes: &[crate::core::roto_brush_engine::RotoStroke],
    duration_frames: u32,
) -> Result<(), String> {
    if strokes.len() > MAX_ROTO_STROKES_PER_LAYER {
        return Err(format!(
            "layer contains more than {MAX_ROTO_STROKES_PER_LAYER} Roto Brush strokes"
        ));
    }
    let mut total_points = 0usize;
    for stroke in strokes {
        if !stroke.radius.is_finite() || stroke.radius < 0.0 {
            return Err("Roto Brush stroke radius must be finite and non-negative".into());
        }
        if stroke.points.len() > MAX_ROTO_POINTS_PER_LAYER {
            return Err("Roto Brush stroke exceeds the per-layer point limit".into());
        }
        total_points = total_points
            .checked_add(stroke.points.len())
            .filter(|&count| count <= MAX_ROTO_POINTS_PER_LAYER)
            .ok_or_else(|| "layer exceeds the Roto Brush point limit".to_owned())?;
        if stroke
            .points
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        {
            return Err("Roto Brush stroke coordinates must be finite".into());
        }
        if !stroke.pressures.is_empty()
            && (stroke.pressures.len() != stroke.points.len()
                || stroke
                    .pressures
                    .iter()
                    .any(|pressure| !pressure.is_finite() || !(0.0..=1.0).contains(pressure)))
        {
            return Err("Roto Brush stroke pressure data is invalid".into());
        }
        if stroke.frame.is_some_and(|frame| frame >= duration_frames) {
            return Err("Roto Brush stroke frame is outside the composition".into());
        }
    }
    Ok(())
}

pub(super) fn parent_links_are_acyclic<'a>(parents: &HashMap<&'a str, &'a str>) -> bool {
    let mut state = HashMap::<&str, u8>::with_capacity(parents.len());
    for &start in parents.keys() {
        if state.get(start) == Some(&2) {
            continue;
        }
        let mut path = Vec::new();
        let mut current = Some(start);
        while let Some(id) = current {
            match state.get(id) {
                Some(1) => return false,
                Some(2) => break,
                _ => {}
            }
            state.insert(id, 1);
            path.push(id);
            current = parents.get(id).copied();
        }
        for id in path {
            state.insert(id, 2);
        }
    }
    true
}

pub(super) fn index_nested_compositions<'a>(
    composition: &'a Composition,
    index: &mut HashMap<&'a str, &'a Composition>,
) {
    index.insert(composition.id.as_str(), composition);
    for nested in &composition.sub_compositions {
        index_nested_compositions(nested, index);
    }
}

pub(super) fn validate_precomp_graph<'a>(
    compositions: &HashMap<&'a str, &'a Composition>,
) -> Result<(), String> {
    let mut dependency_count = compositions
        .keys()
        .map(|&id| (id, 0usize))
        .collect::<HashMap<_, _>>();
    let mut longest_path = compositions
        .keys()
        .map(|&id| (id, 0usize))
        .collect::<HashMap<_, _>>();
    let mut dependents = HashMap::<&str, Vec<&str>>::new();
    for (&id, composition) in compositions {
        for layer in &composition.layers {
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                let target = comp_id.as_str();
                if compositions.contains_key(target) {
                    *dependency_count.entry(id).or_default() += 1;
                    dependents.entry(target).or_default().push(id);
                }
            }
        }
    }
    let mut ready = dependency_count
        .iter()
        .filter_map(|(&id, &count)| (count == 0).then_some(id))
        .collect::<Vec<_>>();
    let mut processed = 0usize;
    while let Some(id) = ready.pop() {
        processed += 1;
        if let Some(parents) = dependents.get(id) {
            for &dependent in parents {
                let candidate_depth = longest_path
                    .get(id)
                    .copied()
                    .unwrap_or_default()
                    .saturating_add(1);
                let depth = longest_path.entry(dependent).or_default();
                *depth = (*depth).max(candidate_depth);
                if *depth > crate::core::software_renderer::MAX_PRECOMP_DEPTH as usize {
                    return Err(format!(
                        "composition precomp nesting exceeds the renderer limit of {}",
                        crate::core::software_renderer::MAX_PRECOMP_DEPTH
                    ));
                }
                let Some(count) = dependency_count.get_mut(dependent) else {
                    return Err("composition precomp graph is inconsistent".into());
                };
                *count -= 1;
                if *count == 0 {
                    ready.push(dependent);
                }
            }
        }
    }
    if processed != compositions.len() {
        return Err("composition precomp graph contains a cycle".into());
    }
    Ok(())
}
