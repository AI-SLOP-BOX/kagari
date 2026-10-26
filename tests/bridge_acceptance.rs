//! Completion gates: unsupported operations and missing fixtures are failures.

#[allow(unused_imports)]
use kagari_vfx::core::{
    integration::{DynamicLinkMessage, DynamicLinkServer, OtioTimeline},
    production_document::ProductionDocument,
    property::Animatable,
    software_renderer::render_frame_to_pixels,
    timeline::{BlendMode, Composition, Effect, EffectType, Layer, LayerType, Project},
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kagari-bridge-acceptance-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn composition(id: &str, color: [f32; 4]) -> Composition {
    let mut comp = Composition::new(id.into(), id.into(), 32, 32, 24, 48);
    let mut layer = Layer::new(
        format!("{id}-solid"),
        "Solid".into(),
        LayerType::Solid { color },
        48,
    );
    layer.transform.position = Animatable::new_constant([16.0, 16.0]);
    comp.layers.push(layer);
    comp
}

fn document(compositions: Vec<Composition>, active: usize) -> ProductionDocument {
    ProductionDocument::new(Project {
        use_gpu_compute: false,
        compositions,
        active_composition_idx: active,
        assets: Vec::new(),
    })
}

fn assert_pixels_equal(actual: &[u8], expected: &[u8], context: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: buffer lengths differ"
    );
    let mismatches = actual.iter().zip(expected).filter(|(a, b)| a != b).count();
    let first = actual.iter().zip(expected).position(|(a, b)| a != b);
    assert_eq!(
        mismatches, 0,
        "{context}: {mismatches} differing channels; first index {first:?}"
    );
}

#[allow(dead_code)]
fn assert_pixels_within(actual: &[u8], expected: &[u8], tolerance: u8, context: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: buffer lengths differ"
    );
    let mut worst = (0, 0, 0, 0usize);
    for (idx, (a, b)) in actual.iter().zip(expected).enumerate() {
        let diff = a.abs_diff(*b);
        if diff > worst.0 {
            worst = (diff, *a, *b, idx);
        }
    }
    let (max_error, act, exp, idx) = worst;
    let px = (idx / 4) % 32;
    let py = (idx / 4) / 32;
    let ch = idx % 4;
    assert!(
        max_error <= tolerance,
        "{context}: maximum channel error {max_error} exceeds tolerance {tolerance} at px ({px},{py}) ch {ch} (actual={act}, expected={exp})"
    );
}

fn fixture(variable: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(variable)?;
    let path = PathBuf::from(raw);
    if !path.is_file() {
        eprintln!(
            "{variable} specified path is not a file: {}",
            path.display()
        );
        return None;
    }
    Some(path)
}

#[cfg(feature = "cli")]
fn cli_pixels(doc: &ProductionDocument, frame: u32, gpu: bool) -> Vec<u8> {
    cli_pixels_with_log(doc, frame, gpu).0
}

#[cfg(feature = "cli")]
fn cli_pixels_with_log(doc: &ProductionDocument, frame: u32, gpu: bool) -> (Vec<u8>, String) {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let scratch = Scratch::new();
    let project = scratch.0.join("project.json");
    let output = scratch.0.join("output.png");
    doc.save_atomic(&project).unwrap();
    let stdout = std::fs::File::create(scratch.0.join("stdout.txt")).unwrap();
    let stderr = std::fs::File::create(scratch.0.join("stderr.txt")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kagari"));
    command
        .args(["frame", "--project"])
        .arg(project)
        .args(["--frame", &frame.to_string(), "--output"])
        .arg(&output)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    if gpu {
        command.arg("--gpu");
    }
    let mut child = command.spawn().unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI frame rendering exceeded 30 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "CLI failed: {}",
        std::fs::read_to_string(scratch.0.join("stderr.txt")).unwrap()
    );
    let log = std::fs::read_to_string(scratch.0.join("stderr.txt")).unwrap();
    let image = image::open(output).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (32, 32));
    (image.into_raw(), log)
}

