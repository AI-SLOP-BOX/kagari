use super::*;

fn test_cache_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "kagari_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("test cache directory should be created");
    root
}

#[test]
fn legacy_nonempty_but_corrupt_frames_are_rejected() {
    let root = test_cache_dir("corrupt_video_cache");
    std::fs::write(root.join("frame_00000.webp"), b"nonempty but not an image")
        .expect("corrupt test frame should be written");

    assert!(!video_frame_cache_is_complete(
        root.to_str().expect("temporary path should be UTF-8"),
        1
    ));

    std::fs::remove_dir_all(root).expect("test cache directory should be removable");
}

#[test]
fn cache_manifest_detects_frame_content_changes() {
    let root = test_cache_dir("video_cache_manifest");
    let frame = root.join("frame_00000.png");
    image::RgbaImage::from_pixel(4, 4, image::Rgba([30, 80, 120, 255]))
        .save(&frame)
        .expect("legacy frame should be written");

    let cache_path = root.to_str().expect("temporary path should be UTF-8");
    assert!(video_frame_cache_is_complete(cache_path, 1));
    assert!(root.join(FRAME_CACHE_MANIFEST_FILE).is_file());

    std::fs::write(&frame, b"different nonempty content").expect("mutated frame should be written");
    assert!(!video_frame_cache_is_complete(cache_path, 1));

    std::fs::remove_dir_all(root).expect("test cache directory should be removable");
}

#[test]
fn import_dimensions_respect_decoded_byte_budget() {
    assert!(dimensions_within_import_budget(8192, 4096));
    assert!(!dimensions_within_import_budget(8192, 4097));
    assert!(!dimensions_within_import_budget(16_384, 16_384));
    assert!(!dimensions_within_import_budget(16_385, 1));
}

#[test]
fn disk_estimate_scales_with_frames_and_includes_audio() {
    let still_video = estimate_import_disk_bytes(1920, 1080, 30, 1.0, false)
        .expect("bounded dimensions should produce an estimate");
    let longer_video = estimate_import_disk_bytes(1920, 1080, 60, 2.0, false)
        .expect("bounded duration should produce an estimate");
    let with_audio = estimate_import_disk_bytes(1920, 1080, 30, 1.0, true)
        .expect("bounded duration should include audio estimate");

    assert_eq!(longer_video, still_video * 2);
    assert!(with_audio > still_video);
}

#[test]
fn test_frame_path_clamps_to_range() {
    let asset = VideoAsset {
        source_path: "src.mp4".into(),
        frames_dir: "/tmp/media/frames".into(),
        frame_count: 10,
        fps: 30.0,
        width: 640,
        height: 360,
        audio_wav: None,
    };
    assert!(frame_path(&asset, 0).ends_with("frame_00000.png"));
    assert!(frame_path(&asset, 5).ends_with("frame_00005.png"));
    // Out-of-range clamps to last frame (freeze-frame behavior)
    assert_eq!(frame_path(&asset, 999), frame_path(&asset, 9));
}

#[test]
fn test_import_rejects_missing_source() {
    let result = import_video(
        "/nonexistent/video.mp4",
        Path::new("/tmp/kagari_vid_test"),
        30.0,
    );
    // Missing source must be rejected whether or not ffmpeg exists
    if ffmpeg_available() {
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}

#[test]
fn test_import_rejects_without_ffmpeg_gracefully() {
    // If ffmpeg is absent we get a clean error, never a panic
    let result = import_video("/dev/null", Path::new("/tmp/kagari_vid_test2"), 30.0);
    let err = result.expect_err("/dev/null is never a valid video source");
    assert!(
        err.contains("ffmpeg not found")
            || err.contains("source file not found")
            || err.contains("could not determine video"),
        "unexpected import failure mode: {}",
        err
    );
}

#[test]
fn import_directory_is_unique_without_overwriting_existing_media() {
    let root = std::env::temp_dir().join(format!(
        "kagari_video_import_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    ));
    let requested = root.join("clip");
    let first = create_unique_import_dir(&requested).expect("first directory should be made");
    let second = create_unique_import_dir(&requested).expect("second directory should be made");
    assert_eq!(first, requested);
    assert_eq!(second, root.join("clip_1"));
    assert!(first.is_dir());
    assert!(second.is_dir());
    std::fs::remove_dir_all(root).expect("test directories should be removable");
}

#[test]
fn image_sequence_import_reindexes_frames_as_webp() {
    let root = std::env::temp_dir().join(format!(
        "kagari_image_sequence_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("test root should be created");
    for frame in [1u32, 2, 3] {
        let image = image::RgbaImage::from_pixel(8, 4, image::Rgba([frame as u8, 20, 40, 255]));
        image
            .save(root.join(format!("shot_{frame:04}.png")))
            .expect("source frame should be writable");
    }

    let source = root.join("shot_0002.png");
    let asset = import_image_sequence(&source, &root.join("cache"), 24.0)
        .expect("numbered stills should import");
    assert_eq!(asset.frame_count, 3);
    assert!(frame_path(&asset, 0).ends_with("frame_00000.webp"));
    assert!(frame_path(&asset, 2).is_file());
    assert_eq!(
        image::image_dimensions(frame_path(&asset, 1)).unwrap(),
        (8, 4)
    );

    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn refresh_rebuilds_missing_video_cache_from_source() {
    if !ffmpeg_available() {
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "kagari_video_relink_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("test root should be created");
    let source = root.join("clip.mp4");
    let encoded = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=32x32:r=10",
            "-t",
            "0.2",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&source)
        .output()
        .expect("ffmpeg should be executable when reported available");
    if !encoded.status.success() {
        std::fs::remove_dir_all(root).expect("test root should be removable");
        return;
    }

    let mut project = crate::core::timeline::Project::default();
    project.compositions[0]
        .layers
        .push(crate::core::timeline::Layer::new(
            "video".into(),
            "Clip".into(),
            crate::core::timeline::LayerType::Video {
                source: source.to_string_lossy().into_owned(),
                frames_dir: root.join("missing").to_string_lossy().into_owned(),
                frame_count: 0,
                audio_wav: None,
                audio_volume: crate::core::property::Animatable::new_constant(0.0),
                speed: 1.0,
            },
            30,
        ));

    let cache_root = root.join("cache");
    let (refreshed, errors) = refresh_missing_video_caches(&mut project, &cache_root);
    assert_eq!(refreshed, 1);
    assert!(errors.is_empty(), "unexpected cache errors: {errors:?}");
    let crate::core::timeline::LayerType::Video {
        frames_dir,
        frame_count,
        audio_wav,
        ..
    } = &project.compositions[0].layers.last().unwrap().layer_type
    else {
        panic!("expected refreshed video layer");
    };
    assert!(*frame_count > 0);
    assert!(frame_path_in_dir(frames_dir, 0).is_file());
    assert!(audio_wav.is_none());

    std::fs::remove_dir_all(root).expect("test root should be removable");
}
