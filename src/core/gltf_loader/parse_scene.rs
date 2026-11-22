use super::*;

type GltfMatrix = [f32; 16];

fn identity_gltf_matrix() -> GltfMatrix {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn multiply_gltf_matrices(a: GltfMatrix, b: GltfMatrix) -> GltfMatrix {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            result[column * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[column * 4 + k]).sum();
        }
    }
    result
}

fn gltf_node_matrix(node: &GltfNodeJson) -> Result<GltfMatrix, String> {
    if let Some(matrix) = &node.matrix {
        if node.translation.is_some() || node.rotation.is_some() || node.scale.is_some() {
            return Err("glTF node cannot define both matrix and TRS transforms".to_string());
        }
        let matrix: GltfMatrix = matrix
            .as_slice()
            .try_into()
            .map_err(|_| "glTF node matrix must contain 16 values".to_string())?;
        if matrix.iter().any(|value| !value.is_finite()) {
            return Err("glTF node matrix contains a non-finite value".to_string());
        }
        if matrix[3].abs() > 1.0e-6
            || matrix[7].abs() > 1.0e-6
            || matrix[11].abs() > 1.0e-6
            || (matrix[15] - 1.0).abs() > 1.0e-6
        {
            return Err("glTF node matrix must be an affine transform".to_string());
        }
        return Ok(matrix);
    }
    let translation = node.translation.unwrap_or([0.0; 3]);
    let scale = node.scale.unwrap_or([1.0; 3]);
    let mut rotation = node.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    if translation
        .iter()
        .chain(scale.iter())
        .any(|value| !value.is_finite())
        || rotation.iter().any(|value| !value.is_finite())
    {
        return Err("glTF node TRS contains a non-finite value".to_string());
    }
    let length = rotation
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if !length.is_finite() || length <= 1.0e-8 {
        return Err("glTF node rotation quaternion has invalid or zero length".to_string());
    }
    rotation.iter_mut().for_each(|value| *value /= length);
    let [x, y, z, w] = rotation;
    let [sx, sy, sz] = scale;
    let mut matrix = identity_gltf_matrix();
    matrix[0] = (1.0 - 2.0 * (y * y + z * z)) * sx;
    matrix[1] = (2.0 * (x * y + z * w)) * sx;
    matrix[2] = (2.0 * (x * z - y * w)) * sx;
    matrix[4] = (2.0 * (x * y - z * w)) * sy;
    matrix[5] = (1.0 - 2.0 * (x * x + z * z)) * sy;
    matrix[6] = (2.0 * (y * z + x * w)) * sy;
    matrix[8] = (2.0 * (x * z + y * w)) * sz;
    matrix[9] = (2.0 * (y * z - x * w)) * sz;
    matrix[10] = (1.0 - 2.0 * (x * x + y * y)) * sz;
    matrix[12..15].copy_from_slice(&translation);
    Ok(matrix)
}

pub(super) fn transform_gltf_position(matrix: GltfMatrix, point: [f32; 3]) -> [f32; 3] {
    [
        matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12],
        matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13],
        matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14],
    ]
}

pub(super) fn transform_gltf_normal(matrix: GltfMatrix, normal: [f32; 3]) -> Option<[f32; 3]> {
    let a = [matrix[0], matrix[4], matrix[8]];
    let b = [matrix[1], matrix[5], matrix[9]];
    let c = [matrix[2], matrix[6], matrix[10]];
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if !determinant.is_finite() || determinant.abs() <= 1.0e-12 {
        return None;
    }
    let inverse_det = 1.0 / determinant;
    let inverse = [
        (b[1] * c[2] - b[2] * c[1]) * inverse_det,
        (a[2] * c[1] - a[1] * c[2]) * inverse_det,
        (a[1] * b[2] - a[2] * b[1]) * inverse_det,
        (b[2] * c[0] - b[0] * c[2]) * inverse_det,
        (a[0] * c[2] - a[2] * c[0]) * inverse_det,
        (a[2] * b[0] - a[0] * b[2]) * inverse_det,
        (b[0] * c[1] - b[1] * c[0]) * inverse_det,
        (a[1] * c[0] - a[0] * c[1]) * inverse_det,
        (a[0] * b[1] - a[1] * b[0]) * inverse_det,
    ];
    let transformed = [
        inverse[0] * normal[0] + inverse[3] * normal[1] + inverse[6] * normal[2],
        inverse[1] * normal[0] + inverse[4] * normal[1] + inverse[7] * normal[2],
        inverse[2] * normal[0] + inverse[5] * normal[1] + inverse[8] * normal[2],
    ];
    let length = transformed
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    (length.is_finite() && length > 1.0e-8).then(|| transformed.map(|value| value / length))
}

