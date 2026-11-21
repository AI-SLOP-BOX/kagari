use super::parse_texture::{apply_gltf_sampler, apply_gltf_texture_transform};
use super::*;

pub(super) fn initialize_materials_and_textures(
    root: &GltfRootJson,
    binary_payloads: &[&[u8]],
    mesh: &mut Mesh3D,
) -> Result<(), String> {
    // Map PBR materials
    mesh.materials.clear();
    if root.materials.is_empty() {
        mesh.materials
            .push(gltf_default_material("DefaultMaterial"));
    } else {
        for mat_json in &root.materials {
            if mat_json
                .emissive_factor
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            {
                return Err("glTF emissiveFactor components must be finite and within 0..1".into());
            }
            let mut mat = PbrMaterial3D {
                name: mat_json
                    .name
                    .clone()
                    .unwrap_or_else(|| "Material".to_string()),
                ..Default::default()
            };
            mat.albedo_color = default_base_color_factor();
            mat.metallic = default_factor_one();
            mat.roughness = default_factor_one();
            if let Some(pbr) = &mat_json.pbr_metallic_roughness {
                if pbr
                    .base_color_factor
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                {
                    return Err(
                        "glTF baseColorFactor components must be finite and within 0..1".into(),
                    );
                }
                if !pbr.metallic_factor.is_finite()
                    || !(0.0..=1.0).contains(&pbr.metallic_factor)
                    || !pbr.roughness_factor.is_finite()
                    || !(0.0..=1.0).contains(&pbr.roughness_factor)
                {
                    return Err(
                        "glTF metallicFactor and roughnessFactor must be finite and within 0..1"
                            .into(),
                    );
                }
                mat.albedo_color = pbr.base_color_factor;
                mat.metallic = pbr.metallic_factor;
                mat.roughness = pbr.roughness_factor;
                mat.albedo_texture_index = pbr.base_color_texture.as_ref().map(|info| info.index);
                if let Some(info) = &pbr.base_color_texture {
                    apply_gltf_texture_transform(&mut mat, info)?;
                }
            }
            mat.emission = mat_json.emissive_factor;
            mat.alpha_mode = mat_json
                .alpha_mode
                .as_deref()
                .unwrap_or("OPAQUE")
                .to_string();
            if !matches!(mat.alpha_mode.as_str(), "OPAQUE" | "MASK" | "BLEND") {
                return Err(format!(
                    "glTF material has unsupported alphaMode `{}`",
                    mat.alpha_mode
                ));
            }
            if let Some(cutoff) = mat_json.alpha_cutoff {
                if !cutoff.is_finite() || !(0.0..=1.0).contains(&cutoff) {
                    return Err("glTF material alphaCutoff must be finite and within 0..1".into());
                }
                mat.alpha_cutoff = cutoff;
            }
            mesh.materials.push(mat);
        }
    }

    // Decode embedded image sources once; material references retain an index into this list.
    let mut embedded_image_bytes = 0usize;
    for image in &root.images {
        let bytes = if let Some(uri) = image.uri.as_deref() {
            if let Some(data_uri) = uri.strip_prefix("data:") {
                let (metadata, payload) = data_uri
                    .split_once(',')
                    .ok_or("Malformed glTF image data URI")?;
                if metadata.to_ascii_lowercase().ends_with(";base64") {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD
                        .decode(payload)
                        .map_err(|e| format!("Invalid glTF image base64: {e}"))?
                } else {
                    percent_decode_uri_component(payload)?
                }
            } else {
                Vec::new()
            }
        } else if let Some(view_index) = image.buffer_view {
            let view = root
                .buffer_views
                .get(view_index)
                .ok_or("Invalid glTF image bufferView")?;
            let buffer = binary_payloads
                .get(view.buffer)
                .ok_or("Invalid glTF image buffer index")?;
            let end = view
                .byte_offset
                .checked_add(view.byte_length)
                .filter(|&end| end <= buffer.len())
                .ok_or("glTF image bufferView is out of bounds")?;
            buffer[view.byte_offset..end].to_vec()
        } else {
            return Err("glTF image must declare a URI or bufferView".into());
        };
        if !bytes.is_empty() {
            let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
                .with_guessed_format()
                .map_err(|e| format!("Could not identify embedded glTF image: {e}"))?;
            let (width, height) = reader
                .into_dimensions()
                .map_err(|e| format!("Could not inspect embedded glTF image: {e}"))?;
            let decoded_size = (width as usize)
                .checked_mul(height as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .filter(|&size| {
                    width > 0
                        && height > 0
                        && width <= crate::core::software_renderer::MAX_RENDER_DIMENSION
                        && height <= crate::core::software_renderer::MAX_RENDER_DIMENSION
                        && size <= 256 * 1024 * 1024
                })
                .ok_or("Embedded glTF texture exceeds the 256 MiB decoded image limit")?;
            embedded_image_bytes = embedded_image_bytes
                .checked_add(decoded_size)
                .filter(|&size| size <= 256 * 1024 * 1024)
                .ok_or("Combined embedded glTF textures exceed the 256 MiB decoded image limit")?;
            let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
                .with_guessed_format()
                .map_err(|e| format!("Could not identify embedded glTF image: {e}"))?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
            limits.max_image_height = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
            limits.max_alloc = Some(256 * 1024 * 1024);
            reader.limits(limits);
            let decoded = reader
                .decode()
                .map_err(|e| format!("Could not decode embedded glTF image within limits: {e}"))?
                .into_rgba8();
            if decoded.width() != width || decoded.height() != height {
                return Err("Embedded glTF image dimensions changed while decoding".into());
            }
            mesh.embedded_textures.push(std::sync::Arc::new(
                crate::core::image_cache::CachedImage {
                    width: decoded.width(),
                    height: decoded.height(),
                    pixels: decoded.into_raw(),
                },
            ));
        } else {
            mesh.embedded_textures.push(std::sync::Arc::new(
                crate::core::image_cache::CachedImage {
                    width: 0,
                    height: 0,
                    pixels: Vec::new(),
                },
            ));
        }
    }
    for material in &mut mesh.materials {
        if let Some(texture_index) = material.albedo_texture_index {
            let sampler = root
                .textures
                .get(texture_index)
                .and_then(|texture| texture.sampler)
                .map(|index| {
                    root.samplers
                        .get(index)
                        .ok_or_else(|| format!("Invalid glTF sampler index {index}"))
                })
                .transpose()?;
            apply_gltf_sampler(material, sampler)?;
            material.albedo_texture_index = Some(
                root.textures
                    .get(texture_index)
                    .and_then(|texture| texture.source)
                    .ok_or_else(|| {
                        format!("glTF texture {texture_index} has no supported image source")
                    })?,
            );
        }
    }
    Ok(())
}
