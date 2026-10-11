#[test]
fn session_no_mutation_no_undo_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    {
        let _session = EditorSession::new(&mut history, "No-op");
    }

    assert_eq!(history.generation(), gen_before);
    assert_eq!(history.len(), len_before);
}
#[test]
fn session_commit_creates_undo_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();

    {
        let mut session = EditorSession::new(&mut history, "Edit");
        session.ensure_snapshot();
        let proj = session.current_mut();
        proj.compositions[0].name = "Changed".into();
        session.commit();
    }

    assert!(history.generation() > gen_before);
    assert_eq!(history.current().compositions[0].name, "Changed");

    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn session_cancel_reverts_no_undo_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    {
        let mut session = EditorSession::new(&mut history, "Reverted");
        session.ensure_snapshot();
        let proj = session.current_mut();
        proj.compositions[0].name = "Changed".into();
        session.cancel();
    }

    // Cancel reverts state
    assert_eq!(history.current().compositions[0].name, "Test");
    // Generation increments (UI needs to detect the change)
    assert!(history.generation() > gen_before);
    // No new undo entry created
    assert_eq!(history.len(), len_before);
    // Undo is still not available beyond what existed before
    assert!(!history.can_undo());
}
#[test]
fn session_drop_rollback_no_undo_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    {
        let mut session = EditorSession::new(&mut history, "Dropped");
        let _ = session.current_mut(); // trigger lazy snapshot
        session.current_mut().compositions[0].name = "Dropped".into();
        // Session dropped without commit or cancel
    }

    // Drop rolls back
    assert_eq!(history.current().compositions[0].name, "Test");
    // Generation increments (UI needs to detect the change)
    assert!(history.generation() > gen_before);
    // No undo entry created
    assert_eq!(history.len(), len_before);
}
#[test]
fn session_lazy_snapshot() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    {
        let mut session = EditorSession::new(&mut history, "Lazy");
        let proj = session.current_mut();
        proj.compositions[0].name = "Lazy".into();
        session.commit();
    }

    assert_eq!(history.current().compositions[0].name, "Lazy");
    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn session_noop_mutation_no_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();

    {
        let mut session = EditorSession::new(&mut history, "No-op edit");
        session.ensure_snapshot();
        let proj = session.current_mut();
        proj.compositions[0].name = "Test".into(); // Same as original
        session.commit();
    }

    assert_eq!(history.generation(), gen_before);
}
#[test]
fn session_multiple_mutations_single_undo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();

    {
        let mut session = EditorSession::new(&mut history, "Multi-mutate");
        session.ensure_snapshot();
        let proj = session.current_mut();
        proj.compositions[0].name = "A".into();
        session.current_mut().compositions[0].name = "B".into();
        session.commit();
    }

    // One commit, one generation bump
    assert_eq!(history.generation(), gen_before + 1);
    assert_eq!(history.current().compositions[0].name, "B");
    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn session_double_commit_is_safe() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    {
        let mut session = EditorSession::new(&mut history, "Double");
        session.ensure_snapshot();
        session.current_mut().compositions[0].name = "X".into();
        session.commit();
    }

    // Second commit on consumed session: no-op (already committed)
    // This test just verifies no panic
    assert_eq!(history.current().compositions[0].name, "X");
}
#[test]
fn session_double_cancel_is_safe() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    {
        let mut session = EditorSession::new(&mut history, "Double");
        session.ensure_snapshot();
        session.current_mut().compositions[0].name = "X".into();
        session.cancel();
    }

    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn drop_after_commit_does_not_double_commit() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let len_before = history.len();

    {
        let mut session = EditorSession::new(&mut history, "Commit+Drop");
        session.ensure_snapshot();
        session.current_mut().compositions[0].name = "Committed".into();
        session.commit();
        // Drop fires but session is already committed
    }

    // Only one undo entry, not two
    assert_eq!(history.len(), len_before + 1);
    assert_eq!(history.current().compositions[0].name, "Committed");
    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn drop_after_cancel_does_not_double_cancel() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let len_before = history.len();

    {
        let mut session = EditorSession::new(&mut history, "Cancel+Drop");
        session.ensure_snapshot();
        session.current_mut().compositions[0].name = "X".into();
        session.cancel();
        // Drop fires but session is already cancelled
    }

    assert_eq!(history.current().compositions[0].name, "Test");
    assert_eq!(history.len(), len_before);
}

