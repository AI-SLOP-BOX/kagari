#[cfg(test)]
mod tests {
    use crate::core::physics::*;

    #[test]
    fn test_kinematic_gravity_integration() {
        let solver = KinematicSolver::default();
        let mut state = KinematicState::default();

        solver.update_state(&mut state, 1.0 / 30.0);
        assert!(
            state.position.y > 0.0,
            "Gravity should accelerate layer downward in AE coordinate space"
        );
    }

    #[test]
    fn test_rigidbody_simulation_fall_and_bounce() {
        let mut world = PhysicsWorld::new();

        // Dynamic falling box
        let body_box =
            RigidBody::new_box(None, [100.0, 0.0], 50.0, 50.0, 1.0, RigidBodyType::Dynamic);
        let box_idx = world.add_body(body_box);

        // Static floor
        let body_floor = RigidBody::new_box(
            None,
            [100.0, 200.0],
            500.0,
            40.0,
            0.0,
            RigidBodyType::Static,
        );
        world.add_body(body_floor);

        // Run simulation for 1 second (30 fps)
        for _ in 0..30 {
            world.step(1.0 / 30.0);
        }

        // The box should fall down and stop/bounce on top of the floor (~155-175 px)
        assert!(world.bodies[box_idx].position[1] > 100.0);
        assert!(world.bodies[box_idx].position[1] < 220.0);
    }

    #[test]
    fn test_spring_constraint_pulls_bodies_together() {
        let mut world = PhysicsWorld::new();
        world.gravity = [0.0, 0.0]; // Zero gravity to isolate spring force

        let b1 = world.add_body(RigidBody::new_circle(
            None,
            [0.0, 0.0],
            10.0,
            1.0,
            RigidBodyType::Dynamic,
        ));
        let b2 = world.add_body(RigidBody::new_circle(
            None,
            [100.0, 0.0],
            10.0,
            1.0,
            RigidBodyType::Dynamic,
        ));

        world.add_spring(DistanceSpring {
            body_a: b1,
            body_b: Some(b2),
            anchor_a_local: [0.0, 0.0],
            anchor_b_local_or_world: [0.0, 0.0],
            rest_length: 20.0,
            stiffness: 50.0,
            damping: 2.0,
        });

        // Step simulation forward
        world.step(0.1);

        // b1 should move right, b2 should move left
        assert!(world.bodies[b1].position[0] > 0.0);
        assert!(world.bodies[b2].position[0] < 100.0);
    }

    #[test]
    fn test_keyframe_baker_outputs_valid_tracks() {
        let mut world = PhysicsWorld::new();
        let layer_id = 42;

        let b = RigidBody::new_box(
            Some(layer_id),
            [50.0, 50.0],
            20.0,
            20.0,
            1.0,
            RigidBodyType::Dynamic,
        );
        world.add_body(b);

        let baked = bake_physics_simulation_to_keyframes(&mut world, 0, 10, 30.0);
        assert!(baked.contains_key(&layer_id));

        let (pos_kfs, rot_kfs) = baked.get(&layer_id).unwrap();
        assert_eq!(pos_kfs.len(), 11);
        assert_eq!(rot_kfs.len(), 11);
        assert_eq!(pos_kfs[0].frame, 0);
        assert_eq!(pos_kfs[10].frame, 10);
    }

    #[test]
    fn test_spring_overshoot_and_bounce_decay() {
        let val1 = calc_spring_overshoot(0.1, 3.0, 5.0, 0.2);
        assert!(val1.abs() > 0.0);

        let bounce0 = calc_bounce_decay(0.05, 3, 0.5);
        let bounce1 = calc_bounce_decay(0.4, 3, 0.5);
        assert!(bounce0 >= 0.0);
        assert!(bounce1 >= 0.0);
    }
}
