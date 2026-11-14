use super::*;

pub(super) fn apply_gltf_sampler(
    material: &mut PbrMaterial3D,
    sampler: Option<&GltfSamplerJson>,
) -> Result<(), String> {
    let Some(sampler) = sampler else {
        return Ok(());
    };
    material.texture_wrap_s = sampler.wrap_s.unwrap_or(10497);
    material.texture_wrap_t = sampler.wrap_t.unwrap_or(10497);
    if ![33071, 33648, 10497].contains(&material.texture_wrap_s)
        || ![33071, 33648, 10497].contains(&material.texture_wrap_t)
    {
        return Err("glTF texture sampler uses an unsupported wrap mode".into());
    }
    let nearest = |value: Option<u32>, valid: &[u32]| -> Result<bool, String> {
        match value {
            None => Ok(false),
            Some(value) if valid.contains(&value) => Ok(matches!(value, 9728 | 9984 | 9986)),
            Some(value) => Err(format!("Unsupported glTF texture filter {value}")),
        }
    };
    let min_nearest = nearest(sampler.min_filter, &[9728, 9729, 9984, 9985, 9986, 9987])?;
    let mag_nearest = nearest(sampler.mag_filter, &[9728, 9729])?;
    material.texture_min_nearest = min_nearest;
    material.texture_mag_nearest = mag_nearest;
    Ok(())
}

pub(super) fn apply_gltf_texture_transform(
    material: &mut PbrMaterial3D,
    info: &GltfTextureInfo,
) -> Result<(), String> {
    let transform = info
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.texture_transform.as_ref());
    let effective_tex_coord = transform
        .and_then(|value| value.tex_coord)
        .unwrap_or(info.tex_coord);
    if effective_tex_coord > 7 {
        return Err(format!("glTF texture requests TEXCOORD_{effective_tex_coord}, but the supported range is TEXCOORD_0 through TEXCOORD_7"));
    }
    material.texture_tex_coord = effective_tex_coord;
    let Some(transform) = transform else {
        return Ok(());
    };
    let offset = transform.offset.unwrap_or([0.0; 2]);
    let scale = transform.scale.unwrap_or([1.0; 2]);
    let rotation = transform.rotation.unwrap_or(0.0);
    if offset
        .iter()
        .chain(scale.iter())
        .any(|value| !value.is_finite())
        || !rotation.is_finite()
    {
        return Err("glTF KHR_texture_transform contains a non-finite value".into());
    }
    material.texture_uv_offset = offset;
    material.texture_uv_scale = scale;
    material.texture_uv_rotation = rotation;
    Ok(())
}
