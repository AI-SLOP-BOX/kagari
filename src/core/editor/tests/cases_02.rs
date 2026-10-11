#[test]
fn integration_two_separate_drags_two_entries() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // First drag
    {
        let mut session = EditorSession::new(&mut history, "Drag 1");
        session.current_mut().compositions[0].name = "After Drag 1".into();
        session.commit();
    }

    // Second drag
    {
        let mut session = EditorSession::new(&mut history, "Drag 2");
        session.current_mut().compositions[0].name = "After Drag 2".into();
        session.commit();
    }

    assert_eq!(history.len(), 3); // initial + 2 edits
    assert_eq!(history.current().compositions[0].name, "After Drag 2");

    history.undo();
    assert_eq!(history.current().compositions[0].name, "After Drag 1");

    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");
}
#[test]
fn integration_noop_action_zero_entries() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    // Multiple no-op sessions
    for _ in 0..10 {
        let mut session = EditorSession::new(&mut history, "No-op");
        session.ensure_snapshot();
        // Don't actually change anything
        session.commit();
    }

    assert_eq!(history.generation(), gen_before);
    assert_eq!(history.len(), len_before);
}
#[test]
fn integration_jump_to_generation() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // Make some edits to have multiple entries
    for i in 0..5 {
        let mut session = EditorSession::new(&mut history, format!("Edit {}", i));
        session.current_mut().compositions[0].name = format!("State {}", i);
        session.commit();
    }

    let gen_before_jump = history.generation();

    // Jump to index 2 (State 1, since index 0 = "Test", 1 = "State 0", 2 = "State 1")
    assert!(history.jump_to(2));
    assert!(history.generation() > gen_before_jump);
    assert_eq!(history.current().compositions[0].name, "State 1");

    // Jump to same index: no-op
    let gen_after = history.generation();
    assert!(!history.jump_to(2));
    assert_eq!(history.generation(), gen_after);
}
#[test]
fn integration_cancel_exact_restoration() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    // Make a complex mutation, then cancel
    {
        let mut session = EditorSession::new(&mut history, "Complex Edit");
        let proj = session.current_mut();
        proj.compositions[0].name = "Changed".into();
        proj.compositions[0].layers[0].name = "Changed Layer".into();
        proj.compositions[0].layers[0].effects.push(Effect {
            id: "new_effect".into(),
            name: "New".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(99.0),
            },
            enabled: false,
        });
        session.cancel();
    }

    // Exact restoration
    assert_eq!(history.current().compositions[0].name, "Test");
    assert_eq!(history.current().compositions[0].layers[0].name, "Layer0");
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);

    // No undo entry created
    assert_eq!(history.len(), len_before);
    // Generation advances (UI needs to detect the change)
    assert!(history.generation() > gen_before);
    // Undo is not available (nothing to undo)
    assert!(!history.can_undo());
}
#[test]
fn integration_dropped_transaction_rollback() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen_before = history.generation();
    let len_before = history.len();

    // Simulate error/early return: mutate, then let session drop
    {
        let mut session = EditorSession::new(&mut history, "Dropped Edit");
        let proj = session.current_mut();
        proj.compositions[0].name = "Should Be Reverted".into();
        proj.compositions[0].layers[0].effects.clear();
        // Drop without commit → rollback
    }

    // Exact restoration
    assert_eq!(history.current().compositions[0].name, "Test");
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);
    assert_eq!(history.len(), len_before);
    assert!(history.generation() > gen_before);
    assert!(!history.can_undo());
}

