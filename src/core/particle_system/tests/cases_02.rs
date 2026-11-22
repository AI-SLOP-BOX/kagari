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
    let y_strong: f32 =
        strong.particles.iter().map(|p| p.y).sum::<f32>() / strong.particles.len().max(1) as f32;
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
    emitter.gravity_curve = LifeCurve::constant(1.0);
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
