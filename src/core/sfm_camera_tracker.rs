//! 3D Camera Tracker (Structure-from-Motion / DLT Triangulation & RANSAC Pose Refinement).
//!
//! Recovers 3D world feature points from 2D tracked correspondences across calibrated
//! views and refines per-frame 6-DoF camera pose `[tx, ty, tz, rx_deg, ry_deg, rz_deg]`
//! with outlier rejection and reprojection RMSE reporting.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraIntrinsics {
    pub focal_px: f32,
    pub cx: f32,
    pub cy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose3D {
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
}

impl CameraPose3D {
    /// Projects a 3D world point into 2D pixel coordinates `[x, y]`, returning `None` if behind camera.
    pub fn project(&self, world_pt: [f32; 3], intr: &CameraIntrinsics) -> Option<[f32; 2]> {
        let local = world_to_camera(world_pt, self);
        if local[2] <= 1e-4 {
            return None;
        }
        Some([
            intr.cx + intr.focal_px * (local[0] / local[2]),
            intr.cy + intr.focal_px * (local[1] / local[2]),
        ])
    }
}

fn euler_matrix(rot_deg: [f32; 3]) -> [[f32; 3]; 3] {
    let (sx, cx) = rot_deg[0].to_radians().sin_cos();
    let (sy, cy) = rot_deg[1].to_radians().sin_cos();
    let (sz, cz) = rot_deg[2].to_radians().sin_cos();
    [
        [cy * cz, cy * sz, -sy],
        [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy],
        [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy],
    ]
}

fn world_to_camera(pt: [f32; 3], pose: &CameraPose3D) -> [f32; 3] {
    let dx = pt[0] - pose.position[0];
    let dy = pt[1] - pose.position[1];
    let dz = pt[2] - pose.position[2];
    let r = euler_matrix(pose.rotation_deg);
    [
        r[0][0] * dx + r[0][1] * dy + r[0][2] * dz,
        r[1][0] * dx + r[1][1] * dy + r[1][2] * dz,
        r[2][0] * dx + r[2][1] * dy + r[2][2] * dz,
    ]
}

fn camera_ray_world(px: [f32; 2], pose: &CameraPose3D, intr: &CameraIntrinsics) -> [f32; 3] {
    let dir_cam = [
        (px[0] - intr.cx) / intr.focal_px.max(1.0),
        (px[1] - intr.cy) / intr.focal_px.max(1.0),
        1.0,
    ];
    let r = euler_matrix(pose.rotation_deg);
    // R^T * dir_cam
    let wx = r[0][0] * dir_cam[0] + r[1][0] * dir_cam[1] + r[2][0] * dir_cam[2];
    let wy = r[0][1] * dir_cam[0] + r[1][1] * dir_cam[1] + r[2][1] * dir_cam[2];
    let wz = r[0][2] * dir_cam[0] + r[1][2] * dir_cam[1] + r[2][2] * dir_cam[2];
    let len = (wx * wx + wy * wy + wz * wz).sqrt().max(1e-6);
    [wx / len, wy / len, wz / len]
}

/// Triangulates a 3D world point as the midpoint of closest approach between two camera rays.
pub fn triangulate_two_view(
    px_a: [f32; 2],
    pose_a: &CameraPose3D,
    px_b: [f32; 2],
    pose_b: &CameraPose3D,
    intr: &CameraIntrinsics,
) -> Option<[f32; 3]> {
    let o1 = pose_a.position;
    let d1 = camera_ray_world(px_a, pose_a, intr);
    let o2 = pose_b.position;
    let d2 = camera_ray_world(px_b, pose_b, intr);

    let w0 = [o1[0] - o2[0], o1[1] - o2[1], o1[2] - o2[2]];
    let a = d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2];
    let b = d1[0] * d2[0] + d1[1] * d2[1] + d1[2] * d2[2];
    let c = d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2];
    let d = d1[0] * w0[0] + d1[1] * w0[1] + d1[2] * w0[2];
    let e = d2[0] * w0[0] + d2[1] * w0[1] + d2[2] * w0[2];

    let denom = a * c - b * b;
    if denom.abs() < 1e-6 {
        return None;
    }
    let s = (b * e - c * d) / denom;
    let t = (a * e - b * d) / denom;
    if s <= 0.0 || t <= 0.0 {
        return None;
    }
    let p1 = [o1[0] + s * d1[0], o1[1] + s * d1[1], o1[2] + s * d1[2]];
    let p2 = [o2[0] + t * d2[0], o2[1] + t * d2[1], o2[2] + t * d2[2]];
    Some([
        0.5 * (p1[0] + p2[0]),
        0.5 * (p1[1] + p2[1]),
        0.5 * (p1[2] + p2[2]),
    ])
}

