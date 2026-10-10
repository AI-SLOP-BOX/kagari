#[cfg(test)]
mod tests {
    use crate::core::automation::engine::ensure_collection_capacity;
    use crate::core::automation::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::Project;

    #[test]
    fn collection_limits_reject_the_next_automation_mutation() {
        let comp_limit = crate::core::production_document::ProductionDocument::MAX_COMPOSITIONS;
        let layer_limit =
            crate::core::production_document::ProductionDocument::MAX_LAYERS_PER_COMPOSITION;

        assert!(ensure_collection_capacity(comp_limit - 1, comp_limit, "compositions").is_ok());
        let comp_error = ensure_collection_capacity(comp_limit, comp_limit, "compositions")
            .unwrap_err()
            .to_string();
        assert!(comp_error.contains("compositions"));

        assert!(ensure_collection_capacity(layer_limit - 1, layer_limit, "layers").is_ok());
        let layer_error = ensure_collection_capacity(layer_limit, layer_limit, "layers")
            .unwrap_err()
            .to_string();
        assert!(layer_error.contains("layers"));
    }

    #[test]
    fn test_batch_creation_script() {
        let mut project = Project::default();
        let script = r#"
            new_comp("Titles", 1920, 1080, 30, 60);
            add_solid("BG", "101018");
            add_text("Hello", "こんにちは", 72);
            set_opacity("BG", 80);
            log("done");
        "#;
        let logs = run_script(&mut project, script).expect("script runs");
        assert!(logs.contains(&"done".to_string()));
        let comp = project.active_composition();
        assert_eq!(comp.name, "Titles");
        assert_eq!(comp.layers.len(), 2, "solid + text added");
        assert_eq!(project.compositions.len(), 2, "default + new");
    }

    #[test]
    fn test_keyframe_script_animates_position() {
        let mut project = Project::default();
        let script = r#"
            add_text("T", "Slide", 48);
            key_position("T", 0, -200.0, 540.0);
            key_position("T", 30, 960.0, 540.0);
        "#;
        run_script(&mut project, script).unwrap();
        let comp = project.active_composition();
        let layer = comp.layers.last().unwrap();
        match &layer.transform.position {
            Animatable::Animated(kfs) => {
                assert_eq!(kfs.len(), 2);
                assert_eq!(kfs[0].frame, 0);
                assert_eq!(kfs.last().unwrap().frame, 30);
            }
            _ => panic!("expected animated position"),
        }
    }

    #[test]
    fn test_hex_parsing() {
        assert_eq!(parse_hex("#FF0000"), [1.0, 0.0, 0.0]);
        assert_eq!(parse_hex("00ff00"), [0.0, 1.0, 0.0]);
        assert_eq!(parse_hex("zzz"), [1.0, 1.0, 1.0], "fallback white");
    }

    #[test]
    fn test_error_propagates_and_unsets_pointer() {
        let mut project = Project::default();
        let err = run_script(&mut project, "this_is_not_a_fn();").unwrap_err();
        assert!(err.contains("script error"));
        // Pointer cleared: further calls safe
        assert!(with_project(|_| 1).is_none());
    }

    #[test]
    fn test_script_failure_rolls_back_all_mutations() {
        let mut project = Project::default();
        let original = project.clone();
        let err = run_script(
            &mut project,
            r#"
                new_comp("partial", 1920, 1080, 30, 60);
                add_text("T", "must not commit", 48);
                missing_function();
            "#,
        )
        .unwrap_err();

        assert!(err.contains("script error"));
        assert_eq!(
            serde_json::to_value(&project).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }

    #[test]
    fn test_project_scope_clears_pointer_during_unwind() {
        let mut project = Project::default();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = ProjectScope::enter(&mut project).unwrap();
            panic!("simulated native callback panic");
        }));

        assert!(result.is_err());
        assert!(with_project(|_| 1).is_none());
    }

    #[test]
    fn test_nested_project_scope_is_rejected() {
        let mut first = Project::default();
        let mut second = Project::default();
        let _scope = ProjectScope::enter(&mut first).unwrap();

        let error = ProjectScope::enter(&mut second).unwrap_err();
        assert!(error.contains("nested automation"));
    }

    #[test]
    fn test_mutations_fail_closed_when_active_composition_is_missing() {
        let mut project = Project::default();
        project.compositions.clear();
        let script = r#"
            add_solid("BG", "ffffff");
            add_text("T", "safe", 32);
            set_opacity("BG", 50);
            key_position("T", 10, 1.0, 2.0);
        "#;
        assert!(run_script(&mut project, script).is_err());
        assert!(project.compositions.is_empty());
    }

    #[test]
    fn test_transform_mutations_sanitize_nonfinite_numbers() {
        let mut project = Project::default();
        run_script(
            &mut project,
            r#"
                add_text("T", "safe", 32);
                set_position("T", parse_float("NaN"), parse_float("Inf"));
                set_opacity("T", parse_float("NaN"));
                key_position("T", 1, parse_float("NaN"), parse_float("Inf"));
            "#,
        )
        .unwrap();
        let layer = project.active_composition().layers.last().unwrap();
        assert!(layer
            .transform
            .position
            .evaluate(1)
            .iter()
            .all(|v| v.is_finite()));
        assert!(layer.transform.opacity.evaluate(1).is_finite());
    }

    #[test]
    fn test_script_bounds_extreme_integer_inputs() {
        let mut project = Project::default();
        let original = project.clone();
        let result = run_script(
            &mut project,
            r#"
                new_comp("bounded", 9223372036854775807, -5, 0, 9223372036854775807);
                add_text("T", "safe", 9223372036854775807);
                key_position("T", 9223372036854775807, 1.0, 2.0);
            "#,
        );
        assert!(result.is_err());
        assert_eq!(
            serde_json::to_value(&project).unwrap(),
            serde_json::to_value(&original).unwrap(),
            "invalid script mutations must roll back"
        );
    }

    #[test]
    fn test_script_source_size_is_bounded() {
        let mut project = Project::default();
        let original = project.clone();
        let oversized = " ".repeat(MAX_SCRIPT_SOURCE_BYTES + 1);
        assert!(run_script(&mut project, &oversized).is_err());
        assert_eq!(
            serde_json::to_value(&project).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
}
