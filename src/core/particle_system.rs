//! Basic particle emitter system for After Effects-style particle animations.
//!
//! Supports:
//! - Emitter with configurable rate, lifetime, speed, spread
//! - Gravity (with lifetime curve), wind gusts, turbulence and air drag
//! - Boundary collisions with restitution & friction
//! - Per-particle size, color, and opacity over lifetime
//! - Point and box emitter shapes

pub mod raster;
pub mod simulation;
pub mod types;

pub use raster::draw_dot;
pub use simulation::ParticleSystem;
pub use types::{CameraProjection, EmitterShape, FadeCurve, Particle, ParticleEmitter};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::particle_forces::LifeCurve;

    #[test]
    fn test_draw_dot_ignores_invalid_raster_inputs() {
        let original = vec![17u8; 16];
        for (center, half, alpha, color) in [
            ([f32::NAN, 1.0], 2.0, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, f32::INFINITY], 2.0, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], f32::NAN, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], 2.0, f32::NAN, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], 2.0, 1.0, [f32::INFINITY, 1.0, 1.0]),
        ] {
            let mut pixels = original.clone();
            draw_dot(
                &mut pixels,
                (2, 2),
                (center[0], center[1]),
                half,
                color,
                alpha,
                1,
            );
            assert_eq!(pixels, original);
        }
    }

    #[test]
    fn test_particle_emitter_defaults() {
        let e = ParticleEmitter::default();
        assert_eq!(e.shape, EmitterShape::Point);
        assert_eq!(e.blend_mode, 1);
        assert!(e.rate > 0.0);
        assert!(e.lifetime > 0.0);
        assert_eq!(e.vortex_strength, 0.0);
        assert_eq!(e.attract_strength, 0.0);
    }

    #[test]
    fn test_vortex_induces_tangential_velocity() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            vortex_strength: 500.0,
            vortex_center: [100.0, 100.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 200.0,
            y: 100.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(200.0, 100.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        assert!(
            p.vy > 10.0,
            "vortex must induce tangential +vy, got {}",
            p.vy
        );
        assert!(
            p.vx.abs() < 1.0,
            "no radial velocity expected, got {}",
            p.vx
        );
    }

    #[test]
    fn test_attraction_pulls_particle_toward_point() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            attract_strength: 800.0,
            attract_center: [300.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 100.0,
            y: 300.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(100.0, 300.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        assert!(
            p.vx > 20.0,
            "attraction must accelerate toward point, got {}",
            p.vx
        );
    }

    #[test]
    fn test_repulsion_pushes_particle_away() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            attract_strength: -800.0,
            attract_center: [100.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 110.0,
            y: 300.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(110.0, 300.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        assert!(
            p.vx > 20.0,
            "repulsion must push away from point, got {}",
            p.vx
        );
    }

    #[test]
    fn test_death_spawn_emits_children() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            lifetime: 1.0,
            death_spawn_count: 4,
            max_particles: 100,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 50.0,
            y: 60.0,
            vx: 0.0,
            vy: 0.0,
            life: 0.001,
            max_life: 1.0,
            size: 8.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(50.0, 60.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.016, 0.0, 0.0);
        assert_eq!(sys.particles.len(), 4, "expected 4 children");
        for c in &sys.particles {
            assert!((c.x - 50.0).abs() < 2.0 && (c.y - 60.0).abs() < 2.0);
            assert!(c.life > 0.0 && c.max_life <= 0.55, "child lifetime scaled");
        }
    }

    #[test]
    fn test_death_spawn_respects_max_particles() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            death_spawn_count: 10,
            max_particles: 3,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        for i in 0..5u32 {
            sys.particles.push(Particle {
                x: i as f32 * 10.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                life: 0.001,
                max_life: 1.0,
                size: 4.0,
                rotation: 0.0,
                angular_velocity: 0.0,
                trail: [(i as f32 * 10.0, 0.0); 8],
                trail_len: 0,
                z: 0.0,
            });
        }
        sys.update(0.016, 0.0, 0.0);
        assert!(
            sys.particles.len() <= 3,
            "must cap at max_particles, got {}",
            sys.particles.len()
        );
    }

    #[test]
    fn test_camera_projection_near_particle_larger() {
        let proj = CameraProjection {
            cam_x: 500.0,
            cam_y: 400.0,
            cam_z: -1000.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        let near = proj.project(500.0, 400.0, -900.0).expect("in front");
        let far = proj.project(500.0, 400.0, -500.0).expect("in front");
        assert!(
            near.2 > far.2 * 2.5,
            "nearer particle scales bigger: {} vs {}",
            near.2,
            far.2
        );
        assert!((near.0 - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_camera_projection_behind_returns_none() {
        let proj = CameraProjection {
            cam_x: 0.0,
            cam_y: 0.0,
            cam_z: -100.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        assert!(proj.project(10.0, 10.0, -200.0).is_none(), "behind camera");
    }

    #[test]
    fn test_depth_spawn_assigns_z_within_range() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            depth_enabled: true,
            depth_range: [-200.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        for _ in 0..32 {
            sys.emit_accumulator = 1.0;
            sys.update(0.001, 100.0, 100.0);
        }
        assert!(!sys.particles.is_empty());
        for p in &sys.particles {
            assert!(
                (-200.0..=300.0).contains(&p.z),
                "spawned z out of range: {}",
                p.z
            );
        }
    }

    #[test]
    fn test_projected_render_scales_dot_size() {
        let e = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(e.clone());
        sys.particles.push(Particle {
            x: 15.0,
            y: -15.0,
            vx: 0.0,
            vy: 0.0,
            life: 1.0,
            max_life: 1.0,
            size: 8.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(15.0, -15.0); 8],
            trail_len: 0,
            z: -950.0,
        });
        let mut buf = vec![0u8; 64 * 64 * 4];
        let proj = CameraProjection {
            cam_x: 0.0,
            cam_y: 0.0,
            cam_z: -1000.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        sys.render_projected(&mut buf, 64, 64, 0.0, Some(&proj));
        let lit = buf.as_chunks::<4>().0.iter().filter(|c| c[3] > 0).count();
        assert!(lit > 400, "depth-scaled dot must be large, got {lit} px");
    }

    #[test]
    fn test_particle_system_emit() {
        let mut ps = ParticleSystem::new(ParticleEmitter::default());
        ps.emit_accumulator = 10.0;
        ps.update(0.01, 100.0, 100.0);
        assert!(!ps.particles.is_empty());
    }

    #[test]
    fn test_particle_lifetime() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 0.1,
            lifetime_variance: 0.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(1.0, 0.0, 0.0);
        let count_before = ps.particles.len();
        ps.update(0.5, 0.0, 0.0);
        assert!(ps.particles.len() <= count_before);
    }

    #[test]
    fn test_particle_render() {
        let emitter = ParticleEmitter {
            rate: 100.0,
            size_start: 10.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(0.1, 50.0, 50.0);
        let mut buf = vec![0u8; 100 * 100 * 4];
        ps.render(&mut buf, 100, 100, 0.0);
        assert!(buf.iter().any(|&b| b > 0));
    }

    #[test]
    fn test_fade_curve() {
        assert_eq!(FadeCurve::Linear.apply(0.0), 1.0);
        assert_eq!(FadeCurve::Linear.apply(1.0), 0.0);
        assert!(FadeCurve::EaseIn.apply(0.5) > FadeCurve::Linear.apply(0.5));
        assert!(FadeCurve::EaseOut.apply(0.5) < FadeCurve::Linear.apply(0.5));
    }

    #[test]
    fn test_fade_curve_affects_render() {
        let mut buf_linear = vec![0u8; 100 * 100 * 4];
        let mut buf_ease = vec![0u8; 100 * 100 * 4];

        for (buf, curve) in [
            (&mut buf_linear, FadeCurve::Linear),
            (&mut buf_ease, FadeCurve::EaseIn),
        ] {
            let emitter = ParticleEmitter {
                rate: 100.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                size_start: 20.0,
                fade_curve: curve,
                ..Default::default()
            };
            let mut ps = ParticleSystem::new(emitter);
            ps.update(0.5, 50.0, 50.0);
            ps.render(buf, 100, 100, 0.0);
        }
        let sum = |b: &[u8]| b.iter().map(|&v| v as u64).sum::<u64>();
        assert!(sum(&buf_ease) >= sum(&buf_linear));
    }

    #[test]
    fn test_spin_initial_rotation_and_variance() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            rotation_start: 45.0,
            rotation_speed: 90.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(0.01, 0.0, 0.0);
        assert!(!ps.particles.is_empty());
        for p in &ps.particles {
            assert!((p.rotation - (45.0 + 90.0 * 0.01)).abs() < 0.001);
        }

        let make = || {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                rotation_start: 0.0,
                rotation_speed: 90.0,
                rotation_speed_variance: 0.5,
                ..Default::default()
            })
        };
        let mut a = make();
        let mut b = make();
        a.update(0.05, 0.0, 0.0);
        b.update(0.05, 0.0, 0.0);
        let rots: Vec<f32> = a.particles.iter().map(|p| p.rotation).collect();
        assert!(!rots.is_empty());
        assert!(
            rots.windows(2).any(|w| (w[0] - w[1]).abs() > f32::EPSILON),
            "variance should differ per particle"
        );
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.rotation, pb.rotation);
        }
    }

    #[test]
    fn test_emitter_shapes() {
        for shape in [
            EmitterShape::Point,
            EmitterShape::Box,
            EmitterShape::Circle,
            EmitterShape::Line,
        ] {
            let emitter = ParticleEmitter {
                shape,
                rate: 10.0,
                ..Default::default()
            };
            let mut ps = ParticleSystem::new(emitter);
            ps.update(1.0, 50.0, 50.0);
            assert!(
                !ps.particles.is_empty(),
                "Shape {:?} should emit particles",
                shape
            );
        }
    }

    #[test]
    fn test_gravity_curve_accelerates_fall() {
        let make = |curve: LifeCurve| {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                speed: 0.0,
                speed_variance: 0.0,
                spread_degrees: 0.0,
                gravity: [0.0, 100.0],
                gravity_curve: curve,
                ..Default::default()
            })
        };
        let mut weak = make(LifeCurve::constant(1.0));
        let mut strong = make(LifeCurve::constant(2.0));
        weak.update(0.5, 0.0, 0.0);
        strong.update(0.5, 0.0, 0.0);
        let y_weak: f32 =
            weak.particles.iter().map(|p| p.y).sum::<f32>() / weak.particles.len().max(1) as f32;
        let y_strong: f32 = strong.particles.iter().map(|p| p.y).sum::<f32>()
            / strong.particles.len().max(1) as f32;
        assert!(
            y_strong > y_weak * 1.5,
            "curve-multiplied gravity should fall faster"
        );

        let mut a = make(LifeCurve(vec![0.0, 3.0]));
        let mut b = make(LifeCurve(vec![0.0, 3.0]));
        a.update(0.3, 0.0, 0.0);
        b.update(0.3, 0.0, 0.0);
        let ya: Vec<f32> = a.particles.iter().map(|p| p.y).collect();
        let yb: Vec<f32> = b.particles.iter().map(|p| p.y).collect();
        assert_eq!(ya, yb);
    }

    #[test]
    fn test_wind_gust_changes_velocity_over_time() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 0.0],
            wind: [100.0, 0.0],
            wind_gust_strength: 80.0,
            wind_gust_frequency: 1.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.emit_accumulator = 5.0;
        ps.update(0.01, 0.0, 0.0);
        let vx_early = ps.particles[0].vx;
        let mut ps2 = ParticleSystem::new(ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 0.0],
            wind: [100.0, 0.0],
            wind_gust_strength: 80.0,
            wind_gust_frequency: 1.0,
            ..Default::default()
        });
        ps2.emit_accumulator = 5.0;
        for _ in 0..50 {
            ps2.update(0.01, 0.0, 0.0);
        }
        let vx_late =
            ps2.particles.iter().map(|p| p.vx).sum::<f32>() / ps2.particles.len().max(1) as f32;
        assert!(
            (vx_early - vx_late).abs() > 1.0,
            "gust should modulate wind over time"
        );
    }

    #[test]
    fn test_collision_bounces_on_floor() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 500.0],
            collision_enabled: true,
            collision_bounds: [-1000.0, -1000.0, 1000.0, 200.0],
            restitution: 0.6,
            surface_friction: 0.9,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.emit_accumulator = 5.0;
        for _ in 0..120 {
            ps.update(0.05, 0.0, 0.0);
        }
        assert!(!ps.particles.is_empty());
        for p in &ps.particles {
            assert!(p.y <= 200.0 + 1e-3, "particle fell through floor: {}", p.y);
        }
        assert!(
            ps.particles.iter().any(|p| p.vy < -1.0),
            "no bounce detected"
        );
    }

    #[test]
    fn test_drag_slows_particles() {
        let make = |drag: f32| {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                speed: 300.0,
                speed_variance: 0.0,
                spread_degrees: 0.0,
                gravity: [0.0, 0.0],
                drag,
                ..Default::default()
            })
        };
        let mut free = make(0.0);
        let mut damped = make(4.0);
        free.update(1.0, 0.0, 0.0);
        damped.update(1.0, 0.0, 0.0);
        let v_free = free.particles[0].vx;
        let v_damped = damped.particles[0].vx;
        assert!(v_free > v_damped, "drag must reduce velocity");
        assert!(v_damped > 0.0);
    }

    #[test]
    fn test_emitter_serde_backward_compat_without_new_fields() {
        let json = r#"{
            "rate": 30.0, "max_particles": 500, "lifetime": 1.5,
            "lifetime_variance": 0.1, "speed": 150.0, "speed_variance": 0.2,
            "spread_degrees": 180.0, "shape": "Point",
            "emitter_size": [50.0, 50.0], "gravity": [0.0, 200.0],
            "wind": [0.0, 0.0], "turbulence": 0.0,
            "color_start": [1.0, 1.0, 1.0, 1.0], "color_end": [0.0, 0.0, 0.0, 0.0],
            "size_start": 6.0, "size_end": 1.0,
            "opacity_start": 1.0, "opacity_end": 0.0, "blend_mode": 1
        }"#;
        let e: ParticleEmitter = serde_json::from_str(json).unwrap_or_default();
        assert_eq!(e.rate, 30.0);
        assert_eq!(e.gravity_curve, LifeCurve::default());
        assert!(!e.collision_enabled);
        assert_eq!(e.drag, 0.0);
        assert_eq!(e.wind_gust_strength, 0.0);
    }

    #[test]
    fn test_simulation_is_deterministic_with_all_forces() {
        let make = || {
            ParticleSystem::new(ParticleEmitter {
                rate: 500.0,
                lifetime: 3.0,
                gravity_curve: LifeCurve(vec![0.2, 1.5, 0.8]),
                wind: [60.0, -20.0],
                wind_gust_strength: 40.0,
                wind_gust_frequency: 2.0,
                drag: 0.8,
                turbulence: 15.0,
                collision_enabled: true,
                collision_bounds: [0.0, 0.0, 800.0, 600.0],
                restitution: 0.55,
                surface_friction: 0.85,
                ..Default::default()
            })
        };
        let mut a = make();
        let mut b = make();
        for _ in 0..60 {
            a.update(1.0 / 30.0, 400.0, 100.0);
        }
        for _ in 0..60 {
            b.update(1.0 / 30.0, 400.0, 100.0);
        }
        assert_eq!(a.particles.len(), b.particles.len());
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.x, pb.x);
            assert_eq!(pa.y, pb.y);
            assert_eq!(pa.vx, pb.vx);
            assert_eq!(pa.vy, pb.vy);
        }
    }
}

