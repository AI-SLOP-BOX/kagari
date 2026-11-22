use super::parse_materials::initialize_materials_and_textures;
use super::parse_normals::generate_missing_normals;
use super::parse_scene::{
    gltf_mesh_instances, transform_gltf_normal, transform_gltf_position,
    validate_gltf_expanded_size,
};
use super::*;

pub fn parse_gltf_json_with_buffers(
    json_data: &[u8],
    binary_payloads: &[&[u8]],
) -> Result<Mesh3D, String> {
    let root: GltfRootJson =
        serde_json::from_slice(json_data).map_err(|e| format!("Failed to parse glTF JSON: {e}"))?;
    validate_gltf_asset_version(root.asset.as_ref())?;
    validate_gltf_feature_support(&root)?;
    if !root.buffers.is_empty() && root.buffers.len() != binary_payloads.len() {
        return Err("glTF buffer count does not match the supplied binary payloads".to_string());
    }
    for (index, buffer) in root.buffers.iter().enumerate() {
        if buffer.byte_length > binary_payloads[index].len() {
            return Err(format!(
                "glTF binary buffer {index} is shorter than its declared byteLength"
            ));
        }
    }

    if root.meshes.is_empty() {
        return Err("glTF contains no meshes".to_string());
    }
    let mesh_instances = gltf_mesh_instances(&root)?;
    if mesh_instances.is_empty() {
        return Err("glTF selected scene contains no mesh instances".to_string());
    }
    validate_gltf_expanded_size(&root, &mesh_instances)?;

    let mut mesh = Mesh3D::new(
        root.meshes[mesh_instances[0].0]
            .name
            .clone()
            .unwrap_or_else(|| "glTF_Model".to_string()),
    );
    let mut missing_normal_flags = Vec::new();
    // Generated normals can be shared across duplicated UV vertices, but only
    // inside one primitive instance so separately authored hard boundaries stay split.
    let mut normal_smoothing_keys = Vec::new();
    let mut primitive_group = 0usize;

    initialize_materials_and_textures(&root, binary_payloads, &mut mesh)?;

    let read_accessor = |view_idx: usize,
                         offset: usize,
                         element_size: usize,
                         component_alignment: usize,
                         count: usize|
     -> Result<(&[u8], usize), String> {
        let view = root
            .buffer_views
            .get(view_idx)
            .ok_or_else(|| "Invalid buffer view index".to_string())?;
        let binary_payload = binary_payloads
            .get(view.buffer)
            .ok_or_else(|| "Invalid glTF buffer index".to_string())?;
        let stride = view.byte_stride.unwrap_or(element_size);
        if stride < element_size
            || view
                .byte_stride
                .is_some_and(|byte_stride| byte_stride > 252 || byte_stride % 4 != 0)
        {
            return Err("Invalid glTF buffer view byteStride".to_string());
        }
        let span = if count == 0 {
            0
        } else {
            (count - 1)
                .checked_mul(stride)
                .and_then(|offset| offset.checked_add(element_size))
                .ok_or_else(|| "Accessor byte range overflows address space".to_string())?
        };
        let start = view
            .byte_offset
            .checked_add(offset)
            .ok_or_else(|| "Buffer view offset overflows address space".to_string())?;
        if component_alignment == 0
            || !offset.is_multiple_of(component_alignment)
            || !view.byte_offset.is_multiple_of(component_alignment)
            || !start.is_multiple_of(component_alignment)
        {
            return Err("glTF accessor byteOffset is not component-aligned".into());
        }
        let end = start
            .checked_add(span)
            .ok_or_else(|| "Buffer view length overflows address space".to_string())?;
        let view_end = view
            .byte_offset
            .checked_add(view.byte_length)
            .ok_or_else(|| "Buffer view end overflows address space".to_string())?;
        if end > view_end || view_end > binary_payload.len() {
            return Err("Buffer view out of binary payload bounds".to_string());
        }
        Ok((&binary_payload[start..end], stride))
    };

    let materialize_accessor = |accessor: &GltfAccessor,
                                element_size: usize|
     -> Result<Vec<u8>, String> {
        let component_alignment = match accessor.component_type {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            other => return Err(format!("Unsupported glTF accessor component type {other}")),
        };
        let dense_size = accessor
            .count
            .checked_mul(element_size)
            .filter(|&size| size <= MAX_GLTF_EXPANDED_BYTES)
            .ok_or("glTF accessor exceeds the expanded data limit")?;
        let mut dense = Vec::new();
        dense
            .try_reserve_exact(dense_size)
            .map_err(|error| format!("Could not reserve glTF accessor data: {error}"))?;
        dense.resize(dense_size, 0);
        if let Some(view_index) = accessor.buffer_view {
            let (source, stride) = read_accessor(
                view_index,
                accessor.byte_offset,
                element_size,
                component_alignment,
                accessor.count,
            )?;
            for element in 0..accessor.count {
                dense[element * element_size..(element + 1) * element_size]
                    .copy_from_slice(&source[element * stride..element * stride + element_size]);
            }
        }
        if let Some(sparse) = &accessor.sparse {
            if sparse.count == 0 || sparse.count > accessor.count {
                return Err("glTF sparse accessor count exceeds accessor count".into());
            }
            let index_size = match sparse.indices.component_type {
                5121 => 1,
                5123 => 2,
                5125 => 4,
                other => {
                    return Err(format!(
                        "Unsupported glTF sparse index component type {other}"
                    ))
                }
            };
            if sparse.indices.byte_offset % index_size != 0 {
                return Err("glTF sparse index byteOffset is not component-aligned".into());
            }
            let component_size = match accessor.component_type {
                5120 | 5121 => 1,
                5122 | 5123 => 2,
                5125 | 5126 => 4,
                other => {
                    return Err(format!(
                        "Unsupported sparse accessor component type {other}"
                    ))
                }
            };
            if sparse.values.byte_offset % component_size != 0 {
                return Err("glTF sparse value byteOffset is not component-aligned".into());
            }
            for view_index in [sparse.indices.buffer_view, sparse.values.buffer_view] {
                let view = root
                    .buffer_views
                    .get(view_index)
                    .ok_or("Invalid glTF sparse bufferView")?;
                if view.byte_stride.is_some() {
                    return Err("glTF sparse bufferViews cannot define byteStride".into());
                }
            }
            let (index_bytes, _) = read_accessor(
                sparse.indices.buffer_view,
                sparse.indices.byte_offset,
                index_size,
                index_size,
                sparse.count,
            )?;
            let (value_bytes, _) = read_accessor(
                sparse.values.buffer_view,
                sparse.values.byte_offset,
                element_size,
                component_alignment,
                sparse.count,
            )?;
            let mut previous_index = None;
            for sparse_index in 0..sparse.count {
                let offset = sparse_index * index_size;
                let index = match index_size {
                    1 => index_bytes[offset] as usize,
                    2 => u16::from_le_bytes(index_bytes[offset..offset + 2].try_into().unwrap())
                        as usize,
                    _ => u32::from_le_bytes(index_bytes[offset..offset + 4].try_into().unwrap())
                        as usize,
                };
                if index >= accessor.count
                    || previous_index.is_some_and(|previous| index <= previous)
                {
                    return Err("glTF sparse accessor indices must be strictly increasing and within the accessor".into());
                }
                let destination = index * element_size;
                dense[destination..destination + element_size].copy_from_slice(
                    &value_bytes[sparse_index * element_size..(sparse_index + 1) * element_size],
                );
                previous_index = Some(index);
            }
        }
        Ok(dense)
    };

    for &(mesh_index, node_transform) in &mesh_instances {
        let source_mesh = &root.meshes[mesh_index];
        for prim in &source_mesh.primitives {
            let smoothing_group = primitive_group;
            primitive_group = primitive_group.saturating_add(1);
            let base_vertex = u32::try_from(mesh.vertices.len())
                .map_err(|_| "glTF mesh exceeds the 32-bit vertex index limit".to_string())?;

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
            if pos_acc.component_type != 5126
                || pos_acc.normalized
                || pos_acc.accessor_type != "VEC3"
            {
                return Err("POSITION accessor must be FLOAT VEC3".to_string());
            }
            let vertex_count = pos_acc.count;
            let pos_owned = if pos_acc.sparse.is_some() || pos_acc.buffer_view.is_none() {
                Some(materialize_accessor(pos_acc, 12)?)
            } else {
                None
            };
            let (pos_bytes, pos_stride) = if let Some(bytes) = pos_owned.as_deref() {
                (bytes, 12)
            } else {
                read_accessor(
                    pos_acc.buffer_view.unwrap(),
                    pos_acc.byte_offset,
                    12,
                    4,
                    vertex_count,
                )?
            };
            mesh.vertices
                .try_reserve(vertex_count)
                .map_err(|error| format!("Could not reserve glTF vertex data: {error}"))?;
            normal_smoothing_keys
                .try_reserve(vertex_count)
                .map_err(|error| {
                    format!("Could not reserve glTF normal-smoothing data: {error}")
                })?;
            missing_normal_flags
                .try_reserve(vertex_count)
                .map_err(|error| format!("Could not reserve glTF normal flags: {error}"))?;
            for vertex_index in 0..vertex_count {
                let start = vertex_index * pos_stride;
                let chunk = &pos_bytes[start..start + 12];
                let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
                let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
                let z = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    return Err("POSITION accessor contains a non-finite value".to_string());
                }
                let position = transform_gltf_position(node_transform, [x, y, z]);
                if position.iter().any(|value| !value.is_finite()) {
                    return Err("glTF node transform produced a non-finite vertex".to_string());
                }
                mesh.vertices.push(position);
                normal_smoothing_keys.push((
                    smoothing_group,
                    position.map(|coordinate| {
                        if coordinate == 0.0 {
                            0
                        } else {
                            coordinate.to_bits()
                        }
                    }),
                ));
                for (axis, coordinate) in position.iter().copied().enumerate() {
                    mesh.bbox_min[axis] = mesh.bbox_min[axis].min(coordinate);
                    mesh.bbox_max[axis] = mesh.bbox_max[axis].max(coordinate);
                }
            }
            missing_normal_flags
                .resize(mesh.vertices.len(), !prim.attributes.contains_key("NORMAL"));

            // 2. NORMAL attribute (optional)
            if let Some(&norm_acc_idx) = prim.attributes.get("NORMAL") {
                let norm_acc = root
                    .accessors
                    .get(norm_acc_idx)
                    .ok_or_else(|| "Invalid NORMAL accessor".to_string())?;
                if norm_acc.accessor_type != "VEC3" || norm_acc.count != vertex_count {
                    return Err("NORMAL accessor must be VEC3 matching POSITION".to_string());
                }
                let component_size =
                    match (norm_acc.component_type, norm_acc.normalized) {
                        (5126, false) => 4,
                        (5120, true) => 1,
                        (5122, true) => 2,
                        _ => return Err(
                            "NORMAL accessor must use FLOAT or normalized BYTE/SHORT components"
                                .to_string(),
                        ),
                    };
                let element_size = component_size * 3;
                let norm_owned = if norm_acc.sparse.is_some() || norm_acc.buffer_view.is_none() {
                    Some(materialize_accessor(norm_acc, element_size)?)
                } else {
                    None
                };
                let (norm_bytes, norm_stride) = if let Some(bytes) = norm_owned.as_deref() {
                    (bytes, element_size)
                } else {
                    read_accessor(
                        norm_acc.buffer_view.unwrap(),
                        norm_acc.byte_offset,
                        element_size,
                        component_size,
                        vertex_count,
                    )?
                };
                mesh.normals
                    .try_reserve(vertex_count)
                    .map_err(|error| format!("Could not reserve glTF normal data: {error}"))?;
                for vertex_index in 0..vertex_count {
                    let start = vertex_index * norm_stride;
                    let chunk = &norm_bytes[start..start + element_size];
                    let normal = [
                        decode_gltf_component(
                            &chunk[..component_size],
                            norm_acc.component_type,
                            norm_acc.normalized,
                        ),
                        decode_gltf_component(
                            &chunk[component_size..component_size * 2],
                            norm_acc.component_type,
                            norm_acc.normalized,
                        ),
                        decode_gltf_component(
                            &chunk[component_size * 2..],
                            norm_acc.component_type,
                            norm_acc.normalized,
                        ),
                    ];
                    let normal = [
                        normal[0].ok_or_else(|| "Invalid NORMAL component encoding".to_string())?,
                        normal[1].ok_or_else(|| "Invalid NORMAL component encoding".to_string())?,
                        normal[2].ok_or_else(|| "Invalid NORMAL component encoding".to_string())?,
                    ];
                    if normal.iter().any(|value| !value.is_finite()) {
                        return Err("NORMAL accessor contains a non-finite value".to_string());
                    }
                    mesh.normals.push(transform_gltf_normal(node_transform, normal).ok_or_else(
                    || "glTF node transform is singular and cannot transform vertex normals".to_string(),
                )?);
                }
            }
            // Keep array alignment until the complete indexed geometry can be
            // used to generate normals for primitives without NORMAL data.
            while mesh.normals.len() < mesh.vertices.len() {
                mesh.normals.push([0.0; 3]);
            }

            // 3. COLOR_0 is optional; missing values in mixed meshes are white.
            let has_mesh_colors = source_mesh
                .primitives
                .iter()
                .any(|primitive| primitive.attributes.contains_key("COLOR_0"));
            if has_mesh_colors {
                mesh.colors.resize(base_vertex as usize, [1.0; 4]);
                if let Some(&color_accessor_index) = prim.attributes.get("COLOR_0") {
                    let accessor = root
                        .accessors
                        .get(color_accessor_index)
                        .ok_or("Invalid COLOR_0 accessor")?;
                    if !matches!(accessor.accessor_type.as_str(), "VEC3" | "VEC4")
                        || accessor.count != vertex_count
                    {
                        return Err(
                            "COLOR_0 accessor must be VEC3 or VEC4 matching POSITION".into()
                        );
                    }
                    let component_count = if accessor.accessor_type == "VEC3" {
                        3
                    } else {
                        4
                    };
                    let component_size =
                        match (accessor.component_type, accessor.normalized) {
                            (5126, false) => 4,
                            (5121, true) => 1,
                            (5123, true) => 2,
                            _ => return Err(
                                "COLOR_0 accessor must use FLOAT or normalized unsigned components"
                                    .into(),
                            ),
                        };
                    let element_size = component_count * component_size;
                    let owned = if accessor.sparse.is_some() || accessor.buffer_view.is_none() {
                        Some(materialize_accessor(accessor, element_size)?)
                    } else {
                        None
                    };
                    let (bytes, stride) = if let Some(bytes) = owned.as_deref() {
                        (bytes, element_size)
                    } else {
                        read_accessor(
                            accessor.buffer_view.unwrap(),
                            accessor.byte_offset,
                            element_size,
                            component_size,
                            vertex_count,
                        )?
                    };
                    mesh.colors
                        .try_reserve(vertex_count)
                        .map_err(|error| format!("Could not reserve COLOR_0 data: {error}"))?;
                    for vertex in 0..vertex_count {
                        let start = vertex * stride;
                        let data = &bytes[start..start + element_size];
                        let mut color = [1.0; 4];
                        for channel in 0..component_count {
                            color[channel] = decode_gltf_component(
                                &data[channel * component_size..(channel + 1) * component_size],
                                accessor.component_type,
                                accessor.normalized,
                            )
                            .ok_or("Invalid COLOR_0 component encoding")?;
                        }
                        if color
                            .iter()
                            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                        {
                            return Err(
                                "COLOR_0 accessor components must be finite and within 0..1".into(),
                            );
                        }
                        mesh.colors.push(color);
                    }
                } else {
                    mesh.colors.resize(mesh.vertices.len(), [1.0; 4]);
                }
            }

            // 4. Read all supported glTF texture-coordinate sets. Material texture
            // selectors are validated against each primitive below.
            for set in &mut mesh.uv_sets {
                set.resize(base_vertex as usize, [0.0; 2]);
            }
            for channel in 0..=7 {
                let key = format!("TEXCOORD_{channel}");
                let Some(&uv_acc_idx) = prim.attributes.get(&key) else {
                    continue;
                };
                let uv_acc = root
                    .accessors
                    .get(uv_acc_idx)
                    .ok_or_else(|| format!("Invalid {key} accessor"))?;
                if uv_acc.accessor_type != "VEC2" || uv_acc.count != vertex_count {
                    return Err(format!("{key} accessor must be VEC2 matching POSITION"));
                }
                let component_size = match (uv_acc.component_type, uv_acc.normalized) {
                    (5126, false) => 4,
                    (5121, true) => 1,
                    (5123, true) => 2,
                    _ => {
                        return Err(format!(
                            "{key} accessor must use FLOAT or normalized unsigned components"
                        ))
                    }
                };
                let element_size = component_size * 2;
                let owned = if uv_acc.sparse.is_some() || uv_acc.buffer_view.is_none() {
                    Some(materialize_accessor(uv_acc, element_size)?)
                } else {
                    None
                };
                let (bytes, stride) = if let Some(bytes) = owned.as_deref() {
                    (bytes, element_size)
                } else {
                    read_accessor(
                        uv_acc.buffer_view.unwrap(),
                        uv_acc.byte_offset,
                        element_size,
                        component_size,
                        vertex_count,
                    )?
                };
                let mut values = Vec::new();
                values
                    .try_reserve(vertex_count)
                    .map_err(|error| format!("Could not reserve {key} data: {error}"))?;
                for vertex in 0..vertex_count {
                    let start = vertex * stride;
                    let data = &bytes[start..start + element_size];
                    let uv = [
                        decode_gltf_component(
                            &data[..component_size],
                            uv_acc.component_type,
                            uv_acc.normalized,
                        )
                        .ok_or_else(|| format!("Invalid {key} component encoding"))?,
                        decode_gltf_component(
                            &data[component_size..],
                            uv_acc.component_type,
                            uv_acc.normalized,
                        )
                        .ok_or_else(|| format!("Invalid {key} component encoding"))?,
                    ];
                    if uv.iter().any(|value| !value.is_finite()) {
                        return Err(format!("{key} accessor contains a non-finite value"));
                    }
                    values.push(uv);
                }
                if channel == 0 {
                    mesh.uvs.extend(values);
                } else {
                    if mesh.uv_sets.len() <= channel {
                        mesh.uv_sets.resize_with(channel + 1, Vec::new);
                    }
                    mesh.uv_sets[channel].extend(values);
                }
            }
            mesh.uvs.resize(mesh.vertices.len(), [0.0; 2]);
            for set in &mut mesh.uv_sets {
                set.resize(mesh.vertices.len(), [0.0; 2]);
            }

            // 5. Read primitive indices before converting its topology to triangles.
            let mat_idx = prim.material.unwrap_or(0);
            if mat_idx >= mesh.materials.len() {
                return Err(format!(
                "glTF primitive references material {mat_idx}, but only {} materials are defined",
                mesh.materials.len()
            ));
            }
            let texture_channel = mesh.materials[mat_idx].texture_tex_coord;
            if mesh.materials[mat_idx].albedo_texture_index.is_some()
                && !prim
                    .attributes
                    .contains_key(&format!("TEXCOORD_{texture_channel}"))
            {
                return Err(format!(
                    "Material {mat_idx} requires missing TEXCOORD_{texture_channel} data"
                ));
            }
            let mut local_indices = Vec::new();
            if let Some(idx_acc_idx) = prim.indices {
                let idx_acc = root
                    .accessors
                    .get(idx_acc_idx)
                    .ok_or_else(|| "Invalid index accessor".to_string())?;
                if idx_acc.accessor_type != "SCALAR" || idx_acc.normalized {
                    return Err("Index accessor must be SCALAR".to_string());
                }
                local_indices
                    .try_reserve(idx_acc.count)
                    .map_err(|error| format!("Could not reserve glTF index data: {error}"))?;

                match idx_acc.component_type {
                    5123 => {
                        // USHORT (16-bit)
                        let owned = if idx_acc.sparse.is_some() || idx_acc.buffer_view.is_none() {
                            Some(materialize_accessor(idx_acc, 2)?)
                        } else {
                            None
                        };
                        let (raw, stride) = if let Some(bytes) = owned.as_deref() {
                            (bytes, 2)
                        } else {
                            read_accessor(
                                idx_acc.buffer_view.unwrap(),
                                idx_acc.byte_offset,
                                2,
                                2,
                                idx_acc.count,
                            )?
                        };
                        for index in 0..idx_acc.count {
                            let start = index * stride;
                            let chunk = &raw[start..start + 2];
                            let i = u16::from_le_bytes(chunk.try_into().unwrap()) as u32;
                            if i as usize >= vertex_count {
                                return Err(
                                    "Index accessor references a missing vertex".to_string()
                                );
                            }
                            local_indices.push(i);
                        }
                    }
                    5125 => {
                        // UINT (32-bit)
                        let owned = if idx_acc.sparse.is_some() || idx_acc.buffer_view.is_none() {
                            Some(materialize_accessor(idx_acc, 4)?)
                        } else {
                            None
                        };
                        let (raw, stride) = if let Some(bytes) = owned.as_deref() {
                            (bytes, 4)
                        } else {
                            read_accessor(
                                idx_acc.buffer_view.unwrap(),
                                idx_acc.byte_offset,
                                4,
                                4,
                                idx_acc.count,
                            )?
                        };
                        for index in 0..idx_acc.count {
                            let start = index * stride;
                            let chunk = &raw[start..start + 4];
                            let i = u32::from_le_bytes(chunk.try_into().unwrap());
                            if i as usize >= vertex_count {
                                return Err(
                                    "Index accessor references a missing vertex".to_string()
                                );
                            }
                            local_indices.push(i);
                        }
                    }
                    5121 => {
                        // UBYTE (8-bit)
                        let owned = if idx_acc.sparse.is_some() || idx_acc.buffer_view.is_none() {
                            Some(materialize_accessor(idx_acc, 1)?)
                        } else {
                            None
                        };
                        let (raw, stride) = if let Some(bytes) = owned.as_deref() {
                            (bytes, 1)
                        } else {
                            read_accessor(
                                idx_acc.buffer_view.unwrap(),
                                idx_acc.byte_offset,
                                1,
                                1,
                                idx_acc.count,
                            )?
                        };
                        for index in 0..idx_acc.count {
                            let b = raw[index * stride];
                            if b as usize >= vertex_count {
                                return Err(
                                    "Index accessor references a missing vertex".to_string()
                                );
                            }
                            local_indices.push(b as u32);
                        }
                    }
                    other => return Err(format!("Unsupported index component type: {other}")),
                }
            } else {
                // Non-indexed: sequentially generate triangles
                let count = u32::try_from(vertex_count)
                    .map_err(|_| "glTF mesh exceeds the 32-bit vertex index limit".to_string())?;
                local_indices
                    .try_reserve(vertex_count)
                    .map_err(|error| format!("Could not reserve glTF index data: {error}"))?;
                for i in 0..count {
                    local_indices.push(i);
                }
            }

            // The CPU renderer accepts triangles only. Expand valid glTF strip/fan
            // topologies and reject other modes rather than treating their indices
            // as unrelated triangle-list elements.
            let triangle_count = match prim.mode {
                4 => {
                    if !local_indices.len().is_multiple_of(3) {
                        return Err("TRIANGLES primitive index count must be divisible by 3".into());
                    }
                    local_indices.len() / 3
                }
                5 | 6 => local_indices.len().saturating_sub(2),
                other => {
                    return Err(format!(
                    "Unsupported glTF primitive mode {other}; only TRIANGLES, TRIANGLE_STRIP, and TRIANGLE_FAN are supported"
                ));
                }
            };
            let max_output_indices = triangle_count
                .checked_mul(3)
                .ok_or_else(|| "glTF triangle index count overflows address space".to_string())?;
            mesh.indices
                .try_reserve(max_output_indices)
                .map_err(|error| format!("Could not reserve glTF triangle data: {error}"))?;
            let triangle_start = mesh.indices.len();
            match prim.mode {
                4 => {
                    for &index in &local_indices {
                        mesh.indices
                            .push(base_vertex.checked_add(index).ok_or_else(|| {
                                "glTF mesh exceeds the 32-bit vertex index limit".to_string()
                            })?);
                    }
                }
                5 => {
                    for index in 0..triangle_count {
                        let triangle = if index % 2 == 0 {
                            [
                                local_indices[index],
                                local_indices[index + 1],
                                local_indices[index + 2],
                            ]
                        } else {
                            [
                                local_indices[index + 1],
                                local_indices[index],
                                local_indices[index + 2],
                            ]
                        };
                        if triangle[0] == triangle[1]
                            || triangle[1] == triangle[2]
                            || triangle[0] == triangle[2]
                        {
                            continue;
                        }
                        for index in triangle {
                            mesh.indices
                                .push(base_vertex.checked_add(index).ok_or_else(|| {
                                    "glTF mesh exceeds the 32-bit vertex index limit".to_string()
                                })?);
                        }
                    }
                }
                6 => {
                    for index in 0..triangle_count {
                        let triangle = [
                            local_indices[0],
                            local_indices[index + 1],
                            local_indices[index + 2],
                        ];
                        if triangle[0] == triangle[1]
                            || triangle[1] == triangle[2]
                            || triangle[0] == triangle[2]
                        {
                            continue;
                        }
                        for index in triangle {
                            mesh.indices
                                .push(base_vertex.checked_add(index).ok_or_else(|| {
                                    "glTF mesh exceeds the 32-bit vertex index limit".to_string()
                                })?);
                        }
                    }
                }
                _ => unreachable!("primitive mode validated above"),
            }
            let emitted_triangles = (mesh.indices.len() - triangle_start) / 3;
            mesh.material_indices
                .try_reserve(emitted_triangles)
                .map_err(|error| format!("Could not reserve glTF material-index data: {error}"))?;
            for _ in 0..emitted_triangles {
                mesh.material_indices.push(mat_idx);
            }
        }
    }

    if mesh.indices.is_empty() {
        return Err("glTF selected scene contains no triangles".to_string());
    }

    generate_missing_normals(&mut mesh, &missing_normal_flags, &normal_smoothing_keys)?;

    Ok(mesh)
}
