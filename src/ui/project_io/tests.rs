use super::*;

#[test]
fn persisted_lut_path_loads_relative_to_project_and_reports_missing_files() {
    let root = std::env::temp_dir().join(format!("kagari-project-lut-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let project_path = root.join("project.json");
    let lut_path = root.join("look.cube");
    let cube = "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
    std::fs::write(&lut_path, cube).unwrap();

    let (lut, error) = load_project_lut(&project_path, Some("look.cube"));
    assert_eq!(lut.as_ref().map(|lut| lut.size), Some(2));
    assert!(error.is_none());

    let (lut, error) = load_project_lut(&project_path, Some("missing.cube"));
    assert!(lut.is_none());
    assert!(error.is_some());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_prefs_without_flag_default_to_showing_welcome() {
    // Prefs files written before the flag existed must keep old behavior.
    let parsed: RecentsFile = serde_json::from_str(r#"{"recent_projects":[]}"#).unwrap();
    assert!(parsed.show_welcome_on_startup);
    assert!(RecentsFile::default().show_welcome_on_startup);
    // Explicit false survives a serde roundtrip.
    let parsed: RecentsFile =
        serde_json::from_str(r#"{"recent_projects":[],"show_welcome_on_startup":false}"#)
            .unwrap();
    assert!(!parsed.show_welcome_on_startup);
}

#[test]
fn starred_projects_toggle_is_bounded_and_roundtrips() {
    let parsed: RecentsFile = serde_json::from_str(r#"{"recent_projects":[]}"#).unwrap();
    assert!(parsed.starred_projects.is_empty());

    let mut prefs = RecentsFile::default();
    assert!(toggle_starred_key(&mut prefs, "first.json".into()));
    assert!(!toggle_starred_key(&mut prefs, "first.json".into()));
    assert!(prefs.starred_projects.is_empty());

    for index in 0..=MAX_STARRED_PROJECTS {
        assert!(toggle_starred_key(
            &mut prefs,
            format!("project-{index}.json")
        ));
    }
    assert_eq!(prefs.starred_projects.len(), MAX_STARRED_PROJECTS);
    assert!(!prefs.starred_projects.iter().any(|p| p == "project-0.json"));
    assert!(prefs
        .starred_projects
        .iter()
        .any(|p| p == "project-100.json"));
}
