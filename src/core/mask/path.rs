use super::types::eval_cubic_bezier;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use serde::{Deserialize, Serialize};

// ─── Mask Path ─────────────────────────────────────────────────────────────

/// An animatable Bezier mask path — a list of vertices that can be keyframed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskPath {
    pub vertices: Animatable<Vec<[f32; 2]>>,
    /// Optional incoming/outgoing tangent handles per vertex for cubic Bezier curves.
    /// Format per vertex: ([in_x, in_y], [out_x, out_y]) relative to position.
    #[serde(default)]
    pub tangents: Option<Vec<([f32; 2], [f32; 2])>>,
    pub is_closed: bool,
}

impl MaskPath {
    pub fn new_rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        let verts = vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: None,
            is_closed: true,
        }
    }

    pub fn new_closed(verts: Vec<[f32; 2]>) -> Self {
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: None,
            is_closed: true,
        }
    }

    pub fn new_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        // Approximate circle/ellipse with 4 cubic bezier segments
        // Control point distance k ≈ 0.55228475 * radius
        let kx = rx * 0.552_284_8;
        let ky = ry * 0.552_284_8;
        let verts = vec![
            [cx, cy - ry], // top
            [cx + rx, cy], // right
            [cx, cy + ry], // bottom
            [cx - rx, cy], // left
        ];
        let tangents = vec![
            ([-kx, 0.0], [kx, 0.0]), // top: in left, out right
            ([0.0, -ky], [0.0, ky]), // right: in up, out down
            ([kx, 0.0], [-kx, 0.0]), // bottom: in right, out left
            ([0.0, ky], [0.0, -ky]), // left: in down, out up
        ];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: Some(tangents),
            is_closed: true,
        }
    }

    /// Get evaluated vertices at a given frame.
    /// Uses the standard [`Animatable`] evaluator so Hold/Bezier/Linear
    /// keyframes behave exactly like every other animated property
    /// (a previous hand-rolled linear-only version silently broke Hold).
    pub fn get_vertices(&self, frame: u32) -> Vec<[f32; 2]> {
        match &self.vertices {
            Animatable::Constant(v) => v.clone(),
            Animatable::Animated(kfs) => {
                if kfs.is_empty() {
                    Vec::new()
                } else {
                    self.vertices.value_at(frame)
                }
            }
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering into a caller-provided output vector.
    /// Reuses existing vector allocations to eliminate heap allocation pressure during interactive scrubbing.
    pub fn to_polygon_into(&self, frame: u32, segments_per_edge: u32, out_vec: &mut Vec<[f32; 2]>) {
        out_vec.clear();
        let verts = self.get_vertices(frame);
        if verts.len() < 2 {
            out_vec.extend(verts);
            return;
        }

        let n = verts.len();
        let end = if self.is_closed { n } else { n - 1 };

        out_vec.reserve(end * segments_per_edge as usize + 1);

        for i in 0..end {
            let p0 = verts[i];
            let next_i = (i + 1) % n;
            let p1 = verts[next_i];

            let bezier_control_points = if let Some(tangs) = &self.tangents {
                if tangs.len() == n {
                    let out_t0 = tangs[i].1;
                    let in_t1 = tangs[next_i].0;
                    let c0 = [p0[0] + out_t0[0], p0[1] + out_t0[1]];
                    let c1 = [p1[0] + in_t1[0], p1[1] + in_t1[1]];
                    Some((c0, c1))
                } else {
                    None
                }
            } else {
                None
            };

            for s in 0..segments_per_edge {
                let t = s as f32 / segments_per_edge as f32;
                let pt = if let Some((c0, c1)) = bezier_control_points {
                    eval_cubic_bezier(p0, c0, c1, p1, t)
                } else {
                    [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                };
                out_vec.push(pt);
            }
        }

        if self.is_closed {
            if let Some(&first) = out_vec.first() {
                out_vec.push(first);
            }
        }
    }

    /// Return mask anchor vertices at the given frame (not curve-sampled).
    pub fn vertices_at_frame(&self, frame: u32) -> Vec<[f32; 2]> {
        match &self.vertices {
            Animatable::Constant(v) => v.clone(),
            Animatable::Animated(kfs) => {
                if kfs.is_empty() {
                    Vec::new()
                } else if kfs.len() == 1 || frame <= kfs[0].frame {
                    kfs[0].value.clone()
                } else if let Some(last_kf) = kfs.last() {
                    if frame >= last_kf.frame {
                        last_kf.value.clone()
                    } else {
                        let mut prev = &kfs[0];
                        let mut next = &kfs[0];
                        for kf in kfs {
                            if kf.frame <= frame {
                                prev = kf;
                            }
                            if kf.frame >= frame {
                                next = kf;
                                break;
                            }
                        }
                        if prev.frame == next.frame {
                            prev.value.clone()
                        } else {
                            let t = (frame - prev.frame) as f32 / (next.frame - prev.frame) as f32;
                            prev.value
                                .iter()
                                .zip(next.value.iter())
                                .map(|(&p0, &p1)| {
                                    [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                                })
                                .collect()
                        }
                    }
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Move a single anchor vertex at `frame`, preserving tangents and keyframe structure.
    pub fn set_vertex_at_frame(&mut self, frame: u32, vertex_idx: usize, new_pos: [f32; 2]) {
        match &mut self.vertices {
            Animatable::Constant(verts) => {
                if vertex_idx < verts.len() {
                    verts[vertex_idx] = new_pos;
                }
            }
            Animatable::Animated(keyframes) => {
                if let Some(kf) = keyframes.iter_mut().find(|k| k.frame == frame) {
                    if vertex_idx < kf.value.len() {
                        kf.value[vertex_idx] = new_pos;
                    }
                } else {
                    let mut current = self.vertices_at_frame(frame);
                    if vertex_idx < current.len() {
                        current[vertex_idx] = new_pos;
                        self.vertices.add_keyframe(Keyframe::new(
                            frame,
                            current,
                            InterpolationType::Linear,
                        ));
                    }
                }
            }
        }
    }

    /// Subdivides segment `segment_idx` at parameter `t` in [0, 1] using de Casteljau split,
    /// inserting a new anchor point and updating tangent handles smoothly.
    pub fn insert_vertex_at_frame(
        &mut self,
        frame: u32,
        segment_idx: usize,
        t: f32,
    ) -> Option<usize> {
        let t_clamped = t.clamp(0.01, 0.99);
        let verts = self.vertices_at_frame(frame);
        let n = verts.len();
        if n < 2 || segment_idx >= n {
            return None;
        }

        let next_idx = (segment_idx + 1) % n;
        let p0 = verts[segment_idx];
        let p3 = verts[next_idx];

        let (c0, c1) = if let Some(tangents) = &self.tangents {
            let out_h = tangents.get(segment_idx).map(|t| t.1).unwrap_or([0.0, 0.0]);
            let in_h = tangents.get(next_idx).map(|t| t.0).unwrap_or([0.0, 0.0]);
            (
                [p0[0] + out_h[0], p0[1] + out_h[1]],
                [p3[0] + in_h[0], p3[1] + in_h[1]],
            )
        } else {
            (p0, p3)
        };

        // de Casteljau subdivision at parameter t
        let q0 = [
            p0[0] + (c0[0] - p0[0]) * t_clamped,
            p0[1] + (c0[1] - p0[1]) * t_clamped,
        ];
        let q1 = [
            c0[0] + (c1[0] - c0[0]) * t_clamped,
            c0[1] + (c1[1] - c0[1]) * t_clamped,
        ];
        let q2 = [
            c1[0] + (p3[0] - c1[0]) * t_clamped,
            c1[1] + (p3[1] - c1[1]) * t_clamped,
        ];

        let r0 = [
            q0[0] + (q1[0] - q0[0]) * t_clamped,
            q0[1] + (q1[1] - q0[1]) * t_clamped,
        ];
        let r1 = [
            q1[0] + (q2[0] - q1[0]) * t_clamped,
            q1[1] + (q2[1] - q1[1]) * t_clamped,
        ];

        let mid = [
            r0[0] + (r1[0] - r0[0]) * t_clamped,
            r0[1] + (r1[1] - r0[1]) * t_clamped,
        ];
        let new_idx = segment_idx + 1;

        match &mut self.vertices {
            Animatable::Constant(verts) => {
                verts.insert(new_idx, mid);
            }
            Animatable::Animated(kfs) => {
                for kf in kfs.iter_mut() {
                    if new_idx <= kf.value.len() {
                        kf.value.insert(new_idx, mid);
                    }
                }
            }
        }

        if let Some(tangents) = &mut self.tangents {
            // Update segment_idx out tangent to (q0 - p0)
            if segment_idx < tangents.len() {
                tangents[segment_idx].1 = [q0[0] - p0[0], q0[1] - p0[1]];
            }
            // Insert new vertex tangents: in=(r0 - mid), out=(r1 - mid)
            tangents.insert(
                new_idx,
                (
                    [r0[0] - mid[0], r0[1] - mid[1]],
                    [r1[0] - mid[0], r1[1] - mid[1]],
                ),
            );
            // Update next vertex in tangent to (p3 - q2)
            let updated_next = (new_idx + 1) % tangents.len();
            if updated_next < tangents.len() {
                tangents[updated_next].0 = [q2[0] - p3[0], q2[1] - p3[1]];
            }
        }

        Some(new_idx)
    }

    /// Removes anchor vertex at `vertex_idx` maintaining polygon continuity.
    pub fn remove_vertex_at_frame(&mut self, vertex_idx: usize) -> bool {
        let mut removed = false;
        match &mut self.vertices {
            Animatable::Constant(verts) => {
                if verts.len() > 3 && vertex_idx < verts.len() {
                    verts.remove(vertex_idx);
                    removed = true;
                }
            }
            Animatable::Animated(kfs) => {
                for kf in kfs.iter_mut() {
                    if kf.value.len() > 3 && vertex_idx < kf.value.len() {
                        kf.value.remove(vertex_idx);
                        removed = true;
                    }
                }
            }
        }

        if removed {
            if let Some(tangents) = &mut self.tangents {
                if vertex_idx < tangents.len() {
                    tangents.remove(vertex_idx);
                }
            }
        }

        removed
    }

    /// Sets or updates incoming/outgoing tangent handles with optional smooth collinear link.
    pub fn set_tangents_at_vertex(
        &mut self,
        vertex_idx: usize,
        tangent_in: [f32; 2],
        tangent_out: [f32; 2],
        link_collinear: bool,
    ) {
        let count = match &self.vertices {
            Animatable::Constant(v) => v.len(),
            Animatable::Animated(kfs) => kfs.first().map(|k| k.value.len()).unwrap_or(0),
        };
        if vertex_idx >= count {
            return;
        }

        let t_in = tangent_in;
        let mut t_out = tangent_out;

        if link_collinear {
            // Mirror out handle based on in handle opposite vector
            let len_out = (t_out[0].powi(2) + t_out[1].powi(2)).sqrt().max(1.0);
            let len_in = (t_in[0].powi(2) + t_in[1].powi(2)).sqrt().max(1.0);
            let ratio = len_out / len_in;
            t_out = [-t_in[0] * ratio, -t_in[1] * ratio];
        }

        if self.tangents.is_none() {
            self.tangents = Some(vec![([0.0, 0.0], [0.0, 0.0]); count]);
        }

        if let Some(tangents) = &mut self.tangents {
            if vertex_idx < tangents.len() {
                tangents[vertex_idx] = (t_in, t_out);
            }
        }
    }

    /// Transforms a subset of vertices (scale, rotate, translate) around a bounding box pivot point.
    /// Also rotates and scales tangent handles accordingly.
    pub fn transform_selected_vertices(
        &mut self,
        frame: u32,
        selected_indices: &[usize],
        scale: [f32; 2],
        rotation_deg: f32,
        translation: [f32; 2],
        custom_pivot: Option<[f32; 2]>,
    ) {
        if selected_indices.is_empty() {
            return;
        }

        let mut verts = self.vertices_at_frame(frame);
        if verts.is_empty() {
            return;
        }

        // Calculate pivot as centroid of selected vertices if not provided
        let pivot = custom_pivot.unwrap_or_else(|| {
            let mut sum_x = 0.0f32;
            let mut sum_y = 0.0f32;
            let mut count = 0.0f32;
            for &idx in selected_indices {
                if let Some(p) = verts.get(idx) {
                    sum_x += p[0];
                    sum_y += p[1];
                    count += 1.0;
                }
            }
            if count > 0.0 {
                [sum_x / count, sum_y / count]
            } else {
                [0.0, 0.0]
            }
        });

        let rad = rotation_deg.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        let transform_point = |p: [f32; 2]| -> [f32; 2] {
            let dx = (p[0] - pivot[0]) * scale[0];
            let dy = (p[1] - pivot[1]) * scale[1];
            let rx = dx * cos_r - dy * sin_r;
            let ry = dx * sin_r + dy * cos_r;
            [
                pivot[0] + rx + translation[0],
                pivot[1] + ry + translation[1],
            ]
        };

        let transform_vector = |v: [f32; 2]| -> [f32; 2] {
            let dx = v[0] * scale[0];
            let dy = v[1] * scale[1];
            [dx * cos_r - dy * sin_r, dx * sin_r + dy * cos_r]
        };

        for &idx in selected_indices {
            if idx < verts.len() {
                verts[idx] = transform_point(verts[idx]);
                self.set_vertex_at_frame(frame, idx, verts[idx]);

                if let Some(tangents) = &mut self.tangents {
                    if idx < tangents.len() {
                        let (t_in, t_out) = tangents[idx];
                        tangents[idx] = (transform_vector(t_in), transform_vector(t_out));
                    }
                }
            }
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering.
    /// Returns a flat list of [x, y] pairs. Uses cubic Bezier curves when tangents exist.
    pub fn to_polygon(&self, frame: u32, segments_per_edge: u32) -> Vec<[f32; 2]> {
        let mut result = Vec::new();
        self.to_polygon_into(frame, segments_per_edge, &mut result);
        result
    }
}