#[cfg(test)]
mod particle_collision_tests {
    use super::*;
    use crate::core::particle_forces::LifeCurve as LC;

    fn headon_system() -> ParticleSystem {
        let mut emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            turbulence: 0.0,
            collision_enabled: false,
            particle_collisions: true,
            particle_diameter: 10.0,
            restitution: 1.0,
            ..Default::default()
        };
        emitter.gravity_curve = LC::constant(1.0);
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 46.0,
            y: 50.0,
            vx: 30.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(46.0, 50.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.particles.push(Particle {
            x: 54.0,
            y: 50.0,
            vx: -30.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(54.0, 50.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys
    }

    #[test]
    fn test_system_resolves_particle_particle_collision() {
        let mut sys = headon_system();
        sys.update(0.016, 0.0, 0.0);
        let (a, b) = (&sys.particles[0], &sys.particles[1]);
        assert!((a.vx + 30.0).abs() < 1e-3, "a.vx {}", a.vx);
        assert!((b.vx - 30.0).abs() < 1e-3, "b.vx {}", b.vx);
        let dist = (b.x - a.x).hypot(b.y - a.y);
        assert!(dist >= 9.9, "post distance {dist}");
    }

    #[test]
    fn test_flag_off_leaves_overlapping_pair_alone() {
        let mut sys = headon_system();
        sys.emitter.particle_collisions = false;
        let before = (sys.particles[0].vx, sys.particles[1].vx);
        sys.update(0.016, 0.0, 0.0);
        assert_eq!(sys.particles[0].vx, before.0, "flag off → untouched");
    }
}
