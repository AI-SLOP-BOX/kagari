/// Resolve a particle against an axis-aligned boundary box.
///
/// `bounds` = [min_x, min_y, max_x, max_y]. On contact the normal velocity
/// component is reflected scaled by `restitution` (0 = dead stop, 1 = perfect
/// bounce) and the tangential component is scaled by `friction`.
/// Returns true when any contact occurred this step.
pub fn resolve_bounds_collision(
    pos: &mut [f32; 2],
    vel: &mut [f32; 2],
    bounds: [f32; 4],
    restitution: f32,
    friction: f32,
) -> bool {
    let (min_x, min_y, max_x, max_y) = (bounds[0], bounds[1], bounds[2], bounds[3]);
    if min_x > max_x || min_y > max_y {
        return false;
    }
    let mut hit = false;

    if pos[0] < min_x {
        pos[0] = min_x;
        if vel[0] < 0.0 {
            vel[0] = -vel[0] * restitution;
            vel[1] *= friction;
            hit = true;
        }
    } else if pos[0] > max_x {
        pos[0] = max_x;
        if vel[0] > 0.0 {
            vel[0] = -vel[0] * restitution;
            vel[1] *= friction;
            hit = true;
        }
    }

    if pos[1] < min_y {
        pos[1] = min_y;
        if vel[1] < 0.0 {
            vel[1] = -vel[1] * restitution;
            vel[0] *= friction;
            hit = true;
        }
    } else if pos[1] > max_y {
        pos[1] = max_y;
        if vel[1] > 0.0 {
            vel[1] = -vel[1] * restitution;
            vel[0] *= friction;
            hit = true;
        }
    }

    hit
}

/// Resolves an elastic collision between two soft-sphere particles of equal
/// mass. On overlap the pair is separated symmetrically along the contact
/// normal and normal-relative velocity is reflected with `restitution`
/// (0 = perfectly inelastic, 1 = perfectly elastic).
///
/// Returns true when a collision was resolved this call.
#[allow(clippy::too_many_arguments)]
pub fn resolve_particle_collision(
    pos_a: &mut [f32; 2],
    vel_a: &mut [f32; 2],
    pos_b: &mut [f32; 2],
    vel_b: &mut [f32; 2],
    radius_sum: f32,
    restitution: f32,
) -> bool {
    if radius_sum <= 0.0 {
        return false;
    }
    let dx = pos_b[0] - pos_a[0];
    let dy = pos_b[1] - pos_a[1];
    let dist_sq = dx * dx + dy * dy;
    if dist_sq >= radius_sum * radius_sum {
        return false;
    }

    // Contact normal; degenerate coincident particles split along +X.
    let dist = dist_sq.sqrt();
    let (nx, ny) = if dist > 1e-6 {
        (dx / dist, dy / dist)
    } else {
        (1.0, 0.0)
    };

    // Symmetric positional correction.
    let overlap = radius_sum - dist;
    let half = overlap * 0.5;
    pos_a[0] -= nx * half;
    pos_a[1] -= ny * half;
    pos_b[0] += nx * half;
    pos_b[1] += ny * half;

    // Impulse along the normal for approaching pairs only.
    let rvx = vel_b[0] - vel_a[0];
    let rvy = vel_b[1] - vel_a[1];
    let vn = rvx * nx + rvy * ny;
    if vn < 0.0 {
        let e = restitution.clamp(0.0, 1.0);
        let j = -(1.0 + e) * vn * 0.5; // equal masses
        vel_a[0] -= nx * j;
        vel_a[1] -= ny * j;
        vel_b[0] += nx * j;
        vel_b[1] += ny * j;
    }
    true
}

/// Convenience O(n²) pass applying [`resolve_particle_collision`] to every
/// pair sharing a uniform `radius` (diameter per particle). Suitable for the
/// few-hundred-particle counts typical of AE-style emitters.
pub fn resolve_pairwise_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    diameter: f32,
    restitution: f32,
) -> u32 {
    let n = positions.len().min(velocities.len());
    let mut hits = 0u32;
    for i in 0..n {
        let (p_head, p_tail) = positions.split_at_mut(i + 1);
        let (v_head, v_tail) = velocities.split_at_mut(i + 1);
        for (bp, bv) in p_tail.iter_mut().zip(v_tail.iter_mut()) {
            if resolve_particle_collision(
                &mut p_head[i],
                &mut v_head[i],
                bp,
                bv,
                diameter,
                restitution,
            ) {
                hits += 1;
            }
        }
    }
    hits
}
