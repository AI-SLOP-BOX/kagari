#[test]
fn live_sources_override_saved_curves_in_frame_snapshot() {
    let mut document = ProductionDocument::new(Project::default());
    document.bindings = vec![AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.layer.layer_bg.opacity".into(),
        curve: crate::core::automation_binding::AutomationCurve::from_events([
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.0,
            },
        ])
        .unwrap(),
        input_min: 0.0,
        input_max: 1.0,
        output_min: 10.0,
        output_max: 90.0,
    }];
    let live_sources = HashMap::from([(String::from("audio.bass"), 1.0)]);

    let snapshot = document
        .composition_for_frame_with_sources(0, 0, &live_sources)
        .unwrap();
    assert_eq!(snapshot.layers[0].transform.opacity.evaluate(0), 90.0);
}
#[test]
fn applies_transform_and_effect_targets_as_keyframes() {
    let mut project = Project::default();
    let layer_id = project.compositions[0].layers[0].id.clone();
    let layer = &mut project.compositions[0].layers[0];
    let effect = crate::core::timeline::Effect {
        id: "blur-1".into(),
        name: "Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: crate::core::property::Animatable::new_constant(2.0),
        },
        enabled: true,
    };
    layer.effects.push(effect);
    let mut document = ProductionDocument::new(project);
    let targets = HashMap::from([
        (format!("vfx.layer.{layer_id}.opacity"), 75.0),
        (format!("vfx.layer.{layer_id}.position.x"), 320.0),
        (
            format!("vfx.layer.{layer_id}.effect.blur-1.blur_radius"),
            18.0,
        ),
        (format!("vfx.layer.{layer_id}.effect.blur-1.enabled"), 0.0),
        (format!("vfx.layer.{layer_id}.enabled"), 0.0),
        (format!("vfx.layer.{layer_id}.motion_blur"), 1.0),
        (format!("vfx.layer.{layer_id}.anchor_point.x"), 12.0),
        (format!("vfx.layer.{layer_id}.position.z"), 48.0),
        (format!("vfx.layer.{layer_id}.rotation.x"), 10.0),
        (format!("vfx.layer.{layer_id}.rotation.y"), 20.0),
        (format!("vfx.layer.{layer_id}.rotation.z"), 30.0),
        (format!("vfx.layer.{layer_id}.scale.z"), 125.0),
    ]);
    assert_eq!(document.apply_binding_targets_at_frame(&targets, 12), 12);
    let layer = &document.project.compositions[0].layers[0];
    assert_eq!(layer.transform.opacity.evaluate(12), 75.0);
    assert_eq!(layer.transform.position.evaluate(12)[0], 320.0);
    assert!(!layer.visible);
    assert!(layer.motion_blur);
    assert_eq!(layer.transform.anchor_point.evaluate(12)[0], 12.0);
    assert_eq!(layer.transform_3d.position.evaluate(12)[2], 48.0);
    assert_eq!(layer.transform_3d.rotation.evaluate(12), [10.0, 20.0, 30.0]);
    assert_eq!(layer.transform_3d.scale.evaluate(12)[2], 125.0);
    match &layer.effects[0].effect_type {
        EffectType::GaussianBlur { blur_radius } => {
            assert_eq!(blur_radius.evaluate(12), 18.0)
        }
        _ => panic!("expected Gaussian Blur"),
    }
    assert!(!layer.effects[0].enabled);
}
#[test]
fn applies_targets_to_layer_ids_containing_dots() {
    let mut project = Project::default();
    project.compositions[0].layers[0].id = "hero.main".into();
    let mut document = ProductionDocument::new(project);
    let targets = HashMap::from([(String::from("vfx.layer.hero.main.opacity"), 42.0)]);

    assert_eq!(document.apply_binding_targets_at_frame(&targets, 7), 1);
    assert_eq!(
        document.project.compositions[0].layers[0]
            .transform
            .opacity
            .evaluate(7),
        42.0
    );
}
#[test]
fn binding_time_targets_remain_inside_valid_layer_range() {
    let mut document = ProductionDocument::new(Project::default());
    let layer_id = document.project.compositions[0].layers[0].id.clone();
    let targets = HashMap::from([
        (format!("vfx.layer.{layer_id}.in_frame"), 500.0),
        (format!("vfx.layer.{layer_id}.out_frame"), -10.0),
    ]);

    assert_eq!(document.apply_binding_targets_at_frame(&targets, 0), 2);
    let layer = &document.project.compositions[0].layers[0];
    assert!(layer.in_frame < layer.out_frame);
    assert!(layer.out_frame <= document.project.compositions[0].duration_frames);
}
#[test]
fn camera_binding_clamps_optical_parameters_per_composition() {
    let mut document = ProductionDocument::new(Project::default());
    let comp_id = document.project.compositions[0].id.clone();
    let targets = HashMap::from([
        (format!("vfx.comp.{comp_id}.camera.fov"), 500.0),
        (format!("vfx.comp.{comp_id}.camera.focus_distance"), -2.0),
        (format!("vfx.comp.{comp_id}.camera.aperture"), -1.0),
        (format!("vfx.comp.{comp_id}.camera.dof_enabled"), 1.0),
        (format!("vfx.comp.{comp_id}.camera.dof_max_blur"), 500.0),
        (format!("vfx.comp.{comp_id}.camera.position.z"), 120.0),
        (format!("vfx.comp.{comp_id}.camera.rotation.y"), 35.0),
        (format!("vfx.comp.{comp_id}.camera.scale.x"), 110.0),
    ]);

    assert_eq!(document.apply_binding_targets_at_frame(&targets, 0), 8);
    let camera = &document.project.compositions[0].active_camera;
    assert_eq!(camera.fov_degrees, 179.0);
    assert_eq!(camera.focus_distance, 0.0);
    assert_eq!(camera.aperture, 0.0);
    assert!(camera.dof_enabled);
    assert_eq!(camera.dof_max_blur, 64.0);
    assert_eq!(camera.transform.position.evaluate(0)[2], 120.0);
    assert_eq!(camera.transform.rotation.evaluate(0)[1], 35.0);
    assert_eq!(camera.transform.scale.evaluate(0)[0], 110.0);
}
#[test]
fn camera_binding_targets_selected_scene_camera_not_legacy_fallback() {
    let mut document = ProductionDocument::new(Project::default());
    let comp = &mut document.project.compositions[0];
    comp.cameras.push(crate::core::timeline::Camera3D {
        name: "Wide".into(),
        ..crate::core::timeline::Camera3D::default()
    });
    comp.set_active_camera(Some(0));
    let comp_id = comp.id.clone();
    let targets = HashMap::from([(format!("vfx.comp.{comp_id}.camera.fov"), 92.0)]);

    assert_eq!(document.apply_binding_targets_at_frame(&targets, 0), 1);
    assert_eq!(
        document.project.compositions[0].cameras[0].fov_degrees,
        92.0
    );
    assert_eq!(
        document.project.compositions[0].active_camera.fov_degrees,
        50.0
    );
}
#[test]
fn sample_automation_binding_is_available_on_video_timeline() {
    let mut document = ProductionDocument::new(Project::default());
    document.audio.sample_rate = 48_000;
    document
        .add_sample_automation_binding(
            "audio.bass",
            "vfx.glow.intensity",
            [(0, 0.0), (24_000, 1.0)],
            (0.0, 1.0),
            (0.0, 100.0),
        )
        .unwrap();

    let values = document.evaluate_bindings_at_time(crate::core::unified_time::Time::new(1, 4));
    assert_eq!(values.get("vfx.glow.intensity"), Some(&50.0));
}
#[test]
fn validates_all_bindings_with_an_actionable_index() {
    let mut document = ProductionDocument::new(Project::default());
    document.bindings.push(AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.glow.intensity".into(),
        curve: crate::core::automation_binding::AutomationCurve {
            points: vec![crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.0,
            }],
        },
        input_min: 1.0,
        input_max: 1.0,
        output_min: 0.0,
        output_max: 100.0,
    });
    assert_eq!(
        document.validate_bindings().unwrap_err(),
        "binding 0: automation input range must be increasing"
    );
}
