#[test]
fn rejects_future_schema_and_invalid_audio() {
    let mut document = ProductionDocument::new(Project::default());
    document.schema_version += 1;
    assert!(document.validate().is_err());

    document.schema_version = ProductionDocument::CURRENT_SCHEMA_VERSION;
    document.audio.sample_rate = 0;
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.audio.master_gain = f32::NAN;
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.audio.master_gain = -0.1;
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.audio.master_gain = 1_000.1;
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.audio.channels.push(MixerChannel {
        gain_db: 25.0,
        pan: 0.0,
        mute: false,
        solo: false,
    });
    assert!(document.validate().is_err());

    let mut document = ProductionDocument::new(Project::default());
    document.audio.channels.push(MixerChannel {
        gain_db: 0.0,
        pan: 100.1,
        mute: false,
        solo: false,
    });
    assert!(document.validate().is_err());
}
