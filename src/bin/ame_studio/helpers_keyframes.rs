pub(crate) fn move_layer_keyframes_at_frame(
    layer: &mut kagari_vfx::core::timeline::Layer,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let mut changed = layer.transform.position.move_keyframe(from_frame, to_frame);
    changed |= layer
        .transform
        .anchor_point
        .move_keyframe(from_frame, to_frame);
    changed |= layer.transform.scale.move_keyframe(from_frame, to_frame);
    changed |= layer.transform.rotation.move_keyframe(from_frame, to_frame);
    changed |= layer.transform.opacity.move_keyframe(from_frame, to_frame);
    if layer.is_3d {
        changed |= layer
            .transform_3d
            .position
            .move_keyframe(from_frame, to_frame);
        changed |= layer
            .transform_3d
            .rotation
            .move_keyframe(from_frame, to_frame);
        changed |= layer.transform_3d.scale.move_keyframe(from_frame, to_frame);
    }
    if let Some(time_remap) = &mut layer.time_remap {
        changed |= time_remap.move_keyframe(from_frame, to_frame);
    }
    match &mut layer.layer_type {
        kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
            changed |= volume.move_keyframe(from_frame, to_frame);
        }
        kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
            kagari_vfx::core::timeline::ShapeType::Rectangle {
                width,
                height,
                corner_radius,
            } => {
                changed |= width.move_keyframe(from_frame, to_frame);
                changed |= height.move_keyframe(from_frame, to_frame);
                changed |= corner_radius.move_keyframe(from_frame, to_frame);
            }
            kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                changed |= width.move_keyframe(from_frame, to_frame);
                changed |= height.move_keyframe(from_frame, to_frame);
            }
            _ => {}
        },
        _ => {}
    }
    for effect in &mut layer.effects {
        for (_, parameter) in effect.effect_type.animatable_params() {
            use kagari_vfx::core::effect_params::ParamRef;
            changed |= match parameter {
                ParamRef::Scalar(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec2(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec3(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec4Color(track) => track.move_keyframe(from_frame, to_frame),
            };
        }
    }
    for mask in &mut layer.masks {
        changed |= mask.path.vertices.move_keyframe(from_frame, to_frame);
        changed |= mask.feather.move_keyframe(from_frame, to_frame);
        changed |= mask.opacity.move_keyframe(from_frame, to_frame);
        changed |= mask.expansion.move_keyframe(from_frame, to_frame);
    }
    changed
}

pub(crate) fn move_camera_keyframes_at_frame(
    camera: &mut kagari_vfx::core::timeline::Camera3D,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let mut changed = false;
    for track in [
        camera.fov_animation.as_mut(),
        camera.focus_distance_animation.as_mut(),
        camera.aperture_animation.as_mut(),
        camera.dof_max_blur_animation.as_mut(),
        camera.dof_enabled_animation.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        changed |= track.move_keyframe(from_frame, to_frame);
    }
    changed
}

pub(crate) fn layer_keyframe_frames(layer: &kagari_vfx::core::timeline::Layer) -> Vec<u32> {
    let mut frames = Vec::new();
    macro_rules! add_track_frames {
        ($track:expr) => {
            if let Some(keyframes) = $track.keyframes() {
                frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
            }
        };
    }

    add_track_frames!(layer.transform.anchor_point);
    add_track_frames!(layer.transform.position);
    add_track_frames!(layer.transform.scale);
    add_track_frames!(layer.transform.rotation);
    add_track_frames!(layer.transform.opacity);
    if layer.is_3d {
        add_track_frames!(layer.transform_3d.position);
        add_track_frames!(layer.transform_3d.rotation);
        add_track_frames!(layer.transform_3d.scale);
    }
    if let Some(time_remap) = &layer.time_remap {
        add_track_frames!(time_remap);
    }
    for effect in &layer.effects {
        for (_, parameter) in effect.effect_type.animatable_params_ref() {
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Scalar(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec2(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec3(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec4Color(track) => {
                    add_track_frames!(track);
                }
            }
        }
    }
    for mask in &layer.masks {
        add_track_frames!(mask.path.vertices);
        add_track_frames!(mask.feather);
        add_track_frames!(mask.opacity);
        add_track_frames!(mask.expansion);
    }
    match &layer.layer_type {
        kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
            add_track_frames!(volume);
        }
        kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
            kagari_vfx::core::timeline::ShapeType::Rectangle {
                width,
                height,
                corner_radius,
            } => {
                add_track_frames!(width);
                add_track_frames!(height);
                add_track_frames!(corner_radius);
            }
            kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                add_track_frames!(width);
                add_track_frames!(height);
            }
            kagari_vfx::core::timeline::ShapeType::Star {
                points,
                inner_radius,
                outer_radius,
            } => {
                add_track_frames!(points);
                add_track_frames!(inner_radius);
                add_track_frames!(outer_radius);
            }
            kagari_vfx::core::timeline::ShapeType::Polygon { sides, radius } => {
                add_track_frames!(sides);
                add_track_frames!(radius);
            }
            kagari_vfx::core::timeline::ShapeType::FreeformBezier { .. } => {}
        },
        _ => {}
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn layer_keyframe_frames_for_composition(
    composition: &kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Vec<u32> {
    let mut frames = layer_keyframe_frames(layer);
    let camera_id = layer
        .scene_object
        .as_ref()
        .and_then(|scene_object| match scene_object {
            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => Some(id.as_str()),
            _ => None,
        });
    if let Some(camera) =
        camera_id.and_then(|id| composition.cameras.iter().find(|camera| camera.id == id))
    {
        frames.extend(camera_keyframe_frames(camera));
    }
    let light_id = layer
        .scene_object
        .as_ref()
        .and_then(|scene_object| match scene_object {
            kagari_vfx::core::timeline::SceneObjectRef::Light { id } => Some(id.as_str()),
            _ => None,
        });
    if let Some(track) = light_id
        .and_then(|id| composition.lights.iter().find(|light| light.id == id))
        .and_then(|light| light.intensity_animation.as_ref())
        .and_then(|track| track.keyframes())
    {
        frames.extend(track.iter().map(|key| key.frame));
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn camera_keyframe_frames(camera: &kagari_vfx::core::timeline::Camera3D) -> Vec<u32> {
    let mut frames = Vec::new();
    for track in [
        camera.fov_animation.as_ref(),
        camera.focus_distance_animation.as_ref(),
        camera.aperture_animation.as_ref(),
        camera.dof_max_blur_animation.as_ref(),
        camera.dof_enabled_animation.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(keyframes) = track.keyframes() {
            frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
        }
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn camera_for_layer<'a>(
    composition: &'a kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Option<&'a kagari_vfx::core::timeline::Camera3D> {
    let kagari_vfx::core::timeline::SceneObjectRef::Camera { id } = layer.scene_object.as_ref()?
    else {
        return None;
    };
    composition.cameras.iter().find(|camera| camera.id == *id)
}

pub(crate) fn light_for_layer<'a>(
    composition: &'a kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Option<&'a kagari_vfx::core::timeline::Light3D> {
    let light_id = match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Light { id } => id,
        _ => return None,
    };
    composition
        .lights
        .iter()
        .find(|light| light.id == *light_id)
}

pub(crate) fn camera_id_for_layer(layer: &kagari_vfx::core::timeline::Layer) -> Option<String> {
    match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => Some(id.clone()),
        _ => None,
    }
}

pub(crate) fn graph_targets_mut<'a>(
    composition: &'a mut kagari_vfx::core::timeline::Composition,
    layer_id: &str,
) -> Option<(
    &'a mut kagari_vfx::core::timeline::Layer,
    Option<&'a mut kagari_vfx::core::timeline::Camera3D>,
    Option<&'a mut kagari_vfx::core::timeline::Light3D>,
)> {
    let camera_id = composition
        .layers
        .iter()
        .find(|layer| layer.id == layer_id && !layer.locked)
        .and_then(camera_id_for_layer);
    let light_id = composition
        .layers
        .iter()
        .find(|layer| layer.id == layer_id && !layer.locked)
        .and_then(light_id_for_layer);
    let (layers, cameras, lights) = (
        &mut composition.layers,
        &mut composition.cameras,
        &mut composition.lights,
    );
    let layer = layers
        .iter_mut()
        .find(|layer| layer.id == layer_id && !layer.locked)?;
    let camera = camera_id.and_then(|id| cameras.iter_mut().find(|camera| camera.id == id));
    let light = light_id.and_then(|id| lights.iter_mut().find(|light| light.id == id));
    Some((layer, camera, light))
}

pub(crate) fn light_id_for_layer(layer: &kagari_vfx::core::timeline::Layer) -> Option<String> {
    match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Light { id } => Some(id.clone()),
        _ => None,
    }
}