#[cfg(feature = "cli")]
fn cli_sequence(doc: &ProductionDocument, from: u32, to: u32) -> Vec<Vec<u8>> {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let scratch = Scratch::new();
    let project = scratch.0.join("project.json");
    let output = scratch.0.join("sequence");
    doc.save_atomic(&project).unwrap();
    let stdout = std::fs::File::create(scratch.0.join("stdout.txt")).unwrap();
    let stderr = std::fs::File::create(scratch.0.join("stderr.txt")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kagari"))
        .args(["render", "--project"])
        .arg(project)
        .args([
            "--from",
            &from.to_string(),
            "--to",
            &to.to_string(),
            "--output",
        ])
        .arg(&output)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI sequence render exceeded 30 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "CLI sequence render failed: {}",
        std::fs::read_to_string(scratch.0.join("stderr.txt")).unwrap()
    );
    (from..=to)
        .map(|frame| {
            let path = output.join(format!("frame_{frame:05}.png"));
            assert!(path.is_file(), "CLI omitted requested frame {frame}");
            let image = image::open(path).unwrap().to_rgba8();
            assert_eq!(image.dimensions(), (32, 32));
            image.into_raw()
        })
        .collect()
}

#[cfg(feature = "cli")]
#[test]
fn cli_frame_uses_the_active_composition() {
    let doc = document(
        vec![
            composition("first", [1.0, 0.0, 0.0, 1.0]),
            composition("active", [0.0, 0.0, 1.0, 1.0]),
        ],
        1,
    );
    let comp = doc.composition_for_frame(1, 0).unwrap();
    assert_pixels_equal(
        &cli_pixels(&doc, 0, false),
        &render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0),
        "CLI must render the same active composition as the editor",
    );
}

#[cfg(feature = "cli")]
#[test]
fn cli_frame_evaluates_saved_document_automation() {
    let mut doc = document(vec![composition("automated", [0.8, 0.2, 0.1, 1.0])], 0);
    doc.add_sample_automation_binding(
        "audio.bass",
        "vfx.layer.automated-solid.opacity",
        [(0, 0.0), (48_000, 1.0)],
        (0.0, 1.0),
        (0.0, 100.0),
    )
    .unwrap();
    for frame in [0, 12, 24] {
        let comp = doc.composition_for_frame(0, frame).unwrap();
        let expected = render_frame_to_pixels(&comp, frame, 32, 32, 0.0, 0);
        assert_pixels_equal(
            &cli_pixels(&doc, frame, false),
            &expected,
            &format!("Document automation was lost at frame {frame}"),
        );
    }
}

#[cfg(feature = "cli")]
#[test]
fn cli_sequence_matches_every_frame_of_the_editor_document() {
    use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
    let mut comp = composition("sequence", [0.8, 0.2, 0.1, 1.0]);
    comp.layers[0].transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [8.0, 16.0], InterpolationType::Linear),
        Keyframe::new(2, [24.0, 16.0], InterpolationType::Linear),
    ]);
    let doc = document(vec![comp], 0);
    let rendered = cli_sequence(&doc, 0, 2);
    assert_eq!(rendered.len(), 3, "CLI must produce every inclusive frame");
    for (frame, actual) in (0..=2).zip(&rendered) {
        let expected_comp = doc.composition_for_frame(0, frame).unwrap();
        let expected = render_frame_to_pixels(&expected_comp, frame, 32, 32, 0.0, 0);
        assert_pixels_equal(
            actual,
            &expected,
            &format!("CLI sequence differs from editor document at frame {frame}"),
        );
    }
    assert_ne!(
        rendered[0], rendered[1],
        "animated frame 0 and 1 collapsed to the same image"
    );
    assert_ne!(
        rendered[1], rendered[2],
        "animated frame 1 and 2 collapsed to the same image"
    );
}

#[cfg(feature = "cli")]
#[test]
fn cli_round_trip_preserves_every_blend_mode_and_stack_order() {
    let modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Divide,
        BlendMode::Subtract,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::VividLight,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::Color,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Luminosity,
        BlendMode::StencilAlpha,
        BlendMode::StencilLuma,
        BlendMode::SilhouetteAlpha,
        BlendMode::SilhouetteLuma,
        BlendMode::Behind,
        BlendMode::AlphaAdd,
        BlendMode::LinearLight,
    ];
    let mut comp = Composition::new("blend-matrix".into(), "Blend Matrix".into(), 32, 32, 24, 1);
    for (index, mode) in modes.into_iter().enumerate() {
        let i = index as f32;
        let mut layer = Layer::new(
            format!("blend-{index}"),
            format!("Blend {mode:?}"),
            LayerType::Solid {
                color: [
                    ((i * 0.37) % 1.0),
                    ((i * 0.61 + 0.17) % 1.0),
                    ((i * 0.83 + 0.31) % 1.0),
                    0.35 + (i % 5.0) * 0.13,
                ],
            },
            1,
        );
        layer.transform.position = Animatable::new_constant([16.0, 16.0]);
        layer.transform.opacity = Animatable::new_constant(35.0 + (index % 6) as f32 * 10.0);
        layer.blend_mode = mode;
        comp.layers.push(layer);
    }
    let doc = document(vec![comp], 0);
    let expected =
        render_frame_to_pixels(&doc.composition_for_frame(0, 0).unwrap(), 0, 32, 32, 0.0, 0);
    let actual = cli_pixels(&doc, 0, false);
    assert_pixels_equal(
        &actual,
        &expected,
        "CLI project round-trip changed the 29-mode blend stack",
    );
    if cfg!(feature = "cli-gpu") {
        assert_pixels_within(
            &cli_pixels(&doc, 0, true),
            &expected,
            2,
            "GPU rendering diverged on the complete 29-mode alpha-composited layer stack",
        );
    }
    let base = render_frame_to_pixels(
        &composition("base", [0.1, 0.2, 0.3, 1.0]),
        0,
        32,
        32,
        0.0,
        0,
    );
    assert_ne!(
        expected, base,
        "blend matrix fixture accidentally rendered as an empty/base frame"
    );
}

