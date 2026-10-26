//! Motion Sketch Real-Time Freehand Tracking & Keyframe Baking Engine (AE Parity).
//!
//! Captures mouse/stylus drag gestures with high-resolution timestamps during playback
//! and resamples the trajectory into smooth position keyframes on the target layer.

#![allow(dead_code)]

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::timeline::Layer;

const MAX_MOTION_SKETCH_BAKE_SAMPLES: usize = 4096;
const MAX_MOTION_SKETCH_RECORDED_SAMPLES: usize = MAX_MOTION_SKETCH_BAKE_SAMPLES * 2;

pub fn record_motion_sketch_sample(
    recording: &mut Vec<(u32, [f32; 2])>,
    frame: u32,
    position: [f32; 2],
) {
    if !position.iter().all(|value| value.is_finite()) {
        return;
    }
    if let Some(last) = recording.last_mut().filter(|last| last.0 == frame) {
        last.1 = position;
        return;
    }
    if recording.len() >= MAX_MOTION_SKETCH_RECORDED_SAMPLES {
        *recording = uniformly_sample(recording, MAX_MOTION_SKETCH_BAKE_SAMPLES);
    }
    recording.push((frame, position));
}

/// Converts recorded composition-space samples into the keyframes committed by the editor.
pub fn build_position_keyframes(
    recording: &[(u32, [f32; 2])],
    simplify_tolerance: f32,
) -> Vec<Keyframe<[f32; 2]>> {
    let mut ordered: Vec<_> = recording
        .iter()
        .copied()
        .filter(|(_, position)| position.iter().all(|value| value.is_finite()))
        .collect();
    ordered.sort_by_key(|(frame, _)| *frame);
    let mut unique = Vec::with_capacity(ordered.len());
    for sample in ordered {
        if unique
            .last()
            .is_some_and(|previous: &(u32, [f32; 2])| previous.0 == sample.0)
        {
            *unique.last_mut().expect("last sample exists") = sample;
        } else {
            unique.push(sample);
        }
    }
    let bounded = uniformly_sample(&unique, MAX_MOTION_SKETCH_BAKE_SAMPLES);
    let tolerance = if simplify_tolerance.is_finite() {
        simplify_tolerance.max(0.0)
    } else {
        0.0
    };
    let simplified = simplify_recording(&bounded, tolerance);
    let smoothed = smooth_recording(&simplified);
    let mut keyframes: Vec<_> = smoothed
        .into_iter()
        .map(|(frame, position)| {
            Keyframe::new(
                frame,
                position,
                InterpolationType::Bezier {
                    outgoing: crate::core::keyframe::BezierControlPoint {
                        influence: 40.0,
                        speed: 80.0,
                    },
                    incoming: crate::core::keyframe::BezierControlPoint {
                        influence: 40.0,
                        speed: 0.0,
                    },
                    custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
                },
            )
        })
        .collect();

    keyframes.sort_by_key(|keyframe| keyframe.frame);
    let mut unique = Vec::with_capacity(keyframes.len());
    for keyframe in keyframes {
        if unique
            .last()
            .is_some_and(|previous: &Keyframe<[f32; 2]>| previous.frame == keyframe.frame)
        {
            *unique.last_mut().expect("last keyframe exists") = keyframe;
        } else {
            unique.push(keyframe);
        }
    }
    unique
}

fn simplify_recording(points: &[(u32, [f32; 2])], tolerance: f32) -> Vec<(u32, [f32; 2])> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut ranges = vec![(0usize, points.len() - 1)];
    while let Some((start, end)) = ranges.pop() {
        if end <= start + 1 {
            continue;
        }
        let (max_idx, max_distance) = (start + 1..end)
            .map(|index| {
                (
                    index,
                    point_to_line_distance(points[index].1, points[start].1, points[end].1),
                )
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or((start, 0.0));
        if max_distance > tolerance {
            keep[max_idx] = true;
            ranges.push((start, max_idx));
            ranges.push((max_idx, end));
        }
    }
    points
        .iter()
        .copied()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(point))
        .collect()
}

fn uniformly_sample(points: &[(u32, [f32; 2])], limit: usize) -> Vec<(u32, [f32; 2])> {
    if points.len() <= limit || limit < 2 {
        return points.to_vec();
    }
    (0..limit)
        .map(|index| {
            let source_index = index * (points.len() - 1) / (limit - 1);
            points[source_index]
        })
        .collect()
}

