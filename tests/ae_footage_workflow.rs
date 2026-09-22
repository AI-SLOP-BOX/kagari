//! End-to-end coverage for the footage path that users rely on most:
//! numbered media import, proxy generation, tracking, preview rendering, and
//! image-sequence export.

use kagari_vfx::core::ffmpeg_export::{start_png_sequence_export, ExportEvent};
use kagari_vfx::core::proxy::{generate_preview_proxy_for_layer, ProxyResolution};
use kagari_vfx::core::timeline::{Composition, Layer, LayerType, TrackerPoint};
use kagari_vfx::core::tracker_engine::TrackerEngine;
use kagari_vfx::core::video_import::import_image_sequence;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TestWorkspace(PathBuf);

impl TestWorkspace {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("kagari_{label}_{}_{}", std::process::id(), nonce));
        std::fs::create_dir_all(&path).expect("workflow test workspace should be writable");
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

fn write_moving_plate(path: &Path, offset: u32) {
    let mut image = image::RgbaImage::from_pixel(24, 16, image::Rgba([0, 0, 0, 0]));
    for y in 5..11 {
        for x in offset..offset + 6 {
            image.put_pixel(x, y, image::Rgba([255, 80, 20, 255]));
        }
    }
    image
        .save_with_format(path, image::ImageFormat::Png)
        .expect("workflow frame should be writable");
}

#[test]
fn footage_workflow_import_proxy_track_preview_and_export_stays_connected() {
    let workspace = TestWorkspace::new("footage_workflow");
    let source_dir = workspace.path().join("source");
    let cache_dir = workspace.path().join("cache");
    let proxy_dir = workspace.path().join("proxies");
    let export_dir = workspace.path().join("export");
    std::fs::create_dir_all(&source_dir).expect("source directory should be writable");

    let first_frame = source_dir.join("plate_0001.png");
    write_moving_plate(&first_frame, 4);
    write_moving_plate(&source_dir.join("plate_0002.png"), 6);
    write_moving_plate(&source_dir.join("plate_0003.png"), 8);

    let asset = import_image_sequence(&first_frame, &cache_dir, 24.0)
        .expect("numbered image sequence should enter WebP footage cache");
    assert_eq!(asset.frame_count, 3);
    assert_eq!(asset.fps, 24.0);
    assert!(Path::new(&asset.frames_dir)
        .join("frame_00000.webp")
        .is_file());

    let mut layer = Layer::new(
        "plate".into(),
        "Moving Plate".into(),
        LayerType::Video {
            source: asset.source_path.clone(),
            frames_dir: asset.frames_dir.clone(),
            frame_count: asset.frame_count,
            audio_wav: asset.audio_wav.clone(),
            speed: 1.0,
        },
        3,
    );
    layer.transform.position = kagari_vfx::core::property::Animatable::new_constant([12.0, 8.0]);
    layer.trackers.push(TrackerPoint::new(
        "plate-point".into(),
        "Plate point".into(),
        [7.0, 8.0],
    ));

    let tracked = TrackerEngine::track_next_frame(&layer, 24, 0, 0)
        .expect("tracker should read the imported WebP frame sequence");
    assert!(tracked.iter().all(|value| value.is_finite()));
    assert!(tracked[0] > 7.0, "tracking should follow the moving plate");

    let proxy_path = generate_preview_proxy_for_layer(&layer, ProxyResolution::Half, &proxy_dir)
        .expect("video footage should generate a WebP proxy sequence");
    layer.proxy.enabled = true;
    layer.proxy.resolution = ProxyResolution::Half;
    layer.proxy.proxy_path = Some(proxy_path.clone());
    assert!(Path::new(&proxy_path).join("frame_00000.webp").is_file());

    let mut comp = Composition::new("main".into(), "Main".into(), 24, 16, 24, 3);
    comp.layers.push(layer);
    let preview = kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        &comp, 1, 24, 16, 0.0, 0,
    );
    assert_eq!(preview.len(), 24 * 16 * 4);
    assert!(preview.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));

    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let export_comp = comp.clone();
    start_png_sequence_export(
        export_dir.clone(),
        "plate".into(),
        24,
        16,
        3,
        0,
        sender,
        cancel,
        move |frame| {
            kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
                &export_comp,
                frame,
                24,
                16,
                0.0,
                0,
            )
        },
    );

    let mut finished = false;
    for _ in 0..8 {
        match receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("export worker should report progress or completion")
        {
            ExportEvent::Finished(_) => {
                finished = true;
                break;
            }
            ExportEvent::Progress(_, _) => {}
            ExportEvent::Error(message) => panic!("workflow export failed: {message}"),
        }
    }
    assert!(finished);
    for frame in 0..3 {
        let output = export_dir.join(format!("plate_{frame:04}.png"));
        assert!(
            output.is_file(),
            "missing exported frame {}",
            output.display()
        );
        assert_eq!(image::image_dimensions(output).unwrap(), (24, 16));
    }
}
