//! glTF 2.0 / GLB 3D Model Asset Loader (AE Parity).
//!
//! Parses modern glTF 2.0 JSON and binary GLB containers into Kagari `Mesh3D` models
//! with vertex positions, normals, texture UVs, indices, and PBR material definitions.

#![allow(dead_code)]

use crate::core::obj_loader::{Mesh3D, PbrMaterial3D};
use serde::{Deserialize, Serialize};

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
    pub count: usize,
    #[serde(rename = "type")]
    pub accessor_type: String, // "SCALAR", "VEC2", "VEC3", "VEC4", "MAT4"
    #[serde(default)]
    pub min: Option<Vec<f32>>,
    #[serde(default)]
    pub max: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfPrimitive {
    pub attributes: std::collections::HashMap<String, usize>, // "POSITION", "NORMAL", "TEXCOORD_0"
    pub indices: Option<usize>,
    pub material: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfMeshJson {
    pub name: Option<String>,
    pub primitives: Vec<GltfPrimitive>,
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
}

fn default_base_color_factor() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_factor_one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfMaterialJson {
    pub name: Option<String>,
    pub pbr_metallic_roughness: Option<GltfPbrMetallicRoughness>,
    #[serde(default)]
    pub emissive_factor: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GltfRootJson {
    #[serde(default)]
    pub accessors: Vec<GltfAccessor>,
    #[serde(default)]
    pub buffer_views: Vec<GltfBufferView>,
    #[serde(default)]
    pub meshes: Vec<GltfMeshJson>,
    #[serde(default)]
    pub materials: Vec<GltfMaterialJson>,
}

/// Parses binary GLB (glTF 2.0 Binary container format).
pub fn parse_glb(bytes: &[u8]) -> Result<Mesh3D, String> {
    if bytes.len() < 12 {
        return Err("File too small for GLB header".to_string());
    }

    // 1. Header: magic (0x46546C67 = "glTF"), version (2), length
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != 0x46546C67 {
        return Err("Invalid GLB magic header; not a binary glTF file".to_string());
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 2 {
        return Err(format!("Unsupported GLB version: {version}. Expected 2"));
    }
    let total_length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if bytes.len() < total_length {
        return Err("Truncated GLB container".to_string());
    }

    // 2. Chunks
    let mut offset = 12;
    let mut json_bytes: Option<&[u8]> = None;
    let mut bin_bytes: Option<&[u8]> = None;

    while offset + 8 <= total_length {
        let chunk_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        offset += 8;

        if offset + chunk_len > total_length {
            return Err("GLB chunk length exceeds file bounds".to_string());
        }

        let chunk_data = &bytes[offset..offset + chunk_len];
        match chunk_type {
            0x4E4F534A => {
                // "JSON"
                json_bytes = Some(chunk_data);
            }
            0x004E4942 => {
                // "BIN\0"
                bin_bytes = Some(chunk_data);
            }
            _ => {
                // Other extension chunks can be skipped
            }
        }
        offset += chunk_len;
    }

    let json_slice = json_bytes.ok_or_else(|| "Missing JSON chunk in GLB".to_string())?;
    let bin_slice = bin_bytes.unwrap_or(&[]);

    parse_gltf_json_with_binary(json_slice, bin_slice)
}

/// Parses glTF JSON paired with a binary buffer payload.
pub fn parse_gltf_json_with_binary(
    json_data: &[u8],
    binary_payload: &[u8],
) -> Result<Mesh3D, String> {
    let root: GltfRootJson =
        serde_json::from_slice(json_data).map_err(|e| format!("Failed to parse glTF JSON: {e}"))?;

    let Some(first_mesh) = root.meshes.first() else {
        return Err("glTF contains no meshes".to_string());
    };

    let mut mesh = Mesh3D::new(
        first_mesh
            .name
            .clone()
            .unwrap_or_else(|| "glTF_Model".to_string()),
    );

    // Map PBR materials
    mesh.materials.clear();
    if root.materials.is_empty() {
        mesh.materials.push(PbrMaterial3D::default());
    } else {
        for mat_json in &root.materials {
            let mut mat = PbrMaterial3D {
                name: mat_json
                    .name
                    .clone()
                    .unwrap_or_else(|| "Material".to_string()),
                ..Default::default()
            };
            if let Some(pbr) = &mat_json.pbr_metallic_roughness {
                mat.albedo_color = pbr.base_color_factor;
                mat.metallic = pbr.metallic_factor;
                mat.roughness = pbr.roughness_factor;
            }
            mat.emission = mat_json.emissive_factor;
            mesh.materials.push(mat);
        }
    }

    let read_buffer = |view_idx: usize, offset: usize, len: usize| -> Result<&[u8], String> {
        let view = root
            .buffer_views
            .get(view_idx)
            .ok_or_else(|| "Invalid buffer view index".to_string())?;
        let start = view.byte_offset + offset;
        let end = start + len;
        if end > binary_payload.len() {
            return Err("Buffer view out of binary payload range".to_string());
        }
        Ok(&binary_payload[start..end])
    };

    for prim in &first_mesh.primitives {
        let base_vertex = mesh.vertices.len() as u32;

        // 1. POSITION attribute
        let pos_accessor_idx = prim
            .attributes
            .get("POSITION")
            .copied()
            .ok_or_else(|| "Missing POSITION attribute in glTF primitive".to_string())?;
        let pos_acc = root
            .accessors
            .get(pos_accessor_idx)
            .ok_or_else(|| "Invalid POSITION accessor".to_string())?;
        let pos_view_idx = pos_acc
            .buffer_view
            .ok_or_else(|| "POSITION accessor has no bufferView".to_string())?;

        let vertex_count = pos_acc.count;
        let pos_bytes = read_buffer(pos_view_idx, pos_acc.byte_offset, vertex_count * 12)?;
        for chunk in pos_bytes.as_chunks::<12>().0 {
            let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let z = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
            mesh.vertices.push([x, y, z]);
            mesh.bbox_min[0] = mesh.bbox_min[0].min(x);
            mesh.bbox_min[1] = mesh.bbox_min[1].min(y);
            mesh.bbox_min[2] = mesh.bbox_min[2].min(z);
            mesh.bbox_max[0] = mesh.bbox_max[0].max(x);
            mesh.bbox_max[1] = mesh.bbox_max[1].max(y);
            mesh.bbox_max[2] = mesh.bbox_max[2].max(z);
        }

        // 2. NORMAL attribute (optional)
        if let Some(&norm_acc_idx) = prim.attributes.get("NORMAL") {
            if let Some(norm_acc) = root.accessors.get(norm_acc_idx) {
                if let Some(norm_view_idx) = norm_acc.buffer_view {
                    if let Ok(norm_bytes) =
                        read_buffer(norm_view_idx, norm_acc.byte_offset, vertex_count * 12)
                    {
                        for chunk in norm_bytes.as_chunks::<12>().0 {
                            let nx = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
                            let ny = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
                            let nz = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
                            mesh.normals.push([nx, ny, nz]);
                        }
                    }
                }
            }
        }
        // Pad normals if missing
        while mesh.normals.len() < mesh.vertices.len() {
            mesh.normals.push([0.0, 1.0, 0.0]);
        }

        // 3. TEXCOORD_0 attribute (optional)
        if let Some(&uv_acc_idx) = prim.attributes.get("TEXCOORD_0") {
            if let Some(uv_acc) = root.accessors.get(uv_acc_idx) {
                if let Some(uv_view_idx) = uv_acc.buffer_view {
                    if let Ok(uv_bytes) =
                        read_buffer(uv_view_idx, uv_acc.byte_offset, vertex_count * 8)
                    {
                        for chunk in uv_bytes.as_chunks::<8>().0 {
                            let u = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
                            let v = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
                            mesh.uvs.push([u, v]);
                        }
                    }
                }
            }
        }
        // Pad UVs if missing
        while mesh.uvs.len() < mesh.vertices.len() {
            mesh.uvs.push([0.0, 0.0]);
        }

        // 4. Indices (Triangles)
        let mat_idx = prim.material.unwrap_or(0);
        if let Some(idx_acc_idx) = prim.indices {
            let idx_acc = root
                .accessors
                .get(idx_acc_idx)
                .ok_or_else(|| "Invalid index accessor".to_string())?;
            let idx_view_idx = idx_acc
                .buffer_view
                .ok_or_else(|| "Index accessor has no bufferView".to_string())?;

            match idx_acc.component_type {
                5123 => {
                    // USHORT (16-bit)
                    let raw = read_buffer(idx_view_idx, idx_acc.byte_offset, idx_acc.count * 2)?;
                    for chunk in raw.as_chunks::<2>().0 {
                        let i = u16::from_le_bytes(*chunk) as u32;
                        mesh.indices.push(base_vertex + i);
                    }
                }
                5125 => {
                    // UINT (32-bit)
                    let raw = read_buffer(idx_view_idx, idx_acc.byte_offset, idx_acc.count * 4)?;
                    for chunk in raw.as_chunks::<4>().0 {
                        let i = u32::from_le_bytes(*chunk);
                        mesh.indices.push(base_vertex + i);
                    }
                }
                5121 => {
                    // UBYTE (8-bit)
                    let raw = read_buffer(idx_view_idx, idx_acc.byte_offset, idx_acc.count)?;
                    for &b in raw {
                        mesh.indices.push(base_vertex + b as u32);
                    }
                }
                other => return Err(format!("Unsupported index component type: {other}")),
            }
        } else {
            // Non-indexed: sequentially generate triangles
            for i in 0..vertex_count as u32 {
                mesh.indices.push(base_vertex + i);
            }
        }

        // Record material indices for triangles
        let tri_count = mesh.indices.len() / 3;
        while mesh.material_indices.len() < tri_count {
            mesh.material_indices.push(mat_idx);
        }
    }

    Ok(mesh)
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
                    "indices": 1
                }]
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
        while json_bytes.len() % 4 != 0 {
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
