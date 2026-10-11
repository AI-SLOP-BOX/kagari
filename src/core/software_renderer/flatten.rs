use super::*;

#[allow(clippy::too_many_arguments)]
/// Expand every collapsed (`is_collapsed`) PreComp layer into its children,
/// composing parent transform into each child so they render in the parent's
/// coordinate space (AE "Collapse Transformations"). Recursive up to
/// MAX_PRECOMP_DEPTH. When nothing is collapsed this is a cheap clone-free
/// passthrough for callers that need an owned Composition anyway.
pub fn flatten_collapsed(comp: &Composition, frame: u32) -> Composition {
    flatten_collapsed_limited(comp, frame, 0)
}

fn flatten_collapsed_limited(comp: &Composition, frame: u32, depth: u32) -> Composition {
    let mut out = comp.clone();
    if depth >= MAX_PRECOMP_DEPTH {
        return out;
    }
    let mut expanded: Vec<Layer> = Vec::with_capacity(out.layers.len());
    let any_collapsed = out
        .layers
        .iter()
        .any(|l| l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. }));
    if !any_collapsed {
        return out;
    }
    let layers_std = std::mem::take(&mut out.layers);
    for layer in layers_std {
        let (Some(sub_id), true) = (
            match &layer.layer_type {
                LayerType::PreComp { comp_id } => Some(comp_id.clone()),
                _ => None,
            },
            layer.is_collapsed && layer.is_active(frame),
        ) else {
            expanded.push(layer);
            continue;
        };
        let Some(sub) = comp.find_sub_comp(&sub_id) else {
            expanded.push(layer);
            continue;
        };
        // A collapsed PreComp still owns the source-time mapping for its
        // children. The parent transform is evaluated at composition time,
        // while the child content is evaluated at the mapped source frame.
        // Without this split, freeze/reverse/time-stretched collapsed comps
        // silently display the wrong child frame.
        let child_frame = layer.effective_render_frame(frame, comp.fps);
        // Parent transform (already resolves parenting chains)
        let (ppos, pscale, prot, popa) = out.resolve_world_transform(
            &{
                // resolve against ORIGINAL comp so chains above this layer hold
                comp.layers
                    .iter()
                    .find(|l| l.id == layer.id)
                    .cloned()
                    .unwrap_or(layer.clone())
            },
            frame,
        );
        let prad = prot.to_radians();
        let (pc, ps) = (prad.cos(), prad.sin());
        let _pz = if layer.is_3d {
            layer.transform_3d.position.evaluate(frame)[2]
        } else {
            0.0
        };

        // Compose: parent ∘ child (2D affine). Sub-comp coordinates are
        // absolute within the sub frame; map them relative to the sub center
        // onto the parent layer's center before scaling/rotating.
        let flattened_sub = flatten_collapsed_limited(sub, child_frame, depth + 1);
        let sub_cx = flattened_sub.width as f32 * 0.5;
        let sub_cy = flattened_sub.height as f32 * 0.5;
        for child_source in &flattened_sub.layers {
            let mut child = child_source.clone();
            if !child.is_active(child_frame) || !child.visible {
                continue;
            }
            let (cpos, cscale, crot, copa) =
                flattened_sub.resolve_world_transform(&child, child_frame);
            // Compose: parent ∘ child (2D affine)
            let sx = cscale[0] * pscale[0] / 100.0;
            let sy = cscale[1] * pscale[1] / 100.0;
            let rel_x = cpos[0] - sub_cx;
            let rel_y = cpos[1] - sub_cy;
            let lx = rel_x * pscale[0] / 100.0;
            let ly = rel_y * pscale[1] / 100.0;
            let npos = [ppos[0] + lx * pc - ly * ps, ppos[1] + lx * ps + ly * pc];
            let nrot = prot + crot;
            let nopa = (popa / 100.0) * (copa / 100.0) * 100.0;

            child.transform.position = crate::core::property::Animatable::new_constant(npos);
            child.transform.scale =
                crate::core::property::Animatable::new_constant([sx.max(0.001), sy.max(0.001)]);
            child.transform.rotation = crate::core::property::Animatable::new_constant(nrot);
            child.transform.opacity = crate::core::property::Animatable::new_constant(nopa);

            // 3D continuity: cascade full 3D position, rotation and scale into parent space
            if layer.is_3d || child.is_3d {
                let p_pos3d = if layer.is_3d {
                    layer.transform_3d.position.evaluate(frame)
                } else {
                    [ppos[0], ppos[1], 0.0]
                };
                let c_pos3d = if child.is_3d {
                    child.transform_3d.position.evaluate(child_frame)
                } else {
                    [cpos[0], cpos[1], 0.0]
                };
                let p_rot3d = if layer.is_3d {
                    layer.transform_3d.rotation.evaluate(frame)
                } else {
                    [0.0, 0.0, prot]
                };
                let c_rot3d = if child.is_3d {
                    child.transform_3d.rotation.evaluate(child_frame)
                } else {
                    [0.0, 0.0, crot]
                };
                let p_scale3d = if layer.is_3d {
                    layer.transform_3d.scale.evaluate(frame)
                } else {
                    [pscale[0], pscale[1], 100.0]
                };
                let c_scale3d = if child.is_3d {
                    child.transform_3d.scale.evaluate(child_frame)
                } else {
                    [cscale[0], cscale[1], 100.0]
                };

                child.is_3d = true;
                child.transform_3d.position = crate::core::property::Animatable::new_constant([
                    npos[0],
                    npos[1],
                    p_pos3d[2] + c_pos3d[2],
                ]);
                child.transform_3d.rotation = crate::core::property::Animatable::new_constant([
                    p_rot3d[0] + c_rot3d[0],
                    p_rot3d[1] + c_rot3d[1],
                    p_rot3d[2] + c_rot3d[2],
                ]);
                child.transform_3d.scale = crate::core::property::Animatable::new_constant([
                    (p_scale3d[0] * c_scale3d[0] / 100.0).max(0.001),
                    (p_scale3d[1] * c_scale3d[1] / 100.0).max(0.001),
                    (p_scale3d[2] * c_scale3d[2] / 100.0).max(0.001),
                ]);
            }
            expanded.push(child);
        }
    }
    out.layers = expanded;
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn perspective_project_layer(
    cam_fov: f32,
    cam_pos: [f32; 3],
    cam_rot: [f32; 3], // degrees: [rx, ry, rz]
    layer_pos: [f32; 3],
    layer_rot: [f32; 3],   // degrees
    layer_scale: [f32; 2], // percent (100 = no scale)
    layer_width: f32,
    layer_height: f32,
    screen_width: f32,
    screen_height: f32,
) -> Option<[[f32; 4]; 4]> {
    let hw = layer_width * 0.5 * (layer_scale[0] / 100.0);
    let hh = layer_height * 0.5 * (layer_scale[1] / 100.0);

    let corners: [[f32; 3]; 4] = [
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [hw, hh, 0.0],
        [-hw, hh, 0.0],
    ];

    // Euler angles to rotation matrix (XYZ order)
    let to_rad = |d: f32| d * std::f32::consts::PI / 180.0;
    let (rx, ry, rz) = (
        to_rad(layer_rot[0]),
        to_rad(layer_rot[1]),
        to_rad(layer_rot[2]),
    );
    let (crx, srx) = (rx.cos(), rx.sin());
    let (cry, sry) = (ry.cos(), ry.sin());
    let (crz, srz) = (rz.cos(), rz.sin());

    // Rz * Ry * Rx
    let rotate = |p: [f32; 3]| -> [f32; 3] {
        // Rx
        let y1 = p[1] * crx - p[2] * srx;
        let z1 = p[1] * srx + p[2] * crx;
        let x1 = p[0];
        // Ry
        let x2 = x1 * cry + z1 * sry;
        let z2 = -x1 * sry + z1 * cry;
        let y2 = y1;
        // Rz
        [x2 * crz - y2 * srz, x2 * srz + y2 * crz, z2]
    };

    // World-space corners
    let world: Vec<[f32; 3]> = corners
        .iter()
        .map(|c| {
            let r = rotate(*c);
            [
                r[0] + layer_pos[0],
                r[1] + layer_pos[1],
                r[2] + layer_pos[2],
            ]
        })
        .collect();

    // Camera transform (simplified: translate + Z-rotate only)
    let cam_zr = to_rad(cam_rot[2]);
    let (ccrz, ssrz) = (cam_zr.cos(), cam_zr.sin());

    let cam_space: Vec<[f32; 3]> = world
        .iter()
        .map(|c| {
            let dx = c[0] - cam_pos[0];
            let dy = c[1] - cam_pos[1];
            let dz = c[2] - cam_pos[2];
            [dx * ccrz - dy * ssrz, dx * ssrz + dy * ccrz, dz]
        })
        .collect();

    if cam_space.iter().any(|c| c[2] <= 0.1) {
        return None;
    }

    let fov_rad = cam_fov * std::f32::consts::PI / 180.0;
    let focal = (screen_height * 0.5) / (fov_rad * 0.5).tan();

    let mut result = [[0.0f32; 4]; 4];
    for (i, c) in cam_space.iter().enumerate() {
        let z = c[2];
        let sx = screen_width * 0.5 + (c[0] * focal) / z;
        let sy = screen_height * 0.5 - (c[1] * focal) / z;
        let u = if i == 0 || i == 3 { 0.0 } else { 1.0 };
        let v = if i == 0 || i == 1 { 0.0 } else { 1.0 };
        result[i] = [sx, sy, u, v];
    }
    Some(result)
}
