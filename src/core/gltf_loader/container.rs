use super::*;

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
    if total_length < 12 {
        return Err("Invalid GLB container length".to_string());
    }
    if bytes.len() != total_length {
        return Err(if bytes.len() < total_length {
            "Truncated GLB container".to_string()
        } else {
            "GLB container has trailing data beyond its declared length".to_string()
        });
    }

    // 2. Chunks
    let mut offset = 12;
    let mut json_bytes: Option<&[u8]> = None;
    let mut bin_bytes: Option<&[u8]> = None;

    while offset < total_length {
        if total_length - offset < 8 {
            return Err("Incomplete GLB chunk header".to_string());
        }
        let chunk_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        if !chunk_len.is_multiple_of(4) {
            return Err("GLB chunk length is not 4-byte aligned".into());
        }
        if offset == 12 && chunk_type != 0x4E4F534A {
            return Err("The first GLB chunk must contain JSON".into());
        }
        offset += 8;

        let Some(chunk_end) = offset.checked_add(chunk_len) else {
            return Err("GLB chunk length overflows address space".to_string());
        };
        if chunk_end > total_length {
            return Err("GLB chunk length exceeds file bounds".to_string());
        }

        let chunk_data = &bytes[offset..chunk_end];
        match chunk_type {
            0x4E4F534A => {
                // "JSON"
                if json_bytes.is_some() {
                    return Err("GLB contains multiple JSON chunks".into());
                }
                json_bytes = Some(chunk_data);
            }
            0x004E4942 => {
                // "BIN\0"
                if bin_bytes.is_some() {
                    return Err("GLB contains multiple BIN chunks".into());
                }
                bin_bytes = Some(chunk_data);
            }
            _ => {
                // Other extension chunks can be skipped
            }
        }
        offset = chunk_end;
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
    parse_gltf_json_with_buffers(json_data, &[binary_payload])
}
