#[test]
fn round_trip_preserves_audio_vfx_contract() {
    let mut document = ProductionDocument::new(Project::default());
    document.color_lut_path = Some("looks/film.cube".into());
    document.audio.sample_rate = 44_100;
    document.bindings.push(AutomationBinding {
        source: "audio.bass".into(),
        target: "vfx.glow.intensity".into(),
        curve: crate::core::automation_binding::AutomationCurve {
            points: vec![crate::core::automation_binding::AutomationPoint {
                time: crate::core::unified_time::Time::ZERO,
                value: 0.5,
            }],
        },
        input_min: 0.0,
        input_max: 1.0,
        output_min: 0.0,
        output_max: 100.0,
    });

    let restored = ProductionDocument::from_json(&document.to_json().unwrap()).unwrap();
    assert_eq!(restored.color_lut_path.as_deref(), Some("looks/film.cube"));
    assert_eq!(restored.audio.sample_rate, 44_100);
    assert_eq!(restored.bindings.len(), 1);
    assert_eq!(
        restored
            .clock()
            .sample_position(crate::core::unified_time::Time::new(1, 1)),
        44_100
    );
}
#[test]
fn legacy_document_without_color_lut_path_defaults_to_none() {
    let document = ProductionDocument::new(Project::default());
    let mut value = serde_json::to_value(document).unwrap();
    value.as_object_mut().unwrap().remove("color_lut_path");
    let restored = ProductionDocument::from_json(&value.to_string()).unwrap();
    assert_eq!(restored.color_lut_path, None);
}
