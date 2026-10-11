use super::*;
use std::collections::HashSet;

mod animation;
mod references;
pub(super) use animation::*;
use references::*;

impl ProductionDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version > Self::CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "production document schema {} is newer than supported {}",
                self.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            ));
        }
        if self.project.compositions.is_empty() {
            return Err("production document must contain at least one composition".into());
        }
        if self.color_lut_path.as_ref().is_some_and(|path| {
            path.is_empty() || path.len() > Self::MAX_ASSET_STRING_LENGTH || path.contains('\0')
        }) {
            return Err("production document LUT path is invalid or too long".into());
        }
        if self.project.compositions.len() > Self::MAX_COMPOSITIONS {
            return Err("production document contains too many compositions".into());
        }
        if self.project.active_composition_idx >= self.project.compositions.len() {
            return Err("production document active composition index is out of range".into());
        }
        let mut composition_ids = HashSet::new();
        for composition in &self.project.compositions {
            validate_composition(composition, 0, &mut composition_ids)?;
        }
        for composition in &self.project.compositions {
            validate_precomp_references(composition, &composition_ids)?;
        }
        let mut composition_index = HashMap::new();
        for composition in &self.project.compositions {
            index_nested_compositions(composition, &mut composition_index);
        }
        if !precomp_graph_is_acyclic(&composition_index) {
            return Err("composition precomp graph contains a cycle".into());
        }
        let mut asset_ids = HashSet::new();
        if self.project.assets.len() > Self::MAX_ASSETS {
            return Err("production document contains too many assets".into());
        }
        let folder_ids = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => Some(asset.id.as_str()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for asset in &self.project.assets {
            if asset.id.trim().is_empty()
                || asset.name.trim().is_empty()
                || !metadata_text_is_safe(&asset.id)
                || !metadata_text_is_safe(&asset.name)
                || !asset_ids.insert(asset.id.clone())
            {
                return Err("asset ids and names must be non-empty; ids must be unique".into());
            }
            if asset.id.len() > Self::MAX_ASSET_STRING_LENGTH
                || asset.name.len() > Self::MAX_ASSET_STRING_LENGTH
            {
                return Err("asset ids and names are too long".into());
            }
            if let Some(parent_folder) = &asset.parent_folder {
                if !metadata_text_is_safe(parent_folder)
                    || parent_folder.len() > Self::MAX_ASSET_STRING_LENGTH
                    || !folder_ids.contains(parent_folder.as_str())
                {
                    return Err("asset parent folder reference is invalid".into());
                }
            }
            match &asset.item_type {
                ProjectItemType::Folder { name }
                    if name.trim().is_empty() || !metadata_text_is_safe(name) =>
                {
                    return Err("asset folder name must not be empty".into());
                }
                ProjectItemType::Folder { name } if name.len() > Self::MAX_ASSET_STRING_LENGTH => {
                    return Err("asset folder name is too long".into());
                }
                ProjectItemType::Composition { comp_idx }
                    if *comp_idx >= self.project.compositions.len() =>
                {
                    return Err("asset composition index is out of range".into());
                }
                ProjectItemType::Image { width, height, .. }
                    if *width == 0 || *height == 0 || *width > 65_535 || *height > 65_535 =>
                {
                    return Err("asset image dimensions are out of range".into());
                }
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Video { path, .. }
                | ProjectItemType::Audio { path, .. }
                    if path.trim().is_empty() || !metadata_text_is_safe(path) =>
                {
                    return Err("media asset path must not be empty".into());
                }
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Video { path, .. }
                | ProjectItemType::Audio { path, .. }
                    if path.len() > Self::MAX_ASSET_STRING_LENGTH =>
                {
                    return Err("media asset path is too long".into());
                }
                ProjectItemType::Solid { color }
                    if color.iter().any(|channel| !channel.is_finite()) =>
                {
                    return Err("solid asset color must be finite".into());
                }
                ProjectItemType::Video { duration_sec, .. }
                | ProjectItemType::Audio { duration_sec, .. }
                    if !duration_sec.is_finite()
                        || *duration_sec < 0.0
                        || *duration_sec > Self::MAX_ASSET_DURATION_SEC =>
                {
                    return Err("asset duration is out of range".into());
                }
                _ => {}
            }
        }
        let folder_parents = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => asset
                    .parent_folder
                    .as_deref()
                    .map(|parent| (asset.id.as_str(), parent)),
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        if !parent_links_are_acyclic(&folder_parents) {
            return Err("asset folder hierarchy contains a cycle".into());
        }
        if !(1..=384_000).contains(&self.audio.sample_rate) {
            return Err("audio sample rate is outside the supported range".into());
        }
        if !self.audio.master_gain.is_finite() || !(0.0..=1_000.0).contains(&self.audio.master_gain)
        {
            return Err("audio master gain is out of range".into());
        }
        if self.audio.channels.len() > Self::MAX_AUDIO_CHANNELS {
            return Err("production document contains too many audio channels".into());
        }
        for channel in &self.audio.channels {
            channel.validate().map_err(str::to_owned)?;
        }
        self.audio.correction.validate().map_err(str::to_owned)?;
        self.tempo.validate().map_err(str::to_owned)?;
        self.validate_bindings()?;
        Ok(())
    }
}

