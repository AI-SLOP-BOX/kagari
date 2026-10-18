use super::*;

pub(super) fn validate_lottie_comp(comp: &Composition, allow_precomp: bool) -> Result<(), String> {
    for layer in &comp.layers {
        if !matches!(
            layer.blend_mode,
            BlendMode::Normal
                | BlendMode::Multiply
                | BlendMode::Screen
                | BlendMode::Overlay
                | BlendMode::Darken
                | BlendMode::Lighten
                | BlendMode::ColorDodge
                | BlendMode::ColorBurn
                | BlendMode::HardLight
                | BlendMode::SoftLight
                | BlendMode::Difference
                | BlendMode::Exclusion
                | BlendMode::Hue
                | BlendMode::Saturation
                | BlendMode::Color
                | BlendMode::Luminosity
                | BlendMode::Add
        ) {
            return Err(format!(
                "Lottie export does not support blend mode {:?} on layer '{}'",
                layer.blend_mode, layer.name
            ));
        }
        match &layer.layer_type {
            LayerType::Solid { .. }
            | LayerType::Image { .. }
            | LayerType::Null
            | LayerType::Text { .. }
            | LayerType::Shape { .. } => {}
            LayerType::PreComp { .. } if allow_precomp => {}
            LayerType::PreComp { .. } => {
                return Err(format!(
                    "Layer '{}' is a precomposition; export the whole project to preserve it",
                    layer.name
                ));
            }
            LayerType::Video { .. }
            | LayerType::Model3D { .. }
            | LayerType::Audio { .. }
            | LayerType::AdjustmentLayer
            | LayerType::Particle { .. } => {
                return Err(format!(
                    "Lottie export does not support layer '{}' ({})",
                    layer.name,
                    match &layer.layer_type {
                        LayerType::Video { .. } => "video",
                        LayerType::Model3D { .. } => "3D model",
                        LayerType::Audio { .. } => "audio",
                        LayerType::AdjustmentLayer => "adjustment",
                        LayerType::Particle { .. } => "particle",
                        _ => unreachable!(),
                    }
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn collect_image_assets(
    compositions: &[&Composition],
) -> Result<(std::collections::HashMap<String, String>, Vec<Value>), String> {
    use std::collections::{HashMap, HashSet};

    let mut ids = HashMap::new();
    let mut seen_paths = HashSet::new();
    let mut used_ids: HashSet<String> = compositions.iter().map(|comp| comp.id.clone()).collect();
    let mut assets = Vec::new();

    for composition in compositions {
        for layer in &composition.layers {
            let LayerType::Image { path } = &layer.layer_type else {
                continue;
            };
            if !seen_paths.insert(path.clone()) {
                continue;
            }
            let path_ref = std::path::Path::new(path);
            let (width, height) = image::image_dimensions(path_ref)
                .map_err(|error| format!("Cannot export image layer '{}': {error}", layer.name))?;
            let filename = path_ref
                .file_name()
                .ok_or_else(|| format!("Image path has no filename: {path}"))?
                .to_string_lossy()
                .into_owned();
            let directory = path_ref
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| {
                    let mut value = parent.to_string_lossy().into_owned();
                    if !value.ends_with(std::path::MAIN_SEPARATOR) {
                        value.push(std::path::MAIN_SEPARATOR);
                    }
                    value
                })
                .unwrap_or_default();
            let mut id = format!("kagari_image_{}", assets.len());
            while !used_ids.insert(id.clone()) {
                id.push('_');
            }
            ids.insert(path.clone(), id.clone());
            assets.push(json!({
                "id": id,
                "w": width,
                "h": height,
                "u": directory,
                "p": filename,
                "e": 0,
            }));
        }
    }
    Ok((ids, assets))
}
