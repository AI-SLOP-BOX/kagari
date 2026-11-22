use super::ops::{apply_polygon_boolean, BooleanOp};
use crate::core::mask::point_in_polygon;

/// Vector Fill Rule (EvenOdd / NonZero) for compound multi-contour shapes with holes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Compound 2D Vector Shape consisting of multiple closed polygon contours (e.g. outer loop + holes).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CompoundShape2D {
    pub contours: Vec<Vec<[f32; 2]>>,
    pub fill_rule: FillRule,
}

impl CompoundShape2D {
    pub fn from_single_polygon(poly: Vec<[f32; 2]>) -> Self {
        Self {
            contours: vec![poly],
            fill_rule: FillRule::NonZero,
        }
    }

    /// Evaluates whether a point is filled according to the shape's fill rule.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        match self.fill_rule {
            FillRule::EvenOdd => {
                let mut inside = false;
                for contour in &self.contours {
                    if point_in_polygon(x, y, contour) {
                        inside = !inside;
                    }
                }
                inside
            }
            FillRule::NonZero => {
                if self.contours.is_empty() {
                    return false;
                }
                let in_outer = point_in_polygon(x, y, &self.contours[0]);
                if !in_outer {
                    return false;
                }
                for hole in self.contours.iter().skip(1) {
                    if point_in_polygon(x, y, hole) {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Applies a Boolean operation with another compound shape.
    pub fn apply_boolean(&self, other: &CompoundShape2D, op: BooleanOp) -> CompoundShape2D {
        let mut result_contours = Vec::new();

        for c_a in &self.contours {
            for c_b in &other.contours {
                let res = apply_polygon_boolean(c_a, c_b, op);
                result_contours.extend(res);
            }
        }

        CompoundShape2D {
            contours: result_contours,
            fill_rule: self.fill_rule,
        }
    }
}

/// Procedural Wiggle Paths Modifier (AE Parity: Add > Wiggle Paths).
/// Deforms polygon vertices and edges with smooth or jagged pseudo-random motion.
pub fn apply_wiggle_paths(
    points: &[[f32; 2]],
    size: f32,
    detail: f32,
    seed: u32,
    time: f32,
    is_smooth: bool,
) -> Vec<[f32; 2]> {
    if points.is_empty() || size.abs() < 1e-4 {
        return points.to_vec();
    }

    let mut out = Vec::with_capacity(points.len() * 2);
    let n = points.len();

    for i in 0..n {
        let p = points[i];
        let next_p = points[(i + 1) % n];

        let hash_x =
            ((seed as f32 * 13.37 + i as f32 * 7.19 + time * 2.0).sin() * 43_758.547).fract();
        let hash_y =
            ((seed as f32 * 17.13 + i as f32 * 11.31 + time * 2.0).cos() * 23_421.63).fract();
        let dx = (hash_x - 0.5) * 2.0 * size;
        let dy = (hash_y - 0.5) * 2.0 * size;

        out.push([p[0] + dx, p[1] + dy]);

        let sub_divisions = (detail.round() as usize).clamp(1, 8);
        if sub_divisions > 1 {
            for s in 1..sub_divisions {
                let t = s as f32 / sub_divisions as f32;
                let mid_x = p[0] + (next_p[0] - p[0]) * t;
                let mid_y = p[1] + (next_p[1] - p[1]) * t;

                let sub_hash_x = ((seed as f32 * 19.81 + (i * 10 + s) as f32 * 5.73 + time * 2.5)
                    .sin()
                    * 37821.12)
                    .fract();
                let sub_hash_y = ((seed as f32 * 23.47 + (i * 10 + s) as f32 * 9.27 + time * 2.5)
                    .cos()
                    * 19482.84)
                    .fract();

                let sub_dx = (sub_hash_x - 0.5) * 2.0 * size * if is_smooth { 0.5 } else { 1.0 };
                let sub_dy = (sub_hash_y - 0.5) * 2.0 * size * if is_smooth { 0.5 } else { 1.0 };

                out.push([mid_x + sub_dx, mid_y + sub_dy]);
            }
        }
    }

    out
}

/// Evaluates Shape Repeater Modifier, generating stacked geometric copies with affine transforms.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_shape_repeater(
    contours: &[Vec<[f32; 2]>],
    copies: u32,
    offset: f32,
    transform_pos: [f32; 2],
    transform_scale: [f32; 2],
    transform_rot_deg: f32,
    start_opacity: f32,
    end_opacity: f32,
) -> Vec<(Vec<[f32; 2]>, f32)> {
    if contours.is_empty() || copies == 0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(contours.len() * copies as usize);

    for c in 0..copies {
        let copy_index = c as f32 + offset;
        let progress = if copies > 1 {
            c as f32 / (copies - 1) as f32
        } else {
            0.0
        };
        let opacity = (start_opacity + (end_opacity - start_opacity) * progress).clamp(0.0, 1.0);

        let angle_rad = (transform_rot_deg * copy_index).to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let sx = transform_scale[0].powf(copy_index);
        let sy = transform_scale[1].powf(copy_index);
        let tx = transform_pos[0] * copy_index;
        let ty = transform_pos[1] * copy_index;

        for contour in contours {
            let transformed_contour: Vec<[f32; 2]> = contour
                .iter()
                .map(|&p| {
                    let scaled_x = p[0] * sx;
                    let scaled_y = p[1] * sy;
                    let rot_x = scaled_x * cos_a - scaled_y * sin_a;
                    let rot_y = scaled_x * sin_a + scaled_y * cos_a;
                    [rot_x + tx, rot_y + ty]
                })
                .collect();

            result.push((transformed_contour, opacity));
        }
    }

    result
}
