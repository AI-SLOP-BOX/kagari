//! Cinema 4D-style MoGraph Cloner & Effectors Engine.
//!
//! Generates instanced 3D transforms (`CloneInstance` with 4x4 model matrices)
//! from `Linear`, `Grid`, and `Radial` cloners modulated by `Plain`, `Random`,
//! and `Step` effectors with `Spherical` or `Infinite` falloff fields.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialPlane {
    XY,
    XZ,
    YZ,
}

#[derive(Debug, Clone)]
pub enum ClonerMode {
    Linear {
        count: u32,
        offset: [f32; 3],
        rotate_step_deg: [f32; 3],
        scale_step: [f32; 3],
    },
    Grid {
        counts: [u32; 3],
        size: [f32; 3],
    },
    Radial {
        count: u32,
        radius: f32,
        plane: RadialPlane,
        start_angle_deg: f32,
        end_angle_deg: f32,
        align_to_circle: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum FalloffField {
    Infinite,
    Sphere {
        center: [f32; 3],
        radius: f32,
        inner_radius: f32,
    },
}

impl FalloffField {
    #[inline]
    pub fn weight_at(&self, pos: [f32; 3]) -> f32 {
        match *self {
            FalloffField::Infinite => 1.0,
            FalloffField::Sphere {
                center,
                radius,
                inner_radius,
            } => {
                let dx = pos[0] - center[0];
                let dy = pos[1] - center[1];
                let dz = pos[2] - center[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                let r = radius.max(1e-4);
                let inner = inner_radius.clamp(0.0, r);
                if dist <= inner {
                    1.0
                } else if dist >= r {
                    0.0
                } else {
                    let t = 1.0 - (dist - inner) / (r - inner).max(1e-4);
                    t * t * (3.0 - 2.0 * t)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Effector {
    Plain {
        strength: f32,
        position: [f32; 3],
        rotation_deg: [f32; 3],
        scale: [f32; 3],
        falloff: FalloffField,
    },
    Random {
        seed: u64,
        strength: f32,
        position_amp: [f32; 3],
        rotation_amp_deg: [f32; 3],
        scale_amp: f32,
        falloff: FalloffField,
    },
    Step {
        strength: f32,
        scale_end: [f32; 3],
        rotation_end_deg: [f32; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloneInstance {
    pub index: u32,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
    pub weight: f32,
    pub model_matrix: [[f32; 4]; 4],
}

/// Evaluates a MoGraph Cloner + ordered Effector stack into `CloneInstance` transforms.
pub fn evaluate_cloner(mode: &ClonerMode, effectors: &[Effector]) -> Vec<CloneInstance> {
    let mut clones = generate_base_clones(mode);
    let total = clones.len().max(1);

    for (idx, clone) in clones.iter_mut().enumerate() {
        let step_t = if total > 1 {
            idx as f32 / (total - 1) as f32
        } else {
            0.0
        };
        let mut max_w = 0.0_f32;

        for eff in effectors {
            match *eff {
                Effector::Plain {
                    strength,
                    position,
                    rotation_deg,
                    scale,
                    falloff,
                } => {
                    let w = (strength * falloff.weight_at(clone.position)).clamp(0.0, 2.0);
                    max_w = max_w.max(w);
                    for c in 0..3 {
                        clone.position[c] += position[c] * w;
                        clone.rotation_deg[c] += rotation_deg[c] * w;
                        clone.scale[c] *= 1.0 + (scale[c] - 1.0) * w;
                    }
                }
                Effector::Random {
                    seed,
                    strength,
                    position_amp,
                    rotation_amp_deg,
                    scale_amp,
                    falloff,
                } => {
                    let w = (strength * falloff.weight_at(clone.position)).clamp(0.0, 2.0);
                    max_w = max_w.max(w);
                    for c in 0..3 {
                        let r_pos = signed_hash(seed, idx as u64, c as u64);
                        let r_rot = signed_hash(seed, idx as u64, (c + 3) as u64);
                        clone.position[c] += position_amp[c] * r_pos * w;
                        clone.rotation_deg[c] += rotation_amp_deg[c] * r_rot * w;
                    }
                    let r_scl = signed_hash(seed, idx as u64, 6);
                    let s_factor = (1.0 + scale_amp * r_scl * w).max(0.01);
                    for c in 0..3 {
                        clone.scale[c] *= s_factor;
                    }
                }
                Effector::Step {
                    strength,
                    scale_end,
                    rotation_end_deg,
                } => {
                    let w = (strength * step_t).clamp(0.0, 2.0);
                    max_w = max_w.max(w);
                    for c in 0..3 {
                        clone.rotation_deg[c] += rotation_end_deg[c] * w;
                        clone.scale[c] *= 1.0 + (scale_end[c] - 1.0) * w;
                    }
                }
            }
        }
        clone.weight = max_w;
        clone.model_matrix = compose_trs_matrix(clone.position, clone.rotation_deg, clone.scale);
    }

    clones
}

fn generate_base_clones(mode: &ClonerMode) -> Vec<CloneInstance> {
    let mut out = Vec::new();
    match *mode {
        ClonerMode::Linear {
            count,
            offset,
            rotate_step_deg,
            scale_step,
        } => {
            let n = count.min(4096);
            for i in 0..n {
                let f = i as f32;
                let pos = [offset[0] * f, offset[1] * f, offset[2] * f];
                let rot = [
                    rotate_step_deg[0] * f,
                    rotate_step_deg[1] * f,
                    rotate_step_deg[2] * f,
                ];
                let scl = [
                    (1.0 + scale_step[0] * f).max(0.01),
                    (1.0 + scale_step[1] * f).max(0.01),
                    (1.0 + scale_step[2] * f).max(0.01),
                ];
                out.push(CloneInstance {
                    index: i,
                    position: pos,
                    rotation_deg: rot,
                    scale: scl,
                    weight: 0.0,
                    model_matrix: compose_trs_matrix(pos, rot, scl),
                });
            }
        }
        ClonerMode::Grid { counts, size } => {
            let nx = counts[0].min(32);
            let ny = counts[1].min(32);
            let nz = counts[2].min(32);
            let mut idx = 0u32;
            for iz in 0..nz {
                let tz = if nz > 1 {
                    iz as f32 / (nz - 1) as f32 - 0.5
                } else {
                    0.0
                };
                for iy in 0..ny {
                    let ty = if ny > 1 {
                        iy as f32 / (ny - 1) as f32 - 0.5
                    } else {
                        0.0
                    };
                    for ix in 0..nx {
                        let tx = if nx > 1 {
                            ix as f32 / (nx - 1) as f32 - 0.5
                        } else {
                            0.0
                        };
                        let pos = [tx * size[0], ty * size[1], tz * size[2]];
                        out.push(CloneInstance {
                            index: idx,
                            position: pos,
                            rotation_deg: [0.0, 0.0, 0.0],
                            scale: [1.0, 1.0, 1.0],
                            weight: 0.0,
                            model_matrix: compose_trs_matrix(pos, [0.0; 3], [1.0; 3]),
                        });
                        idx += 1;
                    }
                }
            }
        }
        ClonerMode::Radial {
            count,
            radius,
            plane,
            start_angle_deg,
            end_angle_deg,
            align_to_circle,
        } => {
            let n = count.min(4096);
            let span = end_angle_deg - start_angle_deg;
            for i in 0..n {
                let t = if n > 0 { i as f32 / n as f32 } else { 0.0 };
                let deg = start_angle_deg + span * t;
                let (s, c) = deg.to_radians().sin_cos();
                let pos = match plane {
                    RadialPlane::XY => [radius * c, radius * s, 0.0],
                    RadialPlane::XZ => [radius * c, 0.0, radius * s],
                    RadialPlane::YZ => [0.0, radius * c, radius * s],
                };
                let rot = if align_to_circle {
                    match plane {
                        RadialPlane::XY => [0.0, 0.0, deg],
                        RadialPlane::XZ => [0.0, -deg, 0.0],
                        RadialPlane::YZ => [deg, 0.0, 0.0],
                    }
                } else {
                    [0.0, 0.0, 0.0]
                };
                out.push(CloneInstance {
                    index: i,
                    position: pos,
                    rotation_deg: rot,
                    scale: [1.0, 1.0, 1.0],
                    weight: 0.0,
                    model_matrix: compose_trs_matrix(pos, rot, [1.0; 3]),
                });
            }
        }
    }
    out
}

#[inline]
fn signed_hash(seed: u64, idx: u64, channel: u64) -> f32 {
    let mut x = seed
        .wrapping_add(idx.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(channel.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    let unit = (x >> 40) as f32 / ((1u64 << 24) as f32);
    unit * 2.0 - 1.0
}

fn compose_trs_matrix(pos: [f32; 3], rot_deg: [f32; 3], scale: [f32; 3]) -> [[f32; 4]; 4] {
    let (sx, cx) = rot_deg[0].to_radians().sin_cos();
    let (sy, cy) = rot_deg[1].to_radians().sin_cos();
    let (sz, cz) = rot_deg[2].to_radians().sin_cos();

    // Z * Y * X Euler rotation with non-uniform scale
    let m00 = (cy * cz) * scale[0];
    let m01 = (cy * sz) * scale[0];
    let m02 = (-sy) * scale[0];

    let m10 = (sx * sy * cz - cx * sz) * scale[1];
    let m11 = (sx * sy * sz + cx * cz) * scale[1];
    let m12 = (sx * cy) * scale[1];

    let m20 = (cx * sy * cz + sx * sz) * scale[2];
    let m21 = (cx * sy * sz - sx * cz) * scale[2];
    let m22 = (cx * cy) * scale[2];

    [
        [m00, m01, m02, 0.0],
        [m10, m11, m12, 0.0],
        [m20, m21, m22, 0.0],
        [pos[0], pos[1], pos[2], 1.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_cloner_with_spherical_plain_and_random_effector() {
        let mode = ClonerMode::Grid {
            counts: [3, 3, 1],
            size: [200.0, 200.0, 0.0],
        };
        let effectors = [
            Effector::Plain {
                strength: 1.0,
                position: [0.0, 0.0, 50.0],
                rotation_deg: [0.0, 45.0, 0.0],
                scale: [2.0, 2.0, 2.0],
                falloff: FalloffField::Sphere {
                    center: [0.0, 0.0, 0.0],
                    radius: 80.0,
                    inner_radius: 10.0,
                },
            },
            Effector::Random {
                seed: 42,
                strength: 1.0,
                position_amp: [5.0, 5.0, 0.0],
                rotation_amp_deg: [0.0, 0.0, 15.0],
                scale_amp: 0.1,
                falloff: FalloffField::Infinite,
            },
        ];

        let clones = evaluate_cloner(&mode, &effectors);
        assert_eq!(clones.len(), 9);
        // Center clone (index 4 at [0,0,0]) gets full spherical plain lift z=+50
        assert!((clones[4].position[2] - 50.0).abs() < 1e-4);
        assert!(clones[4].scale[0] > 1.7);
        // Corner clone (index 0 at [-100,-100,0], dist ~141 > 80) gets zero spherical lift
        assert!(clones[0].position[2].abs() < 1e-4);
    }

    #[test]
    fn radial_cloner_and_step_effector_progresses_smoothly() {
        let mode = ClonerMode::Radial {
            count: 4,
            radius: 100.0,
            plane: RadialPlane::XZ,
            start_angle_deg: 0.0,
            end_angle_deg: 360.0,
            align_to_circle: true,
        };
        let effectors = [Effector::Step {
            strength: 1.0,
            scale_end: [3.0, 3.0, 3.0],
            rotation_end_deg: [0.0, 0.0, 90.0],
        }];

        let clones = evaluate_cloner(&mode, &effectors);
        assert_eq!(clones.len(), 4);
        assert!((clones[0].scale[0] - 1.0).abs() < 1e-5);
        assert!((clones[3].scale[0] - 3.0).abs() < 1e-5);
    }
}
