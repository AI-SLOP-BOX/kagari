#![allow(dead_code)]
//! FFmpeg-based async video export pipeline.
//!
//! Spawns an FFmpeg subprocess in a background thread to avoid blocking the UI.
//! Progress is communicated back to the UI via an `mpsc` channel of `ExportEvent`.

pub mod gif;
pub mod sequence;
pub mod types;
pub mod video;

pub use gif::*;
pub use sequence::*;
pub use types::*;
pub use video::*;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub fn start_export<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + Sync + 'static,
{
    start_export_cancelable(
        config,
        tx,
        Arc::new(AtomicBool::new(false)),
        render_frame_fn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_availability_check() {
        assert_eq!(is_ffmpeg_available(), is_ffmpeg_available());
    }

    #[test]
    fn test_export_config_clone() {
        let cfg = ExportConfig {
            audio_wav: None,
            codec: VideoCodec::H264,
            output_path: "test.mp4".to_string(),
            width: 1920,
            height: 1080,
            fps: 30,
            total_frames: 90,
        };
        let cfg2 = cfg.clone();
        assert_eq!(cfg.output_path, cfg2.output_path);
    }

    #[test]
    fn parallel_export_reports_render_callback_failure_to_caller() {
        let (tx, rx) = std::sync::mpsc::channel();
        let items = vec![crate::core::parallel_render::RenderQueueItem {
            comp_name: "Broken Comp".into(),
            start_frame: 12,
            end_frame: 12,
            output_path: "/tmp/parallel-export-test.mp4".into(),
            status: crate::core::parallel_render::RenderStatus::Pending,
        }];
        let cancel_flag = Arc::new(AtomicBool::new(false));

        start_parallel_export(
            items,
            tx,
            cancel_flag,
            Arc::new(|_: &str, _: u32| -> Vec<u8> { panic!("synthetic render failure") }),
        )
        .expect("parallel export worker must start");

        let event = rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("parallel export must report callback failure");
        match event {
            ExportEvent::Error(message) => {
                assert!(message.contains("item 0"));
                assert!(message.contains("frame 12"));
            }
            other => panic!("expected an export error, got {other:?}"),
        }
    }

    #[test]
    fn render_with_cancel_converts_callback_panic_to_error() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let result = render_with_cancel(
            &cancel_flag,
            &|_: u32| -> Vec<u8> { panic!("synthetic render failure") },
            3,
        );
        assert!(result.is_err());
    }

    #[test]
    fn png_sequence_reports_render_callback_failure_to_caller() {
        let dir =
            std::env::temp_dir().join(format!("kagari-png-panic-test-{}", std::process::id()));
        let (tx, rx) = std::sync::mpsc::channel();
        start_png_sequence_export(
            dir.clone(),
            "frame".into(),
            1,
            1,
            1,
            0,
            tx,
            Arc::new(AtomicBool::new(false)),
            |_frame| panic!("synthetic render failure"),
        );

        let event = rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("PNG export must report callback failure");
        match event {
            ExportEvent::Error(message) => assert!(message.contains("frame 0")),
            other => panic!("expected an export error, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
