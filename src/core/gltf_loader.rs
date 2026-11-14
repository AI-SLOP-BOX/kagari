//! glTF 2.0 / GLB 3D Model Asset Loader (AE Parity).
//!
//! Parses modern glTF 2.0 JSON and binary GLB containers into Kagari `Mesh3D` models
//! with vertex positions, normals, texture UVs, indices, and PBR material definitions.

#![allow(dead_code)]

use crate::core::obj_loader::{Mesh3D, PbrMaterial3D};
use serde::{Deserialize, Serialize};

mod container;
mod parse;
mod parse_materials;
mod parse_normals;
mod parse_scene;
mod parse_texture;
pub use container::{parse_glb, parse_gltf_json_with_binary};
pub use parse::parse_gltf_json_with_buffers;
use parse_texture::apply_gltf_sampler;

pub const MAX_GLTF_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_GLTF_EXPANDED_BYTES: usize = 256 * 1024 * 1024;
const MAX_GLTF_MESH_INSTANCES: usize = 100_000;

/// Read a GLB/glTF file without allowing an imported asset to allocate
/// unbounded memory before the parser can validate its contents.
pub fn read_gltf_file(path: impl AsRef<std::path::Path>) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let path = path.as_ref();
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Could not open 3D model {}: {error}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|error| format!("Could not inspect 3D model {}: {error}", path.display()))?
        .len();
    if size > MAX_GLTF_FILE_BYTES {
        return Err(format!(
            "3D model exceeds the {} MiB import limit",
            MAX_GLTF_FILE_BYTES / (1024 * 1024)
        ));
    }
    let initial_capacity = usize::try_from(size)
        .map_err(|_| "3D model size exceeds this platform's address space".to_string())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(initial_capacity)
        .map_err(|error| format!("Could not reserve 3D model import memory: {error}"))?;
    file.take(MAX_GLTF_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read 3D model {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_GLTF_FILE_BYTES {
        return Err(format!(
            "3D model exceeds the {} MiB import limit",
            MAX_GLTF_FILE_BYTES / (1024 * 1024)
        ));
    }
    Ok(bytes)
}

fn read_gltf_buffer_file(
    path: impl AsRef<std::path::Path>,
    expected: usize,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let path = path.as_ref();
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Could not open glTF buffer {}: {error}", path.display()))?;
    let available = file
        .metadata()
        .map_err(|error| format!("Could not inspect glTF buffer {}: {error}", path.display()))?
        .len();
    if available < expected as u64 {
        return Err("glTF binary buffer is shorter than its declared byteLength".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected)
        .map_err(|error| format!("Could not reserve glTF buffer memory: {error}"))?;
    file.take(expected as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read glTF buffer {}: {error}", path.display()))?;
    if bytes.len() != expected {
        return Err("glTF binary buffer changed or ended while it was being read".into());
    }
    Ok(bytes)
}

/// Load a GLB or glTF model, including its declared external or embedded binary
/// buffers. The combined decoded buffer size is bounded by the import limit.
pub fn load_gltf_asset(path: impl AsRef<std::path::Path>) -> Result<Mesh3D, String> {
    let path = path.as_ref();
    let bytes = read_gltf_file(path)?;
    let mut mesh = match path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("glb") => parse_glb(&bytes),
        Some("gltf") => {
            let root: GltfRootJson = serde_json::from_slice(&bytes)
                .map_err(|error| format!("Failed to parse glTF JSON: {error}"))?;
            validate_gltf_asset_version(root.asset.as_ref())?;
            validate_gltf_feature_support(&root)?;
            let mut total_declared_bytes = 0usize;
            let mut binary_buffers = Vec::with_capacity(root.buffers.len());
            for buffer in &root.buffers {
                if buffer.byte_length == 0 {
                    return Err("glTF buffers must declare a nonzero byteLength".into());
                }
                total_declared_bytes = total_declared_bytes
                    .checked_add(buffer.byte_length)
                    .filter(|&size| size <= MAX_GLTF_FILE_BYTES as usize)
                    .ok_or_else(|| {
                        "Combined glTF buffers exceed the 128 MiB import limit".to_string()
                    })?;
                let uri = buffer
                    .uri
                    .as_deref()
                    .ok_or_else(|| "glTF JSON buffer must declare a URI".to_string())?;
                let binary = if let Some(data_uri) = uri.strip_prefix("data:") {
                    let (metadata, payload) = data_uri
                        .split_once(',')
                        .ok_or_else(|| "Malformed glTF data URI".to_string())?;
                    if metadata.to_ascii_lowercase().ends_with(";base64") {
                        let max_encoded = buffer
                            .byte_length
                            .checked_add(2)
                            .and_then(|size| size.checked_div(3))
                            .and_then(|groups| groups.checked_mul(4))
                            .ok_or("glTF data URI size overflows address space")?;
                        if payload.len() > max_encoded {
                            return Err(
                                "glTF data URI exceeds its declared buffer byteLength".into()
                            );
                        }
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD
                            .decode(payload)
                            .map_err(|error| format!("Invalid base64 glTF buffer: {error}"))?
                    } else {
                        let max_encoded = buffer
                            .byte_length
                            .checked_mul(3)
                            .ok_or("glTF data URI size overflows address space")?;
                        if payload.len() > max_encoded {
                            return Err(
                                "glTF data URI exceeds its declared buffer byteLength".into()
                            );
                        }
                        percent_decode_uri_component(payload)?
                    }
                } else {
                    let buffer_path = resolve_gltf_buffer_path(path, uri)?;
                    read_gltf_buffer_file(buffer_path, buffer.byte_length)?
                };
                if binary.len() != buffer.byte_length {
                    return Err("glTF buffer length does not match its declared byteLength".into());
                }
                binary_buffers.push(binary);
            }
            let binary_slices = binary_buffers.iter().map(Vec::as_slice).collect::<Vec<_>>();
            parse_gltf_json_with_buffers(&bytes, &binary_slices)
        }
        _ => Err("3D model must use the .glb or .gltf extension".to_string()),
    }?;
    {
        let json = if path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("glb"))
        {
            glb_json_chunk(&bytes)?
        } else {
            bytes.as_slice()
        };
        let root: GltfRootJson =
            serde_json::from_slice(json).map_err(|e| format!("Failed to parse glTF JSON: {e}"))?;
        for (material_index, material) in mesh.materials.iter_mut().enumerate() {
            let Some(texture_index) = root
                .materials
                .get(material_index)
                .and_then(|m| m.pbr_metallic_roughness.as_ref())
                .and_then(|p| p.base_color_texture.as_ref())
                .map(|info| info.index)
            else {
                continue;
            };
            let image_index = root
                .textures
                .get(texture_index)
                .and_then(|t| t.source)
                .ok_or_else(|| {
                    format!("glTF texture {texture_index} has no supported image source")
                })?;
            material.albedo_texture_index = Some(image_index);
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
            let image = root
                .images
                .get(image_index)
                .ok_or_else(|| format!("Invalid glTF image index {image_index}"))?;
            if let Some(uri) = image.uri.as_deref().filter(|u| !u.starts_with("data:")) {
                material.albedo_texture = Some(
                    resolve_gltf_buffer_path(path, uri)?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    Ok(mesh)
}

pub(crate) fn glb_json_chunk(bytes: &[u8]) -> Result<&[u8], String> {
    if bytes.len() < 12 || u32::from_le_bytes(bytes[0..4].try_into().unwrap()) != 0x46546C67 {
        return Err("Invalid GLB header".into());
    }
    let declared = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if declared != bytes.len() {
        return Err("GLB declared length does not match file length".into());
    }
    let mut offset = 12usize;
    while offset < bytes.len() {
        let header_end = offset
            .checked_add(8)
            .filter(|&end| end <= bytes.len())
            .ok_or("Truncated GLB chunk header")?;
        let length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(bytes[offset + 4..header_end].try_into().unwrap());
        let end = header_end
            .checked_add(length)
            .filter(|&end| end <= bytes.len())
            .ok_or("GLB chunk is out of bounds")?;
        if kind == 0x4E4F534A {
            return Ok(&bytes[header_end..end]);
        }
        offset = end;
    }
    Err("Missing JSON chunk in GLB".into())
}

pub(crate) fn resolve_gltf_buffer_path(
    model_path: &std::path::Path,
    uri: &str,
) -> Result<std::path::PathBuf, String> {
    if uri.starts_with("data:") || uri.contains('?') || uri.contains('#') || uri.contains(':') {
        return Err("Only relative file paths are supported for external glTF buffers".to_string());
    }
    let decoded_uri = percent_decode_uri_component(uri)?;
    let decoded_uri = std::str::from_utf8(&decoded_uri)
        .map_err(|_| "glTF buffer URI is not valid UTF-8".to_string())?;
    let relative_path = std::path::Path::new(decoded_uri);
    if relative_path.is_absolute() {
        return Err("Absolute paths are not allowed for external glTF buffers".to_string());
    }
    Ok(model_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(relative_path))
}

fn percent_decode_uri_component(value: &str) -> Result<Vec<u8>, String> {
    let input = value.as_bytes();
    let mut decoded = Vec::with_capacity(input.len());
    let mut offset = 0;
    while offset < input.len() {
        if input[offset] == b'%' {
            let hex = input
                .get(offset + 1..offset + 3)
                .ok_or_else(|| "Malformed percent escape in glTF buffer URI".to_string())?;
            let hex = std::str::from_utf8(hex)
                .map_err(|_| "Malformed percent escape in glTF buffer URI".to_string())?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| "Malformed percent escape in glTF buffer URI".to_string())?;
            decoded.push(byte);
            offset += 3;
        } else {
            decoded.push(input[offset]);
            offset += 1;
        }
    }
    Ok(decoded)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfBufferView {
    pub buffer: usize,
    #[serde(default)]
    pub byte_offset: usize,
    pub byte_length: usize,
    #[serde(default)]
    pub byte_stride: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfAccessor {
    pub buffer_view: Option<usize>,
    #[serde(default)]
    pub byte_offset: usize,
    pub component_type: u32, // 5120=BYTE, 5121=UBYTE, 5122=SHORT, 5123=USHORT, 5125=UINT, 5126=FLOAT
    #[serde(default)]
    pub normalized: bool,
    pub count: usize,
    #[serde(rename = "type")]
    pub accessor_type: String, // "SCALAR", "VEC2", "VEC3", "VEC4", "MAT4"
    #[serde(default)]
    pub min: Option<Vec<f32>>,
    #[serde(default)]
    pub max: Option<Vec<f32>>,
    #[serde(default)]
    pub sparse: Option<GltfSparseAccessor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfSparseAccessor {
    pub count: usize,
    pub indices: GltfSparseIndices,
    pub values: GltfSparseValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfSparseIndices {
    pub buffer_view: usize,
    #[serde(default)]
    pub byte_offset: usize,
    pub component_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfSparseValues {
    pub buffer_view: usize,
    #[serde(default)]
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfPrimitive {
    pub attributes: std::collections::HashMap<String, usize>, // "POSITION", "NORMAL", "TEXCOORD_0"
    pub indices: Option<usize>,
    pub material: Option<usize>,
    #[serde(default = "default_primitive_mode")]
    pub mode: u32,
    #[serde(default)]
    pub targets: Vec<serde_json::Value>,
}

fn default_primitive_mode() -> u32 {
    4 // TRIANGLES
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfMeshJson {
    pub name: Option<String>,
    pub primitives: Vec<GltfPrimitive>,
    pub weights: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfPbrMetallicRoughness {
    #[serde(default = "default_base_color_factor")]
    pub base_color_factor: [f32; 4],
    #[serde(default = "default_factor_one")]
    pub metallic_factor: f32,
    #[serde(default = "default_factor_one")]
    pub roughness_factor: f32,
    pub base_color_texture: Option<GltfTextureInfo>,
    pub metallic_roughness_texture: Option<GltfTextureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfTextureInfo {
    pub index: usize,
    #[serde(default)]
    pub tex_coord: u32,
    #[serde(default)]
    pub extensions: Option<GltfTextureInfoExtensions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfTextureInfoExtensions {
    #[serde(rename = "KHR_texture_transform")]
    pub texture_transform: Option<GltfTextureTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfTextureTransform {
    pub offset: Option<[f32; 2]>,
    pub rotation: Option<f32>,
    pub scale: Option<[f32; 2]>,
    pub tex_coord: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfTextureJson {
    pub source: Option<usize>,
    pub sampler: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfSamplerJson {
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
    pub wrap_s: Option<u32>,
    pub wrap_t: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfImageJson {
    pub uri: Option<String>,
    pub buffer_view: Option<usize>,
    pub mime_type: Option<String>,
}

fn default_base_color_factor() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_factor_one() -> f32 {
    1.0
}

fn decode_gltf_component(bytes: &[u8], component_type: u32, normalized: bool) -> Option<f32> {
    match (component_type, normalized) {
        (5126, false) if bytes.len() == 4 => Some(f32::from_le_bytes(bytes.try_into().ok()?)),
        (5120, true) if bytes.len() == 1 => Some(((bytes[0] as i8 as f32) / 127.0).max(-1.0)),
        (5121, true) if bytes.len() == 1 => Some(bytes[0] as f32 / 255.0),
        (5122, true) if bytes.len() == 2 => {
            Some(((i16::from_le_bytes(bytes.try_into().ok()?) as f32) / 32767.0).max(-1.0))
        }
        (5123, true) if bytes.len() == 2 => {
            Some(u16::from_le_bytes(bytes.try_into().ok()?) as f32 / 65535.0)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfMaterialJson {
    pub name: Option<String>,
    pub pbr_metallic_roughness: Option<GltfPbrMetallicRoughness>,
    #[serde(default)]
    pub emissive_factor: [f32; 3],
    pub alpha_mode: Option<String>,
    pub alpha_cutoff: Option<f32>,
    pub normal_texture: Option<GltfTextureInfo>,
    pub occlusion_texture: Option<GltfTextureInfo>,
    pub emissive_texture: Option<GltfTextureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfNodeJson {
    pub mesh: Option<usize>,
    pub skin: Option<usize>,
    pub weights: Option<Vec<f32>>,
    #[serde(default)]
    pub children: Vec<usize>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    pub matrix: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfSceneJson {
    #[serde(default)]
    pub nodes: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfBuffer {
    pub byte_length: usize,
    #[serde(default)]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfAssetInfo {
    pub version: Option<String>,
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfRootJson {
    pub asset: Option<GltfAssetInfo>,
    #[serde(default)]
    pub buffers: Vec<GltfBuffer>,
    #[serde(default)]
    pub accessors: Vec<GltfAccessor>,
    #[serde(default)]
    pub buffer_views: Vec<GltfBufferView>,
    #[serde(default)]
    pub meshes: Vec<GltfMeshJson>,
    #[serde(default)]
    pub materials: Vec<GltfMaterialJson>,
    #[serde(default)]
    pub images: Vec<GltfImageJson>,
    #[serde(default)]
    pub textures: Vec<GltfTextureJson>,
    #[serde(default)]
    pub samplers: Vec<GltfSamplerJson>,
    #[serde(default)]
    pub nodes: Vec<GltfNodeJson>,
    #[serde(default)]
    pub scenes: Vec<GltfSceneJson>,
    pub scene: Option<usize>,
    #[serde(default)]
    pub animations: Vec<serde_json::Value>,
    #[serde(default)]
    pub extensions_required: Vec<String>,
}

fn gltf_default_material(name: &str) -> PbrMaterial3D {
    PbrMaterial3D {
        name: name.to_string(),
        albedo_color: default_base_color_factor(),
        metallic: default_factor_one(),
        roughness: default_factor_one(),
        ..Default::default()
    }
}

fn validate_gltf_asset_version(asset: Option<&GltfAssetInfo>) -> Result<(), String> {
    let asset = asset.ok_or("glTF JSON is missing the required asset declaration")?;
    let version = asset
        .version
        .as_deref()
        .ok_or("glTF asset is missing its version")?;
    if version != "2.0" {
        return Err(format!(
            "Unsupported glTF asset version `{version}`; expected 2.0"
        ));
    }
    if let Some(minimum) = asset.min_version.as_deref() {
        let (major, minor) = minimum
            .split_once('.')
            .and_then(|(major, minor)| {
                Some((major.parse::<u32>().ok()?, minor.parse::<u32>().ok()?))
            })
            .ok_or_else(|| format!("Invalid glTF minimum version `{minimum}`"))?;
        if major > 2 || (major == 2 && minor > 0) {
            return Err(format!(
                "Unsupported glTF minimum version `{minimum}`; this importer supports 2.0"
            ));
        }
    }
    Ok(())
}

fn validate_gltf_feature_support(root: &GltfRootJson) -> Result<(), String> {
    if root.accessors.iter().any(|accessor| accessor.count == 0) {
        return Err("glTF accessors must contain at least one element".into());
    }
    for extension in &root.extensions_required {
        if extension != "KHR_texture_transform" {
            return Err(format!("glTF requires unsupported extension `{extension}`"));
        }
    }
    if !root.animations.is_empty() {
        return Err(
            "Animated glTF assets are not supported by the static 3D layer importer".into(),
        );
    }
    if root.nodes.iter().any(|node| node.skin.is_some()) {
        return Err("Skinned glTF assets are not supported by the static 3D layer importer".into());
    }
    if root.nodes.iter().any(|node| node.weights.is_some())
        || root.meshes.iter().any(|mesh| mesh.weights.is_some())
        || root
            .meshes
            .iter()
            .flat_map(|mesh| &mesh.primitives)
            .any(|primitive| !primitive.targets.is_empty())
    {
        return Err("glTF morph targets are not supported by the static 3D layer importer".into());
    }
    if root
        .meshes
        .iter()
        .flat_map(|mesh| &mesh.primitives)
        .any(|primitive| {
            primitive.attributes.keys().any(|attribute| {
                attribute.starts_with("JOINTS_") || attribute.starts_with("WEIGHTS_")
            })
        })
    {
        return Err(
            "glTF vertex skinning attributes are not supported by the static 3D layer importer"
                .into(),
        );
    }
    if root.materials.iter().any(|material| {
        material.normal_texture.is_some()
            || material.occlusion_texture.is_some()
            || material.emissive_texture.is_some()
            || material
                .pbr_metallic_roughness
                .as_ref()
                .is_some_and(|pbr| pbr.metallic_roughness_texture.is_some())
    }) {
        return Err("glTF normal, occlusion, emissive, and metallic-roughness textures are not supported by the 3D renderer".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gltf_synthetic_triangle() {
        let json_str = r#"{
            "asset": {"version": "2.0"},
            "meshes": [{
                "name": "Triangle",
                "primitives": [{
                    "attributes": {"POSITION": 0},
                    "indices": 1,
                    "material": 0
                }]
            }],
            "materials": [{
                "pbrMetallicRoughness": {
                    "metallicFactor": 0.73,
                    "roughnessFactor": 0.21
                }
            }],
            "accessors": [
                {
                    "bufferView": 0,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3"
                },
                {
                    "bufferView": 1,
                    "byteOffset": 0,
                    "componentType": 5123,
                    "count": 3,
                    "type": "SCALAR"
                }
            ],
            "bufferViews": [
                {
                    "buffer": 0,
                    "byteOffset": 0,
                    "byteLength": 36
                },
                {
                    "buffer": 0,
                    "byteOffset": 36,
                    "byteLength": 6
                }
            ]
        }"#;

        // Position binary (3 * 3 * 4 bytes = 36 bytes)
        let mut bin = Vec::new();
        // v0: (0, 0, 0)
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        // v1: (1, 0, 0)
        bin.extend_from_slice(&1.0f32.to_le_bytes());
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        // v2: (0, 1, 0)
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        bin.extend_from_slice(&1.0f32.to_le_bytes());
        bin.extend_from_slice(&0.0f32.to_le_bytes());
        // Indices: 0, 1, 2 (3 * 2 bytes = 6 bytes)
        bin.extend_from_slice(&0u16.to_le_bytes());
        bin.extend_from_slice(&1u16.to_le_bytes());
        bin.extend_from_slice(&2u16.to_le_bytes());

        let mesh = parse_gltf_json_with_binary(json_str.as_bytes(), &bin).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(mesh.bbox_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bbox_max, [1.0, 1.0, 0.0]);
        assert_eq!(mesh.material_indices, vec![0]);
        assert_eq!(mesh.materials[0].metallic, 0.73);
        assert_eq!(mesh.materials[0].roughness, 0.21);
    }

    #[test]
    fn test_parse_glb_synthetic() {
        let json_str = r#"{"asset":{"version":"2.0"},"meshes":[{"name":"Box","primitives":[{"attributes":{"POSITION":0},"indices":1}]}],"accessors":[{"bufferView":0,"byteOffset":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"byteOffset":0,"componentType":5123,"count":3,"type":"SCALAR"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}]}"#;
        let mut bin = Vec::new();
        for _ in 0..3 {
            bin.extend_from_slice(&0.0f32.to_le_bytes());
            bin.extend_from_slice(&1.0f32.to_le_bytes());
            bin.extend_from_slice(&0.0f32.to_le_bytes());
        }
        bin.extend_from_slice(&0u16.to_le_bytes());
        bin.extend_from_slice(&1u16.to_le_bytes());
        bin.extend_from_slice(&2u16.to_le_bytes());

        // Pad JSON to 4-byte alignment
        let mut json_bytes = json_str.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(0x20); // space padding for JSON
        }
        // Pad BIN to 4-byte alignment
        while bin.len() % 4 != 0 {
            bin.push(0x00);
        }

        let total_len = 12 + (8 + json_bytes.len()) + (8 + bin.len());
        let mut glb = Vec::new();
        // Magic
        glb.extend_from_slice(&0x46546C67u32.to_le_bytes());
        // Version 2
        glb.extend_from_slice(&2u32.to_le_bytes());
        // Total Length
        glb.extend_from_slice(&(total_len as u32).to_le_bytes());

        // Chunk 0: JSON (0x4E4F534A)
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
        glb.extend_from_slice(&json_bytes);

        // Chunk 1: BIN (0x004E4942)
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E4942u32.to_le_bytes());
        glb.extend_from_slice(&bin);

        let mesh = parse_glb(&glb).unwrap();
        assert_eq!(mesh.name, "Box");
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }
}
