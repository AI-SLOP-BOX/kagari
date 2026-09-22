use kagari_vfx::core::mask::Mask;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::roto_assist::{bake_optical_flow_roto_matte, set_roto_matte_keyframe};
use kagari_vfx::core::roto_brush_engine::{RotoBrushSettings, RotoStroke, RotoStrokeType};
use kagari_vfx::core::timeline::{Composition, Layer, LayerType};
use kagari_vfx::core::video_import::{ffmpeg_available, import_video};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestWorkspace(PathBuf);

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kagari_roto_footage_{}_{}",
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

fn write_subject_frame(path: &Path, left: u32) {
    let mut image = image::RgbaImage::from_pixel(24, 16, image::Rgba([0, 0, 0, 0]));
    for y in 5..11 {
        for x in left..left + 6 {
            image.put_pixel(x, y, image::Rgba([230, 35, 20, 255]));
        }
    }
    image
        .save_with_format(path, image::ImageFormat::Png)
        .expect("subject frame should be writable");
}

fn polygon_contains(polygon: &[[f32; 2]], point: [f32; 2]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

#[test]
fn imported_webp_footage_roto_propagates_persists_and_previews() {
    if !ffmpeg_available() {
        eprintln!("skipping roto footage workflow: FFmpeg is not installed");
        return;
    }
    let workspace = TestWorkspace::new();
    let source_dir = workspace.path().join("source");
    std::fs::create_dir_all(&source_dir).expect("source directory should be writable");
    let first_frame = source_dir.join("subject_0001.png");
    write_subject_frame(&first_frame, 4);
    write_subject_frame(&source_dir.join("subject_0002.png"), 6);
    write_subject_frame(&source_dir.join("subject_0003.png"), 8);
    let source_video = workspace.path().join("subject.mp4");
    let encoded = Command::new("ffmpeg")
        .args(["-y", "-framerate", "24", "-i"])
        .arg(source_dir.join("subject_%04d.png"))
        .args(["-frames:v", "3", "-c:v", "mpeg4", "-q:v", "2"])
        .arg(&source_video)
        .output()
        .expect("FFmpeg should encode the test footage");
    assert!(
        encoded.status.success(),
        "test video encoding failed: {}",
        String::from_utf8_lossy(&encoded.stderr)
    );

    let footage = import_video(
        source_video
            .to_str()
            .expect("test video path should be UTF-8"),
        &workspace.path().join("media"),
        24.0,
    )
    .expect("encoded subject clip should import as WebP footage");
    assert_eq!(footage.frame_count, 3);
    assert!(
        Path::new(&footage.frames_dir)
            .join("frame_00002.webp")
            .is_file()
            || Path::new(&footage.frames_dir)
                .join("frame_00002.png")
                .is_file(),
        "FFmpeg import should produce a decoded third frame"
    );

    let mut layer = Layer::new(
        "subject".into(),
        "Moving Subject".into(),
        LayerType::Video {
            source: footage.source_path,
            frames_dir: footage.frames_dir,
            frame_count: footage.frame_count,
            audio_wav: footage.audio_wav,
            speed: 1.0,
        },
        3,
    );
    layer.transform.position = Animatable::new_constant([12.0, 8.0]);
    layer.roto_brush_strokes = vec![
        RotoStroke {
            stroke_type: RotoStrokeType::Foreground,
            points: vec![[6.0, 7.0]],
            radius: 2.0,
            frame: Some(0),
        },
        RotoStroke {
            stroke_type: RotoStrokeType::Background,
            points: vec![[1.0, 7.0], [16.0, 7.0]],
            radius: 2.0,
            frame: Some(0),
        },
        RotoStroke {
            stroke_type: RotoStrokeType::Background,
            points: vec![[20.0, 7.0]],
            radius: 2.0,
            frame: Some(2),
        },
    ];
    let mut mask = Mask::new_rect("roto".into(), "Roto Brush Matte".into(), 3.0, 4.0, 9.0, 8.0);
    let strokes = layer.roto_brush_strokes.clone();
    let mut comp = Composition::new("main".into(), "Main".into(), 24, 16, 24, 3);
    comp.background_color = [0.0; 4];
    let mut source_comp = comp.clone();
    source_comp.background_color = [0.0; 4];
    source_comp.layers.push(layer.clone());
    let settings = RotoBrushSettings {
        feather_radius: 0.0,
        contrast: 1.0,
        ..Default::default()
    };
    let propagated =
        bake_optical_flow_roto_matte(&mask, &strokes, &settings, 0, 0, 2, 24, 16, |frame| {
            Some(
                kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
                    &source_comp,
                    frame,
                    24,
                    16,
                    0.0,
                    0,
                ),
            )
        })
        .expect("roto propagation should consume the imported footage frames");
    let Animatable::Animated(keyframes) = propagated else {
        panic!("propagation should create one matte key per footage frame");
    };
    assert_eq!(
        keyframes.iter().map(|key| key.frame).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    let vertex_count = keyframes[0].value.len();
    assert!(keyframes.iter().all(|key| key.value.len() == vertex_count));
    let last_polygon = &keyframes[2].value;
    assert!(polygon_contains(last_polygon, [10.0, 7.0]));
    assert!(!polygon_contains(last_polygon, [20.0, 7.0]));

    for keyframe in keyframes {
        assert!(set_roto_matte_keyframe(
            &mut mask,
            keyframe.frame,
            keyframe.value
        ));
    }
    layer.masks.push(mask);
    let mut rendered_comp = comp;
    rendered_comp.layers.push(layer);
    let serialized = serde_json::to_vec(&rendered_comp).expect("roto project should serialize");
    let reopened: Composition =
        serde_json::from_slice(&serialized).expect("roto project should reopen");
    let preview = kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        &reopened, 2, 24, 16, 0.0, 0,
    );
    let pixel = |x: usize, y: usize| &preview[(y * 24 + x) * 4..(y * 24 + x + 1) * 4];
    assert!(
        pixel(10, 7)[3] > 200,
        "propagated subject should remain visible"
    );
    assert!(
        pixel(20, 7)[3] < 5,
        "background should be removed by the matte"
    );
}
