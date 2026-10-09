use super::raster::draw_dot;
use super::types::{CameraProjection, EmitterShape, Particle, ParticleEmitter};
use crate::core::particle_forces::{
    apply_drag, resolve_bounds_collision, resolve_pairwise_collisions, wind_with_gust,
};

/// Particle system simulation.
#[derive(Clone)]
pub struct ParticleSystem {
    pub emitter: ParticleEmitter,
    pub particles: Vec<Particle>,
    /// Accumulated time for emission scheduling
    pub emit_accumulator: f32,
    /// Total simulated time (drives wind gusts)
    pub elapsed_time: f32,
    /// PRNG state for deterministic randomness
    rng_state: u64,
}

impl ParticleSystem {
    pub fn new(emitter: ParticleEmitter) -> Self {
        Self {
            emitter,
            particles: Vec::new(),
            emit_accumulator: 0.0,
            elapsed_time: 0.0,
            rng_state: 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    /// Simple xorshift64 PRNG.
    fn next_random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state & 0xFFFF) as f32 / 65535.0
    }

    /// Emit a single particle at the emitter position.
    fn emit_particle(&mut self, emitter_x: f32, emitter_y: f32) {
        if self.particles.len() >= self.emitter.max_particles as usize {
            return;
        }

        let lifetime = self.emitter.lifetime
            * (1.0 - self.emitter.lifetime_variance * (self.next_random() * 2.0 - 1.0));
        let speed = self.emitter.speed
            * (1.0 + self.emitter.speed_variance * (self.next_random() * 2.0 - 1.0));
        let rotation_speed = if self.emitter.rotation_speed_variance > 0.0 {
            self.emitter.rotation_speed
                * (1.0 + self.emitter.rotation_speed_variance * (self.next_random() * 2.0 - 1.0))
        } else {
            self.emitter.rotation_speed
        };

        let spread_rad = self.emitter.spread_degrees.to_radians();
        let angle = (self.next_random() - 0.5) * spread_rad;

        let (px, py) = match self.emitter.shape {
            EmitterShape::Point => (emitter_x, emitter_y),
            EmitterShape::Box => (
                emitter_x + (self.next_random() - 0.5) * self.emitter.emitter_size[0],
                emitter_y + (self.next_random() - 0.5) * self.emitter.emitter_size[1],
            ),
            EmitterShape::Circle => {
                let r = self.next_random().sqrt() * self.emitter.emitter_size[0] * 0.5;
                let a = self.next_random() * std::f32::consts::TAU;
                (emitter_x + r * a.cos(), emitter_y + r * a.sin())
            }
            EmitterShape::Line => (
                emitter_x + (self.next_random() - 0.5) * self.emitter.emitter_size[0],
                emitter_y,
            ),
            EmitterShape::Ring => {
                let r = self.emitter.emitter_size[0] * 0.5;
                let a = self.next_random() * std::f32::consts::TAU;
                (emitter_x + r * a.cos(), emitter_y + r * a.sin())
            }
        };

        let z = if self.emitter.depth_enabled {
            let [z0, z1] = self.emitter.depth_range;
            z0 + (z1 - z0) * self.next_random()
        } else {
            0.0
        };

        self.particles.push(Particle {
            x: px,
            y: py,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            life: lifetime,
            max_life: lifetime,
            size: self.emitter.size_start,
            rotation: self.emitter.rotation_start,
            angular_velocity: rotation_speed,
            trail: [(px, py); 8],
            trail_len: 0,
            z,
        });
    }