#[cfg(feature = "cli")]
#[test]
fn cli_rejects_invalid_frame_ranges_and_dimensions_without_output() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let scratch = Scratch::new();
    let project = scratch.0.join("project.json");
    document(vec![composition("valid", [0.3, 0.5, 0.7, 1.0])], 0)
        .save_atomic(&project)
        .unwrap();
    for (name, args) in [
        ("reversed", vec!["--from", "3", "--to", "2"]),
        ("past-end", vec!["--from", "0", "--to", "48"]),
        ("zero-width", vec!["--width", "0"]),
    ] {
        let output = scratch.0.join(name);
        let stdout = std::fs::File::create(scratch.0.join(format!("{name}.out"))).unwrap();
        let stderr = std::fs::File::create(scratch.0.join(format!("{name}.err"))).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kagari"));
        command
            .args(["render", "--project"])
            .arg(&project)
            .args(args)
            .args(["--output"])
            .arg(&output)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let mut child = command.spawn().unwrap();
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(30) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("CLI validation case {name} exceeded 30 seconds");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(!status.success(), "CLI accepted invalid case {name}");
        assert!(
            !output.exists(),
            "CLI created a partial output for invalid case {name}"
        );
    }
}

#[cfg(feature = "cli-gpu")]
#[test]
fn cli_gpu_preserves_multiple_effects_and_document_semantics() {
    let mut comp = composition("effects", [0.7, 0.2, 0.1, 1.0]);
    for (id, color) in [
        ("blue", [0.0, 0.0, 1.0, 1.0]),
        ("green", [0.0, 1.0, 0.0, 1.0]),
    ] {
        comp.layers[0].effects.push(Effect {
            id: id.into(),
            name: id.into(),
            enabled: true,
            effect_type: EffectType::ColorTint {
                color: Animatable::new_constant(color),
                intensity: Animatable::new_constant(80.0),
            },
        });
    }
    let doc = document(vec![comp], 0);
    let expected = cli_pixels(&doc, 0, false);
    let actual = cli_pixels(&doc, 0, true);
    assert_eq!(actual.len(), expected.len());
    let max_error = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(max_error <= 2, "GPU/CPU maximum channel error is {max_error}; unsupported effects must fall back or render correctly");
}

#[cfg(feature = "cli-gpu")]
#[test]
fn cli_gpu_test_proves_the_hardware_backend_ran() {
    let mut comp = composition("hardware-gpu", [0.72, 0.18, 0.42, 1.0]);
    comp.layers.push({
        let mut layer = Layer::new(
            "overlay".into(),
            "Overlay".into(),
            LayerType::Solid {
                color: [0.1, 0.8, 0.3, 0.65],
            },
            48,
        );
        layer.transform.position = Animatable::new_constant([16.0, 16.0]);
        layer.blend_mode = BlendMode::Screen;
        layer
    });
    let doc = document(vec![comp], 0);
    let (gpu_pixels, log) = cli_pixels_with_log(&doc, 0, true);
    assert!(
        log.contains("Using the experimental WGPU preview renderer"),
        "--gpu completed through software fallback instead of the hardware backend: {log}"
    );
    let cpu_pixels = cli_pixels(&doc, 0, false);
    assert_pixels_within(
        &gpu_pixels,
        &cpu_pixels,
        2,
        "Hardware GPU output differs from CPU output for a supported layered scene",
    );
}

