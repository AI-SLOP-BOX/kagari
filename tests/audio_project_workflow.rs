//! Compressed audio must survive the project-bin → timeline → save/load path
//! and remain audible through the real mixer.

use kagari_vfx::core::audio_engine::{mix_audio_for_frame, AudioBuffer, MasterDspParams};
use kagari_vfx::core::production_document::ProductionDocument;
use kagari_vfx::core::timeline::{
    Composition, Layer, LayerType, Project, ProjectItem, ProjectItemType,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestWorkspace(PathBuf);

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kagari_audio_project_{}_{}",
            std::process::id(),
            nonce
        ));
        std::fs::create_dir_all(&path).expect("test workspace should be writable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn compressed_audio_asset_survives_project_reload_and_mixes() {
    let workspace = TestWorkspace::new();
    let audio_path = workspace.path().join("tone.mp3");
    std::fs::write(
        &audio_path,
        include_bytes!("fixtures/kagari_audio_tone.mp3"),
    )
    .expect("copy MP3 fixture into project workspace");

    let decoded = AudioBuffer::load_audio(&audio_path).expect("Symphonia should decode MP3");
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.sample_rate, 44_100);
    assert!(decoded.peak_at(0.05, 0.1) > 0.1);

    let mut composition = Composition::new("main".into(), "Main".into(), 64, 64, 30, 30);
    composition.layers.push(Layer::new(
        "music".into(),
        "Tone.mp3".into(),
        LayerType::Audio {
            path: audio_path.to_string_lossy().into_owned(),
            volume: kagari_vfx::core::property::Animatable::new_constant(0.0),
        },
        30,
    ));

    let project = Project {
        compositions: vec![composition],
        active_composition_idx: 0,
        assets: vec![ProjectItem::new(
            "asset_tone",
            "Tone.mp3",
            ProjectItemType::Audio {
                path: audio_path.to_string_lossy().into_owned(),
                duration_sec: decoded.samples.len() as f32
                    / (decoded.sample_rate as f32 * decoded.channels as f32),
            },
        )],
        use_gpu_compute: false,
    };
    let document = ProductionDocument::new(project);
    let project_path = workspace.path().join("audio.kagari");
    document
        .save_atomic(&project_path)
        .expect("save production document atomically");

    let reopened = ProductionDocument::load(&project_path).expect("reload saved project");
    let reopened_project = reopened.project();
    let asset_audio_path = match &reopened_project.assets[0].item_type {
        ProjectItemType::Audio { path, .. } => path,
        other => panic!("expected audio project-bin item after reload, found {other:?}"),
    };
    assert_eq!(
        asset_audio_path.as_str(),
        audio_path.to_string_lossy().as_ref()
    );
    let reopened_composition = &reopened_project.compositions[0];
    let reopened_audio = match &reopened_composition.layers[0].layer_type {
        LayerType::Audio { path, .. } => path,
        other => panic!("expected audio layer after reload, found {other:?}"),
    };
    assert_eq!(
        reopened_audio.as_str(),
        audio_path.to_string_lossy().as_ref()
    );

    let (mix, meter) = mix_audio_for_frame(
        reopened_composition,
        3,
        48_000,
        1_024,
        &MasterDspParams::bypass(),
    );
    assert!(mix.iter().any(|sample| sample.abs() > 0.01));
    assert!(meter.peak_db_left > -40.0, "MP3 was not present in the mix");
}

#[cfg(feature = "gui")]
#[test]
fn project_bin_add_button_inserts_audio_layer_from_pointer_input() {
    use eframe::egui::{self, CentralPanel, Event, PointerButton, RawInput, Shape};
    use kagari_vfx::core::history::ProjectHistory;
    use kagari_vfx::KagariApp;

    let workspace = TestWorkspace::new();
    let audio_path = workspace.path().join("tone.mp3");
    std::fs::write(
        &audio_path,
        include_bytes!("fixtures/kagari_audio_tone.mp3"),
    )
    .expect("copy MP3 fixture into project workspace");

    let composition = Composition::new("main".into(), "Main".into(), 64, 64, 30, 30);
    let project = Project {
        compositions: vec![composition],
        active_composition_idx: 0,
        assets: vec![ProjectItem::new(
            "asset_tone",
            "Tone.mp3",
            ProjectItemType::Audio {
                path: audio_path.to_string_lossy().into_owned(),
                duration_sec: 0.2,
            },
        )],
        use_gpu_compute: false,
    };
    let mut app = KagariApp::default();
    app.history = ProjectHistory::new(project);
    let ctx = egui::Context::default();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("selected_project_asset"), 0usize));
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 800.0));

    let output = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        },
        |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                kagari_vfx::ui::project_panel::draw(&mut app, ui);
            });
        },
    );
    let click_pos = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Add to Active Comp" => {
                Some(text.visual_bounding_rect().center())
            }
            _ => None,
        })
        .expect("project bin should render its add-to-composition button label");

    let click_input = |pressed| RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(click_pos),
            Event::PointerButton {
                pos: click_pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run(click_input(true), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            kagari_vfx::ui::project_panel::draw(&mut app, ui);
        });
    });
    let _ = ctx.run(click_input(false), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            kagari_vfx::ui::project_panel::draw(&mut app, ui);
        });
    });

    let layers = &app.history.current().active_composition().layers;
    assert_eq!(
        layers.len(),
        1,
        "button click should create exactly one layer"
    );
    match &layers[0].layer_type {
        LayerType::Audio { path, .. } => {
            assert_eq!(path.as_str(), audio_path.to_string_lossy().as_ref());
        }
        other => panic!("project-bin click inserted the wrong layer type: {other:?}"),
    }
    assert!(app.history.can_undo(), "the UI action should be undoable");
    assert_eq!(
        app.history
            .undo()
            .expect("undo the UI insertion")
            .active_composition()
            .layers
            .len(),
        0
    );
    assert_eq!(
        app.history
            .redo()
            .expect("redo the UI insertion")
            .active_composition()
            .layers
            .len(),
        1
    );

    let project_path = workspace.path().join("audio_ui.kagari");
    ProductionDocument::new(app.history.current().clone())
        .save_atomic(&project_path)
        .expect("save the project-bin insertion");
    let reopened = ProductionDocument::load(&project_path).expect("reload the UI-created layer");
    let reopened_composition = &reopened.project().compositions[0];
    assert!(matches!(
        reopened.project().assets[0].item_type,
        ProjectItemType::Audio { .. }
    ));
    let (mix, meter) = mix_audio_for_frame(
        reopened_composition,
        3,
        48_000,
        1_024,
        &MasterDspParams::bypass(),
    );
    assert!(mix.iter().any(|sample| sample.abs() > 0.01));
    assert!(meter.peak_db_left > -40.0, "UI-added MP3 was not mixed");
}