    /// Step the particle simulation by dt seconds.
    pub fn update(&mut self, dt: f32, emitter_x: f32, emitter_y: f32) {
        // Emit new particles
        self.emit_accumulator += dt * self.emitter.rate;
        while self.emit_accumulator >= 1.0 {
            self.emit_particle(emitter_x, emitter_y);
            self.emit_accumulator -= 1.0;
        }
        self.elapsed_time += dt;

        let gravity = self.emitter.gravity;
        let gcurve = self.emitter.gravity_curve.clone();
        let gust_strength = self.emitter.wind_gust_strength;
        let gust_freq = self.emitter.wind_gust_frequency;
        let base_wind = self.emitter.wind;
        let turbulence = self.emitter.turbulence;
        let size_start = self.emitter.size_start;
        let size_end = self.emitter.size_end;
        let drag = self.emitter.drag;
        let collide = self.emitter.collision_enabled;
        let bounds = self.emitter.collision_bounds;
        let restitution = self.emitter.restitution.clamp(0.0, 1.0);
        let friction = self.emitter.surface_friction.clamp(0.0, 1.0);
        let vortex_strength = self.emitter.vortex_strength;
        let vortex_center = self.emitter.vortex_center;
        let attract_strength = self.emitter.attract_strength;
        let attract_center = self.emitter.attract_center;

        // Update existing particles
        for p in &mut self.particles {
            p.life -= dt;
            if p.life <= 0.0 {
                continue;
            }

            let age_t = 1.0 - (p.life / p.max_life).max(0.0);
            let gmul = gcurve.eval(age_t);
            let wind = wind_with_gust(base_wind, gust_strength, gust_freq, self.elapsed_time);
            p.vx += (gravity[0] * gmul + wind[0]) * dt;
            p.vy += (gravity[1] * gmul + wind[1]) * dt;

            apply_drag(&mut p.vx, &mut p.vy, drag, dt);

            if vortex_strength != 0.0 {
                let rx = p.x - vortex_center[0];
                let ry = p.y - vortex_center[1];
                let dist = (rx * rx + ry * ry).sqrt().max(8.0);
                p.vx += (-ry / dist) * vortex_strength * dt;
                p.vy += (rx / dist) * vortex_strength * dt;
            }

            if attract_strength != 0.0 {
                let ax = attract_center[0] - p.x;
                let ay = attract_center[1] - p.y;
                let dist = (ax * ax + ay * ay).sqrt().max(4.0);
                p.vx += (ax / dist) * attract_strength * dt;
                p.vy += (ay / dist) * attract_strength * dt;
            }

            if turbulence > 0.0 {
                let t = p.life * 10.0;
                p.vx += (t.sin() * 0.5 + (t * 2.7).sin() * 0.3) * turbulence * dt;
                p.vy += (t.cos() * 0.5 + (t * 3.1).cos() * 0.3) * turbulence * dt;
            }

            p.x += p.vx * dt;
            p.y += p.vy * dt;

            let max_trail = (self.emitter.trail_length as usize).min(8);
            if max_trail > 0 {
                for i in (1..max_trail).rev() {
                    p.trail[i] = p.trail[i - 1];
                }
                p.trail[0] = (p.x, p.y);
                if (p.trail_len as usize) < max_trail {
                    p.trail_len += 1;
                }
            }

            if collide {
                let mut pos = [p.x, p.y];
                let mut vel = [p.vx, p.vy];
                resolve_bounds_collision(&mut pos, &mut vel, bounds, restitution, friction);
                p.x = pos[0];
                p.y = pos[1];
                p.vx = vel[0];
                p.vy = vel[1];
            }

            p.rotation += p.angular_velocity * dt;

            let t = (1.0 - (p.life / p.max_life)).clamp(0.0, 1.0);
            p.size = size_start + (size_end - size_start) * t;
        }

        if self.emitter.particle_collisions && self.particles.len() > 1 {
            let mut pos: Vec<[f32; 2]> = self.particles.iter().map(|p| [p.x, p.y]).collect();
            let mut vel: Vec<[f32; 2]> = self.particles.iter().map(|p| [p.vx, p.vy]).collect();
            resolve_pairwise_collisions(
                &mut pos,
                &mut vel,
                self.emitter.particle_diameter.max(0.1),
                restitution,
            );
            for (p, (np, nv)) in self.particles.iter_mut().zip(pos.into_iter().zip(vel)) {
                p.x = np[0];
                p.y = np[1];
                p.vx = nv[0];
                p.vy = nv[1];
            }
        }

        if self.emitter.death_spawn_count > 0 {
            let count = self.emitter.death_spawn_count;
            let speed_scale = self.emitter.death_spawn_speed_scale;
            let life_scale = self.emitter.death_spawn_life_scale;
            let _spread = self.emitter.spread_degrees.to_radians();
            let base_speed = self.emitter.speed * speed_scale;
            let dead: Vec<(f32, f32, f32)> = self
                .particles
                .iter()
                .filter(|p| p.life <= 0.0)
                .map(|p| (p.x, p.y, p.z))
                .collect();
            for (dx, dy, parent_z) in dead {
                for _ in 0..count {
                    if self.particles.len() >= self.emitter.max_particles as usize {
                        break;
                    }
                    let angle = self.next_random() * std::f32::consts::TAU;
                    let speed = base_speed * (0.5 + self.next_random() * 0.5);
                    let lifetime = (self.emitter.lifetime * life_scale).max(0.05);
                    self.particles.push(Particle {
                        x: dx,
                        y: dy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed,
                        life: lifetime,
                        max_life: lifetime,
                        size: self.emitter.size_start * 0.5,
                        rotation: 0.0,
                        angular_velocity: 0.0,
                        trail: [(dx, dy); 8],
                        trail_len: 0,
                        z: parent_z,
                    });
                }
            }
        }

        self.particles.retain(|p| p.life > 0.0);
    }