fn point_to_line_distance(point: [f32; 2], start: [f32; 2], end: [f32; 2]) -> f32 {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length_squared = delta[0] * delta[0] + delta[1] * delta[1];
    if length_squared < 1e-10 {
        return ((point[0] - start[0]).powi(2) + (point[1] - start[1]).powi(2)).sqrt();
    }

    let t = (((point[0] - start[0]) * delta[0] + (point[1] - start[1]) * delta[1])
        / length_squared)
        .clamp(0.0, 1.0);
    let projection = [start[0] + t * delta[0], start[1] + t * delta[1]];
    ((point[0] - projection[0]).powi(2) + (point[1] - projection[1]).powi(2)).sqrt()
}

fn smooth_recording(points: &[(u32, [f32; 2])]) -> Vec<(u32, [f32; 2])> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(points.len());
    result.push(points[0]);
    for window in points.windows(3) {
        result.push((
            window[1].0,
            [
                (window[0].1[0] + window[1].1[0] + window[2].1[0]) / 3.0,
                (window[0].1[1] + window[1].1[1] + window[2].1[1]) / 3.0,
            ],
        ));
    }
    result.push(*points.last().expect("three or more points"));
    result
}

/// Captured raw motion sketch coordinate point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSketchSample {
    pub time_sec: f64,
    pub position: [f32; 2],
}

/// Active motion sketch recording session.
#[derive(Debug, Clone, Default)]
pub struct MotionSketchSession {
    pub is_recording: bool,
    pub samples: Vec<MotionSketchSample>,
    pub smoothing_radius: usize,
}

impl MotionSketchSession {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            samples: Vec::new(),
            smoothing_radius: 1,
        }
    }

    /// Starts a new recording session.
    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.samples.clear();
    }

    /// Records a single mouse/pen position sample.
    pub fn record_sample(&mut self, time_sec: f64, position: [f32; 2]) {
        if self.is_recording {
            self.samples.push(MotionSketchSample { time_sec, position });
        }
    }

    /// Stops recording and returns captured samples count.
    pub fn stop_recording(&mut self) -> usize {
        self.is_recording = false;
        self.samples.len()
    }

    /// Resamples captured timestamped gesture into discrete frame keyframes.
    pub fn bake_to_layer(
        &self,
        layer: &mut Layer,
        fps: f32,
        start_frame: u32,
        duration_frames: u32,
    ) {
        if self.samples.len() < 2 || fps <= 0.0 {
            return;
        }

        let start_time = self.samples.first().map(|s| s.time_sec).unwrap_or(0.0);
        let mut keyframes = Vec::new();

        for f in 0..duration_frames {
            let target_time = start_time + (f as f64 / fps as f64);

            // Linear search for enclosing sample interval
            let pos = if target_time <= self.samples[0].time_sec {
                self.samples[0].position
            } else if target_time >= self.samples.last().unwrap().time_sec {
                self.samples.last().unwrap().position
            } else {
                let mut p = self.samples[0].position;
                for i in 0..(self.samples.len() - 1) {
                    let s0 = &self.samples[i];
                    let s1 = &self.samples[i + 1];
                    if target_time >= s0.time_sec && target_time <= s1.time_sec {
                        let dt = (s1.time_sec - s0.time_sec).max(1e-5);
                        let t = ((target_time - s0.time_sec) / dt) as f32;
                        p = [
                            s0.position[0] + (s1.position[0] - s0.position[0]) * t,
                            s0.position[1] + (s1.position[1] - s0.position[1]) * t,
                        ];
                        break;
                    }
                }
                p
            };

            keyframes.push(Keyframe::new(
                start_frame + f,
                pos,
                InterpolationType::Linear,
            ));
        }

        layer.transform.position = crate::core::property::Animatable::new_animated(keyframes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::LayerType;

    #[test]
    fn test_motion_sketch_recording_and_baking() {
        let mut session = MotionSketchSession::new();
        session.start_recording();
        session.record_sample(0.0, [100.0, 100.0]);
        session.record_sample(0.5, [200.0, 150.0]);
        session.record_sample(1.0, [300.0, 200.0]);
        session.stop_recording();

        let mut layer = Layer::new("1".into(), "Sketch Layer".into(), LayerType::Null, 10);
        session.bake_to_layer(&mut layer, 2.0, 0, 3); // 2 fps over 1 second = 3 frames (0, 1, 2)

        assert_eq!(layer.transform.position.evaluate(0), [100.0, 100.0]);
        assert_eq!(layer.transform.position.evaluate(1), [200.0, 150.0]);
        assert_eq!(layer.transform.position.evaluate(2), [300.0, 200.0]);
    }
}