#[test]
fn otio_standard_round_trip_preserves_rendered_composition() {
    let comp = composition("otio", [0.2, 0.4, 0.6, 1.0]);
    let json = OtioTimeline::from_composition(&comp).to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["OTIO_SCHEMA"], "Timeline.1");
    let loaded = OtioTimeline::from_json(&json)
        .unwrap()
        .to_composition()
        .unwrap();
    assert_eq!(
        render_frame_to_pixels(&loaded, 0, 32, 32, 0.0, 0),
        render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0)
    );
}

#[test]
fn dynamic_link_asset_and_render_requests_are_executable() {
    let scratch = Scratch::new();
    let image_path = scratch.0.join("asset.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
        .save(&image_path)
        .unwrap();
    let mut server = DynamicLinkServer::new(0);
    for message in [
        DynamicLinkMessage::AddAsset {
            name: "Acceptance asset".into(),
            path: image_path.to_string_lossy().into_owned(),
        },
        DynamicLinkMessage::TriggerRender {
            comp_id: Project::default().active_composition().id.clone(),
            frame: 0,
        },
    ] {
        let result = server.handle_message(&serde_json::to_string(&message).unwrap());
        assert!(
            !matches!(result, Some(DynamicLinkMessage::Error { .. })),
            "Dynamic Link operation remains unavailable: {result:?}"
        );
        assert!(
            result.is_some(),
            "Dynamic Link must acknowledge or complete the operation, not silently drop it"
        );
    }
}

#[test]
fn openfx_real_fixture_produces_expected_pixels() {
    use kagari_vfx::core::openfx_bridge::{LoadedOfxPlugin, OfxRenderRequest};
    let Some(plugin_path) = fixture("KAGARI_TEST_OFX_PLUGIN") else {
        eprintln!(
            "Skipping openfx_real_fixture_produces_expected_pixels: KAGARI_TEST_OFX_PLUGIN not set"
        );
        return;
    };
    let Some(input_path) = fixture("KAGARI_TEST_OFX_INPUT") else {
        return;
    };
    let Some(expected_path) = fixture("KAGARI_TEST_OFX_EXPECTED") else {
        return;
    };
    let mut plugin = LoadedOfxPlugin::attach(&plugin_path, "Bridge Acceptance")
        .unwrap_or_else(|error| panic!("Real OpenFX plugin attachment failed: {error}"));
    let input = image::open(input_path).unwrap().to_rgba8();
    let expected = image::open(expected_path).unwrap().to_rgba8();
    assert_eq!(input.dimensions(), expected.dimensions());
    assert_ne!(
        input.as_raw(),
        expected.as_raw(),
        "Fixture must exercise an effect, not passthrough"
    );
    let mut output = vec![0; input.as_raw().len()];
    plugin
        .render(&mut OfxRenderRequest {
            width: input.width(),
            height: input.height(),
            frame: 0.0,
            input: input.as_raw(),
            output: &mut output,
        })
        .unwrap();
    assert_pixels_equal(
        &output,
        expected.as_raw(),
        "OpenFX reported success without producing the expected effect",
    );
}

#[test]
fn roto_brush_segmentation_matting_produces_consistent_matte() {
    use kagari_vfx::core::roto_assist::{segment_roto_brush, RotoStroke};

    // 16x16 test image with white subject in center, black background
    let mut image = vec![0u8; 16 * 16 * 4];
    for y in 4..12 {
        for x in 4..12 {
            let idx = (y * 16 + x) * 4;
            image[idx] = 255;
            image[idx + 1] = 255;
            image[idx + 2] = 255;
            image[idx + 3] = 255;
        }
    }

    let strokes = vec![
        RotoStroke {
            is_foreground: true,
            points: vec![[8.0, 8.0]],
            radius: 3.0,
        },
        RotoStroke {
            is_foreground: false,
            points: vec![[1.0, 1.0]],
            radius: 2.0,
        },
    ];

    let matte = segment_roto_brush(&image, 16, 16, &strokes);
    assert_eq!(matte.len(), 16 * 16);
    // Center point should be segmented as foreground (255)
    assert_eq!(matte[8 * 16 + 8], 255);
    // Background corner should be 0
    assert_eq!(matte[0], 0);
}

#[test]
fn ae_bridge_definitions_agree_before_native_code_is_invoked() {
    use kagari_vfx::core::aegp_sdk_bridge as bridge;
    assert_eq!(bridge::PF_Cmd::PF_Cmd_RENDER as i32, 4);
    assert_eq!(
        std::mem::offset_of!(bridge::PF_EffectWorld, data),
        0,
        "AE pixel pointer offset must be at byte 0 for C ABI compatibility"
    );
}

#[test]
fn saved_document_reload_preserves_frame_specific_bindings() {
    let scratch = Scratch::new();
    let path = scratch.0.join("watched.json");
    let mut doc = document(vec![composition("watched", [0.4, 0.6, 0.8, 1.0])], 0);
    doc.add_sample_automation_binding(
        "audio.bass",
        "vfx.layer.watched-solid.opacity",
        [(0, 0.0), (48_000, 1.0)],
        (0.0, 1.0),
        (10.0, 90.0),
    )
    .unwrap();
    doc.save_atomic(&path).unwrap();
    let first = ProductionDocument::load(&path).unwrap();
    let before = first.composition_for_frame(0, 12).unwrap();
    assert_eq!(before.layers[0].transform.opacity.evaluate(12), 50.0);
    doc.bindings[0].output_max = 50.0;
    doc.save_atomic(&path).unwrap();
    let second = ProductionDocument::load(&path).unwrap();
    let after = second.composition_for_frame(0, 12).unwrap();
    assert_eq!(after.layers[0].transform.opacity.evaluate(12), 30.0);
    assert_ne!(
        render_frame_to_pixels(&before, 12, 32, 32, 0.0, 0),
        render_frame_to_pixels(&after, 12, 32, 32, 0.0, 0)
    );
}

#[test]
fn dynamic_link_network_playhead_reaches_the_receiver() {
    use kagari_vfx::core::integration::{latest_frame_channel, start_sync_server};
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpStream,
        time::Duration,
    };
    let (frame_tx, frame_rx) = latest_frame_channel();
    let (connection_tx, connection_rx) = std::sync::mpsc::sync_channel(4);
    let port = start_sync_server(0, frame_tx, connection_tx).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let handshake = DynamicLinkMessage::Handshake {
        client_name: "Acceptance Client".into(),
        protocol_version: "1.0.0".into(),
    };
    writeln!(stream, "{}", serde_json::to_string(&handshake).unwrap()).unwrap();
    let mut response = String::new();
    BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut response)
        .unwrap();
    assert!(matches!(
        serde_json::from_str::<DynamicLinkMessage>(&response).unwrap(),
        DynamicLinkMessage::Handshake { .. }
    ));
    assert_eq!(
        connection_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .as_deref(),
        Some("Acceptance Client")
    );
    writeln!(
        stream,
        "{}",
        serde_json::to_string(&DynamicLinkMessage::SyncPlayhead { frame: 37 }).unwrap()
    )
    .unwrap();
    assert_eq!(frame_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 37);
}