pub(super) fn gltf_mesh_instances(root: &GltfRootJson) -> Result<Vec<(usize, GltfMatrix)>, String> {
    if root.scene.is_some() && root.scenes.is_empty() {
        return Err("glTF selects a scene but defines no scenes".to_string());
    }
    if root.nodes.is_empty() {
        if !root.scenes.is_empty() {
            let scene_index = root.scene.unwrap_or(0);
            let scene = root
                .scenes
                .get(scene_index)
                .ok_or_else(|| format!("glTF references missing scene {scene_index}"))?;
            if !scene.nodes.is_empty() {
                return Err("glTF scene references nodes but the node table is empty".to_string());
            }
            return Err("glTF selected scene contains no mesh instances".to_string());
        }
        return Ok((0..root.meshes.len())
            .map(|index| (index, identity_gltf_matrix()))
            .collect());
    }
    let mut parent_counts = vec![0u8; root.nodes.len()];
    for node in &root.nodes {
        for &child in &node.children {
            let Some(parent_count) = parent_counts.get_mut(child) else {
                return Err(format!("glTF node references missing child node {child}"));
            };
            *parent_count = parent_count.saturating_add(1);
            if *parent_count > 1 {
                return Err(format!("glTF node {child} has multiple parents"));
            }
        }
    }
    let mut visit_state = vec![0u8; root.nodes.len()];
    for start in 0..root.nodes.len() {
        if visit_state[start] != 0 {
            continue;
        }
        let mut graph_stack = vec![(start, false)];
        while let Some((node_index, exiting)) = graph_stack.pop() {
            if exiting {
                visit_state[node_index] = 2;
                continue;
            }
            match visit_state[node_index] {
                1 => {
                    return Err(format!(
                        "glTF node hierarchy contains a cycle at node {node_index}"
                    ));
                }
                2 => continue,
                _ => {}
            }
            visit_state[node_index] = 1;
            graph_stack.push((node_index, true));
            for &child in root.nodes[node_index].children.iter().rev() {
                graph_stack.push((child, false));
            }
        }
    }
    let roots = if root.scenes.is_empty() {
        parent_counts
            .iter()
            .enumerate()
            .filter_map(|(index, &parent_count)| (parent_count == 0).then_some(index))
            .collect::<Vec<_>>()
    } else {
        let scene_index = root.scene.unwrap_or(0);
        root.scenes
            .get(scene_index)
            .ok_or_else(|| format!("glTF references missing scene {scene_index}"))?
            .nodes
            .clone()
    };
    let mut declared_roots = vec![false; root.nodes.len()];
    for &node_index in &roots {
        let Some(parent_count) = parent_counts.get(node_index) else {
            return Err(format!(
                "glTF scene references missing root node {node_index}"
            ));
        };
        if *parent_count != 0 {
            return Err(format!(
                "glTF scene root node {node_index} also has a parent"
            ));
        }
        if std::mem::replace(&mut declared_roots[node_index], true) {
            return Err(format!(
                "glTF scene lists root node {node_index} more than once"
            ));
        }
    }

    let mut instances = Vec::new();
    let mut active = vec![false; root.nodes.len()];
    let mut stack = roots
        .into_iter()
        .rev()
        .map(|index| (index, identity_gltf_matrix(), false))
        .collect::<Vec<_>>();
    while let Some((node_index, parent_matrix, exiting)) = stack.pop() {
        let Some(node) = root.nodes.get(node_index) else {
            return Err(format!("glTF scene references missing node {node_index}"));
        };
        if exiting {
            active[node_index] = false;
            continue;
        }
        if active[node_index] {
            return Err(format!(
                "glTF node hierarchy contains a cycle at node {node_index}"
            ));
        }
        active[node_index] = true;
        let world_matrix = multiply_gltf_matrices(parent_matrix, gltf_node_matrix(node)?);
        if world_matrix.iter().any(|value| !value.is_finite()) {
            return Err("glTF node hierarchy transform is non-finite".to_string());
        }
        if let Some(mesh_index) = node.mesh {
            if mesh_index >= root.meshes.len() {
                return Err(format!("glTF node references missing mesh {mesh_index}"));
            }
            if instances.len() >= MAX_GLTF_MESH_INSTANCES {
                return Err("glTF scene exceeds the 100,000 mesh-instance limit".to_string());
            }
            instances.push((mesh_index, world_matrix));
        }
        stack.push((node_index, world_matrix, true));
        for &child in node.children.iter().rev() {
            stack.push((child, world_matrix, false));
        }
    }
    Ok(instances)
}