fn validate_composition(
    composition: &crate::core::timeline::Composition,
    depth: usize,
    composition_ids: &mut HashSet<String>,
) -> Result<(), String> {
    if depth > 1024 {
        return Err("production document composition nesting is too deep".into());
    }
    if composition.layers.len() > ProductionDocument::MAX_LAYERS_PER_COMPOSITION {
        return Err("composition contains too many layers".into());
    }
    if composition.sub_compositions.len() > ProductionDocument::MAX_SUB_COMPOSITIONS_PER_COMPOSITION
    {
        return Err("composition contains too many nested compositions".into());
    }
    if composition.cameras.len() > ProductionDocument::MAX_CAMERAS_PER_COMPOSITION {
        return Err("composition contains too many cameras".into());
    }
    if composition.lights.len() > ProductionDocument::MAX_LIGHTS_PER_COMPOSITION {
        return Err("composition contains too many lights".into());
    }
    if composition.markers.len() > ProductionDocument::MAX_MARKERS_PER_COMPOSITION {
        return Err("composition contains too many markers".into());
    }
    if !metadata_text_is_safe(&composition.id)
        || composition.id.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
    {
        return Err("composition id must not be empty".into());
    }
    if !metadata_text_is_safe(&composition.name)
        || composition.name.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
    {
        return Err("composition name must not be empty".into());
    }
    if !composition_ids.insert(composition.id.clone()) {
        return Err(format!("duplicate composition id: {}", composition.id));
    }
    if !(1..=ProductionDocument::MAX_COMPOSITION_DIMENSION).contains(&composition.width)
        || !(1..=ProductionDocument::MAX_COMPOSITION_DIMENSION).contains(&composition.height)
    {
        return Err("composition dimensions are outside the supported range".into());
    }
    if !(1..=240).contains(&composition.fps)
        || !(1..=ProductionDocument::MAX_COMPOSITION_FRAMES).contains(&composition.duration_frames)
    {
        return Err("composition frame rate or duration is invalid".into());
    }
    if !composition.motion_blur_shutter_angle.is_finite()
        || !composition.motion_blur_shutter_phase.is_finite()
    {
        return Err("composition motion blur settings must be finite".into());
    }
    if composition
        .background_color
        .iter()
        .any(|channel| !channel.is_finite())
    {
        return Err("composition background color must be finite".into());
    }
    validate_camera(&composition.active_camera)?;
    let mut camera_names = HashSet::new();
    if !camera_names.insert(composition.active_camera.name.clone()) {
        return Err("camera names must be unique within a composition".into());
    }
    for camera in &composition.cameras {
        validate_camera(camera)?;
        if !camera_names.insert(camera.name.clone()) {
            return Err("camera names must be unique within a composition".into());
        }
    }
    let mut light_ids = HashSet::new();
    let mut light_names = HashSet::new();
    for light in &composition.lights {
        if !metadata_text_is_safe(&light.id)
            || !metadata_text_is_safe(&light.name)
            || light.id.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
            || light.name.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
            || !light_ids.insert(light.id.clone())
            || !light_names.insert(light.name.clone())
            || light.color.iter().any(|value| !value.is_finite())
            || !light.intensity.is_finite()
            || light.intensity < 0.0
            || !vector3_animation_is_finite(&light.position)
            || !light.shadow_darkness.is_finite()
            || !(0.0..=100.0).contains(&light.shadow_darkness)
            || !light.falloff.is_finite()
            || light.falloff < 0.0
            || !light.max_radius.is_finite()
            || light.max_radius < 0.0
            || !light_type_is_valid(&light.light_type)
        {
            return Err("light settings are invalid".into());
        }
    }
    for marker in &composition.markers {
        if marker.frame >= composition.duration_frames
            || marker.label.trim().is_empty()
            || !metadata_text_is_safe(&marker.label)
            || marker.label.chars().count() > 4_096
            || marker
                .color
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err("timeline marker settings are invalid".into());
        }
    }
    let mut layer_ids = HashSet::new();
    let all_layer_ids = composition
        .layers
        .iter()
        .map(|layer| layer.id.as_str())
        .collect::<HashSet<_>>();
    let mut layer_parents = HashMap::new();
    for (layer_index, layer) in composition.layers.iter().enumerate() {
        if !metadata_text_is_safe(&layer.id)
            || !metadata_text_is_safe(&layer.name)
            || layer.id.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
            || layer.name.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
            || !layer_ids.insert(layer.id.clone())
        {
            return Err("layer ids must be non-empty and unique within a composition".into());
        }
        if let Some(parent_id) = &layer.parent_id {
            if !metadata_text_is_safe(parent_id)
                || parent_id.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                || !all_layer_ids.contains(parent_id.as_str())
            {
                return Err("layer parent reference is invalid".into());
            }
            layer_parents.insert(layer.id.as_str(), parent_id.as_str());
        }
        if layer.in_frame >= layer.out_frame || layer.out_frame > composition.duration_frames {
            return Err("layer frame range must fit within the composition".into());
        }
        if layer_index == 0 && layer.track_matte != crate::core::timeline::TrackMatteMode::None {
            return Err("the first layer cannot use a track matte without a source layer".into());
        }
        if !vector_animation_is_finite(&layer.transform.anchor_point)
            || !vector_animation_is_finite(&layer.transform.position)
            || !vector_animation_is_finite(&layer.transform.scale)
            || !scalar_animation_is_finite(&layer.transform.rotation)
            || !scalar_animation_is_percentage(&layer.transform.opacity)
        {
            return Err("layer transform animation values must be finite".into());
        }
        if !vector3_animation_is_finite(&layer.transform_3d.position)
            || !vector3_animation_is_finite(&layer.transform_3d.rotation)
            || !vector3_animation_is_finite(&layer.transform_3d.scale)
        {
            return Err("layer 3D transform animation values must be finite".into());
        }
        if layer.time_remap.as_ref().is_some_and(|value| {
            !scalar_animation_is_nonnegative(value)
                || !scalar_animation_is_at_most(
                    value,
                    ProductionDocument::MAX_COMPOSITION_FRAMES as f32,
                )
        }) {
            return Err("layer time remap values must be finite and non-negative".into());
        }
        if let LayerType::Video {
            source,
            frames_dir,
            frame_count,
            audio_wav,
            speed,
            ..
        } = &layer.layer_type
        {
            if !metadata_text_is_safe(source)
                || !metadata_text_is_safe(frames_dir)
                || source.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                || frames_dir.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                || audio_wav.as_ref().is_some_and(|path| {
                    !metadata_text_is_safe(path)
                        || path.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                })
                || *frame_count == 0
                || !speed.is_finite()
                || *speed <= 0.0
                || *speed > 1_000.0
            {
                return Err("video layer source, frame count, or speed is invalid".into());
            }
        }
        if let LayerType::Audio { path, volume } = &layer.layer_type {
            if !metadata_text_is_safe(path)
                || path.len() > ProductionDocument::MAX_ASSET_STRING_LENGTH
                || !scalar_animation_is_finite(volume)
            {
                return Err("audio layer path or volume is invalid".into());
            }
        }
        if let LayerType::Text {
            text,
            font_size,
            color,
            tracking,
            leading,
            align,
            font_family,
            stroke_color,
            stroke_width,
            ..
        } = &layer.layer_type
        {
            if text.chars().count() > 1_000_000
                || *font_size == 0
                || *font_size > 16_384
                || color.iter().any(|value| !value.is_finite())
                || stroke_color.iter().any(|value| !value.is_finite())
                || !tracking.is_finite()
                || !(-100_000.0..=100_000.0).contains(tracking)
                || !leading.is_finite()
                || *leading <= 0.0
                || *leading > 1_000.0
                || !stroke_width.is_finite()
                || *stroke_width < 0.0
                || *align > 2
                || font_family.trim().is_empty()
                || *stroke_width > 16_384.0
            {
                return Err("text layer settings are invalid".into());
            }
        }
        if let LayerType::Shape {
            shape_type,
            color,
            stroke_color,
            stroke_width,
            extrusion_depth,
            bevel_depth,
            fill_type,
        } = &layer.layer_type
        {
            if color.iter().any(|value| !value.is_finite())
                || stroke_color.iter().any(|value| !value.is_finite())
                || !stroke_width.is_finite()
                || *stroke_width < 0.0
                || !extrusion_depth.is_finite()
                || *extrusion_depth < 0.0
                || !bevel_depth.is_finite()
                || *bevel_depth < 0.0
            {
                return Err("shape layer settings are invalid".into());
            }
            if let crate::core::timeline::ShapeType::FreeformBezier {
                points,
                tangents,
                closed,
            } = shape_type
            {
                if points.len() != tangents.len() || points.len() < if *closed { 3 } else { 2 } {
                    return Err("shape points and tangents must have matching lengths".into());
                }
                if points
                    .iter()
                    .flatten()
                    .chain(
                        tangents.iter().flat_map(|(incoming, outgoing)| {
                            incoming.iter().chain(outgoing.iter())
                        }),
                    )
                    .any(|value| !value.is_finite())
                {
                    return Err("shape geometry coordinates must be finite".into());
                }
            }
            let shape_animation_values = match shape_type {
                crate::core::timeline::ShapeType::FreeformBezier { .. } => Vec::new(),
                crate::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => vec![width, height, corner_radius],
                crate::core::timeline::ShapeType::Ellipse { width, height } => {
                    vec![width, height]
                }
                crate::core::timeline::ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => vec![points, inner_radius, outer_radius],
                crate::core::timeline::ShapeType::Polygon { sides, radius } => {
                    vec![sides, radius]
                }
            };
            if shape_animation_values
                .iter()
                .any(|value| !scalar_animation_is_finite(value))
            {
                return Err("shape animation values must be finite".into());
            }
            let shape_nonnegative_values = match shape_type {
                crate::core::timeline::ShapeType::FreeformBezier { .. } => Vec::new(),
                crate::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => vec![width, height, corner_radius],
                crate::core::timeline::ShapeType::Ellipse { width, height } => {
                    vec![width, height]
                }
                crate::core::timeline::ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => vec![points, inner_radius, outer_radius],
                crate::core::timeline::ShapeType::Polygon { sides, radius } => {
                    vec![sides, radius]
                }
            };
            if shape_nonnegative_values
                .iter()
                .any(|value| !scalar_animation_is_nonnegative(value))
            {
                return Err("shape dimensions must be non-negative".into());
            }
            match shape_type {
                crate::core::timeline::ShapeType::Star { points, .. }
                | crate::core::timeline::ShapeType::Polygon { sides: points, .. }
                    if !scalar_animation_is_at_least(points, 3.0) =>
                {
                    return Err("polygon point counts must be at least three".into());
                }
                _ => {}
            }
            let fill_is_valid = match fill_type {
                crate::core::timeline::ShapeFillType::Solid => true,
                crate::core::timeline::ShapeFillType::LinearGradient {
                    start,
                    end,
                    colors,
                    stops,
                } => {
                    start
                        .iter()
                        .chain(end.iter())
                        .all(|value| value.is_finite())
                        && valid_gradient_stops(colors.as_slice(), stops.as_slice())
                }
                crate::core::timeline::ShapeFillType::RadialGradient {
                    center,
                    radius,
                    colors,
                    stops,
                } => {
                    center.iter().all(|value| value.is_finite())
                        && radius.is_finite()
                        && *radius > 0.0
                        && valid_gradient_stops(colors.as_slice(), stops.as_slice())
                }
            };
            if !fill_is_valid {
                return Err("shape gradient settings are invalid".into());
            }
        }
        if let LayerType::Particle { emitter } = &layer.layer_type {
            if !particle_emitter_is_valid(emitter) {
                return Err("particle emitter settings are invalid".into());
            }
        }
        if layer.effects.len() > ProductionDocument::MAX_EFFECTS_PER_LAYER {
            return Err("layer contains too many effects".into());
        }
        if layer.masks.len() > ProductionDocument::MAX_MASKS_PER_LAYER {
            return Err("layer contains too many masks".into());
        }
        let mut effect_ids = HashSet::new();
        for effect in &layer.effects {
            if effect.id.trim().is_empty()
                || effect.name.trim().is_empty()
                || !effect_ids.insert(effect.id.clone())
            {
                return Err("effect ids must be non-empty and unique within a layer".into());
            }
            let effect_debug = format!("{:?}", effect.effect_type);
            if !effect_debug_is_finite(&effect_debug) {
                return Err("effect parameters must be finite and serializable".into());
            }
        }
        let mut mask_ids = HashSet::new();
        for mask in &layer.masks {
            if mask.id.trim().is_empty()
                || mask.name.trim().is_empty()
                || !mask_ids.insert(mask.id.clone())
            {
                return Err("mask ids must be non-empty and unique within a layer".into());
            }
            validate_mask(mask)?;
        }
    }
    if !parent_links_are_acyclic(&layer_parents) {
        return Err("layer parent hierarchy contains a cycle".into());
    }
    for nested in &composition.sub_compositions {
        validate_composition(nested, depth + 1, composition_ids)?;
    }
    Ok(())
}
