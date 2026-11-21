use std::collections::HashMap;

/// Bound dense particle clusters from turning one simulation frame into an
/// unbounded all-pairs workload. Stable particle order keeps truncation
/// deterministic; sparse scenes retain the full spatial-grid path.
pub const MAX_PAIRWISE_COLLISION_TESTS_PER_STEP: usize = 250_000;
static PAIRWISE_COLLISION_BUDGET_WARNING_EMITTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PairwiseCollisionStats {
    pub(crate) hits: u32,
    pub(crate) pair_tests: usize,
    pub(crate) capped: bool,
}

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

/// Resolve nearby particle pairs using a uniform-grid broad phase.
///
/// Cell width equals the collision diameter, so overlapping particles can
/// only occupy the same or adjacent cells. Candidate indices are sorted before
/// resolution to keep the pair order deterministic.
pub fn resolve_pairwise_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    diameter: f32,
    restitution: f32,
) -> u32 {
    let stats = resolve_pairwise_collisions_with_budget(
        positions,
        velocities,
        diameter,
        restitution,
        MAX_PAIRWISE_COLLISION_TESTS_PER_STEP,
    );
    if stats.capped
        && !PAIRWISE_COLLISION_BUDGET_WARNING_EMITTED
            .swap(true, std::sync::atomic::Ordering::Relaxed)
    {
        log::warn!(
            "Particle collision workload reached the per-step limit of {MAX_PAIRWISE_COLLISION_TESTS_PER_STEP} pair tests; remaining collisions were skipped"
        );
    }
    stats.hits
}

pub(crate) fn resolve_pairwise_collisions_with_budget(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    diameter: f32,
    restitution: f32,
    max_pair_tests: usize,
) -> PairwiseCollisionStats {
    let n = positions.len().min(velocities.len());
    if n < 2 || !diameter.is_finite() || diameter <= 0.0 {
        return PairwiseCollisionStats {
            hits: 0,
            pair_tests: 0,
            capped: false,
        };
    }

    let cell_for = |position: [f32; 2]| -> Option<(i64, i64)> {
        if !position[0].is_finite() || !position[1].is_finite() {
            return None;
        }
        Some((
            (position[0] / diameter).floor() as i64,
            (position[1] / diameter).floor() as i64,
        ))
    };

    // Particle counts may be configured far above the usual scene size. Avoid
    // reserving one hash bucket per particle up front; the grid grows only as
    // occupied cells require it.
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut cells = Vec::with_capacity(n);
    for (index, position) in positions[..n].iter().copied().enumerate() {
        let cell = cell_for(position);
        cells.push(cell);
        if let Some(cell) = cell {
            grid.entry(cell).or_default().push(index);
        }
    }

    let mut hits = 0u32;
    let mut pair_tests = 0usize;
    for i in 0..n {
        let mut next_j = i + 1;
        while let Some((cx, cy)) = cell_for(positions[i]) {
            let mut candidate = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some(x) = cx.checked_add(dx) else {
                        continue;
                    };
                    let Some(y) = cy.checked_add(dy) else {
                        continue;
                    };
                    if let Some(indices) = grid.get(&(x, y)) {
                        let offset = indices.partition_point(|&j| j < next_j);
                        if let Some(&j) = indices.get(offset) {
                            if candidate.is_none_or(|current| j < current) {
                                candidate = Some(j);
                            }
                        }
                    }
                }
            }
            let Some(j) = candidate else {
                break;
            };
            if pair_tests >= max_pair_tests {
                return PairwiseCollisionStats {
                    hits,
                    pair_tests,
                    capped: true,
                };
            }
            pair_tests += 1;
            next_j = j + 1;

            let collided = {
                let (positions_before_j, positions_from_j) = positions.split_at_mut(j);
                let (velocities_before_j, velocities_from_j) = velocities.split_at_mut(j);
                resolve_particle_collision(
                    &mut positions_before_j[i],
                    &mut velocities_before_j[i],
                    &mut positions_from_j[0],
                    &mut velocities_from_j[0],
                    diameter,
                    restitution,
                )
            };
            if collided {
                hits = hits.saturating_add(1);
                for index in [i, j] {
                    let new_cell = cell_for(positions[index]);
                    if cells[index] != new_cell {
                        if let Some(old_cell) = cells[index] {
                            if let Some(indices) = grid.get_mut(&old_cell) {
                                indices.retain(|&other| other != index);
                                if indices.is_empty() {
                                    grid.remove(&old_cell);
                                }
                            }
                        }
                        if let Some(new_cell) = new_cell {
                            let indices = grid.entry(new_cell).or_default();
                            if let Err(position) = indices.binary_search(&index) {
                                indices.insert(position, index);
                            }
                        }
                        cells[index] = new_cell;
                    }
                }
            }
        }
    }
    PairwiseCollisionStats {
        hits,
        pair_tests,
        capped: false,
    }
}
