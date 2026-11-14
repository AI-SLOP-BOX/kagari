use super::*;

#[test]
fn project_edits_during_a_drag_advance_preview_revision_without_committing_history() {
    let mut app = AfterEffectsStudio::new().expect("Studio should initialize");
    app.history_interaction_snapshot = Some(app.project_model.clone());
    let preview_revision = app.preview_revision;
    let history_generation = app.history.generation();

    app.mark_project_changed();

    assert_eq!(
        app.preview_revision,
        preview_revision.wrapping_add(1),
        "interactive edits must invalidate the Studio's stale viewport result"
    );
    assert_eq!(app.history.generation(), history_generation);
    assert!(app.history_interaction_snapshot.is_some());
}

#[test]
fn committed_project_edits_advance_preview_and_history_generations_once() {
    let mut app = AfterEffectsStudio::new().expect("Studio should initialize");
    app.project_model.active_composition_mut().name = "Changed composition".into();
    let preview_revision = app.preview_revision;
    let history_generation = app.history.generation();

    app.mark_project_changed();

    assert_eq!(
        app.preview_revision,
        preview_revision.wrapping_add(1),
        "committing an edit must invalidate the Studio's stale viewport result"
    );
    assert_eq!(
        app.history.generation(),
        history_generation.wrapping_add(1),
        "one committed edit must create one history generation"
    );
    assert!(app.history.can_undo());
}

#[test]
fn loading_a_project_advances_project_and_preview_generations_once() {
    let mut app = AfterEffectsStudio::new().expect("Studio should initialize");
    let project_generation = app.project_generation;
    let preview_revision = app.preview_revision;
    let project = kagari_vfx::core::timeline::Project::empty_for_new_project();

    app.install_loaded_project(project, "cache-load", None)
        .expect("project should install");

    assert_eq!(
        app.project_generation,
        project_generation.wrapping_add(1),
        "loading a project must advance its identity generation once"
    );
    assert_eq!(
        app.preview_revision,
        preview_revision.wrapping_add(1),
        "loading a project must invalidate the Studio's stale viewport result"
    );
}