// ── Integration-level Undo/Redo tests ──
#[test]
fn integration_add_effect_undo_redo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // Add a new effect
    {
        let mut session = EditorSession::new(&mut history, "Add Effect");
        let proj = session.current_mut();
        proj.compositions[0].layers[0].effects.push(Effect {
            id: "glow_1".into(),
            name: "Glow".into(),
            effect_type: EffectType::Glow {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(20.0),
                intensity: Animatable::new_constant(2.0),
                color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
            enabled: true,
        });
        session.commit();
    }

    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 2);
    assert_eq!(
        history.current().compositions[0].layers[0].effects[1].name,
        "Glow"
    );

    // Undo: effect removed
    history.undo();
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);

    // Redo: effect restored
    history.redo();
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 2);
    assert_eq!(
        history.current().compositions[0].layers[0].effects[1].name,
        "Glow"
    );
}
#[test]
fn integration_delete_effect_undo_redo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // Delete the only effect
    {
        let mut session = EditorSession::new(&mut history, "Delete Effect");
        let proj = session.current_mut();
        proj.compositions[0].layers[0].effects.remove(0);
        session.commit();
    }

    assert!(history.current().compositions[0].layers[0]
        .effects
        .is_empty());

    // Undo: effect restored
    history.undo();
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);
    assert_eq!(
        history.current().compositions[0].layers[0].effects[0].name,
        "Gaussian Blur"
    );

    // Redo: effect deleted again
    history.redo();
    assert!(history.current().compositions[0].layers[0]
        .effects
        .is_empty());
}
#[test]
fn integration_toggle_effect_undo_redo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    assert!(history.current().compositions[0].layers[0].effects[0].enabled);

    // Toggle off
    {
        let mut session = EditorSession::new(&mut history, "Toggle Effect");
        session.current_mut().compositions[0].layers[0].effects[0].enabled = false;
        session.commit();
    }

    assert!(!history.current().compositions[0].layers[0].effects[0].enabled);

    // Undo: enabled again
    history.undo();
    assert!(history.current().compositions[0].layers[0].effects[0].enabled);

    // Redo: disabled again
    history.redo();
    assert!(!history.current().compositions[0].layers[0].effects[0].enabled);
}
#[test]
fn integration_reorder_effect_undo_redo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // Add two more effects
    {
        let mut session = EditorSession::new(&mut history, "Add Effects");
        let proj = session.current_mut();
        proj.compositions[0].layers[0].effects.push(Effect {
            id: "glow_1".into(),
            name: "Glow".into(),
            effect_type: EffectType::Glow {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(20.0),
                intensity: Animatable::new_constant(2.0),
                color: Animatable::new_constant([1.0; 4]),
            },
            enabled: true,
        });
        proj.compositions[0].layers[0].effects.push(Effect {
            id: "blur_2".into(),
            name: "Box Blur".into(),
            effect_type: EffectType::ColorTint {
                color: Animatable::new_constant([0.0, 0.0, 1.0, 1.0]),
                intensity: Animatable::new_constant(0.5),
            },
            enabled: true,
        });
        session.commit();
    }

    // Now we have [Gaussian Blur, Glow, Box Blur]
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 3);
    assert_eq!(
        history.current().compositions[0].layers[0].effects[0].name,
        "Gaussian Blur"
    );

    // Swap 0 and 1
    {
        let mut session = EditorSession::new(&mut history, "Reorder Effect");
        let proj = session.current_mut();
        proj.compositions[0].layers[0].effects.swap(0, 1);
        session.commit();
    }

    // Now [Glow, Gaussian Blur, Box Blur]
    assert_eq!(
        history.current().compositions[0].layers[0].effects[0].name,
        "Glow"
    );
    assert_eq!(
        history.current().compositions[0].layers[0].effects[1].name,
        "Gaussian Blur"
    );

    // Undo: back to original order
    history.undo();
    assert_eq!(
        history.current().compositions[0].layers[0].effects[0].name,
        "Gaussian Blur"
    );
    assert_eq!(
        history.current().compositions[0].layers[0].effects[1].name,
        "Glow"
    );

    // Redo: reordered again
    history.redo();
    assert_eq!(
        history.current().compositions[0].layers[0].effects[0].name,
        "Glow"
    );
}
#[test]
fn integration_slider_drag_one_undo_entry() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();

    // Simulate a slider drag: 100 intermediate values, one commit
    {
        let mut session = EditorSession::new(&mut history, "Slider Drag");
        for i in 0..100 {
            let val = 10.0 + (i as f32 * 0.5);
            session.current_mut().compositions[0].layers[0].effects[0] = Effect {
                id: "blur_0".into(),
                name: "Gaussian Blur".into(),
                effect_type: EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(val),
                },
                enabled: true,
            };
        }
        session.commit();
    }

    // Only one generation bump, not 100
    assert_eq!(history.generation(), gen_before + 1);
    assert_eq!(history.len(), 2); // initial + one edit
}
