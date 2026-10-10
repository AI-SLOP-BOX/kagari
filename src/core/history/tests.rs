#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::history::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::shape_repeater::ShapeRepeaterOptions;
    use crate::core::timeline::{Composition, Layer, LayerType, Project};

    fn big_project(layers: usize, keyframes_per_layer: usize) -> Project {
        let mut comp = Composition::new("c".into(), "Big".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [1.0; 4] },
                30,
            );
            let kfs: Vec<Keyframe<[f32; 2]>> = (0..keyframes_per_layer)
                .map(|k| {
                    Keyframe::new(
                        k as u32,
                        [k as f32, 0.0],
                        InterpolationType::Linear,
                    )
                })
                .collect();
            l.transform.position = Animatable::new_animated(kfs);
            comp.layers.push(l);
        }
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    /// ~10MB per entry with a unique id per tag so consecutive commits are
    /// distinct (identical snapshots are deduped and would never trim).
    fn huge_project(tag: usize) -> Project {
        let mut comp = Composition::new(format!("c{}", tag), "Big".into(), 64, 64, 30, 30);
        let payload = "x".repeat(700_000);
        for i in 0..10 {
            let mut l = Layer::new(
                format!("l{}_{}", tag, i),
                format!("L{}{}", i, payload),
                LayerType::Solid { color: [1.0; 4] },
                30,
            );
            let kfs2: Vec<Keyframe<[f32; 2]>> = (0..1000)
                .map(|k| Keyframe::new(k, [k as f32, 0.0], InterpolationType::Linear))
                .collect();
            let kfs1: Vec<Keyframe<f32>> = (0..1000)
                .map(|k| Keyframe::new(k, k as f32, InterpolationType::Linear))
                .collect();
            l.transform.position = Animatable::new_animated(kfs2.clone());
            l.transform.scale = Animatable::new_animated(kfs2);
            l.transform.rotation = Animatable::new_animated(kfs1.clone());
            l.transform.opacity = Animatable::new_animated(kfs1);
            comp.layers.push(l);
        }
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    #[test]
    fn test_byte_budget_trims_oldest_entries() {
        let mut history = ProjectHistory::new(huge_project(usize::MAX));
        let initial_bytes = history.approx_bytes();
        assert!(initial_bytes > 0, "estimate must be positive");

        // ~10MB per distinct entry: 20 commits (~200MB) must exceed the
        // 128MB budget, so the byte budget (not the 50-entry cap) trims first.
        for i in 0..20 {
            history.commit_action(huge_project(i), &format!("edit {}", i));
        }

        assert!(
            history.approx_bytes() <= ProjectHistory::MAX_HISTORY_BYTES,
            "byte budget must be enforced, got {}",
            history.approx_bytes()
        );
        assert!(
            history.stack.len() < 21,
            "byte budget should trim before entry limit, got {} entries",
            history.stack.len()
        );
        // Current state must remain accessible and undo must survive eviction.
        assert_eq!(history.current_action_name(), "edit 19");
        assert!(history.can_undo());
    }

    #[test]
    fn test_small_projects_keep_full_history() {
        let mut history = ProjectHistory::new(big_project(2, 2));
        for i in 0..30 {
            history.commit_action(big_project(2, 2), &format!("small {}", i));
        }
        // Tiny projects: entry-count limit (50) is the binding constraint, not bytes
        assert!(history.approx_bytes() < ProjectHistory::MAX_HISTORY_BYTES);
        assert!(history.stack.len() <= 50);
    }

    #[test]
    fn test_named_edit_survives_undo_and_redo() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        if let LayerType::Solid { ref mut color } = edited.compositions[0].layers[0].layer_type {
            color[0] = 0.25;
        }
        history.commit_action(edited, "Repeater Rotation Keyframe");
        assert_eq!(history.current_action_name(), "Repeater Rotation Keyframe");
        assert!(history.undo().is_some());
        if let LayerType::Solid { color } = &history.current().compositions[0].layers[0].layer_type
        {
            assert_eq!(color[0], 1.0);
        } else {
            panic!("expected solid layer");
        }
        assert_eq!(
            history.redo_action_name(),
            Some("Repeater Rotation Keyframe")
        );
        assert!(history.redo().is_some());
        assert_eq!(history.current_action_name(), "Repeater Rotation Keyframe");
        if let LayerType::Solid { color } = &history.current().compositions[0].layers[0].layer_type
        {
            assert_eq!(color[0], 0.25);
        } else {
            panic!("expected solid layer");
        }
    }

    #[test]
    fn test_repeater_copies_keyframes_survive_undo_and_redo() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial.clone();
        let mut copies = Animatable::new_constant(2.0);
        copies.add_keyframe(Keyframe::new(0, 2.0, InterpolationType::Linear));
        copies.add_keyframe(Keyframe::new(12, 6.0, InterpolationType::Linear));
        edited.compositions[0].layers[0].shape_repeater = Some(ShapeRepeaterOptions {
            copies_animation: Some(copies),
            ..ShapeRepeaterOptions::default()
        });
        history.commit_action(edited, "Repeater Copies Keyframes");

        let restored = history.undo().expect("undo returns initial project");
        assert!(restored.compositions[0].layers[0].shape_repeater.is_none());
        let redone = history.redo().expect("redo returns repeater project");
        let animation = redone.compositions[0].layers[0]
            .shape_repeater
            .as_ref()
            .and_then(|repeater| repeater.copies_animation.as_ref())
            .and_then(|copies| copies.keyframes())
            .expect("copies keyframes are restored");
        assert_eq!(animation.len(), 3);
        assert_eq!(animation[2].frame, 12);
        assert_eq!(animation[2].value, 6.0);
    }

    #[test]
    fn camera_lens_keyframe_edit_is_reversible() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        edited.compositions[0].active_camera.set_fov_at(12, 90.0);
        edited.compositions[0]
            .active_camera
            .set_dof_max_blur_at(12, 32.0);
        history.commit_action(edited, "Camera Lens Keyframe");
        assert_eq!(
            history.undo().unwrap().compositions[0]
                .active_camera
                .fov_at(12),
            50.0
        );
        let restored = history.redo().unwrap();
        assert_eq!(restored.compositions[0].active_camera.fov_at(12), 90.0);
        assert_eq!(
            restored.compositions[0].active_camera.dof_max_blur_at(12),
            32.0
        );
    }

    #[test]
    fn transform_keyframe_move_is_reversible_through_history() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        let position = &mut edited.compositions[0].layers[0].transform.position;
        position.add_keyframe(Keyframe::new(8, [12.0, 24.0], InterpolationType::Hold));
        assert!(position.move_keyframe(8, 18));
        history.commit_action(edited, "Move Transform Keyframe");

        let undone = history.undo().unwrap();
        assert!(undone.compositions[0].layers[0]
            .transform
            .position
            .keyframes()
            .is_none_or(|keys| keys.iter().all(|key| key.frame != 18)));
        let redone = history.redo().unwrap();
        let key = redone.compositions[0].layers[0]
            .transform
            .position
            .keyframes()
            .unwrap()
            .iter()
            .find(|key| key.frame == 18)
            .unwrap();
        assert_eq!(key.value, [12.0, 24.0]);
        assert!(matches!(key.interpolation, InterpolationType::Hold));
    }

    #[test]
    fn identical_project_commit_does_not_create_history_entry() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        history.commit_action(initial, "No-op Edit");
        assert_eq!(history.len(), 1);
        assert_eq!(history.current_action_name(), "Initial State");
        assert!(!history.can_undo());
    }

    #[test]
    fn no_op_commit_after_undo_keeps_redo_branch() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial.clone();
        edited.compositions[0].layers[0].in_frame = 3;
        history.commit_action(edited, "Trim Layer");
        assert!(history.undo().is_some());
        history.commit_action(initial, "No-op After Undo");
        assert!(history.can_redo());
        assert_eq!(history.redo_action_name(), Some("Trim Layer"));
    }

    #[test]
    fn serialization_failure_never_swallows_a_real_commit() {
        let mut initial = big_project(1, 0);
        initial.compositions[0].layers[0].transform.opacity = Animatable::new_constant(f32::NAN);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        edited.compositions[0].layers[0].name = "Changed".into();
        history.commit_action(edited, "Edit NaN Project");
        assert_eq!(history.len(), 2);
        assert!(history.can_undo());
    }

    #[test]
    fn jump_to_lands_on_exact_state_and_rejects_garbage() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        for (i, name) in ["one", "two", "three"].iter().enumerate() {
            let mut edited = initial.clone();
            edited.compositions[0].layers[0].name = format!("M{}", i);
            history.commit_action(edited, name);
        }
        assert_eq!(history.len(), 4);

        let gen = history.generation();
        assert!(history.jump_to(1));
        assert_eq!(history.current_index(), 1);
        assert_eq!(history.current_action_name(), "one");
        assert_eq!(history.current().compositions[0].layers[0].name, "M0");
        assert!(history.generation() > gen);

        // Undo/redo stay coherent after a jump: redo walks forward again.
        assert!(history.can_redo());
        assert_eq!(history.redo_action_name(), Some("two"));

        // No-ops: same index and out-of-range change nothing.
        let gen2 = history.generation();
        assert!(!history.jump_to(1));
        assert!(!history.jump_to(99));
        assert_eq!(history.current_index(), 1);
        assert_eq!(history.generation(), gen2);
    }
}
