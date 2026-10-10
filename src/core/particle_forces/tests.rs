#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;

    #[test]
    fn test_life_curve_interpolation() {
        assert_eq!(LifeCurve::default().eval(0.5), 1.0);
        assert_eq!(LifeCurve(vec![]).eval(0.7), 1.0);
        let c = LifeCurve(vec![0.0, 1.0, 0.0]);
        assert_eq!(c.eval(0.0), 0.0);
        assert_eq!(c.eval(0.5), 1.0);
        assert_eq!(c.eval(1.0), 0.0);
        assert!((c.eval(0.25) - 0.5).abs() < 1e-6);
        // Out-of-range t clamps to endpoints.
        assert_eq!(c.eval(-1.0), 0.0);
        assert_eq!(c.eval(2.0), 0.0);
    }

    #[test]
    fn test_wind_gust_bounded_and_deterministic() {
        let base = [40.0, 0.0];
        let a = wind_with_gust(base, 20.0, 0.5, 0.25);
        let b = wind_with_gust(base, 20.0, 0.5, 0.25);
        assert_eq!(a, b);
        // Gust magnitude never exceeds base + strength.
        for t in 0..100 {
            let w = wind_with_gust(base, 20.0, 0.5, t as f32 * 0.01);
            assert!(w[0] >= 40.0 - 20.0 - 1e-4 && w[0] <= 40.0 + 20.0 + 1e-4);
        }
        // Zero strength returns base exactly.
        assert_eq!(wind_with_gust(base, 0.0, 1.0, 3.0), base);
        // Zero base wind gusts along +X.
        let w = wind_with_gust([0.0, 0.0], 5.0, 1.0, 0.25); // sin(pi/2)=1
        assert!((w[0] - 5.0).abs() < 1e-4);
        assert!(w[1].abs() < 1e-4);
    }

    #[test]
    fn test_drag_decays_velocity() {
        let mut vx = 100.0;
        let mut vy = 0.0;
        for _ in 0..100 {
            apply_drag(&mut vx, &mut vy, 2.0, 0.016);
        }
        assert!(vx < 100.0 && vx > 0.0);
        // Zero drag is a no-op.
        let mut ux = 50.0;
        let mut uy = 50.0;
        apply_drag(&mut ux, &mut uy, 0.0, 0.016);
        assert_eq!(ux, 50.0);
        assert_eq!(uy, 50.0);
    }

    #[test]
    fn test_floor_collision_reflects_with_restitution() {
        let mut pos = [50.0, 105.0];
        let mut vel = [10.0, 40.0];
        let hit = resolve_bounds_collision(&mut pos, &mut vel, [0.0, 0.0, 100.0, 100.0], 0.5, 0.9);
        assert!(hit);
        assert_eq!(pos[1], 100.0);
        assert!((vel[1] + 20.0).abs() < 1e-5); // -40 * 0.5
        assert!((vel[0] - 9.0).abs() < 1e-5); // 10 * 0.9 friction
    }

    #[test]
    fn test_wall_and_ceiling_collision() {
        let mut pos = [-5.0, 50.0];
        let mut vel = [-30.0, 0.0];
        assert!(resolve_bounds_collision(
            &mut pos,
            &mut vel,
            [0.0, 0.0, 100.0, 100.0],
            1.0,
            1.0
        ));
        assert_eq!(pos[0], 0.0);
        assert_eq!(vel[0], 30.0);

        let mut pos2 = [50.0, -1.0];
        let mut vel2 = [0.0, -10.0];
        assert!(resolve_bounds_collision(
            &mut pos2,
            &mut vel2,
            [0.0, 0.0, 100.0, 100.0],
            0.0,
            1.0
        ));
        assert_eq!(pos2[1], 0.0);
        assert_eq!(vel2[1], 0.0); // restitution 0 kills the bounce
    }

    #[test]
    fn test_no_contact_returns_false() {
        let mut pos = [50.0, 50.0];
        let mut vel = [1.0, 1.0];
        assert!(!resolve_bounds_collision(
            &mut pos,
            &mut vel,
            [0.0, 0.0, 100.0, 100.0],
            0.5,
            1.0
        ));
        assert_eq!(pos, [50.0, 50.0]);
        assert_eq!(vel, [1.0, 1.0]);
        // Inverted bounds are ignored defensively.
        let mut p2 = [50.0, 50.0];
        let mut v2 = [1.0, 1.0];
        assert!(!resolve_bounds_collision(
            &mut p2,
            &mut v2,
            [100.0, 100.0, 0.0, 0.0],
            0.5,
            1.0
        ));
    }

    #[test]
    fn test_life_curve_serde_roundtrip() {
        let c = LifeCurve(vec![1.0, 0.5, 2.0]);
        let json = serde_json::to_string(&c).unwrap_or_default();
        let back: LifeCurve = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(c, back);
    }

    #[test]
    fn test_head_on_elastic_collision_swaps_velocities() {
        // Equal masses, restitution 1: velocities exchange exactly.
        let mut pa = [40.0, 50.0];
        let mut va = [30.0, 0.0];
        let mut pb = [60.0, 50.0];
        let mut vb = [-30.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 24.0, 1.0
        ));
        assert!(
            (va[0] + 30.0).abs() < 1e-4,
            "a takes b's velocity: {}",
            va[0]
        );
        assert!(
            (vb[0] - 30.0).abs() < 1e-4,
            "b takes a's velocity: {}",
            vb[0]
        );
        // Particles separated to contact distance.
        let dist = (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
        assert!((dist - 24.0).abs() < 1e-3, "post distance {dist}");
    }

    #[test]
    fn test_inelastic_collision_kills_relative_normal_velocity() {
        let mut pa = [10.0, 10.0];
        let mut va = [20.0, 0.0];
        let mut pb = [26.0, 10.0];
        let mut vb = [0.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 20.0, 0.0
        ));
        let vn_after = (vb[0] - va[0]) * 1.0 + (vb[1] - va[1]) * 0.0;
        assert!(
            vn_after.abs() < 1e-4,
            "normal relative velocity must vanish"
        );
        // Both move right together afterwards (momentum conserved).
        assert!(va[0] > 5.0 && vb[0] > 5.0);
    }

    #[test]
    fn test_separating_pair_is_untouched() {
        // Already separating along the normal → no impulse.
        let mut pa = [0.0, 0.0];
        let mut va = [-10.0, 0.0];
        let mut pb = [8.0, 0.0]; // overlapping
        let mut vb = [10.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 12.0, 1.0
        ));
        // Positions still separated, velocities unchanged.
        assert_eq!(va, [-10.0, 0.0]);
        assert_eq!(vb, [10.0, 0.0]);
    }

    #[test]
    fn test_far_apart_and_degenerate_inputs() {
        let mut pa = [0.0, 0.0];
        let mut va = [1.0, 1.0];
        let mut pb = [100.0, 100.0];
        let mut vb = [-1.0, -1.0];
        assert!(!resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 10.0, 0.8
        ));
        assert_eq!(pa, [0.0, 0.0]);
        // Zero radius is always false.
        assert!(!resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 0.0, 1.0
        ));
        // Coincident particles split cleanly without NaN.
        let mut qa = [50.0, 50.0];
        let mut qva = [0.0, 0.0];
        let mut qb = [50.0, 50.0];
        let mut qvb = [0.0, 0.0];
        assert!(resolve_particle_collision(
            &mut qa, &mut qva, &mut qb, &mut qvb, 8.0, 1.0
        ));
        assert!(qa[0].is_finite() && qb[0].is_finite());
        assert!((qb[0] - qa[0]).abs() >= 7.9);
    }

    #[test]
    fn test_pairwise_batch_counts_and_resolves() {
        // Three particles in a row, first one slams into the middle one.
        let mut pos = vec![[0.0, 0.0], [15.0, 0.0], [90.0, 0.0]];
        let mut vel = vec![[50.0, 0.0], [0.0, 0.0], [0.0, 0.0]];
        let hits = resolve_pairwise_collisions(&mut pos, &mut vel, 16.0, 0.9);
        assert_eq!(hits, 1, "only the close pair collides");
        // Momentum conserved along x.
        let total_before = 50.0;
        let total_after = vel[0][0] + vel[1][0];
        assert!((total_after - total_before).abs() < 1e-3);
        // Third particle untouched.
        assert_eq!(vel[2], [0.0, 0.0]);
        // Empty input safe.
        let mut ep: Vec<[f32; 2]> = vec![];
        let mut ev: Vec<[f32; 2]> = vec![];
        assert_eq!(resolve_pairwise_collisions(&mut ep, &mut ev, 4.0, 1.0), 0);
    }

    #[test]
    fn test_vortex_force_pulls_inward_and_tangential() {
        let f = vortex_force([100.0, 0.0], [0.0, 0.0], 10.0, 20.0);
        // Point is on +X axis relative to center (0,0)
        // Inward pull should be along -X (-10.0)
        // Tangential speed should be along +Y (+20.0)
        assert!((f[0] + 10.0).abs() < 1e-3);
        assert!((f[1] - 20.0).abs() < 1e-3);
    }

    #[test]
    fn test_curl_noise_turbulence_produces_finite_vectors() {
        let f = curl_noise_turbulence([50.0, 50.0], 0.05, 10.0);
        assert!(f[0].is_finite());
        assert!(f[1].is_finite());
    }
}
