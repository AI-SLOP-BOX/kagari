#[test]
fn rejects_unbounded_audio_and_binding_collections() {
    let mut document = ProductionDocument::new(Project::default());
    document.audio.channels.resize(
        ProductionDocument::MAX_AUDIO_CHANNELS + 1,
        MixerChannel::default(),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.bindings.resize(
        ProductionDocument::MAX_BINDINGS + 1,
        AutomationBinding {
            source: "audio.x".into(),
            target: "vfx.y".into(),
            curve: crate::core::automation_binding::AutomationCurve {
                points: vec![crate::core::automation_binding::AutomationPoint {
                    time: crate::core::unified_time::Time::ZERO,
                    value: 0.0,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
        },
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.assets.resize(
        ProductionDocument::MAX_ASSETS + 1,
        crate::core::timeline::ProjectItem::new(
            "asset",
            "Asset",
            ProjectItemType::Folder { name: "x".into() },
        ),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].layers.resize(
        ProductionDocument::MAX_LAYERS_PER_COMPOSITION + 1,
        crate::core::timeline::Layer::new("layer".into(), "Layer".into(), LayerType::Null, 1),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].sub_compositions.resize(
        ProductionDocument::MAX_SUB_COMPOSITIONS_PER_COMPOSITION + 1,
        crate::core::timeline::Composition::new("nested".into(), "Nested".into(), 1, 1, 1, 1),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].cameras.resize(
        ProductionDocument::MAX_CAMERAS_PER_COMPOSITION + 1,
        Default::default(),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].markers.resize(
        ProductionDocument::MAX_MARKERS_PER_COMPOSITION + 1,
        crate::core::timeline::TimelineMarker {
            frame: 0,
            label: "Marker".into(),
            color: [0.0, 0.0, 0.0],
        },
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].lights.resize(
        ProductionDocument::MAX_LIGHTS_PER_COMPOSITION + 1,
        Default::default(),
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].layers[0].effects.resize(
        ProductionDocument::MAX_EFFECTS_PER_LAYER + 1,
        crate::core::timeline::Effect {
            id: "effect".into(),
            name: "Blur".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: crate::core::property::Animatable::new_constant(1.0),
            },
            enabled: true,
        },
    );
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.project.compositions[0].layers[0].masks.resize(
        ProductionDocument::MAX_MASKS_PER_LAYER + 1,
        crate::core::mask::Mask::new_rect("mask".into(), "Mask".into(), 0.0, 0.0, 1.0, 1.0),
    );
    assert!(document.validate().is_err());
}
#[test]
fn rejects_unsorted_or_duplicate_animation_keyframes() {
    for frames in [[10, 5], [5, 5]] {
        let mut document = ProductionDocument::new(Project::default());
        document.project.compositions[0].layers[0].transform.opacity =
            Animatable::new_animated(vec![
                Keyframe::new(frames[0], 20.0, InterpolationType::Linear),
                Keyframe::new(frames[1], 80.0, InterpolationType::Linear),
            ]);
        assert!(document.validate().is_err());
    }
}
#[test]
fn rejects_oversized_asset_metadata() {
    let mut document = ProductionDocument::new(Project::default());
    document
        .project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "asset-id",
            "Asset",
            ProjectItemType::Image {
                path: "x".repeat(ProductionDocument::MAX_ASSET_STRING_LENGTH + 1),
                width: 1,
                height: 1,
            },
        ));
    assert!(document.validate().is_err());
}
#[test]
fn rejects_control_characters_in_asset_metadata() {
    let mut document = ProductionDocument::new(Project::default());
    document
        .project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "asset\0id",
            "Asset",
            ProjectItemType::Folder {
                name: "Folder".into(),
            },
        ));
    assert!(document.validate().is_err());
}
#[test]
fn rejects_untrimmed_asset_metadata() {
    let mut document = ProductionDocument::new(Project::default());
    document
        .project
        .assets
        .push(crate::core::timeline::ProjectItem::new(
            "asset-id",
            " Asset ",
            ProjectItemType::Folder {
                name: "Folder".into(),
            },
        ));
    assert!(document.validate().is_err());
}
#[test]
fn gradient_stops_require_matching_sorted_unit_interval_values() {
    let colors = [[0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]];
    assert!(valid_gradient_stops(&colors, &[0.0, 1.0]));
    assert!(!valid_gradient_stops(&colors, &[0.8, 0.2]));
    assert!(!valid_gradient_stops(&colors, &[-0.1, 1.0]));
    assert!(!valid_gradient_stops(&colors, &[0.0, 1.1]));
    assert!(!valid_gradient_stops(&colors, &[0.0]));
    assert!(!valid_gradient_stops(&[[0.0, 0.0, 0.0, 1.0]], &[0.0]));
}
#[test]
fn effect_finite_check_does_not_match_field_names() {
    assert!(effect_debug_is_finite("Bezier { influence: 0.333 }"));
    assert!(!effect_debug_is_finite("GaussianBlur { radius: NaN }"));
    assert!(!effect_debug_is_finite("GaussianBlur { radius: -inf }"));
}
#[test]
fn particle_emitter_validation_rejects_invalid_ranges_and_curves() {
    let mut emitter = crate::core::particle_system::ParticleEmitter::default();
    assert!(particle_emitter_is_valid(&emitter));
    emitter.spread_degrees = 361.0;
    assert!(!particle_emitter_is_valid(&emitter));
    emitter.spread_degrees = 0.0;
    emitter.max_particles = 10_000_001;
    assert!(!particle_emitter_is_valid(&emitter));
    emitter.max_particles = 10_000;
    emitter.gravity_curve.0[0] = f32::NAN;
    assert!(!particle_emitter_is_valid(&emitter));
    emitter.gravity_curve.0[0] = 1.0;
    emitter.collision_enabled = true;
    emitter.collision_bounds = [10.0, 10.0, 0.0, 0.0];
    assert!(!particle_emitter_is_valid(&emitter));
    emitter.collision_bounds = [0.0, 0.0, 10.0, 10.0];
    emitter.color_start[3] = 1.1;
    assert!(!particle_emitter_is_valid(&emitter));
}
#[test]
fn spot_light_validation_bounds_cone_settings() {
    assert!(light_type_is_valid(
        &crate::core::timeline::LightType::Spot {
            cone_angle_deg: 45.0,
            cone_feather_pct: 50.0,
        }
    ));
    assert!(!light_type_is_valid(
        &crate::core::timeline::LightType::Spot {
            cone_angle_deg: 181.0,
            cone_feather_pct: 50.0,
        }
    ));
    assert!(!light_type_is_valid(
        &crate::core::timeline::LightType::Spot {
            cone_angle_deg: 45.0,
            cone_feather_pct: -1.0,
        }
    ));
}
#[test]
fn atomic_save_and_load_preserve_contract() {
    let directory =
        std::env::temp_dir().join(format!("kagari_production_document_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.aura");
    let document = ProductionDocument::new(Project::default());

    document.save_atomic(&path).unwrap();
    let loaded = ProductionDocument::load(&path).unwrap();
    assert_eq!(loaded.schema_version, document.schema_version);
    assert!(path.exists());
    assert!(!directory.join("session.production.tmp").exists());

    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn failed_atomic_replace_removes_temporary_document() {
    let directory = std::env::temp_dir().join(format!(
        "kagari_production_document_failure_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let target = directory.join("session.aura");
    std::fs::create_dir(&target).unwrap();

    let document = ProductionDocument::new(Project::default());
    assert!(document.save_atomic(&target).is_err());

    let temporary_files = std::fs::read_dir(&directory)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|name| name.to_string_lossy().starts_with("session.production."))
        .collect::<Vec<_>>();
    assert!(
        temporary_files.is_empty(),
        "temporary files: {temporary_files:?}"
    );

    let _ = std::fs::remove_dir_all(directory);
}
#[test]
fn upgrades_legacy_project_json_with_safe_defaults() {
    let legacy =
        crate::core::project_migration::save_project_versioned(&Project::default()).unwrap();
    let document = ProductionDocument::from_legacy_project_json(&legacy).unwrap();

    assert_eq!(document.project().compositions.len(), 1);
    assert_eq!(document.audio.sample_rate, 48_000);
    assert!(document.bindings.is_empty());
}
#[test]
fn legacy_migration_rejects_unusable_project() {
    let mut project = Project::default();
    project.active_composition_idx = project.compositions.len();
    let legacy = crate::core::project_migration::save_project_versioned(&project).unwrap();
    assert!(ProductionDocument::from_legacy_project_json(&legacy).is_err());
}
#[test]
fn partial_production_documents_receive_domain_defaults() {
    let mut value: serde_json::Value = serde_json::from_str(
        &serde_json::to_string(&ProductionDocument::new(Project::default())).unwrap(),
    )
    .unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("audio");
    object.remove("tempo");
    object.remove("bindings");
    let document = ProductionDocument::from_json(&serde_json::to_string(&value).unwrap()).unwrap();
    assert_eq!(document.audio.sample_rate, 48_000);
    assert_eq!(
        document
            .tempo
            .beat_at(crate::core::unified_time::Time::new(1, 1)),
        2.0
    );
    assert!(document.bindings.is_empty());
}
#[test]
fn rejects_track_matte_without_source_layer() {
    let mut project = Project::default();
    project.compositions[0].layers[0].track_matte =
        crate::core::timeline::TrackMatteMode::AlphaMatte;
    assert!(ProductionDocument::new(project).validate().is_err());
}
#[test]
fn evaluates_multiple_automation_bindings_from_live_sources() {
    let mut document = ProductionDocument::new(Project::default());
    let curve = crate::core::automation_binding::AutomationCurve {
        points: vec![crate::core::automation_binding::AutomationPoint {
            time: crate::core::unified_time::Time::ZERO,
            value: 0.0,
        }],
    };
    document.bindings = vec![
        AutomationBinding {
            source: "audio.bass".into(),
            target: "vfx.glow.intensity".into(),
            curve: curve.clone(),
            input_min: 0.0,
            input_max: 1.0,
            output_min: 10.0,
            output_max: 50.0,
        },
        AutomationBinding {
            source: "midi.cc1".into(),
            target: "vfx.particles.rate".into(),
            curve,
            input_min: 0.0,
            input_max: 127.0,
            output_min: 0.0,
            output_max: 1000.0,
        },
    ];
    let sources = HashMap::from([
        ("audio.bass".into(), 0.5),
        ("midi.cc1".into(), 127.0),
        ("unbound".into(), f64::NAN),
    ]);
    let targets = document.evaluate_bindings(&sources);
    assert_eq!(targets.get("vfx.glow.intensity"), Some(&30.0));
    assert_eq!(targets.get("vfx.particles.rate"), Some(&1000.0));
    assert!(!targets.contains_key("unbound"));
}
#[test]
fn evaluates_saved_binding_curves_at_shared_time() {
    let mut document = ProductionDocument::new(Project::default());
    document.bindings = vec![AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.layer.layer_bg.opacity".into(),
        curve: crate::core::automation_binding::AutomationCurve::from_events([
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.0,
            },
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::new(1, 1),
                value: 1.0,
            },
        ])
        .unwrap(),
        input_min: 0.0,
        input_max: 1.0,
        output_min: 20.0,
        output_max: 80.0,
    }];

    let targets = document.evaluate_bindings_at_time(crate::core::unified_time::Time::new(1, 2));
    assert_eq!(targets.get("vfx.layer.layer_bg.opacity"), Some(&50.0));
}
#[test]
fn composition_snapshot_applies_saved_bindings_without_mutating_document() {
    let mut document = ProductionDocument::new(Project::default());
    document.bindings = vec![AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.layer.layer_bg.opacity".into(),
        curve: crate::core::automation_binding::AutomationCurve::from_events([
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.0,
            },
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::new(10, 1),
                value: 1.0,
            },
        ])
        .unwrap(),
        input_min: 0.0,
        input_max: 1.0,
        output_min: 10.0,
        output_max: 90.0,
    }];

    let snapshot = document.composition_for_frame(0, 150).unwrap();
    assert_eq!(snapshot.layers[0].transform.opacity.evaluate(150), 50.0);
    assert_eq!(
        document.project.compositions[0].layers[0]
            .transform
            .opacity
            .evaluate(150),
        100.0
    );
}
#[test]
fn composition_snapshot_scopes_duplicate_layer_ids_and_keeps_precomp_data() {
    let mut project = Project::default();
    let mut child = Composition::new("comp_child".into(), "Child".into(), 320, 180, 30, 30);
    child.add_layer(Layer::new(
        "child.layer".into(),
        "Child layer".into(),
        LayerType::Null,
        30,
    ));
    project.compositions.push(child);
    let mut document = ProductionDocument::new(project);
    document.bindings = vec![AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.comp.comp_child.layer.child.layer.opacity".into(),
        curve: crate::core::automation_binding::AutomationCurve::from_events([
            crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.0,
            },
        ])
        .unwrap(),
        input_min: 0.0,
        input_max: 1.0,
        output_min: 25.0,
        output_max: 75.0,
    }];

    let child_snapshot = document.composition_for_frame(1, 0).unwrap();
    let main_snapshot = document.composition_for_frame(0, 0).unwrap();
    assert_eq!(child_snapshot.id, "comp_child");
    assert_eq!(child_snapshot.layers[0].transform.opacity.evaluate(0), 25.0);
    assert_eq!(main_snapshot.layers[0].transform.opacity.evaluate(0), 100.0);
}