    /// Render all particles into an RGBA pixel buffer (flat, no camera).
    pub fn render(&self, buffer: &mut [u8], buf_width: u32, buf_height: u32, _time: f32) {
        self.render_projected(buffer, buf_width, buf_height, _time, None);
    }

    /// Render with optional camera projection.
    pub fn render_projected(
        &self,
        buffer: &mut [u8],
        buf_width: u32,
        buf_height: u32,
        _time: f32,
        proj: Option<&CameraProjection>,
    ) {
        let e = &self.emitter;
        for p in &self.particles {
            if p.life <= 0.0 {
                continue;
            }

            let t = 1.0 - (p.life / p.max_life);
            let r = e.color_start[0] + (e.color_end[0] - e.color_start[0]) * t;
            let g = e.color_start[1] + (e.color_end[1] - e.color_start[1]) * t;
            let b = e.color_start[2] + (e.color_end[2] - e.color_start[2]) * t;
            let a = (e.color_start[3] + (e.color_end[3] - e.color_start[3]) * t)
                * e.fade_curve.apply(t);

            let Some((sx, sy, sscale)) =
                proj.map_or(Some((p.x, p.y, 1.0)), |cp| cp.project(p.x, p.y, p.z))
            else {
                continue;
            };

            if p.trail_len > 0 && e.trail_taper > 0.01 {
                let max_trail = p.trail_len as usize;
                let trail_size = p.size * 0.4;
                for i in 0..max_trail {
                    let (tx, ty) = p.trail[i];
                    let fade = e.trail_taper.powi(i as i32 + 1);
                    let ta = a * fade;
                    if ta < 0.01 {
                        continue;
                    }
                    let Some((txs, tys, ts_scale)) =
                        proj.map_or(Some((tx, ty, 1.0)), |cp| cp.project(tx, ty, p.z))
                    else {
                        continue;
                    };
                    let half_t =
                        trail_size * ts_scale * 0.5 * (1.0 - (i as f32 / max_trail as f32) * 0.5);
                    draw_dot(
                        buffer,
                        (buf_width, buf_height),
                        (txs, tys),
                        half_t,
                        [r, g, b],
                        ta,
                        e.blend_mode,
                    );
                }
            }

            let half = p.size * sscale * 0.5;
            draw_dot(
                buffer,
                (buf_width, buf_height),
                (sx, sy),
                half,
                [r, g, b],
                a,
                e.blend_mode,
            );
        }
    }
}
