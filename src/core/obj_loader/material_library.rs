use super::{parse_obj_str_with_materials, Mesh3D, PbrMaterial3D};
use std::collections::{HashMap, HashSet};

const MAX_OBJ_MATERIAL_LIBRARIES: usize = 1024;
const MAX_OBJ_MATERIAL_PATH_BYTES: usize = 4096;

pub(crate) struct ObjFileSource {
    content: String,
    pub(crate) material_paths: Vec<std::path::PathBuf>,
}

pub(crate) fn read_obj_file_source(
    path: impl AsRef<std::path::Path>,
) -> Result<ObjFileSource, String> {
    let path = path.as_ref();
    let content =
        crate::core::project_migration::read_bounded_text_file(path, 128 * 1024 * 1024)
            .map_err(|error| format!("Could not read OBJ model '{}': {error}", path.display()))?;
    let material_paths = material_library_paths_from_content(path, &content)?;
    Ok(ObjFileSource {
        content,
        material_paths,
    })
}

pub fn obj_material_library_paths(
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<std::path::PathBuf>, String> {
    let path = path.as_ref();
    let content =
        crate::core::project_migration::read_bounded_text_file(path, 128 * 1024 * 1024)
            .map_err(|error| format!("Could not read OBJ model '{}': {error}", path.display()))?;
    material_library_paths_from_content(path, &content)
}

fn material_library_paths_from_content(
    path: &std::path::Path,
    content: &str,
) -> Result<Vec<std::path::PathBuf>, String> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let names = obj_material_library_names(content)?;
    let mut libraries = Vec::new();
    let mut seen = HashSet::new();
    libraries
        .try_reserve(names.len())
        .map_err(|error| format!("Could not reserve OBJ material paths: {error}"))?;
    seen.try_reserve(names.len())
        .map_err(|error| format!("Could not reserve OBJ material path index: {error}"))?;
    for name in names {
        let material_path = parent.join(name);
        if seen.insert(material_path.clone()) {
            libraries.push(material_path);
        }
    }
    Ok(libraries)
}

pub fn parse_obj_file(path: impl AsRef<std::path::Path>) -> Result<Mesh3D, String> {
    parse_obj_file_source(read_obj_file_source(path)?)
}

pub(crate) fn parse_obj_file_source(source: ObjFileSource) -> Result<Mesh3D, String> {
    let mut materials = HashMap::new();
    let mut total_library_bytes = 0usize;
    for library_path in &source.material_paths {
        let library =
            crate::core::project_migration::read_bounded_text_file(library_path, 16 * 1024 * 1024)
                .map_err(|error| {
                    format!(
                        "Could not read OBJ material library '{}': {error}",
                        library_path.display()
                    )
                })?;
        total_library_bytes = total_library_bytes
            .checked_add(library.len())
            .filter(|&size| size <= 128 * 1024 * 1024)
            .ok_or_else(|| "Combined OBJ material libraries exceed 128 MiB".to_string())?;
        for (name, mut material) in parse_mtl_str(&library)? {
            if let Some(texture) = material.albedo_texture.as_mut() {
                let texture_path = library_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(&*texture);
                *texture = texture_path.to_string_lossy().into_owned();
            }
            materials.insert(name, material);
        }
    }
    parse_obj_str_with_materials(&source.content, &materials)
}

fn obj_material_library_names(content: &str) -> Result<Vec<String>, String> {
    let mut libraries = Vec::new();
    for line in content.lines() {
        let content = line.split('#').next().unwrap_or_default().trim();
        let mut parts = content.split_whitespace();
        if parts.next() != Some("mtllib") {
            continue;
        }
        let rest = content.strip_prefix("mtllib").unwrap_or_default().trim();
        let rest = rest.trim();
        if let Some(quoted) = rest.strip_prefix('"').and_then(|rest| rest.split_once('"')) {
            push_obj_library_name(&mut libraries, quoted.0)?;
        } else {
            for name in rest.split_whitespace() {
                push_obj_library_name(&mut libraries, name)?;
            }
        }
    }
    Ok(libraries)
}

fn push_obj_library_name(libraries: &mut Vec<String>, name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("OBJ material library path is empty".into());
    }
    if name.len() > MAX_OBJ_MATERIAL_PATH_BYTES {
        return Err("OBJ material library path exceeds 4,096 bytes".into());
    }
    if libraries.len() >= MAX_OBJ_MATERIAL_LIBRARIES {
        return Err("OBJ file references more than 1,024 material libraries".into());
    }
    libraries
        .try_reserve(1)
        .map_err(|error| format!("Could not reserve OBJ material library list: {error}"))?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(name.len())
        .map_err(|error| format!("Could not reserve OBJ material library path: {error}"))?;
    owned.push_str(name);
    libraries.push(owned);
    Ok(())
}

fn parse_mtl_str(content: &str) -> Result<HashMap<String, PbrMaterial3D>, String> {
    let mut materials = HashMap::new();
    let mut current: Option<PbrMaterial3D> = None;
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else { continue };
        match tag.to_ascii_lowercase().as_str() {
            "newmtl" => {
                if let Some(material) = current.take() {
                    materials.insert(material.name.clone(), material);
                }
                let name = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return Err("OBJ MTL newmtl directive has no name".to_string());
                }
                current = Some(PbrMaterial3D {
                    name,
                    albedo_color: [0.8, 0.8, 0.8, 1.0],
                    ..Default::default()
                });
            }
            "kd" | "ke" => {
                let values = parse_mtl_values(parts, tag, 3)?;
                if let Some(material) = current.as_mut() {
                    if tag.eq_ignore_ascii_case("kd") {
                        material.albedo_color[..3].copy_from_slice(&values);
                    } else {
                        material.emission.copy_from_slice(&values);
                    }
                }
            }
            "d" | "tr" => {
                let values = parse_mtl_values(parts, tag, 1)?;
                if let Some(material) = current.as_mut() {
                    material.albedo_color[3] = if tag.eq_ignore_ascii_case("tr") {
                        1.0 - values[0]
                    } else {
                        values[0]
                    }
                    .clamp(0.0, 1.0);
                    material.alpha_mode = "BLEND".to_string();
                }
            }
            "ns" => {
                let values = parse_mtl_values(parts, tag, 1)?;
                if values[0] < 0.0 {
                    return Err("OBJ MTL Ns value must be non-negative".to_string());
                }
                if let Some(material) = current.as_mut() {
                    material.roughness = (2.0 / (values[0] + 2.0)).sqrt().clamp(0.0, 1.0);
                }
            }
            "map_kd" => {
                let texture = line[tag.len()..].trim();
                let texture = texture
                    .strip_prefix('"')
                    .and_then(|value| value.split_once('"').map(|(name, _)| name))
                    .or_else(|| texture.split_whitespace().last())
                    .ok_or_else(|| "OBJ MTL map_Kd directive has no filename".to_string())?;
                if let Some(material) = current.as_mut() {
                    material.albedo_texture = Some(texture.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(material) = current {
        materials.insert(material.name.clone(), material);
    }
    Ok(materials)
}

fn parse_mtl_values<'a>(
    parts: impl Iterator<Item = &'a str>,
    directive: &str,
    count: usize,
) -> Result<Vec<f32>, String> {
    let values = parts
        .take(count)
        .map(|value| {
            value
                .parse::<f32>()
                .map_err(|_| format!("invalid OBJ MTL {directive} value"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != count || values.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "OBJ MTL {directive} requires {count} finite values"
        ));
    }
    Ok(values)
}
