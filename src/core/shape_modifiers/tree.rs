use super::operators::{
    apply_pucker_bloat, apply_round_corners, apply_zig_zag, trim_path_vertices,
    PuckerBloatOptions, ZigZagOptions,
};
use crate::core::mask::MaskVertex;

#[derive(Debug, Clone)]
pub struct RenderableShapePath {
    pub vertices: Vec<MaskVertex>,
    pub closed: bool,
    pub fill_color: Option<[f32; 4]>,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShapeTransform {
    pub anchor_point: [f32; 2],
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32, // Degrees
    pub opacity: f32,  // 0.0 to 100.0%
}

impl Default for ShapeTransform {
    fn default() -> Self {
        Self {
            anchor_point: [0.0, 0.0],
            position: [0.0, 0.0],
            scale: [100.0, 100.0],
            rotation: 0.0,
            opacity: 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MergePathsMode {
    #[default]
    Merge,
    Add,
    Subtract,
    Intersect,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShapeContentItem {
    Group {
        name: String,
        items: Vec<ShapeContentItem>,
        transform: ShapeTransform,
    },
    Path {
        name: String,
        vertices: Vec<MaskVertex>,
        closed: bool,
    },
    Rectangle {
        name: String,
        size: [f32; 2],
        position: [f32; 2],
        roundness: f32,
    },
    Ellipse {
        name: String,
        size: [f32; 2],
        position: [f32; 2],
    },
    Fill {
        name: String,
        color: [f32; 4],
        opacity: f32,
    },
    Stroke {
        name: String,
        color: [f32; 4],
        width: f32,
        opacity: f32,
        line_cap: u8,
        line_join: u8,
    },
    TrimPaths {
        name: String,
        start: f32,  // 0.0 to 100.0%
        end: f32,    // 0.0 to 100.0%
        offset: f32, // Degrees or %
    },
    MergePaths {
        name: String,
        mode: MergePathsMode,
    },
    RoundCorners {
        name: String,
        radius: f32,
    },
    PuckerBloat {
        name: String,
        amount: f32,
    },
    ZigZag {
        name: String,
        size: f32,
        ridges: u32,
        smooth: bool,
    },
    Repeater {
        name: String,
        copies: u32,
        offset: f32,
        transform: ShapeTransform,
    },
}

/// Evaluates a recursive tree of ShapeContentItems, sequentially applying geometric modifiers
/// and outputting the final styled vector path list for the software/GPU renderer.
pub fn evaluate_shape_tree(items: &[ShapeContentItem]) -> Vec<RenderableShapePath> {
    let mut current_paths: Vec<(Vec<MaskVertex>, bool)> = Vec::new();
    let mut active_fill: Option<[f32; 4]> = None;
    let mut active_stroke: Option<[f32; 4]> = None;
    let mut active_stroke_w: f32 = 0.0;
    let mut output_paths: Vec<RenderableShapePath> = Vec::new();

    for item in items {
        match item {
            ShapeContentItem::Group {
                items: sub_items,
                transform,
                ..
            } => {
                let sub_rendered = evaluate_shape_tree(sub_items);
                let rad = transform.rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let sx = transform.scale[0] * 0.01;
                let sy = transform.scale[1] * 0.01;

                for mut sp in sub_rendered {
                    for v in &mut sp.vertices {
                        // Apply local group transform: Anchor -> Scale/Rot -> Position
                        let lx = (v.position[0] - transform.anchor_point[0]) * sx;
                        let ly = (v.position[1] - transform.anchor_point[1]) * sy;
                        let rx = lx * cos_r - ly * sin_r + transform.position[0];
                        let ry = lx * sin_r + ly * cos_r + transform.position[1];
                        v.position = [rx, ry];
                    }
                    output_paths.push(sp);
                }
            }
            ShapeContentItem::Path {
                vertices, closed, ..
            } => {
                current_paths.push((vertices.clone(), *closed));
            }
            ShapeContentItem::Rectangle {
                size,
                position,
                roundness,
                ..
            } => {
                let hw = size[0] * 0.5;
                let hh = size[1] * 0.5;
                let raw_rect = vec![
                    MaskVertex::new(position[0] - hw, position[1] - hh),
                    MaskVertex::new(position[0] + hw, position[1] - hh),
                    MaskVertex::new(position[0] + hw, position[1] + hh),
                    MaskVertex::new(position[0] - hw, position[1] + hh),
                ];
                let verts = if *roundness > 0.001 {
                    apply_round_corners(&raw_rect, *roundness, true)
                } else {
                    raw_rect
                };
                current_paths.push((verts, true));
            }
            ShapeContentItem::Ellipse { size, position, .. } => {
                let hw = size[0] * 0.5;
                let hh = size[1] * 0.5;
                let k_x = hw * 0.552_284_8;
                let k_y = hh * 0.552_284_8;

                let mut top = MaskVertex::new(position[0], position[1] - hh);
                top.tangent_in = [-k_x, 0.0];
                top.tangent_out = [k_x, 0.0];

                let mut right = MaskVertex::new(position[0] + hw, position[1]);
                right.tangent_in = [0.0, -k_y];
                right.tangent_out = [0.0, k_y];

                let mut bottom = MaskVertex::new(position[0], position[1] + hh);
                bottom.tangent_in = [k_x, 0.0];
                bottom.tangent_out = [-k_x, 0.0];

                let mut left = MaskVertex::new(position[0] - hw, position[1]);
                left.tangent_in = [0.0, k_y];
                left.tangent_out = [0.0, -k_y];

                current_paths.push((vec![top, right, bottom, left], true));
            }
            ShapeContentItem::TrimPaths {
                start, end, offset, ..
            } => {
                let mut trimmed = Vec::new();
                for (verts, closed) in current_paths {
                    let res = trim_path_vertices(&verts, *start, *end, *offset, closed);
                    trimmed.push((res, false));
                }
                current_paths = trimmed;
            }
            ShapeContentItem::MergePaths { mode, .. } => {
                if current_paths.len() >= 2 {
                    let mut polys: Vec<Vec<[f32; 2]>> = current_paths
                        .iter()
                        .map(|(verts, _)| verts.iter().map(|v| v.position).collect())
                        .collect();

                    let boolean_op = match mode {
                        MergePathsMode::Add | MergePathsMode::Merge => {
                            crate::core::shape_boolean::BooleanOp::Union
                        }
                        MergePathsMode::Subtract => crate::core::shape_boolean::BooleanOp::Subtract,
                        MergePathsMode::Intersect => {
                            crate::core::shape_boolean::BooleanOp::Intersect
                        }
                        MergePathsMode::Exclude => crate::core::shape_boolean::BooleanOp::Exclude,
                    };

                    let mut acc_polys = vec![polys.remove(0)];
                    for next_poly in polys {
                        let mut next_acc = Vec::new();
                        for acc in &acc_polys {
                            let res = crate::core::shape_boolean::apply_polygon_boolean(
                                acc, &next_poly, boolean_op,
                            );
                            next_acc.extend(res);
                        }
                        if next_acc.is_empty() {
                            acc_polys = vec![next_poly];
                        } else {
                            acc_polys = next_acc;
                        }
                    }

                    current_paths = acc_polys
                        .into_iter()
                        .map(|poly| {
                            let verts = poly
                                .into_iter()
                                .map(|p| MaskVertex::new(p[0], p[1]))
                                .collect();
                            (verts, true)
                        })
                        .collect();
                }
            }
            ShapeContentItem::RoundCorners { radius, .. } => {
                for (verts, closed) in &mut current_paths {
                    *verts = apply_round_corners(verts, *radius, *closed);
                }
            }
            ShapeContentItem::PuckerBloat { amount, .. } => {
                let opt = PuckerBloatOptions { amount: *amount };
                for (verts, _) in &mut current_paths {
                    *verts = apply_pucker_bloat(verts, &opt);
                }
            }
            ShapeContentItem::ZigZag {
                size,
                ridges,
                smooth,
                ..
            } => {
                let opt = ZigZagOptions {
                    size: *size,
                    ridges_per_segment: *ridges,
                    smooth: *smooth,
                };
                for (verts, _) in &mut current_paths {
                    *verts = apply_zig_zag(verts, &opt);
                }
            }
            ShapeContentItem::Repeater {
                copies,
                offset,
                transform,
                ..
            } => {
                let mut duplicated = Vec::new();
                if *copies == 0 {
                    current_paths.clear();
                    continue;
                }
                let count = (*copies).min(4096);
                let offset = if offset.is_finite() { *offset } else { 0.0 };
                let rotation = if transform.rotation.is_finite() {
                    transform.rotation
                } else {
                    0.0
                };
                let scale_x = if transform.scale[0].is_finite() {
                    transform.scale[0]
                } else {
                    100.0
                };
                let scale_y = if transform.scale[1].is_finite() {
                    transform.scale[1]
                } else {
                    100.0
                };
                for i in 0..count {
                    let progress = i as f32 + offset;
                    let rad = (rotation * progress).to_radians();
                    let cos_r = rad.cos();
                    let sin_r = rad.sin();
                    let sx = (100.0 + (scale_x - 100.0) * progress) * 0.01;
                    let sy = (100.0 + (scale_y - 100.0) * progress) * 0.01;
                    let tx = if transform.position[0].is_finite() {
                        transform.position[0]
                    } else {
                        0.0
                    } * progress;
                    let ty = if transform.position[1].is_finite() {
                        transform.position[1]
                    } else {
                        0.0
                    } * progress;

                    for (verts, closed) in &current_paths {
                        let mut rep_verts = verts.clone();
                        for v in &mut rep_verts {
                            let lx = (v.position[0] - transform.anchor_point[0]) * sx;
                            let ly = (v.position[1] - transform.anchor_point[1]) * sy;
                            let rx = lx * cos_r - ly * sin_r + tx;
                            let ry = lx * sin_r + ly * cos_r + ty;
                            v.position = [rx, ry];
                        }
                        duplicated.push((rep_verts, *closed));
                    }
                }
                current_paths = duplicated;
            }
            ShapeContentItem::Fill { color, opacity, .. } => {
                let mut c = *color;
                c[3] *= (*opacity * 0.01).clamp(0.0, 1.0);
                active_fill = Some(c);
            }
            ShapeContentItem::Stroke {
                color,
                width,
                opacity,
                ..
            } => {
                let mut c = *color;
                c[3] *= (*opacity * 0.01).clamp(0.0, 1.0);
                active_stroke = Some(c);
                active_stroke_w = *width;
            }
        }
    }

    // Flush current paths with active styling
    for (verts, closed) in current_paths {
        output_paths.push(RenderableShapePath {
            vertices: verts,
            closed,
            fill_color: active_fill,
            stroke_color: active_stroke,
            stroke_width: active_stroke_w,
        });
    }

    output_paths
}