// ── Generation semantics tests ──
#[test]
fn generation_increments_on_commit() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen0 = history.generation();

    let mut session = EditorSession::new(&mut history, "Edit");
    session.current_mut().compositions[0].name = "A".into();
    session.commit();

    assert_eq!(history.generation(), gen0 + 1);
}
#[test]
fn generation_increments_on_undo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    let mut session = EditorSession::new(&mut history, "Edit");
    session.current_mut().compositions[0].name = "A".into();
    session.commit();

    let gen_after_commit = history.generation();
    history.undo();
    assert_eq!(history.generation(), gen_after_commit + 1);
}
#[test]
fn generation_increments_on_redo() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    let mut session = EditorSession::new(&mut history, "Edit");
    session.current_mut().compositions[0].name = "A".into();
    session.commit();

    history.undo();
    let gen_after_undo = history.generation();
    history.redo();
    assert_eq!(history.generation(), gen_after_undo + 1);
}
#[test]
fn generation_increments_on_jump_to() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    let mut session = EditorSession::new(&mut history, "Edit");
    session.current_mut().compositions[0].name = "A".into();
    session.commit();

    let gen_before = history.generation();
    history.jump_to(0);
    assert_eq!(history.generation(), gen_before + 1);
}
#[test]
fn generation_not_advanced_on_noop_commit() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);
    let gen0 = history.generation();

    // Commit identical state
    let same = history.current().clone();
    history.commit_action(same, "No-op");
    assert_eq!(history.generation(), gen0);
}
#[test]
fn undo_redo_after_jump_to_remains_correct() {
    let project = make_test_project();
    let mut history = ProjectHistory::new(project);

    // Create entries: [Initial, A, B, C]
    for name in &["A", "B", "C"] {
        let mut session = EditorSession::new(&mut history, "Edit");
        session.current_mut().compositions[0].name = name.to_string();
        session.commit();
    }

    // Jump to index 1 (A)
    history.jump_to(1);
    assert_eq!(history.current().compositions[0].name, "A");

    // Undo from A → Initial
    history.undo();
    assert_eq!(history.current().compositions[0].name, "Test");

    // Redo → A
    history.redo();
    assert_eq!(history.current().compositions[0].name, "A");

    // Redo → B
    history.redo();
    assert_eq!(history.current().compositions[0].name, "B");

    // Undo → A
    history.undo();
    assert_eq!(history.current().compositions[0].name, "A");
}

// ── Large-project performance test ──
#[test]
fn large_project_idle_zero_clone_cost() {
    use std::time::Instant;

    // Build a large project: 300 layers, 10 keyframes each
    let mut comp = Composition::new("c".into(), "Big".into(), 1920, 1080, 30, 300);
    for i in 0..300 {
        let mut l = crate::core::timeline::Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            crate::core::timeline::LayerType::Solid { color: [1.0; 4] },
            300,
        );
        let kfs: Vec<crate::core::keyframe::Keyframe<[f32; 2]>> = (0..10)
            .map(|k| {
                crate::core::keyframe::Keyframe::new(
                    k * 30,
                    [k as f32 * 10.0, k as f32 * 5.0],
                    crate::core::keyframe::InterpolationType::Linear,
                )
            })
            .collect();
        l.transform.position = crate::core::property::Animatable::new_animated(kfs);
        comp.layers.push(l);
    }
    let project = Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };

    let mut history = ProjectHistory::new(project.clone());

    // Simulate 1000 idle frames: create session, don't mutate, drop.
    // This must NOT clone the project.
    let start = Instant::now();
    for _ in 0..1000 {
        let session = EditorSession::new(&mut history, "Idle");
        // No current_mut() call → no snapshot captured → no clone
        drop(session);
    }
    let elapsed = start.elapsed();

    // 1000 idle frames should complete in well under 100ms
    // (no cloning, just session creation/destruction)
    assert!(
        elapsed.as_millis() < 100,
        "1000 idle frames took {:?} — expected <100ms (possible unwanted clone)",
        elapsed
    );

    // Verify no undo entries were created
    assert_eq!(history.len(), 1);
}
#[test]
fn large_project_discrete_action_cost() {
    use std::time::Instant;

    // Build a medium project: 100 layers (500 is too slow for CI due to
    // per-commit project cloning, but this validates the O(1) idle path).
    let mut comp = Composition::new("c".into(), "Medium".into(), 1920, 1080, 30, 300);
    for i in 0..100 {
        let mut l = crate::core::timeline::Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            crate::core::timeline::LayerType::Solid { color: [1.0; 4] },
            300,
        );
        l.effects.push(Effect {
            id: format!("blur_{}", i),
            name: format!("Blur {}", i),
            effect_type: EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(10.0),
            },
            enabled: true,
        });
        comp.layers.push(l);
    }
    let project = Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };

    let mut history = ProjectHistory::new(project);

    // Perform 20 discrete toggle actions
    let start = Instant::now();
    for i in 0..20 {
        let mut session = EditorSession::new(&mut history, "Toggle");
        session.current_mut().compositions[0].layers[i].effects[0].enabled = false;
        session.commit();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 5000,
        "20 toggles on 100-layer project took {:?}",
        elapsed
    );

    // Verify correct state
    for i in 0..20 {
        assert!(!history.current().compositions[0].layers[i].effects[0].enabled);
    }
    for i in 20..100 {
        assert!(history.current().compositions[0].layers[i].effects[0].enabled);
    }

    // Verify all 20 undos work
    for _ in 0..20 {
        history.undo();
    }
    for i in 0..100 {
        assert!(history.current().compositions[0].layers[i].effects[0].enabled);
    }
}
