//! Editing-assist algorithms used by automatic timeline cleanup commands.

pub mod audio;
pub mod motion;
pub mod spatial;
pub mod types;
pub mod vision;

pub use audio::{detect_audio_beats, detect_audio_clipping, detect_silence, normalization_gain_db};
pub use motion::{build_auto_reframe_path, reduce_motion_path};
pub use spatial::{
    camera_shake_score, find_loop_candidates, find_safe_overlay_position, sharpness_score,
    smart_aspect_crop,
};
pub use types::{
    ExposureReport, LetterboxInsets, LoopCandidate, PixelBounds, ReframePoint, TimeRange,
};
pub use vision::{
    analyze_exposure_clipping, deduplicate_frame_indices, detect_black_frames, detect_flash_frames,
    detect_frozen_frames, detect_letterbox_insets, detect_scene_cuts, transparent_content_bounds,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_bounds_find_single_pixel() {
        let mut pixels = vec![0; 4 * 3 * 4];
        pixels[(2 * 4 + 1) * 4 + 3] = 255;
        assert_eq!(
            transparent_content_bounds(&pixels, 4, 3, 0),
            Some(PixelBounds {
                x: 1,
                y: 2,
                width: 1,
                height: 1
            })
        );
    }

    #[test]
    fn transparent_bounds_reject_bad_buffer() {
        assert_eq!(transparent_content_bounds(&[0; 3], 1, 1, 0), None);
    }

    #[test]
    fn scene_cuts_detect_black_to_white() {
        let black = vec![0; 4 * 4 * 4];
        let white = vec![255; 4 * 4 * 4];
        assert_eq!(
            detect_scene_cuts(&[&black, &black, &white], 4, 4, 0.5),
            vec![2]
        );
    }

    #[test]
    fn scene_cuts_ignore_identical_frames() {
        let frame = vec![42; 4 * 4 * 4];
        assert!(detect_scene_cuts(&[&frame, &frame], 4, 4, 0.1).is_empty());
    }

    #[test]
    fn frozen_frames_find_multiple_runs() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![200; 2 * 2 * 4];
        assert_eq!(
            detect_frozen_frames(&[&a, &a, &a, &b, &b], 2, 2, 0.0, 2),
            vec![0..=2, 3..=4]
        );
    }

    #[test]
    fn frozen_frames_reject_mismatched_frame() {
        assert!(detect_frozen_frames(&[&[0; 16], &[0; 3]], 2, 2, 0.0, 2).is_empty());
    }

    #[test]
    fn black_frame_detection_allows_small_bright_area() {
        let black = vec![0; 10 * 10 * 4];
        let mut almost_black = black.clone();
        almost_black[0..4].fill(255);
        let bright = vec![255; 10 * 10 * 4];
        assert_eq!(
            detect_black_frames(&[&black, &almost_black, &bright], 10, 10, 8, 0.98),
            vec![0, 1]
        );
    }

    #[test]
    fn letterbox_detects_top_and_bottom_bars() {
        let mut frame = vec![255; 6 * 6 * 4];
        for y in [0, 5] {
            for x in 0..6 {
                frame[(y * 6 + x) * 4..(y * 6 + x + 1) * 4].fill(0);
            }
        }
        assert_eq!(
            detect_letterbox_insets(&frame, 6, 6, 4, 1.0),
            Some(LetterboxInsets {
                top: 1,
                bottom: 1,
                left: 0,
                right: 0
            })
        );
    }

    #[test]
    fn letterbox_rejects_fully_black_frame() {
        assert_eq!(detect_letterbox_insets(&[0; 4 * 4 * 4], 4, 4, 8, 1.0), None);
    }

    #[test]
    fn loop_candidates_prefer_matching_frames() {
        let a = vec![10; 4 * 4 * 4];
        let b = vec![100; 4 * 4 * 4];
        let candidates = find_loop_candidates(&[&a, &b, &a], 4, 4, 2, 1);
        assert_eq!(candidates[0].start_frame, 0);
        assert_eq!(candidates[0].end_frame, 2);
        assert_eq!(candidates[0].error, 0.0);
    }

    #[test]
    fn exposure_report_counts_only_neutral_clipping() {
        let frame = [0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255];
        let report = analyze_exposure_clipping(&frame, 3, 1, 4, 250).unwrap();
        assert!((report.shadow_clipped_fraction - 1.0 / 3.0).abs() < 0.001);
        assert!((report.highlight_clipped_fraction - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn flash_detection_finds_single_frame_spike() {
        let dark = vec![10; 2 * 2 * 4];
        let bright = vec![240; 2 * 2 * 4];
        assert_eq!(
            detect_flash_frames(&[&dark, &bright, &dark], 2, 2, 100.0),
            vec![1]
        );
    }

    #[test]
    fn flash_detection_ignores_gradual_change() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![100; 2 * 2 * 4];
        let c = vec![200; 2 * 2 * 4];
        assert!(detect_flash_frames(&[&a, &b, &c], 2, 2, 50.0).is_empty());
    }

    #[test]
    fn deduplication_keeps_only_visual_changes() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![20; 2 * 2 * 4];
        assert_eq!(
            deduplicate_frame_indices(&[&a, &a, &b, &b, &a], 2, 2, 0.0),
            vec![0, 2, 4]
        );
    }

    #[test]
    fn safe_overlay_avoids_occupied_bottom() {
        let occupied = [PixelBounds {
            x: 0,
            y: 60,
            width: 100,
            height: 40,
        }];
        let result = find_safe_overlay_position([100, 100], [40, 20], &occupied, 10).unwrap();
        assert_eq!(
            result,
            PixelBounds {
                x: 30,
                y: 10,
                width: 40,
                height: 20
            }
        );
    }

    #[test]
    fn safe_overlay_rejects_oversized_content() {
        assert!(find_safe_overlay_position([100, 100], [100, 20], &[], 1).is_none());
    }

    #[test]
    fn sharpness_score_distinguishes_edge_from_flat_frame() {
        let flat = vec![100; 5 * 5 * 4];
        let mut edge = flat.clone();
        for y in 0..5 {
            for x in 2..5 {
                edge[(y * 5 + x) * 4..(y * 5 + x) * 4 + 3].fill(255);
            }
        }
        assert!(sharpness_score(&edge, 5, 5).unwrap() > sharpness_score(&flat, 5, 5).unwrap());
    }

    #[test]
    fn shake_score_ignores_constant_velocity() {
        assert_eq!(
            camera_shake_score(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]),
            0.0
        );
        assert!(camera_shake_score(&[[0.0, 0.0], [2.0, 0.0], [0.0, 0.0]]) > 0.0);
    }

    #[test]
    fn smart_crop_keeps_target_aspect_and_focus() {
        let focus = [PixelBounds {
            x: 80,
            y: 20,
            width: 20,
            height: 20,
        }];
        let crop = smart_aspect_crop([100, 100], 0.5, &focus).unwrap();
        assert_eq!([crop.width, crop.height], [50, 100]);
        assert_eq!(crop.x, 50);
    }

    #[test]
    fn smart_crop_defaults_to_center() {
        assert_eq!(
            smart_aspect_crop([100, 50], 1.0, &[]),
            Some(PixelBounds {
                x: 25,
                y: 0,
                width: 50,
                height: 50
            })
        );
    }

    #[test]
    fn smart_crop_ignores_regions_outside_frame() {
        let crop = smart_aspect_crop(
            [100, 100],
            1.0,
            &[PixelBounds {
                x: 10_000,
                y: 10_000,
                width: 20,
                height: 20,
            }],
        )
        .unwrap();
        assert_eq!(
            crop,
            PixelBounds {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }
        );
    }

    #[test]
    fn silence_detection_merges_windows() {
        let mut samples = vec![0.0; 100];
        samples.extend(vec![1.0; 100]);
        let ranges = detect_silence(&samples, 100, -40.0, 0.5);
        assert_eq!(
            ranges,
            vec![TimeRange {
                start_seconds: 0.0,
                end_seconds: 1.0
            }]
        );
    }

    #[test]
    fn nonfinite_audio_is_not_silence() {
        assert!(detect_silence(&[f32::NAN; 100], 100, -40.0, 0.1).is_empty());
    }

    #[test]
    fn beat_detection_finds_energy_transients() {
        let mut samples = vec![0.0; 1_000];
        for start in [200, 600] {
            samples[start..start + 10].fill(1.0);
        }
        let beats = detect_audio_beats(&samples, 1_000, 0.5, 0.1);
        assert_eq!(beats, vec![0.2, 0.6]);
    }

    #[test]
    fn beat_detection_rejects_invalid_rate() {
        assert!(detect_audio_beats(&[1.0], 0, 1.0, 0.1).is_empty());
    }

    #[test]
    fn normalization_gain_targets_rms() {
        let gain = normalization_gain_db(&[0.1; 100], -6.0, 0.0);
        assert!((gain - 14.0).abs() < 0.01);
    }

    #[test]
    fn normalization_gain_respects_peak_ceiling() {
        let mut samples = vec![0.0; 100];
        samples[0] = 1.0;
        let gain = normalization_gain_db(&samples, -6.0, -1.0);
        assert!((gain + 1.0).abs() < 0.01);
    }

    #[test]
    fn normalization_gain_ignores_nonfinite_samples() {
        assert_eq!(normalization_gain_db(&[f32::NAN], -12.0, -1.0), 0.0);
    }

    #[test]
    fn normalization_gain_handles_extreme_finite_samples() {
        let gain = normalization_gain_db(&[f32::MAX, -f32::MAX], -12.0, -1.0);
        assert!(gain.is_finite());
        assert!((-60.0..=60.0).contains(&gain));
    }

    #[test]
    fn audio_clipping_merges_consecutive_samples() {
        let ranges = detect_audio_clipping(&[0.0, 1.0, 1.0, 0.0, -1.0, -1.0], 10, 0.99, 2);
        assert_eq!(
            ranges,
            vec![
                TimeRange {
                    start_seconds: 0.1,
                    end_seconds: 0.3
                },
                TimeRange {
                    start_seconds: 0.4,
                    end_seconds: 0.6
                },
            ]
        );
    }

    #[test]
    fn audio_clipping_ignores_nonfinite_samples() {
        assert!(detect_audio_clipping(&[f32::NAN, 1.0], 10, 0.99, 2).is_empty());
    }

    #[test]
    fn auto_reframe_clamps_to_crop_safe_region() {
        let path = build_auto_reframe_path(&[[0.0, 0.0], [100.0, 50.0]], [100, 50], [40, 20], 0.0);
        assert_eq!(path[0].center, [20.0, 10.0]);
        assert_eq!(path[1].center, [80.0, 40.0]);
    }

    #[test]
    fn auto_reframe_smooths_motion() {
        let path = build_auto_reframe_path(&[[20.0, 20.0], [80.0, 20.0]], [100, 40], [20, 20], 0.5);
        assert_eq!(path[1].center, [50.0, 20.0]);
    }

    #[test]
    fn auto_reframe_rejects_impossible_crop() {
        assert!(build_auto_reframe_path(&[[1.0, 1.0]], [10, 10], [20, 20], 0.5).is_empty());
    }

    #[test]
    fn straight_motion_path_keeps_endpoints() {
        let points = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]];
        assert_eq!(reduce_motion_path(&points, 0.01), vec![0, 3]);
    }

    #[test]
    fn motion_path_keeps_corner() {
        let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        assert_eq!(reduce_motion_path(&points, 0.1), vec![0, 1, 2]);
    }

    #[test]
    fn invalid_motion_path_is_preserved() {
        let points = [[0.0, 0.0], [f32::NAN, 1.0], [2.0, 2.0]];
        assert_eq!(reduce_motion_path(&points, 1.0), vec![0, 1, 2]);
    }
}