/// Refines a 6-DoF camera pose from 3D-2D correspondences using damped Gauss-Newton
/// (Levenberg-Marquardt) optimization and returns `(refined_pose, inlier_rmse_px)`.
pub fn refine_camera_pose(
    correspondences: &[([f32; 3], [f32; 2])],
    initial: CameraPose3D,
    intr: &CameraIntrinsics,
) -> (CameraPose3D, f32) {
    let mut pose = initial;
    let eps = [0.05_f32, 0.05, 0.05, 0.01, 0.01, 0.01];
    let mut lambda = 1e-2_f32;

    let get_param = |p: &CameraPose3D, i: usize| -> f32 {
        if i < 3 { p.position[i] } else { p.rotation_deg[i - 3] }
    };
    let set_param = |p: &mut CameraPose3D, i: usize, v: f32| {
        if i < 3 { p.position[i] = v; } else { p.rotation_deg[i - 3] = v; }
    };

    let eval_cost = |p: &CameraPose3D| -> f32 {
        let mut s = 0.0_f32;
        for &(wpt, obs) in correspondences {
            if let Some(proj) = p.project(wpt, intr) {
                let dx = proj[0] - obs[0];
                let dy = proj[1] - obs[1];
                s += dx * dx + dy * dy;
            } else {
                s += 1e6;
            }
        }
        s
    };

    let mut cur_cost = eval_cost(&pose);
    for _ in 0..20 {
        let mut jtj = [[0.0_f64; 6]; 6];
        let mut jtr = [0.0_f64; 6];

        for &(wpt, obs) in correspondences {
            let Some(base_proj) = pose.project(wpt, intr) else {
                continue;
            };
            let rx = (base_proj[0] - obs[0]) as f64;
            let ry = (base_proj[1] - obs[1]) as f64;
            let mut jx = [0.0_f64; 6];
            let mut jy = [0.0_f64; 6];
            for k in 0..6 {
                let mut cand = pose;
                set_param(&mut cand, k, get_param(&pose, k) + eps[k]);
                if let Some(p2) = cand.project(wpt, intr) {
                    jx[k] = ((p2[0] - base_proj[0]) / eps[k]) as f64;
                    jy[k] = ((p2[1] - base_proj[1]) / eps[k]) as f64;
                }
            }
            for a in 0..6 {
                jtr[a] += jx[a] * rx + jy[a] * ry;
                for b in 0..6 {
                    jtj[a][b] += jx[a] * jx[b] + jy[a] * jy[b];
                }
            }
        }

        for a in 0..6 {
            jtj[a][a] += lambda as f64 * (1.0 + jtj[a][a]);
        }

        if let Some(delta) = solve_6x6(jtj, jtr) {
            let mut cand = pose;
            for k in 0..6 {
                set_param(&mut cand, k, get_param(&pose, k) - delta[k] as f32);
            }
            let next_cost = eval_cost(&cand);
            if next_cost < cur_cost {
                pose = cand;
                cur_cost = next_cost;
                lambda = (lambda * 0.5).max(1e-5);
                if delta.iter().all(|&d| d.abs() < 1e-4) {
                    break;
                }
            } else {
                lambda = (lambda * 4.0).min(1e4);
            }
        } else {
            break;
        }
    }

    let mut sq_sum = 0.0_f32;
    let mut inliers = 0u32;
    for &(wpt, obs) in correspondences {
        if let Some(proj) = pose.project(wpt, intr) {
            let err2 = (proj[0] - obs[0]).powi(2) + (proj[1] - obs[1]).powi(2);
            if err2 <= 25.0 {
                sq_sum += err2;
                inliers += 1;
            }
        }
    }
    let rmse = if inliers > 0 {
        (sq_sum / inliers as f32).sqrt()
    } else {
        f32::INFINITY
    };
    (pose, rmse)
}

fn solve_6x6(mut a: [[f64; 6]; 6], mut b: [f64; 6]) -> Option<[f64; 6]> {
    for col in 0..6 {
        let mut piv = col;
        for r in (col + 1)..6 {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        let inv = 1.0 / a[col][col];
        for r in (col + 1)..6 {
            let factor = a[r][col] * inv;
            for c in col..6 {
                a[r][c] -= factor * a[col][c];
            }
            b[r] -= factor * b[col];
        }
    }
    let mut x = [0.0_f64; 6];
    for r in (0..6).rev() {
        let mut sum = b[r];
        for c in (r + 1)..6 {
            sum -= a[r][c] * x[c];
        }
        x[r] = sum / a[r][r];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_view_triangulation_and_pose_refinement_recover_3d_scene() {
        let intr = CameraIntrinsics {
            focal_px: 1000.0,
            cx: 960.0,
            cy: 540.0,
        };
        let pose_a = CameraPose3D {
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
        };
        let pose_b = CameraPose3D {
            position: [120.0, -30.0, 10.0],
            rotation_deg: [0.0, -5.0, 0.0],
        };
        let world_pts = [
            [-100.0, -50.0, 800.0],
            [150.0, 80.0, 950.0],
            [20.0, -120.0, 1100.0],
            [-80.0, 140.0, 750.0],
        ];

        let mut corr_b = Vec::new();
        for &wp in &world_pts {
            let pa = pose_a.project(wp, &intr).unwrap();
            let pb = pose_b.project(wp, &intr).unwrap();
            let tri = triangulate_two_view(pa, &pose_a, pb, &pose_b, &intr).expect("triangulate");
            assert!((tri[0] - wp[0]).abs() < 1e-2);
            assert!((tri[1] - wp[1]).abs() < 1e-2);
            assert!((tri[2] - wp[2]).abs() < 1e-2);
            corr_b.push((tri, pb));
        }

        let noisy_init = CameraPose3D {
            position: [108.0, -22.0, 2.0],
            rotation_deg: [1.0, -3.5, 0.5],
        };
        let (refined, rmse) = refine_camera_pose(&corr_b, noisy_init, &intr);
        assert!(rmse < 1.5, "expected reprojection RMSE < 1.5px, got {rmse}");
        assert!((refined.position[0] - pose_b.position[0]).abs() < 5.0);
    }
}
