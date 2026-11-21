//! Collision detection functions (Circle-Circle, Circle-Box, Box-Box).

use super::rigidbody::{ColliderShape, ContactManifold, RigidBody};

pub fn detect_collision(
    a: &RigidBody,
    b: &RigidBody,
    idx_a: usize,
    idx_b: usize,
) -> Option<ContactManifold> {
    match (&a.shape, &b.shape) {
        (ColliderShape::Circle { radius: ra }, ColliderShape::Circle { radius: rb }) => {
            let dx = b.position[0] - a.position[0];
            let dy = b.position[1] - a.position[1];
            let dist_sq = dx * dx + dy * dy;
            let radius_sum = ra + rb;

            if dist_sq < radius_sum * radius_sum && dist_sq > 1e-6 {
                let dist = dist_sq.sqrt();
                let normal = [dx / dist, dy / dist];
                let penetration = radius_sum - dist;
                let contact_point = [
                    a.position[0] + normal[0] * ra,
                    a.position[1] + normal[1] * ra,
                ];

                Some(ContactManifold {
                    body_a: idx_a,
                    body_b: idx_b,
                    normal,
                    penetration,
                    contact_point,
                })
            } else {
                None
            }
        }
        (ColliderShape::Circle { radius }, ColliderShape::Box { half_extents }) => {
            detect_circle_box(a, b, *radius, *half_extents, idx_a, idx_b, false)
        }
        (ColliderShape::Box { half_extents }, ColliderShape::Circle { radius }) => {
            detect_circle_box(b, a, *radius, *half_extents, idx_b, idx_a, true)
        }
        (ColliderShape::Box { half_extents: ha }, ColliderShape::Box { half_extents: hb }) => {
            detect_box_box(a, b, *ha, *hb, idx_a, idx_b)
        }
    }
}

pub fn detect_circle_box(
    circle_body: &RigidBody,
    box_body: &RigidBody,
    radius: f32,
    half_extents: [f32; 2],
    idx_circle: usize,
    idx_box: usize,
    flip_normal: bool,
) -> Option<ContactManifold> {
    let rel_x = circle_body.position[0] - box_body.position[0];
    let rel_y = circle_body.position[1] - box_body.position[1];

    let rad = -box_body.rotation_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    let local_x = rel_x * cos - rel_y * sin;
    let local_y = rel_x * sin + rel_y * cos;

    let closest_x = local_x.clamp(-half_extents[0], half_extents[0]);
    let closest_y = local_y.clamp(-half_extents[1], half_extents[1]);

    let dx = local_x - closest_x;
    let dy = local_y - closest_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq < radius * radius && dist_sq > 1e-6 {
        let dist = dist_sq.sqrt();
        let local_nx = dx / dist;
        let local_ny = dy / dist;

        let world_rad = box_body.rotation_deg.to_radians();
        let w_cos = world_rad.cos();
        let w_sin = world_rad.sin();
        let world_nx = local_nx * w_cos - local_ny * w_sin;
        let world_ny = local_nx * w_sin + local_ny * w_cos;

        let normal = if flip_normal {
            [-world_nx, -world_ny]
        } else {
            [world_nx, world_ny]
        };
        let penetration = radius - dist;

        Some(ContactManifold {
            body_a: if flip_normal { idx_box } else { idx_circle },
            body_b: if flip_normal { idx_circle } else { idx_box },
            normal,
            penetration,
            contact_point: circle_body.position,
        })
    } else {
        None
    }
}

pub fn detect_box_box(
    a: &RigidBody,
    b: &RigidBody,
    ha: [f32; 2],
    hb: [f32; 2],
    idx_a: usize,
    idx_b: usize,
) -> Option<ContactManifold> {
    let dx = b.position[0] - a.position[0];
    let dy = b.position[1] - a.position[1];

    let px = (ha[0] + hb[0]) - dx.abs();
    let py = (ha[1] + hb[1]) - dy.abs();

    if px > 0.0 && py > 0.0 {
        if px < py {
            let normal = if dx > 0.0 { [1.0, 0.0] } else { [-1.0, 0.0] };
            Some(ContactManifold {
                body_a: idx_a,
                body_b: idx_b,
                normal,
                penetration: px,
                contact_point: [a.position[0] + normal[0] * ha[0], a.position[1]],
            })
        } else {
            let normal = if dy > 0.0 { [0.0, 1.0] } else { [0.0, -1.0] };
            Some(ContactManifold {
                body_a: idx_a,
                body_b: idx_b,
                normal,
                penetration: py,
                contact_point: [a.position[0], a.position[1] + normal[1] * ha[1]],
            })
        }
    } else {
        None
    }
}