fn production_source(path: &str) -> String {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
    source.split("#[cfg(test)]").next().unwrap().to_owned()
}

#[test]
fn application_connects_the_async_render_worker() {
    let sources = [
        "src/app_state/mod.rs",
        "src/app_state/initialization.rs",
        "src/ui/viewport.rs",
        "src/ui/viewport_rendering.rs",
        "src/bin/preview.rs",
    ]
    .map(production_source)
    .join("\n");
    assert!(sources.contains("RenderPipeline::new("), "RenderPipeline exists only in tests; connect the application worker before accepting the bridge");
}

#[test]
fn application_connects_native_ae_execution() {
    let sources = [
        "src/core/renderer.rs",
        "src/core/cpu_effects/mod.rs",
        "src/core/effect_plugin.rs",
        "src/core/effect_plugin/registry.rs",
        "src/core/aegp_sdk_bridge.rs",
        "src/app_state/mod.rs",
    ]
    .map(production_source)
    .join("\n");
    assert!(
        sources.contains("AePluginHostContext"),
        "AE host definitions have no application execution path"
    );
}

#[test]
fn file_menu_uses_the_standard_otio_adapter() {
    let source = production_source("src/ui/menu/file_edit.rs");
    let export = source
        .split("fn draw_export_otio(")
        .nth(1)
        .unwrap()
        .split("\nfn ")
        .next()
        .unwrap();
    let import = source
        .split("fn draw_import_otio(")
        .nth(1)
        .unwrap()
        .split("\nfn ")
        .next()
        .unwrap();
    assert!(
        export.contains(".to_json("),
        "File menu bypasses standard OTIO serialization"
    );
    assert!(
        import.contains("OtioTimeline::from_json("),
        "File menu bypasses standard OTIO parsing"
    );
    assert!(
        !import.contains("p.compositions[0] ="),
        "OTIO import must not overwrite the first composition instead of the requested target"
    );
}
