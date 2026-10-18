//! PhysicsWorld container and numerical substepping solver.

use super::collision::detect_collision;
use super::rigidbody::{DistanceSpring, RigidBody, RigidBodyType};

pub struct PhysicsWorld {
    pub gravity: [f32; 2],
    pub air_drag: f32,
    pub substeps: u32,
    pub bodies: Vec<RigidBody>,
    pub springs: Vec<DistanceSpring>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: [0.0, 980.0], // Pixels / s^2 downward
            air_drag: 0.995,
            substeps: 8,
            bodies: Vec::new(),
            springs: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_body(&mut self, mut body: RigidBody) -> usize {
        let idx = self.bodies.len();
        body.id = idx;
        self.bodies.push(body);
        idx
    }

    pub fn add_spring(&mut self, spring: DistanceSpring) {
        self.springs.push(spring);
    }

    /// Advances the entire physics world by `dt` seconds using numerical substepping.
    pub fn step(&mut self, dt: f32) {
        let substeps = self.substeps.max(1);
        let sub_dt = dt / (substeps as f32);

        for _ in 0..substeps {
            self.substep(sub_dt);
        }
    }

    fn substep(&mut self, dt: f32) {
        // 1. Apply gravity & air drag
        let drag_damp = self.air_drag.clamp(0.0, 1.0).powf(dt * 60.0);
        for body in &mut self.bodies {
            if body.body_type == RigidBodyType::Dynamic {
                body.velocity[0] += self.gravity[0] * dt;
                body.velocity[1] += self.gravity[1] * dt;

                body.velocity[0] *= drag_damp;
                body.velocity[1] *= drag_damp;
                body.angular_velocity_deg *= drag_damp;
            }
        }

        // 2. Solve Spring Constraints
        for spring in &self.springs {
            let (pos_a, inv_m_a) = {
                let a = &self.bodies[spring.body_a];
                (a.position, a.inv_mass)
            };

            let (pos_b, inv_m_b) = if let Some(b_idx) = spring.body_b {
                let b = &self.bodies[b_idx];
                (b.position, b.inv_mass)
            } else {
                (spring.anchor_b_local_or_world, 0.0)
            };

            let dx = pos_b[0] - pos_a[0];
            let dy = pos_b[1] - pos_a[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 1e-4 {
                let nx = dx / dist;
                let ny = dy / dist;
                let delta = dist - spring.rest_length;

                // Hooke's Law + Damping
                let force_mag = spring.stiffness * delta;

                if inv_m_a > 0.0 {
                    self.bodies[spring.body_a].velocity[0] += (nx * force_mag * inv_m_a) * dt;
                    self.bodies[spring.body_a].velocity[1] += (ny * force_mag * inv_m_a) * dt;
                }

                if let Some(b_idx) = spring.body_b {
                    if inv_m_b > 0.0 {
                        self.bodies[b_idx].velocity[0] -= (nx * force_mag * inv_m_b) * dt;
                        self.bodies[b_idx].velocity[1] -= (ny * force_mag * inv_m_b) * dt;
                    }
                }
            }
        }

        // 3. Collision Detection (Narrowphase)
        let mut contacts = Vec::new();
        let num_bodies = self.bodies.len();
        for i in 0..num_bodies {
            for j in (i + 1)..num_bodies {
                if self.bodies[i].body_type == RigidBodyType::Static
                    && self.bodies[j].body_type == RigidBodyType::Static
                {
                    continue;
                }

                if let Some(contact) = detect_collision(&self.bodies[i], &self.bodies[j], i, j) {
                    contacts.push(contact);
                }
            }
        }

        // 4. Collision Resolution (Impulse-based)
        for contact in contacts {
            let inv_mass_sum =
                self.bodies[contact.body_a].inv_mass + self.bodies[contact.body_b].inv_mass;
            if inv_mass_sum <= 1e-6 {
                continue;
            }

            // Positional correction (slop separation)
            let slop = 0.05;
            let percent = 0.4;
            let correction = (contact.penetration - slop).max(0.0) / inv_mass_sum * percent;
            let corr_x = contact.normal[0] * correction;
            let corr_y = contact.normal[1] * correction;

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].position[0] -= corr_x * ma;
                self.bodies[contact.body_a].position[1] -= corr_y * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].position[0] += corr_x * mb;
                self.bodies[contact.body_b].position[1] += corr_y * mb;
            }

            // Velocity impulse
            let va = self.bodies[contact.body_a].velocity;
            let vb = self.bodies[contact.body_b].velocity;
            let rvx = vb[0] - va[0];
            let rvy = vb[1] - va[1];

            let vel_along_normal = rvx * contact.normal[0] + rvy * contact.normal[1];
            if vel_along_normal > 0.0 {
                continue; // Moving away
            }

            let e = self.bodies[contact.body_a]
                .restitution
                .min(self.bodies[contact.body_b].restitution);
            let j = -(1.0 + e) * vel_along_normal / inv_mass_sum;

            let impulse_x = contact.normal[0] * j;
            let impulse_y = contact.normal[1] * j;

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].velocity[0] -= impulse_x * ma;
                self.bodies[contact.body_a].velocity[1] -= impulse_y * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].velocity[0] += impulse_x * mb;
                self.bodies[contact.body_b].velocity[1] += impulse_y * mb;
            }

            // Friction impulse
            let tangent = [-contact.normal[1], contact.normal[0]];
            let vt = rvx * tangent[0] + rvy * tangent[1];
            let mu =
                (self.bodies[contact.body_a].friction + self.bodies[contact.body_b].friction) * 0.5;
            let jt = -vt / inv_mass_sum;
            let friction_impulse = jt.clamp(-j * mu, j * mu);

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].velocity[0] -= tangent[0] * friction_impulse * ma;
                self.bodies[contact.body_a].velocity[1] -= tangent[1] * friction_impulse * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].velocity[0] += tangent[0] * friction_impulse * mb;
                self.bodies[contact.body_b].velocity[1] += tangent[1] * friction_impulse * mb;
            }
        }

        // 5. Integrate positions & rotations
        for body in &mut self.bodies {
            if body.body_type == RigidBodyType::Dynamic {
                body.position[0] += body.velocity[0] * dt;
                body.position[1] += body.velocity[1] * dt;
                body.rotation_deg += body.angular_velocity_deg * dt;
            }
        }
    }
}