pub(super) fn validate_gltf_expanded_size(
    root: &GltfRootJson,
    instances: &[(usize, GltfMatrix)],
) -> Result<(), String> {
    let mut estimated_bytes = 0usize;
    for &(mesh_index, _) in instances {
        let source_mesh = root
            .meshes
            .get(mesh_index)
            .ok_or_else(|| format!("glTF node references missing mesh {mesh_index}"))?;
        for primitive in &source_mesh.primitives {
            let mut max_uv_channel = 0usize;
            for attribute in primitive
                .attributes
                .keys()
                .filter(|name| name.starts_with("TEXCOORD_"))
            {
                let channel = attribute["TEXCOORD_".len()..]
                    .parse::<usize>()
                    .map_err(|_| {
                        format!("Invalid glTF texture-coordinate attribute `{attribute}`")
                    })?;
                if channel > 7 {
                    return Err(format!("Unsupported glTF texture-coordinate attribute `{attribute}`; supported sets are 0 through 7"));
                }
                max_uv_channel = max_uv_channel.max(channel);
            }
            let position_index = primitive
                .attributes
                .get("POSITION")
                .ok_or_else(|| "Missing POSITION attribute in glTF primitive".to_string())?;
            let vertex_count = root
                .accessors
                .get(*position_index)
                .ok_or_else(|| "Invalid POSITION accessor".to_string())?
                .count;
            let source_index_count = if let Some(index) = primitive.indices {
                root.accessors
                    .get(index)
                    .ok_or_else(|| "Invalid index accessor".to_string())?
                    .count
            } else {
                vertex_count
            };
            let triangle_count = match primitive.mode {
                4 => source_index_count / 3,
                5 | 6 => source_index_count.saturating_sub(2),
                mode => {
                    return Err(format!(
                        "Unsupported glTF primitive mode {mode}; only TRIANGLES, TRIANGLE_STRIP, and TRIANGLE_FAN are supported"
                    ));
                }
            };
            let has_vertex_colors = source_mesh
                .primitives
                .iter()
                .any(|candidate| candidate.attributes.contains_key("COLOR_0"));
            let output_indices = triangle_count
                .checked_mul(3)
                .ok_or_else(|| "glTF expanded index count overflows address space".to_string())?;
            let primitive_bytes = vertex_count
                .checked_mul(
                    std::mem::size_of::<[f32; 3]>() * 2
                        + (max_uv_channel + 1) * std::mem::size_of::<[f32; 2]>()
                        + if primitive.attributes.contains_key("NORMAL") {
                            0
                        } else {
                            // Generated-normal position keys plus a conservative
                            // allowance for HashMap buckets during accumulation.
                            std::mem::size_of::<(usize, [u32; 3])>()
                                + std::mem::size_of::<bool>()
                                + std::mem::size_of::<((usize, [u32; 3]), [f32; 3])>() * 2
                        }
                        + if has_vertex_colors {
                            std::mem::size_of::<[f32; 4]>()
                        } else {
                            0
                        },
                )
                .and_then(|bytes| {
                    bytes.checked_add(output_indices.checked_mul(std::mem::size_of::<u32>())?)
                })
                .and_then(|bytes| {
                    bytes.checked_add(triangle_count.checked_mul(std::mem::size_of::<usize>())?)
                })
                .and_then(|bytes| {
                    bytes.checked_add(source_index_count.checked_mul(std::mem::size_of::<u32>())?)
                })
                .ok_or_else(|| "glTF expanded mesh size overflows address space".to_string())?;
            estimated_bytes = estimated_bytes
                .checked_add(primitive_bytes)
                .filter(|&bytes| bytes <= MAX_GLTF_EXPANDED_BYTES)
                .ok_or_else(|| "Expanded glTF scene exceeds the 256 MiB mesh limit".to_string())?;
        }
    }
    Ok(())
}
